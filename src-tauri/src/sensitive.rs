//! Sensitive-path blocklist shared by every feature that pulls local files
//! into a review (portals, the `annot-content://` protocol).
//!
//! Base-dir confinement only guarantees "no bytes from outside the reviewed
//! directory" — but secrets routinely live *inside* it (`.env` next to the
//! page under review). This blocklist is the tripwire for those.

use std::path::Path;

/// Sensitive path patterns that should be blocked.
pub const SENSITIVE_PATTERNS: &[&str] = &[
    "id_rsa",
    "id_dsa",
    "id_ecdsa",
    "id_ed25519",
    ".env",
    "credentials",
    "secrets",
    ".ssh/",
    ".aws/",
    ".gcp/",
    ".pem",
    ".key",
];

/// The first sensitive pattern the path matches, if any.
/// Lowercased substring match over the whole path.
pub fn sensitive_match(path: &Path) -> Option<&'static str> {
    let path_str = path.to_string_lossy().to_lowercase();
    SENSITIVE_PATTERNS
        .iter()
        .find(|pattern| path_str.contains(**pattern))
        .copied()
}

/// Whether the path matches any sensitive pattern.
pub fn is_sensitive_path(path: &Path) -> bool {
    sensitive_match(path).is_some()
}
