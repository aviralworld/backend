use serde::Serialize;
use uuid::Uuid;

use crate::recording::{ChildRecording, PartialRecording};

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum SuccessResponse<'a> {
    Children {
        parent: String,
        children: Vec<ChildRecording>,
    },
    Count(i64),
    Healthz {
        revision: Option<&'a str>,
        timestamp: Option<&'a str>,
        version: &'a str,
    },
    Lookup {
        id: Uuid,
        tokens: Vec<Uuid>,
    },
    Random {
        recordings: Vec<PartialRecording>,
    },
    Token {
        id: String,
        parent_id: String,
    },
    Upload {
        id: String,
        // TODO these should not be options
        tokens: Option<Vec<Uuid>>,
        key: Option<Uuid>,
    },
}
