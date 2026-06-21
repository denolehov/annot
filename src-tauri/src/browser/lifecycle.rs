//! SSE-disconnect-with-grace shutdown contract. Mirrors hl's
//! `internal/lifecycle/sse.go` — the browser holds an EventSource open as
//! a liveness signal; when it drops (and stays dropped for the grace
//! period), the server runs `format_output` and exits.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

use super::state::BrowserState;

const GRACE_PERIOD: Duration = Duration::from_millis(200);
const INITIAL_CONNECT_TIMEOUT: Duration = Duration::from_secs(60);
pub const PING_INTERVAL: Duration = Duration::from_secs(30);

pub struct LifecycleHandler {
    active_conns: AtomicUsize,
    has_ever_connected: AtomicBool,
    shutting_down: AtomicBool,
    /// Active grace-period timer (if a disconnect armed one). Aborted on
    /// reconnect within grace.
    grace_handle: Mutex<Option<JoinHandle<()>>>,
    /// One-shot sender into `run_browser`'s select! loop. Taken exactly
    /// once when shutdown is first triggered.
    shutdown_tx: Mutex<Option<oneshot::Sender<()>>>,
}

impl LifecycleHandler {
    pub fn new(shutdown_tx: oneshot::Sender<()>) -> Arc<Self> {
        Arc::new(Self {
            active_conns: AtomicUsize::new(0),
            has_ever_connected: AtomicBool::new(false),
            shutting_down: AtomicBool::new(false),
            grace_handle: Mutex::new(None),
            shutdown_tx: Mutex::new(Some(shutdown_tx)),
        })
    }

    pub fn is_shutting_down(&self) -> bool {
        self.shutting_down.load(Ordering::SeqCst)
    }

    /// Called when an SSE connection is established. Cancels any pending
    /// grace timer.
    pub fn on_connect(&self) {
        self.active_conns.fetch_add(1, Ordering::SeqCst);
        self.has_ever_connected.store(true, Ordering::SeqCst);
        if let Some(h) = self.grace_handle.lock().take() {
            h.abort();
        }
    }

    /// Called when an SSE connection drops. If no connections remain,
    /// arms the grace-period timer that will run `format_output` and
    /// trigger shutdown if no reconnect lands first.
    pub fn on_disconnect(self: &Arc<Self>, state: Arc<BrowserState>) {
        let prev = self.active_conns.fetch_sub(1, Ordering::SeqCst);
        if prev > 1 {
            return;
        }
        if self.shutting_down.load(Ordering::SeqCst) {
            return;
        }

        let this = Arc::clone(self);
        let handle = tokio::spawn(async move {
            tokio::time::sleep(GRACE_PERIOD).await;
            if this.shutting_down.load(Ordering::SeqCst) {
                return;
            }
            if let Err(e) = crate::commands::finish_review_browser_impl(&state.review, state.json) {
                eprintln!("annot --browser: finish_review failed in grace handler: {e}");
            }
            this.trigger_shutdown();
        });

        if let Some(old) = self.grace_handle.lock().replace(handle) {
            old.abort();
        }
    }

    /// Immediate shutdown trigger — used by the explicit `finish_review`
    /// dispatch arm. The caller is expected to have already run
    /// `finish_review_browser_impl` (to print output) before calling this.
    pub fn trigger_shutdown(&self) {
        if self.shutting_down.swap(true, Ordering::SeqCst) {
            return;
        }
        if let Some(h) = self.grace_handle.lock().take() {
            h.abort();
        }
        if let Some(tx) = self.shutdown_tx.lock().take() {
            let _ = tx.send(());
        }
    }

    /// Spawn a task that triggers shutdown if no SSE client ever connects
    /// within the timeout window. Defends against `wslview` / `xdg-open`
    /// silently failing.
    pub fn spawn_initial_connect_timeout(self: &Arc<Self>) {
        let this = Arc::clone(self);
        tokio::spawn(async move {
            tokio::time::sleep(INITIAL_CONNECT_TIMEOUT).await;
            if this.has_ever_connected.load(Ordering::SeqCst) {
                return;
            }
            eprintln!(
                "annot --browser: no client connected within {}s — exiting",
                INITIAL_CONNECT_TIMEOUT.as_secs()
            );
            this.trigger_shutdown();
        });
    }
}
