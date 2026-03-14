use crate::error::{PanaudError, Result};
use crate::types::AudioData;
use std::mem::MaybeUninit;
use std::path::Path;

/// Encode `AudioData` as an MP3 file using mp3lame-encoder.
pub fn encode_mp3(audio: &AudioData, path: &Path) -> Result<()> {
    use mp3lame_encoder::{Builder, FlushNoGap, InterleavedPcm};

    let mut builder = Builder::new()
        .ok_or_else(|| PanaudError::encode(path, "failed to create LAME encoder", "internal error; please report this bug"))?;

    builder
        .set_num_channels(audio.channels as u8)
        .map_err(|e| PanaudError::encode(path, format!("set channels: {e}"), "check audio parameters (sample rate, channels)"))?;

    builder
        .set_sample_rate(audio.sample_rate)
        .map_err(|e| PanaudError::encode(path, format!("set sample rate: {e}"), "check audio parameters (sample rate, channels)"))?;

    builder
        .set_quality(mp3lame_encoder::Quality::Best)
        .map_err(|e| PanaudError::encode(path, format!("set quality: {e}"), "check audio parameters (sample rate, channels)"))?;

    let mut encoder = builder
        .build()
        .map_err(|e| PanaudError::encode(path, format!("build encoder: {e}"), "check audio parameters (sample rate, channels)"))?;

    let pcm_i16 = audio.samples_as_i16();
    let input = InterleavedPcm(&pcm_i16);

    // Allocate output buffer (worst case: 1.25 * samples + 7200)
    let buf_size = (pcm_i16.len() as f64 * 1.25) as usize + 7200;
    let mut mp3_buf: Vec<MaybeUninit<u8>> = vec![MaybeUninit::uninit(); buf_size];

    let encoded_size = encoder
        .encode(input, &mut mp3_buf)
        .map_err(|e| PanaudError::encode(path, format!("MP3 encoding failed: {e}"), "check that the audio data is valid"))?;

    let flush_size = encoder
        .flush::<FlushNoGap>(&mut mp3_buf[encoded_size..])
        .map_err(|e| PanaudError::encode(path, format!("MP3 flush failed: {e}"), "check that the audio data is valid"))?;

    let total_size = encoded_size + flush_size;
    // SAFETY: mp3lame-encoder initialized the first `total_size` bytes
    let mp3_data: &[u8] =
        unsafe { std::slice::from_raw_parts(mp3_buf.as_ptr() as *const u8, total_size) };

    std::fs::write(path, mp3_data)
        .map_err(|e| PanaudError::encode(path, e.to_string(), "check that the output directory exists and is writable"))?;

    Ok(())
}
