//! Declarative string builder for formatted output.
//!
//! OutputBuilder provides a structured way to build annotation output text
//! with automatic indentation management and mode-aware alignment.

/// Output mode determines line number formatting.
#[derive(Debug, Clone)]
pub enum BuilderMode {
    /// File mode: single line number column
    File { line_num_width: usize },
    /// Diff mode: two line number columns (old:new)
    Diff {
        left_width: usize,
        right_width: usize,
    },
}

impl Default for BuilderMode {
    fn default() -> Self {
        BuilderMode::File { line_num_width: 3 }
    }
}

/// Separator line used between content blocks.
pub const SEPARATOR: &str = "────────────────────────────────────";

/// Section divider between major sections.
pub const SECTION_DIVIDER: &str = "\n---\n\n";

/// Declarative string builder with automatic indentation and alignment.
pub struct OutputBuilder {
    buffer: String,
    indent_level: usize,
    mode: BuilderMode,
}

impl OutputBuilder {
    /// Create a new builder with the specified mode.
    pub fn new(mode: BuilderMode) -> Self {
        Self {
            buffer: String::new(),
            indent_level: 0,
            mode,
        }
    }

    // === Structural ===

    /// Add a named section with header and indented body.
    ///
    /// The content closure is called with indentation increased by one level.
    pub fn section(&mut self, name: &str, f: impl FnOnce(&mut Self)) -> &mut Self {
        self.line(&format!("{}:", name));
        self.indented(f);
        self.blank_line();
        self
    }

    /// Execute closure with increased indentation.
    pub fn indented(&mut self, f: impl FnOnce(&mut Self)) -> &mut Self {
        self.indent_level += 1;
        f(self);
        self.indent_level = self.indent_level.saturating_sub(1);
        self
    }

    /// Add section divider: "\n---\n\n"
    pub fn divider(&mut self) -> &mut Self {
        self.buffer.push_str(SECTION_DIVIDER);
        self
    }

    /// Add separator line: "────────────────────────────────────"
    pub fn separator(&mut self) -> &mut Self {
        self.raw_line(SEPARATOR);
        self
    }

    /// Add empty line.
    pub fn blank_line(&mut self) -> &mut Self {
        self.buffer.push('\n');
        self
    }

    // === Content ===

    /// Add single line with current indentation.
    pub fn line(&mut self, text: &str) -> &mut Self {
        let indent = "  ".repeat(self.indent_level);
        self.buffer.push_str(&indent);
        self.buffer.push_str(text);
        self.buffer.push('\n');
        self
    }

    /// Add raw line without indentation.
    pub fn raw_line(&mut self, text: &str) -> &mut Self {
        self.buffer.push_str(text);
        self.buffer.push('\n');
        self
    }

    /// Add raw text without newline.
    pub fn raw(&mut self, text: &str) -> &mut Self {
        self.buffer.push_str(text);
        self
    }

    /// Add key-value field on single line.
    pub fn field(&mut self, key: &str, value: &str) -> &mut Self {
        self.line(&format!("{}: {}", key, value))
    }

    /// Add key with multi-line indented value.
    pub fn field_multiline(&mut self, key: &str, text: &str) -> &mut Self {
        self.line(&format!("{}:", key));
        self.indented(|b| {
            for line in text.lines() {
                b.line(line);
            }
        });
        self
    }

    // === Annotation-specific ===

    /// Format code line: "  42 | content"
    pub fn code_line(&mut self, num: u32, content: &str) -> &mut Self {
        let width = self.line_num_width();
        self.buffer
            .push_str(&format!("{:>width$} | {}\n", num, content, width = width + 3));
        self
    }

    /// Format selected code line with ">" prefix: ">  42 | content"
    pub fn selected_code_line(&mut self, num: u32, content: &str) -> &mut Self {
        let width = self.line_num_width();
        self.buffer
            .push_str(&format!("> {:>width$} | {}\n", num, content, width = width + 1));
        self
    }

    /// Format diff line: "  42:43 | content" or "    :43 | content"
    pub fn diff_line(
        &mut self,
        old: Option<u32>,
        new: Option<u32>,
        content: &str,
        is_selected: bool,
    ) -> &mut Self {
        let width = self.line_num_width();
        let prefix = if is_selected { "> " } else { "  " };
        let old_str = old.map(|n| n.to_string()).unwrap_or_default();
        let new_str = new.map(|n| n.to_string()).unwrap_or_default();

        self.buffer.push_str(&format!(
            "{}{:>w$}:{:<w$} | {}\n",
            prefix,
            old_str,
            new_str,
            content,
            w = width
        ));
        self
    }

    /// Format arrow line: "     └──> content" (padding calculated from mode)
    pub fn arrow(&mut self, content: &str) -> &mut Self {
        let padding = self.arrow_padding();
        self.buffer
            .push_str(&format!("{}└──> {}\n", " ".repeat(padding), content));
        self
    }

    /// Format continuation line aligned with arrow content.
    pub fn arrow_continuation(&mut self, content: &str) -> &mut Self {
        let padding = self.arrow_padding() + 5; // 5 = "└──> ".len()
        self.buffer
            .push_str(&format!("{}{}\n", " ".repeat(padding), content));
        self
    }

    // === Output ===

    /// Consume builder and return the formatted string.
    pub fn build(self) -> String {
        self.buffer
    }

    /// Check if buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    // === Internal ===

    fn line_num_width(&self) -> usize {
        match &self.mode {
            BuilderMode::File { line_num_width } => *line_num_width,
            BuilderMode::Diff {
                left_width,
                right_width,
            } => (*left_width).max(*right_width),
        }
    }

    fn arrow_padding(&self) -> usize {
        match &self.mode {
            // "> {:>width+1} | " => 2 + width+1 + 3 = width + 6, but we want to align with "|"
            // Actually: "> " (2) + number (width+1) + " | " (3) = width + 6
            // Arrow should be under the "|", so padding = width + 4
            BuilderMode::File { line_num_width } => line_num_width + 4,
            // "> " (2) + old (w) + ":" (1) + new (w) + " | " (3) = 2w + 6
            // Arrow should be under the "|", so padding = 2w + 4
            BuilderMode::Diff {
                left_width,
                right_width,
            } => left_width + right_width + 4,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_line() {
        let mut b = OutputBuilder::new(BuilderMode::default());
        b.line("hello");
        assert_eq!(b.build(), "hello\n");
    }

    #[test]
    fn indented_content() {
        let mut b = OutputBuilder::new(BuilderMode::default());
        b.line("outer");
        b.indented(|b| {
            b.line("inner");
        });
        b.line("outer again");
        assert_eq!(b.build(), "outer\n  inner\nouter again\n");
    }

    #[test]
    fn section_formatting() {
        let mut b = OutputBuilder::new(BuilderMode::default());
        b.section("LEGEND", |b| {
            b.line("[# TAG] instruction");
        });
        assert_eq!(b.build(), "LEGEND:\n  [# TAG] instruction\n\n");
    }

    #[test]
    fn field_and_multiline() {
        let mut b = OutputBuilder::new(BuilderMode::default());
        b.field("Key", "value");
        b.field_multiline("Multi", "line1\nline2");
        assert_eq!(b.build(), "Key: value\nMulti:\n  line1\n  line2\n");
    }

    #[test]
    fn code_lines_file_mode() {
        let mut b = OutputBuilder::new(BuilderMode::File { line_num_width: 2 });
        b.code_line(5, "content");
        b.selected_code_line(6, "selected");
        let output = b.build();
        // code_line: width = 2 + 3 = 5, so "    5 | content"
        assert!(output.contains("    5 | content"), "Got: {}", output);
        // selected_code_line: "> " + width=3, so ">   6 | selected"
        assert!(output.contains(">   6 | selected"), "Got: {}", output);
    }

    #[test]
    fn diff_lines() {
        let mut b = OutputBuilder::new(BuilderMode::Diff {
            left_width: 2,
            right_width: 2,
        });
        b.diff_line(Some(10), Some(10), "context", false);
        b.diff_line(Some(11), None, "deleted", true);
        b.diff_line(None, Some(11), "added", true);
        let output = b.build();
        assert!(output.contains("  10:10 | context"));
        assert!(output.contains("> 11:   | deleted"));
        assert!(output.contains(">   :11 | added"));
    }

    #[test]
    fn arrow_alignment_file_mode() {
        let mut b = OutputBuilder::new(BuilderMode::File { line_num_width: 2 });
        b.selected_code_line(42, "code here");
        b.arrow("first line");
        b.arrow_continuation("second line");
        let output = b.build();
        // Arrow should align under the pipe
        assert!(output.contains("└──> first line"));
        assert!(output.contains("     second line"));
    }

    #[test]
    fn separator_and_divider() {
        let mut b = OutputBuilder::new(BuilderMode::default());
        b.line("before");
        b.separator();
        b.divider();
        b.line("after");
        let output = b.build();
        assert!(output.contains(SEPARATOR));
        assert!(output.contains("---"));
    }
}
