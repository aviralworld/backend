use url::Url;
use uuid::Uuid;

/// Convenience wrapper for URL generation functions.
#[derive(Clone)]
pub struct Urls {
    /// Top-level URL, including trailing slash.
    base: Url,

    /// Path for all recordings-related actions.
    pub(crate) recordings_path: String,

    /// Prefix for all recordings-related actions.
    recordings_prefix: String,
}

impl Urls {
    /// Create a new instance. `recordings_prefix` should *not* include a trailing slash.
    pub fn new(base: impl AsRef<str>, recordings_prefix: impl Into<String>) -> Self {
        let base =
            Url::parse(base.as_ref()).unwrap_or_else(|_| panic!("parse {} as URL", base.as_ref()));
        let recordings_path = recordings_prefix.into();
        let recordings_prefix = format!("{}/", recordings_path);

        Urls {
            base,
            recordings_path,
            recordings_prefix,
        }
    }

    pub fn recordings(&self) -> Url {
        self.base
            .join(&self.recordings_prefix)
            .expect("get recordings URL")
    }

    pub fn recording(&self, id: &Uuid) -> Url {
        let id = format!("{}", id);
        self.recordings()
            .join(&id)
            .unwrap_or_else(|_| panic!("get URL for recording {}", id))
    }
}
