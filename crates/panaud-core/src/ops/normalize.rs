use crate::error::{PanaudError, Result};
use crate::ops::{CommandSchema, Operation, OperationDescription};
use crate::schema::{ParamRange, ParamSchema, ParamType};
use crate::types::AudioData;

/// Peak-normalize audio to a target dBFS level.
pub struct NormalizeOp {
    /// Target peak level in dBFS (e.g. -1.0).
    target_db: f32,
}

impl NormalizeOp {
    pub fn new(target_db: f32) -> Self {
        Self { target_db }
    }
}

impl Operation<AudioData, PanaudError> for NormalizeOp {
    fn name(&self) -> &str {
        "normalize"
    }

    fn apply(&self, mut input: AudioData) -> Result<AudioData> {
        let peak = input
            .samples
            .iter()
            .map(|s| s.abs())
            .fold(0.0_f32, f32::max);

        if peak == 0.0 {
            return Err(PanaudError::InvalidArgument {
                message: "cannot normalize silent audio (peak is 0)".into(),
                suggestion: "ensure the input audio contains non-silent samples".into(),
            });
        }

        let target_linear = 10_f32.powf(self.target_db / 20.0);
        let factor = target_linear / peak;

        for sample in &mut input.samples {
            *sample = (*sample * factor).clamp(-1.0, 1.0);
        }

        Ok(input)
    }

    fn describe(&self) -> OperationDescription {
        OperationDescription {
            operation: "normalize".into(),
            params: serde_json::json!({
                "target_db": self.target_db,
            }),
            description: format!("Peak-normalize to {:.1} dBFS", self.target_db),
        }
    }

    fn schema() -> CommandSchema {
        CommandSchema {
            command: "normalize".into(),
            description: "Peak-normalize audio to a target dBFS level".into(),
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
                    name: "target".into(),
                    param_type: ParamType::Float,
                    required: false,
                    description: "Target peak level in dBFS (default: -1.0)".into(),
                    default: Some(serde_json::json!(-1.0)),
                    choices: None,
                    range: Some(ParamRange {
                        min: -100.0,
                        max: 0.0,
                    }),
                },
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_peak() {
        let audio = AudioData {
            samples: vec![0.25, -0.5, 0.1, -0.1],
            sample_rate: 44100,
            channels: 2,
        };
        let op = NormalizeOp::new(0.0); // normalize to 0 dBFS (peak = 1.0)
        let result = op.apply(audio).unwrap();
        // peak was 0.5, factor = 1.0 / 0.5 = 2.0
        assert!((result.samples[0] - 0.5).abs() < 1e-6);
        assert!((result.samples[1] - (-1.0)).abs() < 1e-6);
    }

    #[test]
    fn normalize_to_minus_1_dbfs() {
        let audio = AudioData {
            samples: vec![0.5, -0.5],
            sample_rate: 44100,
            channels: 1,
        };
        let op = NormalizeOp::new(-1.0);
        let result = op.apply(audio).unwrap();
        // target_linear = 10^(-1/20) ≈ 0.891
        // factor = 0.891 / 0.5 ≈ 1.782
        let expected = 0.5 * (10_f32.powf(-1.0 / 20.0) / 0.5);
        assert!((result.samples[0] - expected).abs() < 1e-4);
    }

    #[test]
    fn normalize_silent_audio_errors() {
        let audio = AudioData {
            samples: vec![0.0, 0.0],
            sample_rate: 44100,
            channels: 1,
        };
        let op = NormalizeOp::new(-1.0);
        assert!(op.apply(audio).is_err());
    }
}
