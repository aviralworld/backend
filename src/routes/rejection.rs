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
    Ages,
    Categories,
    Children { parent: String },
    Count,
    Delete { id: String },
    Formats,
    Genders,
    Hide { id: String },
    Random { count: i16 },
    Retrieve { id: String },
    Token { id: String },
    Upload { id: Option<String> },
}

impl Context {
    pub fn ages() -> Context {
        Context::Ages
    }

    pub fn categories() -> Context {
        Context::Categories
    }

    pub fn children(parent: String) -> Context {
        Context::Children { parent }
    }

    pub fn count() -> Context {
        Context::Count
    }

    pub fn delete(id: String) -> Context {
        Context::Delete { id }
    }

    pub fn formats() -> Context {
        Context::Formats
    }

    pub fn genders() -> Context {
        Context::Genders
    }

    pub fn hide(id: String) -> Context {
        Context::Hide { id }
    }

    pub fn random(count: i16) -> Context {
        Context::Random { count }
    }

    pub fn retrieve(id: String) -> Context {
        Context::Retrieve { id }
    }

    pub fn token(id: String) -> Context {
        Context::Token { id }
    }

    pub fn upload(id: Option<String>) -> Context {
        Context::Upload { id }
    }
}
