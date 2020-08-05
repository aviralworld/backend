use std::sync::Arc;

use futures::future::{BoxFuture, FutureExt};
use rusoto_s3::{PutObjectRequest, S3Client, StreamingBody, S3};
use url::{ParseError, Url};

use crate::errors::BackendError;

pub mod mock;

pub trait Store: Send + Sync {
    /// The type of successful result.
    type Output;

    /// The type of raw data.
    type Raw;

    /// Saves the given data under the given key.
    fn save(&self, key: String, raw: Self::Raw) -> BoxFuture<Result<Self::Output, BackendError>>;

    fn get_url(&self, key: impl AsRef<str>) -> Result<Url, ParseError>;
}

/// A store that saves its data to S3.
pub struct S3Store {
    client: Arc<S3Client>,
    acl: String,
    bucket: String,
    cache_control: String,
    content_type: String,
    base_url: Url,
    extension: String,
}

impl S3Store {
    /// Creates a new instance.
    pub fn new(
        client: Arc<S3Client>,
        acl: String,
        bucket: String,
        cache_control: String,
        content_type: String,
        base_url: Url,
        extension: String,
    ) -> Self {
        Self {
            client,
            acl,
            bucket,
            cache_control,
            content_type,
            base_url,
            extension,
        }
    }
}

impl Store for S3Store {
    type Output = ();
    type Raw = Vec<u8>;

    fn save(&self, key: String, raw: Vec<u8>) -> BoxFuture<Result<(), BackendError>> {
        upload(self, key, raw).boxed()
    }

    fn get_url(&self, key: impl AsRef<str>) -> Result<Url, ParseError> {
        self.base_url
            .join(&format!("{}.{}", key.as_ref(), self.extension))
    }
}

async fn upload(store: &S3Store, key: String, raw: Vec<u8>) -> Result<(), BackendError> {
    use std::convert::TryFrom;

    let len = i64::try_from(raw.len()).expect("raw data length must be within range of i64");

    let request = PutObjectRequest {
        acl: Some(store.acl.clone()),
        body: Some(StreamingBody::from(raw)),
        bucket: store.bucket.clone(),
        cache_control: Some(store.cache_control.clone()),
        content_length: Some(len),
        content_type: Some(store.content_type.clone()),
        key,
        ..Default::default()
    };

    let result = store.client.put_object(request).await;

    match result {
        Ok(_) => Ok(()),
        Err(e) => Err(BackendError::UploadFailed { source: e }),
    }
}
