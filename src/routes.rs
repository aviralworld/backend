use std::convert::Infallible;
use std::sync::Arc;

use futures::{
    future::{BoxFuture, FutureExt},
    StreamExt,
};
use serde::{Deserialize, Serialize};
use slog::{debug, error, info, Logger};
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
    store: Arc<impl Store<Output = O, Raw = Part> + 'a>,
) -> impl warp::Filter<Extract = (impl Reply,), Error = Infallible> + Clone + 'a {
    let store = store.clone();
    let logger1 = logger.clone();
    let logger2 = logger.clone();

    // TODO this should stream the body from the request, but warp
    // doesn't support that yet
    warp::path("recordings")
        .and(warp::post())
        .and(form().max_length(MAX_CONTENT_LENGTH))
        .and_then(
            move |content: FormData| -> BoxFuture<Result<WithStatus<Json>, reject::Rejection>> {
                debug!(logger1, "recording submitted");

                process_upload(logger1.clone(), store.clone(), content).boxed()
            },
        )
        .recover(move |r| format_rejection(logger2.clone(), r))
}

async fn process_upload<O>(
    logger: Arc<Logger>,
    store: Arc<impl Store<Output = O, Raw = Part>>,
    content: FormData,
) -> Result<WithStatus<Json>, reject::Rejection> {
    let mut parts = collect_parts(content).await?;
    debug!(logger, "collected parts");
    let upload = parse_parts(&mut parts).map_err(reject::custom)?;
    debug!(logger, "parsed parts");

    // TODO verify audio is Opus

    {
        let key = Uuid::new_v4();
        let key_as_str = format!("{}", key);
        let logger = logger.new(slog::o!("key" => key_as_str.clone()));
        debug!(logger, "generated key");

        store
            .save(key_as_str.clone(), upload.audio)
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
) -> Result<WithStatus<Json>, Infallible> {
    let mut code = StatusCode::INTERNAL_SERVER_ERROR;

    if let Some(e) = rej.find::<BackendError>() {
        error!(logger, "Backend error"; "error" => format!("{:?}", e));

        use BackendError::*;

        match e {
            BadRequest => code = StatusCode::BAD_REQUEST,
            PartsMissing => code = StatusCode::BAD_REQUEST,
            Sqlx { .. } => code = StatusCode::INTERNAL_SERVER_ERROR,
        }
    } else {
        error!(logger, "Unknown rejection"; "rejection" => format!("{:?}", rej));
    }

    let response = StorageResponse {
        status: Response::Error,
        key: None,
    };

    Ok(with_status(json(&response), code))
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
    use std::sync::RwLock;

    use futures::future::BoxFuture;
    use proptest::prelude::*;
    use serde::Deserialize;
    use warp::filters::multipart::Part;

    use crate::errors::StoreError;
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
        type Raw = Part;

        fn save(&self, key: String, raw: Part) -> BoxFuture<Result<(), StoreError>> {
            use futures::FutureExt;

            mock_save(&self, key, raw).boxed()
        }
    }

    async fn mock_save(store: &MockStore, key: String, raw: Part) -> Result<(), StoreError> {
        use bytes::Buf;
        use futures::StreamExt;

        let vec_of_results = raw.stream().collect::<Vec<_>>().await;
        let vec_of_bufs = vec_of_results
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let vec_of_vecs = vec_of_bufs
            .into_iter()
            .map(|b| b.bytes().to_vec())
            .collect::<Vec<_>>();

        store.map.write().unwrap().insert(key, vec_of_vecs.concat());

        Ok(())
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 50, ..ProptestConfig::default()
        })]

        // 500..100000 is the size range in bytes; I'd like to make it
        // 20 MB (20_971_520), but the tests get very slow
        #[test]
        fn uploading_things_works(data in prop::collection::vec(0u8..255u8, 500..100000), boundary in "-{5,20}[a-zA-Z0-9]{10,20}") {
            use std::borrow::Borrow;
            use std::sync::Arc;

            use futures::executor::block_on;
            use slog;

            let content_type = multipart_content_type(&boundary);

            let store = MockStore { ..Default::default() };

            let logger = slog::Logger::root(slog::Discard, slog::o!());
            let logger_arc = Arc::new(logger);

            let filter = super::make_upload_route(logger_arc.clone(), Arc::new(store));

            let request_body = make_multipart_body(boundary.as_bytes(), "{}".as_bytes(), &data);

            let response = block_on(warp::test::request()
                                    .path("/recordings/")
                                    .method("POST")
                                    .header("content-type", &content_type)
                                    .body(request_body)
                                    .reply(&filter));

            let status = response.status();
            let body = String::from_utf8_lossy(response.body());

            prop_assert!(status.is_success());

            let deserialized: Reply = serde_json::from_str(body.borrow())?;
            prop_assert_eq!(deserialized.status, "Ok", "response status must be okay");
            prop_assert!(deserialized.key.unwrap() != "", "response must provide non-blank key");
        }
    }

    const NEWLINE: &[u8] = "\r\n".as_bytes();
    const METADATA_HEADER: &[u8] =
        "Content-Disposition: form-data; name=\"metadata\"\r\n\r\n".as_bytes();
    const AUDIO_HEADER: &[u8] =
        "Content-Disposition: form-data; name=\"audio\"\r\nContent-Type: audio/ogg\r\n\r\n"
            .as_bytes();
    const BOUNDARY_LEADER: &[u8] = &[b'-', b'-'];

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
