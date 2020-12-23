use std::sync::Mutex;

use slog::Drain;
use slog::{o, Fuse, Logger};
use slog_async::Async;
use slog_json::Json;

pub fn initialize_logger() -> slog::Logger {
    // TODO is this the correct sequence?
    let drain = Mutex::new(Json::default(std::io::stderr())).map(Fuse);
    let drain = Async::new(drain).build().fuse();

    #[cfg(feature = "enable_warp_logging")]
    pretty_env_logger::init();

    Logger::root(
        drain,
        o!("version" => env!("CARGO_PKG_VERSION"), "revision" => option_env!("BACKEND_REVISION"), "build_timestamp" => option_env!("BUILD_TIMESTAMP").unwrap_or_else(|| "")),
    )
}
