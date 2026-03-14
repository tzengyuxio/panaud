use crate::error::{PanaudError, Result};
use crate::ops::{CommandSchema, Operation, OperationDescription};
use crate::schema::{ParamRange, ParamSchema, ParamType};
use crate::types::AudioData;
use rubato::{
    Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction,
};

/// Resample audio to a target sample rate.
pub struct ResampleOp {
    target_rate: u32,
}

impl ResampleOp {
    pub fn new(target_rate: u32) -> Result<Self> {
        if target_rate == 0 {
            return Err(PanaudError::InvalidArgument {
                message: "target sample rate must be greater than 0".into(),
                suggestion: "common sample rates: 8000, 22050, 44100, 48000, 96000".into(),
            });
        }
        Ok(Self { target_rate })
    }
}

impl Operation<AudioData, PanaudError> for ResampleOp {
    fn name(&self) -> &str {
        "resample"
    }

    fn apply(&self, input: AudioData) -> Result<AudioData> {
        if input.sample_rate == self.target_rate {
            return Ok(input);
        }

        let channels = input.channels as usize;
        let ratio = self.target_rate as f64 / input.sample_rate as f64;

        let params = SincInterpolationParameters {
            sinc_len: 256,
            f_cutoff: 0.95,
            interpolation: SincInterpolationType::Linear,
            oversampling_factor: 256,
            window: WindowFunction::BlackmanHarris2,
        };

        let chunk_size = 1024;
        let mut resampler =
            SincFixedIn::<f32>::new(ratio, 2.0, params, chunk_size, channels).map_err(|e| {
                PanaudError::ResampleError {
                    message: format!("failed to create resampler: {e}"),
                    suggestion: "check that sample rate ratio is reasonable".into(),
                }
            })?;

        // Deinterleave
        let channel_data = input.deinterleave();

        let num_frames = input.num_frames() as usize;
        let estimated_output = (num_frames as f64 * ratio).ceil() as usize + chunk_size;
        let mut output_channels: Vec<Vec<f32>> =
            (0..channels).map(|_| Vec::with_capacity(estimated_output)).collect();

        let mut pos = 0;
        while pos < num_frames {
            let end = (pos + chunk_size).min(num_frames);
            let actual_len = end - pos;
            let is_last = end >= num_frames;

            let chunk: Vec<Vec<f32>> = channel_data
                .iter()
                .map(|ch| ch[pos..end].to_vec())
                .collect();

            let resampled = if is_last && actual_len < chunk_size {
                resampler
                    .process_partial(Some(&chunk), None)
                    .map_err(|e| PanaudError::ResampleError {
                        message: format!("resample error: {e}"),
                        suggestion: "this may indicate corrupted audio data".into(),
                    })?
            } else {
                resampler
                    .process(&chunk, None)
                    .map_err(|e| PanaudError::ResampleError {
                        message: format!("resample error: {e}"),
                        suggestion: "this may indicate corrupted audio data".into(),
                    })?
            };

            for (ch_idx, ch_data) in resampled.into_iter().enumerate() {
                output_channels[ch_idx].extend(ch_data.into_iter());
            }

            pos += chunk_size;
        }

        // Reinterleave
        let out_frames = output_channels[0].len();
        let mut samples = Vec::with_capacity(out_frames * channels);
        for frame in 0..out_frames {
            for ch in &output_channels {
                samples.push(ch[frame]);
            }
        }

        Ok(AudioData {
            samples,
            sample_rate: self.target_rate,
            channels: input.channels,
        })
    }

    fn describe(&self) -> OperationDescription {
        OperationDescription {
            operation: "resample".into(),
            params: serde_json::json!({
                "target_rate": self.target_rate,
            }),
            description: format!("Resample to {} Hz", self.target_rate),
        }
    }

    fn schema() -> CommandSchema {
        CommandSchema {
            command: "resample".into(),
            description: "Resample audio to a target sample rate".into(),
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
                    name: "rate".into(),
                    param_type: ParamType::Integer,
                    required: true,
                    description: "Target sample rate in Hz".into(),
                    default: None,
                    choices: None,
                    range: Some(ParamRange {
                        min: 1.0,
                        max: 384000.0,
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
    fn same_rate_noop() {
        let audio = AudioData {
            samples: vec![0.5; 44100],
            sample_rate: 44100,
            channels: 1,
        };
        let op = ResampleOp::new(44100).unwrap();
        let result = op.apply(audio).unwrap();
        assert_eq!(result.sample_rate, 44100);
        assert_eq!(result.samples.len(), 44100);
    }

    #[test]
    fn zero_rate_errors() {
        assert!(ResampleOp::new(0).is_err());
    }

    #[test]
    fn resample_changes_rate() {
        // Create 1 second of mono silence at 44100 Hz
        let audio = AudioData {
            samples: vec![0.0; 44100],
            sample_rate: 44100,
            channels: 1,
        };
        let op = ResampleOp::new(22050).unwrap();
        let result = op.apply(audio).unwrap();
        assert_eq!(result.sample_rate, 22050);
        // Output should be approximately half the frames (with some padding tolerance)
        let expected = 22050_i64;
        let actual = result.samples.len() as i64;
        assert!(
            (actual - expected).unsigned_abs() < 1024,
            "expected ~{expected} samples, got {actual}"
        );
    }
}
