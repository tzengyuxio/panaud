use crate::app::{InfoArgs, OutputFormat};
use crate::output;
use panaud_core::error::PanaudError;
use panaud_core::info::AudioInfo;
use panaud_core::schema::{CommandSchema, ParamSchema, ParamType};
use std::path::Path;

pub fn schema() -> CommandSchema {
    CommandSchema {
        command: "info".into(),
        description: "Show audio file metadata and properties".into(),
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
                name: "fields".into(),
                param_type: ParamType::String,
                required: false,
                description: "Comma-separated list of fields to include in output".into(),
                default: None,
                choices: Some(vec![
                    "path".into(),
                    "format".into(),
                    "codec".into(),
                    "sample_rate".into(),
                    "channels".into(),
                    "duration_secs".into(),
                    "num_frames".into(),
                    "file_size".into(),
                ]),
                range: None,
            },
        ],
    }
}

pub fn run(args: &InfoArgs, format: OutputFormat, show_schema: bool) -> i32 {
    if show_schema {
        let s = schema();
        output::print_json(&serde_json::to_value(&s).unwrap());
        return 0;
    }

    let input = match &args.input {
        Some(i) => i,
        None => {
            let err = PanaudError::InvalidArgument {
                message: "missing required argument: input".into(),
                suggestion: "usage: panaud info <file>".into(),
            };
            return output::print_error(format, &err);
        }
    };

    let path = Path::new(input);
    let info = match AudioInfo::from_path(path) {
        Ok(i) => i,
        Err(e) => return output::print_error(format, &e),
    };

    let fields: Vec<String> = args
        .fields
        .as_ref()
        .map(|f| f.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_default();

    match format {
        OutputFormat::Human => {
            println!("{}", info.to_human_string(&fields));
        }
        OutputFormat::Json => {
            let json = info.to_filtered_json(&fields);
            output::print_json(&json);
        }
    }

    0
}
