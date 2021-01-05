use std::sync::Mutex;

use slog::Drain;
use slog::{o, Fuse, Logger};
use slog_async::Async;
use slog_json::Json;

use crate::info;

pub fn initialize_logger() -> slog::Logger {
    // TODO is this the correct sequence?
    let drain = Mutex::new(Json::default(std::io::stderr())).map(Fuse);
    let drain = Async::new(drain).build().fuse();

    #[cfg(feature = "enable_warp_logging")]
    pretty_env_logger::init();

    Logger::root(
        drain,
        o!("version" => info::VERSION, "revision" => info::REVISION, "build_timestamp" => info::BUILD_TIMESTAMP),
    )
}
