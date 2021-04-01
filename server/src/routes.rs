use std::sync::Arc;

use futures::future::{BoxFuture, FutureExt};
use log::{debug, error, trace, Logger};
use serde::Serialize;
use url::Url;
use uuid::Uuid;
use warp::filters::multipart::{form, FormData, Part};
use warp::http::StatusCode;
use warp::reject;
use warp::reply::{json, with_header, with_status, Json, Reply, WithHeader, WithStatus};
use warp::Filter;

use crate::environment::Environment;
use crate::errors::{summarize_delete_errors, BackendError};
use crate::io::parse_upload;
use crate::recording::{ChildRecording, PartialRecording, UploadMetadata};
use crate::{audio::format::AudioFormat, db::Db, environment, mime_type::MimeType};

mod rejection;

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum SuccessResponse<'a> {
    Children {
        parent: String,
        children: Vec<ChildRecording>,
    },
    Count(i64),
    Healthz {
        revision: Option<&'a str>,
        timestamp: Option<&'a str>,
        version: &'a str,
    },
    Lookup {
        id: Uuid,
    },
    Random {
        recordings: Vec<PartialRecording>,
    },
    Token {
        id: String,
        parent_id: String,
    },
    Upload {
        id: String,
        // TODO these should not be options
        tokens: Option<Vec<Uuid>>,
        key: Option<Uuid>,
    },
}

/// The maximum form data size to accept. This should be enforced by the HTTP gateway, so on the Rust side it’s set to an unreasonably large number.
const MAX_CONTENT_LENGTH: u64 = 2 * 1024 * 1024 * 1024;

// the filters can be simplified once async closures are stabilized
// (rust/rust-lang#62290) and `impl Trait` can be used with closures;
// in the mean time, we have to use `BoxFuture`

pub fn make_formats_route<'a, O: Clone + Send + Sync + 'a>(
    environment: Environment<O>,
) -> impl warp::Filter<Extract = (impl Reply,), Error = reject::Rejection> + Clone + 'a {
    let recordings_path = environment.urls.recordings_path.clone();

    let handler =
        move || -> BoxFuture<Result<Json, reject::Rejection>> {
            let environment = environment.clone();

            async move {
                let formats = environment.db.retrieve_format_essences().await.map_err(
                    |e: BackendError| rejection::Rejection::new(rejection::Context::formats(), e),
                )?;

                Ok(json(&formats))
            }
            .boxed()
        };

    // TODO make this cacheable
    warp::path(recordings_path)
        .and(warp::path("formats"))
        .and(warp::path::end())
        .and(warp::get())
        .and_then(handler)
}

pub fn make_ages_list_route<'a, O: Clone + Send + Sync + 'a>(
    environment: Environment<O>,
) -> impl warp::Filter<Extract = (impl Reply,), Error = reject::Rejection> + Clone + 'a {
    let recordings_path = environment.urls.recordings_path.clone();

    let handler = move || -> BoxFuture<Result<Json, reject::Rejection>> {
        let environment = environment.clone();

        async move {
            let ages = environment
                .db
                .retrieve_ages()
                .await
                .map_err(|e: BackendError| {
                    rejection::Rejection::new(rejection::Context::ages(), e)
                })?;

            Ok(json(&ages))
        }
        .boxed()
    };

    // TODO make this cacheable
    warp::path(recordings_path)
        .and(warp::path("ages"))
        .and(warp::path::end())
        .and(warp::get())
        .and_then(handler)
}

pub fn make_categories_list_route<'a, O: Clone + Send + Sync + 'a>(
    environment: Environment<O>,
) -> impl warp::Filter<Extract = (impl Reply,), Error = reject::Rejection> + Clone + 'a {
    let recordings_path = environment.urls.recordings_path.clone();

    let handler = move || -> BoxFuture<Result<Json, reject::Rejection>> {
        let environment = environment.clone();

        async move {
            let categories =
                environment
                    .db
                    .retrieve_categories()
                    .await
                    .map_err(|e: BackendError| {
                        rejection::Rejection::new(rejection::Context::categories(), e)
                    })?;

            Ok(json(&categories))
        }
        .boxed()
    };

    // TODO make this cacheable
    warp::path(recordings_path)
        .and(warp::path("categories"))
        .and(warp::path::end())
        .and(warp::get())
        .and_then(handler)
}

pub fn make_genders_list_route<'a, O: Clone + Send + Sync + 'a>(
    environment: Environment<O>,
) -> impl warp::Filter<Extract = (impl Reply,), Error = reject::Rejection> + Clone + 'a {
    let recordings_path = environment.urls.recordings_path.clone();

    let handler = move || -> BoxFuture<Result<Json, reject::Rejection>> {
        let environment = environment.clone();

        async move {
            let genders = environment
                .db
                .retrieve_genders()
                .await
                .map_err(|e: BackendError| {
                    rejection::Rejection::new(rejection::Context::genders(), e)
                })?;

            Ok(json(&genders))
        }
        .boxed()
    };

    // TODO make this cacheable
    warp::path(recordings_path)
        .and(warp::path("genders"))
        .and(warp::path::end())
        .and(warp::get())
        .and_then(handler)
}

pub fn make_count_route<'a, O: Clone + Send + Sync + 'a>(
    environment: Environment<O>,
) -> impl warp::Filter<Extract = (impl Reply,), Error = reject::Rejection> + Clone + 'a {
    let recordings_path = environment.urls.recordings_path.clone();

    let handler = move || -> BoxFuture<Result<Json, reject::Rejection>> {
        let environment = environment.clone();

        async move {
            let count = environment
                .db
                .count_all()
                .await
                .map_err(|e: BackendError| {
                    rejection::Rejection::new(rejection::Context::count(), e)
                })?;

            Ok(json(&SuccessResponse::Count(count)))
        }
        .boxed()
    };

    warp::path(recordings_path)
        .and(warp::path("count"))
        .and(warp::path::end())
        .and(warp::get())
        .and_then(handler)
}

pub fn make_upload_route<'a, O: Clone + Send + Sync + 'a>(
    environment: Environment<O>,
) -> impl warp::Filter<Extract = (impl Reply,), Error = reject::Rejection> + Clone + 'a {
    let urls = environment.urls.clone();

    // TODO this should stream the body from the request, but warp
    // doesn't support that yet; however, see
    // <https://github.com/cetra3/mpart-async>
    let handler =
        move |content: FormData| -> BoxFuture<Result<WithHeader<WithStatus<Json>>, reject::Rejection>> {
            let environment = environment.clone();

            async move {
                use log::o;

                let Environment {
                    logger,
                    db,
                    checker,
                    ..
                } = environment.clone();

                let error_handler =
                    |e: BackendError| rejection::Rejection::new(rejection::Context::upload(None), e);

                debug!(logger, "Parsing submission...");
                let upload = parse_upload(content).await.map_err(error_handler)?;

                let metadata = parse_recording_metadata(logger.clone(), upload.metadata)
                    .await
                    .map_err(error_handler)?;

                let token = metadata.token;

                let logger = Arc::new(logger.new(o!("token" => format!("{}", token.clone()))));

                debug!(logger, "Locking token...");
                let parent_id = lock_token(logger.clone(), db.clone(), token)
                    .await
                    .map_err(error_handler)?;

                let logger = Arc::new(logger.new(o!("parent_id" => format!("{}", parent_id.clone()))));

                let error_handler = |e: BackendError| {
                    // first spawn a task to release the token, logging any
                    // errors, then go back to normal error handling
                    let logger = logger.clone();
                    let db = db.clone();
                    let token = token;

                    tokio::spawn(async move {
                        release_token(logger.clone(), db.clone(), token)
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
                let email = metadata.email.clone(); // save for later
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
                    Some(mime_type) => complete_upload(environment.clone(), id, token, email, mime_type, verified_audio, error_handler).await,
                    // why does this work but not directly returning `Err(error_handler(BackendError::...))`?
                    None => Err(BackendError::InvalidAudioFormat {
                        format: audio_format,
                    })
                        .map_err(&error_handler)?,
                }
            }
            .boxed()
        }
    ;

    warp::path(urls.recordings_path.clone())
        .and(warp::path::end())
        .and(warp::post())
        .and(form().max_length(MAX_CONTENT_LENGTH))
        .and_then(handler)
}

pub fn make_children_route<'a, O: Clone + Send + Sync + 'a>(
    environment: Environment<O>,
) -> impl warp::Filter<Extract = (impl Reply,), Error = reject::Rejection> + Clone + 'a {
    let recordings_path = environment.urls.recordings_path.clone();

    let handler = move |parent: String| -> BoxFuture<Result<WithStatus<Json>, reject::Rejection>> {
        let environment = environment.clone();

        async move {
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
        }.boxed()
    };

    warp::path(recordings_path)
        .and(warp::path!("id" / String / "children"))
        .and(warp::path::end())
        .and(warp::get())
        .and_then(handler)
}

pub fn make_delete_route<'a, O: Clone + Send + Sync + 'a>(
    environment: Environment<O>,
) -> impl warp::Filter<Extract = (impl Reply,), Error = reject::Rejection> + Clone + 'a {
    let recordings_path = environment.urls.recordings_path.clone();

    let handler = move |id: String| -> BoxFuture<Result<StatusCode, reject::Rejection>> {
        let environment = environment.clone();

        async move {
            let error_handler = |e: BackendError| {
                rejection::Rejection::new(rejection::Context::delete(id.clone()), e)
            };

            let id = Uuid::parse_str(&id)
                .map_err(|_| BackendError::InvalidId(id.clone()))
                .map_err(error_handler)?;
            debug!(environment.logger, "Deleting recording..."; "id" => format!("{}", &id));

            environment.store.delete(&id).await.map_err(error_handler)?;
            environment
                .db
                .delete(&id)
                .await
                .map_err(|e| summarize_delete_errors(id.clone(), e))
                .map_err(error_handler)?;

            Ok(StatusCode::NO_CONTENT)
        }
        .boxed()
    };

    warp::path(recordings_path)
        .and(warp::path("id"))
        .and(warp::path::param::<String>())
        .and(warp::path::end())
        .and(warp::delete())
        .and_then(handler)
}

pub fn make_retrieve_route<'a, O: Clone + Send + Sync + 'a>(
    environment: Environment<O>,
) -> impl warp::Filter<Extract = (impl Reply,), Error = reject::Rejection> + Clone + 'a {
    let recordings_path = environment.urls.recordings_path.clone();

    let handler = move |id: String| -> BoxFuture<Result<WithStatus<Json>, reject::Rejection>> {
        let environment = environment.clone();

        async move {
            use crate::recording::Recording;

            let error_handler = |e: BackendError| {
                rejection::Rejection::new(rejection::Context::retrieve(id.clone()), e)
            };

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
        .boxed()
    };

    warp::path(recordings_path)
        .and(warp::path("id"))
        .and(warp::path::param::<String>())
        .and(warp::path::end())
        .and(warp::get())
        .and_then(handler)
}

pub fn make_random_route<'a, O: Clone + Send + Sync + 'a>(
    environment: Environment<O>,
) -> impl warp::Filter<Extract = (impl Reply,), Error = reject::Rejection> + Clone + 'a {
    let recordings_path = environment.urls.recordings_path.clone();

    let handler = move |count: u8| -> BoxFuture<Result<Json, reject::Rejection>> {
        let environment = environment.clone();
        let count = count as i16;

        async move {
            let error_handler =
                |e: BackendError| rejection::Rejection::new(rejection::Context::random(count), e);

            let recordings = environment
                .db
                .retrieve_random(count)
                .await
                .map_err(error_handler)?;

            Ok(json(&SuccessResponse::Random { recordings }))
        }
        .boxed()
    };

    warp::path(recordings_path)
        .and(warp::path("random"))
        .and(warp::path::param::<u8>())
        .and(warp::path::end())
        .and(warp::get())
        .and_then(handler)
}

pub fn make_token_route<'a, O: Clone + Send + Sync + 'a>(
    environment: Environment<O>,
) -> impl warp::Filter<Extract = (impl Reply,), Error = reject::Rejection> + Clone + 'a {
    let recordings_path = environment.urls.recordings_path.clone();

    let handler = move |id: Uuid| -> BoxFuture<Result<WithStatus<Json>, reject::Rejection>> {
        let environment = environment.clone();

        async move {
            let error_handler = |e: BackendError| {
                rejection::Rejection::new(rejection::Context::token(id.to_string()), e)
            };

            let token = environment
                .db
                .retrieve_token(&id)
                .await
                .map_err(error_handler)?;

            match token {
                Some(token) => Ok(with_status(
                    json(&SuccessResponse::Token {
                        id: token.id.to_string(),
                        parent_id: token.parent_id.to_string(),
                    }),
                    StatusCode::OK,
                )),
                _ => Ok(with_status(json(&()), StatusCode::NOT_FOUND)),
            }
        }
        .boxed()
    };

    warp::path(recordings_path)
        .and(warp::path("token"))
        .and(warp::path::param::<Uuid>())
        .and(warp::path::end())
        .and(warp::get())
        .and_then(handler)
}

pub fn make_lookup_key_route<'a, O: Clone + Send + Sync + 'a>(
    environment: Environment<O>,
) -> impl warp::Filter<Extract = (impl Reply,), Error = reject::Rejection> + Clone + 'a {
    let recordings_path = environment.urls.recordings_path.clone();

    let handler = move |key: String| -> BoxFuture<Result<WithStatus<Json>, reject::Rejection>> {
        let environment = environment.clone();

        async move {
            let error_handler = |e: BackendError| {
                rejection::Rejection::new(rejection::Context::lookup_key(key.clone()), e)
            };

            let key = Uuid::parse_str(&key)
                .map_err(|_| BackendError::InvalidId(key.clone()))
                .map_err(error_handler)?;
            debug!(environment.logger, "Looking up key..."; "key" => format!("{}", key));

            let option = environment.db.lookup_key(&key).await.map_err(error_handler)?;

            match option {
                Some(id) => Ok(with_status(json(&SuccessResponse::Lookup { id }), StatusCode::OK)),
                _ => Ok(with_status(json(&()), StatusCode::NOT_FOUND))
            }
        }.boxed()
    };

    warp::path(recordings_path)
        .and(warp::path("lookup"))
        .and(warp::path::param::<String>())
        .and(warp::path::end())
        .and(warp::get())
        .and_then(handler)
}

pub fn make_healthz_route<'a, O: Clone + Send + Sync + 'a>(
    _environment: Environment<O>,
) -> impl warp::Filter<Extract = (impl Reply,), Error = reject::Rejection> + Clone + 'a {
    warp::path("healthz").and(warp::get()).map(move || {
        Ok(json(&SuccessResponse::Healthz {
            revision: info::REVISION,
            timestamp: info::BUILD_TIMESTAMP,
            version: info::VERSION,
        }))
    })
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
    let parent_id = db.lock_token(&token).await?;

    parent_id.ok_or(BackendError::InvalidToken { token })
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

async fn complete_upload<'a, O: Clone + Send + Sync + 'a>(
    environment: Environment<O>,
    id: Uuid,
    token: Uuid,
    email: Option<String>,
    mime_type: MimeType,
    verified_audio: Vec<u8>,
    error_handler: impl Fn(BackendError) -> rejection::Rejection,
) -> Result<WithHeader<WithStatus<Json>>, reject::Rejection> {
    let logger = environment.logger.clone();
    let db = environment.db.clone();
    let store = environment.store.clone();

    store
        .save(&id, mime_type.essence.clone(), verified_audio)
        .await
        .map_err(&error_handler)?;

    debug!(logger, "Updating recording URL...");
    update_recording_url(logger.clone(), db.clone(), store.clone(), &id, mime_type)
        .await
        .map_err(&error_handler)?;

    debug!(logger, "Removing parent token...");
    db.remove_token(&token).await.map_err(&error_handler)?;

    debug!(logger, "Creating child tokens...");
    let tokens = create_tokens(
        logger.clone(),
        db.clone(),
        id,
        environment.config.tokens_per_recording,
    )
    .await
    .map_err(&error_handler)?;

    let key = db
        .create_key(&id, email)
        .await
        .map_err(&error_handler)?;

    let id_as_str = format!("{}", id);

    debug!(logger, "Sending response...");
    let response = SuccessResponse::Upload {
        id: id_as_str,
        tokens: Some(tokens),
        key: Some(key),
    };

    Ok(with_header(
        with_status(json(&response), StatusCode::CREATED),
        "location",
        environment.urls.recording(&id).as_str(),
    ))
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

async fn create_tokens(
    logger: Arc<Logger>,
    db: Arc<dyn Db + Send + Sync>,
    token: Uuid,
    count: u8,
) -> Result<Vec<Uuid>, BackendError> {
    let mut tokens: Vec<Uuid> = vec![];

    for i in 0..count {
        trace!(logger, "Creating token #{}...", i; "parent" => format!("{}", token));
        let token = db.create_token(&token).await?;
        tokens.push(token);
    }

    Ok(tokens)
}

pub async fn format_rejection(
    logger: Arc<Logger>,
    rej: reject::Rejection,
) -> Result<WithStatus<Json>, reject::Rejection> {
    if let Some(r) = rej.find::<rejection::Rejection>() {
        error!(logger, "Backend error"; "context" => ?r.context, "error" => ?r.error, "message" => %r.error);
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
        InvalidId { .. } | PartsMissing | MalformedUploadMetadata { .. } => StatusCode::BAD_REQUEST,
        NameAlreadyExists => StatusCode::FORBIDDEN,
        InvalidToken { .. } => StatusCode::UNAUTHORIZED,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
