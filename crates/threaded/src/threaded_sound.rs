//| Rawedio | Copyright 2026 Natalie Baker, et al | MIT / Apache License v2.0 |//

use std::thread::Thread;
use std::time::Duration;

use portable_atomic::{AtomicUsize, Ordering};
use rawedio::{NextState, RawedioError, Sound};

use crate::sync::{PoolQueueCons, PoolQueueProd, create_pool_queue};

const fn calculate_buffer_size(
    duration: Duration,
    sample_rate: usize,
    channel_count: usize,
) -> usize {
    let secs = duration.as_secs_f64();
    let samples = f64::floor(secs * sample_rate as f64) as usize;
    samples * channel_count
}

/// Creates a tx/rx pair of sounds that wraps a source sound. The
/// `ThreadedSoundTx` will fill buffers of the given duration and
/// send them to the `ThreadedSoundRx` to sample from. Ensure
/// that `buffer_dur` and `buffer_count` have the right latency
/// and capacity so the `ThreadedSoundRx` doesn't starve. If it has
/// no remaining samples, it will pause.
pub fn create_threaded_sound<S: Sound>(
    buffer_dur: Duration,
    buffer_count: usize,
    inner: S,
    wake_thread: Option<Thread>,
) -> (ThreadedSoundTx<S>, ThreadedSoundRx) {
    let sample_rate = inner.sample_rate();
    let channel_count = inner.channel_count();

    let buffer_size = calculate_buffer_size(buffer_dur, sample_rate, channel_count);

    let (tx, rx) = create_pool_queue(buffer_count, || SampleChunk {
        channel_count,
        sample_rate,
        samples: vec![0.0; buffer_size],
        next_state: NextState::Playing,
    });

    (
        ThreadedSoundTx {
            tx: Some(tx),
            inner,
            buffer_dur,
        },
        ThreadedSoundRx {
            rx,
            sample_rate,
            channel_count,
            samples_filled: 0,
            wake_thread,
            underrun_count: AtomicUsize::new(0),
        },
    )
}

struct SampleChunk {
    samples: Vec<f32>,
    next_state: NextState,
    sample_rate: usize,
    channel_count: usize,
}

/// A sound that receives audio samples
/// buffered from another thread.
pub struct ThreadedSoundRx {
    rx: PoolQueueCons<SampleChunk>,
    sample_rate: usize,
    channel_count: usize,
    samples_filled: usize,
    wake_thread: Option<Thread>,
    underrun_count: AtomicUsize,
}

impl ThreadedSoundRx {
    /// Returns the total number of samples that
    /// have been underrun, because the tx thread
    /// failed to supply them in time. Ideally 0.
    #[must_use]
    pub fn underrun_count(&self) -> usize {
        self.underrun_count.load(Ordering::Relaxed)
    }
}

impl Sound for ThreadedSoundRx {
    fn channel_count(&self) -> usize {
        self.channel_count
    }

    fn sample_rate(&self) -> usize {
        self.sample_rate
    }

    fn fill_next_frames(&mut self, buffer: &mut [f32]) -> Result<(usize, NextState), RawedioError> {
        let mut next_state = NextState::Playing;
        let mut samples_written = 0;
        while (samples_written < buffer.len()) 
            && let Some(mut slot) = self.rx.dequeue()
        {
            slot.dismiss(); // We'll manually return the slot to the tx

            let sample_count = usize::min(
                buffer.len() - samples_written,
                slot.samples.len() - self.samples_filled,
            );

            let src = &slot.samples[self.samples_filled..self.samples_filled + sample_count];
            let dst = &mut buffer[samples_written..samples_written + sample_count];
            dst.copy_from_slice(src);

            self.samples_filled += sample_count;
            samples_written += sample_count;

            if self.samples_filled >= slot.samples.len() {
                next_state = slot.next_state;
                self.samples_filled = 0;
                self.sample_rate = slot.sample_rate;
                self.channel_count = slot.channel_count;
                slot.commit();
                if let Some(wake_thread) = &mut self.wake_thread {
                    wake_thread.unpark();
                }
                if next_state != NextState::Playing {
                    break;
                }
            }
        }

        if samples_written < buffer.len() && next_state == NextState::Playing {
            next_state = NextState::WouldBlock;
            self.underrun_count
                .fetch_add(buffer.len() - samples_written, Ordering::Relaxed);
        }

        // If there are no slots with samples and it's disconnected, then stop the sound.
        if !self.rx.is_connected() {
            next_state = NextState::Finished;
        }

        Ok((samples_written, next_state))
    }
}

pub struct ThreadedSoundTx<S: Sound> {
    tx: Option<PoolQueueProd<SampleChunk>>,
    inner: S,
    buffer_dur: Duration,
}

impl<S: Sound> ThreadedSoundTx<S> {
    pub fn is_connected(&self) -> bool {
        self.tx
            .as_ref()
            .is_some_and(super::sync::PoolQueueProd::is_connected)
    }

    pub fn update(&mut self) -> bool {
        let Some(tx) = &mut self.tx else {
            return false;
        };

        if !tx.is_connected() {
            self.tx = None;
            return false;
        }

        let mut should_stop = false;
        while let Some(mut buffer) = tx.enqueue() {
            buffer.samples.resize(
                calculate_buffer_size(
                    self.buffer_dur,
                    self.inner.sample_rate(),
                    self.inner.channel_count(),
                ),
                0.0,
            );

            self.inner.on_start_of_batch();

            match self.inner.fill_next_frames(&mut buffer.samples) {
                Ok((count, next)) => {
                    buffer.samples.truncate(count);
                    buffer.next_state = next;
                    buffer.channel_count = self.inner.channel_count();
                    buffer.sample_rate = self.inner.sample_rate();
                    if next == NextState::Finished {
                        should_stop = true;
                        break;
                    }
                }
                Err(e) => {
                    log::error!("transmit sound stopping, received error: {e}");
                    buffer.samples.clear();
                    buffer.next_state = NextState::Finished;
                    should_stop = true;
                    break;
                }
            }
        }

        if should_stop {
            self.tx = None;
        }

        !should_stop
    }
}
