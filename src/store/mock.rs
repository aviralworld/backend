use std::collections::HashMap;
use std::sync::RwLock;

use futures::future::BoxFuture;

use crate::errors;
use crate::store::Store;

#[derive(Default)]
pub struct MockStore {
    map: RwLock<HashMap<String, Vec<u8>>>,
}

impl Store for MockStore {
    type Output = ();
    type Raw = Vec<u8>;

    fn save(&self, key: String, raw: Vec<u8>) -> BoxFuture<Result<(), errors::StoreError>> {
        use futures::FutureExt;

        mock_save(&self, key, raw).boxed()
    }
}

async fn mock_save(store: &MockStore, key: String, raw: Vec<u8>) -> Result<(), errors::StoreError> {
    store.map.write().unwrap().insert(key, raw);

    Ok(())
}
