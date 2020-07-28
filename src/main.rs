use std::error::Error;
use std::sync::Arc;

use slog::Drain;
use slog_async;
use slog_json;
use std::env;
use std::sync::Mutex;

use dotenv;
use pretty_env_logger;
use rusoto_core::request::HttpClient;
use rusoto_core::Region;
use rusoto_credential::StaticProvider;
use rusoto_s3::S3Client;
use tokio;

use backend::audio;
use backend::config::get_variable;
use backend::routes::make_upload_route;
use backend::store::S3Store;

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

    let store = S3Store::new(client, acl, bucket, cache_control, content_type);

    let enable_warp_logging = get_variable("BACKEND_ENABLE_WARP_LOGGING");

    if enable_warp_logging == "1" {
        pretty_env_logger::init();
    }

    let logger = Arc::new(initialize_logger());

    let ffprobe_path = env::var("BACKEND_FFPROBE_PATH").ok();
    let expected_codec = get_variable("BACKEND_CODEC");
    let checker = audio::make_wrapper(ffprobe_path, expected_codec);

    let routes = make_upload_route(logger, Arc::new(store), Arc::new(checker));
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
