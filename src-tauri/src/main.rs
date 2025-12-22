// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::process;
use std::time::Instant;

use clap::Parser;

use annot_lib::input::InputMode;

/// Times an expression and prints the duration to stderr if `ANNOT_PERF` is set.
macro_rules! timed {
    ($label:expr, $expr:expr) => {{
        let _start = Instant::now();
        let _result = $expr;
        if std::env::var("ANNOT_PERF").is_ok() {
            eprintln!("[perf] {}: {:?}", $label, _start.elapsed());
        }
        _result
    }};
}

#[derive(Parser)]
#[command(name = "annot")]
#[command(about = "Ephemeral annotation tool for AI workflows")]
#[command(long_about = "annot opens a file for annotation.\n\n\
Examples:\n  \
annot document.md              # Open file for annotation\n  \
cat file.go | annot            # Pipe content from stdin\n  \
cat file.go | annot -l main.go # Pipe with label (for syntax highlighting)\n  \
annot mcp                      # Run as MCP server")]
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
    let startup_start = Instant::now();

    let cli = timed!("cli_parse", Cli::parse());

    // Generate context once (avoids duplicate symbol errors)
    let context = timed!("generate_context", tauri::generate_context!());

    // Handle MCP subcommand
    if let Some(Command::Mcp) = cli.command {
        annot_lib::run_mcp(context);
        return;
    }

    // Detect input mode from CLI args and stdin state
    let (mode, warning) = timed!(
        "detect_input_mode",
        match InputMode::detect(cli.file, cli.label) {
            Ok(result) => result,
            Err(e) => {
                eprintln!("{}", e);
                process::exit(1);
            }
        }
    );

    // Print warning if both stdin and file were provided
    if let Some(warning) = warning {
        eprintln!("{}", warning);
    }

    // Resolve content from the input mode (reads file/stdin)
    let input = timed!(
        "resolve_input",
        match mode.resolve() {
            Ok(input) => input,
            Err(e) => {
                eprintln!("{}", e);
                process::exit(1);
            }
        }
    );

    // Load config
    let tags = timed!("load_tags", annot_lib::config::load_tags());
    let exit_modes = timed!("load_exit_modes", annot_lib::config::load_exit_modes());

    // Create state based on content type (priority: diff > markdown > file)
    let state = timed!(
        "create_state",
        if input.is_diff {
            match annot_lib::state::AppState::from_diff(
                input.label,
                &input.content,
                tags,
                exit_modes,
            ) {
                Ok(state) => state,
                Err(e) => {
                    eprintln!("Error parsing diff: {}", e);
                    process::exit(1);
                }
            }
        } else if input.is_markdown {
            annot_lib::state::AppState::from_markdown(
                input.label,
                &input.content,
                &input.path_hint,
                tags,
                exit_modes,
                false, // CLI mode: not ephemeral
            )
        } else {
            annot_lib::state::AppState::from_file(
                input.label,
                &input.content,
                &input.path_hint,
                tags,
                exit_modes,
            )
        }
    );

    if std::env::var("ANNOT_PERF").is_ok() {
        eprintln!("[perf] total_startup: {:?}", startup_start.elapsed());
    }

    annot_lib::run(state, context);
}
