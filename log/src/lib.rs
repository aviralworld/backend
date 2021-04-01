use std::sync::Mutex;

use slog::Drain;
use slog::Fuse;
pub use slog::{debug, error, info, o, trace, warn, Logger};
use slog_async::Async;
use slog_json::Json;

pub fn initialize_logger() -> slog::Logger {
    #[cfg(feature = "env_logging")]
    env_logging::initialize_global_logger();

    // TODO is this the correct sequence?
    let drain = Mutex::new(Json::default(std::io::stderr())).map(Fuse);
    let drain = Async::new(drain).build().fuse();

    let logger = Logger::root(
        drain,
        o!("version" => info::VERSION, "revision" => info::REVISION, "build_timestamp" => info::BUILD_TIMESTAMP),
    );

    logger
}

#[cfg(feature = "env_logging")]
mod env_logging {
    use std::sync::{Arc, RwLock};

    use lazy_static::lazy_static;
    use slog_scope::GlobalLoggerGuard;

    lazy_static! {
        static ref GUARD: Arc<RwLock<Option<GlobalLoggerGuard>>> = Arc::new(RwLock::new(None));
    }

    pub fn initialize_global_logger() {
        if GUARD.read().unwrap().is_none() {
            *GUARD.write().unwrap() = Some(slog_envlogger::init().unwrap());
        }
    }
}
