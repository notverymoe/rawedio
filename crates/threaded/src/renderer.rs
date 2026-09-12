//| Rawedio | Copyright 2026 Natalie Baker, et al | MIT / Apache License v2.0 |//

// TODO stop other threaded renderer worker

use std::marker::PhantomData;
use std::time::Duration;

use rawedio::manager::BackendSource;
use rawedio::operators::SoundMixer;
use rawedio::wrappers::{ChannelCountConverter, Controllable, Controller, SampleRateConverter};
use rawedio::{RawedioError, Sound, NextState};

use crate::{ThreadedSoundManager, ThreadedSoundRx};

pub struct ForSound;
pub struct ForBackendSource;

/// Threaded renderer backed by a sound
pub type ThreadedRendererSound<S> = ThreadedRenderer<S, ForSound>;

/// Threaded renderer backed by a mixer
pub type ThreadedRendererMixer = ThreadedRenderer<SoundMixer, ForBackendSource>;

const RENDERER_BUFFER_DURATION: Duration = Duration::from_millis(4);
const RENDERER_BUFFER_COUNT: usize = 5;

fn wrap_in_converter<S: Sound>(inner: S, sample_rate: u32, channel_count: u16) -> SampleRateConverter<ChannelCountConverter<S>> {
    SampleRateConverter::new(
        ChannelCountConverter::new(
            inner,
            channel_count
        ), 
        sample_rate
    )
}

/// A threaded backend source. Mixes on a separate thread from
/// the backend.
pub struct ThreadedRenderer<S: Sound, B: Send = ForBackendSource> {
    _thread_manager: ThreadedSoundManager<Controllable<S>>,
    mixer_controller: Controller<S>,
    inner: SampleRateConverter<ChannelCountConverter<ThreadedSoundRx>>,
    metadata_changed: bool,
    _marker: PhantomData<B>,
}

impl ThreadedRenderer<SoundMixer, ForBackendSource> {

    /// Creates a threaded renderer that pulls from a mixer
    #[must_use]
    pub fn new_mixer(
        controllable: Controllable<SoundMixer>,
        controller:   Controller<SoundMixer>
    ) -> Self {
        Self::new_inner(controllable, controller)
    }

}

impl<S: Sound + 'static> ThreadedRenderer<S, ForSound> {

    /// Creates a threaded renderer that pulls from an arbitrary sound
    #[must_use]
    pub fn new_sound(
        controllable: Controllable<S>,
        controller:   Controller<S>
    ) -> Self {
        Self::new_inner(controllable, controller)
    }

}

impl<S: Sound + 'static, B: Send> ThreadedRenderer<S, B> {
    fn new_inner(
        controllable: Controllable<S>,
        controller:   Controller<S>
    ) -> Self {

        let sample_rate   = controllable.sample_rate();
        let channel_count = controllable.channel_count();

        let mut thread_manager = ThreadedSoundManager::<Controllable<S>>::new_with_timeout(RENDERER_BUFFER_DURATION/2);
        let (_, rx) = thread_manager.add(
            RENDERER_BUFFER_DURATION,
            RENDERER_BUFFER_COUNT,
            controllable
        ).unwrap();

        ThreadedRenderer { 
            _thread_manager: thread_manager,
            mixer_controller: controller,
            inner: wrap_in_converter(rx, sample_rate, channel_count),
            metadata_changed: false,
            _marker: PhantomData,
        }
    }
}

impl<S: Sound + BackendSource> BackendSource for ThreadedRenderer<S, ForBackendSource> {
    fn set_output_channel_count_and_sample_rate(
        &mut self,
        output_channel_count: u16,
        output_sample_rate: u32,
    ) {
        self.metadata_changed = true;

        // We update our end first, so that way the next samples
        // will immediately match the requested format.
        replace_with::replace_with_or_abort(&mut self.inner, |inner| {
            let inner = inner.into_inner().into_inner();
            wrap_in_converter(inner, output_sample_rate, output_channel_count)
        });

        // Then we also send it to the mixer thread, so that maximum
        // quality is retained and we only passthrough on this thread.
        self.mixer_controller.send_command(Box::new(move |mixer| {
            mixer.set_output_channel_count_and_sample_rate(
                output_channel_count,
                output_sample_rate
            );
        }));
    }
}

impl<S: Sound> BackendSource for ThreadedRenderer<S, ForSound> {
    fn set_output_channel_count_and_sample_rate(
        &mut self,
        output_channel_count: u16,
        output_sample_rate: u32,
    ) {
        self.metadata_changed = true;

        // We update our end first, so that way the next samples
        // will immediately match the requested format.
        replace_with::replace_with_or_abort(&mut self.inner, |inner| {
            let inner = inner.into_inner().into_inner();
            wrap_in_converter(inner, output_sample_rate, output_channel_count)
        });
    }
}

impl<S: Sound, B: Send> Sound for ThreadedRenderer<S, B> {
    fn channel_count(&self) -> u16 {
        self.inner.channel_count()
    }

    fn sample_rate(&self) -> u32 {
        self.inner.sample_rate()
    }

    /// Inform the playing or queued sounds that a new batch of samples will be
    /// requested. This must only be called when the next sample to be delivered
    /// from `next_sample` is for the first channel.
    ///
    /// See [`Sound::on_start_of_batch`]
    fn on_start_of_batch(&mut self) {
        self.inner.on_start_of_batch();
    }

    /// Get the next sample.
    ///
    /// `MetadataChanged` will only be returned from Renderer if
    /// `set_output_channel_count_and_sample_rate` was called. If `Paused`
    /// is returned the backend may choose to pause itself or play silence.
    /// `Finished` will be returned if no sounds are playing and the Manager of
    /// the Renderer has been dropped.
    ///
    /// Guaranteed to not return an Error.
    fn fill_next_frames(
        &mut self,
        buffer: &mut [i16],
    ) -> Result<(usize, NextState), RawedioError> {
        if self.metadata_changed {
            self.metadata_changed = false;
            return Ok((0, NextState::MetadataChanged));
        }

        self.inner.fill_next_frames(buffer)
    }
}
