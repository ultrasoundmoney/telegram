use std::{error, fmt};

/// Errors returned while building or sending Telegram messages.
#[derive(Debug)]
pub enum Error {
    CallbackDataTooLong { bytes: usize },
    MessageTooLong { chars: usize, limit: usize },
    Http(String),
    Telegram(String),
    Json(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CallbackDataTooLong { bytes } => {
                write!(f, "telegram callback data is {bytes} bytes, limit is 64")
            }
            Self::MessageTooLong { chars, limit } => {
                write!(f, "telegram message is {chars} chars, limit is {limit}")
            }
            Self::Http(error) => write!(f, "telegram http error: {error}"),
            Self::Telegram(error) => write!(f, "telegram api error: {error}"),
            Self::Json(error) => write!(f, "telegram json error: {error}"),
        }
    }
}

impl error::Error for Error {}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error.to_string())
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::Http(error.to_string())
    }
}

#[cfg(feature = "async")]
impl From<reqwest::Error> for Error {
    fn from(error: reqwest::Error) -> Self {
        Self::Http(error.to_string())
    }
}

#[cfg(feature = "blocking")]
impl From<ureq::Error> for Error {
    fn from(error: ureq::Error) -> Self {
        Self::Http(error.to_string())
    }
}
