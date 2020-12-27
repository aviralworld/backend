use serde::{Deserialize, Serialize};

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
