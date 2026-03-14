use crate::app::{OutputFormat, ResampleArgs};
use crate::commands::pipeline_runner::PipelineContext;
use crate::output;
use panaud_core::ops::Operation;
use panaud_core::pipeline::Pipeline;
use serde::Serialize;

#[cfg(feature = "resample")]
use panaud_core::ops::resample::ResampleOp;

#[derive(Serialize)]
struct ResampleResult {
    input: String,
    output: String,
    target_rate: u32,
    output_size: u64,
}

pub fn run(args: &ResampleArgs, format: OutputFormat, dry_run: bool, show_schema: bool) -> i32 {
    #[cfg(not(feature = "resample"))]
    {
        let _ = (args, format, dry_run, show_schema);
        let err = panaud_core::error::PanaudError::UnsupportedFormat {
            format: "resample".into(),
            suggestion: "rebuild with the 'resample' feature enabled".into(),
        };
        return output::print_error(format, &err);
    }

    #[cfg(feature = "resample")]
    {
        if show_schema {
            let s = ResampleOp::schema();
            output::print_json(&serde_json::to_value(&s).unwrap());
            return 0;
        }

        let ctx = match PipelineContext::from_io_args(
            &args.io,
            format,
            "panaud resample <input> -o <output> --rate <hz>",
        ) {
            Some(c) => c,
            None => return 5,
        };

        let op = match ResampleOp::new(args.rate) {
            Ok(op) => op,
            Err(e) => return output::print_error(format, &e),
        };

        let pipeline = Pipeline::new().push(op);

        if dry_run {
            let plan = pipeline.describe();
            output::print_output(
                format,
                &format!(
                    "Would resample {} → {} at {} Hz",
                    ctx.input, ctx.output_path_str, args.rate
                ),
                &plan,
            );
            return 0;
        }

        let output_size = match ctx.run_pipeline(&pipeline, args.io.overwrite) {
            Some(size) => size,
            None => return 1,
        };

        let result = ResampleResult {
            input: ctx.input.to_string(),
            output: ctx.output_path_str,
            target_rate: args.rate,
            output_size,
        };

        output::print_output(
            format,
            &format!(
                "Resampled {} → {} at {} Hz",
                result.input, result.output, result.target_rate
            ),
            &result,
        );

        0
    }
}
