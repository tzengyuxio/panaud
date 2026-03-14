use crate::app::{FadeArgs, OutputFormat};
use crate::commands::pipeline_runner::PipelineContext;
use crate::output;
use panaud_core::ops::fade::FadeOp;
use panaud_core::ops::Operation;
use panaud_core::pipeline::Pipeline;
use serde::Serialize;

#[derive(Serialize)]
struct FadeResult {
    input: String,
    output: String,
    fade_in: Option<String>,
    fade_out: Option<String>,
    output_size: u64,
}

pub fn run(args: &FadeArgs, format: OutputFormat, dry_run: bool, show_schema: bool) -> i32 {
    if show_schema {
        let s = FadeOp::schema();
        output::print_json(&serde_json::to_value(&s).unwrap());
        return 0;
    }

    let ctx = match PipelineContext::from_io_args(
        &args.io,
        format,
        "panaud fade <input> -o <output> --in <time> --out <time>",
    ) {
        Some(c) => c,
        None => return 5,
    };

    let fade_op = match FadeOp::new(args.fade_in.as_deref(), args.fade_out.as_deref()) {
        Ok(op) => op,
        Err(e) => return output::print_error(format, &e),
    };

    let pipeline = Pipeline::new().push(fade_op);

    if dry_run {
        let plan = pipeline.describe();
        output::print_output(
            format,
            &format!("Would apply fade to {} → {}", ctx.input, ctx.output_path_str),
            &plan,
        );
        return 0;
    }

    let output_size = match ctx.run_pipeline(&pipeline, args.io.overwrite) {
        Some(size) => size,
        None => return 1,
    };

    let result = FadeResult {
        input: ctx.input.to_string(),
        output: ctx.output_path_str,
        fade_in: args.fade_in.clone(),
        fade_out: args.fade_out.clone(),
        output_size,
    };

    output::print_output(
        format,
        &format!("Applied fade to {} → {}", result.input, result.output),
        &result,
    );

    0
}
