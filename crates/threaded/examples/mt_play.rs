//| Rawedio | Copyright 2026 Natalie Baker, et al | MIT / Apache License v2.0 |//

use std::{error::Error, time::Duration};

use rawedio::{Sound, wrappers::{Controllable, SetSpeed}};

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let Some(file_path) = args() else {
        eprintln!("usage: FILE_PATH");
        std::process::exit(2);
    };

    let (mut manager, _backend) = rawedio_threaded::start()?;

    // Open decoder
    let sound = rawedio::sources::open_file(file_path)?;

    // Send to decoder thread, get reciever sound back
    let (_, sound) = manager.add(Box::new(sound)).unwrap();

    // Wrap reciever sound with adjustable speed and a completion notifier
    let (sound, mut controller) = Controllable::new(sound.with_adjustable_speed_of(0.5));
    let (sound, notifier) = sound.with_adjustable_speed_of(0.5).with_completion_notifier();

    // Add reciever sound to main mixer
    manager.play(Box::new(sound));

    // Play some realtime speed warping
    let mut time = 1.0;
    while notifier.try_recv().is_err() {
        let speed = f32::sin(time)*0.2 + 1.2;
        controller.send_command(Box::new(move |sound| sound.set_speed(speed)));
        std::thread::sleep(Duration::from_millis(4));
        time += 0.05;
    }

    Ok(())
}

fn args() -> Option<String> {
    let mut args = std::env::args();
    args.next()?;
    args.next()
}
