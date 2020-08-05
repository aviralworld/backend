use crate::errors::BackendError;

pub trait CodecChecker {
    fn check_codec(
        &self,
        data: &[u8],
        expected_codec: &str,
        expected_format: &str,
    ) -> Result<(), BackendError>;

    fn new(ffprobe_path: Option<String>) -> Self;
}

pub fn make_wrapper(
    ffprobe_path: Option<String>,
    expected_codec: String,
    expected_format: String,
) -> impl Fn(&[u8]) -> Result<(), BackendError> {
    let checker = inner::Checker::new(ffprobe_path);
    let expected_codec = expected_codec;

    move |data: &[u8]| {
        checker
            .check_codec(data, &expected_codec, &expected_format)
            .map(|_| ())
    }
}

#[cfg(not(use_ffmpeg_sys))]
mod inner {
    use std::ffi::OsString;
    use std::path::PathBuf;

    use lazy_static::lazy_static;
    use serde::Deserialize;

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
        fn check_codec(
            &self,
            data: &[u8],
            expected_codec: &str,
            expected_format: &str,
        ) -> Result<(), BackendError> {
            use std::io::Write;
            use std::process::Command;

            use tempfile::NamedTempFile;

            let output_path = {
                let mut output =
                    NamedTempFile::new().map_err(|e| BackendError::TemporaryFileError(e))?;
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
            let format = &parsed.format.format_name;

            eprintln!(
                "comparing actual {:?} inside {:?} to expected {:?} inside {:?}",
                codec, format, expected_codec, expected_format
            );

            if codec == expected_codec && format == expected_format {
                return Ok(());
            }

            Err(BackendError::WrongMediaType {
                actual_codec: codec.to_owned(),
                expected_codec: expected_codec.to_owned(),
                actual_format: format.to_owned(),
                expected_format: expected_format.to_owned(),
            })
        }

        fn new(path: Option<String>) -> Self {
            Checker {
                ffprobe: PathBuf::from(
                    path.expect("must provide ffprobe path or use ffmpeg library"),
                ),
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
