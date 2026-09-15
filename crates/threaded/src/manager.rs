//| Rawedio | Copyright 2026 Natalie Baker, et al | MIT / Apache License v2.0 |//

use std::error::Error;
use std::time::Duration;

use rawedio::Sound;
use rawedio::operators::SoundMixer;
use rawedio::wrappers::{Controllable, Controller, SoundId};

use crate::{RawedioThreadingError, ThreadedRendererMixer, ThreadedSoundManager, ThreadedSoundRx};

const BUFFER_DECODE_DUR: Duration = Duration::from_millis(100);
const BUFFER_DECODE_COUNT: usize = 4;

/// A Manager can play sounds by rendering sounds on a [`Renderer`] for a
/// backend.
pub struct ThreadedManager {
    mixer_controller: Controller<SoundMixer>,
    decode_manager: ThreadedSoundManager<Box<dyn Sound>>,
}

// These are undocumented, should not be relied on and subject to change.
// Backend implementations should set their values at startup.
const DEFAULT_CHANNEL_COUNT: usize = 1;
const DEFAULT_SAMPLE_RATE: usize = 1000; // Purposely low value to discourage use

impl ThreadedManager {
    /// Create a new Manager and the renderer its samples will render to.
    ///
    /// Normally you do not need to call this function directly but you instead
    /// call `.start(...)` on a backend which will call this function.
    pub fn new() -> Result<(Self, ThreadedRendererMixer), RawedioThreadingError> {
        let (mixer, mixer_controller) =
            Controllable::new(SoundMixer::new(DEFAULT_CHANNEL_COUNT, DEFAULT_SAMPLE_RATE));
        let decode_manager = ThreadedSoundManager::new()?;
        let renderer = ThreadedRendererMixer::new_mixer(mixer, mixer_controller.clone())?;
        let manager = ThreadedManager {
            mixer_controller,
            decode_manager,
        };
        Ok((manager, renderer))
    }

    /// Add a new Sound to be played in parallel to any existing sounds.
    ///
    /// If you want to play Sounds sequentially use a
    /// [`SoundList`][crate::sounds::SoundList].
    ///
    /// See the modifier functions on [Sound] to control sounds before and/or
    /// after playing.
    pub fn play(&mut self, sound: Box<dyn Sound>) {
        self.mixer_controller.add(sound);
    }

    /// Stop playing and remove all audio sounds. New sounds can still be added.
    pub fn clear(&mut self) {
        self.mixer_controller.clear();
    }

    /// Add a sound to the decoder / generator thread
    pub fn add(
        &mut self,
        sound: Box<dyn Sound>,
    ) -> Result<(SoundId, ThreadedSoundRx), Box<dyn Error>> {
        Ok(self
            .decode_manager
            .add(BUFFER_DECODE_DUR, BUFFER_DECODE_COUNT, sound)?)
    }

    /// Add a sound to the decoder / generator thread
    pub fn remove(&mut self, id: SoundId) -> Result<(), Box<dyn Error>> {
        Ok(self.decode_manager.remove(id)?)
    }
}

impl std::fmt::Debug for ThreadedManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ThreadedManager").finish()
    }
}
