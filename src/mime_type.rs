use crate::audio::format::AudioFormat;

#[derive(Clone, Debug)]
pub struct MimeType {
    pub id: i16,
    pub audio_format: AudioFormat,
    pub essence: String,
    pub extension: String,
}

impl MimeType {
    pub fn new(id: i16, audio_format: AudioFormat, essence: String, extension: String) -> Self {
        Self {
            id,
            audio_format,
            essence,
            extension
        }
    }
}
