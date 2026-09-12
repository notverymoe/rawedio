//| Rawedio | Copyright 2026 Natalie Baker, et al | MIT / Apache License v2.0 |//

use std::error::Error;

use rawedio::Sound;

fn main() -> Result<(), Box<dyn Error>> {
    let Some(file_path) = args() else {
        eprintln!("usage: FILE_PATH");
        std::process::exit(2);
    };

    let (mut manager, _backend) = rawedio_threaded::start()?;
    let (sound, notifier) = rawedio::sources::open_file(file_path)?.with_completion_notifier();

    manager.play(Box::new(sound));
    let _ = notifier.recv();

    Ok(())
}

fn args() -> Option<String> {
    let mut args = std::env::args();
    args.next()?;
    args.next()
}
