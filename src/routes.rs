use std::sync::Arc;

use futures::{
    future::{BoxFuture, FutureExt},
    StreamExt,
};
use serde::{Deserialize, Serialize};
use slog::{debug, error, Logger};
// use sqlx::prelude::*;
// use sqlx::postgres::PgPool;
use uuid::Uuid;
use warp::filters::multipart::{form, FormData, Part};
use warp::http::StatusCode;
use warp::reject;
use warp::reply::{json, with_status, Json, Reply, WithStatus};
use warp::Filter;

use crate::errors::{BackendError, StoreError};
use crate::queries::retrieval;
use crate::store::Store;

// create, delete, update, retrieve, count

struct Upload {
    audio: Part,
    metadata: Part,
}

#[derive(Deserialize, Serialize)]
struct StorageResponse {
    status: Response,
    key: Option<String>,
}

#[derive(Deserialize, Serialize)]
enum Response {
    Ok,
    Error,
}

/// The maximum form data size to accept. This should be enforced by the HTTP gateway, so on the Rust side it’s set to an unreasonably large number.
const MAX_CONTENT_LENGTH: u64 = 2 * 1024 * 1024 * 1024;

pub fn make_upload_route<'a, O: 'a>(
    logger: Arc<Logger>,
    store: Arc<impl Store<Output = O, Raw = Vec<u8>> + 'a>,
    checker: Arc<impl Fn(&[u8]) -> Result<(), BackendError> + Send + Sync + 'a>,
) -> impl warp::Filter<Extract = (impl Reply,), Error = reject::Rejection> + Clone + 'a {
    let store = store.clone();
    let logger1 = logger.clone();
    let logger2 = logger.clone();
    let checker = checker.clone();

    // TODO this should stream the body from the request, but warp
    // doesn't support that yet
    warp::path("recordings")
        .and(warp::post())
        .and(form().max_length(MAX_CONTENT_LENGTH))
        .and_then(
            move |content: FormData| -> BoxFuture<Result<WithStatus<Json>, reject::Rejection>> {
                debug!(logger1, "recording submitted");

                process_upload(logger1.clone(), store.clone(), checker.clone(), content).boxed()
            },
        )
        .recover(move |r| format_rejection(logger2.clone(), r))
}

async fn process_upload<O>(
    logger: Arc<Logger>,
    store: Arc<impl Store<Output = O, Raw = Vec<u8>>>,
    checker: Arc<impl Fn(&[u8]) -> Result<(), BackendError>>,
    content: FormData,
) -> Result<WithStatus<Json>, reject::Rejection> {
    use crate::io;

    let mut parts = collect_parts(content).await?;
    debug!(logger, "collected parts");
    let upload = parse_parts(&mut parts).map_err(reject::custom)?;
    debug!(logger, "parsed parts");

    let audio_data = io::part_as_stream(upload.audio)
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| reject::custom(StoreError::MalformedFormSubmission))?
        .concat();

    checker(&audio_data).map_err(reject::custom)?;

    {
        let key = Uuid::new_v4();
        let key_as_str = format!("{}", key);
        let logger = logger.new(slog::o!("key" => key_as_str.clone()));
        debug!(logger, "generated key");

        store
            .save(key_as_str.clone(), audio_data)
            .await
            .map_err(|x| {
                error!(logger, "Failed to save"; "error" => format!("{:?}", x));
                x
            })?;

        debug!(logger, "saved object");

        let response = StorageResponse {
            status: Response::Ok,
            key: Some(key_as_str.clone()),
        };

        Ok(with_status(json(&response), StatusCode::OK))
    }
}

async fn format_rejection(
    logger: Arc<Logger>,
    rej: reject::Rejection,
) -> Result<WithStatus<Json>, reject::Rejection> {
    if let Some(e) = rej.find::<BackendError>() {
        error!(logger, "Backend error"; "error" => format!("{:?}", e));
        let response = StorageResponse {
            status: Response::Error,
            key: None,
        };

        return Ok(with_status(json(&response), status_code_for(e)));
    }

    Err(rej)
}

fn status_code_for(e: &BackendError) -> StatusCode {
    use BackendError::*;

    match e {
        BadRequest | TooManyStreams(_, _) | WrongMediaType(_) => StatusCode::BAD_REQUEST,
        PartsMissing => StatusCode::BAD_REQUEST,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

async fn collect_parts(content: FormData) -> Result<Vec<Part>, BackendError> {
    let parts = (content.collect::<Vec<Result<Part, _>>>()).await;
    let vec = parts
        .into_iter()
        .collect::<Result<Vec<Part>, _>>()
        // TODO this should be a more specific error
        .map_err(|_| BackendError::BadRequest)?;
    Ok(vec)
}

fn parse_parts(parts: &mut Vec<Part>) -> Result<Upload, BackendError> {
    let mut audio = None;
    let mut metadata = None;

    for p in parts.drain(0..) {
        let name = p.name().to_owned();

        if name == "audio" {
            audio = Some(p);
        } else if name == "metadata" {
            metadata = Some(p);
        }
    }

    if metadata.is_none() || audio.is_none() {
        // TODO this should be a more specific error
        return Err(BackendError::PartsMissing);
    }

    Ok(Upload {
        audio: audio.unwrap(),
        metadata: metadata.unwrap(),
    })
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;
    use std::path::Path;
    use std::sync::RwLock;

    use futures::future::BoxFuture;
    use serde::Deserialize;

    use crate::errors;
    use crate::store::Store;

    #[derive(Debug, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Reply {
        status: String,
        key: Option<String>,
    }

    #[derive(Default)]
    struct MockStore {
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

    async fn mock_save(
        store: &MockStore,
        key: String,
        raw: Vec<u8>,
    ) -> Result<(), errors::StoreError> {
        store.map.write().unwrap().insert(key, raw);

        Ok(())
    }

    #[test]
    fn uploading_works() {
        use std::borrow::Borrow;
        use std::env;
        use std::path::Path;
        use std::sync::Arc;

        use futures::executor::block_on;
        use slog;

        let boundary = "thisisaboundary1234";

        let content_type = multipart_content_type(&boundary);

        let store = MockStore {
            ..Default::default()
        };

        let logger = slog::Logger::root(slog::Discard, slog::o!());
        let logger_arc = Arc::new(logger);

        let checker = make_wrapper_for_test();

        let filter =
            super::make_upload_route(logger_arc.clone(), Arc::new(store), Arc::new(checker));

        let cargo_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
        let base_path = Path::new(&cargo_dir);
        let file_path = base_path.join("tests").join("opus_file.ogg");

        let response = block_on(
            upload_file(
                &file_path,
                &content_type,
                boundary.as_bytes(),
                "{}".as_bytes(),
            )
            .reply(&filter),
        );

        let status = response.status();
        let body = String::from_utf8_lossy(response.body());

        assert!(status.is_success());

        let deserialized: Reply =
            serde_json::from_str(body.borrow()).expect("parse response as JSON");
        assert_eq!(deserialized.status, "Ok", "response status must be okay");
        assert!(
            deserialized.key.unwrap() != "",
            "response must provide non-blank key"
        );
    }

    #[test]
    fn bad_requests_fail() {
        use bytes::Bytes;

        fn assert_failed(
            response: warp::http::Response<Bytes>,
            expected_status: u16,
            verify_error_type: &dyn Fn(StatusCode) -> bool,
        ) {
            let status = response.status();
            eprintln!("Got status: {:?}", status);
            eprintln!("Body: {:?}", response.body());
            assert!(verify_error_type(status));
            assert_eq!(status.as_u16(), expected_status);
        }

        use std::sync::Arc;
        use warp::http::StatusCode;

        use futures::executor::block_on;
        use slog;

        let store = MockStore {
            ..Default::default()
        };

        let logger = slog::Logger::root(slog::Discard, slog::o!());
        let logger_arc = Arc::new(logger);

        let checker = make_wrapper_for_test();

        let filter =
            super::make_upload_route(logger_arc.clone(), Arc::new(store), Arc::new(checker));

        {
            let response = block_on(
                warp::test::request()
                    .path("/recordings/")
                    .method("POST")
                    .header("content-type", "text/plain")
                    .header("content-length", 0)
                    .reply(&filter),
            );

            assert_failed(response, 400, &|s: StatusCode| s.is_client_error());
        }
    }

    const NEWLINE: &[u8] = "\r\n".as_bytes();
    const METADATA_HEADER: &[u8] =
        "Content-Disposition: form-data; name=\"metadata\"\r\n\r\n".as_bytes();
    const AUDIO_HEADER: &[u8] =
        "Content-Disposition: form-data; name=\"audio\"\r\nContent-Type: audio/ogg\r\n\r\n"
            .as_bytes();
    const BOUNDARY_LEADER: &[u8] = &[b'-', b'-'];

    fn upload_file(
        path: impl AsRef<Path>,
        content_type: &str,
        boundary: &[u8],
        metadata: &[u8],
    ) -> warp::test::RequestBuilder {
        use std::fs;

        let data =
            fs::read(path.as_ref()).expect(&format!("read file {:?}", path.as_ref().display()));
        let body = make_multipart_body(boundary, metadata, &data);

        warp::test::request()
            .path("/recordings/")
            .method("POST")
            .header("content-type", content_type)
            .header("content-length", body.len())
            .body(body)
    }

    fn make_wrapper_for_test() -> impl Fn(&[u8]) -> Result<(), errors::BackendError> {
        use crate::audio;
        use std::env;

        audio::make_wrapper(
            env::var("BACKEND_FFPROBE_PATH").ok(),
            env::var("BACKEND_CODEC").expect("must define BACKEND_CODEC environment variable"),
        )
    }

    fn make_multipart_body(boundary: &[u8], metadata: &[u8], content: &[u8]) -> Vec<u8> {
        let boundary = boundary_with_leader(boundary);
        let boundary = boundary.as_slice();

        let mut parts = vec![
            boundary,
            NEWLINE,
            METADATA_HEADER,
            metadata,
            NEWLINE,
            boundary,
            NEWLINE,
        ];

        parts.push(AUDIO_HEADER);
        parts.push(&content);
        parts.push(NEWLINE);
        parts.push(boundary);
        parts.push("--".as_bytes());
        parts.push(NEWLINE);

        parts.concat()
    }

    fn boundary_with_leader(boundary: &[u8]) -> Vec<u8> {
        let parts = &[BOUNDARY_LEADER, boundary];
        parts.concat()
    }

    fn multipart_content_type(boundary: &str) -> String {
        format!("multipart/form-data; boundary={}", boundary)
    }
}
