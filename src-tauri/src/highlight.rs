use std::path::Path;

use syntect::html::{ClassStyle, ClassedHTMLGenerator};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

/// Syntax highlighter using syntect with embedded grammars.
pub struct Highlighter {
    syntax_set: SyntaxSet,
}

impl Highlighter {
    /// Create a new highlighter with default syntaxes.
    pub fn new() -> Self {
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
        }
    }

    /// Detect language from file extension.
    /// Returns the syntax name if found, None otherwise.
    pub fn detect_language(&self, path: &str) -> Option<&str> {
        let ext = Path::new(path).extension()?.to_str()?;
        self.syntax_set
            .find_syntax_by_extension(ext)
            .map(|s| s.name.as_str())
    }

    /// Highlight file content and return HTML for each line.
    ///
    /// Each line contains HTML spans with CSS classes (e.g., `<span class="k">fn</span>`).
    /// Falls back to plain text (HTML-escaped) if language is unknown.
    pub fn highlight_lines(&self, content: &str, path: &str) -> Vec<String> {
        let ext = Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        let syntax = self
            .syntax_set
            .find_syntax_by_extension(ext)
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        let mut html_generator = ClassedHTMLGenerator::new_with_class_style(
            syntax,
            &self.syntax_set,
            ClassStyle::Spaced,
        );

        // Parse the entire content to maintain cross-line state
        for line in LinesWithEndings::from(content) {
            // This can fail on invalid UTF-8, but we've already read as String
            let _ = html_generator.parse_html_for_line_which_includes_newline(line);
        }

        // Get the full HTML and split by lines
        let html = html_generator.finalize();

        // Split the HTML output back into lines
        // syntect outputs newlines as actual newlines within the HTML
        html.lines().map(|s| s.to_string()).collect()
    }
}

impl Default for Highlighter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_rust_from_extension() {
        let hl = Highlighter::new();
        assert_eq!(hl.detect_language("src/main.rs"), Some("Rust"));
    }

    #[test]
    fn detects_javascript_from_extension() {
        let hl = Highlighter::new();
        assert_eq!(hl.detect_language("app.js"), Some("JavaScript"));
    }

    #[test]
    fn returns_none_for_unknown_extension() {
        let hl = Highlighter::new();
        assert_eq!(hl.detect_language("file.xyz123"), None);
    }

    #[test]
    fn highlights_rust_code() {
        let hl = Highlighter::new();
        let code = "fn main() {\n    println!(\"Hello\");\n}";
        let lines = hl.highlight_lines(code, "test.rs");

        assert_eq!(lines.len(), 3);
        // First line should contain highlighted "fn" keyword
        assert!(lines[0].contains("class="));
        assert!(lines[0].contains("fn"));
    }

    #[test]
    fn handles_plain_text() {
        let hl = Highlighter::new();
        let code = "just some text\nwith lines";
        let lines = hl.highlight_lines(code, "file.txt");

        assert_eq!(lines.len(), 2);
        // Plain text should still be escaped
        assert!(lines[0].contains("just some text"));
    }

    #[test]
    fn escapes_html_in_code() {
        let hl = Highlighter::new();
        let code = "let x = \"<script>alert('xss')</script>\";";
        let lines = hl.highlight_lines(code, "test.rs");

        // Should be HTML-escaped
        assert!(!lines[0].contains("<script>"));
        assert!(lines[0].contains("&lt;script&gt;") || lines[0].contains("&lt;"));
    }

    /// This test documents the exact HTML structure and CSS classes that syntect produces.
    /// Use this as a reference when writing CSS for syntax highlighting.
    #[test]
    fn documents_html_structure_and_classes() {
        let hl = Highlighter::new();

        // Rust code sample with various token types
        let rust_code = r#"// Comment
fn main() {
    let x = 42;
    let s = "hello";
    println!("Value: {}", x);
}"#;

        let lines = hl.highlight_lines(rust_code, "example.rs");

        // Print the actual HTML for debugging/documentation
        println!("\n=== SYNTECT HTML OUTPUT (Rust) ===");
        for (i, line) in lines.iter().enumerate() {
            println!("Line {}: {}", i + 1, line);
        }
        println!("=== END ===\n");

        // Verify structure: syntect uses <span class="..."> tags
        assert!(lines[0].contains("<span"), "Expected HTML spans in output");

        // Document the actual classes syntect uses (these assertions serve as documentation)
        // Line 1: "// Comment" - should have comment class
        assert!(
            lines[0].contains("class="),
            "Comment line should have CSS classes. Actual: {}",
            lines[0]
        );

        // Line 2: "fn main() {" - should have keyword class for 'fn'
        assert!(
            lines[1].contains("class="),
            "Function definition should have CSS classes. Actual: {}",
            lines[1]
        );

        // Line 3: "let x = 42;" - should have keyword for 'let', number for '42'
        assert!(
            lines[2].contains("class="),
            "Variable declaration should have CSS classes. Actual: {}",
            lines[2]
        );

        // Line 4: 'let s = "hello";' - should have string class
        assert!(
            lines[3].contains("class="),
            "String literal should have CSS classes. Actual: {}",
            lines[3]
        );
    }

    /// Test single-line doc comment (simulates diff line highlighting)
    #[test]
    fn single_line_doc_comment_output() {
        let hl = Highlighter::new();

        // This simulates what happens in diff mode:
        // We strip the prefix and highlight just the code portion
        let code = "    /// This is a doc comment";
        let lines = hl.highlight_lines(code, "file.rs");

        println!("\n=== SINGLE LINE DOC COMMENT ===");
        println!("Input: {:?}", code);
        println!("Output lines count: {}", lines.len());
        for (i, line) in lines.iter().enumerate() {
            println!("Line {}: {:?}", i, line);
        }
        println!("=== END ===\n");

        // Should produce exactly 1 line of output
        assert_eq!(lines.len(), 1, "Single line input should produce single line output");

        // The output should not contain literal newlines
        assert!(!lines[0].contains('\n'), "Output should not contain newline characters");
    }

    /// Documents HTML output for JavaScript to show class naming patterns
    #[test]
    fn documents_javascript_html_classes() {
        let hl = Highlighter::new();

        let js_code = r#"// JS comment
function greet(name) {
    const msg = "Hello " + name;
    return msg;
}"#;

        let lines = hl.highlight_lines(js_code, "example.js");

        println!("\n=== SYNTECT HTML OUTPUT (JavaScript) ===");
        for (i, line) in lines.iter().enumerate() {
            println!("Line {}: {}", i + 1, line);
        }
        println!("=== END ===\n");

        // Verify we get highlighted output
        assert!(lines[1].contains("class="), "Function should be highlighted");
    }
}
