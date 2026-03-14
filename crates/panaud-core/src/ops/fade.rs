use crate::error::{PanaudError, Result};
use crate::ops::{CommandSchema, Operation, OperationDescription};
use crate::schema::{ParamSchema, ParamType};
use crate::time::{parse_time, TimeSpec};
use crate::types::AudioData;

/// Apply fade-in and/or fade-out to audio.
pub struct FadeOp {
    fade_in: Option<TimeSpec>,
    fade_out: Option<TimeSpec>,
}

impl FadeOp {
    pub fn new(fade_in_str: Option<&str>, fade_out_str: Option<&str>) -> Result<Self> {
        let fade_in = fade_in_str.map(parse_time).transpose()?;
        let fade_out = fade_out_str.map(parse_time).transpose()?;

        if fade_in.is_none() && fade_out.is_none() {
            return Err(PanaudError::InvalidArgument {
                message: "at least one of --in or --out must be specified".into(),
                suggestion: "usage: panaud fade <input> -o <output> --in 2s --out 3s".into(),
            });
        }

        Ok(Self { fade_in, fade_out })
    }

    pub fn from_specs(fade_in: Option<TimeSpec>, fade_out: Option<TimeSpec>) -> Result<Self> {
        if fade_in.is_none() && fade_out.is_none() {
            return Err(PanaudError::InvalidArgument {
                message: "at least one of fade_in or fade_out must be specified".into(),
                suggestion: "provide a fade-in and/or fade-out duration".into(),
            });
        }
        Ok(Self { fade_in, fade_out })
    }
}

impl Operation<AudioData, PanaudError> for FadeOp {
    fn name(&self) -> &str {
        "fade"
    }

    fn apply(&self, mut input: AudioData) -> Result<AudioData> {
        let total_frames = input.num_frames();
        let channels = input.channels as usize;

        let fade_in_frames = self
            .fade_in
            .map(|t| t.to_frame(input.sample_rate))
            .unwrap_or(0);
        let fade_out_frames = self
            .fade_out
            .map(|t| t.to_frame(input.sample_rate))
            .unwrap_or(0);

        if fade_in_frames + fade_out_frames > total_frames {
            return Err(PanaudError::InvalidArgument {
                message: format!(
                    "fade-in ({} frames) + fade-out ({} frames) exceeds audio length ({} frames)",
                    fade_in_frames, fade_out_frames, total_frames
                ),
                suggestion: format!(
                    "audio is {:.2}s long; reduce fade durations to fit",
                    input.duration_secs()
                ),
            });
        }

        // Apply fade-in: multiply by i/N for frames 0..fade_in_frames
        if fade_in_frames > 0 {
            for frame in 0..fade_in_frames as usize {
                let factor = frame as f32 / fade_in_frames as f32;
                for ch in 0..channels {
                    input.samples[frame * channels + ch] *= factor;
                }
            }
        }

        // Apply fade-out: multiply by (M-i)/M for last fade_out_frames
        if fade_out_frames > 0 {
            let fade_out_start = total_frames - fade_out_frames;
            for frame in fade_out_start as usize..total_frames as usize {
                let offset = frame - fade_out_start as usize;
                let factor = 1.0 - (offset as f32 / fade_out_frames as f32);
                for ch in 0..channels {
                    input.samples[frame * channels + ch] *= factor;
                }
            }
        }

        Ok(input)
    }

    fn describe(&self) -> OperationDescription {
        OperationDescription {
            operation: "fade".into(),
            params: serde_json::json!({
                "fade_in": self.fade_in.map(|t| t.to_string()),
                "fade_out": self.fade_out.map(|t| t.to_string()),
            }),
            description: format!(
                "Fade in: {}, fade out: {}",
                self.fade_in
                    .map(|t| t.to_string())
                    .unwrap_or_else(|| "none".into()),
                self.fade_out
                    .map(|t| t.to_string())
                    .unwrap_or_else(|| "none".into()),
            ),
        }
    }

    fn schema() -> CommandSchema {
        CommandSchema {
            command: "fade".into(),
            description: "Apply fade-in and/or fade-out to audio".into(),
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
                    name: "in".into(),
                    param_type: ParamType::String,
                    required: false,
                    description: "Fade-in duration (e.g. '2s', '0:02')".into(),
                    default: None,
                    choices: None,
                    range: None,
                },
                ParamSchema {
                    name: "out".into(),
                    param_type: ParamType::String,
                    required: false,
                    description: "Fade-out duration (e.g. '3s', '0:03')".into(),
                    default: None,
                    choices: None,
                    range: None,
                },
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_audio() -> AudioData {
        // 1 second of stereo audio at 100 Hz sample rate = 200 samples
        AudioData {
            samples: vec![1.0; 200],
            sample_rate: 100,
            channels: 2,
        }
    }

    #[test]
    fn fade_in_only() {
        let audio = test_audio();
        let op = FadeOp::from_specs(Some(TimeSpec::Seconds(0.5)), None).unwrap();
        let result = op.apply(audio).unwrap();
        // Frame 0 should be 0.0 (0/50)
        assert_eq!(result.samples[0], 0.0);
        assert_eq!(result.samples[1], 0.0);
        // Frame 25 should be 0.5 (25/50)
        assert!((result.samples[50] - 0.5).abs() < 1e-6);
        // Frame 50 (beyond fade-in) should be 1.0
        assert!((result.samples[100] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn fade_out_only() {
        let audio = test_audio();
        let op = FadeOp::from_specs(None, Some(TimeSpec::Seconds(0.5))).unwrap();
        let result = op.apply(audio).unwrap();
        // Frame 0 should be unchanged
        assert!((result.samples[0] - 1.0).abs() < 1e-6);
        // Last frame should be ~0.0
        let last_idx = result.samples.len() - 2;
        assert!(result.samples[last_idx].abs() < 0.05);
    }

    #[test]
    fn fade_in_and_out() {
        let audio = test_audio();
        let op = FadeOp::from_specs(Some(TimeSpec::Seconds(0.2)), Some(TimeSpec::Seconds(0.2))).unwrap();
        let result = op.apply(audio).unwrap();
        assert_eq!(result.samples[0], 0.0);
        assert!((result.samples[result.samples.len() - 2]).abs() < 0.1);
        // Middle should be untouched
        assert!((result.samples[100] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn fade_exceeds_length() {
        let audio = test_audio();
        let op = FadeOp::from_specs(Some(TimeSpec::Seconds(0.6)), Some(TimeSpec::Seconds(0.6))).unwrap();
        assert!(op.apply(audio).is_err());
    }

    #[test]
    fn fade_requires_at_least_one() {
        assert!(FadeOp::new(None, None).is_err());
        assert!(FadeOp::from_specs(None, None).is_err());
    }
}
