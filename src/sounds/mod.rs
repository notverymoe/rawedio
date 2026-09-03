//| Rawedio | Copyright 2026 Natalie Baker, et al | MIT / Apache License v2.0 |//

//! Items that return or implement [Sound][crate::Sound].
pub mod decoders;
pub mod wrappers;

#[cfg(test)]
mod tests;

mod empty;
mod memory_sound;
mod open_file;
mod silence;
mod sine_wave;
mod sound_list;
mod sound_mixer;
mod sounds_from_fn;

pub use empty::Empty;
pub use memory_sound::MemorySound;
pub use memory_sound::UnsupportedMetadataChangeError;
pub use open_file::open_file;
pub use open_file::open_file_with_buffer_capacity;
pub use silence::Silence;
pub use sine_wave::SineWave;
pub use sound_list::SoundList;
pub use sound_mixer::SoundMixer;
pub use sounds_from_fn::SoundsFromFn;
