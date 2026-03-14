use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "panaud",
    version,
    about = "The Swiss Army knife of audio processing — built for humans and AI agents alike.",
    long_about = "A modern, AI-agent-friendly audio processing tool with structured output, \
                  dry-run support, and consistent syntax."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Output format
    #[arg(long, global = true, default_value = "human")]
    pub format: OutputFormat,

    /// Preview operations without executing
    #[arg(long, global = true)]
    pub dry_run: bool,

    /// Show command parameter schema as JSON
    #[arg(long, global = true)]
    pub schema: bool,

    /// List all supported commands, formats, and features
    #[arg(long)]
    pub capabilities: bool,
}

pub use pan_common::output::OutputFormat;

#[derive(Subcommand)]
pub enum Commands {
    /// Show audio file metadata and properties
    Info(InfoArgs),

    /// Convert audio between formats
    Convert(ConvertArgs),

    /// Trim audio to a time range
    Trim(TrimArgs),

    /// Adjust audio volume
    Volume(VolumeArgs),

    /// Peak-normalize audio
    Normalize(NormalizeArgs),
}

/// Common I/O arguments shared by processing commands.
#[derive(clap::Args)]
pub struct IoArgs {
    /// Input audio file
    pub input: Option<String>,

    /// Output file path (positional)
    pub output_pos: Option<String>,

    /// Output file path
    #[arg(short, long)]
    pub output: Option<String>,

    /// Overwrite output if it exists
    #[arg(long)]
    pub overwrite: bool,
}

impl IoArgs {
    /// Resolve the effective output path from positional or -o flag.
    pub fn output_path(&self) -> Option<&str> {
        self.output
            .as_deref()
            .or(self.output_pos.as_deref())
    }
}

#[derive(clap::Args)]
pub struct InfoArgs {
    /// Input audio file
    pub input: Option<String>,

    /// Comma-separated list of fields to include in output
    #[arg(long)]
    pub fields: Option<String>,
}

#[derive(clap::Args)]
pub struct ConvertArgs {
    #[command(flatten)]
    pub io: IoArgs,

    /// Target format (inferred from output extension if not set)
    #[arg(long)]
    pub to: Option<String>,

    /// Skip if output already exists
    #[arg(long)]
    pub skip_existing: bool,
}

#[derive(clap::Args)]
pub struct TrimArgs {
    #[command(flatten)]
    pub io: IoArgs,

    /// Start time (e.g. '1:30', '90s', '1.5m', '44100S')
    #[arg(short, long)]
    pub start: String,

    /// End time (defaults to end of file)
    #[arg(short, long)]
    pub end: Option<String>,
}

#[derive(clap::Args)]
pub struct VolumeArgs {
    #[command(flatten)]
    pub io: IoArgs,

    /// Gain in dB (e.g. -3 for quieter, +6 for louder)
    #[arg(long)]
    pub gain: Option<f32>,

    /// Linear volume factor (e.g. 0.5 for half, 2.0 for double)
    #[arg(long)]
    pub factor: Option<f32>,
}

#[derive(clap::Args)]
pub struct NormalizeArgs {
    #[command(flatten)]
    pub io: IoArgs,

    /// Target peak level in dBFS (default: -1.0)
    #[arg(long)]
    pub target: Option<f32>,
}
