//| Rawedio | Copyright 2026 Natalie Baker, et al | MIT / Apache License v2.0 |//

use std::{error::Error, io::Error as IOError};

/// Rawedio error.
///
/// These errors mostly originate in decoder libraries. If the library error is
/// just a `IOError` it is unwrapped into the `IoError` variant otherwise the
/// error is wrapped in the `FormatError` variant.
#[derive(Debug)]
pub enum RawedioError {
    /// A I/O operation failed
    IoError(IOError),
    /// An error other than `IOError` occurred. The real type of the boxed
    /// error is normally the error type of the format decoding library.
    FormatError(Box<dyn Error + Send + Sync + 'static>),
}

impl Error for RawedioError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            RawedioError::IoError(e) => e.source(),
            RawedioError::FormatError(e) => Some(e.as_ref()),
        }
    }
}

impl std::fmt::Display for RawedioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            RawedioError::IoError(ref err) => err.fmt(f),
            RawedioError::FormatError(ref inner) => write!(f, "format error: {inner}"),
        }
    }
}

impl From<IOError> for RawedioError {
    fn from(e: IOError) -> Self {
        Self::IoError(e)
    }
}
