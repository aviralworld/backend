use std::sync::Arc;

use bytes::Buf;
use futures::{StreamExt};
use futures::future::{BoxFuture, FutureExt};
use rusoto_s3::{PutObjectRequest, S3Client, S3, StreamingBody};
use warp::filters::multipart::Part;

pub trait Store: Send + Sync {
    /// The type of error.
    type Error;

    /// The type of successful result.
    type Output;

    /// The type of raw data.
    type Raw;

    /// Saves the given data under the given key.
    fn save(&self, key: impl AsRef<str>, raw: Self::Raw) -> BoxFuture<Result<Self::Output, Self::Error>>;
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

    /// Parses raw data into format required by S3.
    fn parse_raw_into_body(&self, raw: Part) -> Result<StreamingBody, ()> {
        use std::io;
        let body = StreamingBody::new(raw.stream().map(|r| {
            r.map(|mut x| x.to_bytes())
                .map_err(|_| io::Error::new(io::ErrorKind::Other, "could not retrieve chunk"))
        }));
        Ok(body)
    }
}

impl Store for S3Store {
    type Error = ();
    type Output = ();
    type Raw = Part;

    fn save(&self, key: impl AsRef<str>, raw: Part) -> BoxFuture<Result<(), ()>> {
        let parsed = self.parse_raw_into_body(raw);

        match parsed {
            Ok(body) => {

                let request = PutObjectRequest {
                    acl: Some(self.acl.clone()),
                    body: Some(body),
                    bucket: self.bucket.clone(),
                    cache_control: Some(self.cache_control.clone()),
                    content_type: Some(self.content_type.clone()),
                    key: key.as_ref().to_owned(),
                    ..Default::default()
                };

                self.client.put_object(request).map(|r| match r {
                    Ok(_) => Ok(()),
                    Err(_) => Err(()),
                }).boxed()
            },
            Err(_) => futures::future::err(()).boxed(),
        }
    }
}
