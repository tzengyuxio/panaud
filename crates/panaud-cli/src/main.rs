mod app;
mod commands;
mod output;

use app::{Cli, Commands, OutputFormat};
use clap::Parser;
use panaud_core::types::AudioFormat;
use serde::Serialize;

#[derive(Serialize)]
struct Capabilities {
    version: String,
    commands: Vec<CommandCap>,
    formats: Vec<FormatCap>,
    global_flags: Vec<String>,
}

#[derive(Serialize)]
struct CommandCap {
    name: String,
    description: String,
}

#[derive(Serialize)]
struct FormatCap {
    name: String,
    extension: String,
    can_decode: bool,
    can_encode: bool,
}

fn capabilities() -> Capabilities {
    Capabilities {
        version: env!("CARGO_PKG_VERSION").to_string(),
        commands: vec![
            CommandCap {
                name: "info".into(),
                description: "Show audio file metadata and properties".into(),
            },
            CommandCap {
                name: "convert".into(),
                description: "Convert audio between formats".into(),
            },
            CommandCap {
                name: "trim".into(),
                description: "Trim audio to a time range".into(),
            },
            CommandCap {
                name: "volume".into(),
                description: "Adjust audio volume".into(),
            },
            CommandCap {
                name: "normalize".into(),
                description: "Peak-normalize audio".into(),
            },
            CommandCap {
                name: "fade".into(),
                description: "Apply fade-in/fade-out to audio".into(),
            },
            CommandCap {
                name: "channels".into(),
                description: "Change audio channel layout".into(),
            },
            CommandCap {
                name: "resample".into(),
                description: "Resample audio to a different sample rate".into(),
            },
            CommandCap {
                name: "concat".into(),
                description: "Concatenate multiple audio files".into(),
            },
            CommandCap {
                name: "split".into(),
                description: "Split audio into multiple files".into(),
            },
        ],
        formats: AudioFormat::all()
            .iter()
            .map(|f| FormatCap {
                name: f.to_string(),
                extension: f.extension().to_string(),
                can_decode: f.can_decode(),
                can_encode: f.can_encode(),
            })
            .collect(),
        global_flags: vec![
            "--format <human|json>".into(),
            "--dry-run".into(),
            "--schema".into(),
            "--capabilities".into(),
        ],
    }
}

fn main() {
    let cli = Cli::parse();

    // Handle --capabilities
    if cli.capabilities {
        let caps = capabilities();
        match cli.format {
            OutputFormat::Human => {
                println!("panaud v{}", caps.version);
                println!();
                println!("Commands:");
                for cmd in &caps.commands {
                    println!("  {:12} {}", cmd.name, cmd.description);
                }
                println!();
                println!("Supported formats:");
                for fmt in &caps.formats {
                    let decode = if fmt.can_decode { "decode" } else { "-" };
                    let encode = if fmt.can_encode { "encode" } else { "-" };
                    println!(
                        "  {:10} .{:5} {:6} {:6}",
                        fmt.name, fmt.extension, decode, encode
                    );
                }
                println!();
                println!("Global flags:");
                for flag in &caps.global_flags {
                    println!("  {flag}");
                }
            }
            OutputFormat::Json => {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&caps).unwrap_or_else(|_| "{}".into())
                );
            }
        }
        std::process::exit(0);
    }

    let exit_code = match &cli.command {
        Some(Commands::Info(args)) => commands::info::run(args, cli.format, cli.schema),
        Some(Commands::Convert(args)) => {
            commands::convert::run(args, cli.format, cli.dry_run, cli.schema)
        }
        Some(Commands::Trim(args)) => {
            commands::trim::run(args, cli.format, cli.dry_run, cli.schema)
        }
        Some(Commands::Volume(args)) => {
            commands::volume::run(args, cli.format, cli.dry_run, cli.schema)
        }
        Some(Commands::Normalize(args)) => {
            commands::normalize::run(args, cli.format, cli.dry_run, cli.schema)
        }
        Some(Commands::Fade(args)) => {
            commands::fade::run(args, cli.format, cli.dry_run, cli.schema)
        }
        Some(Commands::Channels(args)) => {
            commands::channels::run(args, cli.format, cli.dry_run, cli.schema)
        }
        Some(Commands::Resample(args)) => {
            commands::resample::run(args, cli.format, cli.dry_run, cli.schema)
        }
        Some(Commands::Concat(args)) => {
            commands::concat::run(args, cli.format, cli.dry_run, cli.schema)
        }
        Some(Commands::Split(args)) => {
            commands::split::run(args, cli.format, cli.dry_run, cli.schema)
        }
        None => {
            use clap::CommandFactory;
            Cli::command().print_help().ok();
            println!();
            0
        }
    };

    std::process::exit(exit_code);
}
