
use std::{collections::HashMap, sync::{mpsc::{Receiver, TryRecvError}}};

use crate::{NextSampleBuffer, Sound, sounds::async_sound::{controller::{AsyncSoundCommand, AsyncSoundId}, sound::{ASYNC_SAMPLE_CHUNK_CAP, AsyncSampleChunk}}};

pub struct AsyncSoundJob {
    pub source: Box<dyn Sound + Send>,
    pub sample_tx: rtrb::Producer<AsyncSampleChunk>,
}

impl AsyncSoundJob {

    pub fn push_chunk(&mut self, value: AsyncSampleChunk) {
        if matches!(
            self.sample_tx.push(value), 
            Err(rtrb::PushError::Full(_))
        ) {
            unreachable!() // Already counted slots
        }
    }

}

pub struct AsyncSoundWorker {
    queue_rx: Receiver<AsyncSoundCommand>,
    jobs: HashMap<AsyncSoundId, AsyncSoundJob>,
    to_remove: Vec<AsyncSoundId>,
}

impl AsyncSoundWorker {

    pub(super) fn new(queue_rx: Receiver<AsyncSoundCommand>) -> Self {
        Self {
            queue_rx,
            jobs: HashMap::default(),
            to_remove: Vec::default(),
        }
    }

    pub fn process_queue(&mut self) -> bool {
        loop {
            match self.queue_rx.try_recv() {
                Ok(AsyncSoundCommand::Insert(id, sound)) => { self.jobs.insert(id, sound); },
                Err(TryRecvError::Disconnected) => return !self.jobs.is_empty(),
                Err(TryRecvError::Empty) => return true,
            }
        }
    }

    pub fn process_jobs(&mut self) {
        
        // Send batch signal to all first, matching how the mixer and
        // backends approach this. Then process all sound slots.
        for (id, job) in &mut self.jobs {
            if job.sample_tx.is_abandoned() {
                self.to_remove.push(*id);
                continue;
            }

            job.source.on_start_of_batch();
        }

        self.remove_pending();

        for (id, job) in &mut self.jobs {
            let slots = job.sample_tx.slots();
            if slots == 0 {
                continue;
            }

            if !process_job(job, slots) {
                self.to_remove.push(*id);
            }
        }

        self.remove_pending();
    }

    fn remove_pending(&mut self) {
        for id in &self.to_remove {
            self.jobs.remove(id);
        }
        self.to_remove.clear();
    }

}

fn process_job(job: &mut AsyncSoundJob, slots: usize) -> bool {

    let channels = job.source.channel_count() as usize;
    let mut max_samples = ASYNC_SAMPLE_CHUNK_CAP/channels * channels;
    
    for _ in 0..slots {
        let mut data = [0_i16; ASYNC_SAMPLE_CHUNK_CAP];
        match job.source.next_samples_for(&mut data[..max_samples]) {
            Ok(NextSampleBuffer::Continue) => {
                job.push_chunk(AsyncSampleChunk{
                    response: NextSampleBuffer::Continue,
                    max_samples,
                    data,
                    channel_count: job.source.channel_count(),
                    sample_rate:   job.source.sample_rate(),
                });
            },
            Ok(NextSampleBuffer::Finished(count)) => {
                data[count..max_samples].fill(0);
                job.push_chunk(AsyncSampleChunk{
                    response: NextSampleBuffer::MetadataChanged(count),
                    max_samples,
                    data,
                    channel_count: job.source.channel_count(),
                    sample_rate:   job.source.sample_rate(),
                });
                return false; // Finished, remove sound.
            },
            Ok(NextSampleBuffer::MetadataChanged(count)) => {
                data[count..max_samples].fill(0);
                job.push_chunk(AsyncSampleChunk{
                    response: NextSampleBuffer::MetadataChanged(count),
                    max_samples,
                    data,
                    channel_count: job.source.channel_count(),
                    sample_rate:   job.source.sample_rate(),
                });
                // Recalculate max samples with new config, continue
                // to next chunk/slot with the reconfigured stream.
                max_samples = ASYNC_SAMPLE_CHUNK_CAP/channels * channels;
            },
            Ok(NextSampleBuffer::Paused(count)) => {
                data[count..max_samples].fill(0);
                job.push_chunk(AsyncSampleChunk{
                    response: NextSampleBuffer::Paused(count),
                    max_samples,
                    data,
                    channel_count: job.source.channel_count(),
                    sample_rate:   job.source.sample_rate(),
                });
                return true; // Paused, no more filling as part of this "batch"
            }
            Err(e) => {
                log::error!("dropping sound in AsyncSoundWorker which returned error: {e}");
                return false; // Error, remove sound
            },
        }
    }

    true
}