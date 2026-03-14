pub mod decode;
pub mod encode;
#[cfg(feature = "flac-enc")]
pub mod encode_flac;
#[cfg(feature = "mp3-enc")]
pub mod encode_mp3;
#[cfg(feature = "ogg-enc")]
pub mod encode_ogg;
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
            #[cfg(feature = "flac-enc")]
            AudioFormat::Flac => encode_flac::encode_flac(audio, path),
            #[cfg(feature = "mp3-enc")]
            AudioFormat::Mp3 => encode_mp3::encode_mp3(audio, path),
            #[cfg(feature = "ogg-enc")]
            AudioFormat::Ogg => encode_ogg::encode_ogg(audio, path),
            _ => {
                let supported: Vec<String> = AudioFormat::all()
                    .iter()
                    .filter(|f| f.can_encode())
                    .map(|f| f.to_string())
                    .collect();
                Err(PanaudError::UnsupportedFormat {
                    format: format.to_string(),
                    suggestion: std::format!(
                        "supported output formats: {}; enable features for more",
                        supported.join(", ")
                    ),
                })
            }
        }
    }
}
