//| Rawedio | Copyright 2026 Natalie Baker, et al | MIT / Apache License v2.0 |//

//! Sound implementations that act as a base audio source.

mod empty;
mod memory_sound;
mod open_file;
mod silence;
mod sine_wave;

pub use empty::Empty;
pub use memory_sound::{MemorySound, UnsupportedMetadataChangeError};
pub use open_file::{open_file, open_file_with_buffer_capacity};
pub use silence::Silence;
pub use sine_wave::SineWave;
