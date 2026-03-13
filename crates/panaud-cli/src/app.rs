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
    /// Input audio file
    pub input: Option<String>,

    /// Output file path (positional)
    pub output_pos: Option<String>,

    /// Output file path
    #[arg(short, long)]
    pub output: Option<String>,

    /// Target format (inferred from output extension if not set)
    #[arg(long)]
    pub to: Option<String>,

    /// Overwrite output if it exists
    #[arg(long)]
    pub overwrite: bool,

    /// Skip if output already exists
    #[arg(long)]
    pub skip_existing: bool,
}

#[derive(clap::Args)]
pub struct TrimArgs {
    /// Input audio file
    pub input: Option<String>,

    /// Output file path (positional)
    pub output_pos: Option<String>,

    /// Output file path
    #[arg(short, long)]
    pub output: Option<String>,

    /// Start time (e.g. '1:30', '90s', '1.5m', '44100S')
    #[arg(short, long)]
    pub start: String,

    /// End time (defaults to end of file)
    #[arg(short, long)]
    pub end: Option<String>,

    /// Overwrite output if it exists
    #[arg(long)]
    pub overwrite: bool,
}
