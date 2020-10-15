use std::sync::Arc;

use futures::future::{BoxFuture, FutureExt};
use rusoto_s3::{DeleteObjectRequest, PutObjectRequest, S3Client, StreamingBody, S3};
use url::{ParseError, Url};
use uuid::Uuid;

use crate::errors::BackendError;

pub trait Store: Send + Sync {
    /// The type of successful result.
    type Output;

    /// The type of raw data.
    type Raw;

    /// Deletes the given object.
    fn delete(&self, key: &Uuid) -> BoxFuture<Result<(), BackendError>>;

    /// Gets the URL for the given object.
    fn get_url(&self, key: &Uuid) -> Result<Url, ParseError>;

    /// Saves the given data under the given key.
    fn save(&self, key: &Uuid, content_type: String, raw: Self::Raw) -> BoxFuture<Result<Self::Output, BackendError>>;
}

/// A store that saves its data to S3.
pub struct S3Store {
    client: Arc<S3Client>,
    acl: String,
    bucket: String,
    cache_control: String,
    base_url: Url,
}

impl S3Store {
    /// Creates a new instance.
    pub fn new(
        client: Arc<S3Client>,
        acl: String,
        bucket: String,
        cache_control: String,
        base_url: Url,
    ) -> Self {
        Self {
            client,
            acl,
            bucket,
            cache_control,
            base_url,
        }
    }

    pub fn from_env() -> Result<Self, rusoto_core::request::TlsError> {
        use rusoto_core::request::HttpClient;
        use rusoto_core::Region;
        use rusoto_credential::StaticProvider;

        use crate::config::get_variable;

        let access_key = get_variable("S3_ACCESS_KEY");
        let secret_access_key = get_variable("S3_SECRET_ACCESS_KEY");

        let region = Region::Custom {
            name: get_variable("S3_REGION_NAME"),
            endpoint: get_variable("S3_ENDPOINT"),
        };

        let bucket = get_variable("S3_BUCKET_NAME");
        let acl = get_variable("BACKEND_S3_ACL");
        let cache_control = get_variable("BACKEND_S3_CACHE_CONTROL");

        let client = Arc::new(S3Client::new_with(
            HttpClient::new()?,
            StaticProvider::new_minimal(access_key, secret_access_key),
            region,
        ));

        let base_url = Url::parse(&get_variable("S3_BASE_URL")).expect("parse S3_BASE_URL");

        Ok(S3Store::new(
            client,
            acl,
            bucket,
            cache_control,
            base_url,
        ))
    }
}

impl Store for S3Store {
    type Output = ();
    type Raw = Vec<u8>;

    fn delete<'a>(&self, key: &'a Uuid) -> BoxFuture<Result<(), BackendError>> {
        delete(self, *key).boxed()
    }

    fn get_url<'a>(&self, key: &'a Uuid) -> Result<Url, ParseError> {
        self.base_url.join(&key.to_string())
    }

    fn save<'a>(&self, key: &Uuid, content_type: String, raw: Vec<u8>) -> BoxFuture<Result<(), BackendError>> {
        upload(self, *key, content_type, raw).boxed()
    }
}

async fn delete(store: &S3Store, key: Uuid) -> Result<(), BackendError> {
    let request = DeleteObjectRequest {
        bucket: store.bucket.clone(),
        key: key.to_string(),
        ..Default::default()
    };

    let result = store.client.delete_object(request).await;

    result
        .map(|_| ())
        .map_err(|source| BackendError::DeleteFailed { source })
}

async fn upload(store: &S3Store, key: Uuid, content_type: String, raw: Vec<u8>) -> Result<(), BackendError> {
    use std::convert::TryFrom;

    let len = i64::try_from(raw.len()).expect("raw data length must be within range of i64");

    let request = PutObjectRequest {
        acl: Some(store.acl.clone()),
        body: Some(StreamingBody::from(raw)),
        bucket: store.bucket.clone(),
        cache_control: Some(store.cache_control.clone()),
        content_length: Some(len),
        content_type: Some(content_type),
        key: key.to_string(),
        ..Default::default()
    };

    let result = store.client.put_object(request).await;

    match result {
        Ok(_) => Ok(()),
        Err(e) => Err(BackendError::UploadFailed { source: e }),
    }
}
