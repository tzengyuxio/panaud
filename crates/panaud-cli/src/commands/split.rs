use crate::app::{OutputFormat, SplitArgs};
use crate::output;
use panaud_core::codec::CodecRegistry;
use panaud_core::error::PanaudError;
use panaud_core::ops::split::{split_audio, SplitMode};
use panaud_core::schema::{CommandSchema, ParamSchema, ParamType};
use panaud_core::time::parse_time;
use panaud_core::types::AudioFormat;
use serde::Serialize;
use std::path::Path;

pub fn schema() -> CommandSchema {
    CommandSchema {
        command: "split".into(),
        description: "Split audio into multiple files".into(),
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
                description: "Output directory or file prefix".into(),
                default: None,
                choices: None,
                range: None,
            },
            ParamSchema {
                name: "at".into(),
                param_type: ParamType::String,
                required: false,
                description: "Split at specific time points (comma-separated)".into(),
                default: None,
                choices: None,
                range: None,
            },
            ParamSchema {
                name: "count".into(),
                param_type: ParamType::Integer,
                required: false,
                description: "Split into N equal parts".into(),
                default: None,
                choices: None,
                range: None,
            },
            ParamSchema {
                name: "duration".into(),
                param_type: ParamType::String,
                required: false,
                description: "Split into chunks of a given duration".into(),
                default: None,
                choices: None,
                range: None,
            },
        ],
    }
}

#[derive(Serialize)]
struct SplitPlan {
    input: String,
    output_dir: String,
    mode: String,
}

#[derive(Serialize)]
struct SplitResult {
    input: String,
    output_dir: String,
    parts: usize,
    files: Vec<String>,
}

fn parse_split_mode(args: &SplitArgs, format: OutputFormat) -> Option<SplitMode> {
    if let Some(ref at_str) = args.at {
        let time_strs: Vec<&str> = at_str.split(',').collect();
        let mut specs = Vec::new();
        for ts in time_strs {
            match parse_time(ts.trim()) {
                Ok(spec) => specs.push(spec),
                Err(e) => {
                    output::print_error(format, &e);
                    return None;
                }
            }
        }
        return Some(SplitMode::At(specs));
    }

    if let Some(count) = args.count {
        return Some(SplitMode::Count(count));
    }

    if let Some(ref dur_str) = args.duration {
        match parse_time(dur_str) {
            Ok(spec) => return Some(SplitMode::Duration(spec)),
            Err(e) => {
                output::print_error(format, &e);
                return None;
            }
        }
    }

    let err = PanaudError::InvalidArgument {
        message: "no split mode specified".into(),
        suggestion: "use --at <times>, --count <N>, or --duration <time>".into(),
    };
    output::print_error(format, &err);
    None
}

pub fn run(args: &SplitArgs, format: OutputFormat, dry_run: bool, show_schema: bool) -> i32 {
    if show_schema {
        let s = schema();
        output::print_json(&serde_json::to_value(&s).unwrap());
        return 0;
    }

    let input = match &args.input {
        Some(i) => i.as_str(),
        None => {
            let err = PanaudError::InvalidArgument {
                message: "missing required argument: input".into(),
                suggestion: "usage: panaud split <input> -o <output_dir> --count N".into(),
            };
            return output::print_error(format, &err);
        }
    };

    let output_dir_str = match &args.output {
        Some(o) => o.clone(),
        None => {
            let err = PanaudError::InvalidArgument {
                message: "missing required argument: output (-o)".into(),
                suggestion: "usage: panaud split <input> -o <output_dir> --count N".into(),
            };
            return output::print_error(format, &err);
        }
    };

    let mode = match parse_split_mode(args, format) {
        Some(m) => m,
        None => return 5,
    };

    let mode_str = match &mode {
        SplitMode::At(_) => format!("at {}", args.at.as_deref().unwrap_or("")),
        SplitMode::Count(n) => format!("{n} equal parts"),
        SplitMode::Duration(_) => format!("every {}", args.duration.as_deref().unwrap_or("")),
    };

    if dry_run {
        let plan = SplitPlan {
            input: input.to_string(),
            output_dir: output_dir_str,
            mode: mode_str,
        };
        output::print_output(
            format,
            &format!(
                "Would split {} → {} ({})",
                input, plan.output_dir, plan.mode
            ),
            &plan,
        );
        return 0;
    }

    // Decode input
    let input_path = Path::new(input);
    let audio = match CodecRegistry::decode(input_path) {
        Ok(a) => a,
        Err(e) => return output::print_error(format, &e),
    };

    // Split
    let segments = match split_audio(&audio, &mode) {
        Ok(s) => s,
        Err(e) => return output::print_error(format, &e),
    };

    // Determine output format from input
    let out_format = AudioFormat::from_path(input_path).unwrap_or(AudioFormat::Wav);
    if !out_format.can_encode() {
        let err = PanaudError::UnsupportedFormat {
            format: out_format.to_string(),
            suggestion: "use an input format that supports encoding (e.g. .wav, .flac, .mp3)"
                .into(),
        };
        return output::print_error(format, &err);
    }

    // Create output directory
    let output_dir = Path::new(&output_dir_str);
    if let Err(e) = std::fs::create_dir_all(output_dir) {
        let err = PanaudError::IoError {
            message: format!("failed to create output directory: {e}"),
            path: Some(output_dir.to_path_buf()),
            suggestion: "check that the parent directory exists and is writable".into(),
        };
        return output::print_error(format, &err);
    }

    // Determine stem and extension from input
    let stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("part");
    let ext = out_format.extension();

    let mut output_files = Vec::new();
    for (i, segment) in segments.iter().enumerate() {
        let filename = format!("{}_{:03}.{}", stem, i + 1, ext);
        let out_path = output_dir.join(&filename);

        if out_path.exists() && !args.overwrite {
            let err = PanaudError::OutputExists {
                path: out_path,
                suggestion: "use --overwrite to replace existing files".into(),
            };
            return output::print_error(format, &err);
        }

        if let Err(e) = CodecRegistry::encode(segment, &out_path, out_format) {
            return output::print_error(format, &e);
        }

        output_files.push(out_path.to_string_lossy().to_string());
    }

    let result = SplitResult {
        input: input.to_string(),
        output_dir: output_dir_str,
        parts: segments.len(),
        files: output_files,
    };

    output::print_output(
        format,
        &format!(
            "Split {} into {} parts → {}",
            result.input, result.parts, result.output_dir
        ),
        &result,
    );

    0
}
