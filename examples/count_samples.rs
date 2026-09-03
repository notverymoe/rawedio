//| Rawedio | Copyright 2026 Natalie Baker, et al | MIT / Apache License v2.0 |//

use std::error::Error;

use rawedio::{NextState, Sound};

fn main() -> Result<(), Box<dyn Error>> {
    let Some(file_path) = args() else {
        eprintln!("usage: FILE_PATH");
        std::process::exit(2);
    };

    let mut sound = rawedio::sources::open_file(file_path)?;
    let mut num_samples = 0;
    let mut scratch = vec![0; 2048];

    loop {
        let max_samples = scratch.len() / (sound.channel_count() as usize);
        let (count, next) = sound.fill_next_frames(&mut scratch[..max_samples])?;
        num_samples += count;
        match next {
            NextState::Playing => (),
            NextState::Paused => {
                println!("Encountered a Pause. Stopping.");
                break;
            }
            NextState::Finished => {
                break;
            }
            NextState::MetadataChanged => {
                println!(
                    "Encountered MetadataChanged. New sample rate: {}, New channel count: {}",
                    sound.sample_rate(),
                    sound.channel_count()
                );
            }
        }
    }
    println!("Read {num_samples} samples.");

    Ok(())
}

fn args() -> Option<String> {
    let mut args = std::env::args();
    args.next()?;
    args.next()
}
