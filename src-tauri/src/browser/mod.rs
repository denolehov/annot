//! Browser-mode runtime: serves the SvelteKit frontend over localhost so the
//! user's own browser renders annot, bypassing Tauri's WebKit2GTK on Linux.
//!
//! Spike scope: CLI single-file mode only. No MCP, no diff/content modes, no
//! Mermaid/Excalidraw pop-outs, no auth tokens. See
//! `/home/nixos/.claude/plans/whimsical-humming-rabin.md`.

pub mod dispatch;
pub mod launch;
pub mod server;
pub mod state;

use std::sync::Arc;

use parking_lot::Mutex;
use tokio::sync::oneshot;

use crate::review::Review;
use crate::state::AppState;

use self::state::BrowserState;

/// Entry point for `annot --browser <file>`. Builds the Review, stands up the
/// axum server, races the server future against a shutdown signal sent by the
/// finish_with_pending handler. On shutdown, exits the process.
pub fn run_browser(state: AppState, json_output: bool) {
    let review = Review::cli(state.content, state.config, "main".to_string());
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    let browser_state = Arc::new(BrowserState {
        review: Arc::new(Mutex::new(Some(review))),
        json: json_output,
        shutdown_tx: Mutex::new(Some(shutdown_tx)),
    });

    let rt = tokio::runtime::Runtime::new().expect("create tokio runtime");
    rt.block_on(async move {
        tokio::select! {
            // Server exits on its own (rare — usually the user closes the tab)
            _ = server::serve(browser_state) => {}
            // finish_with_pending fired: output already flushed inside the
            // handler. Drop the runtime and exit cleanly.
            _ = shutdown_rx => {}
        }
    });

    std::process::exit(0);
}
