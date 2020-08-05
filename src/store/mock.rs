use std::collections::HashMap;
use std::sync::RwLock;

use futures::future::BoxFuture;
use url::{ParseError, Url};

use crate::errors;
use crate::store::Store;

#[derive(Default)]
pub(crate) struct MockStore {
    pub(crate) map: RwLock<HashMap<String, Vec<u8>>>,
    extension: String,
}

impl MockStore {
    pub fn new(extension: impl AsRef<str>) -> Self {
        MockStore {
            extension: extension.as_ref().to_owned(),
            ..Default::default()
        }
    }
}

impl Store for MockStore {
    type Output = ();
    type Raw = Vec<u8>;

    fn save(&self, key: String, raw: Vec<u8>) -> BoxFuture<Result<(), errors::StoreError>> {
        use futures::FutureExt;

        mock_save(&self, key, raw).boxed()
    }

    fn get_url(&self, key: impl AsRef<str>) -> Result<Url, ParseError> {
        Url::parse(&format!("https://www.example.com/{}.{}", key.as_ref(), self.extension))
    }
}

async fn mock_save(store: &MockStore, key: String, raw: Vec<u8>) -> Result<(), errors::StoreError> {
    store.map.write().unwrap().insert(key, raw);

    Ok(())
}
