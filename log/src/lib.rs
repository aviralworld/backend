use std::sync::Mutex;

use slog::Drain;
use slog::Fuse;
use slog_async::Async;
use slog_json::Json;

pub use slog::{debug, error, info, o, trace, warn, Logger};

pub fn initialize_logger() -> slog::Logger {
    // TODO is this the correct sequence?
    let drain = Mutex::new(Json::default(std::io::stderr())).map(Fuse);
    let drain = Async::new(drain).build().fuse();

    Logger::root(
        drain,
        o!("version" => info::VERSION, "revision" => info::REVISION, "build_timestamp" => info::BUILD_TIMESTAMP),
    )
}
