use rusoto_core::RusotoError;
use rusoto_s3::PutObjectError;
use sqlx;
use thiserror::Error;
use warp::reject;

/// Enumerates high-level errors returned by this library.
#[derive(Debug, Error)]
pub enum BackendError {
    /// Represents an SQL error.
    #[error("SQLx error")]
    Sqlx { source: sqlx::Error },

    /// Represents an error with the request.
    #[error("Bad request")]
    BadRequest,

    /// Represents an error caused by missing parts in a form submission.
    #[error("Missing parts")]
    PartsMissing,
}

/// Enumerates errors returned by the store subsystem.
#[derive(Debug, Error)]
pub enum StoreError {
    /// Represents an error returned when parsing the content to upload.
    #[error("Content parsing error")]
    ContentParsingError,

    /// Represents an error returned by the remote server when uploading.
    #[error("Upload error")]
    UploadError { source: RusotoError<PutObjectError> },
}

impl reject::Reject for BackendError {}

impl From<BackendError> for reject::Rejection {
    fn from(e: BackendError) -> Self {
        warp::reject::custom(e)
    }
}

impl reject::Reject for StoreError {}

impl From<StoreError> for reject::Rejection {
    fn from(e: StoreError) -> Self {
        warp::reject::custom(e)
    }
}
