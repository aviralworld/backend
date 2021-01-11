use serde::{Deserialize, Serialize, Serializer};

/// A label for a choice. The meaning is derived from configuration at
/// runtime.
#[derive(Clone, Debug, Deserialize)]
pub struct Label {
    pub(crate) id: Id,
    pub(crate) label: String,
    pub(crate) description: Option<String>,
}

impl Label {
    pub fn new(id: Id, label: String, description: Option<String>) -> Self {
        Label {
            id,
            label,
            description,
        }
    }
}

/// An ID in the database.
pub type Id = i16;

impl Serialize for Label {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeSeq;

        let mut state = serializer.serialize_seq(Some(3))?;
        state.serialize_element(&self.id)?;
        state.serialize_element(&self.label)?;
        state.serialize_element(&self.description)?;
        state.end()
    }
}
