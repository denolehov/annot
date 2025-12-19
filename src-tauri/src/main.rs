// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::process;

use clap::Parser;

#[derive(Parser)]
#[command(name = "annot")]
#[command(about = "Ephemeral annotation tool for AI workflows")]
struct Cli {
    /// File to open for annotation
    file: PathBuf,
}

fn main() {
    let cli = Cli::parse();

    // Read file content
    let content = match std::fs::read_to_string(&cli.file) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error reading file '{}': {}", cli.file.display(), e);
            process::exit(1);
        }
    };

    // Extract filename for display
    let label = cli
        .file
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| cli.file.display().to_string());

    // Create state and run Tauri
    let state = annot_lib::state::AppState::from_file(label, &content);
    annot_lib::run(state);
}
