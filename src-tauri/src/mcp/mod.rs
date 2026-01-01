pub mod tools;

use std::fs;
use std::panic::AssertUnwindSafe;
use std::path::Path;
use std::path::PathBuf;
use std::sync::mpsc;

use rmcp::handler::server::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, Content, ServerInfo, ServerCapabilities, Implementation};
use rmcp::{tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler, ServiceExt};
use tauri::{AppHandle, Manager, WebviewWindowBuilder};

use crate::input::{ContentSource, DiffSource, McpSource};
use crate::output::FormatResult;
use crate::review::{ActiveReview, Review};
use crate::state::AppState;
use crate::SessionLock;
use tools::{
    GetBookmarkInput, ListBookmarksInput, ReviewContentInput, ReviewDiffInput, ReviewFileInput,
    SessionImage, SessionOutput,
};

/// Instructions for AI agents using the MCP server.
const MCP_INSTRUCTIONS: &str = r#"Human-in-the-loop annotation for AI workflows. Pull the human into the loop to provide located, specific feedback on content.

Tools open a native window where users annotate specific lines with tags and comments. Tools block until the session closes, then return structured output designed for LLM parsing.

## Portals (Code Embeds)

When reviewing markdown content, you can embed live code snippets using portal links:

```markdown
The validation logic in [validate_portal](src/portal.rs#L112-L155) handles security checks.
```

Portal syntax: `[label](path#L<start>-L<end>)` where:
- `label` is displayed inline (use descriptive names like "auth middleware" not "code")
- `path` is relative to the current working directory
- `#L<start>-L<end>` specifies the line range (1-indexed)

The original line stays visible with the portal content expanded below it. Multiple portals on one line each expand separately.

Keep portals small for optimal annotation experience - prefer multiple focused portals over one sprawling embed.

## Highlights

Use `==highlighted text==` to draw the human's attention to specific phrases. Highlights render visually distinct.

## Diagrams

Mermaid code blocks render as interactive diagrams:

```mermaid
graph LR
    A[Input] --> B[Process] --> C[Output]
```"#;

/// MCP server that exposes annotation tools.
#[derive(Clone)]
pub struct AnnotServer {
    app_handle: AppHandle,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl AnnotServer {
    pub fn new(app_handle: AppHandle) -> Self {
        Self {
            app_handle,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Opens a file for human review and annotation. Blocks until the window closes. The user can select line ranges to annotate, apply semantic tags (like [# SECURITY], [# TODO]), and add freeform comments. Returns line-anchored annotations with tags for systematic processing.")]
    async fn review_file(
        &self,
        params: Parameters<ReviewFileInput>,
    ) -> Result<CallToolResult, McpError> {
        let app_handle = self.app_handle.clone();
        let input = params.0;

        let output = tokio::task::spawn_blocking(move || {
            run_file_session(&app_handle, input)
        })
        .await
        .map_err(|e| McpError::internal_error(format!("Task join error: {}", e), None))?
        .map_err(|e| McpError::internal_error(e, None))?;

        Ok(build_mcp_response(output))
    }

    #[tool(description = "Opens agent-generated content (plans, drafts, analysis) for human review. Blocks until the window closes. Best for content you've generated that needs human steering before proceeding. Supports portal links to embed live code (`[label](path#L1-L20)`), highlights (`==important==`), and Mermaid diagrams. Returns line-anchored annotations for iterative refinement.")]
    async fn review_content(
        &self,
        params: Parameters<ReviewContentInput>,
    ) -> Result<CallToolResult, McpError> {
        let app_handle = self.app_handle.clone();
        let input = params.0;

        let output = tokio::task::spawn_blocking(move || {
            run_content_session(&app_handle, input)
        })
        .await
        .map_err(|e| McpError::internal_error(format!("Task join error: {}", e), None))?
        .map_err(|e| McpError::internal_error(e, None))?;

        Ok(build_mcp_response(output))
    }

    #[tool(description = "Opens a diff for human review. Blocks until the window closes. Supports git_diff_args (e.g. [\"--staged\"], [\"main...HEAD\"]) or raw diff_content. Returns annotations anchored to diff lines for targeted feedback on changes.")]
    async fn review_diff(
        &self,
        params: Parameters<ReviewDiffInput>,
    ) -> Result<CallToolResult, McpError> {
        let app_handle = self.app_handle.clone();
        let input = params.0;

        let output = tokio::task::spawn_blocking(move || {
            run_diff_session(&app_handle, input)
        })
        .await
        .map_err(|e| McpError::internal_error(format!("Task join error: {}", e), None))?
        .map_err(|e| McpError::internal_error(e, None))?;

        Ok(build_mcp_response(output))
    }

    #[tool(description = "Retrieves a single bookmark by ID or ID prefix. Returns the full bookmark including its snapshot content. If the prefix matches multiple bookmarks, returns an error with candidates.")]
    async fn get_bookmark(
        &self,
        params: Parameters<GetBookmarkInput>,
    ) -> Result<CallToolResult, McpError> {
        let input = params.0;

        let result = tokio::task::spawn_blocking(move || {
            let config = crate::state::UserConfig::load();
            let bookmarks = config.bookmarks();

            // Find all bookmarks matching the prefix
            let matches: Vec<_> = bookmarks
                .iter()
                .filter(|b| b.id.starts_with(&input.id))
                .collect();

            match matches.len() {
                0 => Err(format!("No bookmark found with ID prefix '{}'", input.id)),
                1 => {
                    let bookmark = matches[0];
                    Ok(format_bookmark_full(bookmark))
                }
                _ => {
                    // Ambiguous - list candidates
                    let candidates: Vec<String> = matches
                        .iter()
                        .map(|b| format!("  {} — {}", &b.id[..6], b.display_label()))
                        .collect();
                    Err(format!(
                        "Ambiguous ID prefix '{}'. Candidates:\n{}",
                        input.id,
                        candidates.join("\n")
                    ))
                }
            }
        })
        .await
        .map_err(|e| McpError::internal_error(format!("Task join error: {}", e), None))?;

        match result {
            Ok(text) => Ok(CallToolResult::success(vec![Content::text(text)])),
            Err(e) => Err(McpError::invalid_params(e, None)),
        }
    }

    #[tool(description = "List all bookmarks, optionally filtered by search query, project path, or limited to a maximum count. Returns a summary list with IDs, labels, sources, and creation dates.")]
    async fn list_bookmarks(
        &self,
        params: Parameters<ListBookmarksInput>,
    ) -> Result<CallToolResult, McpError> {
        let input = params.0;

        let result = tokio::task::spawn_blocking(move || {
            let config = crate::state::UserConfig::load();
            let bookmarks = config.bookmarks();

            // Apply filters
            let mut filtered: Vec<_> = bookmarks
                .iter()
                .filter(|b| {
                    // Project filter
                    if let Some(ref project) = input.project {
                        if let Some(ref path) = b.project_path {
                            if !path.to_string_lossy().contains(project) {
                                return false;
                            }
                        } else {
                            return false;
                        }
                    }

                    // Search filter (label, selected text, context)
                    if let Some(ref query) = input.search {
                        let query_lower = query.to_lowercase();
                        let label_match = b.display_label().to_lowercase().contains(&query_lower);
                        let content_match = b.snapshot.content().to_lowercase().contains(&query_lower);
                        if !label_match && !content_match {
                            return false;
                        }
                    }

                    true
                })
                .collect();

            // Sort by created_at descending (newest first)
            filtered.sort_by(|a, b| b.created_at.cmp(&a.created_at));

            // Apply limit
            if let Some(limit) = input.limit {
                filtered.truncate(limit);
            }

            if filtered.is_empty() {
                "No bookmarks found.".to_string()
            } else {
                format_bookmark_list(&filtered)
            }
        })
        .await
        .map_err(|e| McpError::internal_error(format!("Task join error: {}", e), None))?;

        Ok(CallToolResult::success(vec![Content::text(result)]))
    }
}

#[tool_handler]
impl ServerHandler for AnnotServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: Default::default(),
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(MCP_INSTRUCTIONS.into()),
        }
    }
}

/// Run a file review session.
fn run_file_session(app_handle: &AppHandle, params: ReviewFileInput) -> Result<SessionOutput, String> {
    // Read file content
    let path = Path::new(&params.file_path);
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read file '{}': {}", params.file_path, e))?;

    let content_source = ContentSource::Mcp(McpSource::File {
        path: PathBuf::from(&params.file_path),
    });

    run_session(app_handle, content, params.exit_modes, content_source)
}

/// Run a content review session.
fn run_content_session(
    app_handle: &AppHandle,
    params: ReviewContentInput,
) -> Result<SessionOutput, String> {
    let content_source = ContentSource::Mcp(McpSource::Content {
        label: params.label,
    });

    run_session(app_handle, params.content, params.exit_modes, content_source)
}

/// Run a diff review session.
fn run_diff_session(
    app_handle: &AppHandle,
    params: ReviewDiffInput,
) -> Result<SessionOutput, String> {
    use std::process::Command;

    // Get diff content and derive label + source based on which input was provided
    let (diff_text, derived_label, diff_source) = match (&params.git_diff_args, &params.diff_content) {
        (Some(args), None) => {
            // Git diff mode
            let output = Command::new("git")
                .arg("diff")
                .args(args)
                .output()
                .map_err(|e| format!("Failed to run git: {}", e))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!("git diff failed: {}", stderr));
            }

            let diff = String::from_utf8_lossy(&output.stdout).to_string();
            let label = args
                .first()
                .map(|s| s.trim_start_matches('-').to_string())
                .unwrap_or_else(|| "diff".to_string());
            let source = DiffSource::Git { args: args.clone() };
            (diff, label, source)
        }
        (None, Some(content)) => {
            // Raw diff mode
            (content.clone(), "diff".to_string(), DiffSource::Raw)
        }
        (Some(_), Some(_)) => {
            return Err("Provide either git_diff_args or diff_content, not both".to_string());
        }
        (None, None) => {
            return Err("Provide either git_diff_args or diff_content".to_string());
        }
    };

    let label = params.label.clone().unwrap_or(derived_label);
    let content_source = ContentSource::Mcp(McpSource::Diff {
        label: Some(label),
        source: diff_source,
    });

    // Load config
    let mut config = crate::state::UserConfig::load();

    // Prepend transient exit modes
    if let Some(inputs) = params.exit_modes {
        let transient: Vec<_> = inputs
            .into_iter()
            .enumerate()
            .map(|(i, m)| m.to_exit_mode(i))
            .collect();
        config.prepend_transient_modes(transient);
    }

    // Create state using from_diff
    let content = crate::state::ContentModel::from_diff(&diff_text, content_source)
        .map_err(|e| format!("Invalid diff: {}", e))?;
    let state = AppState::new(content, config);

    run_session_with_state(app_handle, state)
}

/// Run a review session with the given content (for file/content modes).
fn run_session(
    app_handle: &AppHandle,
    content: String,
    exit_modes_input: Option<Vec<tools::ExitModeInput>>,
    content_source: ContentSource,
) -> Result<SessionOutput, String> {
    // Load config
    let mut config = crate::state::UserConfig::load();

    // Prepend transient exit modes from MCP input
    if let Some(inputs) = exit_modes_input {
        let transient_modes: Vec<_> = inputs
            .into_iter()
            .enumerate()
            .map(|(i, m)| m.to_exit_mode(i))
            .collect();
        config.prepend_transient_modes(transient_modes);
    }

    // Create content model (check for markdown by path hint)
    let path_hint = content_source.path_hint().unwrap_or("");
    let content_model = if crate::markdown::is_markdown(path_hint) {
        crate::state::ContentModel::from_markdown(&content, content_source)
    } else {
        crate::state::ContentModel::from_file(&content, content_source)
    };

    let state = AppState::new(content_model, config);
    run_session_with_state(app_handle, state)
}

/// Run a review session with a pre-built AppState.
fn run_session_with_state(
    app_handle: &AppHandle,
    state: AppState,
) -> Result<SessionOutput, String> {
    // Serialize MCP sessions: only one review window at a time.
    // This lock is held for the entire session (window open → close → result returned).
    // Subsequent MCP calls block here until the current session completes.
    let session_lock = app_handle.state::<SessionLock>();
    let _session_guard = session_lock.lock();

    // Show dock icon while window is open
    #[cfg(target_os = "macos")]
    {
        use tauri::ActivationPolicy;
        let _ = app_handle.set_activation_policy(ActivationPolicy::Regular);
    }

    // Create channel for receiving result
    let (tx, rx) = mpsc::channel::<FormatResult>();

    // Generate window label first (needed for Review creation)
    let window_label = format!(
        "session-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    );

    // Create Review and store it (auto-detects file vs diff mode)
    {
        let review = Review::mcp(state.content, state.config, window_label.clone(), tx);
        let slot = app_handle.state::<ActiveReview>();
        *slot.lock() = Some(review);
    }

    let mut builder = WebviewWindowBuilder::new(app_handle, &window_label, tauri::WebviewUrl::App("index.html".into()))
        .title("annot")
        .inner_size(1000.0, 700.0)
        .visible(false) // Will be shown after content loads
        .title_bar_style(tauri::TitleBarStyle::Overlay)
        .hidden_title(true);

    #[cfg(target_os = "macos")]
    {
        builder = builder.traffic_light_position(tauri::LogicalPosition::new(12.0, 22.0));
    }

    let _window = builder
        .build()
        .map_err(|e| format!("Failed to create window: {}", e))?;

    // Block until result received
    let result = rx.recv().map_err(|e| format!("Failed to receive result: {}", e))?;

    // Hide dock icon after window closes
    #[cfg(target_os = "macos")]
    {
        use tauri::ActivationPolicy;
        let _ = app_handle.set_activation_policy(ActivationPolicy::Accessory);
    }

    Ok(SessionOutput {
        text: result.text,
        images: result.images.into_iter().map(|img| SessionImage {
            figure: img.figure,
            data: img.data,
            mime_type: img.mime_type,
        }).collect(),
    })
}

// ════════════════════════════════════════════════════════════════════════════
// BOOKMARK FORMATTING
// ════════════════════════════════════════════════════════════════════════════

/// Format a single bookmark with full details for get_bookmark output.
fn format_bookmark_full(bookmark: &crate::state::Bookmark) -> String {
    use crate::state::BookmarkSnapshot;

    let source_type = match &bookmark.snapshot {
        BookmarkSnapshot::Session { source_type, .. }
        | BookmarkSnapshot::Selection { source_type, .. } => match source_type {
            crate::state::SessionType::File => "file",
            crate::state::SessionType::Diff => "diff",
            crate::state::SessionType::Content => "content",
        },
    };

    let project = bookmark
        .project_path
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "(none)".to_string());

    let mut lines = vec![
        format!("Bookmark: {}", bookmark.id),
        format!("Label: {}", bookmark.display_label()),
        format!("Source: {} ({})", bookmark.snapshot.source_title(), source_type),
        format!("Project: {}", project),
        format!("Created: {}", bookmark.created_at.format("%Y-%m-%d %H:%M:%S UTC")),
        String::new(),
    ];

    // Add selected text for selection bookmarks
    if let BookmarkSnapshot::Selection { selected_text, .. } = &bookmark.snapshot {
        lines.push("─── Selected Text ──────────────────────────────────────".to_string());
        lines.push(selected_text.clone());
        lines.push("─────────────────────────────────────────────────────────".to_string());
        lines.push(String::new());
    }

    lines.push("─── Snapshot ───────────────────────────────────────────".to_string());
    lines.push(bookmark.snapshot.content().to_string());
    lines.push("─────────────────────────────────────────────────────────".to_string());

    lines.join("\n")
}

/// Format a list of bookmarks as a summary table for list_bookmarks output.
fn format_bookmark_list(bookmarks: &[&crate::state::Bookmark]) -> String {
    let mut lines = Vec::new();

    lines.push(format!(
        "{:<12} {:<40} {:<20} {}",
        "ID", "LABEL", "SOURCE", "PROJECT"
    ));
    lines.push("─".repeat(90));

    for bookmark in bookmarks {
        let label = bookmark.display_label();
        let label_display = if label.len() > 38 {
            format!("{}…", &label[..37])
        } else {
            label
        };

        let source = bookmark.snapshot.source_title();
        let source_display = if source.len() > 18 {
            format!("{}…", &source[..17])
        } else {
            source.to_string()
        };

        let project = bookmark
            .project_path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "—".to_string());

        lines.push(format!(
            "{:<12} {:<40} {:<20} {}",
            &bookmark.id[..12.min(bookmark.id.len())],
            label_display,
            source_display,
            project
        ));
    }

    lines.join("\n")
}

// ════════════════════════════════════════════════════════════════════════════
// RESPONSE BUILDERS
// ════════════════════════════════════════════════════════════════════════════

/// Build MCP response from session output.
fn build_mcp_response(output: SessionOutput) -> CallToolResult {
    let text = if output.text.is_empty() {
        "=== REVIEW SESSION COMPLETE ===\nBrowser session closed.\nUser completed review without adding annotations.\n".to_string()
    } else {
        format!(
            "=== REVIEW SESSION COMPLETE ===\nBrowser session closed. All annotations are shown below.\n\n{}",
            output.text
        )
    };

    let mut contents = vec![Content::text(text)];

    // Add images as separate content items (data is already base64-encoded)
    for img in output.images {
        contents.push(Content::image(img.data, img.mime_type));
    }

    CallToolResult::success(contents)
}

/// Run the MCP server on stdio.
pub fn run_mcp_server(app_handle: AppHandle) {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");

    rt.block_on(async {
        let server = AnnotServer::new(app_handle);
        let service = server.serve(rmcp::transport::stdio()).await;
        match service {
            Ok(s) => {
                if let Err(e) = s.waiting().await {
                    eprintln!("MCP server error: {}", e);
                }
            }
            Err(e) => {
                eprintln!("Failed to start MCP server: {}", e);
            }
        }
    });
}

/// Run MCP server in a background thread with panic handling.
pub fn spawn_mcp_thread(app_handle: AppHandle) {
    std::thread::spawn(move || {
        if let Err(e) = std::panic::catch_unwind(AssertUnwindSafe(|| {
            run_mcp_server(app_handle);
        })) {
            eprintln!("MCP server panicked: {:?}", e);
            std::process::exit(1);
        }
    });
}
