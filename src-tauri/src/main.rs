// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::process;

use clap::Parser;

use annot_lib::input::{CliSource, ContentSource, InputMode, RenderingMode};
use annot_lib::state::AppState;

const EXAMPLES: &str = "\
annot opens a file for annotation.

Examples:
  annot document.md              # Open file for annotation
  cat file.go | annot            # Pipe content from stdin
  cat file.go | annot -l main.go # Pipe with label (for syntax highlighting)
  annot diff                     # Review working-tree changes vs HEAD
  annot diff --staged            # Review staged changes
  annot diff main..HEAD -- src/  # Review a revision range, limited to src/
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
    #[arg(long, global = true)]
    json: bool,

    /// Add an exit mode button: "name:instruction" (repeatable)
    ///
    /// Example: --exit-mode "Apply:Apply the changes" --exit-mode "Reject:Discard"
    #[arg(long = "exit-mode", value_name = "NAME:INSTRUCTION", global = true)]
    exit_modes: Vec<String>,
}

#[derive(clap::Subcommand)]
enum Command {
    /// Review git changes (CLI parity with the MCP review_diff tool)
    Diff(DiffArgs),
    /// Run as MCP server (Model Context Protocol)
    Mcp,
    /// Print version information
    Version,
}

#[derive(clap::Args)]
struct DiffArgs {
    /// A revision (diffed against its parent), or a range A..B — A...B diffs
    /// from the merge base, and an empty side means the current revision
    /// (e.g. "main.."). Git repos take revspecs, jj repos take revsets.
    #[arg(value_name = "REVISION", conflicts_with = "staged")]
    range: Option<String>,

    /// Review staged changes (index vs HEAD; git only — jj has no index)
    #[arg(long)]
    staged: bool,

    /// Display label (default: derived from the target)
    #[arg(short = 'l', long = "label")]
    label: Option<String>,

    /// Pathspecs limiting the diff, after "--" (e.g. -- src/ '*.rs');
    /// parsed as jj filesets in a jj repo
    #[arg(last = true, value_name = "PATHSPEC")]
    pathspecs: Vec<String>,
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

    // Load config
    let mut config = annot_lib::state::UserConfig::load();

    // Parse CLI exit modes and prepend as transient
    if !cli.exit_modes.is_empty() {
        let default_colors = [
            "#22c55e", "#eab308", "#ef4444", "#3b82f6", "#a855f7", "#f97316",
        ];
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

    // Resolve content: the diff subcommand renders via the VCS pipeline;
    // everything else reads file/stdin and detects a rendering mode.
    let content = match cli.command {
        Some(Command::Diff(args)) => vcs_diff_content(args),
        _ => cli_input_content(cli.file, cli.label),
    };

    let state = AppState::new(content, config);

    annot_lib::run(state, context, cli.json);
}

/// `annot diff`: render a structured target through the pipeline — the CLI twin
/// of the MCP `review_diff` tool (same targets, same cwd semantics). The tier
/// (git or jj) is chosen from the repo found at cwd, not from the arguments.
fn vcs_diff_content(args: DiffArgs) -> annot_lib::state::ContentModel {
    let target = match annot_lib::input::parse_diff_target(args.range.as_deref(), args.staged) {
        Ok(target) => target,
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
    };
    let cwd = match std::env::current_dir() {
        Ok(cwd) => cwd,
        Err(e) => {
            eprintln!("Failed to resolve working directory: {}", e);
            process::exit(1);
        }
    };
    let label = args.label.unwrap_or_else(|| target.label());
    let source = ContentSource::Cli(CliSource::Diff { label });
    match annot_lib::state::ContentModel::from_vcs(&cwd, &target, &args.pathspecs, source) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
    }
}

/// File-or-stdin input: detect the mode, read the content, pick a renderer.
fn cli_input_content(file: Option<PathBuf>, label: String) -> annot_lib::state::ContentModel {
    let (mode, warning) = match InputMode::detect(file, label) {
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

    match input.rendering_mode {
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
    }
}
