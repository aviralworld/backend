use std::sync::Arc;

use slog::Logger;

use crate::db::Db;
use crate::errors::BackendError;
use crate::store::Store;
use crate::urls::Urls;

#[derive(Clone)]
pub struct Environment<O: Clone + Send + Sync> {
    pub logger: Arc<Logger>,
    pub db: Arc<dyn Db + Send + Sync>,
    pub urls: Arc<Urls>,
    pub store: Arc<dyn Store<Output = O, Raw = Vec<u8>> + Send + Sync>,
    pub checker: Arc<dyn Fn(&[u8]) -> Result<(), BackendError> + Send + Sync>,
}

impl<O: Clone + Send + Sync> Environment<O> {
    pub fn new(
        logger: Arc<Logger>,
        db: Arc<dyn Db + Send + Sync>,
        urls: Arc<Urls>,
        store: Arc<dyn Store<Output = O, Raw = Vec<u8>> + Send + Sync>,
        checker: Arc<dyn Fn(&[u8]) -> Result<(), BackendError> + Send + Sync>,
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
