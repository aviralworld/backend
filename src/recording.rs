use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use url::Url;
use uuid::Uuid;

use crate::normalization;

#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum Recording {
    Active(ActiveRecording),
    Deleted(DeletedRecording),
}

/// A single active recording in the database.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ActiveRecording {
    /// The ID of the recording.
    id: Uuid,

    /// The URL of the file.
    url: Url,

    /// The times it was created and updated.
    #[serde(flatten)]
    times: Times,

    /// The category it falls into.
    category: Label,

    /// Whether this recording is hidden from public view.
    unlisted: bool,

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
    pub fn new(
        id: Uuid,
        times: Times,
        name: String,
        parent: Option<Uuid>,
        url: Url,
        category: Label,
        gender: Option<Label>,
        age: Option<Label>,
        location: Option<String>,
        occupation: Option<String>,
        unlisted: bool,
    ) -> Self {
        ActiveRecording {
            id,
            name,
            times,
            parent,
            url,
            category,
            gender,
            age,
            location,
            occupation,
            unlisted,
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

    /// The ID of the recording it follows, if any.
    pub(crate) parent_id: Option<Uuid>,

    /// The ID of the category it falls into.
    pub(crate) category_id: Id,

    /// Whether this recording is hidden from public view.
    pub(crate) unlisted: bool,
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
    pub(crate) created_at: OffsetDateTime,

    /// The date and time it was last modified.
    pub(crate) updated_at: OffsetDateTime,
}

/// A label for a choice. The meaning is derived from configuration at
/// runtime.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Label(Id, String);

impl Label {
    pub fn new(id: Id, label: String) -> Self {
        Label(id, label)
    }
}

/// An ID in the database.
pub type Id = i16;
