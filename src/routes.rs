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

// create: use warp::filters::body::stream

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

                process_upload(
                    logger1.clone(),
                    store.clone(),
                    content,
                )
                .boxed()
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
        let logger = logger.new(slog::o!("key" => key_as_str.to_owned()));
        debug!(logger, "generated key");

        store.save(key_as_str, upload.audio).await
            .map_err(|x| {
                error!(logger, "Failed to save"; "error" => format!("{:?}", x));
                x
            })?;

        debug!(logger, "saved object");

        let response = StorageResponse {
            status: Response::Ok,
        };

        Ok(with_status(json(&response), StatusCode::OK))
    }
}

async fn format_rejection(logger: Arc<Logger>, rej: reject::Rejection) -> Result<WithStatus<Json>, Infallible> {
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

struct Upload {
    audio: Part,
    metadata: Part,
}

#[derive(Deserialize, Serialize)]
struct StorageResponse {
    status: Response,
}

#[derive(Deserialize, Serialize)]
enum Response {
    Ok,
    Error,
}
