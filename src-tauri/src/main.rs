// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::process;

use clap::Parser;

use annot_lib::input::InputMode;

#[derive(Parser)]
#[command(name = "annot")]
#[command(about = "Ephemeral annotation tool for AI workflows")]
#[command(long_about = "annot opens a file for annotation.\n\n\
Examples:\n  \
annot document.md              # Open file for annotation\n  \
cat file.go | annot            # Pipe content from stdin\n  \
cat file.go | annot -l main.go # Pipe with label (for syntax highlighting)")]
struct Cli {
    /// File to open for annotation
    #[arg(value_name = "FILE")]
    file: Option<PathBuf>,

    /// Label for stdin content (affects syntax highlighting and output headers)
    #[arg(short = 'l', long = "label", default_value = "stdin")]
    label: String,
}

fn main() {
    let cli = Cli::parse();

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

    // Resolve content from the input mode
    let input = match mode.resolve() {
        Ok(input) => input,
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
    };

    // Create state and run Tauri
    let state = annot_lib::state::AppState::from_file(input.label, &input.content, &input.path_hint);
    annot_lib::run(state);
}
