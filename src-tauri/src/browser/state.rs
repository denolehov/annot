use std::sync::Arc;

use parking_lot::Mutex;
use tokio::sync::oneshot;

use crate::review::Review;

/// Shared state across axum handlers. Mirrors what `tauri::Builder::manage`
/// hands out via `State<ActiveReview>` on the WebView path.
pub struct BrowserState {
    pub review: Arc<Mutex<Option<Review>>>,
    pub json: bool,
    /// Set by the finish_with_pending handler to wake the run_browser
    /// select! loop and trigger process exit. Held in a std Mutex (not async)
    /// because we only ever take it once.
    pub shutdown_tx: Mutex<Option<oneshot::Sender<()>>>,
}
