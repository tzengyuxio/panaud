pub mod decode;
pub mod encode;
pub mod probe;

use crate::error::{PanaudError, Result};
use crate::types::{AudioData, AudioFormat};
use std::path::Path;

/// Central codec registry for encoding/decoding audio files.
pub struct CodecRegistry;

impl CodecRegistry {
    /// Decode an audio file into `AudioData`.
    pub fn decode(path: &Path) -> Result<AudioData> {
        decode::decode_file(path)
    }

    /// Encode `AudioData` to a file in the given format.
    pub fn encode(audio: &AudioData, path: &Path, format: AudioFormat) -> Result<()> {
        match format {
            AudioFormat::Wav => encode::encode_wav(audio, path),
            _ => Err(PanaudError::UnsupportedFormat {
                format: format.to_string(),
                suggestion: "v0.1.0 only supports WAV output; convert to WAV first".into(),
            }),
        }
    }
}
