use crate::app::{OutputFormat, TrimArgs};
use crate::output;
use panaud_core::codec::CodecRegistry;
use panaud_core::error::PanaudError;
use panaud_core::ops::trim::TrimOp;
use panaud_core::ops::Operation;
use panaud_core::pipeline::Pipeline;
use panaud_core::types::AudioFormat;
use serde::Serialize;
use std::path::Path;

#[derive(Serialize)]
struct TrimResult {
    input: String,
    output: String,
    start: String,
    end: String,
    output_size: u64,
}

pub fn run(args: &TrimArgs, format: OutputFormat, dry_run: bool, show_schema: bool) -> i32 {
    if show_schema {
        let s = TrimOp::schema();
        output::print_json(&serde_json::to_value(&s).unwrap());
        return 0;
    }

    let input = match &args.input {
        Some(i) => i,
        None => {
            let err = PanaudError::InvalidArgument {
                message: "missing required argument: input".into(),
                suggestion: "usage: panaud trim <input> -o <output> --start <time>".into(),
            };
            return output::print_error(format, &err);
        }
    };

    let output_path_str = match args.output.as_ref().or(args.output_pos.as_ref()) {
        Some(o) => o.clone(),
        None => {
            let err = PanaudError::InvalidArgument {
                message: "missing required argument: output (-o)".into(),
                suggestion: "usage: panaud trim <input> -o <output> --start <time>".into(),
            };
            return output::print_error(format, &err);
        }
    };

    let trim_op = match TrimOp::new(&args.start, args.end.as_deref()) {
        Ok(op) => op,
        Err(e) => return output::print_error(format, &e),
    };

    let input_path = Path::new(input);
    let output_path = Path::new(&output_path_str);

    let pipeline = Pipeline::new().push(trim_op);

    if dry_run {
        let plan = pipeline.describe();
        output::print_output(
            format,
            &format!("Would trim {} → {}", input, output_path_str),
            &plan,
        );
        return 0;
    }

    // Check output exists
    if output_path.exists() && !args.overwrite {
        let err = PanaudError::OutputExists {
            path: output_path.to_path_buf(),
            suggestion: "use --overwrite to replace the existing file".into(),
        };
        return output::print_error(format, &err);
    }

    // Decode
    let audio = match CodecRegistry::decode(input_path) {
        Ok(a) => a,
        Err(e) => return output::print_error(format, &e),
    };

    // Apply pipeline
    let result_audio = match pipeline.execute(audio) {
        Ok(a) => a,
        Err(e) => return output::print_error(format, &e),
    };

    // Determine output format
    let out_format = AudioFormat::from_path(output_path)
        .or_else(|| AudioFormat::from_path(input_path))
        .unwrap_or(AudioFormat::Wav);

    if !out_format.can_encode() {
        let err = PanaudError::UnsupportedFormat {
            format: out_format.to_string(),
            suggestion: "v0.1.0 only supports WAV output; use .wav extension".into(),
        };
        return output::print_error(format, &err);
    }

    // Encode
    if let Err(e) = CodecRegistry::encode(&result_audio, output_path, out_format) {
        return output::print_error(format, &e);
    }

    let output_size = std::fs::metadata(output_path).map(|m| m.len()).unwrap_or(0);

    let result = TrimResult {
        input: input.clone(),
        output: output_path_str,
        start: args.start.clone(),
        end: args.end.clone().unwrap_or_else(|| "end".into()),
        output_size,
    };

    output::print_output(
        format,
        &format!("Trimmed {} → {}", result.input, result.output),
        &result,
    );

    0
}
