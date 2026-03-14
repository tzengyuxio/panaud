use crate::app::{ConcatArgs, OutputFormat};
use crate::output;
use panaud_core::codec::CodecRegistry;
use panaud_core::error::PanaudError;
use panaud_core::schema::{CommandSchema, ParamSchema, ParamType};
use panaud_core::types::AudioFormat;
use serde::Serialize;
use std::path::Path;

pub fn schema() -> CommandSchema {
    CommandSchema {
        command: "concat".into(),
        description: "Concatenate multiple audio files into one".into(),
        params: vec![
            ParamSchema {
                name: "inputs".into(),
                param_type: ParamType::Path,
                required: true,
                description: "Input audio files (two or more)".into(),
                default: None,
                choices: None,
                range: None,
            },
            ParamSchema {
                name: "output".into(),
                param_type: ParamType::Path,
                required: true,
                description: "Output file path".into(),
                default: None,
                choices: None,
                range: None,
            },
            ParamSchema {
                name: "overwrite".into(),
                param_type: ParamType::Boolean,
                required: false,
                description: "Overwrite output if it exists".into(),
                default: Some(serde_json::json!(false)),
                choices: None,
                range: None,
            },
        ],
    }
}

#[derive(Serialize)]
struct ConcatPlan {
    inputs: Vec<String>,
    output: String,
}

#[derive(Serialize)]
struct ConcatResult {
    inputs: Vec<String>,
    output: String,
    output_size: u64,
}

pub fn run(args: &ConcatArgs, format: OutputFormat, dry_run: bool, show_schema: bool) -> i32 {
    if show_schema {
        let s = schema();
        output::print_json(&serde_json::to_value(&s).unwrap());
        return 0;
    }

    if args.inputs.is_empty() {
        let err = PanaudError::InvalidArgument {
            message: "missing required argument: inputs".into(),
            suggestion: "usage: panaud concat <file1> <file2> ... -o <output>".into(),
        };
        return output::print_error(format, &err);
    }

    let output_path_str = match &args.output {
        Some(o) => o.clone(),
        None => {
            let err = PanaudError::InvalidArgument {
                message: "missing required argument: output (-o)".into(),
                suggestion: "usage: panaud concat <file1> <file2> ... -o <output>".into(),
            };
            return output::print_error(format, &err);
        }
    };

    let output_path = Path::new(&output_path_str);

    // Check output exists
    if output_path.exists() && !args.overwrite {
        let err = PanaudError::OutputExists {
            path: output_path.to_path_buf(),
            suggestion: "use --overwrite to replace the existing file".into(),
        };
        return output::print_error(format, &err);
    }

    if dry_run {
        let plan = ConcatPlan {
            inputs: args.inputs.clone(),
            output: output_path_str,
        };
        output::print_output(
            format,
            &format!(
                "Would concatenate {} files → {}",
                args.inputs.len(),
                plan.output
            ),
            &plan,
        );
        return 0;
    }

    // Decode first input to establish format
    let first_path = Path::new(&args.inputs[0]);
    let mut combined = match CodecRegistry::decode(first_path) {
        Ok(a) => a,
        Err(e) => return output::print_error(format, &e),
    };

    // Decode and append remaining inputs
    for (i, input) in args.inputs.iter().enumerate().skip(1) {
        let path = Path::new(input);
        let audio = match CodecRegistry::decode(path) {
            Ok(a) => a,
            Err(e) => return output::print_error(format, &e),
        };
        if audio.sample_rate != combined.sample_rate {
            let err = PanaudError::FormatMismatch {
                message: format!(
                    "input {} has sample rate {} Hz, but input 0 has {} Hz",
                    i, audio.sample_rate, combined.sample_rate
                ),
                suggestion: "all inputs must have the same sample rate; use 'panaud resample' to match them first".into(),
            };
            return output::print_error(format, &err);
        }
        if audio.channels != combined.channels {
            let err = PanaudError::FormatMismatch {
                message: format!(
                    "input {} has {} channels, but input 0 has {} channels",
                    i, audio.channels, combined.channels
                ),
                suggestion: "all inputs must have the same channel count; use 'panaud channels' to match them first".into(),
            };
            return output::print_error(format, &err);
        }
        combined.samples.extend(audio.samples);
    }

    // Determine output format
    let out_format = AudioFormat::from_path(output_path).unwrap_or(AudioFormat::Wav);
    if !out_format.can_encode() {
        let err = PanaudError::UnsupportedFormat {
            format: out_format.to_string(),
            suggestion: "use a supported output format extension (e.g. .wav, .flac, .mp3)".into(),
        };
        return output::print_error(format, &err);
    }

    // Encode
    if let Err(e) = CodecRegistry::encode(&combined, output_path, out_format) {
        return output::print_error(format, &e);
    }

    let output_size = std::fs::metadata(output_path).map(|m| m.len()).unwrap_or(0);

    let result = ConcatResult {
        inputs: args.inputs.clone(),
        output: output_path_str,
        output_size,
    };

    output::print_output(
        format,
        &format!(
            "Concatenated {} files → {}",
            result.inputs.len(),
            result.output
        ),
        &result,
    );

    0
}
