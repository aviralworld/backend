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

use backend::audio::CodecChecker;
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
    let checker = initialize_ogg_checker(env::var("BACKEND_FFPROBE_PATH").ok());

    let expected_codec = get_variable("BACKEND_CODEC");

    let check_fn = move |data: &[u8]| checker.is_codec(data, &expected_codec).map(|_| ());

    let routes = make_upload_route(logger, Arc::new(store), Arc::new(check_fn));
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

fn get_variable(name: &str) -> String {
    env::var(name).expect(&format!("must define {} environment variable", name))
}

#[cfg(not(use_ffmpeg_sys))]
fn initialize_ogg_checker(ffprobe_path: Option<String>) -> impl CodecChecker {
    use backend::audio::Checker;

    Checker::new(ffprobe_path.expect("must define ffprobe path or compile with `use_ffmpeg_sys`"))
}

#[cfg(use_ffmpeg_sys)]
fn initialize_ogg_checker(ffprobe_path: Option<String>) -> impl CodecChecker {
    use backend::audio::Checker;

    Checker
}
