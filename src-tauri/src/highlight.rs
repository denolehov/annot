use std::path::Path;
use std::sync::LazyLock;

use syntect::dumps::from_uncompressed_data;
use syntect::html::{ClassStyle, ClassedHTMLGenerator};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

/// Pre-compiled SyntaxSet loaded from build-time generated dump.
/// This avoids the ~120ms cost of loading/parsing grammars at runtime.
static SYNTAX_SET: LazyLock<SyntaxSet> = LazyLock::new(|| {
    from_uncompressed_data(include_bytes!(concat!(env!("OUT_DIR"), "/syntaxes.packdump")))
        .expect("Failed to load embedded syntax set")
});

/// Syntax highlighter using syntect with embedded grammars.
pub struct Highlighter {
    syntax_set: &'static SyntaxSet,
}

impl Highlighter {
    /// Create a new highlighter using the pre-compiled syntax set.
    pub fn new() -> Self {
        Self {
            syntax_set: &SYNTAX_SET,
        }
    }

    /// Map markdown code fence language names to file extensions for syntect.
    ///
    /// Uses [`crate::lang::fence_language_to_extension`] then applies syntect-specific
    /// fallbacks (e.g., TypeScript → JavaScript since syntect lacks TS grammar).
    pub fn language_to_extension(lang: &str) -> String {
        let ext = crate::lang::fence_language_to_extension(lang);
        Self::extension_fallback(&ext).to_string()
    }

    /// Detect language from file extension.
    /// Returns the syntax name if found, None otherwise.
    pub fn detect_language(&self, path: &str) -> Option<&str> {
        let ext = Path::new(path).extension()?.to_str()?;
        self.syntax_set
            .find_syntax_by_extension(ext)
            .map(|s| s.name.as_str())
    }

    /// Highlight a single-line code snippet and return HTML.
    ///
    /// Returns HTML with spans containing CSS classes for syntax highlighting.
    /// Falls back to plain text (HTML-escaped) if language is unknown.
    pub fn highlight_snippet(&self, snippet: &str, path: &str) -> String {
        let lines = self.highlight_lines(snippet, path);
        lines.into_iter().next().unwrap_or_default()
    }

    /// Map file extensions that syntect doesn't support to ones it does.
    fn extension_fallback(ext: &str) -> &str {
        match ext {
            // TypeScript → JavaScript (syntect default doesn't include TS)
            "ts" | "tsx" | "mts" | "cts" => "js",
            // JSX also uses JavaScript
            "jsx" | "mjs" | "cjs" => "js",
            // Svelte → HTML (syntect default doesn't include Svelte)
            "svelte" => "html",
            _ => ext,
        }
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

        // Try the original extension first, then fall back to a compatible one
        let syntax = self
            .syntax_set
            .find_syntax_by_extension(ext)
            .or_else(|| self.syntax_set.find_syntax_by_extension(Self::extension_fallback(ext)))
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
    fn typescript_falls_back_to_javascript_highlighting() {
        let hl = Highlighter::new();

        // syntect doesn't have TypeScript, but we should still get JS highlighting
        let ts_code = "const x: number = 42;";
        let lines = hl.highlight_lines(ts_code, "app.ts");

        // Should have highlighting classes (not plain text)
        assert!(
            lines[0].contains("class="),
            "TypeScript should be highlighted as JavaScript. Actual: {}",
            lines[0]
        );

        // Test other TS variants
        for ext in &["tsx", "mts", "cts"] {
            let lines = hl.highlight_lines(ts_code, &format!("app.{}", ext));
            assert!(
                lines[0].contains("class="),
                ".{} should be highlighted as JavaScript",
                ext
            );
        }
    }

    #[test]
    fn svelte_falls_back_to_html_highlighting() {
        let hl = Highlighter::new();

        // syntect doesn't have Svelte, but we should get HTML highlighting
        let svelte_code = "<script>\n  let count = 0;\n</script>\n\n<button on:click={() => count++}>\n  Clicked {count} times\n</button>";
        let lines = hl.highlight_lines(svelte_code, "App.svelte");

        // Should have highlighting classes (not plain text)
        assert!(
            lines[0].contains("class="),
            "Svelte should be highlighted as HTML. Actual: {}",
            lines[0]
        );
        // The <script> tag should be recognized
        assert!(
            lines[0].contains("script"),
            "Should recognize script tag. Actual: {}",
            lines[0]
        );
    }

    #[test]
    fn list_all_syntaxes_with_extensions() {
        let hl = Highlighter::new();
        println!("\n=== ALL SYNTAXES ===");
        for syntax in hl.syntax_set.syntaxes() {
            if !syntax.file_extensions.is_empty() {
                println!("{}: {:?}", syntax.name, syntax.file_extensions);
            }
        }
        println!("=== END ===\n");
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

    // ========== MERMAID SYNTAX HIGHLIGHTING TESTS ==========

    #[test]
    fn mermaid_grammar_is_loaded() {
        let hl = Highlighter::new();
        let syntax = hl.syntax_set.find_syntax_by_extension("mermaid");
        assert!(syntax.is_some(), "Mermaid syntax should be loaded");
        assert_eq!(syntax.unwrap().name, "Mermaid");
    }

    #[test]
    fn mermaid_detects_from_extension() {
        let hl = Highlighter::new();
        assert_eq!(hl.detect_language("diagram.mermaid"), Some("Mermaid"));
        assert_eq!(hl.detect_language("diagram.mmd"), Some("Mermaid"));
    }

    #[test]
    fn mermaid_language_to_extension_mapping() {
        // languages crate returns "mmd" as primary extension for Mermaid
        assert_eq!(Highlighter::language_to_extension("mermaid"), "mmd");
        assert_eq!(Highlighter::language_to_extension("mmd"), "mmd");
        assert_eq!(Highlighter::language_to_extension("MERMAID"), "mmd");
    }

    #[test]
    fn mermaid_highlights_comments() {
        let hl = Highlighter::new();
        let code = "%% This is a comment";
        let lines = hl.highlight_lines(code, "diagram.mermaid");

        println!("\n=== MERMAID COMMENT ===");
        println!("Input: {:?}", code);
        println!("Output: {:?}", lines[0]);
        println!("=== END ===\n");

        assert!(
            lines[0].contains("comment"),
            "Comment should have comment class. Actual: {}",
            lines[0]
        );
    }

    #[test]
    fn mermaid_highlights_flowchart() {
        let hl = Highlighter::new();
        let code = r#"graph TD
    A[Start] --> B{Decision}
    B -->|Yes| C[OK]
    B -->|No| D[Cancel]"#;

        let lines = hl.highlight_lines(code, "diagram.mermaid");

        println!("\n=== MERMAID FLOWCHART ===");
        for (i, line) in lines.iter().enumerate() {
            println!("Line {}: {}", i + 1, line);
        }
        println!("=== END ===\n");

        // Line 1: "graph TD" - should have diagram keyword and direction
        assert!(
            lines[0].contains("keyword"),
            "graph should be highlighted as keyword. Actual: {}",
            lines[0]
        );
        assert!(
            lines[0].contains("constant"),
            "TD should be highlighted as constant. Actual: {}",
            lines[0]
        );

        // Lines should have arrow operators
        assert!(
            lines[1].contains("keyword") || lines[1].contains("operator"),
            "Arrow --> should be highlighted. Actual: {}",
            lines[1]
        );
    }

    #[test]
    fn mermaid_highlights_sequence_diagram() {
        let hl = Highlighter::new();
        let code = r#"sequenceDiagram
    participant Alice
    participant Bob
    Alice->>Bob: Hello Bob
    Bob-->>Alice: Hi Alice"#;

        let lines = hl.highlight_lines(code, "diagram.mermaid");

        println!("\n=== MERMAID SEQUENCE DIAGRAM ===");
        for (i, line) in lines.iter().enumerate() {
            println!("Line {}: {}", i + 1, line);
        }
        println!("=== END ===\n");

        // Line 1: sequenceDiagram keyword
        assert!(
            lines[0].contains("keyword"),
            "sequenceDiagram should be highlighted. Actual: {}",
            lines[0]
        );

        // participant keyword
        assert!(
            lines[1].contains("keyword"),
            "participant should be highlighted. Actual: {}",
            lines[1]
        );
    }

    #[test]
    fn mermaid_highlights_class_diagram() {
        let hl = Highlighter::new();
        let code = r#"classDiagram
    class Animal {
        +String name
        +makeSound()
    }
    Animal <|-- Dog"#;

        let lines = hl.highlight_lines(code, "diagram.mermaid");

        println!("\n=== MERMAID CLASS DIAGRAM ===");
        for (i, line) in lines.iter().enumerate() {
            println!("Line {}: {}", i + 1, line);
        }
        println!("=== END ===\n");

        // classDiagram keyword
        assert!(
            lines[0].contains("keyword"),
            "classDiagram should be highlighted. Actual: {}",
            lines[0]
        );

        // class keyword
        assert!(
            lines[1].contains("storage") || lines[1].contains("keyword"),
            "class should be highlighted. Actual: {}",
            lines[1]
        );
    }

    #[test]
    fn mermaid_highlights_strings() {
        let hl = Highlighter::new();
        let code = r#"graph TD
    A["Node with quoted text"]"#;

        let lines = hl.highlight_lines(code, "diagram.mermaid");

        println!("\n=== MERMAID STRINGS ===");
        for (i, line) in lines.iter().enumerate() {
            println!("Line {}: {}", i + 1, line);
        }
        println!("=== END ===\n");

        // Should have string class
        assert!(
            lines[1].contains("string"),
            "Quoted text should be highlighted as string. Actual: {}",
            lines[1]
        );
    }

    #[test]
    fn mermaid_highlights_gantt() {
        let hl = Highlighter::new();
        let code = r#"gantt
    title A Gantt Diagram
    dateFormat YYYY-MM-DD
    section Section
    Task1 :done, a1, 2024-01-01, 30d"#;

        let lines = hl.highlight_lines(code, "diagram.mermaid");

        println!("\n=== MERMAID GANTT ===");
        for (i, line) in lines.iter().enumerate() {
            println!("Line {}: {}", i + 1, line);
        }
        println!("=== END ===\n");

        // gantt keyword
        assert!(
            lines[0].contains("keyword"),
            "gantt should be highlighted. Actual: {}",
            lines[0]
        );

        // title keyword
        assert!(
            lines[1].contains("keyword"),
            "title should be highlighted. Actual: {}",
            lines[1]
        );

        // dateFormat keyword
        assert!(
            lines[2].contains("keyword"),
            "dateFormat should be highlighted. Actual: {}",
            lines[2]
        );
    }

    #[test]
    fn mermaid_highlights_state_diagram() {
        let hl = Highlighter::new();
        let code = r#"stateDiagram-v2
    [*] --> Still
    Still --> Moving
    Moving --> [*]"#;

        let lines = hl.highlight_lines(code, "diagram.mermaid");

        println!("\n=== MERMAID STATE DIAGRAM ===");
        for (i, line) in lines.iter().enumerate() {
            println!("Line {}: {}", i + 1, line);
        }
        println!("=== END ===\n");

        // stateDiagram-v2 keyword
        assert!(
            lines[0].contains("keyword"),
            "stateDiagram-v2 should be highlighted. Actual: {}",
            lines[0]
        );

        // [*] special state marker
        assert!(
            lines[1].contains("constant") || lines[1].contains("class="),
            "[*] should be highlighted. Actual: {}",
            lines[1]
        );
    }

    #[test]
    fn mermaid_highlights_er_diagram() {
        let hl = Highlighter::new();
        let code = r#"erDiagram
    CUSTOMER ||--o{ ORDER : places
    ORDER ||--|{ LINE-ITEM : contains"#;

        let lines = hl.highlight_lines(code, "diagram.mermaid");

        println!("\n=== MERMAID ER DIAGRAM ===");
        for (i, line) in lines.iter().enumerate() {
            println!("Line {}: {}", i + 1, line);
        }
        println!("=== END ===\n");

        // erDiagram keyword
        assert!(
            lines[0].contains("keyword"),
            "erDiagram should be highlighted. Actual: {}",
            lines[0]
        );
    }

    #[test]
    fn mermaid_highlights_gitgraph() {
        let hl = Highlighter::new();
        let code = r#"gitGraph
    commit
    branch develop
    checkout develop
    commit
    checkout main
    merge develop"#;

        let lines = hl.highlight_lines(code, "diagram.mermaid");

        println!("\n=== MERMAID GIT GRAPH ===");
        for (i, line) in lines.iter().enumerate() {
            println!("Line {}: {}", i + 1, line);
        }
        println!("=== END ===\n");

        // gitGraph keyword
        assert!(
            lines[0].contains("keyword"),
            "gitGraph should be highlighted. Actual: {}",
            lines[0]
        );

        // commit, branch, checkout, merge keywords
        assert!(
            lines[1].contains("keyword"),
            "commit should be highlighted. Actual: {}",
            lines[1]
        );
    }

    #[test]
    fn mermaid_highlights_all_arrow_types() {
        let hl = Highlighter::new();
        let code = r#"graph LR
    A --> B
    B ---> C
    C -.-> D
    D ==> E
    E -->> F"#;

        let lines = hl.highlight_lines(code, "diagram.mermaid");

        println!("\n=== MERMAID ARROWS ===");
        for (i, line) in lines.iter().enumerate() {
            println!("Line {}: {}", i + 1, line);
        }
        println!("=== END ===\n");

        // All arrow lines should have highlighting
        for (i, line) in lines.iter().enumerate().skip(1) {
            assert!(
                line.contains("keyword") || line.contains("operator"),
                "Line {} arrow should be highlighted. Actual: {}",
                i + 1,
                line
            );
        }
    }

    #[test]
    fn mermaid_highlights_subgraph() {
        let hl = Highlighter::new();
        let code = r#"graph TD
    subgraph sub1[Title]
        A --> B
    end"#;

        let lines = hl.highlight_lines(code, "diagram.mermaid");

        println!("\n=== MERMAID SUBGRAPH ===");
        for (i, line) in lines.iter().enumerate() {
            println!("Line {}: {}", i + 1, line);
        }
        println!("=== END ===\n");

        // subgraph keyword
        assert!(
            lines[1].contains("keyword"),
            "subgraph should be highlighted. Actual: {}",
            lines[1]
        );

        // end keyword
        assert!(
            lines[3].contains("keyword"),
            "end should be highlighted. Actual: {}",
            lines[3]
        );
    }

    /// Documents CSS classes produced for markdown tables inside code blocks.
    /// This is a regression test for the bug where `.header` CSS selector
    /// was matching `span.meta.table.header.markdown` from syntect.
    #[test]
    fn documents_markdown_table_in_code_block_classes() {
        let hl = Highlighter::new();

        // Markdown table syntax - this is what appears inside a ```md code block
        let table_md = r#"| Col1 | Col2 | Col3 |
|:------------|:-------:|------------:|
| Left-aligned | Centered | Right-aligned |
| blah | blah | blah |"#;

        let lines = hl.highlight_lines(table_md, "file.md");

        println!("\n=== MARKDOWN TABLE CLASSES ===");
        for (i, line) in lines.iter().enumerate() {
            println!("Line {}: {}", i + 1, line);
        }
        println!("=== END ===\n");

        // The header row should produce spans with classes like "meta table header markdown"
        // IMPORTANT: This test documents that syntect produces a "header" class
        // which can conflict with CSS selectors like `.header { display: flex; }`
        assert!(
            lines[0].contains("header"),
            "Table header row should have 'header' class. Actual: {}",
            lines[0]
        );

        // Specifically look for the meta.table.header scope
        assert!(
            lines[0].contains("meta") && lines[0].contains("table"),
            "Should have meta and table classes. Actual: {}",
            lines[0]
        );
    }

    /// Documents the full HTML output for mermaid diagrams
    #[test]
    fn documents_mermaid_html_output() {
        let hl = Highlighter::new();

        let mermaid_code = r#"graph TD
    %% This is a flowchart
    A[Start] --> B{Is it?}
    B -->|Yes| C["Done"]
    B -->|No| D[Think]
    D --> B"#;

        let lines = hl.highlight_lines(mermaid_code, "diagram.mermaid");

        println!("\n=== SYNTECT HTML OUTPUT (Mermaid) ===");
        for (i, line) in lines.iter().enumerate() {
            println!("Line {}: {}", i + 1, line);
        }
        println!("=== END ===\n");

        // Basic verification that we get highlighted output
        assert!(!lines.is_empty(), "Should produce output lines");
        assert!(
            lines[0].contains("class="),
            "Mermaid should be highlighted, not plain text"
        );
    }
}
