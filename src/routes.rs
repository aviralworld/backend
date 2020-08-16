use std::sync::Arc;

use futures::future::{BoxFuture, FutureExt};
use serde::Serialize;
use slog::{debug, error, Logger};
use url::Url;
use uuid::Uuid;
use warp::filters::multipart::{form, FormData, Part};
use warp::http::StatusCode;
use warp::reject;
use warp::reply::{json, with_header, with_status, Json, Reply, WithHeader, WithStatus};
use warp::Filter;

use crate::db::Db;
use crate::errors::BackendError;
use crate::io::parse_upload;
use crate::recording::{ChildRecording, UploadMetadata};
use crate::store::Store;
use crate::urls::Urls;

mod rejection;

// TODO count

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum SuccessResponse {
    Children {
        parent: String,
        children: Vec<ChildRecording>,
    },
    Upload {
        id: String,
    },
    Count(i64),
}

/// The maximum form data size to accept. This should be enforced by the HTTP gateway, so on the Rust side it’s set to an unreasonably large number.
const MAX_CONTENT_LENGTH: u64 = 2 * 1024 * 1024 * 1024;

// the filters can be simplified once async closures are stabilized
// (rust/rust-lang#62290) and `impl Trait` can be used with closures;
// in the mean time, we have to use `BoxFuture` and forward to real
// `async fn`s if we want to use `async`/`await`

// TODO accept environment as single `Environment` struct (causes all
// sorts of reference and lifetime issues)

pub fn make_count_route<'a>(
    logger: Arc<Logger>,
    db: Arc<impl Db + Sync + Send + 'a>,
    urls: Arc<Urls>,
) -> impl warp::Filter<Extract = (impl Reply,), Error = reject::Rejection> + Clone + 'a {
    let logger1 = logger.clone();
    let logger2 = logger.clone();

    let recordings_path = urls.recordings_path.clone();

    warp::path(recordings_path)
        .and(warp::path("count"))
        .and(warp::path::end())
        .and(warp::get())
        .and_then(
            move || -> BoxFuture<Result<Json, reject::Rejection>> {
                get_recording_count(logger1.clone(), db.clone()).boxed()
            },
        )
        .recover(move |r| format_rejection(logger2.clone(), r))
}

pub fn make_upload_route<'a, O: 'a>(
    logger: Arc<Logger>,
    db: Arc<impl Db + Sync + Send + 'a>,
    store: Arc<impl Store<Output = O, Raw = Vec<u8>> + 'a>,
    checker: Arc<impl Fn(&[u8]) -> Result<(), BackendError> + Send + Sync + 'a>,
    urls: Arc<Urls>,
) -> impl warp::Filter<Extract = (impl Reply,), Error = reject::Rejection> + Clone + 'a {
    let logger1 = logger.clone();
    let logger2 = logger.clone();

    // TODO this should stream the body from the request, but warp
    // doesn't support that yet; however, see
    // <https://github.com/cetra3/mpart-async>
    warp::path(urls.recordings_path.clone())
        .and(warp::path::end())
        .and(warp::post())
        .and(form().max_length(MAX_CONTENT_LENGTH))
        .and_then(
            move |content: FormData| -> BoxFuture<Result<WithHeader<WithStatus<Json>>, reject::Rejection>> {
                process_upload(
                    logger1.clone(),
                    db.clone(),
                    store.clone(),
                    checker.clone(),
                    urls.clone(),
                    content,
                )
                .boxed()
            },
        )
        .recover(move |r| format_rejection(logger2.clone(), r))
}

pub fn make_children_route<'a>(
    logger: Arc<Logger>,
    db: Arc<impl Db + Sync + Send + 'a>,
    urls: Arc<Urls>,
) -> impl warp::Filter<Extract = (impl Reply,), Error = reject::Rejection> + Clone + 'a {
    let logger1 = logger.clone();
    let logger2 = logger.clone();

    let recordings_path = urls.recordings_path.clone();

    warp::path(recordings_path)
        .and(warp::path!("id" / String / "children"))
        .and(warp::path::end())
        .and(warp::get())
        .and_then(
            move |parent| -> BoxFuture<Result<WithStatus<Json>, reject::Rejection>> {
                get_children(logger1.clone(), db.clone(), parent).boxed()
            },
        )
        .recover(move |r| format_rejection(logger2.clone(), r))
}

pub fn make_delete_route<'a, O: 'a>(
    logger: Arc<Logger>,
    db: Arc<impl Db + Sync + Send + 'a>,
    store: Arc<impl Store<Output = O, Raw = Vec<u8>> + 'a>,
    urls: Arc<Urls>,
) -> impl warp::Filter<Extract = (impl Reply,), Error = reject::Rejection> + Clone + 'a {
    let logger1 = logger.clone();
    let logger2 = logger.clone();

    let recordings_path = urls.recordings_path.clone();

    warp::path(recordings_path)
        .and(warp::path("id"))
        .and(warp::path::param::<String>())
        .and(warp::path::end())
        .and(warp::delete())
        .and_then(
            move |id| -> BoxFuture<Result<StatusCode, reject::Rejection>> {
                delete_recording(logger1.clone(), db.clone(), store.clone(), id).boxed()
            },
        )
        .recover(move |r| format_rejection(logger2.clone(), r))
}

pub fn make_retrieve_route<'a>(
    logger: Arc<Logger>,
    db: Arc<impl Db + Sync + Send + 'a>,
    urls: Arc<Urls>,
) -> impl warp::Filter<Extract = (impl Reply,), Error = reject::Rejection> + Clone + 'a {
    let logger1 = logger.clone();
    let logger2 = logger.clone();

    let recordings_path = urls.recordings_path.clone();

    warp::path(recordings_path)
        .and(warp::path("id"))
        .and(warp::path::param::<String>())
        .and(warp::path::end())
        .and(warp::get())
        .and_then(
            move |id| -> BoxFuture<Result<WithStatus<Json>, reject::Rejection>> {
                retrieve_recording(logger1.clone(), db.clone(), id).boxed()
            },
        )
        .recover(move |r| format_rejection(logger2.clone(), r))
}

pub fn make_hide_route<'a>(
    logger: Arc<Logger>,
    db: Arc<impl Db + Sync + Send + 'a>,
    urls: Arc<Urls>,
) -> impl warp::Filter<Extract = (impl Reply,), Error = reject::Rejection> + Clone + 'a {
    let logger1 = logger.clone();
    let logger2 = logger.clone();

    let recordings_path = urls.recordings_path.clone();

    warp::path(recordings_path)
        .and(warp::path("id"))
        .and(warp::path::param::<String>())
        .and(warp::path("hide"))
        .and(warp::path::end())
        .and(warp::post())
        .and_then(
            move |id| -> BoxFuture<Result<StatusCode, reject::Rejection>> {
                hide_recording(logger1.clone(), db.clone(), id).boxed()
            },
        )
        .recover(move |r| format_rejection(logger2.clone(), r))
}

async fn process_upload<O>(
    logger: Arc<Logger>,
    db: Arc<impl Db>,
    store: Arc<impl Store<Output = O, Raw = Vec<u8>>>,
    checker: Arc<impl Fn(&[u8]) -> Result<(), BackendError>>,
    urls: Arc<Urls>,
    content: FormData,
) -> Result<WithHeader<WithStatus<Json>>, reject::Rejection> {
    let error_handler =
        |e: BackendError| rejection::Rejection::new(rejection::Context::upload(None), e);

    debug!(logger, "Parsing submission...");
    let upload = parse_upload(content).await.map_err(error_handler)?;
    debug!(logger, "Verifying audio contents...");
    let verified_audio = verify_audio(logger.clone(), checker, upload.audio)
        .await
        .map_err(&error_handler)?;

    // TODO retry in case ID already exists
    debug!(logger, "Writing metadata to database...");
    let id = save_recording_metadata(logger.clone(), db.clone(), upload.metadata)
        .await
        .map_err(&error_handler)?;
    let id_as_str = format!("{}", id);
    let logger = Arc::new(logger.new(slog::o!("id" => id_as_str.clone())));

    let error_handler = |e: BackendError| {
        // TODO delete row from DB
        rejection::Rejection::new(rejection::Context::upload(Some(id_as_str.clone())), e)
    };

    // should this punt to a queue? is that necessary?
    debug!(logger, "Saving recording to store...");
    save_upload_audio(logger.clone(), store.clone(), &id, verified_audio)
        .await
        .map_err(&error_handler)?;

    debug!(logger, "Updating recording URL...");
    update_recording_url(logger.clone(), db.clone(), store.clone(), &id)
        .await
        .map_err(&error_handler)?;

    debug!(logger, "Sending response...");
    let response = SuccessResponse::Upload {
        id: id_as_str.clone(),
    };

    Ok(with_header(
        with_status(json(&response), StatusCode::CREATED),
        "location",
        urls.recording(&id).as_str(),
    ))
}

async fn get_children(
    logger: Arc<Logger>,
    db: Arc<impl Db>,
    parent: String,
) -> Result<WithStatus<Json>, reject::Rejection> {
    let error_handler = |e: BackendError| {
        rejection::Rejection::new(rejection::Context::children(parent.clone()), e)
    };

    let id = Uuid::parse_str(&parent)
        .map_err(|_| BackendError::InvalidId(parent.clone()))
        .map_err(error_handler)?;
    debug!(logger, "Searching for children..."; "parent" => &parent.to_string());

    let children = db.children(&id).await.map_err(error_handler)?;
    let response = SuccessResponse::Children { parent, children };

    Ok(with_status(json(&response), StatusCode::OK))
}

async fn delete_recording<O>(
    logger: Arc<Logger>,
    db: Arc<impl Db>,
    store: Arc<impl Store<Output = O, Raw = Vec<u8>>>,
    id: String,
) -> Result<StatusCode, reject::Rejection> {
    let error_handler =
        |e: BackendError| rejection::Rejection::new(rejection::Context::delete(id.clone()), e);

    let id = Uuid::parse_str(&id)
        .map_err(|_| BackendError::InvalidId(id.clone()))
        .map_err(error_handler)?;
    debug!(logger, "Deleting recording..."; "id" => format!("{}", &id));

    store.delete(&id).await.map_err(error_handler)?;
    db.delete(&id).await.map_err(error_handler)?;

    Ok(StatusCode::NO_CONTENT)
}

async fn retrieve_recording(
    logger: Arc<Logger>,
    db: Arc<impl Db>,
    id: String,
) -> Result<WithStatus<Json>, reject::Rejection> {
    use crate::recording::Recording;

    let error_handler =
        |e: BackendError| rejection::Rejection::new(rejection::Context::retrieve(id.clone()), e);

    let id = Uuid::parse_str(&id)
        .map_err(|_| BackendError::InvalidId(id.clone()))
        .map_err(error_handler)?;
    debug!(logger, "Retrieving recording..."; "id" => format!("{}", &id));

    let option = db.retrieve(&id).await.map_err(error_handler)?;

    match option {
        Some(recording) => {
            let status = match recording {
                Recording::Active(_) => StatusCode::OK,
                Recording::Deleted(_) => StatusCode::GONE,
            };

            Ok(with_status(json(&recording), status))
        }
        None => Ok(with_status(json(&()), StatusCode::NOT_FOUND)),
    }
}

async fn hide_recording(
    logger: Arc<Logger>,
    db: Arc<impl Db>,
    id: String,
) -> Result<StatusCode, reject::Rejection> {
    let error_handler =
        |e: BackendError| rejection::Rejection::new(rejection::Context::hide(id.clone()), e);

    let id = Uuid::parse_str(&id)
        .map_err(|_| BackendError::InvalidId(id.clone()))
        .map_err(error_handler)?;
    debug!(logger, "Hiding recording..."; "id" => format!("{}", &id));

    db.hide(&id).await.map_err(error_handler)?;

    Ok(StatusCode::NO_CONTENT)
}

async fn get_recording_count(
    _logger: Arc<Logger>,
    db: Arc<impl Db>) -> Result<Json, reject::Rejection> {
    let count = db.count_all().await.map_err(|e: BackendError| rejection::Rejection::new(rejection::Context::count(), e))?;

    Ok(json(&SuccessResponse::Count(count)))
}

async fn verify_audio(
    _logger: Arc<Logger>,
    checker: Arc<impl Fn(&[u8]) -> Result<(), BackendError>>,
    audio: Part,
) -> Result<Vec<u8>, BackendError> {
    use crate::io;

    let audio_data = io::part_as_vec(audio)
        .await
        .map_err(|_| BackendError::MalformedFormSubmission)?;

    checker(&audio_data)?;

    Ok(audio_data)
}

async fn save_recording_metadata(
    _logger: Arc<Logger>,
    db: Arc<impl Db>,
    metadata: Part,
) -> Result<Uuid, BackendError> {
    use crate::io;

    let raw_metadata = io::part_as_vec(metadata)
        .await
        .map_err(|_| BackendError::MalformedFormSubmission)?;
    let metadata: UploadMetadata = serde_json::from_slice(&raw_metadata)
        .map_err(BackendError::MalformedUploadMetadata)?;

    let new_recording = db.insert(metadata).await?;
    let id = new_recording.id();

    Ok(*id)
}

async fn save_upload_audio<O>(
    _logger: Arc<Logger>,
    store: Arc<impl Store<Output = O, Raw = Vec<u8>>>,
    key: &Uuid,
    upload: Vec<u8>,
) -> Result<(), BackendError> {
    store.save(key, upload).await?;

    Ok(())
}

async fn update_recording_url<O>(
    _logger: Arc<Logger>,
    db: Arc<impl Db>,
    store: Arc<impl Store<Output = O, Raw = Vec<u8>>>,
    key: &Uuid,
) -> Result<Url, BackendError> {
    let url = store
        .get_url(&key)
        .map_err(|e| BackendError::FailedToGenerateUrl { source: e })?;

    db.update_url(key, &url).await?;

    Ok(url)
}

async fn format_rejection(
    logger: Arc<Logger>,
    rej: reject::Rejection,
) -> Result<WithStatus<Json>, reject::Rejection> {
    if let Some(r) = rej.find::<rejection::Rejection>() {
        error!(logger, "Backend error"; "error" => format!("{:?}", r.error));
        let e = &r.error;
        let flattened = r.flatten();

        return Ok(with_status(json(&flattened), status_code_for(e)));
    }

    Err(rej)
}

fn status_code_for(e: &BackendError) -> StatusCode {
    use BackendError::*;

    match e {
        BadRequest | TooManyStreams(..) => StatusCode::BAD_REQUEST,
        WrongMediaType { .. } => StatusCode::UNSUPPORTED_MEDIA_TYPE,
        PartsMissing => StatusCode::BAD_REQUEST,
        NameAlreadyExists => StatusCode::FORBIDDEN,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
