use std::sync::Arc;

use log::{error, Logger};
use uuid::Uuid;
use warp::filters::multipart::form;
use warp::http::StatusCode;
use warp::reject;
use warp::reply::{json, with_status, Json, Reply, WithStatus};
use warp::{filters::BoxedFilter, Filter};

use crate::environment::Environment;
use crate::errors::BackendError;

pub mod admin;
mod handlers;
mod query;
mod rejection;
mod response;

/// The maximum form data size to accept. This should be enforced by
/// the HTTP gateway, so on the Rust side it’s set to an unreasonably
/// large number.
const MAX_CONTENT_LENGTH: u64 = 2 * 1024 * 1024 * 1024;

type Route = BoxedFilter<(Box<dyn Reply>,)>;

pub fn make_formats_route<O: Clone + Send + Sync + 'static>(environment: Environment<O>) -> Route {
    let recordings_path = environment.urls.recordings_path.clone();

    warp::path(recordings_path)
        .and(warp::path("formats"))
        .and(warp::path::end())
        .and(warp::get())
        .map(move || environment.clone())
        .and_then(handlers::formats)
        .boxed()
}

pub fn make_ages_list_route<O: Clone + Send + Sync + 'static>(
    environment: Environment<O>,
) -> Route {
    let recordings_path = environment.urls.recordings_path.clone();

    warp::path(recordings_path)
        .and(warp::path("ages"))
        .and(warp::path::end())
        .and(warp::get())
        .map(move || environment.clone())
        .and_then(handlers::ages_list)
        .boxed()
}

pub fn make_categories_list_route<O: Clone + Send + Sync + 'static>(
    environment: Environment<O>,
) -> Route {
    let recordings_path = environment.urls.recordings_path.clone();

    // TODO make this cacheable
    warp::path(recordings_path)
        .and(warp::path("categories"))
        .and(warp::path::end())
        .and(warp::get())
        .map(move || environment.clone())
        .and_then(handlers::categories_list)
        .boxed()
}

pub fn make_genders_list_route<O: Clone + Send + Sync + 'static>(
    environment: Environment<O>,
) -> Route {
    let recordings_path = environment.urls.recordings_path.clone();

    warp::path(recordings_path)
        .and(warp::path("genders"))
        .and(warp::path::end())
        .and(warp::get())
        .map(move || environment.clone())
        .and_then(handlers::genders_list)
        .boxed()
}

pub fn make_count_route<O: Clone + Send + Sync + 'static>(environment: Environment<O>) -> Route {
    let recordings_path = environment.urls.recordings_path.clone();

    warp::path(recordings_path)
        .and(warp::path("count"))
        .and(warp::path::end())
        .and(warp::get())
        .map(move || environment.clone())
        .and_then(handlers::count)
        .boxed()
}

pub fn make_upload_route<O: Clone + Send + Sync + 'static>(environment: Environment<O>) -> Route {
    let urls = environment.urls.clone();

    // TODO this should stream the body from the request, but warp
    // doesn't support that yet; however, see
    // <https://github.com/cetra3/mpart-async>

    warp::path(urls.recordings_path.clone())
        .and(warp::path::end())
        .and(warp::post())
        .map(move || environment.clone())
        .and(form().max_length(MAX_CONTENT_LENGTH))
        .and_then(handlers::upload)
        .boxed()
}

pub fn make_children_route<O: Clone + Send + Sync + 'static>(environment: Environment<O>) -> Route {
    let recordings_path = environment.urls.recordings_path.clone();

    warp::any()
        .map(move || environment.clone())
        .and(warp::path(recordings_path))
        .and(warp::path!("id" / String / "children"))
        .and(warp::path::end())
        .and(warp::get())
        .and_then(handlers::children)
        .boxed()
}

pub fn make_delete_route<O: Clone + Send + Sync + 'static>(environment: Environment<O>) -> Route {
    let recordings_path = environment.urls.recordings_path.clone();

    warp::any()
        .map(move || environment.clone())
        .and(warp::path(recordings_path))
        .and(warp::path("id"))
        .and(warp::path::param::<String>())
        .and(warp::path::end())
        .and(warp::delete())
        .and_then(handlers::delete)
        .boxed()
}

pub fn make_retrieve_route<O: Clone + Send + Sync + 'static>(environment: Environment<O>) -> Route {
    let recordings_path = environment.urls.recordings_path.clone();

    warp::any()
        .map(move || environment.clone())
        .and(warp::path(recordings_path))
        .and(warp::path("id"))
        .and(warp::path::param::<String>())
        .and(warp::path::end())
        .and(warp::get())
        .and_then(handlers::retrieve)
        .boxed()
}

pub fn make_random_route<O: Clone + Send + Sync + 'static>(environment: Environment<O>) -> Route {
    let recordings_path = environment.urls.recordings_path.clone();

    warp::any()
        .map(move || environment.clone())
        .and(warp::path(recordings_path))
        .and(warp::path("random"))
        .and(warp::path::param::<u8>())
        .and(warp::path::end())
        .and(warp::get())
        .and_then(handlers::random)
        .boxed()
}

pub fn make_token_route<O: Clone + Send + Sync + 'static>(environment: Environment<O>) -> Route {
    let recordings_path = environment.urls.recordings_path.clone();

    warp::any()
        .map(move || environment.clone())
        .and(warp::path(recordings_path))
        .and(warp::path("token"))
        .and(warp::path::param::<Uuid>())
        .and(warp::path::end())
        .and(warp::get())
        .and_then(handlers::token)
        .boxed()
}

pub fn make_lookup_key_route<O: Clone + Send + Sync + 'static>(
    environment: Environment<O>,
) -> Route {
    let recordings_path = environment.urls.recordings_path.clone();

    warp::any()
        .map(move || environment.clone())
        .and(warp::path(recordings_path))
        .and(warp::path("lookup"))
        .and(warp::path::param::<String>())
        .and(warp::path::end())
        .and(warp::get())
        .and_then(handlers::lookup)
        .boxed()
}

pub fn make_availability_route<O: Clone + Send + Sync + 'static>(
    environment: Environment<O>,
) -> Route {
    let recordings_path = environment.urls.recordings_path.clone();

    warp::any()
        .map(move || environment.clone())
        .and(warp::path(recordings_path))
        .and(warp::path("available"))
        .and(warp::query::<query::AvailabilityQuery>())
        .and(warp::path::end())
        .and(warp::get())
        .and_then(handlers::availability)
        .boxed()
}

pub async fn format_rejection(
    logger: Arc<Logger>,
    rej: reject::Rejection,
) -> Result<WithStatus<Json>, reject::Rejection> {
    if let Some(r) = rej.find::<rejection::Rejection>() {
        let e = &r.error;
        error!(logger, "Backend error"; "context" => ?r.context, "error" => ?r.error, "status" => %status_code_for(e), "message" => %r.error);
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
        InvalidId { .. }
        | PartsMissing
        | MalformedUploadMetadata { .. }
        | MalformedFormSubmission { .. } => StatusCode::BAD_REQUEST,
        NameAlreadyExists => StatusCode::FORBIDDEN,
        InvalidToken { .. } => StatusCode::UNAUTHORIZED,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
