//| Rawedio | Copyright 2026 Natalie Baker, et al | MIT / Apache License v2.0 |//

#![warn(missing_docs)]
#![forbid(unsafe_code)]
#![cfg_attr(any(feature = "cpal", not(doctest)), doc = include_str!("../README.md"))]

pub mod backends;
pub mod decoders;
pub mod manager;
pub mod operators;
pub mod sources;
pub mod utils;
pub mod wrappers;

mod error;
mod legacy;
mod sound;

pub use error::RawedioError;
pub use sound::{NextState, Sound};

/// Start outputting audio with the default backend, device, and configs.
///
/// Currently if an error occurs, it is printed to stderr but not handled in any
/// other way. This should be improved in the future.
/// For more control, create the [`CpalBackend`][backends::CpalBackend]
/// explicitly.
#[cfg(feature = "cpal")]
pub fn start() -> Result<(manager::Manager, backends::CpalBackend), backends::CpalBackendError> {
    let mut backend =
        backends::CpalBackend::with_defaults().ok_or(backends::CpalBackendError::NoDevice)?;
    let manager = backend.start(|error| eprintln!("error with cpal output stream: {error}"))?;
    Ok((manager, backend))
}
