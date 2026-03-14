use crate::app::{OutputFormat, VolumeArgs};
use crate::commands::pipeline_runner::PipelineContext;
use crate::output;
use panaud_core::error::PanaudError;
use panaud_core::ops::volume::VolumeOp;
use panaud_core::ops::Operation;
use panaud_core::pipeline::Pipeline;
use serde::Serialize;

#[derive(Serialize)]
struct VolumeResult {
    input: String,
    output: String,
    gain_db: Option<f32>,
    factor: Option<f32>,
    output_size: u64,
}

pub fn run(args: &VolumeArgs, format: OutputFormat, dry_run: bool, show_schema: bool) -> i32 {
    if show_schema {
        let s = VolumeOp::schema();
        output::print_json(&serde_json::to_value(&s).unwrap());
        return 0;
    }

    let ctx = match PipelineContext::from_io_args(
        &args.io,
        format,
        "panaud volume <input> -o <output> --gain <dB>",
    ) {
        Some(c) => c,
        None => return 5,
    };

    let volume_op = match (args.gain, args.factor) {
        (Some(db), None) => VolumeOp::from_db(db),
        (None, Some(f)) => match VolumeOp::from_factor(f) {
            Ok(op) => op,
            Err(e) => return output::print_error(format, &e),
        },
        (Some(_), Some(_)) => {
            let err = PanaudError::InvalidArgument {
                message: "cannot specify both --gain and --factor".into(),
                suggestion: "use either --gain <dB> or --factor <value>, not both".into(),
            };
            return output::print_error(format, &err);
        }
        (None, None) => {
            let err = PanaudError::InvalidArgument {
                message: "must specify --gain or --factor".into(),
                suggestion:
                    "use --gain <dB> (e.g. --gain -3) or --factor <value> (e.g. --factor 0.5)"
                        .into(),
            };
            return output::print_error(format, &err);
        }
    };

    let pipeline = Pipeline::new().push(volume_op);

    if dry_run {
        let plan = pipeline.describe();
        output::print_output(
            format,
            &format!(
                "Would adjust volume {} → {}",
                ctx.input, ctx.output_path_str
            ),
            &plan,
        );
        return 0;
    }

    let output_size = match ctx.run_pipeline(&pipeline, args.io.overwrite) {
        Some(size) => size,
        None => return 1,
    };

    let result = VolumeResult {
        input: ctx.input.to_string(),
        output: ctx.output_path_str,
        gain_db: args.gain,
        factor: args.factor,
        output_size,
    };

    output::print_output(
        format,
        &format!("Adjusted volume {} → {}", result.input, result.output),
        &result,
    );

    0
}
