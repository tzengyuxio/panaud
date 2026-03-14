use crate::error::{PanaudError, Result};
use crate::types::AudioData;
use std::path::Path;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;

/// Decode an audio file into interleaved f32 samples using symphonia.
pub fn decode_file(path: &Path) -> Result<AudioData> {
    let probed = super::probe::probe_file(path)?;

    let mut format_reader = probed.format;

    let track = format_reader
        .default_track()
        .ok_or_else(|| PanaudError::DecodeError {
            message: "no audio track found".into(),
            path: Some(path.to_path_buf()),
            suggestion: "the file does not contain an audio track".into(),
        })?;

    let sample_rate = track
        .codec_params
        .sample_rate
        .ok_or_else(|| PanaudError::DecodeError {
            message: "unknown sample rate".into(),
            path: Some(path.to_path_buf()),
            suggestion: "the file metadata may be corrupted".into(),
        })?;

    let channels = track
        .codec_params
        .channels
        .map(|c| c.count() as u16)
        .unwrap_or(2);

    let track_id = track.id;

    // Pre-allocate based on metadata when available.
    let estimated_samples = track
        .codec_params
        .n_frames
        .map(|n| n as usize * channels as usize)
        .unwrap_or(0);

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| PanaudError::DecodeError {
            message: e.to_string(),
            path: Some(path.to_path_buf()),
            suggestion: "the audio codec may not be supported".into(),
        })?;

    let mut all_samples: Vec<f32> = Vec::with_capacity(estimated_samples);
    let mut sample_buf: Option<SampleBuffer<f32>> = None;

    loop {
        let packet = match format_reader.next_packet() {
            Ok(p) => p,
            Err(symphonia::core::errors::Error::IoError(ref e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(e) => {
                return Err(PanaudError::DecodeError {
                    message: e.to_string(),
                    path: Some(path.to_path_buf()),
                    suggestion: "error reading audio packets".into(),
                });
            }
        };

        if packet.track_id() != track_id {
            continue;
        }

        let decoded = match decoder.decode(&packet) {
            Ok(d) => d,
            Err(symphonia::core::errors::Error::DecodeError(_)) => continue,
            Err(e) => {
                return Err(PanaudError::DecodeError {
                    message: e.to_string(),
                    path: Some(path.to_path_buf()),
                    suggestion: "error decoding audio packet".into(),
                });
            }
        };

        let spec = *decoded.spec();
        let capacity = decoded.capacity();

        // Reuse the sample buffer if it has sufficient capacity, otherwise allocate.
        let buf = sample_buf.get_or_insert_with(|| SampleBuffer::<f32>::new(capacity as u64, spec));
        buf.copy_interleaved_ref(decoded);
        all_samples.extend_from_slice(buf.samples());
    }

    Ok(AudioData {
        samples: all_samples,
        sample_rate,
        channels,
    })
}
