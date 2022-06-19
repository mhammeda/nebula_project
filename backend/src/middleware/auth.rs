use actix_http::http::HeaderName;

use {
    crate::AppData,
    actix_identity::IdentityPolicy,
    actix_web::{
        cookie::Cookie,
        dev::{ServiceRequest, ServiceResponse},
        error::Error,
        http::header::{self, HeaderValue},
        web, HttpMessage,
    },
    futures_util::future::{ok, Future},
    jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation},
    serde::{Deserialize, Serialize},
    sqlx::{error::Error::RowNotFound, Pool, Postgres},
    std::{
        pin::Pin,
        time::{Duration, SystemTime},
    },
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Token {
    /// Issued At
    pub iat: u64,
    /// Expires
    pub exp: u64,
    pub username: String,
    pub session: [u8; 16],
}

pub struct Authentication {
    secret: [u8; 64],
}

impl Authentication {
    pub fn new<T: AsRef<[u8]>>(slice: T) -> Self {
        let slice = slice.as_ref();

        if slice.len() != 64 {
            panic!("Unexpected key size");
        }

        let mut secret: [u8; 64] = [0u8; 64];
        secret.copy_from_slice(slice);

        Self { secret }
    }

    pub fn generate_token<T: AsRef<str>>(&self, username: T, session: &[u8; 16]) -> String {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("Failed to get timestamp");
        let week = Duration::new(60 * 60 * 24 * 7, 0);

        let iat = now.as_secs();
        let exp = (now + week).as_secs();

        jsonwebtoken::encode(
            &Header::default(),
            &Token {
                iat,
                exp,
                username: username.as_ref().to_owned(),
                session: session.to_owned(),
            },
            &EncodingKey::from_secret(&self.secret),
        )
        .unwrap()
    }
}

pub(crate) async fn validate_cookie(
    value: &str,
    secret: &[u8],
    pool: &Pool<Postgres>,
) -> Result<Option<String>, Error> {
    let token_data = jsonwebtoken::decode::<Token>(
        value,
        &DecodingKey::from_secret(secret),
        &Validation::default(),
    )
    .map_err(|e| crate::Error::BadRequest(e.into()))?;

    let valid: bool = sqlx::query!(
        r#"
            SELECT exists (
                SELECT 1 FROM local_users
                WHERE username = $1
                AND host = $2
                AND session = $3
                LIMIT 1
            );
        "#,
        token_data.claims.username,
        crate::host!(),
        Some(&token_data.claims.session[..])
    )
    .fetch_one(pool)
    .await
    .map_err(|e| crate::Error::from(e))?
    .exists
    .ok_or(crate::Error::from(RowNotFound))?;

    if valid {
        return Ok(Some(token_data.claims.username));
    } else {
        return Ok(None);
    }
}

impl IdentityPolicy for Authentication {
    type Future = Pin<Box<dyn Future<Output = Result<Option<String>, Error>>>>;

    type ResponseFuture = Pin<Box<dyn Future<Output = Result<(), Error>>>>;

    fn from_request(&self, request: &mut ServiceRequest) -> Self::Future {
        // get database connection pool
        let pool = request
            .app_data::<web::Data<AppData>>()
            .expect("Failed to get AppData from request")
            .pool
            .clone();

        // get cookie from request
        if let Some(cookie) = request.cookie("auth") {
            let secret = self.secret.clone();
            return Box::pin(async move { validate_cookie(cookie.value(), &secret, &pool).await });
        }

        return Box::pin(ok(None));
    }

    fn to_response<B>(
        &self,
        identity: Option<String>,
        changed: bool,
        response: &mut ServiceResponse<B>,
    ) -> Self::ResponseFuture {
        if changed {
            if let Some(identity) = identity {
                // generate session
                let session: [u8; 16] = rand::random();

                // generate token
                let token = self.generate_token(&identity, &session);

                // add cookie containing JWT
                let cookie = Cookie::new("auth", token.clone());
                let val = HeaderValue::from_str(&cookie.to_string()).unwrap();
                response.headers_mut().append(header::SET_COOKIE, val);
                response.headers_mut().append(
                    HeaderName::from_static("auth"),
                    HeaderValue::from_str(&token)
                        .expect("token should always be a valid HeaderValue"),
                );

                // get database connection pool
                let pool = response
                    .request()
                    .app_data::<web::Data<AppData>>()
                    .expect("Failed to get AppData from request")
                    .pool
                    .clone();

                return Box::pin(async move {
                    // insert into db
                    sqlx::query!(
                        r#"
                            UPDATE local_users
                            SET session = $3
                            WHERE username = $1
                            AND host = $2
                        "#,
                        identity,
                        crate::host!(),
                        &session[..]
                    )
                    .execute(&pool)
                    .await
                    .map_err(|e| crate::Error::from(e))?;

                    Ok(())
                });
            }
        }

        //TODO: update last-seen time?
        Box::pin(ok(()))
    }
}
