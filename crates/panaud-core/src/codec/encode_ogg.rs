use crate::error::{PanaudError, Result};
use crate::types::AudioData;
use std::path::Path;

/// Encode `AudioData` as an OGG Vorbis file using vorbis_rs.
pub fn encode_ogg(audio: &AudioData, path: &Path) -> Result<()> {
    use std::io::BufWriter;
    use vorbis_rs::VorbisEncoderBuilder;

    let file = std::fs::File::create(path)
        .map_err(|e| PanaudError::encode(path, e.to_string(), "check that the output directory exists and is writable"))?;
    let writer = BufWriter::new(file);

    let mut encoder = VorbisEncoderBuilder::new(
        audio.sample_rate as u32 as i64,
        audio.channels as u8,
        writer,
    )
    .map_err(|e| PanaudError::encode(path, format!("OGG encoder init: {e}"), "check audio parameters (sample rate, channels)"))?
    .build()
    .map_err(|e| PanaudError::encode(path, format!("OGG encoder build: {e}"), "check audio parameters (sample rate, channels)"))?;

    // vorbis_rs expects per-channel sample arrays (non-interleaved)
    let channels = audio.deinterleave();
    let channel_refs: Vec<&[f32]> = channels.iter().map(|c| c.as_slice()).collect();

    encoder
        .encode_audio_block(&channel_refs)
        .map_err(|e| PanaudError::encode(path, format!("OGG encoding failed: {e}"), "check that the audio data is valid"))?;

    encoder.finish()
        .map_err(|e| PanaudError::encode(path, format!("OGG finalize failed: {e}"), "check that the audio data is valid"))?;

    Ok(())
}
