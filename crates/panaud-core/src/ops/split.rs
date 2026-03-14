use crate::error::{PanaudError, Result};
use crate::time::TimeSpec;
use crate::types::AudioData;

/// How to split the audio.
#[derive(Debug, Clone)]
pub enum SplitMode {
    /// Split at specific time points.
    At(Vec<TimeSpec>),
    /// Split into N equal parts.
    Count(u32),
    /// Split into chunks of a given duration.
    Duration(TimeSpec),
}

/// Split audio into multiple segments.
pub fn split_audio(input: &AudioData, mode: &SplitMode) -> Result<Vec<AudioData>> {
    let total_frames = input.num_frames();

    if total_frames == 0 {
        return Err(PanaudError::SplitError {
            message: "cannot split empty audio".into(),
            suggestion: "input audio has no samples".into(),
        });
    }

    let split_points: Vec<u64> = match mode {
        SplitMode::At(times) => {
            let mut points: Vec<u64> = times
                .iter()
                .map(|t| t.to_frame(input.sample_rate))
                .collect();
            points.sort();
            points.dedup();
            // Filter out points beyond audio length
            points.retain(|&p| p > 0 && p < total_frames);
            if points.is_empty() {
                return Err(PanaudError::SplitError {
                    message: "all split points are outside the audio range".into(),
                    suggestion: format!(
                        "audio is {:.2}s long; use split points within that range",
                        input.duration_secs()
                    ),
                });
            }
            points
        }
        SplitMode::Count(n) => {
            if *n < 2 {
                return Err(PanaudError::SplitError {
                    message: "split count must be at least 2".into(),
                    suggestion: "use --count N where N >= 2".into(),
                });
            }
            let chunk_frames = total_frames / *n as u64;
            if chunk_frames == 0 {
                return Err(PanaudError::SplitError {
                    message: format!(
                        "audio too short ({} frames) to split into {} parts",
                        total_frames, n
                    ),
                    suggestion: "reduce the number of parts or use longer audio".into(),
                });
            }
            (1..*n).map(|i| chunk_frames * i as u64).collect()
        }
        SplitMode::Duration(dur) => {
            let chunk_frames = dur.to_frame(input.sample_rate);
            if chunk_frames == 0 {
                return Err(PanaudError::SplitError {
                    message: "split duration must be greater than 0".into(),
                    suggestion: "use a positive duration like '30s' or '1:00'".into(),
                });
            }
            let mut points = Vec::new();
            let mut pos = chunk_frames;
            while pos < total_frames {
                points.push(pos);
                pos += chunk_frames;
            }
            if points.is_empty() {
                return Err(PanaudError::SplitError {
                    message: format!(
                        "duration ({} frames) exceeds audio length ({} frames)",
                        chunk_frames, total_frames
                    ),
                    suggestion: "use a shorter duration or longer audio".into(),
                });
            }
            points
        }
    };

    // Build segments from split points
    let mut segments = Vec::new();
    let mut start = 0_u64;
    for &point in &split_points {
        segments.push(input.slice_frames(start, point));
        start = point;
    }
    // Last segment
    segments.push(input.slice_frames(start, total_frames));

    Ok(segments)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_audio() -> AudioData {
        AudioData {
            samples: (0..100).map(|i| i as f32).collect(),
            sample_rate: 10,
            channels: 1,
        }
    }

    #[test]
    fn split_by_count() {
        let audio = test_audio(); // 100 frames at 10 Hz = 10s
        let parts = split_audio(&audio, &SplitMode::Count(4)).unwrap();
        assert_eq!(parts.len(), 4);
        assert_eq!(parts[0].num_frames(), 25);
        assert_eq!(parts[1].num_frames(), 25);
        assert_eq!(parts[2].num_frames(), 25);
        assert_eq!(parts[3].num_frames(), 25);
    }

    #[test]
    fn split_by_duration() {
        let audio = test_audio(); // 100 frames, 10 Hz = 10s
        let parts = split_audio(&audio, &SplitMode::Duration(TimeSpec::Seconds(3.0))).unwrap();
        // 30 frames per chunk: [0-30), [30-60), [60-90), [90-100)
        assert_eq!(parts.len(), 4);
        assert_eq!(parts[0].num_frames(), 30);
        assert_eq!(parts[3].num_frames(), 10);
    }

    #[test]
    fn split_at_points() {
        let audio = test_audio();
        let parts = split_audio(
            &audio,
            &SplitMode::At(vec![TimeSpec::Seconds(3.0), TimeSpec::Seconds(7.0)]),
        )
        .unwrap();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0].num_frames(), 30);
        assert_eq!(parts[1].num_frames(), 40);
        assert_eq!(parts[2].num_frames(), 30);
    }

    #[test]
    fn split_count_one_errors() {
        let audio = test_audio();
        assert!(split_audio(&audio, &SplitMode::Count(1)).is_err());
    }

    #[test]
    fn split_duration_too_long() {
        let audio = test_audio();
        assert!(split_audio(&audio, &SplitMode::Duration(TimeSpec::Seconds(20.0))).is_err());
    }

    #[test]
    fn split_empty_audio() {
        let audio = AudioData {
            samples: vec![],
            sample_rate: 44100,
            channels: 1,
        };
        assert!(split_audio(&audio, &SplitMode::Count(2)).is_err());
    }
}
