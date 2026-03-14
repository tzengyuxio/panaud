use crate::app::{IoArgs, OutputFormat};
use crate::output;
use panaud_core::codec::CodecRegistry;
use panaud_core::error::PanaudError;
use panaud_core::pipeline::Pipeline;
use panaud_core::types::AudioFormat;
use std::path::Path;

/// Shared context for pipeline-based commands.
pub struct PipelineContext<'a> {
    pub input: &'a str,
    pub output_path_str: String,
    pub format: OutputFormat,
}

impl<'a> PipelineContext<'a> {
    /// Parse and validate common I/O args. Returns None if validation fails (error already printed).
    pub fn from_io_args(io: &'a IoArgs, format: OutputFormat, usage: &str) -> Option<Self> {
        let input = match &io.input {
            Some(i) => i.as_str(),
            None => {
                let err = PanaudError::InvalidArgument {
                    message: "missing required argument: input".into(),
                    suggestion: format!("usage: {usage}"),
                };
                output::print_error(format, &err);
                return None;
            }
        };

        let output_path_str = match io.output_path() {
            Some(o) => o.to_string(),
            None => {
                let err = PanaudError::InvalidArgument {
                    message: "missing required argument: output (-o)".into(),
                    suggestion: format!("usage: {usage}"),
                };
                output::print_error(format, &err);
                return None;
            }
        };

        Some(Self {
            input,
            output_path_str,
            format,
        })
    }

    /// Run a pipeline: check overwrite → decode → execute → detect format → encode.
    /// Returns output file size on success, or prints error and returns None.
    pub fn run_pipeline(&self, pipeline: &Pipeline, overwrite: bool) -> Option<u64> {
        let input_path = Path::new(self.input);
        let output_path = Path::new(&self.output_path_str);

        // Check output exists
        if output_path.exists() && !overwrite {
            let err = PanaudError::OutputExists {
                path: output_path.to_path_buf(),
                suggestion: "use --overwrite to replace the existing file".into(),
            };
            output::print_error(self.format, &err);
            return None;
        }

        // Decode
        let audio = match CodecRegistry::decode(input_path) {
            Ok(a) => a,
            Err(e) => {
                output::print_error(self.format, &e);
                return None;
            }
        };

        // Apply pipeline
        let result_audio = match pipeline.execute(audio) {
            Ok(a) => a,
            Err(e) => {
                output::print_error(self.format, &e);
                return None;
            }
        };

        // Determine output format
        let out_format = AudioFormat::from_path(output_path)
            .or_else(|| AudioFormat::from_path(input_path))
            .unwrap_or(AudioFormat::Wav);

        if !out_format.can_encode() {
            let err = PanaudError::UnsupportedFormat {
                format: out_format.to_string(),
                suggestion: "use a supported output format extension (e.g. .wav, .flac, .mp3)"
                    .into(),
            };
            output::print_error(self.format, &err);
            return None;
        }

        // Encode
        if let Err(e) = CodecRegistry::encode(&result_audio, output_path, out_format) {
            output::print_error(self.format, &e);
            return None;
        }

        let output_size = std::fs::metadata(output_path).map(|m| m.len()).unwrap_or(0);

        Some(output_size)
    }
}
