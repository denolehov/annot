pub mod tools;

use std::fs;
use std::panic::AssertUnwindSafe;
use std::path::Path;
use std::sync::mpsc;

use rmcp::handler::server::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, Content, ServerInfo, ServerCapabilities, Implementation};
use rmcp::{tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler, ServiceExt};
use tauri::{AppHandle, Manager, WebviewWindowBuilder};

use crate::config;
use crate::state::AppState;
use tools::{ReviewContentInput, ReviewFileInput, SessionOutput};

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
            instructions: Some("Annotation tool for human-in-the-loop AI workflows. Tools: review_file (opens a file for annotation), review_content (opens ephemeral content for annotation).".into()),
        }
    }
}

/// Run a file review session.
fn run_file_session(app_handle: &AppHandle, params: ReviewFileInput) -> Result<SessionOutput, String> {
    // Read file content
    let path = Path::new(&params.file_path);
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read file '{}': {}", params.file_path, e))?;

    let label = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| params.file_path.clone());

    run_session(app_handle, label, content, &params.file_path, params.exit_modes)
}

/// Run a content review session.
fn run_content_session(
    app_handle: &AppHandle,
    params: ReviewContentInput,
) -> Result<SessionOutput, String> {
    run_session(
        app_handle,
        params.label.clone(),
        params.content,
        &params.label,
        params.exit_modes,
    )
}

/// Run a review session with the given content.
fn run_session(
    app_handle: &AppHandle,
    label: String,
    content: String,
    path_hint: &str,
    exit_modes_input: Option<Vec<tools::ExitModeInput>>,
) -> Result<SessionOutput, String> {
    // Create channel for receiving result
    let (tx, rx) = mpsc::channel::<String>();

    // Load config
    let tags = config::load_tags();
    let mut exit_modes = config::load_exit_modes();

    // Prepend ephemeral exit modes from MCP input
    if let Some(inputs) = exit_modes_input {
        let ephemeral: Vec<_> = inputs
            .into_iter()
            .enumerate()
            .map(|(i, m)| m.to_exit_mode(i))
            .collect();
        // Insert at beginning
        exit_modes.splice(0..0, ephemeral);
    }

    // Create state
    let state = AppState::from_file(label, &content, path_hint, tags, exit_modes);

    // Store state and sender
    {
        use crate::ResultSender;

        let managed_state = app_handle.state::<std::sync::Mutex<AppState>>();
        let mut guard = managed_state.lock().unwrap();
        *guard = state;

        // Store the sender for finish_session to use
        let sender_state = app_handle.state::<ResultSender>();
        let mut sender_guard = sender_state.lock().unwrap();
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
    let output_text = rx.recv().map_err(|e| format!("Failed to receive result: {}", e))?;

    Ok(SessionOutput { text: output_text })
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

    CallToolResult::success(vec![Content::text(text)])
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
