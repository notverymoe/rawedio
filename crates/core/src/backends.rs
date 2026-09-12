//| Rawedio | Copyright 2026 Natalie Baker, et al | MIT / Apache License v2.0 |//

//! Backends handle the device and OS specific logic for outputting audio.
//!
//! See comments on [Renderer][crate::manager::Renderer] for how to implement
//! a new backend which can be done in this crate or in a separate crate.

#[cfg(feature = "cpal")]
mod cpal_backend;

#[cfg(feature = "cpal")]
pub use cpal_backend::*;
