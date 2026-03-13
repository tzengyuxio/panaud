use crate::error::{PanaudError, Result};
use crate::types::AudioData;
use std::path::Path;

/// Encode `AudioData` as a WAV file using hound.
pub fn encode_wav(audio: &AudioData, path: &Path) -> Result<()> {
    let spec = hound::WavSpec {
        channels: audio.channels,
        sample_rate: audio.sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };

    let mut writer = hound::WavWriter::create(path, spec).map_err(|e| PanaudError::EncodeError {
        message: e.to_string(),
        path: Some(path.to_path_buf()),
        suggestion: "check that the output directory exists and is writable".into(),
    })?;

    for &sample in &audio.samples {
        writer
            .write_sample(sample)
            .map_err(|e| PanaudError::EncodeError {
                message: e.to_string(),
                path: Some(path.to_path_buf()),
                suggestion: "error writing audio sample".into(),
            })?;
    }

    writer
        .finalize()
        .map_err(|e| PanaudError::EncodeError {
            message: e.to_string(),
            path: Some(path.to_path_buf()),
            suggestion: "error finalizing WAV file".into(),
        })?;

    Ok(())
}
