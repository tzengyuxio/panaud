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

    let mut writer = hound::WavWriter::create(path, spec)
        .map_err(|e| PanaudError::encode(path, e.to_string(), "check that the output directory exists and is writable"))?;

    for &sample in &audio.samples {
        writer
            .write_sample(sample)
            .map_err(|e| PanaudError::encode(path, e.to_string(), "error writing audio sample"))?;
    }

    writer
        .finalize()
        .map_err(|e| PanaudError::encode(path, e.to_string(), "error finalizing WAV file"))?;

    Ok(())
}
