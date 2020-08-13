use sqlx;
use thiserror::Error;

/// Enumerates all possible errors returned by this library.
#[derive(Debug, Error)]
pub enum BackendError {
    #[error("Recording already has ID")]
    RecordingAlreadyHasId,

    #[error("SQLX error")]
    SqlxError { source: sqlx::Error },
}
