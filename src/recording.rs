use time::OffsetDateTime;
use uuid::Uuid;

/// The metadata for a single recording.
#[derive(Debug, Clone)]
pub struct Recording {
    /// The ID of the recording.
    id: Uuid,

    /// The age group provided.
    age: Option<AgeGroup>,

    /// The gender provided.
    gender: Option<Gender>,

    /// The location provided (mapped to a Google Maps place name).
    location: Option<String>,

    /// The name provided. Must be unique after normalization.
    name: String,

    /// The occupation provided.
    occupation: Option<String>,

    /// The date and time it was created.
    created: OffsetDateTime,

    /// The ID of the recording it follows, if any.
    parent: Option<Id>,

    /// The category it falls into.
    category: Category,

    /// Whether this recording is hidden from public view.
    unlisted: bool,
}

/// An age group. The meaning is derived from configuration at
/// runtime.
#[derive(Debug, Clone)]
pub struct AgeGroup(Id, String);

/// A gender. The meaning is derived from configuration at runtime.
#[derive(Debug, Clone)]
pub struct Gender(Id, String);

/// A category. The meaning is derived from configuration at runtime.
#[derive(Debug, Clone)]
pub struct Category(Id, String);

/// An ID in the database. This is not numeric data.
#[derive(Debug, Clone)]
pub struct Id(String);
