use std::future::Future;
use std::sync::Arc;

use bytes::Buf;
use futures::{StreamExt};
use futures::future::{BoxFuture, FutureExt};
use rusoto_s3::{PutObjectRequest, S3Client, S3, StreamingBody};
use warp::filters::multipart::Part;

use crate::errors::StoreError;

pub trait Store: Send + Sync {
    /// The type of successful result.
    type Output;

    /// The type of raw data.
    type Raw;

    /// Saves the given data under the given key.
    fn save(&self, key: String, raw: Self::Raw) -> BoxFuture<Result<Self::Output, StoreError>>;
}

/// A store that saves its data to S3.
pub struct S3Store {
    client: Arc<S3Client>,
    acl: String,
    bucket: String,
    cache_control: String,
    content_type: String,
}

impl S3Store {
    /// Creates a new instance.
    pub fn new(client: Arc<S3Client>, acl: String, bucket: String, cache_control: String, content_type: String) -> Self {
        Self {
            client,
            acl,
            bucket,
            cache_control,
            content_type,
        }
    }

    /// Parses raw data into format required by S3. Unused while we
    /// wait on <https://github.com/rusoto/rusoto/issues/1592>.
    fn parse_part_into_body(&self, raw: Part) -> Result<StreamingBody, ()> {
        use std::io;
        let body = StreamingBody::new(raw.stream().map(|r| {
            r.map(|mut x| x.to_bytes())
                .map_err(|_| io::Error::new(io::ErrorKind::Other, "could not retrieve chunk"))
        }));
        Ok(body)
    }
}

impl Store for S3Store {
    type Output = ();
    type Raw = Part;

    fn save(&self, key: String, raw: Part) -> BoxFuture<Result<(), StoreError>> {
        upload(self, key, raw).boxed()
    }
}

async fn upload(store: &S3Store, key: String, raw: Part) -> Result<(), StoreError> {
    use std::io;

    // TODO we'd like to pass the stream itself to the
    // PutObjectRequest, but an oversight in the library makes it
    // omit the Content-Length header in that case, which causes
    // S3 to reject it.
    let stream = raw.stream().map(|r| {
        r.map(|mut x| x.to_bytes())
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "could not retrieve chunk"))
    });

    let full_body = stream.collect::<Vec<_>>().await;

    let mut total: i64 = 0;

    for r in full_body.iter() {
        total += r.as_ref().map_err(|_| StoreError::ContentParsingError)?.remaining() as i64;
    }

    let request = PutObjectRequest {
        acl: Some(store.acl.clone()),
        body: Some(StreamingBody::new(futures::stream::iter(full_body))),
        bucket: store.bucket.clone(),
        cache_control: Some(store.cache_control.clone()),
        content_length: Some(total),
        content_type: Some(store.content_type.clone()),
        key,
        ..Default::default()
    };

    let result = store.client.put_object(request).await;

    match result {
        Ok(_) => Ok(()),
        Err(x) => Err(StoreError::UploadError { source: x }),
    }
}
