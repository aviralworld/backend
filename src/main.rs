use std::env;
use std::error::Error;
use std::sync::Arc;
use std::sync::Mutex;

use dotenv;
use pretty_env_logger;
use rusoto_core::request::HttpClient;
use rusoto_core::Region;
use rusoto_credential::StaticProvider;
use rusoto_s3::S3Client;
use slog::Drain;
use slog_async;
use slog_json;
use sqlx;
use tokio;
use url::Url;

use backend::audio;
use backend::config::get_variable;
use backend::db::PgDb;
use backend::routes::make_upload_route;
use backend::store::S3Store;
use backend::urls::Urls;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv::dotenv().ok();

    let access_key = get_variable("S3_ACCESS_KEY");
    let secret_access_key = get_variable("S3_SECRET_ACCESS_KEY");

    let region = Region::Custom {
        name: get_variable("S3_REGION_NAME"),
        endpoint: get_variable("S3_ENDPOINT"),
    };

    let bucket = get_variable("S3_BUCKET_NAME");
    let content_type = get_variable("S3_CONTENT_TYPE");
    let acl = get_variable("S3_ACL");
    let cache_control = get_variable("S3_CACHE_CONTROL");

    let client = Arc::new(S3Client::new_with(
        HttpClient::new()?,
        StaticProvider::new_minimal(access_key, secret_access_key),
        region,
    ));

    let base_url = Url::parse(&get_variable("S3_BASE_URL")).expect("parse S3_BASE_URL");
    let extension = get_variable("BACKEND_MEDIA_EXTENSION");

    let store = S3Store::new(
        client,
        acl,
        bucket,
        cache_control,
        content_type,
        base_url,
        extension,
    );

    let enable_warp_logging = get_variable("BACKEND_ENABLE_WARP_LOGGING");

    if enable_warp_logging == "1" {
        pretty_env_logger::init();
    }

    let logger = Arc::new(initialize_logger());

    let ffprobe_path = env::var("BACKEND_FFPROBE_PATH").ok();
    let expected_codec = get_variable("BACKEND_MEDIA_CODEC");
    let expected_format = get_variable("BACKEND_MEDIA_FORMAT");
    let checker = audio::make_wrapper(
        logger.clone(),
        ffprobe_path,
        expected_codec,
        expected_format,
    );

    let connection_string = get_variable("BACKEND_DB_CONNECTION_STRING");
    let pool = sqlx::Pool::new(&connection_string)
        .await
        .expect("create database pool from BACKEND_DB_CONNECTION_STRING");
    let db = PgDb::new(pool);

    let urls = Urls::new(
        get_variable("BACKEND_BASE_URL"),
        get_variable("BACKEND_RECORDINGS_PATH"),
    );
    let routes = make_upload_route(
        logger,
        Arc::new(db),
        Arc::new(store),
        Arc::new(checker),
        Arc::new(urls),
    );
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
