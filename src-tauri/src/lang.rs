//! Language detection and mapping utilities.
//!
//! Provides bidirectional mapping between:
//! - File extensions (e.g., "rs", "go", "ts")
//! - Markdown code fence language names (e.g., "rust", "go", "typescript")

/// Map file extension to markdown code fence language name.
/// Used for exporting code blocks with proper language hints.
pub fn extension_to_fence_language(ext: &str) -> &'static str {
    match ext.to_lowercase().as_str() {
        "rs" => "rust",
        "go" => "go",
        "py" => "python",
        "rb" => "ruby",
        "js" | "mjs" | "cjs" => "javascript",
        "ts" | "tsx" | "mts" | "cts" => "typescript",
        "jsx" => "javascript",
        "java" => "java",
        "kt" => "kotlin",
        "scala" => "scala",
        "swift" => "swift",
        "cs" => "csharp",
        "fs" => "fsharp",
        "cpp" | "cc" | "cxx" | "c++" => "cpp",
        "c" | "h" => "c",
        "m" => "objc",
        "hs" => "haskell",
        "ml" => "ocaml",
        "elm" => "elm",
        "ex" | "exs" => "elixir",
        "erl" => "erlang",
        "clj" | "cljs" => "clojure",
        "lisp" | "cl" => "lisp",
        "scm" => "scheme",
        "lua" => "lua",
        "r" => "r",
        "jl" => "julia",
        "pl" | "pm" => "perl",
        "php" => "php",
        "sh" | "bash" | "zsh" => "bash",
        "ps1" => "powershell",
        "sql" => "sql",
        "graphql" | "gql" => "graphql",
        "html" | "htm" => "html",
        "css" => "css",
        "scss" => "scss",
        "less" => "less",
        "svelte" => "svelte",
        "vue" => "vue",
        "xml" => "xml",
        "json" => "json",
        "yaml" | "yml" => "yaml",
        "toml" => "toml",
        "ini" | "conf" => "ini",
        "md" | "markdown" => "markdown",
        "diff" | "patch" => "diff",
        "dockerfile" => "dockerfile",
        "makefile" => "makefile",
        "tex" | "latex" => "latex",
        "vim" => "vim",
        "mermaid" | "mmd" => "mermaid",
        _ => "",
    }
}

/// Map markdown code fence language names to file extensions.
/// Many languages use their full name in code fences but syntect needs the extension.
pub fn fence_language_to_extension(lang: &str) -> String {
    let lower = lang.to_lowercase();
    match lower.as_str() {
        // Common languages with different fence names vs extensions
        // TypeScript falls back to JavaScript (syntect default doesn't include TS)
        "typescript" | "ts" | "tsx" => "js".to_string(),
        "javascript" | "js" | "jsx" => "js".to_string(),
        "python" | "py" => "py".to_string(),
        "ruby" | "rb" => "rb".to_string(),
        "rust" | "rs" => "rs".to_string(),
        "golang" | "go" => "go".to_string(),
        "haskell" | "hs" => "hs".to_string(),
        "csharp" | "c#" => "cs".to_string(),
        "fsharp" | "f#" => "fs".to_string(),
        "cpp" | "c++" => "cpp".to_string(),
        "objectivec" | "objc" | "objective-c" => "m".to_string(),
        "kotlin" | "kt" => "kt".to_string(),
        "scala" => "scala".to_string(),
        "swift" => "swift".to_string(),
        "bash" | "shell" | "sh" | "zsh" => "sh".to_string(),
        "powershell" | "ps1" => "ps1".to_string(),
        "dockerfile" => "Dockerfile".to_string(),
        "makefile" | "make" => "Makefile".to_string(),
        "yaml" | "yml" => "yaml".to_string(),
        "json" => "json".to_string(),
        "xml" => "xml".to_string(),
        "html" => "html".to_string(),
        "css" => "css".to_string(),
        "scss" => "scss".to_string(),
        "less" => "less".to_string(),
        "sql" => "sql".to_string(),
        "graphql" | "gql" => "graphql".to_string(),
        "markdown" | "md" => "md".to_string(),
        "toml" => "toml".to_string(),
        "ini" | "conf" => "ini".to_string(),
        "java" => "java".to_string(),
        "php" => "php".to_string(),
        "perl" | "pl" => "pl".to_string(),
        "lua" => "lua".to_string(),
        "r" => "r".to_string(),
        "julia" | "jl" => "jl".to_string(),
        "elixir" | "ex" => "ex".to_string(),
        "erlang" | "erl" => "erl".to_string(),
        "clojure" | "clj" => "clj".to_string(),
        "lisp" | "cl" => "lisp".to_string(),
        "scheme" | "scm" => "scm".to_string(),
        "ocaml" | "ml" => "ml".to_string(),
        "elm" => "elm".to_string(),
        "vim" | "vimscript" => "vim".to_string(),
        "diff" | "patch" => "diff".to_string(),
        "tex" | "latex" => "tex".to_string(),
        // Mermaid diagrams
        "mermaid" | "mmd" => "mermaid".to_string(),
        // Default: use the language name as-is (might work for some)
        _ => lower,
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
        assert_eq!(extension_to_fence_language("tsx"), "typescript");
        assert_eq!(extension_to_fence_language("py"), "python");
        assert_eq!(extension_to_fence_language("js"), "javascript");
    }

    #[test]
    fn extension_to_fence_unknown_returns_empty() {
        assert_eq!(extension_to_fence_language("unknown"), "");
        assert_eq!(extension_to_fence_language("xyz123"), "");
    }

    #[test]
    fn extension_to_fence_case_insensitive() {
        assert_eq!(extension_to_fence_language("RS"), "rust");
        assert_eq!(extension_to_fence_language("Go"), "go");
    }

    #[test]
    fn fence_to_extension_common_languages() {
        assert_eq!(fence_language_to_extension("rust"), "rs");
        assert_eq!(fence_language_to_extension("go"), "go");
        assert_eq!(fence_language_to_extension("typescript"), "js"); // Falls back to JS for syntect
        assert_eq!(fence_language_to_extension("python"), "py");
        assert_eq!(fence_language_to_extension("javascript"), "js");
    }

    #[test]
    fn fence_to_extension_case_insensitive() {
        assert_eq!(fence_language_to_extension("RUST"), "rs");
        assert_eq!(fence_language_to_extension("Python"), "py");
    }

    #[test]
    fn fence_to_extension_unknown_returns_lowercase() {
        assert_eq!(fence_language_to_extension("unknown"), "unknown");
        assert_eq!(fence_language_to_extension("CUSTOM"), "custom");
    }
}
