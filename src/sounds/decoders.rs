//| Rawedio | Copyright 2026 Natalie Baker, et al | MIT / Apache License v2.0 |//

//! Decoders for various audio formats and file types.
//!
//! These are normally accessed via
//! [`sounds::open_file`][crate::sounds::open_file()].
#[cfg(feature = "qoa")]
mod qoa;
#[cfg(feature = "symphonia")]
mod symphonia;

#[cfg(feature = "qoa")]
pub use qoa::QoaDecoder;
#[cfg(feature = "qoa")]
pub use qoaudio::DecodeError as QoaDecodeError;
#[cfg(feature = "symphonia")]
pub use symphonia::SymphoniaDecoder;

#[cfg(test)]
mod tests;
