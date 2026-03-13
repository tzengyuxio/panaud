use crate::error::{PanaudError, Result};
use std::path::Path;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::{Hint, ProbeResult};

/// Open and probe an audio file, returning symphonia's ProbeResult.
pub fn probe_file(path: &Path) -> Result<ProbeResult> {
    let file = std::fs::File::open(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            PanaudError::FileNotFound {
                path: path.to_path_buf(),
                suggestion: "check that the file path is correct".into(),
            }
        } else {
            PanaudError::IoError {
                message: e.to_string(),
                path: Some(path.to_path_buf()),
                suggestion: "check file permissions".into(),
            }
        }
    })?;

    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|e| PanaudError::DecodeError {
            message: e.to_string(),
            path: Some(path.to_path_buf()),
            suggestion: "the file may be corrupted or in an unsupported format".into(),
        })
}
