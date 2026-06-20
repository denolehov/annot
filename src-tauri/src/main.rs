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
#[command(version)]
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

    /// Output annotations as JSON (includes base64 images)
    #[arg(long)]
    json: bool,

    /// Add an exit mode button: "name:instruction" (repeatable)
    ///
    /// Example: --exit-mode "Apply:Apply the changes" --exit-mode "Reject:Discard"
    #[arg(long = "exit-mode", value_name = "NAME:INSTRUCTION")]
    exit_modes: Vec<String>,
}

#[derive(clap::Subcommand)]
enum Command {
    /// Run as MCP server (Model Context Protocol)
    Mcp,
    /// Print version information
    Version,
}

fn main() {
    // Suppress macOS system logs (XPC, CoreAnalytics, etc.) in release builds
    #[cfg(all(target_os = "macos", not(debug_assertions)))]
    std::env::set_var("OS_ACTIVITY_MODE", "disable");

    let cli = Cli::parse();

    // Handle version subcommand (doesn't need Tauri)
    if let Some(Command::Version) = cli.command {
        println!("annot {}", env!("CARGO_PKG_VERSION"));
        return;
    }

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
    let mut config = annot_lib::state::UserConfig::load();

    // Parse CLI exit modes and prepend as transient
    if !cli.exit_modes.is_empty() {
        let default_colors = ["#22c55e", "#eab308", "#ef4444", "#3b82f6", "#a855f7", "#f97316"];
        let transient_modes: Vec<annot_lib::state::ExitMode> = cli
            .exit_modes
            .iter()
            .enumerate()
            .filter_map(|(i, s)| {
                let (name, instruction) = s.split_once(':')?;
                Some(annot_lib::state::ExitMode {
                    id: annot_lib::id::generate(),
                    name: name.trim().to_string(),
                    color: default_colors[i % default_colors.len()].to_string(),
                    instruction: instruction.trim().to_string(),
                    order: i as u32,
                    source: annot_lib::state::ExitModeSource::Transient,
                })
            })
            .collect();
        config.prepend_transient_modes(transient_modes);
    }

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

    annot_lib::run(state, context, cli.json);
}
