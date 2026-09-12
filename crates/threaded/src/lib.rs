//| Rawedio | Copyright 2026 Natalie Baker, et al | MIT / Apache License v2.0 |//

#![warn(missing_docs)]
#![deny(unsafe_code)]
#![cfg_attr(any(feature = "cpal", not(doctest)), doc = include_str!("../../../README.md"))]

#[allow(unsafe_code)]
pub mod sync;

mod manager;
pub use manager::ThreadedManager;

mod renderer;
pub use renderer::{ThreadedRenderer, ThreadedRendererMixer, ThreadedRendererSound};

#[cfg(feature = "cpal")]
use rawedio::backends::{CpalBackend, CpalBackendError};

/// Start outputting audio with the default backend, device, and configs.
///
/// Currently if an error occurs, it is printed to stderr but not handled in any
/// other way. This should be improved in the future.
/// For more control, create the [`CpalBackend`][backends::CpalBackend]
/// explicitly.
#[cfg(feature = "cpal")]
pub fn start() -> Result<(ThreadedManager, CpalBackend), CpalBackendError> {
    let mut backend = CpalBackend::with_defaults()
        .ok_or(CpalBackendError::NoDevice)?;

    let (manager, renderer) = ThreadedManager::new();
    backend.start_with(
        |error| eprintln!("error with cpal output stream: {error}"),
        renderer
    )?;

    Ok((manager, backend))
}
