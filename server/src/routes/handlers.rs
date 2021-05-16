use std::sync::Arc;
use std::time::{Duration, Instant};

use log::{debug, error, trace, Logger};
use url::Url;
use uuid::Uuid;
use warp::{
    filters::multipart::{FormData, Part},
    http::StatusCode,
    reject,
    reply::{json, with_header, with_status, Reply},
};

use crate::environment::{Environment, SafeStore};
use crate::errors::{summarize_delete_errors, BackendError};
use crate::io::parse_upload;
use crate::recording::UploadMetadata;
use crate::routes::{
    query::AvailabilityQuery,
    rejection::{Context, Rejection},
    response::SuccessResponse,
};
use crate::{audio::format::AudioFormat, db::Db, environment, mime_type::MimeType};

const SERVER_TIMING_HEADER: &str = "server-timing";
type RouteResult = Result<Box<dyn Reply>, reject::Rejection>;

macro_rules! timed {
    ($($expression:stmt);+) => {
        let start = Instant::now();

        // TODO when `try` blocks are stabilized, we can wrap the body
        // and return the headers even on errors
        let result = { $($expression)+ };

        Ok(Box::new(with_header(
            result,
            SERVER_TIMING_HEADER,
            format_server_timing(start.elapsed()),
        )) as Box<dyn Reply>)
    };
}

pub async fn formats<O: SafeStore>(environment: Environment<O>) -> RouteResult {
    timed! {
        let formats = environment
            .db
            .retrieve_format_essences()
            .await
            .map_err(|e: BackendError| Rejection::new(Context::formats(), e))?;

        // TODO make this cacheable
        json(&formats)
    }
}

pub async fn ages_list<O: SafeStore>(environment: Environment<O>) -> RouteResult {
    timed! {
        let ages = environment
            .db
            .retrieve_ages()
            .await
            .map_err(|e: BackendError| Rejection::new(Context::ages(), e))?;

        // TODO make this cacheable
        json(&ages)
    }
}

pub async fn categories_list<O: SafeStore>(environment: Environment<O>) -> RouteResult {
    timed! {
    let categories = environment
        .db
        .retrieve_categories()
        .await
        .map_err(|e: BackendError| Rejection::new(Context::categories(), e))?;

        // TODO make this cacheable
        json(&categories)
    }
}

pub async fn genders_list<O: SafeStore>(environment: Environment<O>) -> RouteResult {
    timed! {
        let genders = environment
            .db
            .retrieve_genders()
            .await
            .map_err(|e: BackendError| Rejection::new(Context::genders(), e))?;

        // TODO make this cacheable
        json(&genders)
    }
}

pub async fn count<O: SafeStore>(environment: Environment<O>) -> RouteResult {
    timed! {
        let count = environment
            .db
            .count_all()
            .await
            .map_err(|e: BackendError| Rejection::new(Context::count(), e))?;

        json(&SuccessResponse::Count(count))
    }
}

pub async fn upload<O: SafeStore + 'static>(
    environment: Environment<O>,
    content: FormData,
) -> RouteResult {
    use log::o;

    timed! {
        let Environment {
            logger,
            db,
            checker,
            ..
        } = environment.clone();

        let error_handler = |e: BackendError| Rejection::new(Context::upload(None), e);

        debug!(logger, "Parsing submission...");
        let upload = parse_upload(content).await.map_err(error_handler)?;

        debug!(logger, "Parsing recording metadata...");
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
            Rejection::new(Context::upload(Some(id_as_str.clone())), e)
        };

        // should this punt to a queue? is that necessary?
        debug!(logger, "Saving recording to store...");

        let mime_type = db
            .retrieve_mime_type(&audio_format)
            .await
            .map_err(&error_handler)?
            .ok_or_else(|| error_handler(BackendError::InvalidAudioFormat {
                format: audio_format,
            }))?;

        complete_upload(
                environment.clone(),
                id,
                token,
                email,
                mime_type,
                verified_audio,
                error_handler,
            )
                .await?
    }
}

pub async fn children<O: SafeStore>(environment: Environment<O>, parent: String) -> RouteResult {
    timed! {
        let error_handler = |e: BackendError| Rejection::new(Context::children(parent.clone()), e);

        let id = Uuid::parse_str(&parent)
            .map_err(|_| BackendError::InvalidId(parent.clone()))
            .map_err(error_handler)?;
        debug!(environment.logger, "Searching for children..."; "parent" => &parent.to_string());

        let children = environment.db.children(&id).await.map_err(error_handler)?;
        let response = SuccessResponse::Children { parent, children };

        with_status(json(&response), StatusCode::OK)
    }
}

pub async fn delete<O: SafeStore>(environment: Environment<O>, id: String) -> RouteResult {
    timed! {
        let error_handler = |e: BackendError| Rejection::new(Context::delete(id.clone()), e);

        let id = Uuid::parse_str(&id)
            .map_err(|_| BackendError::InvalidId(id.clone()))
            .map_err(error_handler)?;
        debug!(environment.logger, "Deleting recording..."; "id" => format!("{}", &id));

        environment.store.delete(&id).await.map_err(error_handler)?;
        environment
            .db
            .delete(&id)
            .await
            .map_err(|e| summarize_delete_errors(id, e))
            .map_err(error_handler)?;

        StatusCode::NO_CONTENT
    }
}

pub async fn retrieve<O: SafeStore>(environment: Environment<O>, id: String) -> RouteResult {
    use crate::recording::Recording;

    timed! {
        let error_handler = |e: BackendError| Rejection::new(Context::retrieve(id.clone()), e);

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

                with_status(json(&recording), status)
            }
            None => with_status(json(&()), StatusCode::NOT_FOUND),
        }
    }
}

pub async fn random<O: SafeStore>(environment: Environment<O>, count: u8) -> RouteResult {
    timed! {
        let count = count as i16;

        let error_handler = |e: BackendError| Rejection::new(Context::random(count), e);

        let recordings = environment
            .db
            .retrieve_random(count)
            .await
            .map_err(error_handler)?;

        json(&SuccessResponse::Random { recordings })
    }
}

pub async fn token<O: SafeStore>(environment: Environment<O>, id: Uuid) -> RouteResult {
    timed! {
        let error_handler = |e: BackendError| Rejection::new(Context::token(id.to_string()), e);

        let token = environment
            .db
            .retrieve_token(&id)
            .await
            .map_err(error_handler)?;

        match token {
            Some(token) => with_status(
                json(&SuccessResponse::Token {
                    id: token.id.to_string(),
                    parent_id: token.parent_id.to_string(),
                }),
                StatusCode::OK,
            ),
            _ => with_status(json(&()), StatusCode::NOT_FOUND),
        }
    }
}

pub async fn lookup<O: SafeStore>(environment: Environment<O>, key: String) -> RouteResult {
    timed! {
        let error_handler = |e: BackendError| Rejection::new(Context::lookup_key(key.clone()), e);

        let key = Uuid::parse_str(&key)
            .map_err(|_| BackendError::InvalidId(key.clone()))
            .map_err(error_handler)?;
        debug!(environment.logger, "Looking up key..."; "key" => format!("{}", key));

        let option = environment
            .db
            .lookup_key(&key)
            .await
            .map_err(error_handler)?;

        match option {
            Some((id, tokens)) => with_status(
                json(&SuccessResponse::Lookup { id, tokens }),
                StatusCode::OK,
            ),
            _ => with_status(json(&()), StatusCode::NOT_FOUND),
        }
    }
}

pub async fn availability<O: SafeStore>(
    environment: Environment<O>,
    query: AvailabilityQuery,
) -> RouteResult {
    timed! {
        let AvailabilityQuery { name } = query;

        let available = environment
            .db
            .check_availability(&name)
            .await
            .map_err(|e| Rejection::new(Context::availability(name.clone()), e))?;

        if available {
            StatusCode::OK
        } else {
            StatusCode::FORBIDDEN
        }
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
    let formats = checker(&audio_data).map_err(|_| BackendError::MalformedFormSubmission)?;
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

async fn complete_upload<O: SafeStore + 'static>(
    environment: Environment<O>,
    id: Uuid,
    token: Uuid,
    email: Option<String>,
    mime_type: MimeType,
    verified_audio: Vec<u8>,
    error_handler: impl Fn(BackendError) -> Rejection,
) -> Result<Box<dyn Reply>, reject::Rejection> {
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

    let key = db.create_key(&id, email).await.map_err(&error_handler)?;

    let id_as_str = format!("{}", id);

    debug!(logger, "Sending response...");
    let response = SuccessResponse::Upload {
        id: id_as_str,
        tokens: Some(tokens),
        key: Some(key),
    };

    Ok(Box::new(with_header(
        with_status(json(&response), StatusCode::CREATED),
        "location",
        environment.urls.recording(&id).as_str(),
    )) as Box<dyn Reply>)
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

fn format_server_timing(seconds: Duration) -> String {
    format!("handler;dur={}", seconds.as_secs_f64() * 1000.0)
}
