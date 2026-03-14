use crate::error::{PanaudError, Result};
use crate::types::AudioData;
use std::path::Path;

/// Encode `AudioData` as a FLAC file using flacenc.
pub fn encode_flac(audio: &AudioData, path: &Path) -> Result<()> {
    use flacenc::component::BitRepr;
    use flacenc::error::Verify;

    let bits_per_sample: usize = 16;
    let samples_i32 = audio.samples_as_i32(bits_per_sample as u32);

    let config = flacenc::config::Encoder::default()
        .into_verified()
        .map_err(|e| PanaudError::encode(path, format!("FLAC encoder config error: {e:?}"), "internal error; please report this bug"))?;

    let source = flacenc::source::MemSource::from_samples(
        &samples_i32,
        audio.channels as usize,
        bits_per_sample,
        audio.sample_rate as usize,
    );

    let flac_stream =
        flacenc::encode_with_fixed_block_size(&config, source, config.block_size)
            .map_err(|e| PanaudError::encode(path, format!("FLAC encoding failed: {e:?}"), "check that the audio data is valid"))?;

    let mut sink = flacenc::bitsink::ByteSink::new();
    flac_stream.write(&mut sink)
        .map_err(|_| PanaudError::encode(path, "failed to serialize FLAC stream", "internal error; please report this bug"))?;

    std::fs::write(path, sink.as_slice())
        .map_err(|e| PanaudError::encode(path, e.to_string(), "check that the output directory exists and is writable"))?;

    Ok(())
}
