use std::str::FromStr;

const DELIMITER: char = '/';

#[derive(Clone, Debug, PartialEq)]
pub struct AudioFormat {
    pub container: String,
    pub codec: String,
}

impl AudioFormat {
    pub fn new(container: String, codec: String) -> Self {
        AudioFormat { container, codec }
    }
}

impl FromStr for AudioFormat {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split(DELIMITER);

        match (parts.next(), parts.next(), parts.next()) {
            (Some(format), Some(codec), None) => {
                Ok(AudioFormat::new(format.to_owned(), codec.to_owned()))
            }
            _ => Err(ParseError),
        }
    }
}

#[derive(Debug)]
pub struct ParseError;
