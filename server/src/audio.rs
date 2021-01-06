use std::path::{Path, PathBuf};
use std::sync::Arc;

use log::Logger;

use crate::errors::BackendError;

pub mod format;

use format::AudioFormat;

pub trait CodecChecker {
    fn identify(&self, logger: Arc<Logger>, data: &[u8]) -> Result<Vec<AudioFormat>, BackendError>;

    fn new(ffprobe_path: Option<impl AsRef<Path>>) -> Self;
}

pub fn make_wrapper(
    logger: Arc<Logger>,
    ffprobe_path: Option<PathBuf>,
) -> impl Fn(&[u8]) -> Result<Vec<AudioFormat>, BackendError> {
    let checker = inner::Checker::new(ffprobe_path);

    move |data: &[u8]| checker.identify(logger.clone(), data)
}

#[cfg(not(use_ffmpeg_sys))]
mod inner {
    use std::ffi::OsString;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    use lazy_static::lazy_static;
    use serde::Deserialize;
    use log::Logger;

    use crate::audio::format::AudioFormat;
    use crate::errors::BackendError;

    lazy_static! {
        static ref FFPROBE_ARGS: Vec<OsString> = vec![
            OsString::from("-hide_banner"),
            OsString::from("-v"),
            OsString::from("error"),
            OsString::from("-of"),
            OsString::from("json"),
            OsString::from("-show_format"),
            OsString::from("-show_entries"),
            OsString::from("stream=codec_name"),
        ];
    }

    pub struct Checker {
        ffprobe: PathBuf,
    }

    #[derive(Deserialize)]
    struct FfprobeOutput {
        streams: Vec<FfprobeStream>,
        format: FfprobeFormat,
    }

    #[derive(Deserialize)]
    struct FfprobeStream {
        codec_name: String,
    }

    #[derive(Deserialize)]
    struct FfprobeFormat {
        format_name: String,
    }

    impl Checker {}

    impl super::CodecChecker for Checker {
        fn identify(
            &self,
            _logger: Arc<Logger>,
            data: &[u8],
        ) -> Result<Vec<AudioFormat>, BackendError> {
            use std::io::Write;
            use std::process::Command;

            use tempfile::NamedTempFile;

            let output_path = {
                let mut output = NamedTempFile::new().map_err(BackendError::TemporaryFileError)?;
                output
                    .write_all(data)
                    .map_err(BackendError::TemporaryFileError)?;
                output.into_temp_path()
            };

            let output = Command::new(&self.ffprobe)
                .args(&[FFPROBE_ARGS.clone(), vec![OsString::from(&output_path)]].concat())
                .output()
                .map_err(BackendError::FfprobeFailed)?;

            let parsed: FfprobeOutput = serde_json::from_slice(&output.stdout)
                .map_err(BackendError::MalformedFfprobeOutput)?;

            let streams = parsed.streams;
            let len = streams.len();

            if len != 1 {
                return Err(BackendError::TooManyStreams(1, len));
            }

            let stream = streams.first().unwrap();
            let codec = &stream.codec_name;

            // ffprobe sometimes returns multiple container formats, so
            // we just take the first one
            let formats = parsed.format.format_name.split(',');

            Ok(formats
                .map(|format| AudioFormat::new(format.to_owned(), codec.to_owned()))
                .collect::<Vec<_>>())
        }

        fn new(path: Option<impl AsRef<Path>>) -> Self {
            Checker {
                ffprobe: path
                    .expect("must provide ffprobe path or use ffmpeg library")
                    .as_ref()
                    .to_owned(),
            }
        }
    }
}

#[cfg(use_ffmpeg_sys)]
mod inner {
    use ffmpeg;

    use crate::errors::BackendError;

    pub struct Checker;

    impl CodecChecker for Checker {
        fn check_codec(&self, data: &[u8], expected_codec: &str) -> Result<(), BackendError> {
            todo!()
        }

        fn new(_path: impl AsRef<Path>) -> Self {
            Self
        }
    }
}
