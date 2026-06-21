use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::Json;
use axum::Router;
use futures_core::Stream;
use serde_json::Value;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tower_http::services::ServeDir;

use super::dispatch::{self, DispatchResult};
use super::launch;
use super::lifecycle::PING_INTERVAL;
use super::state::BrowserState;

pub async fn serve(state: Arc<BrowserState>) {
    let dist = dist_dir();
    if !dist.exists() {
        eprintln!(
            "annot --browser: frontend bundle not found at {}\n\
             Run `pnpm build:browser` first (or set ANNOT_BROWSER_DIST).",
            dist.display()
        );
        std::process::exit(1);
    }

    let app = Router::new()
        .route("/events", get(events_handler))
        .route("/invoke/:cmd", post(invoke_handler))
        .fallback_service(ServeDir::new(&dist).append_index_html_on_directories(true))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind 127.0.0.1");
    let addr = listener.local_addr().expect("local_addr");
    let url = format!("http://{addr}");

    eprintln!("annot browser mode ready: {url}");
    eprintln!("  serving from: {}", dist.display());
    launch::open_url(&url);

    axum::serve(listener, app).await.expect("axum serve");
}

async fn invoke_handler(
    State(state): State<Arc<BrowserState>>,
    Path(cmd): Path<String>,
    body: Bytes,
) -> Response {
    let args: Value = if body.is_empty() {
        Value::Object(Default::default())
    } else {
        match serde_json::from_slice(&body) {
            Ok(v) => v,
            Err(e) => {
                return (StatusCode::BAD_REQUEST, format!("bad json body: {e}"))
                    .into_response()
            }
        }
    };

    match dispatch::dispatch(&cmd, args, &state) {
        DispatchResult::Ok(v) => (StatusCode::OK, Json(v)).into_response(),
        DispatchResult::Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
        DispatchResult::NotImplemented => (
            StatusCode::NOT_IMPLEMENTED,
            format!("command '{cmd}' not wired in browser mode"),
        )
            .into_response(),
    }
}

/// Stream wrapper that decrements the lifecycle's active-connection count
/// when dropped. Dropping happens when axum drops the response body, which
/// happens on client disconnect.
struct LifecycleStream {
    rx: ReceiverStream<Result<Event, std::convert::Infallible>>,
    state: Arc<BrowserState>,
}

impl Stream for LifecycleStream {
    type Item = Result<Event, std::convert::Infallible>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.rx).poll_next(cx)
    }
}

impl Drop for LifecycleStream {
    fn drop(&mut self) {
        self.state
            .lifecycle
            .on_disconnect(Arc::clone(&self.state));
    }
}

/// SSE endpoint that the browser keeps open as a liveness signal. Drop = the
/// server schedules a grace timer (see `LifecycleHandler::on_disconnect`).
async fn events_handler(State(state): State<Arc<BrowserState>>) -> Response {
    // If we've already started shutdown, refuse new connections so the
    // browser's auto-reconnect gives up instead of bouncing the counter.
    if state.lifecycle.is_shutting_down() {
        return (StatusCode::GONE, "session ended").into_response();
    }

    state.lifecycle.on_connect();

    let (tx, rx) = mpsc::channel::<Result<Event, std::convert::Infallible>>(16);

    // Greet the client so the frontend's `connected` listener fires.
    let _ = tx.try_send(Ok(Event::default().event("connected").data("{}")));

    // Background ping task: keeps the connection visible to NAT/proxies and
    // gives the client a periodic "still alive" signal. Exits as soon as the
    // receiver is dropped (i.e., client gone).
    let tx_ping = tx.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(PING_INTERVAL);
        interval.tick().await; // skip the immediate first tick
        loop {
            interval.tick().await;
            if tx_ping
                .send(Ok(Event::default().event("ping").data("{}")))
                .await
                .is_err()
            {
                break;
            }
        }
    });

    let stream = LifecycleStream {
        rx: ReceiverStream::new(rx),
        state,
    };

    Sse::new(stream).into_response()
}

fn dist_dir() -> PathBuf {
    if let Ok(p) = std::env::var("ANNOT_BROWSER_DIST") {
        return PathBuf::from(p);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../build")
}
