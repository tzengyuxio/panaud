use crate::error::{PanaudError, Result};
use crate::ops::{CommandSchema, Operation, OperationDescription};
use crate::schema::{ParamSchema, ParamType};
use crate::time::{parse_time, TimeSpec};
use crate::types::AudioData;

/// Trim audio to a time range [start, end).
pub struct TrimOp {
    start: TimeSpec,
    end: Option<TimeSpec>,
}

impl TrimOp {
    pub fn new(start_str: &str, end_str: Option<&str>) -> Result<Self> {
        let start = parse_time(start_str)?;
        let end = end_str.map(|s| parse_time(s)).transpose()?;
        Ok(Self { start, end })
    }

    /// Create from already-parsed TimeSpecs.
    pub fn from_specs(start: TimeSpec, end: Option<TimeSpec>) -> Self {
        Self { start, end }
    }
}

impl Operation<AudioData, PanaudError> for TrimOp {
    fn name(&self) -> &str {
        "trim"
    }

    fn apply(&self, input: AudioData) -> Result<AudioData> {
        let total_frames = input.num_frames();
        let start_frame = self.start.to_frame(input.sample_rate);
        let end_frame = self
            .end
            .map(|e| e.to_frame(input.sample_rate))
            .unwrap_or(total_frames);

        if start_frame >= total_frames {
            return Err(PanaudError::TrimOutOfRange {
                message: format!(
                    "start position (frame {start_frame}) is beyond audio length ({total_frames} frames)"
                ),
                suggestion: format!(
                    "audio is {:.2}s long; use a start time within that range",
                    input.duration_secs()
                ),
            });
        }

        if end_frame <= start_frame {
            return Err(PanaudError::TrimOutOfRange {
                message: format!(
                    "end position (frame {end_frame}) must be after start (frame {start_frame})"
                ),
                suggestion: "ensure end time is greater than start time".into(),
            });
        }

        Ok(input.slice_frames(start_frame, end_frame))
    }

    fn describe(&self) -> OperationDescription {
        OperationDescription {
            operation: "trim".into(),
            params: serde_json::json!({
                "start": self.start.to_string(),
                "end": self.end.map(|e| e.to_string()),
            }),
            description: format!(
                "Trim audio from {} to {}",
                self.start,
                self.end
                    .map(|e| e.to_string())
                    .unwrap_or_else(|| "end".into())
            ),
        }
    }

    fn schema() -> CommandSchema {
        CommandSchema {
            command: "trim".into(),
            description: "Trim audio to a time range".into(),
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
                    name: "start".into(),
                    param_type: ParamType::String,
                    required: true,
                    description: "Start time (e.g. '1:30', '90s', '1.5m', '44100S')".into(),
                    default: None,
                    choices: None,
                    range: None,
                },
                ParamSchema {
                    name: "end".into(),
                    param_type: ParamType::String,
                    required: false,
                    description: "End time (defaults to end of file)".into(),
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
            samples: (0..200).map(|i| i as f32 / 200.0).collect(),
            sample_rate: 100,
            channels: 2,
        }
    }

    #[test]
    fn trim_start_to_end() {
        let audio = test_audio();
        let op = TrimOp::from_specs(TimeSpec::Seconds(0.5), None);
        let result = op.apply(audio).unwrap();
        assert_eq!(result.num_frames(), 50);
        assert_eq!(result.channels, 2);
        assert_eq!(result.sample_rate, 100);
    }

    #[test]
    fn trim_start_and_end() {
        let audio = test_audio();
        let op = TrimOp::from_specs(TimeSpec::Seconds(0.2), Some(TimeSpec::Seconds(0.8)));
        let result = op.apply(audio).unwrap();
        assert_eq!(result.num_frames(), 60);
    }

    #[test]
    fn trim_with_samples() {
        let audio = test_audio();
        let op = TrimOp::from_specs(TimeSpec::Samples(10), Some(TimeSpec::Samples(50)));
        let result = op.apply(audio).unwrap();
        assert_eq!(result.num_frames(), 40);
    }

    #[test]
    fn trim_start_out_of_range() {
        let audio = test_audio();
        let op = TrimOp::from_specs(TimeSpec::Seconds(2.0), None);
        assert!(op.apply(audio).is_err());
    }

    #[test]
    fn trim_end_before_start() {
        let audio = test_audio();
        let op = TrimOp::from_specs(TimeSpec::Seconds(0.5), Some(TimeSpec::Seconds(0.2)));
        assert!(op.apply(audio).is_err());
    }

    #[test]
    fn trim_from_string() {
        let op = TrimOp::new("1:30", Some("2:00")).unwrap();
        assert_eq!(op.start, TimeSpec::Seconds(90.0));
        assert_eq!(op.end, Some(TimeSpec::Seconds(120.0)));
    }
}
