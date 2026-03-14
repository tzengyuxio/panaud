use crate::app::{ConvertArgs, OutputFormat};
use crate::output;
use panaud_core::codec::CodecRegistry;
use panaud_core::error::PanaudError;
use panaud_core::schema::{CommandSchema, ParamSchema, ParamType};
use panaud_core::types::AudioFormat;
use serde::Serialize;
use std::path::Path;

pub fn schema() -> CommandSchema {
    CommandSchema {
        command: "convert".into(),
        description: "Convert audio between formats".into(),
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
                description: "Output file path".into(),
                default: None,
                choices: None,
                range: None,
            },
            ParamSchema {
                name: "to".into(),
                param_type: ParamType::String,
                required: false,
                description: "Target format (inferred from output extension if not set)".into(),
                default: None,
                choices: Some(
                    AudioFormat::all()
                        .iter()
                        .filter(|f| f.can_encode())
                        .map(|f| f.extension().to_string())
                        .collect(),
                ),
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
            ParamSchema {
                name: "skip-existing".into(),
                param_type: ParamType::Boolean,
                required: false,
                description: "Skip if output already exists".into(),
                default: Some(serde_json::json!(false)),
                choices: None,
                range: None,
            },
        ],
    }
}

#[derive(Serialize)]
struct ConvertPlan {
    input: String,
    output: String,
    from_format: String,
    to_format: String,
}

#[derive(Serialize)]
struct ConvertResult {
    input: String,
    output: String,
    from_format: String,
    to_format: String,
    input_size: u64,
    output_size: u64,
}

pub fn run(args: &ConvertArgs, format: OutputFormat, dry_run: bool, show_schema: bool) -> i32 {
    if show_schema {
        let s = schema();
        output::print_json(&serde_json::to_value(&s).unwrap());
        return 0;
    }

    let input = match &args.io.input {
        Some(i) => i,
        None => {
            let err = PanaudError::InvalidArgument {
                message: "missing required argument: input".into(),
                suggestion: "usage: panaud convert <input> -o <output>".into(),
            };
            return output::print_error(format, &err);
        }
    };

    let output_path_str = match args.io.output_path() {
        Some(o) => o.to_string(),
        None => {
            let err = PanaudError::InvalidArgument {
                message: "missing required argument: output (-o)".into(),
                suggestion: "usage: panaud convert <input> -o <output>".into(),
            };
            return output::print_error(format, &err);
        }
    };

    let input_path = Path::new(input);
    let output_path = Path::new(&output_path_str);

    // Determine source format
    let input_format = match AudioFormat::from_path(input_path) {
        Some(f) => f,
        None => {
            let err = PanaudError::UnknownFormat {
                path: input_path.to_path_buf(),
                suggestion: "use a recognized audio extension: wav, mp3, flac, ogg, aac".into(),
            };
            return output::print_error(format, &err);
        }
    };

    // Determine target format
    let target_format = if let Some(to) = &args.to {
        match AudioFormat::from_extension(to) {
            Some(f) => f,
            None => {
                let err = PanaudError::UnsupportedFormat {
                    format: to.clone(),
                    suggestion: "unsupported output format; use a format with encoding support"
                        .into(),
                };
                return output::print_error(format, &err);
            }
        }
    } else {
        match AudioFormat::from_path(output_path) {
            Some(f) => f,
            None => {
                let err = PanaudError::UnknownFormat {
                    path: output_path.to_path_buf(),
                    suggestion: "specify --to <format> or use a recognized output extension".into(),
                };
                return output::print_error(format, &err);
            }
        }
    };

    if !target_format.can_encode() {
        let err = PanaudError::UnsupportedFormat {
            format: target_format.to_string(),
            suggestion: "use a supported output format extension (e.g. .wav, .flac, .mp3)".into(),
        };
        return output::print_error(format, &err);
    }

    // Check output exists
    if output_path.exists() && !args.io.overwrite {
        if args.skip_existing {
            match format {
                OutputFormat::Human => println!("Skipped: output already exists"),
                OutputFormat::Json => {
                    println!(r#"{{"status": "skipped", "reason": "output_exists"}}"#)
                }
            }
            return 0;
        }
        let err = PanaudError::OutputExists {
            path: output_path.to_path_buf(),
            suggestion: "use --overwrite to replace or --skip-existing to skip".into(),
        };
        return output::print_error(format, &err);
    }

    // Dry run
    if dry_run {
        let plan = ConvertPlan {
            input: input.clone(),
            output: output_path_str,
            from_format: input_format.to_string(),
            to_format: target_format.to_string(),
        };
        output::print_output(
            format,
            &format!(
                "Would convert {} ({}) → {} ({})",
                input, input_format, plan.output, target_format
            ),
            &plan,
        );
        return 0;
    }

    // Decode
    let audio = match CodecRegistry::decode(input_path) {
        Ok(a) => a,
        Err(e) => return output::print_error(format, &e),
    };

    let input_size = std::fs::metadata(input_path).map(|m| m.len()).unwrap_or(0);

    // Encode
    if let Err(e) = CodecRegistry::encode(&audio, output_path, target_format) {
        return output::print_error(format, &e);
    }

    let output_size = std::fs::metadata(output_path).map(|m| m.len()).unwrap_or(0);

    let result = ConvertResult {
        input: input.clone(),
        output: output_path_str,
        from_format: input_format.to_string(),
        to_format: target_format.to_string(),
        input_size,
        output_size,
    };

    output::print_output(
        format,
        &format!(
            "Converted {} → {} ({} → {})",
            result.input, result.output, result.from_format, result.to_format
        ),
        &result,
    );

    0
}
