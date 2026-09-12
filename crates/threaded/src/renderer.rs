//| Rawedio | Copyright 2026 Natalie Baker, et al | MIT / Apache License v2.0 |//

use std::marker::PhantomData;
use std::time::{Duration, Instant};

use rawedio::manager::BackendSource;
use rawedio::operators::SoundMixer;
use rawedio::wrappers::{ChannelCountConverter, Controllable, Controller, SampleRateConverter};
use rawedio::{RawedioError, Sound, NextState};

use crate::sync::{PoolQueueCons, PoolQueueProd, create_pool_queue};

const BUFFER_MS:    usize = 4;
const BUFFER_COUNT: usize = 5;

const fn calculate_buffer_size(channel_count: usize, sample_rate: usize) -> usize {
    (channel_count*sample_rate*BUFFER_MS)/1000
}

struct RendererChunk {
    samples: Vec<i16>,
    next_state: NextState,
    sample_rate: u32,
    channel_count: u16,
}

pub struct ForSound;
pub struct ForBackendSource;

/// Threaded renderer backed by a sound
pub type ThreadedRendererSound<S> = ThreadedRenderer<S, ForSound>;

/// Threaded renderer backed by a mixer
pub type ThreadedRendererMixer = ThreadedRenderer<SoundMixer, ForBackendSource>;

/// A threaded backend source. Mixes on a separate thread from
/// the backend.
pub struct ThreadedRenderer<S: Sound, B: Send = ForBackendSource> {
    mixer_controller: Controller<S>,
    inner: SampleRateConverter<ChannelCountConverter<ThreadedSound>>,
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

        let buffer_size = calculate_buffer_size(
            channel_count as usize,
            sample_rate   as usize
        );

        let (tx, rx) = create_pool_queue(BUFFER_COUNT, || RendererChunk{
            channel_count,
            sample_rate,
            samples:    vec![0; buffer_size],
            next_state: NextState::Playing
        });

        std::thread::Builder::new()
            .name("threaded_rawedio_renderer".to_owned())
            .spawn(make_worker_thread(controllable, tx))
            .unwrap();

        ThreadedRenderer { 
            mixer_controller: controller,
            inner: SampleRateConverter::new(
                ChannelCountConverter::new(
                    ThreadedSound { 
                        rx, 
                        sample_rate, 
                        channel_count,
                        samples_filled: 0,
                    },
                    channel_count
                ), 
                sample_rate
            ),
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
            SampleRateConverter::new(
                ChannelCountConverter::new(
                    inner,
                    output_channel_count
                ), 
                output_sample_rate
            )
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
            SampleRateConverter::new(
                ChannelCountConverter::new(
                    inner,
                    output_channel_count
                ), 
                output_sample_rate
            )
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


fn make_worker_thread<S: Sound>(
    mut src: Controllable<S>,
    mut tx: PoolQueueProd<RendererChunk>,
) -> impl FnOnce() {
    move || {
        let mut max_sleep = Duration::from_millis(BUFFER_MS as u64);
        while tx.is_connected() {
            let start = Instant::now();

            let Some(mut buffer) = tx.enqueue() else {
                std::thread::sleep(max_sleep/2);
                continue;
            };

            buffer.samples.resize(
                calculate_buffer_size(
                    src.channel_count() as usize,
                    src.sample_rate()   as usize
                ),
                0
            );

            src.on_start_of_batch();

            let (samples, next_state) = src
                .fill_next_frames(&mut buffer.samples) 
                .unwrap(); // Never returns Err

            buffer.samples.truncate(samples);
            buffer.next_state = next_state;
            buffer.channel_count = src.channel_count();
            buffer.sample_rate = src.sample_rate();

            max_sleep = Duration::from_millis(BUFFER_MS as u64)
                .checked_sub(start.elapsed())
                .unwrap_or_default();
        }
    }
}


pub struct ThreadedSound {
    rx: PoolQueueCons<RendererChunk>,
    sample_rate: u32,
    channel_count: u16,
    samples_filled: usize,
}

impl Sound for ThreadedSound {
    fn channel_count(&self) -> u16 {
        self.channel_count
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn fill_next_frames(&mut self, buffer: &mut [i16]) -> Result<(usize, NextState), RawedioError> {
        
        let mut next_state = NextState::Playing;
        let mut samples_written = 0;
        let mut did_recv_slot = false;
        while (samples_written < buffer.len()) && let Some(mut slot) = self.rx.dequeue() {
            slot.dismiss(); // We'll manually return the slot to the tx
            did_recv_slot = true;

            let sample_count = usize::min(buffer.len() - samples_written, slot.samples.len() - self.samples_filled);

            let src = &slot.samples[self.samples_filled..self.samples_filled+sample_count];
            let dst = &mut buffer[samples_written..samples_written+sample_count];
            dst.copy_from_slice(src);

            self.samples_filled += sample_count;
            samples_written += sample_count;

            if self.samples_filled >= slot.samples.len() {
                next_state = slot.next_state;
                self.samples_filled = 0;
                self.sample_rate = slot.sample_rate;
                self.channel_count = slot.channel_count;
                slot.commit();
                if next_state != NextState::Playing {
                    break;
                }
            }
        }

        // If there are no slots with samples and it's disconnected, then stop the sound.
        if !did_recv_slot && !self.rx.is_connected() {
            next_state = NextState::Finished;
        }

        Ok((samples_written, next_state))
    }

}