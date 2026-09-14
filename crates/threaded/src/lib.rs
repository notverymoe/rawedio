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

mod pipeline;
pub use pipeline::ThreadedSoundManager;

mod threaded_sound;
use thiserror::Error;
pub use threaded_sound::ThreadedSoundRx;

#[cfg(feature = "cpal")]
use rawedio::backends::{CpalBackend, CpalBackendError};

/// Error related to the threaded rawedio renderer / sounds
#[derive(Debug, Error)]
pub enum RawedioThreadingError {
    /// Created when instantiating a backend and the backend
    /// returns an error. Could be any error type, so must be
    /// boxed explicitly.
    #[error("Encountered backend error: {0}")]
    Backend(Box<dyn std::error::Error>),

    /// Standard IO error. Usually only occurs when creating the thread.
    #[error("Encountered io error: {0}")]
    Io(#[from] std::io::Error),

    /// Error sending a command to the threaded worker, usually
    /// means that the worker has crashed or otherwise stopped
    /// prematurely.
    #[error("Encountered error in when communicating with audio worker")]
    WorkerError,
}

/// Start outputting audio with the default backend, device, and configs.
///
/// Currently if an error occurs, it is printed to stderr but not handled in any
/// other way. This should be improved in the future.
/// For more control, create the [`CpalBackend`][backends::CpalBackend]
/// explicitly.
#[cfg(feature = "cpal")]
pub fn start() -> Result<(ThreadedManager, CpalBackend), RawedioThreadingError> {
    let mut backend = CpalBackend::with_defaults()
        .ok_or(CpalBackendError::NoDevice)
        .map_err(|e| RawedioThreadingError::Backend(e.into()))?;

    let (manager, renderer) = ThreadedManager::new()?;
    backend.start_with(
        |error| eprintln!("error with cpal output stream: {error}"),
        renderer
    ).map_err(|e| RawedioThreadingError::Backend(e.into()))?;

    Ok((manager, backend))
}
