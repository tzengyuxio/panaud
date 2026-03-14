use crate::error::{PanaudError, Result};
use crate::types::AudioFormat;
use serde::Serialize;
use std::path::Path;

/// Metadata extracted from an audio file.
#[derive(Debug, Clone, Serialize)]
pub struct AudioInfo {
    pub path: String,
    pub format: AudioFormat,
    pub sample_rate: u32,
    pub channels: u16,
    pub duration_secs: f64,
    pub num_frames: u64,
    pub codec: String,
    pub file_size: u64,
}

impl AudioInfo {
    /// Extract metadata from an audio file using symphonia probe.
    pub fn from_path(path: &Path) -> Result<Self> {
        let file_size = std::fs::metadata(path).map(|m| m.len()).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                PanaudError::FileNotFound {
                    path: path.to_path_buf(),
                    suggestion: "check that the file path is correct".into(),
                }
            } else {
                PanaudError::IoError {
                    message: e.to_string(),
                    path: Some(path.to_path_buf()),
                    suggestion: "check file permissions".into(),
                }
            }
        })?;

        let format = AudioFormat::from_path(path).ok_or_else(|| PanaudError::UnknownFormat {
            path: path.to_path_buf(),
            suggestion: "use a recognized audio extension: wav, mp3, flac, ogg, aac".into(),
        })?;

        let probed = crate::codec::probe::probe_file(path)?;

        let track = probed
            .format
            .default_track()
            .ok_or_else(|| PanaudError::DecodeError {
                message: "no audio track found".into(),
                path: Some(path.to_path_buf()),
                suggestion: "the file does not contain an audio track".into(),
            })?;

        let sample_rate = track.codec_params.sample_rate.unwrap_or(0);
        let channels = track
            .codec_params
            .channels
            .map(|c| c.count() as u16)
            .unwrap_or(0);

        let num_frames = track.codec_params.n_frames.unwrap_or(0);
        let duration_secs = if sample_rate > 0 {
            num_frames as f64 / sample_rate as f64
        } else {
            0.0
        };

        let codec = track.codec_params.codec.to_string();

        Ok(AudioInfo {
            path: path.display().to_string(),
            format,
            sample_rate,
            channels,
            duration_secs,
            num_frames,
            codec,
            file_size,
        })
    }

    /// Format as human-readable text, optionally filtered.
    pub fn to_human_string(&self, fields: &[String]) -> String {
        let all_fields: Vec<(&str, String)> = vec![
            ("path", format!("File:        {}", self.path)),
            ("format", format!("Format:      {}", self.format)),
            ("codec", format!("Codec:       {}", self.codec)),
            (
                "sample_rate",
                format!("Sample Rate: {} Hz", self.sample_rate),
            ),
            ("channels", format!("Channels:    {}", self.channels)),
            (
                "duration_secs",
                format!("Duration:    {}", format_duration(self.duration_secs)),
            ),
            ("num_frames", format!("Frames:      {}", self.num_frames)),
            (
                "file_size",
                format!("File Size:   {}", format_file_size(self.file_size)),
            ),
        ];

        let lines: Vec<String> = if fields.is_empty() {
            all_fields.into_iter().map(|(_, v)| v).collect()
        } else {
            all_fields
                .into_iter()
                .filter(|(k, _)| fields.iter().any(|f| f == k))
                .map(|(_, v)| v)
                .collect()
        };

        lines.join("\n")
    }

    /// Filter to only specified fields as JSON.
    pub fn to_filtered_json(&self, fields: &[String]) -> serde_json::Value {
        let full = serde_json::to_value(self).unwrap_or_default();
        if fields.is_empty() {
            return full;
        }
        let obj = full.as_object().unwrap();
        let filtered: serde_json::Map<String, serde_json::Value> = obj
            .iter()
            .filter(|(k, _)| fields.iter().any(|f| f == *k))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        serde_json::Value::Object(filtered)
    }
}

fn format_duration(secs: f64) -> String {
    let total_secs = secs as u64;
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;
    let frac = secs - total_secs as f64;

    if hours > 0 {
        format!(
            "{hours}:{minutes:02}:{seconds:02}.{:02}",
            (frac * 100.0) as u32
        )
    } else {
        format!("{minutes}:{seconds:02}.{:02}", (frac * 100.0) as u32)
    }
}

fn format_file_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_duration_minutes() {
        assert_eq!(format_duration(90.5), "1:30.50");
    }

    #[test]
    fn format_duration_hours() {
        assert_eq!(format_duration(3661.0), "1:01:01.00");
    }

    #[test]
    fn format_file_size_bytes() {
        assert_eq!(format_file_size(500), "500 B");
    }

    #[test]
    fn format_file_size_kb() {
        assert_eq!(format_file_size(2048), "2.00 KB");
    }

    #[test]
    fn format_file_size_mb() {
        assert_eq!(format_file_size(1_500_000), "1.43 MB");
    }
}
