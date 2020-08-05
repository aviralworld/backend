use std::io;

use rusoto_core::RusotoError;
use rusoto_s3::PutObjectError;
use serde_json;
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
    #[error("bad request")]
    BadRequest,

    /// Represents an error generating a URL for an uploaded object.
    #[error("couldn't generate URL")]
    FailedToGenerateUrl { source: url::ParseError },

    /// Represents an error caused by missing parts in a form submission.
    #[error("missing parts in upload")]
    PartsMissing,

    /// Represents an error creating or writing to a temporary file.
    #[error("temporary file error")]
    TemporaryFileError(io::Error),

    /// Represents an error running `ffprobe`.
    #[error("error running `ffprobe`")]
    FfprobeFailed(io::Error),

    /// Represents an error caused by `ffprobe` returning malformed JSON.
    #[error("failed to parse JSON received from `ffprobe`: {0}")]
    MalformedFfprobeOutput(serde_json::Error),

    /// Represents an error caused by the user uploading malformed metadata.
    #[error("failed to parse uploaded metadata: {0}")]
    MalformedUploadMetadata(serde_json::Error),

    /// Represents an error caused by the user uploading a media file of the wrong kind.
    #[error("wrong media type (should be {expected_codec} inside {expected_format}; was {actual_codec} inside {actual_format})")]
    WrongMediaType {
        actual_codec: String,
        expected_codec: String,
        actual_format: String,
        expected_format: String,
    },

    /// Represents an error caused by the user uploading a media file with too many streams.
    #[error("too many streams: should be {0}, was {1}")]
    TooManyStreams(usize, usize),

    /// Represents an error returned when parsing the content to upload.
    #[error("failed to parse form submission")]
    MalformedFormSubmission,

    /// Represents an error returned by the remote server when uploading.
    #[error("failed to upload object to S3")]
    UploadFailed { source: RusotoError<PutObjectError> },
}

impl reject::Reject for BackendError {}

impl From<BackendError> for reject::Rejection {
    fn from(e: BackendError) -> Self {
        warp::reject::custom(e)
    }
}
