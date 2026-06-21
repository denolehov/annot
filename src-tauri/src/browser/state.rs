use std::sync::Arc;

use parking_lot::Mutex;

use crate::review::Review;

use super::lifecycle::LifecycleHandler;

/// Shared state across axum handlers. Mirrors what `tauri::Builder::manage`
/// hands out via `State<ActiveReview>` on the WebView path.
pub struct BrowserState {
    pub review: Arc<Mutex<Option<Review>>>,
    pub json: bool,
    /// SSE-disconnect-driven shutdown. Owns the oneshot sender that wakes
    /// run_browser's select! loop.
    pub lifecycle: Arc<LifecycleHandler>,
}
