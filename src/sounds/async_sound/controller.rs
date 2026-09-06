use std::sync::mpsc::{SendError, Sender};

use thiserror::Error;

use crate::{Sound, sounds::{AsyncSound, async_sound::worker::AsyncSoundJob}};


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AsyncSoundId(u64);

#[derive(Debug, Error)]
pub enum AsyncSoundError {
    #[error("Exhausted all ID numbers for controller")]
    ExhaustedIDs,
    
    #[error("Async sound worker stopped unexpectedly")]
    WorkerStopped,
}

pub enum AsyncSoundCommand {
    Insert(AsyncSoundId, AsyncSoundJob),
}

pub struct AsyncSoundController {
    queue_tx: Sender<AsyncSoundCommand>,
    next: u64,
}

impl AsyncSoundController {
    pub(super) const fn new(queue_tx: Sender<AsyncSoundCommand>) -> Self {
        Self {
            queue_tx,
            next: 0,
        }
    }

    pub fn append(&mut self, source: impl Into<Box<dyn Sound + Send>>) -> Result<AsyncSound, AsyncSoundError> {
        let id = AsyncSoundId(self.next);
        self.next = self.next.checked_add(1).ok_or(AsyncSoundError::ExhaustedIDs)?;

        let (sample_tx, sample_rx) = rtrb::RingBuffer::new(3);

        let source = source.into();
        let channel_count = source.channel_count();
        let sample_rate   = source.sample_rate();

        match self.queue_tx.send(AsyncSoundCommand::Insert(
            id, 
            AsyncSoundJob { source, sample_tx }
        )) {
            Ok(()) => {},
            Err(SendError(_)) => return Err(AsyncSoundError::WorkerStopped),
        }

        Ok(AsyncSound::new(id, sample_rx, channel_count, sample_rate))
    }
}