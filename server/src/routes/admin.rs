use std::sync::Arc;

use futures::future::{BoxFuture, FutureExt};
use warp::Filter;
use warp::http::StatusCode;
use warp::reject;
use warp::reply::{json, Reply};

use super::response::SuccessResponse;
use crate::environment::Environment;

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

type TerminationFuture<'a> = BoxFuture<'a, ()>;

type TerminationFunctionWrapper<'a> =
    Arc<dyn Fn() -> TerminationFuture<'a> + Send + Sync + 'a>;

pub fn make_termination_route<'a, O: Clone + Send + Sync + 'a>(
    _environment: Environment<O>,
    terminate: TerminationFunctionWrapper<'a>,
) -> impl warp::Filter<Extract = (impl Reply,), Error = reject::Rejection> + Clone + 'a {
    let terminate = terminate.clone();

    let handler = move || -> BoxFuture<Result<StatusCode, std::convert::Infallible>> {
        let terminate = terminate.clone();

        async move {
            let future = terminate();
            future.await;
            Ok(StatusCode::NO_CONTENT)
        }
        .boxed()
    };

    warp::path("terminate").and(warp::post()).and_then(handler)
}
