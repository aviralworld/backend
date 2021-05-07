use serde::Deserialize;

#[derive(Deserialize)]
pub struct AvailabilityQuery {
    pub name: String,
}
