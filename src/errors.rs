use rusoto_core::RusotoError;
use rusoto_s3::PutObjectError;
use sqlx;
use thiserror::Error;

/// Enumerates high-level errors returned by this library.
#[derive(Debug, Error)]
pub enum BackendError {
    /// Represents an SQL error.
    #[error("SQLx error")]
    SqlxError { source: sqlx::Error },
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
