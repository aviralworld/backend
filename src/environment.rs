use std::sync::Arc;

use slog::Logger;

use crate::{db::Db, audio::format::AudioFormat};
use crate::errors::BackendError;
use crate::store::Store;
use crate::urls::Urls;

pub type Checker = dyn Fn(&[u8]) -> Result<AudioFormat, BackendError> + Send + Sync;
pub type VecStore<O> = dyn Store<Output = O, Raw = Vec<u8>> + Send + Sync;

#[derive(Clone)]
pub struct Environment<O: Clone + Send + Sync> {
    pub logger: Arc<Logger>,
    pub db: Arc<dyn Db + Send + Sync>,
    pub urls: Arc<Urls>,
    pub store: Arc<VecStore<O>>,
    pub checker: Arc<dyn Fn(&[u8]) -> Result<AudioFormat, BackendError> + Send + Sync>,
}

impl<O: Clone + Send + Sync> Environment<O> {
    pub fn new(
        logger: Arc<Logger>,
        db: Arc<dyn Db + Send + Sync>,
        urls: Arc<Urls>,
        store: Arc<VecStore<O>>,
        checker: Arc<Checker>,
    ) -> Self {
        Self {
            logger,
            db,
            urls,
            store,
            checker,
        }
    }
}
