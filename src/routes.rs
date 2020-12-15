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

use crate::environment::Environment;
use crate::errors::BackendError;
use crate::io::parse_upload;
use crate::recording::{ChildRecording, UploadMetadata};
use crate::{audio::format::AudioFormat, db::Db, environment, mime_type::MimeType};

mod rejection;

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

pub fn make_formats_route<'a, O: Clone + Send + Sync + 'a>(
    environment: Environment<O>,
) -> impl warp::Filter<Extract = (impl Reply,), Error = reject::Rejection> + Clone + 'a {
    let recordings_path = environment.urls.recordings_path.clone();
    let logger = environment.logger.clone();

    // TODO make this cacheable
    warp::path(recordings_path)
        .and(warp::path("formats"))
        .and(warp::path::end())
        .and(warp::get())
        .and_then(move || -> BoxFuture<Result<Json, reject::Rejection>> {
            get_formats(environment.clone()).boxed()
        })
        .recover(move |r| format_rejection(logger.clone(), r))
}

pub fn make_ages_list_route<'a, O: Clone + Send + Sync + 'a>(
    environment: Environment<O>,
) -> impl warp::Filter<Extract = (impl Reply,), Error = reject::Rejection> + Clone + 'a {
    let recordings_path = environment.urls.recordings_path.clone();
    let logger = environment.logger.clone();

    // TODO make this cacheable
    warp::path(recordings_path)
        .and(warp::path("ages"))
        .and(warp::path::end())
        .and(warp::get())
        .and_then(move || -> BoxFuture<Result<Json, reject::Rejection>> {
            get_ages(environment.clone()).boxed()
        })
        .recover(move |r| format_rejection(logger.clone(), r))
}

pub fn make_categories_list_route<'a, O: Clone + Send + Sync + 'a>(
    environment: Environment<O>,
) -> impl warp::Filter<Extract = (impl Reply,), Error = reject::Rejection> + Clone + 'a {
    let recordings_path = environment.urls.recordings_path.clone();
    let logger = environment.logger.clone();

    // TODO make this cacheable
    warp::path(recordings_path)
        .and(warp::path("categories"))
        .and(warp::path::end())
        .and(warp::get())
        .and_then(move || -> BoxFuture<Result<Json, reject::Rejection>> {
            get_categories(environment.clone()).boxed()
        })
        .recover(move |r| format_rejection(logger.clone(), r))
}

pub fn make_genders_list_route<'a, O: Clone + Send + Sync + 'a>(
    environment: Environment<O>,
) -> impl warp::Filter<Extract = (impl Reply,), Error = reject::Rejection> + Clone + 'a {
    let recordings_path = environment.urls.recordings_path.clone();
    let logger = environment.logger.clone();

    // TODO make this cacheable
    warp::path(recordings_path)
        .and(warp::path("genders"))
        .and(warp::path::end())
        .and(warp::get())
        .and_then(move || -> BoxFuture<Result<Json, reject::Rejection>> {
            get_genders(environment.clone()).boxed()
        })
        .recover(move |r| format_rejection(logger.clone(), r))
}

pub fn make_count_route<'a, O: Clone + Send + Sync + 'a>(
    environment: Environment<O>,
) -> impl warp::Filter<Extract = (impl Reply,), Error = reject::Rejection> + Clone + 'a {
    let recordings_path = environment.urls.recordings_path.clone();
    let logger = environment.logger.clone();

    warp::path(recordings_path)
        .and(warp::path("count"))
        .and(warp::path::end())
        .and(warp::get())
        .and_then(move || -> BoxFuture<Result<Json, reject::Rejection>> {
            get_recording_count(environment.clone()).boxed()
        })
        .recover(move |r| format_rejection(logger.clone(), r))
}

pub fn make_upload_route<'a, O: Clone + Send + Sync + 'a>(
    environment: Environment<O>,
) -> impl warp::Filter<Extract = (impl Reply,), Error = reject::Rejection> + Clone + 'a {
    let logger = environment.logger.clone();

    // TODO this should stream the body from the request, but warp
    // doesn't support that yet; however, see
    // <https://github.com/cetra3/mpart-async>
    warp::path(environment.urls.recordings_path.clone())
        .and(warp::path::end())
        .and(warp::post())
        .and(form().max_length(MAX_CONTENT_LENGTH))
        .and_then(
            move |content: FormData| -> BoxFuture<Result<WithHeader<WithStatus<Json>>, reject::Rejection>> {
                process_upload(
                    environment.clone(),
                    content,
                )
                .boxed()
            },
        )
        .recover(move |r| format_rejection(logger.clone(), r))
}

pub fn make_children_route<'a, O: Clone + Send + Sync + 'a>(
    environment: Environment<O>,
) -> impl warp::Filter<Extract = (impl Reply,), Error = reject::Rejection> + Clone + 'a {
    let logger = environment.logger.clone();

    let recordings_path = environment.urls.recordings_path.clone();

    warp::path(recordings_path)
        .and(warp::path!("id" / String / "children"))
        .and(warp::path::end())
        .and(warp::get())
        .and_then(
            move |parent| -> BoxFuture<Result<WithStatus<Json>, reject::Rejection>> {
                get_children(environment.clone(), parent).boxed()
            },
        )
        .recover(move |r| format_rejection(logger.clone(), r))
}

pub fn make_delete_route<'a, O: Clone + Send + Sync + 'a>(
    environment: Environment<O>,
) -> impl warp::Filter<Extract = (impl Reply,), Error = reject::Rejection> + Clone + 'a {
    let logger = environment.logger.clone();

    let recordings_path = environment.urls.recordings_path.clone();

    warp::path(recordings_path)
        .and(warp::path("id"))
        .and(warp::path::param::<String>())
        .and(warp::path::end())
        .and(warp::delete())
        .and_then(
            move |id| -> BoxFuture<Result<StatusCode, reject::Rejection>> {
                delete_recording(environment.clone(), id).boxed()
            },
        )
        .recover(move |r| format_rejection(logger.clone(), r))
}

pub fn make_retrieve_route<'a, O: Clone + Send + Sync + 'a>(
    environment: Environment<O>,
) -> impl warp::Filter<Extract = (impl Reply,), Error = reject::Rejection> + Clone + 'a {
    let logger = environment.logger.clone();

    let recordings_path = environment.urls.recordings_path.clone();

    warp::path(recordings_path)
        .and(warp::path("id"))
        .and(warp::path::param::<String>())
        .and(warp::path::end())
        .and(warp::get())
        .and_then(
            move |id| -> BoxFuture<Result<WithStatus<Json>, reject::Rejection>> {
                retrieve_recording(environment.clone(), id).boxed()
            },
        )
        .recover(move |r| format_rejection(logger.clone(), r))
}

async fn get_formats<O: Clone + Send + Sync>(
    environment: Environment<O>,
) -> Result<Json, reject::Rejection> {
    let formats = environment
        .db
        .retrieve_format_essences()
        .await
        .map_err(|e: BackendError| rejection::Rejection::new(rejection::Context::formats(), e))?;

    Ok(json(&formats))
}

async fn get_ages<O: Clone + Send + Sync>(
    environment: Environment<O>,
) -> Result<Json, reject::Rejection> {
    let ages = environment
        .db
        .retrieve_ages()
        .await
        .map_err(|e: BackendError| rejection::Rejection::new(rejection::Context::ages(), e))?;

    Ok(json(&ages))
}

async fn get_categories<O: Clone + Send + Sync>(
    environment: Environment<O>,
) -> Result<Json, reject::Rejection> {
    let categories = environment
        .db
        .retrieve_categories()
        .await
        .map_err(|e: BackendError| {
            rejection::Rejection::new(rejection::Context::categories(), e)
        })?;

    Ok(json(&categories))
}

async fn get_genders<O: Clone + Send + Sync>(
    environment: Environment<O>,
) -> Result<Json, reject::Rejection> {
    let genders = environment
        .db
        .retrieve_genders()
        .await
        .map_err(|e: BackendError| rejection::Rejection::new(rejection::Context::genders(), e))?;

    Ok(json(&genders))
}

async fn get_recording_count<O: Clone + Send + Sync>(
    environment: Environment<O>,
) -> Result<Json, reject::Rejection> {
    let count = environment
        .db
        .count_all()
        .await
        .map_err(|e: BackendError| rejection::Rejection::new(rejection::Context::count(), e))?;

    Ok(json(&SuccessResponse::Count(count)))
}

async fn process_upload<O: Clone + Send + Sync>(
    environment: Environment<O>,
    content: FormData,
) -> Result<WithHeader<WithStatus<Json>>, reject::Rejection> {
    use slog::o;

    let Environment {
        logger,
        store,
        db,
        checker,
        urls,
        ..
    } = environment;
    let checker = checker.clone();

    let error_handler =
        |e: BackendError| rejection::Rejection::new(rejection::Context::upload(None), e);

    debug!(logger, "Parsing submission...");
    let upload = parse_upload(content).await.map_err(error_handler)?;

    let metadata = parse_recording_metadata(logger.clone(), upload.metadata)
        .await
        .map_err(error_handler)?;

    let token = metadata.token.clone();

    let logger = Arc::new(logger.new(o!("token" => format!("{}", token.clone()))));

    debug!(logger, "Locking token...");
    let parent_id = lock_token(logger.clone(), db.clone(), token.clone())
        .await
        .map_err(error_handler)?;

    let error_handler = |e: BackendError| {
        // first spawn a task to release the token, logging any
        // errors, then go back to normal error handling
        let logger = logger.clone();
        let db = db.clone();
        let token = token.clone();

        tokio::spawn(async move {
            release_token(logger.clone(), db.clone(), token.clone())
                .await
                .map_err(|e| {
                    error!(logger, "Failed to release token: {}", e);
                })
        });

        error_handler(e)
    };

    debug!(logger, "Verifying audio contents...");
    let (verified_audio, audio_format) = verify_audio(logger.clone(), checker, upload.audio)
        .await
        .map_err(&error_handler)?;

    // TODO retry in case ID already exists
    debug!(logger, "Writing metadata to database...");
    let id = save_recording_metadata(logger.clone(), db.clone(), &parent_id, metadata)
        .await
        .map_err(&error_handler)?;
    let id_as_str = format!("{}", id);
    let logger = Arc::new(logger.new(o!("id" => id_as_str.clone())));

    let error_handler = |e: BackendError| {
        // TODO delete row from DB
        rejection::Rejection::new(rejection::Context::upload(Some(id_as_str.clone())), e)
    };

    // should this punt to a queue? is that necessary?
    debug!(logger, "Saving recording to store...");
    let mime_type = db
        .retrieve_mime_type(&audio_format)
        .await
        .map_err(&error_handler)?;

    match mime_type {
        Some(mime_type) => {
            save_upload_audio(
                logger.clone(),
                store.clone(),
                &id,
                mime_type.essence.clone(),
                verified_audio,
            )
            .await
            .map_err(&error_handler)?;

            debug!(logger, "Updating recording URL...");
            update_recording_url(logger.clone(), db.clone(), store.clone(), &id, mime_type)
                .await
                .map_err(&error_handler)?;

            debug!(logger, "Removing token...");
            remove_token(logger.clone(), db.clone(), token.clone())
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
        // why does this work but not directly returning `Err(error_handler(BackendError::...))`?
        None => Err(BackendError::InvalidAudioFormat {
            format: audio_format,
        })
        .map_err(&error_handler)?,
    }
}

async fn get_children<O: Clone + Send + Sync>(
    environment: Environment<O>,
    parent: String,
) -> Result<WithStatus<Json>, reject::Rejection> {
    let error_handler = |e: BackendError| {
        rejection::Rejection::new(rejection::Context::children(parent.clone()), e)
    };

    let id = Uuid::parse_str(&parent)
        .map_err(|_| BackendError::InvalidId(parent.clone()))
        .map_err(error_handler)?;
    debug!(environment.logger, "Searching for children..."; "parent" => &parent.to_string());

    let children = environment.db.children(&id).await.map_err(error_handler)?;
    let response = SuccessResponse::Children { parent, children };

    Ok(with_status(json(&response), StatusCode::OK))
}

async fn delete_recording<O: Clone + Send + Sync>(
    environment: Environment<O>,
    id: String,
) -> Result<StatusCode, reject::Rejection> {
    let error_handler =
        |e: BackendError| rejection::Rejection::new(rejection::Context::delete(id.clone()), e);

    let id = Uuid::parse_str(&id)
        .map_err(|_| BackendError::InvalidId(id.clone()))
        .map_err(error_handler)?;
    debug!(environment.logger, "Deleting recording..."; "id" => format!("{}", &id));

    environment.store.delete(&id).await.map_err(error_handler)?;
    environment.db.delete(&id).await.map_err(error_handler)?;

    Ok(StatusCode::NO_CONTENT)
}

async fn retrieve_recording<O: Clone + Send + Sync>(
    environment: Environment<O>,
    id: String,
) -> Result<WithStatus<Json>, reject::Rejection> {
    use crate::recording::Recording;

    let error_handler =
        |e: BackendError| rejection::Rejection::new(rejection::Context::retrieve(id.clone()), e);

    let id = Uuid::parse_str(&id)
        .map_err(|_| BackendError::InvalidId(id.clone()))
        .map_err(error_handler)?;
    debug!(environment.logger, "Retrieving recording..."; "id" => format!("{}", &id));

    let option = environment.db.retrieve(&id).await.map_err(error_handler)?;

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

async fn parse_recording_metadata(
    _logger: Arc<Logger>,
    part: Part,
) -> Result<UploadMetadata, BackendError> {
    use crate::io;

    let raw_metadata = io::part_as_vec(part)
        .await
        .map_err(|_| BackendError::MalformedFormSubmission)?;

    let upload_metadata: UploadMetadata =
        serde_json::from_slice(&raw_metadata).map_err(BackendError::MalformedUploadMetadata)?;

    Ok(upload_metadata)
}

async fn lock_token(
    _logger: Arc<Logger>,
    db: Arc<dyn Db + Send + Sync>,
    token: Uuid,
) -> Result<Uuid, BackendError> {
    let parent_id = db
        .lock_token(&token)
        .await
        .map_err(|_| BackendError::InvalidToken { token })?;

    parent_id.ok_or_else(|| BackendError::InvalidToken { token })
}

async fn release_token(
    _logger: Arc<Logger>,
    db: Arc<dyn Db + Send + Sync>,
    token: Uuid,
) -> Result<(), BackendError> {
    db.release_token(&token).await
}

async fn verify_audio(
    _logger: Arc<Logger>,
    checker: Arc<environment::Checker>,
    audio: Part,
) -> Result<(Vec<u8>, AudioFormat), BackendError> {
    use crate::io;

    let audio_data = io::part_as_vec(audio)
        .await
        .map_err(|_| BackendError::MalformedFormSubmission)?;

    // always use the first format
    let formats = checker(&audio_data)?;
    let format = formats
        .get(0)
        .ok_or(BackendError::UnrecognizedAudioFormat)?;

    Ok((audio_data, format.clone()))
}

async fn save_recording_metadata(
    _logger: Arc<Logger>,
    db: Arc<dyn Db + Send + Sync>,
    parent_id: &Uuid,
    metadata: UploadMetadata,
) -> Result<Uuid, BackendError> {
    let new_recording = db.insert(parent_id, metadata).await?;
    let id = new_recording.id();

    Ok(*id)
}

async fn save_upload_audio<O>(
    _logger: Arc<Logger>,
    store: Arc<environment::VecStore<O>>,
    key: &Uuid,
    content_type: String,
    upload: Vec<u8>,
) -> Result<(), BackendError> {
    store.save(key, content_type, upload).await?;

    Ok(())
}

async fn update_recording_url<O>(
    _logger: Arc<Logger>,
    db: Arc<dyn Db + Send + Sync>,
    store: Arc<environment::VecStore<O>>,
    key: &Uuid,
    mime_type: MimeType,
) -> Result<Url, BackendError> {
    let url = store
        .get_url(&key)
        .map_err(|e| BackendError::FailedToGenerateUrl { source: e })?;

    db.update_url(key, &url, mime_type.clone()).await?;

    Ok(url)
}

async fn remove_token(
    _logger: Arc<Logger>,
    db: Arc<dyn Db + Send + Sync>,
    token: Uuid,
) -> Result<(), BackendError> {
    db.remove_token(&token).await
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
        BackendError::InvalidAudioFormat { .. } => StatusCode::UNSUPPORTED_MEDIA_TYPE,
        PartsMissing => StatusCode::BAD_REQUEST,
        NameAlreadyExists => StatusCode::FORBIDDEN,
        InvalidToken { .. } => StatusCode::UNAUTHORIZED,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
