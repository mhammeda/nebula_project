use {
    crate::fed::client,
    actix_web::{http::StatusCode, HttpResponse},
    log::error,
    serde::Serialize,
};

/// Backend error
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Database error
    #[error("Error occured whilst executing query: {0:?}")]
    Database(#[from] sqlx::Error),

    /// Parse error
    #[error("Error occured whilst parsing: {0:?}")]
    Parse(anyhow::Error),

    /// Bad request
    #[error("Received bad request: {0:?}")]
    BadRequest(anyhow::Error),

    /// Client error
    #[error("Client error: {0:?}")]
    Client(client::Error),

    /// General error
    #[error("{0:?}")]
    General(anyhow::Error),
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Self::Parse(e.into())
    }
}

impl From<argon2::Error> for Error {
    fn from(e: argon2::Error) -> Self {
        Self::General(e.into())
    }
}

impl From<actix_multipart::MultipartError> for Error {
    fn from(e: actix_multipart::MultipartError) -> Self {
        Self::BadRequest(e.into())
    }
}

impl From<image::ImageError> for Error {
    fn from(e: image::ImageError) -> Self {
        Self::BadRequest(e.into())
    }
}

impl From<client::Error> for Error {
    fn from(e: client::Error) -> Self {
        Self::Client(e)
    }
}

#[derive(Debug, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ErrorBody {
    title: String,
    message: String,
}

impl From<&Error> for ErrorBody {
    fn from(e: &Error) -> Self {
        Self {
            title: match e {
                Error::Database(_) => "Database".to_owned(),
                Error::Parse(_) => "Parse".to_owned(),
                Error::BadRequest(_) => "Bad request".to_owned(),
                Error::Client(_) => "Client".to_owned(),
                Error::General(_) => "General".to_owned(),
            },
            message: format!("{}", e),
        }
    }
}

impl actix_web::error::ResponseError for Error {
    fn error_response(&self) -> HttpResponse {
        error!("{}", self);

        match self {
            Error::Database(sqlx::Error::RowNotFound) => {
                HttpResponse::NotFound().json(ErrorBody::from(self))
            }
            Error::BadRequest(_) => HttpResponse::BadRequest().json(ErrorBody::from(self)),
            _ => HttpResponse::InternalServerError().json(ErrorBody::from(self)),
        }
    }

    fn status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}
