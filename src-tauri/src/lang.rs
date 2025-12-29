//! Language detection and mapping utilities.
//!
//! Provides bidirectional mapping between:
//! - File extensions (e.g., "rs", "go", "ts")
//! - Markdown code fence language names (e.g., "rust", "go", "typescript")
//!
//! Uses GitHub's languages.yml dataset via the `languages` crate.

/// Map file extension to markdown code fence language name.
/// Used for exporting code blocks with proper language hints.
///
/// Returns lowercase language name suitable for markdown code fences.
pub fn extension_to_fence_language(ext: &str) -> &'static str {
    // The languages crate handles case-insensitivity
    if let Some(lang) = languages::from_extension(ext) {
        return language_name_to_fence(lang.name);
    }
    ""
}

/// Map markdown code fence language to file extension.
///
/// Returns the primary file extension for the language.
pub fn fence_language_to_extension(lang: &str) -> String {
    let lower = lang.to_lowercase();

    // The languages crate handles case-insensitivity for from_name
    if let Some(lang_info) = languages::from_name(&lower) {
        if let Some(exts) = lang_info.extensions {
            if let Some(ext) = exts.first() {
                // Strip leading dot
                return ext.trim_start_matches('.').to_string();
            }
        }
    }

    // Default: use the language name as-is
    lower
}

/// Convert language name to fence-appropriate lowercase form.
fn language_name_to_fence(name: &str) -> &'static str {
    match name {
        "Rust" => "rust",
        "Go" => "go",
        "Python" => "python",
        "Ruby" => "ruby",
        "JavaScript" => "javascript",
        "TypeScript" => "typescript",
        "TSX" => "tsx",
        "Java" => "java",
        "Kotlin" => "kotlin",
        "Scala" => "scala",
        "Swift" => "swift",
        "C#" => "csharp",
        "F#" => "fsharp",
        "C++" => "cpp",
        "C" => "c",
        "Objective-C" => "objc",
        "Haskell" => "haskell",
        "OCaml" => "ocaml",
        "Elm" => "elm",
        "Elixir" => "elixir",
        "Erlang" => "erlang",
        "Clojure" => "clojure",
        "Common Lisp" => "lisp",
        "Scheme" => "scheme",
        "Lua" => "lua",
        "R" => "r",
        "Julia" => "julia",
        "Perl" => "perl",
        "PHP" => "php",
        "Shell" => "bash",
        "PowerShell" => "powershell",
        "SQL" => "sql",
        "GraphQL" => "graphql",
        "HTML" => "html",
        "CSS" => "css",
        "SCSS" => "scss",
        "Less" => "less",
        "XML" => "xml",
        "JSON" => "json",
        "YAML" => "yaml",
        "TOML" => "toml",
        "INI" => "ini",
        "Markdown" => "markdown",
        "Diff" => "diff",
        "Dockerfile" => "dockerfile",
        "Makefile" => "makefile",
        "TeX" => "latex",
        "Vim Script" => "vim",
        "Svelte" => "svelte",
        "Vue" => "vue",
        "Mermaid" => "mermaid",
        // Default: return empty for unknown languages
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extension_to_fence_common_languages() {
        assert_eq!(extension_to_fence_language("rs"), "rust");
        assert_eq!(extension_to_fence_language("go"), "go");
        assert_eq!(extension_to_fence_language("ts"), "typescript");
        assert_eq!(extension_to_fence_language("tsx"), "tsx");
        assert_eq!(extension_to_fence_language("py"), "python");
        assert_eq!(extension_to_fence_language("js"), "javascript");
    }

    #[test]
    fn extension_to_fence_case_insensitive() {
        assert_eq!(extension_to_fence_language("RS"), "rust");
        assert_eq!(extension_to_fence_language("Go"), "go");
    }

    #[test]
    fn extension_to_fence_unknown_returns_empty() {
        assert_eq!(extension_to_fence_language("xyz123"), "");
    }

    #[test]
    fn extension_to_fence_mermaid_svelte_vue() {
        assert_eq!(extension_to_fence_language("mermaid"), "mermaid");
        assert_eq!(extension_to_fence_language("svelte"), "svelte");
        assert_eq!(extension_to_fence_language("vue"), "vue");
    }

    #[test]
    fn fence_to_extension_common_languages() {
        assert_eq!(fence_language_to_extension("rust"), "rs");
        assert_eq!(fence_language_to_extension("go"), "go");
        assert_eq!(fence_language_to_extension("python"), "py");
        assert_eq!(fence_language_to_extension("typescript"), "ts");
    }

    #[test]
    fn fence_to_extension_case_insensitive() {
        assert_eq!(fence_language_to_extension("RUST"), "rs");
        assert_eq!(fence_language_to_extension("Python"), "py");
    }
}
