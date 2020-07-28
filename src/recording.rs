use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use url::Url;
use uuid::Uuid;

/// A single recording in the database.
#[derive(Clone, Debug, Deserialize)]
pub struct Recording {
    /// The ID of the recording.
    id: Uuid,

    #[serde(flatten)]
    metadata: RecordingMetadata,

    url: Url,
}

/// A single recording in the database before it's uploaded.
#[derive(Clone, Debug, Deserialize)]
pub struct NewRecording {
    /// The ID of the recording.
    id: Uuid,

    #[serde(flatten)]
    metadata: RecordingMetadata,
}

impl NewRecording {
    pub fn new(id: Uuid, metadata: RecordingMetadata) -> Self {
        NewRecording {
            id,
            metadata,
        }
    }

    pub fn id(&self) -> &Uuid {
        &self.id
    }

    pub fn metadata(&self) -> &RecordingMetadata {
        &self.metadata
    }
}

/// The metadata for a single recording.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RecordingMetadata {
    /// The ID of the age group provided.
    pub(crate) age: Option<Id>,

    /// The ID of the gender provided.
    pub(crate) gender: Option<Id>,

    /// The location provided (mapped to a Google Maps place name).
    pub(crate) location: Option<String>,

    /// The name provided. Must be unique after normalization.
    pub(crate) name: String,

    /// The occupation provided.
    pub(crate) occupation: Option<String>,

    /// The date and time it was created.
    pub(crate) created: OffsetDateTime,

    /// The ID of the recording it follows, if any.
    pub(crate) parent: Option<Uuid>,

    /// The ID of the category it falls into.
    pub(crate) category: Id,

    /// Whether this recording is hidden from public view.
    pub(crate) unlisted: bool,
}

/// An age group. The meaning is derived from configuration at
/// runtime.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AgeGroup(Id, String);

/// A gender. The meaning is derived from configuration at runtime.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Gender(Id, String);

/// A category. The meaning is derived from configuration at runtime.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Category(Id, String);

/// An ID in the database.
pub type Id = i8;
