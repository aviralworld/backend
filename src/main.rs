use std::env;
use std::error::Error;
use std::sync::Arc;
use std::sync::Mutex;

use dotenv;
use pretty_env_logger;
use slog::Drain;
use slog_async;
use slog_json;
use sqlx;
use tokio;
use warp::Filter;

use backend::audio;
use backend::config::get_variable;
use backend::db::PgDb;
use backend::routes;
use backend::store::S3Store;
use backend::urls::Urls;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv::dotenv().ok();

    let store = Arc::new(S3Store::from_env().expect("initialize S3 store from environment"));

    let enable_warp_logging = get_variable("BACKEND_ENABLE_WARP_LOGGING");

    if enable_warp_logging == "1" {
        pretty_env_logger::init();
    }

    let logger = Arc::new(initialize_logger());

    let ffprobe_path = env::var("BACKEND_FFPROBE_PATH").ok();
    let expected_codec = get_variable("BACKEND_MEDIA_CODEC");
    let expected_format = get_variable("BACKEND_MEDIA_FORMAT");
    let checker = Arc::new(audio::make_wrapper(
        logger.clone(),
        ffprobe_path,
        expected_codec,
        expected_format,
    ));

    let connection_string = get_variable("BACKEND_DB_CONNECTION_STRING");
    let pool = sqlx::Pool::new(&connection_string)
        .await
        .expect("create database pool from BACKEND_DB_CONNECTION_STRING");
    let db = Arc::new(PgDb::new(pool));

    let urls = Arc::new(Urls::new(
        get_variable("BACKEND_BASE_URL"),
        get_variable("BACKEND_RECORDINGS_PATH"),
    ));

    let count_route = routes::make_count_route(logger.clone(), db.clone(), urls.clone());
    let upload_route = routes::make_upload_route(
        logger.clone(),
        db.clone(),
        store.clone(),
        checker.clone(),
        urls.clone(),
    );
    let children_route = routes::make_children_route(logger.clone(), db.clone(), urls.clone());
    let delete_route =
        routes::make_delete_route(logger.clone(), db.clone(), store.clone(), urls.clone());
    let retrieve_route = routes::make_retrieve_route(logger.clone(), db.clone(), urls.clone());
    let hide_route = routes::make_hide_route(logger.clone(), db.clone(), urls.clone());

    let routes = count_route
        .or(upload_route)
        .or(children_route)
        .or(delete_route)
        .or(retrieve_route)
        .or(hide_route);

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;

    Ok(())
}

fn initialize_logger() -> slog::Logger {
    use slog::{o, Fuse, Logger};
    use slog_async::Async;
    use slog_json::Json;

    // TODO is this the correct sequence?
    let drain = Mutex::new(Json::default(std::io::stderr())).map(Fuse);
    let drain = Async::new(drain).build().fuse();

    Logger::root(drain, o!("version" => env!("CARGO_PKG_VERSION")))
}
