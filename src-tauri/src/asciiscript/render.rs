//! Renderer for asciiscript AST.
//!
//! Converts AST to ASCII box art following the render pipeline:
//! 1. Measure: Bottom-up natural width computation
//! 2. Resolve: Top-down width assignment
//! 3. Allocate: Distribute space in rows, expand spacers
//! 4. Render: Generate ASCII lines

use crate::asciiscript::ast::*;
use crate::asciiscript::parser::{ParseError, Parser};
use unicode_width::UnicodeWidthStr;

/// Get the visual display width of a string (handles Unicode, emoji, CJK).
fn display_width(s: &str) -> usize {
    UnicodeWidthStr::width(s)
}

/// Pad a string to a target visual width using spaces.
/// Standard format! padding doesn't work for Unicode because it counts codepoints, not display width.
fn pad_to_width(s: &str, target_width: usize) -> String {
    let current_width = display_width(s);
    if current_width >= target_width {
        s.to_string()
    } else {
        format!("{}{}", s, " ".repeat(target_width - current_width))
    }
}

/// Truncate a string to fit within a maximum visual width.
/// Adds "…" suffix if truncated.
fn truncate_to_width(s: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    let current_width = display_width(s);
    if current_width <= max_width {
        return s.to_string();
    }
    // Need to truncate - walk characters and accumulate width
    let mut result = String::new();
    let mut width = 0;
    let ellipsis_width = 1; // "…" is width 1
    let target = max_width.saturating_sub(ellipsis_width);

    for c in s.chars() {
        let char_width = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
        if width + char_width > target {
            break;
        }
        result.push(c);
        width += char_width;
    }
    result.push('…');
    result
}

/// Fit a string to exactly the target visual width: truncate if too long, pad if too short.
fn fit_to_width(s: &str, target_width: usize) -> String {
    let current_width = display_width(s);
    if current_width > target_width {
        truncate_to_width(s, target_width)
    } else if current_width < target_width {
        pad_to_width(s, target_width)
    } else {
        s.to_string()
    }
}

/// Render asciiscript source to ASCII art.
pub fn render(source: &str) -> Result<String, ParseError> {
    let program = Parser::parse(source)?;
    Ok(render_program(&program))
}

/// Render a parsed program to ASCII art.
pub fn render_program(program: &Program) -> String {
    let mut ctx = RenderContext::new();
    render_layout(&program.layout, &mut ctx, None);
    ctx.build()
}

/// Rendering context that accumulates output lines.
struct RenderContext {
    lines: Vec<String>,
    indent: usize,
}

impl RenderContext {
    fn new() -> Self {
        Self {
            lines: Vec::new(),
            indent: 0,
        }
    }

    fn push_line(&mut self, line: &str) {
        let indent = " ".repeat(self.indent);
        self.lines.push(format!("{}{}", indent, line));
    }

    fn push_lines(&mut self, lines: &[String]) {
        for line in lines {
            self.push_line(line);
        }
    }

    fn with_indent<F>(&mut self, delta: usize, f: F)
    where
        F: FnOnce(&mut Self),
    {
        self.indent += delta;
        f(self);
        self.indent -= delta;
    }

    fn build(self) -> String {
        self.lines.join("\n")
    }
}

// ========== Measurement ==========

/// Measured dimensions for an element.
#[derive(Debug, Clone, Default)]
struct Measured {
    width: usize,
    height: usize,
    is_spacer: bool,
}

impl Measured {
    fn new(width: usize, height: usize) -> Self {
        Self { width, height, is_spacer: false }
    }

    fn spacer() -> Self {
        Self { width: 0, height: 1, is_spacer: true }
    }
}

fn measure_statement(stmt: &Statement) -> Measured {
    match stmt {
        Statement::Container(c) => measure_container(c),
        Statement::Primitive(p) => measure_primitive(p),
    }
}

fn measure_container(c: &Container) -> Measured {
    // Measure all children
    let child_measurements: Vec<Measured> = c.statements.iter().map(measure_statement).collect();

    match c.kind {
        ContainerKind::Row => {
            // Row: width = sum of children + gaps, height = max height
            let gap = c.modifiers.gap.unwrap_or(1) as usize;
            let non_spacer_count = child_measurements.iter().filter(|m| !m.is_spacer).count();
            let gaps = if non_spacer_count > 1 { (non_spacer_count - 1) * gap } else { 0 };
            let width: usize = child_measurements.iter().map(|m| m.width).sum::<usize>() + gaps;
            let height = child_measurements.iter().map(|m| m.height).max().unwrap_or(1);
            Measured::new(width, height)
        }
        ContainerKind::Column | ContainerKind::Section => {
            // Column/Section: width = max width, height = sum of heights + gaps
            let gap = c.modifiers.gap.unwrap_or(0) as usize;
            let width = child_measurements.iter().map(|m| m.width).max().unwrap_or(0);
            let height: usize = child_measurements.iter().map(|m| m.height).sum::<usize>()
                + if child_measurements.len() > 1 { (child_measurements.len() - 1) * gap } else { 0 };

            // Section adds 2 for indent
            if c.kind == ContainerKind::Section {
                Measured::new(width + 2, height + 1) // +1 for header
            } else {
                Measured::new(width, height)
            }
        }
        ContainerKind::Window | ContainerKind::Box => {
            // Box/Window: add borders + padding
            let padding = c.modifiers.padding.unwrap_or(1) as usize;
            let content_width = child_measurements.iter().map(|m| m.width).max().unwrap_or(0);
            let content_height: usize = child_measurements.iter().map(|m| m.height).sum();

            // Title affects width
            let title_width = c.title.as_ref().map(|t| display_width(t) + 4).unwrap_or(0); // "-- Title "
            let inner_width = content_width.max(title_width);

            // Border (2) + padding on each side
            let measured_width = inner_width + 2 + (padding * 2);
            // Respect explicit width if set, but ensure content still fits
            let width = c.modifiers.width
                .map(|w| (w as usize).max(measured_width))
                .unwrap_or(measured_width);
            let height = content_height + 2 + (padding * 2); // top/bottom border + padding

            Measured::new(width, height)
        }
    }
}

fn measure_primitive(p: &Primitive) -> Measured {
    match p {
        Primitive::Text { content, .. } => {
            let lines: Vec<&str> = content.lines().collect();
            let width = lines.iter().map(|l| display_width(l)).max().unwrap_or(0);
            Measured::new(width, lines.len().max(1))
        }
        Primitive::Input { modifiers, .. } => {
            let width = modifiers.width.unwrap_or(10) as usize;
            Measured::new(width, 1)
        }
        Primitive::Checkbox { label, .. } => {
            // [x] Label
            Measured::new(4 + display_width(label), 1)
        }
        Primitive::Radio { label, .. } => {
            // (o) Label
            Measured::new(4 + display_width(label), 1)
        }
        Primitive::Select { modifiers, .. } => {
            let width = modifiers.width.unwrap_or(10) as usize;
            Measured::new(width, 1)
        }
        Primitive::Button { label, .. } => {
            // [ Label ]
            Measured::new(display_width(label) + 4, 1)
        }
        Primitive::Link { text, .. } => {
            Measured::new(display_width(text), 1)
        }
        Primitive::Code { content, .. } | Primitive::Raw { content, .. } => {
            let lines: Vec<&str> = content.lines().collect();
            let width = lines.iter().map(|l| display_width(l)).max().unwrap_or(0);
            Measured::new(width, lines.len().max(1))
        }
        Primitive::Separator { .. } => {
            // Will expand to container width
            Measured::new(1, 1)
        }
        Primitive::Spacer { .. } => {
            Measured::spacer()
        }
        Primitive::Progress { .. } => {
            // Default progress bar width
            Measured::new(20, 1)
        }
        Primitive::Alert { statements, .. } => {
            // Measure children + add alert chrome
            let child_measurements: Vec<Measured> = statements.iter().map(measure_statement).collect();
            let content_width = child_measurements.iter().map(|m| m.width).max().unwrap_or(0);
            let content_height: usize = child_measurements.iter().map(|m| m.height).sum();
            // Alert adds "═ TYPE ═" header + borders
            Measured::new(content_width + 4, content_height + 2)
        }
        Primitive::Table(table) => measure_table(table),
        Primitive::List(list) => measure_list(list),
    }
}

fn measure_table(table: &Table) -> Measured {
    // Calculate total width from column definitions
    let col_widths: Vec<usize> = table.header.as_ref()
        .map(|h| h.columns.iter().map(|c| c.modifiers.width.unwrap_or(10) as usize).collect())
        .unwrap_or_else(|| vec![10]); // fallback

    // Width = sum of columns + separators (| around each + | between)
    let width = col_widths.iter().sum::<usize>() + col_widths.len() + 1 + (col_widths.len() * 2);

    // Height = header (2 lines with separator) + rows
    let header_height = if table.header.is_some() { 2 } else { 0 };
    let height = header_height + table.rows.len() + 2; // +2 for top/bottom border

    Measured::new(width, height)
}

fn measure_list(list: &List) -> Measured {
    let width = list.items.iter()
        .map(|item| display_width(&item.content) + 3) // "   " or " > "
        .max()
        .unwrap_or(0);
    Measured::new(width, list.items.len())
}

// ========== Rendering ==========

fn render_layout(layout: &Layout, ctx: &mut RenderContext, width: Option<usize>) {
    for stmt in &layout.statements {
        render_statement(stmt, ctx, width);
    }
}

fn render_statement(stmt: &Statement, ctx: &mut RenderContext, width: Option<usize>) {
    match stmt {
        Statement::Container(c) => render_container(c, ctx, width),
        Statement::Primitive(p) => render_primitive(p, ctx, width),
    }
}

fn render_container(c: &Container, ctx: &mut RenderContext, parent_width: Option<usize>) {
    // Determine our width
    let explicit_width = c.modifiers.width.map(|w| w as usize);
    let measured = measure_container(c);

    // For windows/boxes: use max of (explicit, measured) to ensure content fits
    // For other containers: prefer explicit/parent width, fall back to measured
    let width = match c.kind {
        ContainerKind::Window | ContainerKind::Box => {
            let base = explicit_width.or(parent_width).unwrap_or(measured.width);
            base.max(measured.width)
        }
        _ => explicit_width.or(parent_width).unwrap_or(measured.width),
    };

    match c.kind {
        ContainerKind::Window | ContainerKind::Box => {
            render_box(c, ctx, width);
        }
        ContainerKind::Section => {
            render_section(c, ctx, width);
        }
        ContainerKind::Row => {
            render_row(c, ctx, width);
        }
        ContainerKind::Column => {
            for stmt in &c.statements {
                render_statement(stmt, ctx, Some(width));
            }
        }
    }
}

fn render_box(c: &Container, ctx: &mut RenderContext, width: usize) {
    let padding = c.modifiers.padding.unwrap_or(1) as usize;
    let content_width = width.saturating_sub(2 + padding * 2);

    // Top border with optional title
    let top_border = if let Some(title) = &c.title {
        // Truncate title if it would overflow: need room for "+-- " and " -+"
        let max_title_width = width.saturating_sub(6); // +-- title -+
        let display_title = if display_width(title) > max_title_width {
            truncate_to_width(title, max_title_width)
        } else {
            title.clone()
        };
        let title_part = format!("-- {} ", display_title);
        let title_visual_width = display_width(&title_part);
        let remaining = width.saturating_sub(title_visual_width + 2); // +2 for the two '+' chars
        format!("+{}{}+", title_part, "-".repeat(remaining))
    } else {
        format!("+{}+", "-".repeat(width.saturating_sub(2)))
    };
    ctx.push_line(&top_border);

    // Top padding
    for _ in 0..padding {
        ctx.push_line(&format!("|{}|", " ".repeat(width.saturating_sub(2))));
    }

    // Render children
    let mut child_lines: Vec<String> = Vec::new();
    let mut child_ctx = RenderContext::new();
    for stmt in &c.statements {
        render_statement(stmt, &mut child_ctx, Some(content_width));
    }

    // Wrap child lines in borders (truncate if too wide, pad if too narrow)
    for line in child_ctx.lines {
        let fitted = fit_to_width(&line, content_width);
        let padding_str = " ".repeat(padding);
        child_lines.push(format!("|{}{}{}|", padding_str, fitted, padding_str));
    }

    ctx.push_lines(&child_lines);

    // Bottom padding
    for _ in 0..padding {
        ctx.push_line(&format!("|{}|", " ".repeat(width.saturating_sub(2))));
    }

    // Bottom border
    ctx.push_line(&format!("+{}+", "-".repeat(width.saturating_sub(2))));
}

fn render_section(c: &Container, ctx: &mut RenderContext, width: usize) {
    // Section header
    if let Some(title) = &c.title {
        ctx.push_line(&format!("## {}", title));
    }

    // Render children with 2-space indent
    ctx.with_indent(2, |ctx| {
        for stmt in &c.statements {
            render_statement(stmt, ctx, Some(width.saturating_sub(2)));
        }
    });
}

fn render_row(c: &Container, ctx: &mut RenderContext, width: usize) {
    let gap = c.modifiers.gap.unwrap_or(1) as usize;

    // Measure children
    let measurements: Vec<Measured> = c.statements.iter().map(measure_statement).collect();

    // Count spacers and calculate fixed content width
    let spacer_count = measurements.iter().filter(|m| m.is_spacer).count();
    let fixed_width: usize = measurements.iter().filter(|m| !m.is_spacer).map(|m| m.width).sum();
    let non_spacer_count = measurements.iter().filter(|m| !m.is_spacer).count();
    let gaps_width = if non_spacer_count > 1 { (non_spacer_count - 1) * gap } else { 0 };

    // Calculate spacer width
    let remaining = width.saturating_sub(fixed_width + gaps_width);
    let spacer_width = if spacer_count > 0 { remaining / spacer_count } else { 0 };

    // Render each child and collect their lines + track if spacer
    let mut child_outputs: Vec<(Vec<String>, usize, bool)> = Vec::new(); // (lines, width, is_spacer)
    for (stmt, m) in c.statements.iter().zip(measurements.iter()) {
        if m.is_spacer {
            let sw = if spacer_width > 0 { spacer_width } else { 1 };
            child_outputs.push((vec![" ".repeat(sw)], sw, true));
        } else {
            let mut child_ctx = RenderContext::new();
            render_statement(stmt, &mut child_ctx, None);
            let child_width = child_ctx.lines.iter().map(|l| display_width(l)).max().unwrap_or(0);
            child_outputs.push((child_ctx.lines, child_width, false));
        }
    }

    // Find max height across all children
    let max_height = child_outputs.iter().map(|(lines, _, _)| lines.len()).max().unwrap_or(1);

    // Helper: should we add a gap before element at index i?
    // Gaps only between non-spacer elements (matching measurement logic)
    let should_add_gap = |i: usize| -> bool {
        if i == 0 {
            return false;
        }
        let curr_is_spacer = child_outputs[i].2;
        let prev_is_spacer = child_outputs[i - 1].2;
        !curr_is_spacer && !prev_is_spacer
    };

    // If all children are single-line, use simple horizontal join
    if max_height == 1 {
        let gap_str = " ".repeat(gap);
        let mut result = String::new();
        for (i, (lines, _, _)) in child_outputs.iter().enumerate() {
            if should_add_gap(i) {
                result.push_str(&gap_str);
            }
            if let Some(line) = lines.first() {
                result.push_str(line);
            }
        }
        ctx.push_line(&result);
    } else {
        // Multi-line: join children horizontally, line by line
        let gap_str = " ".repeat(gap);
        for row_idx in 0..max_height {
            let mut line = String::new();
            for (i, (lines, child_width, _)) in child_outputs.iter().enumerate() {
                if should_add_gap(i) {
                    line.push_str(&gap_str);
                }
                if row_idx < lines.len() {
                    let child_line = &lines[row_idx];
                    line.push_str(child_line);
                    // Pad to child width if needed for alignment (use visual width)
                    let child_visual_width = display_width(child_line);
                    if child_visual_width < *child_width {
                        line.push_str(&" ".repeat(child_width - child_visual_width));
                    }
                } else {
                    // Pad with spaces for shorter children
                    line.push_str(&" ".repeat(*child_width));
                }
            }
            ctx.push_line(&line);
        }
    }
}

fn render_primitive(p: &Primitive, ctx: &mut RenderContext, width: Option<usize>) {
    match p {
        Primitive::Text { content, modifiers, .. } => {
            for line in content.lines() {
                let output = if let Some(w) = width {
                    let line_width = display_width(line);
                    match modifiers.align {
                        Some(Align::Center) => {
                            let total_padding = w.saturating_sub(line_width);
                            let left_pad = total_padding / 2;
                            let right_pad = total_padding - left_pad;
                            format!("{}{}{}", " ".repeat(left_pad), line, " ".repeat(right_pad))
                        }
                        Some(Align::Right) => {
                            let padding = w.saturating_sub(line_width);
                            format!("{}{}", " ".repeat(padding), line)
                        }
                        _ => line.to_string(),
                    }
                } else {
                    line.to_string()
                };
                ctx.push_line(&output);
            }
        }
        Primitive::Input { modifiers, .. } => {
            let w = modifiers.width.unwrap_or(10) as usize;
            let inner_width = w.saturating_sub(2);
            let fill = if let Some(placeholder) = &modifiers.placeholder {
                let p_width = display_width(placeholder);
                if p_width >= inner_width {
                    truncate_to_width(placeholder, inner_width)
                } else {
                    format!("{}{}", placeholder, "_".repeat(inner_width - p_width))
                }
            } else {
                "_".repeat(inner_width)
            };
            ctx.push_line(&format!("[{}]", fill));
        }
        Primitive::Checkbox { label, checked, .. } => {
            let mark = if *checked { "x" } else { " " };
            ctx.push_line(&format!("[{}] {}", mark, label));
        }
        Primitive::Radio { label, selected, .. } => {
            let mark = if *selected { "o" } else { " " };
            ctx.push_line(&format!("({}) {}", mark, label));
        }
        Primitive::Select { value, modifiers, .. } => {
            let w = modifiers.width.unwrap_or(10) as usize;
            let inner_width = w.saturating_sub(3); // "[" + " v]"
            let value_width = display_width(value);
            let formatted = if value_width > inner_width {
                truncate_to_width(value, inner_width)
            } else {
                pad_to_width(value, inner_width)
            };
            ctx.push_line(&format!("[{} v]", formatted));
        }
        Primitive::Button { label, .. } => {
            ctx.push_line(&format!("[ {} ]", label));
        }
        Primitive::Link { text, .. } => {
            ctx.push_line(text);
        }
        Primitive::Code { content, .. } | Primitive::Raw { content, .. } => {
            for line in content.lines() {
                ctx.push_line(line);
            }
        }
        Primitive::Separator { .. } => {
            let w = width.unwrap_or(40);
            ctx.push_line(&"-".repeat(w));
        }
        Primitive::Spacer { .. } => {
            // Spacer is handled in row rendering
        }
        Primitive::Progress { value, .. } => {
            let bar_width = width.unwrap_or(20).saturating_sub(7); // " 100%"
            let filled = (bar_width * (*value as usize)) / 100;
            let empty = bar_width - filled;
            ctx.push_line(&format!(
                "[{}{}] {}%",
                "█".repeat(filled),
                "░".repeat(empty),
                value
            ));
        }
        Primitive::Alert { alert_type, statements, .. } => {
            render_alert(*alert_type, statements, ctx, width);
        }
        Primitive::Table(table) => {
            render_table(table, ctx);
        }
        Primitive::List(list) => {
            render_list(list, ctx);
        }
    }
}

fn render_alert(alert_type: AlertType, statements: &[Statement], ctx: &mut RenderContext, width: Option<usize>) {
    // Measure content
    let measurements: Vec<Measured> = statements.iter().map(measure_statement).collect();
    let content_width = measurements.iter().map(|m| m.width).max().unwrap_or(0);
    // Use parent width if provided, otherwise size to content
    let total_width = width.unwrap_or(content_width + 4);
    let inner_width = total_width.saturating_sub(4);

    let (border_h, border_v, corner_tl, corner_tr, corner_bl, corner_br) = match alert_type {
        AlertType::Error => ('═', '║', '╔', '╗', '╚', '╝'),
        AlertType::Warn | AlertType::Info => ('─', '│', '┌', '┐', '└', '┘'),
    };

    let type_str = match alert_type {
        AlertType::Error => "ERROR",
        AlertType::Warn => "WARNING",
        AlertType::Info => "INFO",
    };

    // Top border with type label (must match content width: inner_width + 4)
    let label_part = format!("{} {} ", border_h, type_str);
    let label_visual_width = display_width(&label_part);
    let remaining = (inner_width + 2).saturating_sub(label_visual_width);
    ctx.push_line(&format!(
        "{}{}{}{}",
        corner_tl,
        label_part,
        border_h.to_string().repeat(remaining),
        corner_tr
    ));

    // Content
    let mut child_ctx = RenderContext::new();
    for stmt in statements {
        render_statement(stmt, &mut child_ctx, Some(inner_width));
    }

    for line in child_ctx.lines {
        let fitted = fit_to_width(&line, inner_width);
        ctx.push_line(&format!("{} {} {}", border_v, fitted, border_v));
    }

    // Bottom border
    ctx.push_line(&format!(
        "{}{}{}",
        corner_bl,
        border_h.to_string().repeat(inner_width + 2),
        corner_br
    ));
}

fn render_table(table: &Table, ctx: &mut RenderContext) {
    // Get column widths
    let col_widths: Vec<usize> = table.header.as_ref()
        .map(|h| h.columns.iter().map(|c| c.modifiers.width.unwrap_or(10) as usize).collect())
        .unwrap_or_else(|| vec![10]);

    let col_aligns: Vec<Align> = table.header.as_ref()
        .map(|h| h.columns.iter().map(|c| c.modifiers.align.unwrap_or(Align::Left)).collect())
        .unwrap_or_else(|| vec![Align::Left]);

    // Helper to render a row (uses visual width for proper Unicode support)
    let render_row_line = |cells: &[&str], widths: &[usize], aligns: &[Align]| -> String {
        let mut parts: Vec<String> = Vec::new();
        for (i, cell) in cells.iter().enumerate() {
            let w = widths.get(i).copied().unwrap_or(10);
            let align = aligns.get(i).copied().unwrap_or(Align::Left);
            let cell_width = display_width(cell);
            let truncated = if cell_width > w {
                truncate_to_width(cell, w)
            } else {
                cell.to_string()
            };
            let truncated_width = display_width(&truncated);
            let padding_needed = w.saturating_sub(truncated_width);
            let formatted = match align {
                Align::Left => format!("{}{}", truncated, " ".repeat(padding_needed)),
                Align::Center => {
                    let left = padding_needed / 2;
                    let right = padding_needed - left;
                    format!("{}{}{}", " ".repeat(left), truncated, " ".repeat(right))
                }
                Align::Right => format!("{}{}", " ".repeat(padding_needed), truncated),
            };
            parts.push(formatted);
        }
        format!("| {} |", parts.join(" | "))
    };

    // Helper for separator line (matches row format: +-col1-+-col2-+-col3-+)
    let separator = |widths: &[usize]| -> String {
        let parts: Vec<String> = widths.iter().map(|w| "-".repeat(*w)).collect();
        format!("+-{}-+", parts.join("-+-"))
    };

    // Top border
    ctx.push_line(&separator(&col_widths));

    // Header
    if let Some(header) = &table.header {
        let labels: Vec<&str> = header.columns.iter().map(|c| c.label.as_str()).collect();
        ctx.push_line(&render_row_line(&labels, &col_widths, &col_aligns));
        ctx.push_line(&separator(&col_widths));
    }

    // Rows
    for row in &table.rows {
        let cells: Vec<&str> = row.cells.iter().map(|c| c.content.as_str()).collect();
        ctx.push_line(&render_row_line(&cells, &col_widths, &col_aligns));
    }

    // Bottom border
    ctx.push_line(&separator(&col_widths));
}

fn render_list(list: &List, ctx: &mut RenderContext) {
    for item in &list.items {
        let prefix = if item.selected { " > " } else { "   " };
        ctx.push_line(&format!("{}{}", prefix, item.content));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_test(source: &str) -> String {
        render(source).unwrap()
    }

    // ========== Basic tests ==========

    #[test]
    fn empty_layout() {
        let output = render_test("layout { }");
        assert_eq!(output, "");
    }

    #[test]
    fn simple_text() {
        let output = render_test(r#"layout { text "Hello" }"#);
        assert_eq!(output, "Hello");
    }

    #[test]
    fn multiline_text() {
        let output = render_test(r#"layout { text "Line 1\nLine 2" }"#);
        assert!(output.contains("Line 1"));
        // Note: \n is literal in the string, not a newline
    }

    // ========== Input tests ==========

    #[test]
    fn input_basic() {
        let output = render_test("layout { input width:10 }");
        assert_eq!(output, "[________]");
    }

    #[test]
    fn input_with_placeholder() {
        let output = render_test(r#"layout { input width:15 placeholder:"email" }"#);
        assert_eq!(output, "[email________]");
    }

    // ========== Checkbox/Radio tests ==========

    #[test]
    fn checkbox_unchecked() {
        let output = render_test(r#"layout { checkbox "Enable" }"#);
        assert_eq!(output, "[ ] Enable");
    }

    #[test]
    fn checkbox_checked() {
        let output = render_test(r#"layout { checkbox "Enable" checked }"#);
        assert_eq!(output, "[x] Enable");
    }

    #[test]
    fn radio_unselected() {
        let output = render_test(r#"layout { radio "Option A" }"#);
        assert_eq!(output, "( ) Option A");
    }

    #[test]
    fn radio_selected() {
        let output = render_test(r#"layout { radio "Option A" selected }"#);
        assert_eq!(output, "(o) Option A");
    }

    // ========== Select tests ==========

    #[test]
    fn select_basic() {
        let output = render_test(r#"layout { select "Mono" width:10 }"#);
        assert_eq!(output, "[Mono    v]");
    }

    // ========== Button tests ==========

    #[test]
    fn button_basic() {
        let output = render_test(r#"layout { button "OK" }"#);
        assert_eq!(output, "[ OK ]");
    }

    #[test]
    fn button_cancel() {
        let output = render_test(r#"layout { button "Cancel" }"#);
        assert_eq!(output, "[ Cancel ]");
    }

    // ========== Link tests ==========

    #[test]
    fn link_basic() {
        let output = render_test(r#"layout { link "Click here" }"#);
        assert_eq!(output, "Click here");
    }

    // ========== Separator tests ==========

    #[test]
    fn separator_basic() {
        let output = render_test("layout { separator }");
        assert!(output.starts_with("-"));
        assert!(output.len() >= 10);
    }

    // ========== Progress tests ==========

    #[test]
    fn progress_zero() {
        let output = render_test("layout { progress 0 }");
        assert!(output.contains("░"));
        assert!(output.contains("0%"));
    }

    #[test]
    fn progress_fifty() {
        let output = render_test("layout { progress 50 }");
        assert!(output.contains("█"));
        assert!(output.contains("░"));
        assert!(output.contains("50%"));
    }

    #[test]
    fn progress_hundred() {
        let output = render_test("layout { progress 100 }");
        assert!(output.contains("█"));
        assert!(output.contains("100%"));
    }

    // ========== Row tests ==========

    #[test]
    fn row_simple() {
        let output = render_test(r#"layout { row { text "A" text "B" } }"#);
        assert!(output.contains("A"));
        assert!(output.contains("B"));
    }

    #[test]
    fn row_with_spacer() {
        let output = render_test(r#"layout {
            window width:30 {
                row { text "Left" spacer text "Right" }
            }
        }"#);
        assert!(output.contains("Left"));
        assert!(output.contains("Right"));
        // Spacer should create space between them
    }

    // ========== Section tests ==========

    #[test]
    fn section_header() {
        let output = render_test(r#"layout { section "Options" { text "Content" } }"#);
        assert!(output.contains("## Options"));
        assert!(output.contains("Content"));
    }

    #[test]
    fn section_indentation() {
        let output = render_test(r#"layout { section "Options" { text "Indented" } }"#);
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines[0], "## Options");
        assert!(lines[1].starts_with("  ")); // 2-space indent
    }

    // ========== Box/Window tests ==========

    #[test]
    fn window_empty() {
        let output = render_test(r#"layout { window "Test" { } }"#);
        assert!(output.contains("+--"));
        assert!(output.contains("Test"));
        assert!(output.contains("+"));
    }

    #[test]
    fn window_with_content() {
        let output = render_test(r#"layout {
            window "Title" {
                text "Hello"
            }
        }"#);
        assert!(output.contains("Title"));
        assert!(output.contains("Hello"));
        assert!(output.contains("|"));
    }

    #[test]
    fn box_nested() {
        let output = render_test(r#"layout {
            window "Outer" {
                box "Inner" {
                    text "Content"
                }
            }
        }"#);
        assert!(output.contains("Outer"));
        assert!(output.contains("Inner"));
        assert!(output.contains("Content"));
    }

    // ========== Code/Raw tests ==========

    #[test]
    fn code_single_line() {
        let output = render_test(r#"layout { code "fn main() {}" }"#);
        assert_eq!(output, "fn main() {}");
    }

    #[test]
    fn raw_ascii_art() {
        let output = render_test(r#"layout {
            raw ```
┌───┐
│ A │
└───┘
```
        }"#);
        assert!(output.contains("┌───┐"));
        assert!(output.contains("│ A │"));
        assert!(output.contains("└───┘"));
    }

    // ========== Alert tests ==========

    #[test]
    fn alert_error() {
        let output = render_test(r#"layout { alert type:error { text "Error!" } }"#);
        assert!(output.contains("ERROR"));
        assert!(output.contains("Error!"));
        assert!(output.contains("╔"));
        assert!(output.contains("╝"));
    }

    #[test]
    fn alert_warn() {
        let output = render_test(r#"layout { alert type:warn { text "Warning" } }"#);
        assert!(output.contains("WARNING"));
        assert!(output.contains("Warning"));
        assert!(output.contains("┌"));
        assert!(output.contains("┘"));
    }

    #[test]
    fn alert_info() {
        let output = render_test(r#"layout { alert type:info { text "Info" } }"#);
        assert!(output.contains("INFO"));
        assert!(output.contains("Info"));
    }

    // ========== Table tests ==========

    #[test]
    fn table_simple() {
        let output = render_test(r#"layout {
            table {
                header { col "Name" width:10 col "Age" width:5 }
                tr { td "Alice" td "30" }
                tr { td "Bob" td "25" }
            }
        }"#);
        assert!(output.contains("Name"));
        assert!(output.contains("Age"));
        assert!(output.contains("Alice"));
        assert!(output.contains("30"));
        assert!(output.contains("Bob"));
        assert!(output.contains("25"));
        assert!(output.contains("|"));
        assert!(output.contains("+"));
    }

    #[test]
    fn table_alignment() {
        let output = render_test(r#"layout {
            table {
                header { col "Value" width:10 align:right }
                tr { td "123" }
            }
        }"#);
        // Right-aligned "Value" and "123"
        assert!(output.contains("Value"));
        assert!(output.contains("123"));
    }

    // ========== List tests ==========

    #[test]
    fn list_simple() {
        let output = render_test(r#"layout {
            list {
                item "First"
                item "Second"
                item "Third"
            }
        }"#);
        assert!(output.contains("First"));
        assert!(output.contains("Second"));
        assert!(output.contains("Third"));
    }

    #[test]
    fn list_with_selection() {
        let output = render_test(r#"layout {
            list {
                item "A"
                item "B" selected
                item "C"
            }
        }"#);
        let lines: Vec<&str> = output.lines().collect();
        assert!(lines[0].contains("A"));
        assert!(lines[1].contains(">"));
        assert!(lines[1].contains("B"));
        assert!(lines[2].contains("C"));
    }

    // ========== Complex examples ==========

    #[test]
    fn login_form() {
        let output = render_test(r#"layout {
            window "Login" {
                row { text "Username:" input width:20 }
                row { text "Password:" input width:20 }
                separator
                row { spacer button "Cancel" button "Login" }
            }
        }"#);
        assert!(output.contains("Login"));
        assert!(output.contains("Username:"));
        assert!(output.contains("Password:"));
        assert!(output.contains("[ Cancel ]"));
        assert!(output.contains("[ Login ]"));
    }

    #[test]
    fn settings_panel() {
        let output = render_test(r#"layout {
            window "Preferences" width:45 {
                section "Appearance" {
                    checkbox "Dark mode" checked
                    checkbox "Line numbers"
                }
                separator
                row { spacer button "Apply" }
            }
        }"#);
        assert!(output.contains("Preferences"));
        assert!(output.contains("## Appearance"));
        assert!(output.contains("[x] Dark mode"));
        assert!(output.contains("[ ] Line numbers"));
        assert!(output.contains("[ Apply ]"));
    }

    #[test]
    fn build_status() {
        let output = render_test(r#"layout {
            window "Build" {
                text "Compiling..."
                progress 65
                alert type:warn {
                    text "Unused variable"
                }
            }
        }"#);
        assert!(output.contains("Build"));
        assert!(output.contains("Compiling..."));
        assert!(output.contains("65%"));
        assert!(output.contains("WARNING"));
        assert!(output.contains("Unused variable"));
    }
}
