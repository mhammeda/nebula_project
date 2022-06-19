use {
    crate::Error,
    actix_web::HttpRequest,
    anyhow::{anyhow, Result},
    regex::Regex,
    sqlx::{Pool, Postgres},
};

/// Returns whether the supplied user exists
pub(crate) async fn user_exists<U: AsRef<str>, H: AsRef<str>>(
    username: U,
    host: H,
    pool: &Pool<Postgres>,
) -> Result<bool, Error> {
    match sqlx::query!(
        r#"
            SELECT EXISTS(
                SELECT 1 FROM users
                WHERE username = $1 AND host = $2
            )
        "#,
        username.as_ref(),
        host.as_ref()
    )
    .fetch_one(pool)
    .await?
    .exists
    {
        Some(exists) => Ok(exists),
        None => Err(sqlx::error::Error::RowNotFound.into()),
    }
}

/// Returns whether the supplied user is a moderator of the supplied community
pub(crate) async fn is_moderator<U: AsRef<str>, H: AsRef<str>, C: AsRef<str>>(
    username: U,
    host: H,
    community: C,
    pool: &Pool<Postgres>,
) -> Result<bool, Error> {
    match sqlx::query!(
        r#"
            SELECT EXISTS(
                SELECT * FROM moderators
                WHERE username = $1 AND host = $2 AND community = $3
            )
        "#,
        username.as_ref(),
        host.as_ref(),
        community.as_ref()
    )
    .fetch_one(pool)
    .await?
    .exists
    {
        Some(b) => Ok(b),
        None => Err(sqlx::error::Error::RowNotFound.into()),
    }
}

/// Returns whether the supplied user is an admin or not
pub(crate) async fn is_admin<A: AsRef<str>, B: AsRef<str>>(
    db: &Pool<Postgres>,
    username: A,
    host: B,
) -> anyhow::Result<bool, Error> {
    match sqlx::query!(
        r#"
            SELECT EXISTS(SELECT 1 FROM admins WHERE username = $1 AND host = $2)
        "#,
        username.as_ref(),
        host.as_ref(),
    )
    .fetch_one(db)
    .await?
    .exists
    {
        Some(b) => Ok(b),
        None => Err(sqlx::error::Error::RowNotFound.into()),
    }
}

pub fn get_client_host<'a>(req: &'a HttpRequest) -> actix_web::Result<&'a str, Error> {
    if let Some(s) = req.headers().get("Client-Host") {
        if let Ok(s) = s.to_str() {
            return Ok(s);
        }
    }

    Err(Error::BadRequest(anyhow!(
        "Missing or badly-formatted Client-Host header"
    )))
}

pub fn get_user_id<'a>(req: &'a HttpRequest) -> actix_web::Result<&'a str, Error> {
    if let Some(s) = req.headers().get("User-ID") {
        if let Ok(s) = s.to_str() {
            if Regex::new("^[a-zA-Z0-9-_]{1,24}$")
                .expect("Failed to build regular expression")
                .is_match(s)
            {
                return Ok(s);
            }
        }
    }

    Err(Error::BadRequest(anyhow!(
        "Missing or badly-formatted User-ID header"
    )))
}

#[macro_export]
/// Gets hostname of local server
macro_rules! host {
    () => {
        crate::HOST
            .get()
            .expect("Should always be initialised before use")
            .clone()
    };
}
