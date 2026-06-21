//! Flat command dispatcher for browser mode. Mirrors `tauri::generate_handler!`
//! for the golden-path commands only; everything else returns
//! `NotImplemented` so the page doesn't crash when (e.g.) `get_bookmarks`
//! fires during bootstrap.

use std::sync::Arc;

use serde::Deserialize;
use serde_json::Value;

use crate::commands;
use crate::config::Theme;
use crate::state::ContentNode;

use super::state::BrowserState;

/// The single window-label used by browser mode — mirrors the existing
/// `"main"` label that the WebView path uses in `lib.rs::run`.
const LABEL: &str = "main";

/// Outcome of a dispatch attempt.
pub enum DispatchResult {
    Ok(Value),
    Err(String),
    NotImplemented,
}

pub fn dispatch(cmd: &str, args: Value, state: &Arc<BrowserState>) -> DispatchResult {
    match cmd {
        "get_content" => map(commands::get_content_impl(
            &state.review,
            LABEL,
            state.json,
        )),

        "save_content" => {
            #[derive(Deserialize)]
            struct A {
                path: String,
            }
            match parse::<A>(args) {
                Ok(a) => map(commands::save_content_impl(&state.review, LABEL, a.path)),
                Err(e) => DispatchResult::Err(e),
            }
        }

        "upsert_annotation" => {
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct A {
                path: String,
                start_line: u32,
                end_line: u32,
                content: Vec<ContentNode>,
            }
            match parse::<A>(args) {
                Ok(a) => map(commands::upsert_annotation_impl(
                    &state.review,
                    a.path,
                    a.start_line,
                    a.end_line,
                    a.content,
                )
                .map(|_| ())),
                Err(e) => DispatchResult::Err(e),
            }
        }

        "delete_annotation" => {
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct A {
                path: String,
                start_line: u32,
                end_line: u32,
            }
            match parse::<A>(args) {
                Ok(a) => map(commands::delete_annotation_impl(
                    &state.review,
                    a.path,
                    a.start_line,
                    a.end_line,
                )
                .map(|_| ())),
                Err(e) => DispatchResult::Err(e),
            }
        }

        "get_terraform_regions" => {
            #[derive(Deserialize)]
            struct A {
                path: String,
            }
            match parse::<A>(args) {
                Ok(a) => map(commands::get_terraform_regions_impl(&state.review, a.path)),
                Err(e) => DispatchResult::Err(e),
            }
        }

        "get_theme" => map(Ok::<_, String>(commands::get_theme())),

        "set_theme" => {
            #[derive(Deserialize)]
            struct A {
                theme: Theme,
            }
            match parse::<A>(args) {
                Ok(a) => map(commands::set_theme(a.theme)),
                Err(e) => DispatchResult::Err(e),
            }
        }

        _ => DispatchResult::NotImplemented,
    }
}

fn map<T: serde::Serialize>(r: Result<T, String>) -> DispatchResult {
    match r {
        Ok(v) => match serde_json::to_value(v) {
            Ok(v) => DispatchResult::Ok(v),
            Err(e) => DispatchResult::Err(format!("serialize: {e}")),
        },
        Err(e) => DispatchResult::Err(e),
    }
}

fn parse<T: serde::de::DeserializeOwned>(v: Value) -> Result<T, String> {
    serde_json::from_value(v).map_err(|e| format!("bad args: {e}"))
}
