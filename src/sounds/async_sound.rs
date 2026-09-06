
mod controller;
mod worker;
mod sound;

use std::sync::mpsc::channel;

use worker::AsyncSoundWorker;

pub use controller::AsyncSoundController;
pub use sound::AsyncSound;

pub fn start_new_async_sound_worker() -> AsyncSoundController {

    let (queue_tx, queue_rx) = channel(); 

    std::thread::spawn(move || {
        let mut worker = AsyncSoundWorker::new(queue_rx);
        loop {
            if !worker.process_queue() {
                return;
            }
            worker.process_jobs();
            std::thread::yield_now();
        }
    });

    AsyncSoundController::new(queue_tx)
}
