//! `mdtype` CLI. Thin wrapper over the library crates.
//!
//! Every flag maps 1:1 to a library call. The CLI does not contain validation logic.

#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;

/// A type checker for Markdown.
#[derive(Debug, Parser)]
#[command(name = "mdtype", version, about, long_about = None)]
struct Cli {
    /// Files or directories to validate. Defaults to the current directory.
    paths: Vec<PathBuf>,

    /// Path to `.mdtype.yaml`. Defaults to walking up from the current directory.
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Validate every path against this schema, ignoring the glob map.
    #[arg(long, value_name = "FILE")]
    schema: Option<PathBuf>,

    /// Output format. `auto` picks `human` for a tty, `json` otherwise.
    #[arg(short, long, value_enum, default_value_t = Format::Auto)]
    format: Format,

    /// Disable colored output in the human reporter.
    #[arg(long)]
    no_color: bool,

    /// Suppress the summary line.
    #[arg(long)]
    quiet: bool,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum Format {
    /// `human` if stdout is a tty, `json` otherwise.
    Auto,
    /// Colored, grouped-by-file text output.
    Human,
    /// Structured JSON matching the documented contract.
    Json,
}

fn main() -> ExitCode {
    let _cli = Cli::parse();
    // Phase 2.3 wires the full pipeline here. Until then, succeed silently.
    ExitCode::SUCCESS
}
