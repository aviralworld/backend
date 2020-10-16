use std::env;
use std::error::Error;
use std::sync::Arc;
use std::sync::Mutex;

use slog::{info, Drain};
use warp::Filter;

use backend::audio;
use backend::config::{get_ffprobe, get_variable};
use backend::db::PgDb;
use backend::environment::Environment;
use backend::routes;
use backend::store::S3Store;
use backend::urls::Urls;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv::dotenv().ok();

    let store = Arc::new(S3Store::from_env().expect("initialize S3 store from environment"));

    let logger = initialize_logger();
    info!(logger, "Starting...");
    let logger = Arc::new(logger);

    let ffprobe_path = get_ffprobe(env::var("BACKEND_FFPROBE_PATH").ok());
    let checker = Arc::new(audio::make_wrapper(logger.clone(), ffprobe_path));

    let connection_string = get_variable("BACKEND_DB_CONNECTION_STRING");
    let pool = sqlx::Pool::new(&connection_string)
        .await
        .expect("create database pool from BACKEND_DB_CONNECTION_STRING");
    let db = Arc::new(PgDb::new(pool));

    let urls = Arc::new(Urls::new(
        get_variable("BACKEND_BASE_URL"),
        get_variable("BACKEND_RECORDINGS_PATH"),
    ));

    let environment = Environment::new(logger, db, urls, store, checker);

    let formats_route = routes::make_formats_route(environment.clone());
    let count_route = routes::make_count_route(environment.clone());
    let upload_route = routes::make_upload_route(environment.clone());
    let children_route = routes::make_children_route(environment.clone());
    let delete_route = routes::make_delete_route(environment.clone());
    let retrieve_route = routes::make_retrieve_route(environment.clone());
    let hide_route = routes::make_hide_route(environment.clone());

    let routes = formats_route
        .or(count_route)
        .or(upload_route)
        .or(children_route)
        .or(delete_route)
        .or(retrieve_route)
        .or(hide_route);

    let port: u16 = get_variable("BACKEND_PORT")
        .parse()
        .expect("parse BACKEND_PORT as u16");
    warp::serve(routes).run(([127, 0, 0, 1], port)).await;

    Ok(())
}

fn initialize_logger() -> slog::Logger {
    use slog::{o, Fuse, Logger};
    use slog_async::Async;
    use slog_json::Json;

    // TODO is this the correct sequence?
    let drain = Mutex::new(Json::default(std::io::stderr())).map(Fuse);
    let drain = Async::new(drain).build().fuse();

    #[cfg(feature = "enable_warp_logging")]
    pretty_env_logger::init();

    let logger = Logger::root(
        drain,
        o!("version" => env!("CARGO_PKG_VERSION"), "revision" => option_env!("BACKEND_REVISION")),
    );

    logger
}
