use {
    crate::{AppData, Error},
    actix_web::{get, web, Responder, Result},
    once_cell::sync::Lazy,
    prometheus::{
        labels, opts, register_counter, register_gauge, register_histogram, register_histogram_vec,
        Counter, Gauge, Histogram, HistogramVec,
    },
    prometheus::{Encoder, TextEncoder},
};

/// Prometheus metrics scrape endpoint
#[get("/metrics")]
pub(crate) async fn metrics(data: web::Data<AppData>) -> Result<impl Responder, Error> {
    let metric_families = prometheus::gather();

    // get number of users
    NUM_USERS.set(
        sqlx::query!("SELECT COUNT(*) FROM users")
            .fetch_one(&data.pool)
            .await?
            .count
            .unwrap_or(0) as f64,
    );

    // get number of posts
    NUM_POSTS.set(
        sqlx::query!("SELECT COUNT(*) FROM posts")
            .fetch_one(&data.pool)
            .await?
            .count
            .unwrap_or(0) as f64,
    );

    // get number of communities
    NUM_COMMUNITIES.set(
        sqlx::query!("SELECT COUNT(*) FROM communities")
            .fetch_one(&data.pool)
            .await?
            .count
            .unwrap_or(0) as f64,
    );

    // get number of remotes
    NUM_REMOTES.set(
        sqlx::query!("SELECT COUNT(*) FROM remotes")
            .fetch_one(&data.pool)
            .await?
            .count
            .unwrap_or(0) as f64,
    );

    // get number of images
    NUM_IMAGES.set(
        sqlx::query!("SELECT COUNT(*) FROM images")
            .fetch_one(&data.pool)
            .await?
            .count
            .unwrap_or(0) as f64,
    );

    let mut buffer = vec![];
    TextEncoder::new()
        .encode(&metric_families, &mut buffer)
        .unwrap();

    Ok(String::from_utf8(buffer).unwrap())
}

pub static HTTP_COUNTER: Lazy<Counter> = Lazy::new(|| {
    register_counter!(opts!(
        "http_requests_total",
        "Number of HTTP requests made.",
        labels! {"handler" => "all",}
    ))
    .unwrap()
});

pub static HTTP_REQ_HISTOGRAM: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "http_request_duration_seconds",
        "The HTTP request latencies in seconds.",
        &["handler"]
    )
    .unwrap()
});

pub static HTTP_RESP_SIZE_HISTOGRAM: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "http_response_size_bytes",
        "The HTTP response sizes in bytes."
    )
    .unwrap()
});

pub static NUM_USERS: Lazy<Gauge> = Lazy::new(|| {
    register_gauge!(opts!(
        "num_users",
        "Number of users.",
        labels! {"handler" => "all",}
    ))
    .unwrap()
});

pub static NUM_POSTS: Lazy<Gauge> = Lazy::new(|| {
    register_gauge!(opts!(
        "num_posts",
        "Number of posts.",
        labels! {"handler" => "all",}
    ))
    .unwrap()
});

pub static NUM_COMMUNITIES: Lazy<Gauge> = Lazy::new(|| {
    register_gauge!(opts!(
        "num_communities",
        "Number of users.",
        labels! {"handler" => "all",}
    ))
    .unwrap()
});

pub static NUM_REMOTES: Lazy<Gauge> = Lazy::new(|| {
    register_gauge!(opts!(
        "num_remotes",
        "Number of remote servers.",
        labels! {"handler" => "all",}
    ))
    .unwrap()
});

pub static NUM_IMAGES: Lazy<Gauge> = Lazy::new(|| {
    register_gauge!(opts!(
        "num_images",
        "Number of images.",
        labels! {"handler" => "all",}
    ))
    .unwrap()
});
