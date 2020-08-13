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
    fn save(&self, key: &Uuid, raw: Self::Raw) -> BoxFuture<Result<Self::Output, BackendError>>;
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
        let content_type = get_variable("S3_CONTENT_TYPE");
        let acl = get_variable("S3_ACL");
        let cache_control = get_variable("S3_CACHE_CONTROL");

        let client = Arc::new(S3Client::new_with(
            HttpClient::new()?,
            StaticProvider::new_minimal(access_key, secret_access_key),
            region,
        ));

        let base_url = Url::parse(&get_variable("S3_BASE_URL")).expect("parse S3_BASE_URL");
        let extension = get_variable("BACKEND_MEDIA_EXTENSION");

        Ok(S3Store::new(
            client,
            acl,
            bucket,
            cache_control,
            content_type,
            base_url,
            extension,
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
        self.base_url.join(&format!("{}.{}", key, self.extension))
    }

    fn save<'a>(&self, key: &Uuid, raw: Vec<u8>) -> BoxFuture<Result<(), BackendError>> {
        upload(self, *key, raw).boxed()
    }
}

async fn delete(store: &S3Store, key: Uuid) -> Result<(), BackendError> {
    let request = DeleteObjectRequest {
        bucket: store.bucket.clone(),
        key: filename(store, &key),
        ..Default::default()
    };

    let result = store.client.delete_object(request).await;

    result
        .map(|_| ())
        .map_err(|source| BackendError::DeleteFailed { source })
}

async fn upload(store: &S3Store, key: Uuid, raw: Vec<u8>) -> Result<(), BackendError> {
    use std::convert::TryFrom;

    let len = i64::try_from(raw.len()).expect("raw data length must be within range of i64");

    let request = PutObjectRequest {
        acl: Some(store.acl.clone()),
        body: Some(StreamingBody::from(raw)),
        bucket: store.bucket.clone(),
        cache_control: Some(store.cache_control.clone()),
        content_length: Some(len),
        content_type: Some(store.content_type.clone()),
        key: filename(store, &key),
        ..Default::default()
    };

    let result = store.client.put_object(request).await;

    match result {
        Ok(_) => Ok(()),
        Err(e) => Err(BackendError::UploadFailed { source: e }),
    }
}

fn filename(store: &S3Store, key: &Uuid) -> String {
    format!("{}.{}", key, store.extension)
}
