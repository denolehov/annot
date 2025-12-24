// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::process;

use clap::Parser;

use annot_lib::input::{InputMode, RenderingMode};
use annot_lib::state::AppState;

const EXAMPLES: &str = "\
annot opens a file for annotation.

Examples:
  annot document.md              # Open file for annotation
  cat file.go | annot            # Pipe content from stdin
  cat file.go | annot -l main.go # Pipe with label (for syntax highlighting)
  annot mcp                      # Run as MCP server";

#[derive(Parser)]
#[command(name = "annot")]
#[command(about = "Ephemeral annotation tool for AI workflows")]
#[command(long_about = EXAMPLES)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// File to open for annotation
    #[arg(value_name = "FILE")]
    file: Option<PathBuf>,

    /// Label for stdin content (affects syntax highlighting and output headers)
    #[arg(short = 'l', long = "label", default_value = "stdin")]
    label: String,
}

#[derive(clap::Subcommand)]
enum Command {
    /// Run as MCP server (Model Context Protocol)
    Mcp,
}

fn main() {
    let cli = Cli::parse();

    // Generate context once (avoids duplicate symbol errors)
    let context = tauri::generate_context!();

    // Handle MCP subcommand
    if let Some(Command::Mcp) = cli.command {
        annot_lib::run_mcp(context);
        return;
    }

    // Detect input mode from CLI args and stdin state
    let (mode, warning) = match InputMode::detect(cli.file, cli.label) {
        Ok(result) => result,
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
    };

    // Print warning if both stdin and file were provided
    if let Some(warning) = warning {
        eprintln!("{}", warning);
    }

    // Resolve content from the input mode (reads file/stdin)
    let input = match mode.resolve() {
        Ok(input) => input,
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
    };

    // Load config
    let config = annot_lib::state::UserConfig::load();

    // Create content model based on rendering mode
    let content = match input.rendering_mode {
        RenderingMode::Diff => {
            match annot_lib::state::ContentModel::from_diff(&input.content, input.content_source) {
                Ok(content) => content,
                Err(e) => {
                    eprintln!("Error parsing diff: {}", e);
                    process::exit(1);
                }
            }
        }
        RenderingMode::Markdown => {
            annot_lib::state::ContentModel::from_markdown(&input.content, input.content_source)
        }
        RenderingMode::Source => {
            annot_lib::state::ContentModel::from_file(&input.content, input.content_source)
        }
    };

    let state = AppState::new(content, config);

    annot_lib::run(state, context);
}
