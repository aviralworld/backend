use std::env;
use std::error::Error;
use std::sync::Arc;

use slog::info;
use warp::Filter;

use backend::audio;
use backend::config::{get_ffprobe, get_variable};
use backend::db::PgDb;
use backend::environment::{Config, Environment};
use backend::log::initialize_logger;
use backend::routes;
use backend::store::S3Store;
use backend::urls::Urls;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv::dotenv().ok();

    let store = Arc::new(S3Store::from_env().expect("initialize S3 store from environment"));

    let logger = initialize_logger();

    let port: u16 = get_variable("BACKEND_PORT")
        .parse()
        .expect("parse BACKEND_PORT as u16");

    info!(logger, "Starting..."; "port" => port);
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

    let config = Config::new(
        get_variable("BACKEND_TOKENS_PER_RECORDING")
            .parse()
            .expect("parse BACKEND_TOKENS_PER_RECORDING as u8"),
    );
    let environment = Environment::new(logger, db, urls, store, checker, config);

    let formats_route = routes::make_formats_route(environment.clone());
    let ages_list_route = routes::make_ages_list_route(environment.clone());
    let categories_list_route = routes::make_categories_list_route(environment.clone());
    let genders_list_route = routes::make_genders_list_route(environment.clone());
    let count_route = routes::make_count_route(environment.clone());
    let upload_route = routes::make_upload_route(environment.clone());
    let children_route = routes::make_children_route(environment.clone());
    let delete_route = routes::make_delete_route(environment.clone());
    let retrieve_route = routes::make_retrieve_route(environment.clone());
    let random_route = routes::make_random_route(environment.clone());

    let routes = formats_route
        .or(ages_list_route)
        .or(categories_list_route)
        .or(genders_list_route)
        .or(count_route)
        .or(upload_route)
        .or(children_route)
        .or(delete_route)
        .or(random_route)
        .or(retrieve_route);

    warp::serve(routes).run(([0, 0, 0, 0], port)).await;

    Ok(())
}
