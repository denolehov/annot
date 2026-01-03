//! Project file discovery for @ file references.
//!
//! Uses the `ignore` crate to walk directories while respecting .gitignore,
//! and `sublime_fuzzy` for filename-first fuzzy matching.

use std::path::Path;
use std::time::{Duration, Instant};

use parking_lot::Mutex;

use ignore::WalkBuilder;
use sublime_fuzzy::best_match;
use tauri::State;

/// Cached list of project files with timestamp for staleness detection.
pub struct FileCache {
    files: Vec<String>,
    cached_at: Instant,
    root: Option<String>,
}

impl FileCache {
    pub fn new() -> Self {
        Self {
            files: Vec::new(),
            cached_at: Instant::now() - Duration::from_secs(3600), // Start stale
            root: None,
        }
    }

    /// Check if the cache is stale (older than 60 seconds or different root).
    fn is_stale(&self, root: &str) -> bool {
        self.cached_at.elapsed() > Duration::from_secs(60)
            || self.root.as_deref() != Some(root)
    }

    /// Invalidate the cache (called on window focus).
    pub fn invalidate(&mut self) {
        self.cached_at = Instant::now() - Duration::from_secs(3600);
    }

    /// Refresh the cache by scanning the project directory.
    fn refresh(&mut self, root: &str) {
        let root_path = Path::new(root);

        let walker = WalkBuilder::new(root_path)
            .hidden(true)           // Skip hidden files
            .git_ignore(true)       // Respect .gitignore
            .git_global(true)       // Respect global gitignore
            .git_exclude(true)      // Respect .git/info/exclude
            .max_depth(Some(15))    // Reasonable depth limit
            .build();

        self.files = walker
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
            .filter_map(|e| {
                e.path()
                    .strip_prefix(root_path)
                    .ok()
                    .map(|p| p.to_string_lossy().into_owned())
            })
            // Filter out common non-code files
            .filter(|p| !is_excluded_file(p))
            .collect();

        self.cached_at = Instant::now();
        self.root = Some(root.to_string());
    }

    /// Get files, refreshing if stale.
    fn get_files(&mut self, root: &str) -> &[String] {
        if self.is_stale(root) {
            self.refresh(root);
        }
        &self.files
    }
}

/// Filter out files that are unlikely to be referenced in annotations.
fn is_excluded_file(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);

    // Exclude lock files, build artifacts, etc.
    matches!(name,
        "package-lock.json" | "yarn.lock" | "pnpm-lock.yaml" |
        "Cargo.lock" | "go.sum" | "poetry.lock" | "composer.lock" |
        ".DS_Store" | "Thumbs.db"
    ) || name.ends_with(".min.js")
      || name.ends_with(".min.css")
      || name.ends_with(".map")
}

/// Fuzzy filter files with filename-first ranking.
/// Matches on filename get a 2x score boost.
fn fuzzy_filter(files: &[String], query: &str, limit: usize) -> Vec<String> {
    if query.is_empty() {
        return Vec::new(); // Empty query = no results (require min 2 chars on frontend)
    }

    let mut scored: Vec<(isize, &String)> = files
        .iter()
        .filter_map(|path| {
            let filename = Path::new(path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(path);

            // Try filename first (2x boost)
            if let Some(m) = best_match(query, filename) {
                return Some((m.score() * 2, path));
            }

            // Fall back to full path
            best_match(query, path).map(|m| (m.score(), path))
        })
        .collect();

    // Sort by score descending
    scored.sort_by(|a, b| b.0.cmp(&a.0));

    scored
        .into_iter()
        .take(limit)
        .map(|(_, p)| p.clone())
        .collect()
}

/// Tauri-managed state for file cache.
pub type FileCacheState = Mutex<FileCache>;

/// List project files matching the query.
///
/// Uses the `ignore` crate to respect .gitignore and `sublime_fuzzy` for
/// filename-first fuzzy matching.
#[tauri::command]
pub fn list_project_files(
    state: State<FileCacheState>,
    query: String,
    limit: usize,
) -> Vec<String> {
    // Get current working directory as project root
    let root = std::env::current_dir()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| ".".to_string());

    let mut cache = state.lock();
    let files = cache.get_files(&root);

    fuzzy_filter(files, &query, limit.min(50)) // Hard cap at 50
}

/// Invalidate the file cache (call on window focus).
#[tauri::command]
pub fn invalidate_file_cache(state: State<FileCacheState>) {
    let mut cache = state.lock();
    cache.invalidate();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fuzzy_filter_empty_query_returns_empty() {
        let files = vec!["src/main.rs".to_string()];
        assert!(fuzzy_filter(&files, "", 10).is_empty());
    }

    #[test]
    fn fuzzy_filter_prefers_filename_over_path() {
        // When query matches filename directly, it should rank higher
        // than matching only in the path
        let files = vec![
            "src/types/utils.ts".to_string(),  // "types" only in path
            "types.ts".to_string(),             // "types" in filename
        ];
        let result = fuzzy_filter(&files, "types", 10);
        // Filename match (2x boost) should beat path-only match
        assert_eq!(result.first().map(|s| s.as_str()), Some("types.ts"));
    }

    #[test]
    fn fuzzy_filter_respects_limit() {
        let files: Vec<String> = (0..100).map(|i| format!("file{}.ts", i)).collect();
        let result = fuzzy_filter(&files, "file", 5);
        assert_eq!(result.len(), 5);
    }

    #[test]
    fn is_excluded_filters_lock_files() {
        assert!(is_excluded_file("package-lock.json"));
        assert!(is_excluded_file("Cargo.lock"));
        assert!(is_excluded_file("deep/path/yarn.lock"));
        assert!(!is_excluded_file("src/main.rs"));
    }
}
