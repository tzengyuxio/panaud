use crate::error::PanaudError;
use crate::types::AudioData;

pub use pan_common::pipeline::PipelinePlan;

/// Audio processing pipeline — a type alias for the generic pipeline
/// specialized with `AudioData` and `PanaudError`.
pub type Pipeline = pan_common::pipeline::Pipeline<AudioData, PanaudError>;
