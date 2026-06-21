use std::path::PathBuf;
use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::Json;
use axum::Router;
use serde::Deserialize;
use serde_json::Value;
use tower_http::services::ServeDir;

use crate::commands;
use crate::state::ContentNode;

use super::dispatch::{self, DispatchResult};
use super::launch;
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
        .route("/invoke/finish_with_pending", post(finish_with_pending_handler))
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
            format!("command '{cmd}' not wired in browser spike"),
        )
            .into_response(),
    }
}

fn dist_dir() -> PathBuf {
    if let Ok(p) = std::env::var("ANNOT_BROWSER_DIST") {
        return PathBuf::from(p);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../build")
}

#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "lowercase")]
enum PendingOp {
    #[serde(rename_all = "camelCase")]
    Upsert {
        path: String,
        start_line: u32,
        end_line: u32,
        content: Vec<ContentNode>,
    },
    #[serde(rename_all = "camelCase")]
    Delete {
        path: String,
        start_line: u32,
        end_line: u32,
    },
}

#[derive(Deserialize)]
struct FinishPayload {
    #[serde(default)]
    pending: Vec<PendingOp>,
}

/// Browser-mode shutdown endpoint. Receives the last pending-annotation batch
/// from a sendBeacon during pagehide, applies it, runs format_output, prints
/// to stdout, and signals the shutdown channel so run_browser exits.
async fn finish_with_pending_handler(
    State(state): State<Arc<BrowserState>>,
    body: Bytes,
) -> Response {
    let payload: FinishPayload = if body.is_empty() {
        FinishPayload { pending: vec![] }
    } else {
        match serde_json::from_slice(&body) {
            Ok(p) => p,
            Err(e) => {
                return (StatusCode::BAD_REQUEST, format!("bad payload: {e}")).into_response()
            }
        }
    };

    // Apply pending ops (swallow per-op errors so finish still runs).
    for op in payload.pending {
        let res = match op {
            PendingOp::Upsert {
                path,
                start_line,
                end_line,
                content,
            } => commands::upsert_annotation_impl(
                &state.review,
                path,
                start_line,
                end_line,
                content,
            ),
            PendingOp::Delete {
                path,
                start_line,
                end_line,
            } => commands::delete_annotation_impl(&state.review, path, start_line, end_line),
        };
        if let Err(e) = res {
            eprintln!("annot --browser: pending op failed: {e}");
        }
    }

    if let Err(e) = commands::finish_review_browser_impl(&state.review, state.json) {
        eprintln!("annot --browser: finish failed: {e}");
    }

    // Wake the run_browser select! loop.
    if let Some(tx) = state.shutdown_tx.lock().take() {
        let _ = tx.send(());
    }

    StatusCode::OK.into_response()
}
