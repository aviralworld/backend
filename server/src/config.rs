use std::env;
use std::path::PathBuf;

/// Returns the value of the named environment variable if it exists or panics.
pub fn get_variable(name: &str) -> String {
    env::var(name).unwrap_or_else(|_| panic!("must define {} environment variable", name))
}

#[cfg(not(use_ffmpeg_sys))]
pub fn get_ffprobe(env: Option<String>) -> Option<PathBuf> {
    use which::which;

    which("ffprobe")
        .ok()
        .or_else(move || env.map(PathBuf::from))
}

#[cfg(use_ffmpeg_sys)]
pub fn get_ffprobe(env: Option<String>) -> Option<PathBuf> {
    env.map(PathBuf::from)
}
