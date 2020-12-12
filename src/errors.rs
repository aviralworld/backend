use std::io;

use rusoto_core::RusotoError;
use rusoto_s3::{DeleteObjectError, PutObjectError};
use thiserror::Error;
use uuid::Uuid;

use crate::audio::format;

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

    /// Represents an error caused by the user uploading a media file with too many streams.
    #[error("too many streams: should be {0}, was {1}")]
    TooManyStreams(usize, usize),

    /// Represents an error returned when parsing the content to upload.
    #[error("failed to parse form submission")]
    MalformedFormSubmission,

    /// Represents an error returned by the remote server when deleting.
    #[error("failed to delete object from storage")]
    DeleteFailed {
        source: RusotoError<DeleteObjectError>,
    },

    /// Represents an error returned by the remote server when uploading.
    #[error("failed to upload object to S3")]
    UploadFailed { source: RusotoError<PutObjectError> },

    /// Represents an error caused by an ID being reused.
    #[error("ID already exists in database")]
    IdAlreadyExists,

    /// Represents an error caused by a name being reused.
    #[error("name already exists in database")]
    NameAlreadyExists,

    /// Represents an error caused by the user providing an invalid ID.
    #[error("not a valid ID: {0}")]
    InvalidId(String),

    /// Represents an error caused by the user providing a non-existent ID.
    #[error("non-existent ID: {0}")]
    NonExistentId(Uuid),

    /// Represents an error caused by not being able to parse a URL
    /// already in the database.
    #[error("unable to parse URL {url}: {source}")]
    UnableToParseUrl {
        url: String,
        source: url::ParseError,
    },

    /// Represents an error caused by not being able to find a
    /// container & codec combination in the database.
    #[error("invalid audio format: {}/{}", format.container, format.codec)]
    InvalidAudioFormat { format: format::AudioFormat },

    /// Represents an error caused by not being able to recognize any
    /// audio format.
    #[error("unknown audio format")]
    UnrecognizedAudioFormat,
}
