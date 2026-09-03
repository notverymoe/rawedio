//| Rawedio | Copyright 2026 Natalie Baker, et al | MIT / Apache License v2.0 |//

use std::fs::File;
use std::io::BufReader;

use crate::{RawedioError, Sound};

/// Create a Sound that reads from a file with the correct decoder based on the
/// file extension.
///
/// If the file type is not able to be decoded than an
/// [`std::io::ErrorKind::Unsupported`] is returned.
///
/// Uses a `BufReader` internally with the default capacity.
///
/// The returned Sound reads using File. This is generally not recommended
/// on the renderer thread as reading from a file could block the renderer.
/// Consider convert the sound to a `memory_sound` which is stored entirely in RAM
/// (and can be cloned cheaply).
pub fn open_file<P: AsRef<std::path::Path>>(path: P) -> Result<Box<dyn Sound>, RawedioError> {
    let file = File::open(path.as_ref())?;
    let reader = BufReader::new(file);
    open_file_with_reader(path.as_ref(), reader)
}

/// Same as `open_file` but with an explicit `BufReader` capacity.
pub fn open_file_with_buffer_capacity<P: AsRef<std::path::Path>>(
    path: P,
    buffer_capacity: usize,
) -> Result<Box<dyn Sound>, RawedioError> {
    let file = File::open(path.as_ref())?;
    let reader = BufReader::with_capacity(buffer_capacity, file);
    open_file_with_reader(path.as_ref(), reader)
}

fn open_file_with_reader(
    path: &std::path::Path,
    reader: BufReader<File>,
) -> Result<Box<dyn Sound>, RawedioError> {
    let extension = path
        .extension()
        .unwrap_or_default()
        .to_str()
        .unwrap_or_default()
        .to_lowercase();

    #[allow(clippy::match_single_binding)]
    let decoder: Box<dyn Sound> = match extension.as_ref() {
        #[cfg(feature = "qoa")]
        "qoa" => Box::new(crate::decoders::QoaDecoder::new(reader)?),
        #[cfg(feature = "symphonia")]
        _ => Box::new(crate::decoders::SymphoniaDecoder::new(
            Box::new(reader.into_inner()),
            Some(&extension),
        )?),
        #[cfg(not(feature = "symphonia"))]
        _ => return Err(std::io::Error::from(std::io::ErrorKind::Unsupported).into()),
    };
    Ok(decoder)
}
