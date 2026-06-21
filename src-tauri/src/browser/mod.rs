//! Browser-mode runtime: serves the SvelteKit frontend over localhost so the
//! user's own browser renders annot, bypassing Tauri's WebKit2GTK on Linux.
//!
//! Shutdown contract: SSE-disconnect-with-grace (see `lifecycle.rs`). The
//! browser holds an EventSource open; when it drops and stays dropped for
//! 200ms, the server runs `format_output` and exits.
//!
//! Scope: CLI single-file mode only. MCP, diff/content modes, Mermaid/
//! Excalidraw pop-outs, auth tokens, and asset embedding are v1 followups.

pub mod dispatch;
pub mod launch;
pub mod lifecycle;
pub mod server;
pub mod state;

use std::sync::Arc;

use parking_lot::Mutex;
use tokio::sync::oneshot;

use crate::review::Review;
use crate::state::AppState;

use self::lifecycle::LifecycleHandler;
use self::state::BrowserState;

/// Entry point for `annot --browser <file>`. Builds the Review, stands up
/// the axum server, races it against the shutdown signal from the
/// lifecycle handler. On signal, exits the process.
pub fn run_browser(state: AppState, json_output: bool) {
    let review = Review::cli(state.content, state.config, "main".to_string());
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let lifecycle = LifecycleHandler::new(shutdown_tx);

    let browser_state = Arc::new(BrowserState {
        review: Arc::new(Mutex::new(Some(review))),
        json: json_output,
        lifecycle: Arc::clone(&lifecycle),
    });

    let rt = tokio::runtime::Runtime::new().expect("create tokio runtime");
    rt.block_on(async move {
        // 60s defence: if no browser ever connects, exit anyway.
        lifecycle.spawn_initial_connect_timeout();

        tokio::select! {
            _ = server::serve(browser_state) => {}
            _ = shutdown_rx => {}
        }
    });

    std::process::exit(0);
}
