//! `annot-content://` — serves a reviewed page's bytes to the webview on a
//! dedicated origin.
//!
//! HTML review renders the page under review inside an iframe. That frame
//! must load the page and its relative assets (`./styles.css`, `./logo.svg`)
//! — and must NOT share an origin with annot's own frontend. The reviewed
//! page is untrusted (often agent-authored): cross-origin isolation is what
//! keeps it away from annot's DOM, IPC, and `__TAURI_INTERNALS__`. A
//! dedicated scheme buys that on every platform — its origin
//! (`annot-content://localhost` on macOS/Linux, `http://annot-content.localhost`
//! on Windows) is distinct from the app origin, the dev origin, and
//! `asset://`.
//!
//! The serving root is derived per request from the active review's base
//! directory; exactly one review exists at a time, so the URL carries no
//! session id. Every security rule — traversal, symlink escape, extension
//! allowlist, sensitive-path blocklist, size cap — lives in the pure
//! [`resolve`] function so it is a tempdir unit test, not a click-through.

use std::fs;
use std::path::{Component, Path, PathBuf};

use tauri::{AppHandle, Manager, Runtime};

use crate::input::{CliSource, ContentSource, McpSource};
use crate::review::ActiveReview;
use crate::sensitive::is_sensitive_path;

/// Largest asset the protocol will serve.
pub const MAX_ASSET_BYTES: u64 = 32 * 1024 * 1024;

/// The mime table IS the allowlist: an extension absent from it is never
/// served — no `application/octet-stream` fallback, because the fallback
/// would be the hole.
const SERVABLE: &[(&str, &str)] = &[
    ("html", "text/html"),
    ("htm", "text/html"),
    ("css", "text/css"),
    ("js", "text/javascript"),
    ("mjs", "text/javascript"),
    ("json", "application/json"),
    ("map", "application/json"),
    ("svg", "image/svg+xml"),
    ("png", "image/png"),
    ("jpg", "image/jpeg"),
    ("jpeg", "image/jpeg"),
    ("gif", "image/gif"),
    ("webp", "image/webp"),
    ("avif", "image/avif"),
    ("ico", "image/x-icon"),
    ("woff", "font/woff"),
    ("woff2", "font/woff2"),
    ("ttf", "font/ttf"),
    ("otf", "font/otf"),
    ("wasm", "application/wasm"),
    ("txt", "text/plain"),
    ("csv", "text/csv"),
    ("md", "text/markdown"),
];

/// One resolved response body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asset {
    pub bytes: Vec<u8>,
    pub mime: &'static str,
}

/// The only failure vocabulary; every arm maps to a status + `text/plain` body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServeError {
    BadRequest,
    NotFound,
    Forbidden,
    TooLarge,
    NoReview,
}

impl ServeError {
    fn status(self) -> u16 {
        match self {
            ServeError::BadRequest => 400,
            ServeError::NotFound => 404,
            ServeError::Forbidden => 403,
            ServeError::TooLarge => 413,
            ServeError::NoReview => 503,
        }
    }

    fn message(self) -> &'static str {
        match self {
            ServeError::BadRequest => "bad request",
            ServeError::NotFound => "not found",
            ServeError::Forbidden => "forbidden",
            ServeError::TooLarge => "asset too large",
            ServeError::NoReview => "no active review",
        }
    }
}

/// Mime type for a path, by lowercased extension. `None` means *not servable*.
pub fn mime_for(path: &Path) -> Option<&'static str> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    SERVABLE
        .iter()
        .find(|(e, _)| *e == ext)
        .map(|(_, mime)| *mime)
}

/// URL for a path relative to the serving root.
///
/// Real `/` separators with each segment percent-encoded individually — NOT
/// `convertFileSrc()`, which encodes `/` as `%2F` and collapses the path into
/// a single segment, breaking relative asset resolution inside the iframe.
pub fn asset_url(rel: &Path) -> String {
    let path = rel
        .components()
        .map(|c| urlencoding::encode(&c.as_os_str().to_string_lossy()).into_owned())
        .collect::<Vec<_>>()
        .join("/");
    if cfg!(windows) {
        format!("http://annot-content.localhost/{path}")
    } else {
        format!("annot-content://localhost/{path}")
    }
}

/// Reserved Windows DOS device name (`CON`, `NUL`, `COM1`, …), matched per
/// Win32 rules: the base name is everything before the first `.` or `:`,
/// trailing whitespace trimmed, case-insensitive. `COM¹`/`LPT²`-style
/// superscript digits count — legacy parsing resolves them as port numbers.
fn is_reserved_dos_name(component: &str) -> bool {
    let base = component
        .split(['.', ':'])
        .next()
        .unwrap_or("")
        .trim_end_matches(|c: char| c.is_ascii_whitespace() || c == '\u{000B}');
    let upper = base.to_uppercase();
    if matches!(
        upper.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) {
        return true;
    }
    if let Some(rest) = upper
        .strip_prefix("COM")
        .or_else(|| upper.strip_prefix("LPT"))
    {
        let mut chars = rest.chars();
        if let (Some(digit), None) = (chars.next(), chars.next()) {
            return digit.is_ascii_digit() || matches!(digit, '¹' | '²' | '³');
        }
    }
    false
}

/// Resolve a request path against the serving root.
///
/// Pure: every security rule is here and unit-tested. Blocklist checks run
/// against paths *relative* to the root (unlike portals, which match the full
/// path) so a review under e.g. `~/projects/secrets-scanner/` still works.
pub fn resolve(base_dir: &Path, url_path: &str) -> Result<Asset, ServeError> {
    let decoded = urlencoding::decode(url_path).map_err(|_| ServeError::BadRequest)?;
    let rel = decoded.strip_prefix('/').unwrap_or(&decoded);
    if rel.is_empty() {
        return Err(ServeError::BadRequest);
    }
    let rel = Path::new(rel);

    // Reject `..`, absolute paths, and Windows drive/UNC prefixes *before*
    // the join — `Path::join` silently discards the base for absolute paths.
    // Reserved DOS names must also die here: on Windows, `fs::canonicalize`
    // *opens* the name, and legacy Win32 resolution routes `COM1.txt` to the
    // serial device — a read that blocks forever. So the canonicalize
    // backstop below cannot catch these (tower-http's ServeDir rejects them
    // pre-open for the same reason). Checked on every platform so the rule
    // stays uniform and unit-testable.
    let components_ok = rel.components().all(|c| match c {
        Component::Normal(name) => !is_reserved_dos_name(&name.to_string_lossy()),
        _ => false,
    });
    if !components_ok {
        return Err(ServeError::Forbidden);
    }
    if mime_for(rel).is_none() {
        return Err(ServeError::Forbidden);
    }
    if is_sensitive_path(rel) {
        return Err(ServeError::Forbidden);
    }

    // Canonicalizing resolves symlinks: a link inside the dir pointing out of
    // it fails the prefix check here and nowhere else.
    let base = fs::canonicalize(base_dir).map_err(|_| ServeError::NotFound)?;
    let full = fs::canonicalize(base.join(rel)).map_err(|_| ServeError::NotFound)?;
    if !full.starts_with(&base) {
        return Err(ServeError::Forbidden);
    }

    // Re-check the *canonical* target: a symlink inside the dir may point at
    // a sensitive or non-servable file inside the dir (`notes.css -> .env`).
    // Hardlinks remain uncatchable — accepted residual.
    let canonical_rel = full.strip_prefix(&base).unwrap_or(&full);
    let mime = mime_for(canonical_rel).ok_or(ServeError::Forbidden)?;
    if is_sensitive_path(canonical_rel) {
        return Err(ServeError::Forbidden);
    }

    let meta = fs::metadata(&full).map_err(|_| ServeError::NotFound)?;
    if !meta.is_file() {
        return Err(ServeError::NotFound);
    }
    if meta.len() > MAX_ASSET_BYTES {
        return Err(ServeError::TooLarge);
    }
    let bytes = fs::read(&full).map_err(|_| ServeError::NotFound)?;
    Ok(Asset { bytes, mime })
}

/// Serving root of the active review.
///
/// Only file-backed reviews serve: for stdin/`review_content` sources,
/// `base_dir()` falls back to the process cwd, which is not the reviewed
/// page's directory — refuse rather than serve it. In-memory documents get
/// deliberate semantics in S1. The lock is never held across I/O.
fn active_base_dir<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, ServeError> {
    let slot = app.state::<ActiveReview>();
    let guard = slot.lock();
    let review = guard.as_ref().ok_or(ServeError::NoReview)?;
    let source = &review.root_view.content().source;
    match source {
        ContentSource::Cli(CliSource::File { .. }) | ContentSource::Mcp(McpSource::File { .. }) => {
            Ok(source.base_dir())
        }
        _ => Err(ServeError::NoReview),
    }
}

/// The single point where an `Asset` becomes a `Response` (A1 wraps HTML
/// bodies here). No `Access-Control-Allow-Origin`: annot's frontend must not
/// read content-origin bytes, and the frame stays cross-origin. `no-store`
/// because the origin and paths repeat across reviews with different base
/// dirs — a cached body would leak one review's bytes into the next.
fn respond(result: Result<Asset, ServeError>) -> tauri::http::Response<Vec<u8>> {
    let builder = tauri::http::Response::builder().header("Cache-Control", "no-store");
    match result {
        Ok(asset) => builder
            .status(200)
            .header("Content-Type", asset.mime)
            .body(asset.bytes),
        Err(err) => builder
            .status(err.status())
            .header("Content-Type", "text/plain")
            .body(err.message().as_bytes().to_vec()),
    }
    .expect("static response parts are valid")
}

/// Register the protocol on a builder chain. Must be called on BOTH chains —
/// `run()` and `run_mcp()` construct separate builders.
pub fn register<R: Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder.register_asynchronous_uri_scheme_protocol("annot-content", |ctx, request, responder| {
        let base = active_base_dir(ctx.app_handle());
        let path = request.uri().path().to_string();
        // Disk I/O off the UI thread — same failure family as the WebView2
        // window-build deadlock (CLAUDE.md).
        tauri::async_runtime::spawn_blocking(move || {
            responder.respond(respond(base.and_then(|dir| resolve(&dir, &path))));
        });
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn dir_with(files: &[(&str, &str)]) -> TempDir {
        let dir = TempDir::new().unwrap();
        for (path, content) in files {
            let full = dir.path().join(path);
            if let Some(parent) = full.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(full, content).unwrap();
        }
        dir
    }

    #[test]
    fn serves_html_as_text_html() {
        let dir = dir_with(&[("report.html", "<h1>hi</h1>")]);
        let asset = resolve(dir.path(), "/report.html").unwrap();
        assert_eq!(asset.mime, "text/html");
        assert_eq!(asset.bytes, b"<h1>hi</h1>");
    }

    #[test]
    fn serves_nested_asset() {
        let dir = dir_with(&[("assets/app.css", "body{}")]);
        let asset = resolve(dir.path(), "/assets/app.css").unwrap();
        assert_eq!(asset.mime, "text/css");
    }

    #[test]
    fn percent_encoded_name_resolves() {
        let dir = dir_with(&[("my logo.svg", "<svg/>")]);
        let asset = resolve(dir.path(), "/my%20logo.svg").unwrap();
        assert_eq!(asset.mime, "image/svg+xml");
    }

    #[test]
    fn uppercase_extension_resolves() {
        let dir = dir_with(&[("LOGO.SVG", "<svg/>")]);
        let asset = resolve(dir.path(), "/LOGO.SVG").unwrap();
        assert_eq!(asset.mime, "image/svg+xml");
    }

    #[test]
    fn traversal_rejected() {
        let dir = dir_with(&[("report.html", "x")]);
        assert_eq!(
            resolve(dir.path(), "/../../etc/passwd"),
            Err(ServeError::Forbidden)
        );
    }

    #[test]
    fn encoded_traversal_rejected() {
        let dir = dir_with(&[("report.html", "x")]);
        assert_eq!(
            resolve(dir.path(), "/%2e%2e/secret.html"),
            Err(ServeError::Forbidden)
        );
    }

    #[test]
    fn absolute_path_rejected() {
        let dir = dir_with(&[]);
        assert_eq!(
            resolve(dir.path(), "//etc/passwd"),
            Err(ServeError::Forbidden)
        );
    }

    #[test]
    fn sensitive_path_rejected() {
        let dir = dir_with(&[("credentials.json", "{}")]);
        assert_eq!(
            resolve(dir.path(), "/credentials.json"),
            Err(ServeError::Forbidden)
        );
    }

    #[test]
    fn dotenv_rejected() {
        let dir = dir_with(&[(".env", "SECRET=1")]);
        assert_eq!(resolve(dir.path(), "/.env"), Err(ServeError::Forbidden));
    }

    #[test]
    fn unlisted_extension_rejected() {
        let dir = dir_with(&[("main.rs", "fn main() {}")]);
        assert_eq!(resolve(dir.path(), "/main.rs"), Err(ServeError::Forbidden));
    }

    #[test]
    fn missing_file_not_found() {
        let dir = dir_with(&[]);
        assert_eq!(
            resolve(dir.path(), "/missing.html"),
            Err(ServeError::NotFound)
        );
    }

    #[test]
    fn empty_path_bad_request() {
        let dir = dir_with(&[]);
        assert_eq!(resolve(dir.path(), "/"), Err(ServeError::BadRequest));
        assert_eq!(resolve(dir.path(), ""), Err(ServeError::BadRequest));
    }

    #[test]
    fn invalid_percent_encoding_bad_request() {
        let dir = dir_with(&[]);
        assert_eq!(
            resolve(dir.path(), "/%FF.html"),
            Err(ServeError::BadRequest)
        );
    }

    #[test]
    fn oversize_rejected() {
        let dir = dir_with(&[("big.txt", "")]);
        let file = fs::OpenOptions::new()
            .write(true)
            .open(dir.path().join("big.txt"))
            .unwrap();
        file.set_len(MAX_ASSET_BYTES + 1).unwrap();
        assert_eq!(resolve(dir.path(), "/big.txt"), Err(ServeError::TooLarge));
    }

    #[cfg(unix)]
    #[test]
    fn symlink_escaping_dir_rejected() {
        let outside = dir_with(&[("secret.html", "outside")]);
        let dir = dir_with(&[]);
        std::os::unix::fs::symlink(
            outside.path().join("secret.html"),
            dir.path().join("link.html"),
        )
        .unwrap();
        assert_eq!(
            resolve(dir.path(), "/link.html"),
            Err(ServeError::Forbidden)
        );
    }

    #[cfg(unix)]
    #[test]
    fn symlink_to_sensitive_file_inside_dir_rejected() {
        let dir = dir_with(&[(".env", "SECRET=1")]);
        std::os::unix::fs::symlink(dir.path().join(".env"), dir.path().join("notes.css")).unwrap();
        assert_eq!(
            resolve(dir.path(), "/notes.css"),
            Err(ServeError::Forbidden)
        );
    }

    #[cfg(unix)]
    #[test]
    fn symlink_to_unservable_file_inside_dir_rejected() {
        let dir = dir_with(&[("main.rs", "fn main() {}")]);
        std::os::unix::fs::symlink(dir.path().join("main.rs"), dir.path().join("app.css")).unwrap();
        assert_eq!(resolve(dir.path(), "/app.css"), Err(ServeError::Forbidden));
    }

    #[test]
    fn reserved_dos_names_rejected() {
        let dir = dir_with(&[("nul.txt", "x"), ("console.html", "<p>ok</p>")]);
        assert_eq!(resolve(dir.path(), "/nul.txt"), Err(ServeError::Forbidden));
        assert_eq!(resolve(dir.path(), "/COM1.css"), Err(ServeError::Forbidden));
        assert_eq!(
            resolve(dir.path(), "/assets/lpt9.png"),
            Err(ServeError::Forbidden)
        );
        // Non-reserved lookalikes pass the gate.
        assert!(resolve(dir.path(), "/console.html").is_ok());
    }

    #[test]
    fn reserved_dos_name_matching() {
        for name in [
            "CON",
            "nul",
            "Nul.txt",
            "COM1",
            "com9.css",
            "LPT1",
            "lpt¹.txt",
            "CONIN$",
            "aux : stream",
        ] {
            assert!(is_reserved_dos_name(name), "{name} should be reserved");
        }
        for name in [
            "console",
            "com.css",
            "com10.txt",
            "lptx.png",
            "auxiliary.html",
            "nully.txt",
        ] {
            assert!(!is_reserved_dos_name(name), "{name} should not be reserved");
        }
    }

    #[test]
    fn asset_url_encodes_segments_not_separators() {
        let url = asset_url(Path::new("my logo.svg"));
        let expected_host = if cfg!(windows) {
            "http://annot-content.localhost"
        } else {
            "annot-content://localhost"
        };
        assert_eq!(url, format!("{expected_host}/my%20logo.svg"));
        assert_eq!(
            asset_url(Path::new("a b").join("c.css").as_path()),
            format!("{expected_host}/a%20b/c.css")
        );
    }
}
