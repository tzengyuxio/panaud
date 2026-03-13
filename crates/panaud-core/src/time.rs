use crate::error::{PanaudError, Result};
use std::fmt;

/// Parsed time specification.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TimeSpec {
    /// Time in seconds.
    Seconds(f64),
    /// Time in samples.
    Samples(u64),
}

impl TimeSpec {
    /// Convert to a frame index given a sample rate.
    pub fn to_frame(&self, sample_rate: u32) -> u64 {
        match self {
            Self::Seconds(s) => (s * sample_rate as f64).round() as u64,
            Self::Samples(n) => *n,
        }
    }
}

impl fmt::Display for TimeSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Seconds(s) => {
                let total = *s as u64;
                let frac = s - total as f64;
                let hours = total / 3600;
                let minutes = (total % 3600) / 60;
                let seconds = total % 60;
                if hours > 0 {
                    write!(f, "{hours}:{minutes:02}:{seconds:02}.{:02}", (frac * 100.0) as u32)
                } else {
                    write!(f, "{minutes}:{seconds:02}.{:02}", (frac * 100.0) as u32)
                }
            }
            Self::Samples(n) => write!(f, "{n} samples"),
        }
    }
}

/// Parse a time string into a `TimeSpec`.
///
/// Supported formats:
/// - `"90"` or `"90s"` — seconds
/// - `"1.5m"` — minutes
/// - `"1:30"` — minutes:seconds
/// - `"1:02:30"` — hours:minutes:seconds
/// - `"44100S"` — samples (capital S)
pub fn parse_time(input: &str) -> Result<TimeSpec> {
    let input = input.trim();

    if input.is_empty() {
        return Err(PanaudError::InvalidTimeFormat {
            input: input.to_string(),
            suggestion: "use a format like '1:30', '90s', '1.5m', or '44100S' (samples)".into(),
        });
    }

    // Samples: "44100S"
    if input.ends_with('S') {
        let num = &input[..input.len() - 1];
        return num.parse::<u64>().map(TimeSpec::Samples).map_err(|_| {
            PanaudError::InvalidTimeFormat {
                input: input.to_string(),
                suggestion: "sample count must be a positive integer, e.g. '44100S'".into(),
            }
        });
    }

    // Minutes: "1.5m"
    if input.ends_with('m') {
        let num = &input[..input.len() - 1];
        return num
            .parse::<f64>()
            .map(|m| TimeSpec::Seconds(m * 60.0))
            .map_err(|_| PanaudError::InvalidTimeFormat {
                input: input.to_string(),
                suggestion: "minute format should be a number followed by 'm', e.g. '1.5m'".into(),
            });
    }

    // Seconds with suffix: "90s"
    if input.ends_with('s') {
        let num = &input[..input.len() - 1];
        return num
            .parse::<f64>()
            .map(TimeSpec::Seconds)
            .map_err(|_| PanaudError::InvalidTimeFormat {
                input: input.to_string(),
                suggestion: "second format should be a number followed by 's', e.g. '90s'".into(),
            });
    }

    // Colon format: "1:30" or "1:02:30"
    if input.contains(':') {
        let parts: Vec<&str> = input.split(':').collect();
        match parts.len() {
            2 => {
                let minutes = parts[0].parse::<f64>().map_err(|_| {
                    PanaudError::InvalidTimeFormat {
                        input: input.to_string(),
                        suggestion: "format should be 'minutes:seconds', e.g. '1:30'".into(),
                    }
                })?;
                let seconds = parts[1].parse::<f64>().map_err(|_| {
                    PanaudError::InvalidTimeFormat {
                        input: input.to_string(),
                        suggestion: "format should be 'minutes:seconds', e.g. '1:30'".into(),
                    }
                })?;
                return Ok(TimeSpec::Seconds(minutes * 60.0 + seconds));
            }
            3 => {
                let hours = parts[0].parse::<f64>().map_err(|_| {
                    PanaudError::InvalidTimeFormat {
                        input: input.to_string(),
                        suggestion:
                            "format should be 'hours:minutes:seconds', e.g. '1:02:30'".into(),
                    }
                })?;
                let minutes = parts[1].parse::<f64>().map_err(|_| {
                    PanaudError::InvalidTimeFormat {
                        input: input.to_string(),
                        suggestion:
                            "format should be 'hours:minutes:seconds', e.g. '1:02:30'".into(),
                    }
                })?;
                let seconds = parts[2].parse::<f64>().map_err(|_| {
                    PanaudError::InvalidTimeFormat {
                        input: input.to_string(),
                        suggestion:
                            "format should be 'hours:minutes:seconds', e.g. '1:02:30'".into(),
                    }
                })?;
                return Ok(TimeSpec::Seconds(hours * 3600.0 + minutes * 60.0 + seconds));
            }
            _ => {
                return Err(PanaudError::InvalidTimeFormat {
                    input: input.to_string(),
                    suggestion: "use 'mm:ss' or 'hh:mm:ss' format".into(),
                });
            }
        }
    }

    // Plain number: seconds
    input
        .parse::<f64>()
        .map(TimeSpec::Seconds)
        .map_err(|_| PanaudError::InvalidTimeFormat {
            input: input.to_string(),
            suggestion: "use a format like '1:30', '90s', '1.5m', or '44100S' (samples)".into(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_plain_seconds() {
        assert_eq!(parse_time("90").unwrap(), TimeSpec::Seconds(90.0));
    }

    #[test]
    fn parse_seconds_suffix() {
        assert_eq!(parse_time("90s").unwrap(), TimeSpec::Seconds(90.0));
    }

    #[test]
    fn parse_minutes_suffix() {
        assert_eq!(parse_time("1.5m").unwrap(), TimeSpec::Seconds(90.0));
    }

    #[test]
    fn parse_colon_mm_ss() {
        assert_eq!(parse_time("1:30").unwrap(), TimeSpec::Seconds(90.0));
    }

    #[test]
    fn parse_colon_hh_mm_ss() {
        assert_eq!(
            parse_time("1:02:30").unwrap(),
            TimeSpec::Seconds(3750.0)
        );
    }

    #[test]
    fn parse_samples() {
        assert_eq!(parse_time("44100S").unwrap(), TimeSpec::Samples(44100));
    }

    #[test]
    fn parse_empty_is_error() {
        assert!(parse_time("").is_err());
    }

    #[test]
    fn parse_invalid_is_error() {
        assert!(parse_time("abc").is_err());
    }

    #[test]
    fn to_frame_seconds() {
        let spec = TimeSpec::Seconds(1.0);
        assert_eq!(spec.to_frame(44100), 44100);
    }

    #[test]
    fn to_frame_samples() {
        let spec = TimeSpec::Samples(1000);
        assert_eq!(spec.to_frame(44100), 1000);
    }
}
