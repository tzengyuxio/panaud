use crate::error::{PanaudError, Result};
use crate::types::AudioData;

/// Concatenate multiple audio segments into one.
///
/// All inputs must have the same sample rate and channel count.
pub fn concat_audio(inputs: Vec<AudioData>) -> Result<AudioData> {
    if inputs.is_empty() {
        return Err(PanaudError::InvalidArgument {
            message: "concat requires at least one input".into(),
            suggestion: "provide two or more audio files to concatenate".into(),
        });
    }

    let sample_rate = inputs[0].sample_rate;
    let channels = inputs[0].channels;

    for (i, audio) in inputs.iter().enumerate().skip(1) {
        if audio.sample_rate != sample_rate {
            return Err(PanaudError::FormatMismatch {
                message: format!(
                    "input {} has sample rate {} Hz, but input 0 has {} Hz",
                    i, audio.sample_rate, sample_rate
                ),
                suggestion: "all inputs must have the same sample rate; use 'panaud resample' to match them first".into(),
            });
        }
        if audio.channels != channels {
            return Err(PanaudError::FormatMismatch {
                message: format!(
                    "input {} has {} channels, but input 0 has {} channels",
                    i, audio.channels, channels
                ),
                suggestion: "all inputs must have the same channel count; use 'panaud channels' to match them first".into(),
            });
        }
    }

    let total_samples: usize = inputs.iter().map(|a| a.samples.len()).sum();
    let mut samples = Vec::with_capacity(total_samples);
    for audio in inputs {
        samples.extend(audio.samples);
    }

    Ok(AudioData {
        samples,
        sample_rate,
        channels,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concat_two() {
        let a = AudioData {
            samples: vec![1.0, 2.0],
            sample_rate: 44100,
            channels: 1,
        };
        let b = AudioData {
            samples: vec![3.0, 4.0],
            sample_rate: 44100,
            channels: 1,
        };
        let result = concat_audio(vec![a, b]).unwrap();
        assert_eq!(result.samples, vec![1.0, 2.0, 3.0, 4.0]);
        assert_eq!(result.sample_rate, 44100);
        assert_eq!(result.channels, 1);
    }

    #[test]
    fn concat_mismatched_rate() {
        let a = AudioData {
            samples: vec![1.0],
            sample_rate: 44100,
            channels: 1,
        };
        let b = AudioData {
            samples: vec![1.0],
            sample_rate: 48000,
            channels: 1,
        };
        assert!(concat_audio(vec![a, b]).is_err());
    }

    #[test]
    fn concat_mismatched_channels() {
        let a = AudioData {
            samples: vec![1.0, 2.0],
            sample_rate: 44100,
            channels: 2,
        };
        let b = AudioData {
            samples: vec![1.0],
            sample_rate: 44100,
            channels: 1,
        };
        assert!(concat_audio(vec![a, b]).is_err());
    }

    #[test]
    fn concat_empty() {
        assert!(concat_audio(vec![]).is_err());
    }

    #[test]
    fn concat_single() {
        let a = AudioData {
            samples: vec![1.0, 2.0],
            sample_rate: 44100,
            channels: 1,
        };
        let result = concat_audio(vec![a]).unwrap();
        assert_eq!(result.samples, vec![1.0, 2.0]);
    }
}
