#![warn(missing_docs)]

//! Group A12 Backend Application

use {
    actix::{Actor, Addr},
    actix_files::Files,
    actix_identity::IdentityService,
    actix_service::{fn_service, Service},
    actix_web::{
        dev::{BodySize, MessageBody, ServiceRequest, ServiceResponse},
        middleware::Logger,
        App, HttpServer,
    },
    anyhow::{bail, Result},
    futures_util::{future::ok, FutureExt},
    log::info,
    middleware::{auth::Authentication, fedsec::Signed},
    once_cell::sync::OnceCell,
    rsa::RSAPrivateKey,
    sentry::IntoDsn,
    serde::Deserialize,
    sqlx::{postgres::PgPoolOptions, Pool, Postgres},
    std::env,
};

mod error;
mod fed;
mod internal;
mod metrics;
mod middleware;
mod models;
#[cfg(test)]
mod test;
mod util;

pub use error::Error;
pub use fed::client::Client;

/// Max number of database connections to open
const DB_MAX_SIZE: u32 = 15;
/// Length of the salt used in password hashes
pub const SALT_LENGTH: usize = 32;
/// Recovery key wordcount (log2(7530^6) = 77 bits)
pub const RECOVERY_LENGTH: usize = 6;
/// "host" value for entities on this server
pub static HOST: OnceCell<String> = OnceCell::new();

/// Application configuration
#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    /// Fully Qualified Domain Name
    fqdn: String,
    /// Web server address
    web_addr: String,
    /// Database address
    database_url: String,
    /// Location of static frontend content to serve
    dist_path: String,
    /// Sentry DSN
    sentry_dsn: Option<String>,
    /// Authentication secret, 512-bit base64 encoded
    secret: String,
    /// RSA Private Key
    privkey: String,
}

/// Shared application data
#[derive(Debug, Clone)]
struct AppData {
    /// Database connection pool
    pool: Pool<Postgres>,
    /// RSA private key
    privkey: RSAPrivateKey,
    /// WebSocket Server
    ws_server: Addr<internal::ws::server::Server>,
    /// JWT secret
    secret: Vec<u8>,
}

/// Run main application
pub async fn run(config: Config) -> Result<()> {
    if HOST.get_or_init(|| config.fqdn.clone()) != &config.fqdn {
        bail!("Failed to initialise local hostname");
    }

    let _sentry_guard = sentry::init(sentry::ClientOptions {
        dsn: config.sentry_dsn.into_dsn().expect("Failed to parse DSN"),
        release: match env::var("HEROKU_RELEASE_VERSION") {
            Ok(s) => Some(s.into()),
            Err(_) => sentry::release_name!(),
        },
        ..Default::default()
    });

    let mut log_builder = pretty_env_logger::formatted_builder();
    // default to info level
    match env::var("RUST_LOG") {
        Ok(s) => log_builder.parse_filters(&s),
        _ => log_builder.parse_filters("info"),
    };
    let logger = sentry_log::SentryLogger::with_dest(log_builder.build());

    log::set_boxed_logger(Box::new(logger)).expect("Failed to set global logger");
    if cfg!(test) {
        // avoid noise in test output
        log::set_max_level(log::LevelFilter::Off);
    } else {
        log::set_max_level(log::LevelFilter::Trace);
    }

    let data = {
        let pool = PgPoolOptions::new()
            .max_connections(DB_MAX_SIZE)
            .connect(&config.database_url)
            .await?;

        let privkey = RSAPrivateKey::from_pkcs8(&base64::decode(&config.privkey)?)?;

        let ws_server = internal::ws::server::Server::new(pool.clone()).start();

        let secret = base64::decode(&config.secret).expect("Failed to decode base64 secret");

        AppData {
            pool,
            privkey,
            ws_server,
            secret,
        }
    };

    let dist_path = config.dist_path;
    let index_path = format!("{}/index.html", &dist_path);

    // Start HTTP server
    info!("Starting HTTP server at http://{}", config.web_addr);
    HttpServer::new(move || {
        let index_path = index_path.clone();

        App::new()
            .data(data.clone())
            .service(metrics::metrics)
            .service(fed::get_communities)
            .service(fed::get_community_by_id)
            .service(fed::get_community_timestamps)
            .service(fed::get_post_by_id)
            .service(fed::get_posts)
            .service(fed::create_post)
            .service(fed::edit_post)
            .service(fed::delete_post)
            .service(fed::get_users)
            .service(fed::get_user_by_id)
            .service(fed::send_message)
            .service(fed::get_public_key)
            .service(fed::get_known_hosts)
            .service(internal::login)
            .service(internal::logout)
            .service(internal::create_user)
            .service(internal::get_user)
            .service(internal::delete_user)
            .service(internal::change_user_password)
            .service(internal::search_users)
            .service(internal::update_avatar_url)
            .service(internal::create_community)
            .service(internal::get_communities)
            .service(internal::get_community_by_id)
            .service(internal::delete_community)
            .service(internal::subscribe_community)
            .service(internal::unsubscribe_community)
            .service(internal::search_communities)
            .service(internal::add_community_moderator)
            .service(internal::remove_community_moderator)
            .service(internal::get_post)
            .service(internal::get_bulk_post)
            .service(internal::create_post)
            .service(internal::edit_post)
            .service(internal::delete_post)
            .service(internal::search_posts)
            .service(internal::get_admins)
            .service(internal::get_admin_status)
            .service(internal::add_admin)
            .service(internal::remove_admin)
            .service(internal::get_unread)
            .service(internal::get_all)
            .service(internal::mark_read)
            .service(internal::get_messages_with_user)
            .service(internal::send_message_to_user)
            .service(internal::get_remote_servers)
            .service(internal::add_remote_server)
            .service(internal::remove_remote_server)
            .service(internal::get_image)
            .service(internal::add_image)
            .service(internal::remove_image)
            .service(internal::ws::open_ws)
            .service(
                Files::new("/", &dist_path)
                    .show_files_listing()
                    .index_file("index.html")
                    .default_handler(fn_service(move |req: ServiceRequest| {
                        let file = match actix_files::NamedFile::open(&index_path) {
                            Ok(f) => f,
                            Err(e) => return ok(req.error_response(e)),
                        };

                        let (req, _) = req.into_parts();

                        match file.into_response(&req) {
                            Ok(item) => ok(ServiceResponse::new(req.clone(), item)),
                            Err(e) => ok(ServiceResponse::from_err(e, req)),
                        }
                    })),
            )
            .wrap(sentry_actix::Sentry::new())
            .wrap(Logger::default())
            .wrap(Signed)
            .wrap(IdentityService::new(Authentication::new(&data.secret)))
            .wrap_fn(|req, srv| {
                metrics::HTTP_COUNTER.inc();
                let timer = metrics::HTTP_REQ_HISTOGRAM
                    .with_label_values(&["all"])
                    .start_timer();
                srv.call(req).map(|res| {
                    timer.observe_duration();

                    match &res {
                        Ok(res) => match res.response().body().size() {
                            BodySize::Sized(s) => {
                                metrics::HTTP_RESP_SIZE_HISTOGRAM.observe(s as f64)
                            }
                            _ => {}
                        },
                        _ => {}
                    }

                    res
                })
            })
    })
    .bind(config.web_addr)?
    .run()
    .await?;

    Ok(())
}
