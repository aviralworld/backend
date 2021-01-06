pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub const REVISION: Option<&str> = option_env!("BACKEND_REVISION");

pub const BUILD_TIMESTAMP: Option<&str> = option_env!("BUILD_TIMESTAMP");
