pub use pan_common::error::{ExitCode, StructuredError};
use serde::Serialize;
use std::path::PathBuf;

/// Structured error type for panaud.
#[derive(Debug, thiserror::Error, Serialize)]
#[serde(tag = "error", rename_all = "snake_case")]
pub enum PanaudError {
    #[error("file not found: {path}")]
    FileNotFound { path: PathBuf, suggestion: String },

    #[error("permission denied: {path}")]
    PermissionDenied { path: PathBuf, suggestion: String },

    #[error("unsupported format: {format}")]
    UnsupportedFormat { format: String, suggestion: String },

    #[error("unknown format for: {path}")]
    UnknownFormat { path: PathBuf, suggestion: String },

    #[error("decode error: {message}")]
    DecodeError {
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        path: Option<PathBuf>,
        suggestion: String,
    },

    #[error("encode error: {message}")]
    EncodeError {
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        path: Option<PathBuf>,
        suggestion: String,
    },

    #[error("output already exists: {path}")]
    OutputExists { path: PathBuf, suggestion: String },

    #[error("invalid argument: {message}")]
    InvalidArgument { message: String, suggestion: String },

    #[error("invalid time format: {input}")]
    InvalidTimeFormat { input: String, suggestion: String },

    #[error("trim range out of bounds: {message}")]
    TrimOutOfRange { message: String, suggestion: String },

    #[error("format mismatch: {message}")]
    FormatMismatch { message: String, suggestion: String },

    #[error("split error: {message}")]
    SplitError { message: String, suggestion: String },

    #[error("resample error: {message}")]
    ResampleError { message: String, suggestion: String },

    #[error("io error: {message}")]
    IoError {
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        path: Option<PathBuf>,
        suggestion: String,
    },
}

impl StructuredError for PanaudError {
    fn exit_code(&self) -> ExitCode {
        match self {
            Self::FileNotFound { .. } | Self::PermissionDenied { .. } => ExitCode::InputFile,
            Self::OutputExists { .. } => ExitCode::OutputIssue,
            Self::UnsupportedFormat { .. } | Self::UnknownFormat { .. } => ExitCode::Unsupported,
            Self::InvalidArgument { .. }
            | Self::InvalidTimeFormat { .. }
            | Self::TrimOutOfRange { .. }
            | Self::SplitError { .. } => ExitCode::BadArgs,
            Self::FormatMismatch { .. } => ExitCode::BadArgs,
            Self::ResampleError { .. } => ExitCode::OutputIssue,
            Self::DecodeError { .. } => ExitCode::InputFile,
            Self::EncodeError { .. } | Self::IoError { .. } => ExitCode::OutputIssue,
        }
    }

    fn suggestion(&self) -> &str {
        match self {
            Self::FileNotFound { suggestion, .. }
            | Self::PermissionDenied { suggestion, .. }
            | Self::UnsupportedFormat { suggestion, .. }
            | Self::UnknownFormat { suggestion, .. }
            | Self::DecodeError { suggestion, .. }
            | Self::EncodeError { suggestion, .. }
            | Self::OutputExists { suggestion, .. }
            | Self::InvalidArgument { suggestion, .. }
            | Self::InvalidTimeFormat { suggestion, .. }
            | Self::TrimOutOfRange { suggestion, .. }
            | Self::FormatMismatch { suggestion, .. }
            | Self::SplitError { suggestion, .. }
            | Self::ResampleError { suggestion, .. }
            | Self::IoError { suggestion, .. } => suggestion,
        }
    }
}

impl PanaudError {
    /// Convenience constructor for encode errors.
    pub fn encode(
        path: &std::path::Path,
        message: impl Into<String>,
        suggestion: impl Into<String>,
    ) -> Self {
        Self::EncodeError {
            message: message.into(),
            path: Some(path.to_path_buf()),
            suggestion: suggestion.into(),
        }
    }
}

pub type Result<T> = std::result::Result<T, PanaudError>;
