use crate::error::{PanaudError, Result};
use crate::ops::{CommandSchema, Operation, OperationDescription};
use crate::schema::{ParamSchema, ParamType};
use crate::types::AudioData;
use std::fmt;

/// Channel conversion mode.
#[derive(Debug, Clone)]
pub enum ChannelMode {
    /// Mix down to mono (average all channels).
    Mono,
    /// Upmix mono to stereo (duplicate).
    Stereo,
    /// Set a specific channel count.
    Count(u16),
    /// Extract a single channel by name or index.
    Extract(ChannelSelector),
}

impl fmt::Display for ChannelMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Mono => write!(f, "mono"),
            Self::Stereo => write!(f, "stereo"),
            Self::Count(n) => write!(f, "{n} channels"),
            Self::Extract(sel) => write!(f, "extract {sel}"),
        }
    }
}

/// Which channel to extract.
#[derive(Debug, Clone)]
pub enum ChannelSelector {
    Left,
    Right,
    Index(u16),
}

impl fmt::Display for ChannelSelector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Left => write!(f, "left"),
            Self::Right => write!(f, "right"),
            Self::Index(i) => write!(f, "channel {i}"),
        }
    }
}

/// Mix interleaved samples down to mono by averaging all channels.
fn mix_to_mono(samples: &[f32], in_ch: usize, num_frames: usize) -> Vec<f32> {
    let mut mono = Vec::with_capacity(num_frames);
    for frame in 0..num_frames {
        let mut sum = 0.0_f32;
        for ch in 0..in_ch {
            sum += samples[frame * in_ch + ch];
        }
        mono.push(sum / in_ch as f32);
    }
    mono
}

/// Change the channel layout of audio.
pub struct ChannelsOp {
    mode: ChannelMode,
}

impl ChannelsOp {
    pub fn new(mode: ChannelMode) -> Self {
        Self { mode }
    }
}

impl Operation<AudioData, PanaudError> for ChannelsOp {
    fn name(&self) -> &str {
        "channels"
    }

    fn apply(&self, input: AudioData) -> Result<AudioData> {
        let in_ch = input.channels as usize;
        let num_frames = input.num_frames() as usize;

        match &self.mode {
            ChannelMode::Mono => {
                if in_ch == 1 {
                    return Ok(input);
                }
                Ok(AudioData {
                    samples: mix_to_mono(&input.samples, in_ch, num_frames),
                    sample_rate: input.sample_rate,
                    channels: 1,
                })
            }
            ChannelMode::Stereo => {
                if in_ch == 2 {
                    return Ok(input);
                }
                if in_ch == 1 {
                    let mut stereo = Vec::with_capacity(num_frames * 2);
                    for &s in &input.samples {
                        stereo.push(s);
                        stereo.push(s);
                    }
                    return Ok(AudioData {
                        samples: stereo,
                        sample_rate: input.sample_rate,
                        channels: 2,
                    });
                }
                // Multi-channel to stereo: take first two channels
                let mut stereo = Vec::with_capacity(num_frames * 2);
                for frame in 0..num_frames {
                    stereo.push(input.samples[frame * in_ch]);
                    stereo.push(input.samples[frame * in_ch + 1]);
                }
                Ok(AudioData {
                    samples: stereo,
                    sample_rate: input.sample_rate,
                    channels: 2,
                })
            }
            ChannelMode::Count(target) => {
                let target = *target as usize;
                if target == in_ch {
                    return Ok(input);
                }
                if target == 0 {
                    return Err(PanaudError::InvalidArgument {
                        message: "channel count must be at least 1".into(),
                        suggestion: "use --mono for single channel or --count N where N >= 1"
                            .into(),
                    });
                }
                if target < in_ch {
                    // Downmix: keep first `target` channels (consistent with Stereo)
                    let mut out = Vec::with_capacity(num_frames * target);
                    for frame in 0..num_frames {
                        for ch in 0..target {
                            out.push(input.samples[frame * in_ch + ch]);
                        }
                    }
                    Ok(AudioData {
                        samples: out,
                        sample_rate: input.sample_rate,
                        channels: target as u16,
                    })
                } else {
                    // Upmix: mix to mono then duplicate (consistent with Stereo from mono)
                    let mono = mix_to_mono(&input.samples, in_ch, num_frames);
                    let mut out = Vec::with_capacity(num_frames * target);
                    for &s in &mono {
                        for _ in 0..target {
                            out.push(s);
                        }
                    }
                    Ok(AudioData {
                        samples: out,
                        sample_rate: input.sample_rate,
                        channels: target as u16,
                    })
                }
            }
            ChannelMode::Extract(selector) => {
                let ch_idx = match selector {
                    ChannelSelector::Left => 0_usize,
                    ChannelSelector::Right => {
                        if in_ch < 2 {
                            return Err(PanaudError::InvalidArgument {
                                message: "cannot extract right channel from mono audio".into(),
                                suggestion: "input has only 1 channel".into(),
                            });
                        }
                        1
                    }
                    ChannelSelector::Index(i) => {
                        let i = *i as usize;
                        if i >= in_ch {
                            return Err(PanaudError::InvalidArgument {
                                message: format!(
                                    "channel index {} is out of range (audio has {} channels)",
                                    i, in_ch
                                ),
                                suggestion: format!(
                                    "use an index between 0 and {}",
                                    in_ch.saturating_sub(1)
                                ),
                            });
                        }
                        i
                    }
                };

                let mut extracted = Vec::with_capacity(num_frames);
                for frame in 0..num_frames {
                    extracted.push(input.samples[frame * in_ch + ch_idx]);
                }
                Ok(AudioData {
                    samples: extracted,
                    sample_rate: input.sample_rate,
                    channels: 1,
                })
            }
        }
    }

    fn describe(&self) -> OperationDescription {
        let desc = self.mode.to_string();
        OperationDescription {
            operation: "channels".into(),
            params: serde_json::json!({
                "mode": desc,
            }),
            description: format!("Convert channels: {desc}"),
        }
    }

    fn schema() -> CommandSchema {
        CommandSchema {
            command: "channels".into(),
            description: "Change audio channel layout".into(),
            params: vec![
                ParamSchema {
                    name: "input".into(),
                    param_type: ParamType::Path,
                    required: true,
                    description: "Input audio file path".into(),
                    default: None,
                    choices: None,
                    range: None,
                },
                ParamSchema {
                    name: "output".into(),
                    param_type: ParamType::Path,
                    required: true,
                    description: "Output audio file path".into(),
                    default: None,
                    choices: None,
                    range: None,
                },
                ParamSchema {
                    name: "mono".into(),
                    param_type: ParamType::Boolean,
                    required: false,
                    description: "Mix down to mono".into(),
                    default: None,
                    choices: None,
                    range: None,
                },
                ParamSchema {
                    name: "stereo".into(),
                    param_type: ParamType::Boolean,
                    required: false,
                    description: "Convert to stereo".into(),
                    default: None,
                    choices: None,
                    range: None,
                },
                ParamSchema {
                    name: "count".into(),
                    param_type: ParamType::Integer,
                    required: false,
                    description: "Target channel count".into(),
                    default: None,
                    choices: None,
                    range: None,
                },
                ParamSchema {
                    name: "extract".into(),
                    param_type: ParamType::String,
                    required: false,
                    description: "Extract a channel (left, right, or numeric index)".into(),
                    default: None,
                    choices: Some(vec!["left".into(), "right".into()]),
                    range: None,
                },
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stereo_audio() -> AudioData {
        AudioData {
            samples: vec![1.0, 0.0, 0.5, -0.5, 0.2, -0.2],
            sample_rate: 44100,
            channels: 2,
        }
    }

    fn mono_audio() -> AudioData {
        AudioData {
            samples: vec![0.5, -0.5, 0.3],
            sample_rate: 44100,
            channels: 1,
        }
    }

    #[test]
    fn to_mono() {
        let op = ChannelsOp::new(ChannelMode::Mono);
        let result = op.apply(stereo_audio()).unwrap();
        assert_eq!(result.channels, 1);
        assert_eq!(result.num_frames(), 3);
        // Frame 0: avg(1.0, 0.0) = 0.5
        assert!((result.samples[0] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn mono_to_stereo() {
        let op = ChannelsOp::new(ChannelMode::Stereo);
        let result = op.apply(mono_audio()).unwrap();
        assert_eq!(result.channels, 2);
        assert_eq!(result.num_frames(), 3);
        assert!((result.samples[0] - 0.5).abs() < 1e-6);
        assert!((result.samples[1] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn extract_left() {
        let op = ChannelsOp::new(ChannelMode::Extract(ChannelSelector::Left));
        let result = op.apply(stereo_audio()).unwrap();
        assert_eq!(result.channels, 1);
        assert_eq!(result.num_frames(), 3);
        assert!((result.samples[0] - 1.0).abs() < 1e-6);
        assert!((result.samples[1] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn extract_right() {
        let op = ChannelsOp::new(ChannelMode::Extract(ChannelSelector::Right));
        let result = op.apply(stereo_audio()).unwrap();
        assert_eq!(result.channels, 1);
        assert!((result.samples[0] - 0.0).abs() < 1e-6);
        assert!((result.samples[1] - (-0.5)).abs() < 1e-6);
    }

    #[test]
    fn extract_right_from_mono_errors() {
        let op = ChannelsOp::new(ChannelMode::Extract(ChannelSelector::Right));
        assert!(op.apply(mono_audio()).is_err());
    }

    #[test]
    fn already_mono_noop() {
        let op = ChannelsOp::new(ChannelMode::Mono);
        let audio = mono_audio();
        let len = audio.samples.len();
        let result = op.apply(audio).unwrap();
        assert_eq!(result.samples.len(), len);
    }

    #[test]
    fn count_zero_errors() {
        let op = ChannelsOp::new(ChannelMode::Count(0));
        assert!(op.apply(stereo_audio()).is_err());
    }

    #[test]
    fn count_downmix_keeps_first_n() {
        // 4-channel audio → 2 channels should keep first 2
        let audio = AudioData {
            samples: vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0],
            sample_rate: 44100,
            channels: 4,
        };
        let op = ChannelsOp::new(ChannelMode::Count(2));
        let result = op.apply(audio).unwrap();
        assert_eq!(result.channels, 2);
        assert_eq!(result.samples, vec![1.0, 2.0, 5.0, 6.0]);
    }
}
