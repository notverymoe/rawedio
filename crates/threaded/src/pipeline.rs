//| Rawedio | Copyright 2026 Natalie Baker, et al | MIT / Apache License v2.0 |//

use std::hash::BuildHasherDefault;
use std::sync::mpsc::{self, Receiver, SendError, Sender, channel};
use std::thread::Thread;
use std::time::Duration;

use rawedio::Sound;
use rawedio::utils::NoHashIndexMap;
use rawedio::wrappers::SoundId;

use crate::threaded_sound::{ThreadedSoundTx, create_threaded_sound};
use crate::{RawedioThreadingError, ThreadedSoundRx};

/// Manages sounds that render on another thread
pub struct ThreadedSoundManager<S: Sound> {
    next_id: u32,
    commands: mpsc::Sender<WorkerCommand<S>>,
    thread: Thread,
}

impl<S: Sound + 'static> ThreadedSoundManager<S> {
    /// Creates a new `ThreadedSoundManager` with no timeout on
    /// thread parking, allowing the thread sleep until additional
    /// data is requested.
    ///
    /// Useful for background asset decoding.
    ///
    pub fn new() -> Result<Self, RawedioThreadingError> {
        let (tx, rx) = channel();
        Ok(Self::new_with_thread(
            tx,
            start_worker_thread(ThreadedSoundWorker::new(rx))?,
        ))
    }

    /// Creates a new `ThreadedSoundManager` with a timeout on
    /// thread parking, allowing the thread to ensure that it's
    /// executed with a minimum frequency.
    ///
    /// Useful to bound the impact of the consumer lagging.
    ///
    pub fn new_with_timeout(timeout: Duration) -> Result<Self, RawedioThreadingError> {
        let (tx, rx) = channel();
        Ok(Self::new_with_thread(
            tx,
            start_worker_thread_timeout(timeout, ThreadedSoundWorker::new(rx))?,
        ))
    }

    /// Creates a new `ThreadedSoundManager` with a custom thread. Should
    /// respond to commands sent via the provided Sender.
    ///
    /// The provided thread will have `Thread::unpark` called on it to
    /// indicate that there are new commands or free buffers to process.
    ///
    #[must_use]
    pub const fn new_with_thread(commands: Sender<WorkerCommand<S>>, thread: Thread) -> Self {
        Self {
            next_id: 0,
            commands,
            thread,
        }
    }

    /// Move a sound to the worker thread, returns a `ThreadedSoundRx`
    /// which allows you to receive sample data from the thread.
    ///
    /// `buffer_ms` is how long a sample chunk should be in ms
    /// `buffer_count` is how sample chunks should be buffered
    ///
    pub fn add(
        &mut self,
        buffer_dur: Duration,
        buffer_count: usize,
        sound: S,
    ) -> Result<(SoundId, ThreadedSoundRx), SendError<WorkerCommand<S>>> {
        let (tx, rx) =
            create_threaded_sound(buffer_dur, buffer_count, sound, Some(self.thread.clone()));

        let id = SoundId::from_inner(self.next_id);
        self.next_id += 1;

        self.commands.send(WorkerCommand::Insert(id, tx))?;
        self.thread.unpark();
        Ok((id, rx))
    }

    /// Removes a sound from the rendering thread, effectively stopping
    /// it. The `ThreadedSoundRx` will return `NextState::Finished`
    /// when it next runs out of buffered samples.
    ///
    pub fn remove(&mut self, id: SoundId) -> Result<(), SendError<WorkerCommand<S>>> {
        self.commands.send(WorkerCommand::Remove(id))?;
        self.thread.unpark();
        Ok(())
    }
}

impl<S: Sound> Drop for ThreadedSoundManager<S> {
    fn drop(&mut self) {
        // Try to stop the other thread, in case we
        // were dropped but the application is still
        // running. For example, if the user recreates
        // the threaded renderer backend for some reason.
        //
        // If the thread is already stopped, that's fine.
        let _ = self.commands.send(WorkerCommand::Stop);
        self.thread.unpark();
    }
}

pub enum WorkerCommand<S: Sound> {
    Insert(SoundId, ThreadedSoundTx<S>),
    Remove(SoundId),
    Stop,
}

pub struct ThreadedSoundWorker<S: Sound> {
    commands: Option<mpsc::Receiver<WorkerCommand<S>>>,
    jobs: NoHashIndexMap<SoundId, ThreadedSoundTx<S>>,
    to_remove: Vec<SoundId>,
}

impl<S: Sound> ThreadedSoundWorker<S> {
    pub fn new(rx: Receiver<WorkerCommand<S>>) -> Self {
        Self {
            commands: Some(rx),
            jobs: NoHashIndexMap::with_capacity_and_hasher(32, BuildHasherDefault::default()),
            to_remove: Vec::with_capacity(32),
        }
    }

    pub const fn is_running(&self) -> bool {
        self.commands.is_some()
    }

    pub fn update(&mut self) {
        let Some(commands) = &self.commands else {
            return;
        };

        // Process commands
        for command in commands.try_iter() {
            match command {
                WorkerCommand::Insert(id, value) => {
                    self.jobs.insert(id, value);
                }
                WorkerCommand::Remove(id) => {
                    self.jobs.swap_remove(&id);
                }
                WorkerCommand::Stop => {
                    self.commands = None;
                    return;
                }
            }
        }

        // Process jobs
        for (id, job) in &mut self.jobs {
            if !job.is_connected() {
                self.to_remove.push(*id);
                continue;
            }

            job.update();
        }

        // Remove finished/errored jobs
        for id in self.to_remove.drain(..).rev() {
            self.jobs.swap_remove(&id);
        }
    }
}

fn start_worker_thread<S: Sound + 'static>(
    mut tx: ThreadedSoundWorker<S>,
) -> Result<Thread, RawedioThreadingError> {
    Ok(std::thread::Builder::new()
        .name("threaded_rawedio_renderer".to_owned())
        .spawn(move || {
            log::info!("th start");
            while tx.is_running() {
                tx.update();
                std::thread::park();
            }
            log::info!("th end");
        })?
        .thread()
        .clone())
}

fn start_worker_thread_timeout<S: Sound + 'static>(
    park_timeout: Duration,
    mut tx: ThreadedSoundWorker<S>,
) -> Result<Thread, RawedioThreadingError> {
    Ok(std::thread::Builder::new()
        .name("threaded_rawedio_renderer".to_owned())
        .spawn(move || {
            while tx.is_running() {
                tx.update();
                std::thread::park_timeout(park_timeout);
            }
        })?
        .thread()
        .clone())
}
