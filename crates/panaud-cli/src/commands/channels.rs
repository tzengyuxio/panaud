use crate::app::{ChannelsArgs, OutputFormat};
use crate::commands::pipeline_runner::PipelineContext;
use crate::output;
use panaud_core::ops::channels::{ChannelMode, ChannelSelector, ChannelsOp};
use panaud_core::ops::Operation;
use panaud_core::pipeline::Pipeline;
use serde::Serialize;

#[derive(Serialize)]
struct ChannelsResult {
    input: String,
    output: String,
    mode: String,
    output_size: u64,
}

fn parse_mode(args: &ChannelsArgs, format: OutputFormat) -> Option<ChannelMode> {
    if args.mono {
        return Some(ChannelMode::Mono);
    }
    if args.stereo {
        return Some(ChannelMode::Stereo);
    }
    if let Some(n) = args.count {
        return Some(ChannelMode::Count(n));
    }
    if let Some(ref sel) = args.extract {
        let selector = match sel.to_lowercase().as_str() {
            "left" | "l" => ChannelSelector::Left,
            "right" | "r" => ChannelSelector::Right,
            other => match other.parse::<u16>() {
                Ok(i) => ChannelSelector::Index(i),
                Err(_) => {
                    let err = panaud_core::error::PanaudError::InvalidArgument {
                        message: format!("invalid channel selector: '{sel}'"),
                        suggestion:
                            "use 'left', 'right', or a numeric channel index (e.g. '0')".into(),
                    };
                    output::print_error(format, &err);
                    return None;
                }
            },
        };
        return Some(ChannelMode::Extract(selector));
    }

    let err = panaud_core::error::PanaudError::InvalidArgument {
        message: "no channel mode specified".into(),
        suggestion: "use --mono, --stereo, --count <N>, or --extract <channel>".into(),
    };
    output::print_error(format, &err);
    None
}

pub fn run(args: &ChannelsArgs, format: OutputFormat, dry_run: bool, show_schema: bool) -> i32 {
    if show_schema {
        let s = ChannelsOp::schema();
        output::print_json(&serde_json::to_value(&s).unwrap());
        return 0;
    }

    let ctx = match PipelineContext::from_io_args(
        &args.io,
        format,
        "panaud channels <input> -o <output> --mono|--stereo|--count N|--extract <ch>",
    ) {
        Some(c) => c,
        None => return 5,
    };

    let mode = match parse_mode(args, format) {
        Some(m) => m,
        None => return 5,
    };

    let mode_str = mode.to_string();

    let op = ChannelsOp::new(mode);
    let pipeline = Pipeline::new().push(op);

    if dry_run {
        let plan = pipeline.describe();
        output::print_output(
            format,
            &format!(
                "Would change channels ({}) {} → {}",
                mode_str, ctx.input, ctx.output_path_str
            ),
            &plan,
        );
        return 0;
    }

    let output_size = match ctx.run_pipeline(&pipeline, args.io.overwrite) {
        Some(size) => size,
        None => return 1,
    };

    let result = ChannelsResult {
        input: ctx.input.to_string(),
        output: ctx.output_path_str,
        mode: mode_str,
        output_size,
    };

    output::print_output(
        format,
        &format!(
            "Changed channels ({}) {} → {}",
            result.mode, result.input, result.output
        ),
        &result,
    );

    0
}
