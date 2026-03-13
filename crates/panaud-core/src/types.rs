use serde::Serialize;
use std::fmt;

/// Interleaved f32 audio data.
#[derive(Debug, Clone)]
pub struct AudioData {
    /// Interleaved f32 samples (channel-interleaved).
    pub samples: Vec<f32>,
    /// Sample rate in Hz.
    pub sample_rate: u32,
    /// Number of channels.
    pub channels: u16,
}

impl AudioData {
    /// Number of frames (one frame = one sample per channel).
    pub fn num_frames(&self) -> u64 {
        if self.channels == 0 {
            return 0;
        }
        self.samples.len() as u64 / self.channels as u64
    }

    /// Duration in seconds.
    pub fn duration_secs(&self) -> f64 {
        if self.sample_rate == 0 {
            return 0.0;
        }
        self.num_frames() as f64 / self.sample_rate as f64
    }

    /// Extract a slice of frames [start_frame, end_frame).
    pub fn slice_frames(&self, start_frame: u64, end_frame: u64) -> AudioData {
        let ch = self.channels as u64;
        let total = self.num_frames();
        let start = start_frame.min(total);
        let end = end_frame.min(total);
        let start_idx = (start * ch) as usize;
        let end_idx = (end * ch) as usize;
        AudioData {
            samples: self.samples[start_idx..end_idx].to_vec(),
            sample_rate: self.sample_rate,
            channels: self.channels,
        }
    }
}

/// Supported audio formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AudioFormat {
    Wav,
    Flac,
    Mp3,
    Ogg,
    Aac,
}

impl AudioFormat {
    /// Detect format from file extension.
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "wav" => Some(Self::Wav),
            "flac" => Some(Self::Flac),
            "mp3" => Some(Self::Mp3),
            "ogg" | "oga" => Some(Self::Ogg),
            "aac" | "m4a" => Some(Self::Aac),
            _ => None,
        }
    }

    /// Detect format from file path extension.
    pub fn from_path(path: &std::path::Path) -> Option<Self> {
        path.extension()
            .and_then(|e| e.to_str())
            .and_then(Self::from_extension)
    }

    /// Whether this format can be encoded in v0.1.0.
    pub fn can_encode(&self) -> bool {
        matches!(self, Self::Wav)
    }

    /// Whether this format can be decoded.
    pub fn can_decode(&self) -> bool {
        true
    }

    /// File extension string.
    pub fn extension(&self) -> &str {
        match self {
            Self::Wav => "wav",
            Self::Flac => "flac",
            Self::Mp3 => "mp3",
            Self::Ogg => "ogg",
            Self::Aac => "aac",
        }
    }

    /// All known formats.
    pub fn all() -> &'static [AudioFormat] {
        &[
            Self::Wav,
            Self::Flac,
            Self::Mp3,
            Self::Ogg,
            Self::Aac,
        ]
    }
}

impl fmt::Display for AudioFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.extension().to_uppercase())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_data_num_frames() {
        let data = AudioData {
            samples: vec![0.0; 100],
            sample_rate: 44100,
            channels: 2,
        };
        assert_eq!(data.num_frames(), 50);
    }

    #[test]
    fn audio_data_duration() {
        let data = AudioData {
            samples: vec![0.0; 44100 * 2],
            sample_rate: 44100,
            channels: 2,
        };
        assert!((data.duration_secs() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn audio_data_slice_frames() {
        let data = AudioData {
            samples: (0..20).map(|i| i as f32).collect(),
            sample_rate: 10,
            channels: 2,
        };
        let sliced = data.slice_frames(2, 5);
        assert_eq!(sliced.num_frames(), 3);
        assert_eq!(sliced.samples.len(), 6);
        assert_eq!(sliced.samples[0], 4.0);
    }

    #[test]
    fn audio_format_from_extension() {
        assert_eq!(AudioFormat::from_extension("wav"), Some(AudioFormat::Wav));
        assert_eq!(AudioFormat::from_extension("MP3"), Some(AudioFormat::Mp3));
        assert_eq!(AudioFormat::from_extension("xyz"), None);
    }

    #[test]
    fn audio_format_can_encode() {
        assert!(AudioFormat::Wav.can_encode());
        assert!(!AudioFormat::Mp3.can_encode());
        assert!(!AudioFormat::Flac.can_encode());
    }
}
