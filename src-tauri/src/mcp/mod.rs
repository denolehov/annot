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

use crate::config;
use crate::input::{ContentSource, DiffSource, McpSource};
use crate::output::FormatResult;
use crate::state::AppState;
use tools::{ReviewContentInput, ReviewDiffInput, ReviewFileInput, SessionImage, SessionOutput};

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

    #[tool(description = "Opens a file in the annotation interface. Blocks until the window closes, then returns all annotations as structured text.")]
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

    #[tool(description = "Opens ephemeral (agent-generated) content for annotation. Use for reviewing plans, drafts, or other generated text. Blocks until the window closes.")]
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

    #[tool(description = "Opens a unified diff in hl for annotation. Supports git-aware generation (preferred) or raw diff content. Blocks until browser closes.")]
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
            instructions: Some("Annotation tool for human-in-the-loop AI workflows. Tools: review_file (opens a file for annotation), review_content (opens ephemeral content for annotation), review_diff (opens a unified diff for annotation).".into()),
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
    let tags = config::load_tags();
    let mut exit_modes = config::load_exit_modes();

    // Prepend transient exit modes
    if let Some(inputs) = params.exit_modes {
        let transient: Vec<_> = inputs
            .into_iter()
            .enumerate()
            .map(|(i, m)| m.to_exit_mode(i))
            .collect();
        exit_modes.splice(0..0, transient);
    }

    // Create state using from_diff
    let state = AppState::from_diff(&diff_text, tags, exit_modes, content_source)
        .map_err(|e| format!("Invalid diff: {}", e))?;

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
    let tags = config::load_tags();
    let mut exit_modes = config::load_exit_modes();

    // Prepend transient exit modes from MCP input
    if let Some(inputs) = exit_modes_input {
        let transient_modes: Vec<_> = inputs
            .into_iter()
            .enumerate()
            .map(|(i, m)| m.to_exit_mode(i))
            .collect();
        exit_modes.splice(0..0, transient_modes);
    }

    // Create state (check for markdown by path hint)
    let path_hint = content_source.path_hint().unwrap_or("");
    let state = if crate::markdown::is_markdown(path_hint) {
        AppState::from_markdown(&content, tags, exit_modes, content_source)
    } else {
        AppState::from_file(&content, tags, exit_modes, content_source)
    };

    run_session_with_state(app_handle, state)
}

/// Run a review session with a pre-built AppState.
fn run_session_with_state(
    app_handle: &AppHandle,
    state: AppState,
) -> Result<SessionOutput, String> {
    // Show dock icon while window is open
    #[cfg(target_os = "macos")]
    {
        use tauri::ActivationPolicy;
        let _ = app_handle.set_activation_policy(ActivationPolicy::Regular);
    }

    // Create channel for receiving result
    let (tx, rx) = mpsc::channel::<FormatResult>();

    // Store state and sender
    {
        use crate::ResultSender;

        let managed_state = app_handle.state::<parking_lot::Mutex<AppState>>();
        let mut guard = managed_state.lock();
        *guard = state;

        // Store the sender for finish_session to use
        let sender_state = app_handle.state::<ResultSender>();
        let mut sender_guard = sender_state.lock();
        *sender_guard = Some(tx);
    }

    // Create window
    let window_label = format!(
        "session-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    );

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
