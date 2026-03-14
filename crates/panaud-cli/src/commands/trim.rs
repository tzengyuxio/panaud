use crate::app::{OutputFormat, TrimArgs};
use crate::commands::pipeline_runner::PipelineContext;
use crate::output;
use panaud_core::ops::trim::TrimOp;
use panaud_core::ops::Operation;
use panaud_core::pipeline::Pipeline;
use serde::Serialize;

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

    let ctx = match PipelineContext::from_io_args(
        &args.io,
        format,
        "panaud trim <input> -o <output> --start <time>",
    ) {
        Some(c) => c,
        None => return 5,
    };

    let trim_op = match TrimOp::new(&args.start, args.end.as_deref()) {
        Ok(op) => op,
        Err(e) => return output::print_error(format, &e),
    };

    let pipeline = Pipeline::new().push(trim_op);

    if dry_run {
        let plan = pipeline.describe();
        output::print_output(
            format,
            &format!("Would trim {} → {}", ctx.input, ctx.output_path_str),
            &plan,
        );
        return 0;
    }

    let output_size = match ctx.run_pipeline(&pipeline, args.io.overwrite) {
        Some(size) => size,
        None => return 1,
    };

    let result = TrimResult {
        input: ctx.input.to_string(),
        output: ctx.output_path_str,
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
