use serde::Serialize;
use warp::reject;

use crate::errors::BackendError;

#[derive(Debug)]
pub struct Rejection {
    pub(crate) context: Context,
    pub(crate) error: BackendError,
}

impl Rejection {
    pub fn new(context: Context, error: BackendError) -> Self {
        Rejection { context, error }
    }

    pub fn flatten(&self) -> FlattenedRejection {
        FlattenedRejection {
            context: self.context.clone(),
            message: format!("{}", self.error),
        }
    }
}

impl reject::Reject for Rejection {}

#[derive(Debug, Serialize)]
pub struct FlattenedRejection {
    #[serde(flatten)]
    pub(crate) context: Context,
    pub(crate) message: String,
}

impl From<Rejection> for reject::Rejection {
    fn from(e: Rejection) -> Self {
        reject::custom(e)
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum Context {
    Children { parent: String },
    Count,
    Delete { id: String },
    Hide { id: String },
    Retrieve { id: String },
    Upload { id: Option<String> },
}

impl Context {
    pub fn children(parent: String) -> Context {
        Context::Children { parent }
    }

    pub fn count() -> Context {
        Context::Count
    }

    pub fn delete(id: String) -> Context {
        Context::Delete { id }
    }

    pub fn hide(id: String) -> Context {
        Context::Hide { id }
    }

    pub fn retrieve(id: String) -> Context {
        Context::Retrieve { id }
    }

    pub fn upload(id: Option<String>) -> Context {
        Context::Upload { id }
    }
}
