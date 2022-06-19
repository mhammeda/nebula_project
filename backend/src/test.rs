use {
    crate::Config,
    actix_rt::time::delay_for,
    actix_web::{
        client::Client,
        http::{header::CONTENT_TYPE, Cookie, StatusCode},
        HttpMessage,
    },
    once_cell::sync::Lazy,
    sqlx::Connection,
    std::{
        thread,
        time::{Duration, Instant},
    },
};

/// Seconds to wait for backend to start
const TIMEOUT: u64 = 5;

/// Address of instance of backend
///
/// First dereference will destroy the database and start a new instance of the backend, will
/// panic if backend does not begin responding to requests after TIMEOUT seconds.
pub static ADDR: Lazy<String> = Lazy::new(|| {
    // load environment variables from .env file, failing silently
    dotenv::dotenv().ok();

    let config = envy::from_env::<Config>().unwrap();

    let addr = format!("http://{}", config.web_addr);

    // start backend instance on another thread
    thread::spawn(move || {
        actix::run(async move {
            drop_tables(config.database_url.as_str()).await;
            crate::run(config).await.unwrap()
        })
        .unwrap();
    });

    // block until backend is running by repeatedly making get requests to the backend root
    // path until successful or timeout expires
    let moved_addr = addr.clone();
    thread::spawn(move || {
        actix::run(async move {
            let start = Instant::now();

            let client = Client::new();

            loop {
                if start.elapsed().as_secs() >= TIMEOUT {
                    panic!("Server failed to start within {} seconds", TIMEOUT);
                }

                match client.get(&format!("{}/", moved_addr)).send().await {
                    Ok(r) => {
                        if r.status().is_success() {
                            break;
                        }
                    }
                    Err(_) => {}
                }

                delay_for(Duration::from_millis(1000)).await;
            }

            actix::System::current().stop();
        })
        .unwrap();
    })
    .join()
    .unwrap();

    addr
});

/// **WARNING: DROPS ALL TABLES IN THE SUPPLIED DATABASE**
async fn drop_tables(addr: &str) {
    let mut conn = sqlx::PgConnection::connect(addr).await.unwrap();

    sqlx::query!(
        r#"
            DROP SCHEMA public CASCADE
        "#,
    )
    .execute(&mut conn)
    .await
    .unwrap();

    sqlx::query!(
        r#"
            CREATE SCHEMA public
        "#,
    )
    .execute(&mut conn)
    .await
    .unwrap();

    sqlx::migrate!("./migrations").run(&mut conn).await.unwrap();
}

/// Creates a new user and logs in
pub async fn new_user_login() -> (Client, String, Cookie<'static>) {
    let client = Client::new();

    let username = format!("{}", rand::random::<u16>());
    let password = format!("{}_password", &username);

    // Create a user
    let res = client
        .post(format!("{}/internal/users", *ADDR))
        .header(CONTENT_TYPE, "application/json")
        .send_body(format!(
            "{{\"username\":\"{}\",\"password\":\"{}\"}}",
            username, password
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Login succesfully
    let res = client
        .post(format!("{}/internal/login", *ADDR))
        .header(CONTENT_TYPE, "application/json")
        .send_body(format!(
            "{{\"username\":\"{}\",\"password\":\"{}\"}}",
            username, password
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let cookie = res
        .cookies()
        .expect("Did not receive cookie in login response")[0]
        .clone();

    (client, username, cookie)
}

/// Gives the supplied user admin privileges
pub async fn make_admin<A: AsRef<str>, B: AsRef<str>>(username: A, host: B) {
    let mut conn = sqlx::PgConnection::connect(&envy::from_env::<Config>().unwrap().database_url)
        .await
        .unwrap();

    sqlx::query!(
        r#"
            INSERT INTO admins VALUES ($1, $2)
        "#,
        username.as_ref(),
        host.as_ref(),
    )
    .execute(&mut conn)
    .await
    .unwrap();
}
