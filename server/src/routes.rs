use std::sync::Arc;

use log::{error, Logger};
use warp::http::StatusCode;
use warp::reject;
use warp::reply::{json, with_status, Json, WithStatus};

use crate::errors::BackendError;

pub mod admin;
mod handlers;
mod query;
mod rejection;
mod response;

pub use internal::*;

/// The maximum form data size to accept. This should be enforced by
/// the HTTP gateway, so on the Rust side it’s set to an unreasonably
/// large number.
const MAX_CONTENT_LENGTH: u64 = 2 * 1024 * 1024 * 1024;

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

mod internal {
    use uuid::Uuid;
    use warp::filters::multipart::form;
    use warp::filters::BoxedFilter;
    use warp::path::end;
    use warp::Filter;
    use warp::Reply;
    use warp::{delete, get as g, path as p, path::param as par, post, query};

    use super::{handlers, query as q, MAX_CONTENT_LENGTH};
    use crate::environment::Environment;

    type Route = BoxedFilter<(Box<dyn Reply>,)>;

    macro_rules! route_filter {
    ($route_variable:ident; $first:expr) => (let $route_variable = $route_variable.and($first););
    ($route_variable:ident; $first:expr, $($rest:expr),+) => (
        let $route_variable = $route_variable.and($first);
        route_filter!($route_variable; $($rest),+);
    )
}

    macro_rules! route {
    ($name:ident => $handler:ident, $route_variable:ident; $($filters:expr),+) => (
        pub fn $name<O: Clone + Send + Sync + 'static>(environment: Environment<O>) -> Route {
            let r = environment.urls.recordings_path.clone();

            let $route_variable = warp::any()
                .map(move || environment.clone())
                .and(p(r));

            route_filter!($route_variable; $($filters),+);

            $route_variable.and_then(handlers::$handler)
                .boxed()
        }
    );
}

    route!(make_formats_route => formats, rt; p("formats"), end(), g());
    route!(make_ages_list_route => ages_list, rt; p("ages"), end(), g());
    route!(make_categories_list_route => categories_list, rt; p("categories"), end(), g());
    route!(make_genders_list_route => genders_list, rt; p("genders"), end(), g());
    route!(make_count_route => count, rt; p("count"), end(), g());
    route!(make_upload_route => upload, rt; end(), post(), form().max_length(MAX_CONTENT_LENGTH));
    route!(make_children_route => children, rt; p!("id" / String / "children"), end(), g());
    route!(make_delete_route => delete, rt; p("id"), par::<String>(), end(), delete());
    route!(make_retrieve_route => retrieve, rt; p("id"), par::<String>(), end(), g());
    route!(make_random_route => random, rt; p("random"), par::<u8>(), end(), g());
    route!(make_token_route => token, rt; p("token"), par::<Uuid>(), end(), g());
    route!(make_lookup_route => lookup, rt; p("lookup"), par::<String>(), end(), g());
    route!(make_availability_route => availability, rt; p("available"), query::<q::AvailabilityQuery>(), end(), g());
}
