use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use url::Url;
use uuid::Uuid;

/// A single recording in the database.
#[derive(Clone, Debug, Deserialize)]
pub struct Recording {
    /// The ID of the recording.
    id: Uuid,

    /// The URL of the file.
    url: Url,

    /// The times it was created and updated.
    #[serde(flatten)]
    times: Times,

    /// The user-submitted metadata.
    #[serde(flatten)]
    metadata: RecordingMetadata,
}

/// A single recording in the database before it's uploaded.
#[derive(Clone, Debug, Deserialize)]
pub struct NewRecording {
    /// The ID of the recording.
    id: Uuid,

    /// The times it was created and updated.
    #[serde(flatten)]
    times: Times,

    /// The user-submitted metadata.
    #[serde(flatten)]
    metadata: RecordingMetadata,
}

impl NewRecording {
    pub fn new(id: Uuid, created_at: OffsetDateTime, updated_at: OffsetDateTime, metadata: RecordingMetadata) -> Self {
        NewRecording {
            id,
            metadata,
            times: Times {
                created_at,
                updated_at,
            },
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
    pub(crate) age_id: Option<Id>,

    /// The ID of the gender provided.
    pub(crate) gender_id: Option<Id>,

    /// The location provided (mapped to a Google Maps place name).
    pub(crate) location: Option<String>,

    /// The name provided. Must be unique after normalization.
    pub(crate) name: String,

    /// The occupation provided.
    pub(crate) occupation: Option<String>,

    /// The ID of the recording it follows, if any.
    pub(crate) parent_id: Option<Uuid>,

    /// The ID of the category it falls into.
    pub(crate) category_id: Id,

    /// Whether this recording is hidden from public view.
    pub(crate) unlisted: bool,
}

/// A single recording in the database.
#[derive(Clone, Debug, Deserialize)]
pub struct Times {
    /// The date and time it was created.
    pub(crate) created_at: OffsetDateTime,

    /// The date and time it was last modified.
    pub(crate) updated_at: OffsetDateTime,
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
pub type Id = i16;
