use crate::app::{NormalizeArgs, OutputFormat};
use crate::commands::pipeline_runner::PipelineContext;
use crate::output;
use panaud_core::ops::normalize::NormalizeOp;
use panaud_core::ops::Operation;
use panaud_core::pipeline::Pipeline;
use serde::Serialize;

#[derive(Serialize)]
struct NormalizeResult {
    input: String,
    output: String,
    target_db: f32,
    output_size: u64,
}

pub fn run(args: &NormalizeArgs, format: OutputFormat, dry_run: bool, show_schema: bool) -> i32 {
    if show_schema {
        let s = NormalizeOp::schema();
        output::print_json(&serde_json::to_value(&s).unwrap());
        return 0;
    }

    let ctx = match PipelineContext::from_io_args(
        &args.io,
        format,
        "panaud normalize <input> -o <output>",
    ) {
        Some(c) => c,
        None => return 5,
    };

    let target_db = args.target.unwrap_or(-1.0);
    let normalize_op = NormalizeOp::new(target_db);
    let pipeline = Pipeline::new().push(normalize_op);

    if dry_run {
        let plan = pipeline.describe();
        output::print_output(
            format,
            &format!("Would normalize {} → {}", ctx.input, ctx.output_path_str),
            &plan,
        );
        return 0;
    }

    let output_size = match ctx.run_pipeline(&pipeline, args.io.overwrite) {
        Some(size) => size,
        None => return 1,
    };

    let result = NormalizeResult {
        input: ctx.input.to_string(),
        output: ctx.output_path_str,
        target_db,
        output_size,
    };

    output::print_output(
        format,
        &format!("Normalized {} → {}", result.input, result.output),
        &result,
    );

    0
}
