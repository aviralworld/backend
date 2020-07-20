use crate::errors::BackendError;

pub trait CodecChecker {
    fn is_codec(&self, data: &[u8], expected_codec: &str) -> Result<bool, BackendError>;

    fn new(ffprobe_path: Option<String>) -> Self;
}

pub fn make_wrapper(ffprobe_path: Option<String>, expected_codec: String) -> impl Fn(&[u8]) -> Result<(), BackendError> {
    let checker = inner::Checker::new(ffprobe_path);
    let expected_codec = expected_codec;

    move |data: &[u8]| checker.is_codec(data, &expected_codec).map(|_| ())
}

#[cfg(not(use_ffmpeg_sys))]
mod inner {
    use std::ffi::OsString;
    use std::path::{Path, PathBuf};

    use lazy_static::lazy_static;

    use crate::errors::BackendError;

    lazy_static! {
        static ref FFPROBE_ARGS: Vec<OsString> = vec![
            OsString::from("-hide_banner"),
            OsString::from("-v"),
            OsString::from("error"),
            OsString::from("-of"),
            OsString::from("json"),
            OsString::from("-show_entries"),
            OsString::from("stream=codec_name"),
        ];
    }

    pub struct Checker {
        ffprobe: PathBuf,
    }

    impl Checker {
    }

    impl super::CodecChecker for Checker {
        fn is_codec(&self, data: &[u8], expected_codec: &str) -> Result<bool, BackendError> {
            use std::io::Write;
            use std::process::Command;

            use serde::Deserialize;
            use tempfile::NamedTempFile;

            #[derive(Deserialize)]
            struct FfprobeOutput {
                programs: Vec<String>,
                streams: Vec<FfprobeStream>,
            }

            #[derive(Deserialize)]
            struct FfprobeStream {
                codec_name: String,
            }

            let output_path = {
                let mut output = NamedTempFile::new()
                    .map_err(|e| BackendError::TemporaryFileError(e))?;
                output
                    .write_all(data)
                    .map_err(BackendError::TemporaryFileError)?;
                output.into_temp_path()
            };

            // TODO handle malformed output differently
            let output = Command::new(&self.ffprobe)
                .args(&[FFPROBE_ARGS.clone(), vec![OsString::from(&output_path)]].concat())
                .output()
                .map_err(BackendError::FfprobeFailed)?;

            let parsed: FfprobeOutput =
                serde_json::from_slice(&output.stdout).map_err(BackendError::MalformedFfprobeOutput)?;

            let streams = parsed.streams;
            let len = streams.len();

            if len != 1 {
                return Err(BackendError::TooManyStreams(1, len));
            }

            let stream = streams.first().unwrap();
            let codec = &stream.codec_name;
            Ok(codec == expected_codec)
        }

        fn new(path: Option<String>) -> Self {
            Checker {
                ffprobe: PathBuf::from(path.expect("must provide ffprobe path or use ffmpeg library")),
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
        fn is_codec(&self, data: &[u8], expected_codec: &str) -> Result<bool, BackendError> {
            unimplemented!()
        }

        fn new(_path: impl AsRef<Path>) -> Self {
            Self
        }
    }
}
