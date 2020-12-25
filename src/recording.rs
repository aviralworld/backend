use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use url::Url;
use uuid::Uuid;

use crate::label::{Id, Label};
use crate::normalization;

#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum Recording {
    Active(ActiveRecording),
    Deleted(DeletedRecording),
}

/// A minimal version of an active recording in the database.
#[derive(Clone, Debug, Serialize)]
pub struct PartialRecording {
    /// The ID of the recording.
    id: Uuid,

    /// The name provided.
    name: String,

    /// The location provided, if any.
    location: Option<String>,
}

impl PartialRecording {
    pub fn new(id: Uuid, name: String, location: Option<String>) -> Self {
        Self { id, name, location }
    }
}

/// A single active recording in the database.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ActiveRecording {
    /// The ID of the recording.
    id: Uuid,

    /// The URL of the file.
    url: Url,

    /// The MIME type of the file.
    mime_type: Label,

    /// The times it was created and updated.
    #[serde(flatten)]
    times: Times,

    /// The category it falls into.
    category: Label,

    /// The ID of the recording it follows, if any.
    parent: Option<Uuid>,

    /// The name provided. Must be unique after normalization.
    name: String,

    /// The age group provided.
    age: Option<Label>,

    /// The gender provided.
    gender: Option<Label>,

    /// The location provided (mapped to a Google Maps place name).
    location: Option<String>,

    /// The occupation provided.
    occupation: Option<String>,
}

impl ActiveRecording {
    // TODO revisit whether we can get around the lint
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: Uuid,
        times: Times,
        name: String,
        parent: Option<Uuid>,
        url: Url,
        mime_type: Label,
        category: Label,
        gender: Option<Label>,
        age: Option<Label>,
        location: Option<String>,
        occupation: Option<String>,
    ) -> Self {
        ActiveRecording {
            id,
            name,
            times,
            parent,
            url,
            mime_type,
            category,
            gender,
            age,
            location,
            occupation,
        }
    }
}

/// A single recording deleted from the database.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DeletedRecording {
    /// The ID of the recording.
    id: Uuid,

    /// The times it was created and updated.
    #[serde(flatten)]
    times: Times,

    /// The time it was deleted.
    #[serde(skip_serializing)]
    deleted_at: OffsetDateTime,

    /// The ID of the recording it follows, if any.
    parent: Option<Uuid>,
}

impl DeletedRecording {
    pub fn new(id: Uuid, times: Times, deleted_at: OffsetDateTime, parent: Option<Uuid>) -> Self {
        DeletedRecording {
            id,
            times,
            deleted_at,
            parent,
        }
    }
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
    metadata: UploadMetadata,
}

impl NewRecording {
    pub fn new(
        id: Uuid,
        created_at: OffsetDateTime,
        updated_at: OffsetDateTime,
        metadata: UploadMetadata,
    ) -> Self {
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

    pub fn metadata(&self) -> &UploadMetadata {
        &self.metadata
    }
}

/// The metadata for a single recording.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UploadMetadata {
    /// The ID of the age group provided.
    pub(crate) age_id: Option<Id>,

    /// The ID of the gender provided.
    pub(crate) gender_id: Option<Id>,

    /// The location provided (mapped to a Google Maps place name).
    #[serde(default)]
    #[serde(deserialize_with = "normalization::deserialize_option")]
    pub(crate) location: Option<String>,

    /// The name provided. Must be unique after normalization.
    #[serde(deserialize_with = "normalization::deserialize")]
    pub(crate) name: String,

    /// The occupation provided.
    #[serde(default)]
    #[serde(deserialize_with = "normalization::deserialize_option")]
    pub(crate) occupation: Option<String>,

    /// The token using which this recording was created.
    pub(crate) token: Uuid,

    /// The ID of the category it falls into.
    pub(crate) category_id: Id,
}

/// A simplified view of a recording that follows another.
#[derive(Clone, Debug, Deserialize, sqlx::FromRow, Serialize)]
pub struct ChildRecording {
    /// The ID of the recording.
    id: Uuid,

    /// The name provided.
    name: String,
}

/// A single recording in the database.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Times {
    /// The date and time it was created.
    #[serde(with = "time::serde::timestamp")]
    pub(crate) created_at: OffsetDateTime,

    /// The date and time it was last modified.
    #[serde(with = "time::serde::timestamp")]
    pub(crate) updated_at: OffsetDateTime,
}

/// A token to create a new recording.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RecordingToken {
    /// The ID of the token.
    pub(crate) id: Uuid,

    /// The ID of the parent recording.
    pub(crate) parent_id: Uuid,
}

impl RecordingToken {
    pub fn new(id: Uuid, parent_id: Uuid) -> Self {
        Self { id, parent_id }
    }
}
