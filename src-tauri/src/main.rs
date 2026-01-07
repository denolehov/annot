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
}

#[derive(clap::Subcommand)]
enum Command {
    /// Run as MCP server (Model Context Protocol)
    Mcp,
    /// Manage bookmarks
    #[command(subcommand)]
    Bookmark(BookmarksCommand),
    /// Print version information
    Version,
}

#[derive(clap::Subcommand)]
enum BookmarksCommand {
    /// List all bookmarks
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Sort order by creation date: "asc" (oldest first) or "desc" (newest first, default)
        #[arg(long, default_value = "desc")]
        sort: String,
    },
    /// Show a bookmark's full snapshot
    Show {
        /// Bookmark ID or prefix
        id: String,
    },
    /// Delete a bookmark
    Delete {
        /// Bookmark ID or prefix
        id: String,
    },
    /// Open a bookmark for annotation
    Open {
        /// Bookmark ID or prefix
        id: String,
    },
}

fn main() {
    // Suppress macOS system logs (XPC, CoreAnalytics, etc.) in release builds
    #[cfg(all(target_os = "macos", not(debug_assertions)))]
    std::env::set_var("OS_ACTIVITY_MODE", "disable");

    let cli = Cli::parse();

    // Handle bookmark subcommands that don't need Tauri (list/show/delete)
    if let Some(Command::Bookmark(cmd)) = &cli.command {
        if !matches!(cmd, BookmarksCommand::Open { .. }) {
            handle_bookmarks_command(cmd);
            return;
        }
    }

    // Handle version subcommand (doesn't need Tauri)
    if let Some(Command::Version) = cli.command {
        println!("annot {}", env!("CARGO_PKG_VERSION"));
        return;
    }

    // Generate context once (avoids duplicate symbol errors)
    let context = tauri::generate_context!();

    // Handle bookmark open (needs Tauri window)
    if let Some(Command::Bookmark(BookmarksCommand::Open { id })) = &cli.command {
        handle_bookmark_open(id, context);
        return;
    }

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

// ════════════════════════════════════════════════════════════════════════════
// BOOKMARK CLI HANDLERS
// ════════════════════════════════════════════════════════════════════════════

fn handle_bookmarks_command(cmd: &BookmarksCommand) {
    use annot_lib::state::UserConfig;

    let mut config = UserConfig::load();

    match cmd {
        BookmarksCommand::List { json, sort } => {
            let bookmarks = config.bookmarks();

            if bookmarks.is_empty() {
                if *json {
                    println!("[]");
                } else {
                    println!("*No bookmarks.*");
                }
                return;
            }

            // Sort bookmarks by creation date
            let mut sorted: Vec<_> = bookmarks.iter().collect();
            let descending = sort == "desc";
            sorted.sort_by(|a, b| {
                if descending {
                    b.created_at.cmp(&a.created_at)
                } else {
                    a.created_at.cmp(&b.created_at)
                }
            });

            if *json {
                // Collect sorted refs into owned values for JSON serialization
                let sorted_owned: Vec<_> = sorted.into_iter().cloned().collect();
                println!("{}", serde_json::to_string_pretty(&sorted_owned).unwrap());
            } else {
                // Markdown table
                println!("| ID | Label | Source | Project |");
                println!("|-----|-------|--------|---------|");

                for bookmark in sorted {
                    let label = bookmark.display_label().replace('|', "\\|");
                    let label_display = if label.len() > 38 {
                        format!("{}…", &label[..37])
                    } else {
                        label
                    };

                    let source = bookmark.snapshot.source_title().replace('|', "\\|");
                    let source_display = if source.len() > 18 {
                        format!("{}…", &source[..17])
                    } else {
                        source
                    };

                    let project = bookmark
                        .project_path
                        .as_ref()
                        .and_then(|p| p.file_name())
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "—".to_string());

                    println!(
                        "| {} | {} | {} | {} |",
                        &bookmark.id[..12.min(bookmark.id.len())],
                        label_display,
                        source_display,
                        project
                    );
                }
            }
        }

        BookmarksCommand::Show { id } => {
            match find_bookmark(&config, id) {
                Ok(bookmark) => print_bookmark_markdown(bookmark),
                Err(e) => {
                    eprintln!("{}", e);
                    process::exit(1);
                }
            }
        }

        BookmarksCommand::Delete { id } => {
            // First resolve the full ID
            let full_id = match find_bookmark(&config, id) {
                Ok(bookmark) => bookmark.id.clone(),
                Err(e) => {
                    eprintln!("{}", e);
                    process::exit(1);
                }
            };

            if config.delete_bookmark(&full_id) {
                println!("Deleted bookmark {}", full_id);
            } else {
                eprintln!("Failed to delete bookmark {}", full_id);
                process::exit(1);
            }
        }

        // Handled separately in main() since it needs Tauri context
        BookmarksCommand::Open { .. } => unreachable!(),
    }
}

fn find_bookmark<'a>(
    config: &'a annot_lib::state::UserConfig,
    id_prefix: &str,
) -> Result<&'a annot_lib::state::Bookmark, String> {
    let bookmarks = config.bookmarks();
    let matches: Vec<_> = bookmarks
        .iter()
        .filter(|b| b.id.starts_with(id_prefix))
        .collect();

    match matches.len() {
        0 => Err(format!("No bookmark found with ID prefix '{}'", id_prefix)),
        1 => Ok(matches[0]),
        _ => {
            let candidates: Vec<String> = matches
                .iter()
                .map(|b| format!("  {} — {}", &b.id[..6], b.display_label()))
                .collect();
            Err(format!(
                "Ambiguous ID prefix '{}'. Candidates:\n{}",
                id_prefix,
                candidates.join("\n")
            ))
        }
    }
}

fn print_bookmark_markdown(bookmark: &annot_lib::state::Bookmark) {
    use annot_lib::state::BookmarkSnapshot;

    let project = bookmark
        .project_path
        .as_ref()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "(none)".to_string());

    println!("# {}", bookmark.display_label());
    println!();
    println!("**Source**: {}", bookmark.snapshot.source_title());
    println!("**Project**: {}", project);
    println!(
        "**Created**: {}",
        bookmark.created_at.format("%B %d, %Y")
    );
    println!();
    println!("---");
    println!();

    // For selection bookmarks, show selected text
    if let BookmarkSnapshot::Selection { selected_text, .. } = &bookmark.snapshot {
        println!("```");
        println!("{}", selected_text);
        println!("```");
    } else {
        // For session bookmarks, show context
        println!("{}", bookmark.snapshot.content());
    }
}

fn handle_bookmark_open(id: &str, context: tauri::Context) {
    use annot_lib::input::{CliSource, ContentSource};
    use annot_lib::state::{BookmarkSnapshot, ContentModel, SessionType, UserConfig};

    let config = UserConfig::load();

    // Find bookmark with prefix matching (reuse existing helper)
    let bookmark = match find_bookmark(&config, id) {
        Ok(b) => b.clone(),
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
    };

    // Extract content and type from snapshot
    let (source_type, source_title, content_str) = match &bookmark.snapshot {
        BookmarkSnapshot::Session {
            source_type,
            source_title,
            context,
        } => (source_type, source_title.clone(), context.clone()),
        BookmarkSnapshot::Selection {
            source_type,
            source_title,
            context,
            ..
        } => {
            // Show full context (per requirements)
            (source_type, source_title.clone(), context.clone())
        }
    };

    // Create ContentSource with original label (for syntax highlighting)
    let content_source = ContentSource::Cli(CliSource::Stdin {
        label: source_title.clone(),
    });

    // Build ContentModel based on source_type
    let content = match source_type {
        SessionType::Diff => match ContentModel::from_diff(&content_str, content_source) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Error parsing bookmark diff: {}", e);
                process::exit(1);
            }
        },
        SessionType::Content | SessionType::File => {
            // Check if markdown by label extension
            if source_title.ends_with(".md") {
                ContentModel::from_markdown(&content_str, content_source)
            } else {
                ContentModel::from_file(&content_str, content_source)
            }
        }
    };

    let state = AppState::new(content, config);
    annot_lib::run(state, context);
}
