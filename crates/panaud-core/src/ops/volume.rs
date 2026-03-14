use crate::error::{PanaudError, Result};
use crate::ops::{CommandSchema, Operation, OperationDescription};
use crate::schema::{ParamRange, ParamSchema, ParamType};
use crate::types::AudioData;

/// Apply gain (volume adjustment) to audio data.
pub struct VolumeOp {
    /// Linear gain factor.
    factor: f32,
    /// Original dB value, if constructed from dB.
    db: Option<f32>,
}

impl VolumeOp {
    /// Create from a dB gain value.
    pub fn from_db(db: f32) -> Self {
        let factor = 10_f32.powf(db / 20.0);
        Self {
            factor,
            db: Some(db),
        }
    }

    /// Create from a linear factor.
    pub fn from_factor(factor: f32) -> Result<Self> {
        if factor < 0.0 {
            return Err(PanaudError::InvalidArgument {
                message: "volume factor must be non-negative".into(),
                suggestion: "use a value >= 0.0 (e.g. 0.5 for half volume)".into(),
            });
        }
        Ok(Self { factor, db: None })
    }
}

impl Operation<AudioData, PanaudError> for VolumeOp {
    fn name(&self) -> &str {
        "volume"
    }

    fn apply(&self, mut input: AudioData) -> Result<AudioData> {
        for sample in &mut input.samples {
            *sample = (*sample * self.factor).clamp(-1.0, 1.0);
        }
        Ok(input)
    }

    fn describe(&self) -> OperationDescription {
        let desc = if let Some(db) = self.db {
            format!("Adjust volume by {db:+.1} dB (factor {:.4})", self.factor)
        } else {
            format!("Adjust volume by factor {:.4}", self.factor)
        };
        OperationDescription {
            operation: "volume".into(),
            params: serde_json::json!({
                "factor": self.factor,
                "db": self.db,
            }),
            description: desc,
        }
    }

    fn schema() -> CommandSchema {
        CommandSchema {
            command: "volume".into(),
            description: "Adjust audio volume by gain (dB) or linear factor".into(),
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
                    name: "gain".into(),
                    param_type: ParamType::Float,
                    required: false,
                    description: "Gain in dB (e.g. -3 for quieter, +6 for louder)".into(),
                    default: None,
                    choices: None,
                    range: Some(ParamRange {
                        min: -100.0,
                        max: 100.0,
                    }),
                },
                ParamSchema {
                    name: "factor".into(),
                    param_type: ParamType::Float,
                    required: false,
                    description: "Linear volume factor (e.g. 0.5 for half, 2.0 for double)".into(),
                    default: None,
                    choices: None,
                    range: Some(ParamRange {
                        min: 0.0,
                        max: 100.0,
                    }),
                },
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_audio() -> AudioData {
        AudioData {
            samples: vec![0.5, -0.5, 0.25, -0.25],
            sample_rate: 44100,
            channels: 2,
        }
    }

    #[test]
    fn volume_from_db_zero() {
        let op = VolumeOp::from_db(0.0);
        let result = op.apply(test_audio()).unwrap();
        assert!((result.samples[0] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn volume_from_db_negative() {
        let op = VolumeOp::from_db(-20.0); // factor ~0.1
        let result = op.apply(test_audio()).unwrap();
        assert!((result.samples[0] - 0.05).abs() < 0.01);
    }

    #[test]
    fn volume_from_factor() {
        let op = VolumeOp::from_factor(2.0).unwrap();
        let result = op.apply(test_audio()).unwrap();
        // 0.5 * 2.0 = 1.0 (clamped)
        assert!((result.samples[0] - 1.0).abs() < 1e-6);
        // -0.5 * 2.0 = -1.0 (clamped)
        assert!((result.samples[1] - (-1.0)).abs() < 1e-6);
    }

    #[test]
    fn volume_clamps() {
        let op = VolumeOp::from_factor(10.0).unwrap();
        let result = op.apply(test_audio()).unwrap();
        assert_eq!(result.samples[0], 1.0);
        assert_eq!(result.samples[1], -1.0);
    }

    #[test]
    fn volume_negative_factor_rejected() {
        assert!(VolumeOp::from_factor(-1.0).is_err());
    }
}
