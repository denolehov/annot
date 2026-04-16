//! Snapshot tests for asciiscript using insta.
//!
//! Run `cargo insta review` to interactively accept/reject snapshots.

use crate::asciiscript::render;

// ========== Basic primitives ==========

#[test]
fn text_simple() {
    insta::assert_snapshot!(render(r#"layout { text "Hello, World!" }"#).unwrap());
}

#[test]
fn text_aligned() {
    insta::assert_snapshot!(render(r#"layout {
        window width:30 {
            text "Left" align:left
            text "Center" align:center
            text "Right" align:right
        }
    }"#).unwrap());
}

#[test]
fn input_variants() {
    insta::assert_snapshot!(render(r#"layout {
        input width:10
        input width:20 placeholder:"email@example.com"
    }"#).unwrap());
}

#[test]
fn checkbox_states() {
    insta::assert_snapshot!(render(r#"layout {
        checkbox "Unchecked option"
        checkbox "Checked option" checked
    }"#).unwrap());
}

#[test]
fn radio_states() {
    insta::assert_snapshot!(render(r#"layout {
        radio "Option A"
        radio "Option B" selected
        radio "Option C"
    }"#).unwrap());
}

#[test]
fn select_dropdown() {
    insta::assert_snapshot!(render(r#"layout {
        select "Mono" width:12
        select "Very Long Option Name" width:12
    }"#).unwrap());
}

#[test]
fn buttons() {
    insta::assert_snapshot!(render(r#"layout {
        button "OK"
        button "Cancel"
        button "Submit Form"
    }"#).unwrap());
}

#[test]
fn progress_bars() {
    insta::assert_snapshot!(render(r#"layout {
        progress 0
        progress 25
        progress 50
        progress 75
        progress 100
    }"#).unwrap());
}

#[test]
fn separator() {
    insta::assert_snapshot!(render(r#"layout {
        window width:30 {
            text "Above"
            separator
            text "Below"
        }
    }"#).unwrap());
}

// ========== Containers ==========

#[test]
fn window_titled() {
    insta::assert_snapshot!(render(r#"layout {
        window "My Application" {
            text "Window content here"
        }
    }"#).unwrap());
}

#[test]
fn window_explicit_width() {
    insta::assert_snapshot!(render(r#"layout {
        window "Fixed Width" width:40 {
            text "Content"
        }
    }"#).unwrap());
}

#[test]
fn box_titled() {
    insta::assert_snapshot!(render(r#"layout {
        box "Settings" {
            text "Box content"
        }
    }"#).unwrap());
}

#[test]
fn section_with_content() {
    insta::assert_snapshot!(render(r#"layout {
        section "General" {
            checkbox "Option 1"
            checkbox "Option 2" checked
        }
    }"#).unwrap());
}

#[test]
fn row_layout() {
    insta::assert_snapshot!(render(r#"layout {
        row { text "Label:" input width:15 }
    }"#).unwrap());
}

#[test]
fn row_with_spacer() {
    insta::assert_snapshot!(render(r#"layout {
        window width:40 {
            row { text "Left" spacer text "Right" }
        }
    }"#).unwrap());
}

#[test]
fn row_buttons_spaced() {
    insta::assert_snapshot!(render(r#"layout {
        window width:40 {
            row { spacer button "Cancel" button "OK" }
        }
    }"#).unwrap());
}

#[test]
fn nested_containers() {
    insta::assert_snapshot!(render(r#"layout {
        window "Outer" {
            box "Inner" {
                text "Deeply nested"
            }
        }
    }"#).unwrap());
}

// ========== Alerts ==========

#[test]
fn alert_error() {
    insta::assert_snapshot!(render(r#"layout {
        alert type:error {
            text "Something went wrong!"
        }
    }"#).unwrap());
}

#[test]
fn alert_warning() {
    insta::assert_snapshot!(render(r#"layout {
        alert type:warn {
            text "Proceed with caution"
        }
    }"#).unwrap());
}

#[test]
fn alert_info() {
    insta::assert_snapshot!(render(r#"layout {
        alert type:info {
            text "Here's some information"
        }
    }"#).unwrap());
}

// ========== Tables ==========

#[test]
fn table_basic() {
    insta::assert_snapshot!(render(r#"layout {
        table {
            header { col "Name" width:12 col "Age" width:5 }
            tr { td "Alice" td "30" }
            tr { td "Bob" td "25" }
            tr { td "Charlie" td "35" }
        }
    }"#).unwrap());
}

#[test]
fn table_aligned_columns() {
    insta::assert_snapshot!(render(r#"layout {
        table {
            header {
                col "Item" width:15 align:left
                col "Qty" width:5 align:center
                col "Price" width:10 align:right
            }
            tr { td "Widget" td "10" td "$99.00" }
            tr { td "Gadget" td "5" td "$149.50" }
        }
    }"#).unwrap());
}

// ========== Lists ==========

#[test]
fn list_basic() {
    insta::assert_snapshot!(render(r#"layout {
        list {
            item "First item"
            item "Second item"
            item "Third item"
        }
    }"#).unwrap());
}

#[test]
fn list_with_selection() {
    insta::assert_snapshot!(render(r#"layout {
        list {
            item "Option A"
            item "Option B" selected
            item "Option C"
        }
    }"#).unwrap());
}

// ========== Code and Raw ==========

#[test]
fn code_block() {
    insta::assert_snapshot!(render(r#"layout {
        code ```
fn main() {
    println!("Hello, world!");
}
```
    }"#).unwrap());
}

#[test]
fn raw_ascii_art() {
    insta::assert_snapshot!(render(r#"layout {
        raw ```
┌─────────────┐
│  ASCII Art  │
└─────────────┘
```
    }"#).unwrap());
}

// ========== Complex layouts ==========

#[test]
fn login_form() {
    insta::assert_snapshot!(render(r#"layout {
        window "Login" width:40 {
            row { text "Username:" input width:20 }
            row { text "Password:" input width:20 }
            spacer
            separator
            row { spacer button "Cancel" button "Login" }
        }
    }"#).unwrap());
}

#[test]
fn settings_panel() {
    insta::assert_snapshot!(render(r#"layout {
        window "Preferences" width:45 {
            section "Appearance" {
                row { text "Theme:" radio "Dark" selected radio "Light" }
                checkbox "Show line numbers" checked
                checkbox "Word wrap"
            }
            section "Editor" {
                row { text "Tab size:" input width:5 placeholder:"4" }
                row { text "Font:" select "Monospace" width:15 }
            }
            separator
            row { spacer button "Cancel" button "Apply" }
        }
    }"#).unwrap());
}

#[test]
fn build_status_dialog() {
    insta::assert_snapshot!(render(r#"layout {
        window "Build Status" width:40 {
            text "Compiling project..."
            progress 65
            separator
            alert type:warn {
                text "unused variable `x`"
            }
            spacer
            row { spacer button "Cancel" }
        }
    }"#).unwrap());
}

#[test]
fn data_table_with_actions() {
    insta::assert_snapshot!(render(r#"layout {
        window "Users" width:50 {
            table {
                header {
                    col "ID" width:5 align:right
                    col "Name" width:15
                    col "Email" width:20
                }
                tr { td "1" td "Alice" td "alice@example.com" }
                tr { td "2" td "Bob" td "bob@example.com" }
                tr { td "3" td "Charlie" td "charlie@ex.com" }
            }
            separator
            row { text "3 users" spacer button "Add" button "Delete" }
        }
    }"#).unwrap());
}

#[test]
fn confirmation_dialog() {
    insta::assert_snapshot!(render(r#"layout {
        window "Confirm Delete" width:35 {
            alert type:error {
                text "This action cannot be undone!"
            }
            spacer
            text "Delete 5 selected files?"
            spacer
            row { spacer button "Cancel" button "Delete" style:danger }
        }
    }"#).unwrap());
}

#[test]
fn file_browser() {
    insta::assert_snapshot!(render(r#"layout {
        window "Open File" width:40 {
            list {
                item "Documents" selected
                item "Downloads"
                item "Pictures"
                item "Videos"
            }
            separator
            row { text "File:" input width:20 }
            row { spacer button "Cancel" button "Open" }
        }
    }"#).unwrap());
}

// ========== Edge cases ==========

#[test]
fn empty_layout() {
    insta::assert_snapshot!(render(r#"layout { }"#).unwrap());
}

#[test]
fn empty_window() {
    insta::assert_snapshot!(render(r#"layout { window "Empty" { } }"#).unwrap());
}

#[test]
fn empty_row() {
    insta::assert_snapshot!(render(r#"layout { row { } }"#).unwrap());
}

#[test]
fn empty_section() {
    insta::assert_snapshot!(render(r#"layout { section "Empty Section" { } }"#).unwrap());
}

#[test]
fn minimal_input() {
    insta::assert_snapshot!(render(r#"layout { input width:3 }"#).unwrap());
}

#[test]
fn minimal_table() {
    insta::assert_snapshot!(render(r#"layout {
        table {
            header { col "X" width:1 }
            tr { td "1" }
        }
    }"#).unwrap());
}

#[test]
fn single_item_list() {
    insta::assert_snapshot!(render(r#"layout {
        list { item "Only one" }
    }"#).unwrap());
}

#[test]
fn text_with_newlines() {
    insta::assert_snapshot!(render(r#"layout { text "Line one\nLine two\nLine three" }"#).unwrap());
}

#[test]
fn very_long_text() {
    insta::assert_snapshot!(render(r#"layout {
        window width:30 {
            text "This is a very long piece of text that should overflow the window boundaries"
        }
    }"#).unwrap());
}

#[test]
fn very_long_button_label() {
    insta::assert_snapshot!(render(r#"layout { button "This is an extremely long button label" }"#).unwrap());
}

#[test]
fn truncated_select_value() {
    insta::assert_snapshot!(render(r#"layout { select "This value is way too long to fit" width:15 }"#).unwrap());
}

#[test]
fn unicode_in_text() {
    insta::assert_snapshot!(render(r#"layout { text "Hello 世界! 🌍 Привет мир!" }"#).unwrap());
}

#[test]
fn unicode_box_alignment() {
    // Tests that boxes with CJK (width 2 per char) and emoji align correctly
    insta::assert_snapshot!(render(r#"layout {
        window "日本語タイトル" width:40 {
            box "设置 ⚙️" {
                text "你好世界"
                text "Hello World"
                checkbox "启用 🔔 通知" checked
            }
            separator
            table {
                header {
                    col "名前" width:10
                    col "Status" width:10
                }
                tr { td "田中太郎" td "✅ Active" }
                tr { td "Alice" td "❌ Inactive" }
                tr { td "北京" td "🔄 Pending" }
            }
            row { spacer button "取消" button "确定 ✓" }
        }
    }"#).unwrap());
}

#[test]
fn special_chars_in_labels() {
    insta::assert_snapshot!(render(r#"layout {
        button "Save & Exit"
        checkbox "Enable \"quotes\" option"
        text "100% complete"
    }"#).unwrap());
}

// ========== Deep nesting ==========

#[test]
fn deeply_nested_boxes() {
    insta::assert_snapshot!(render(r#"layout {
        window "Level 1" {
            box "Level 2" {
                box "Level 3" {
                    box "Level 4" {
                        text "Deep inside"
                    }
                }
            }
        }
    }"#).unwrap());
}

#[test]
fn nested_sections() {
    insta::assert_snapshot!(render(r#"layout {
        section "Outer" {
            text "Outer content"
            section "Inner" {
                text "Inner content"
                section "Deep" {
                    text "Deep content"
                }
            }
        }
    }"#).unwrap());
}

#[test]
fn complex_nested_rows() {
    insta::assert_snapshot!(render(r#"layout {
        window width:60 {
            row {
                column {
                    text "Left Column"
                    checkbox "Option A"
                    checkbox "Option B"
                }
                column {
                    text "Right Column"
                    radio "Choice 1"
                    radio "Choice 2" selected
                }
            }
        }
    }"#).unwrap());
}

// ========== Multiple spacers ==========

#[test]
fn multiple_spacers_in_row() {
    insta::assert_snapshot!(render(r#"layout {
        window width:50 {
            row { text "A" spacer text "B" spacer text "C" }
        }
    }"#).unwrap());
}

#[test]
fn spacer_at_start() {
    insta::assert_snapshot!(render(r#"layout {
        window width:40 {
            row { spacer button "Right-aligned" }
        }
    }"#).unwrap());
}

#[test]
fn spacer_at_end() {
    insta::assert_snapshot!(render(r#"layout {
        window width:40 {
            row { button "Left-aligned" spacer }
        }
    }"#).unwrap());
}

#[test]
fn only_spacers() {
    insta::assert_snapshot!(render(r#"layout {
        window width:30 {
            row { spacer spacer spacer }
        }
    }"#).unwrap());
}

// ========== Large tables ==========

#[test]
fn wide_table() {
    insta::assert_snapshot!(render(r#"layout {
        table {
            header {
                col "ID" width:5
                col "First Name" width:12
                col "Last Name" width:12
                col "Email" width:25
                col "Status" width:10
            }
            tr { td "1" td "Alice" td "Johnson" td "alice.j@example.com" td "Active" }
            tr { td "2" td "Bob" td "Smith" td "bob.smith@example.com" td "Inactive" }
            tr { td "3" td "Charlie" td "Brown" td "charlie.b@company.org" td "Pending" }
        }
    }"#).unwrap());
}

#[test]
fn tall_table() {
    insta::assert_snapshot!(render(r#"layout {
        table {
            header { col "Row" width:5 col "Value" width:15 }
            tr { td "1" td "First" }
            tr { td "2" td "Second" }
            tr { td "3" td "Third" }
            tr { td "4" td "Fourth" }
            tr { td "5" td "Fifth" }
            tr { td "6" td "Sixth" }
            tr { td "7" td "Seventh" }
            tr { td "8" td "Eighth" }
            tr { td "9" td "Ninth" }
            tr { td "10" td "Tenth" }
        }
    }"#).unwrap());
}

// ========== Alerts with content ==========

#[test]
fn alert_with_multiple_lines() {
    insta::assert_snapshot!(render(r#"layout {
        alert type:error {
            text "Error: Operation failed"
            text "Reason: Network timeout"
            text "Action: Please retry"
        }
    }"#).unwrap());
}

#[test]
fn alert_with_code() {
    insta::assert_snapshot!(render(r#"layout {
        alert type:warn {
            text "Deprecation warning:"
            code "use_old_api() -> use_new_api()"
        }
    }"#).unwrap());
}

#[test]
fn stacked_alerts() {
    insta::assert_snapshot!(render(r#"layout {
        alert type:error { text "Critical error!" }
        alert type:warn { text "Warning message" }
        alert type:info { text "Information note" }
    }"#).unwrap());
}

// ========== Complex forms ==========

#[test]
fn registration_form() {
    insta::assert_snapshot!(render(r#"layout {
        window "Create Account" width:50 {
            section "Personal Information" {
                row { text "First name:" input width:20 }
                row { text "Last name:" input width:20 }
                row { text "Email:" input width:30 placeholder:"you@example.com" }
            }
            section "Password" {
                row { text "Password:" input width:25 }
                row { text "Confirm:" input width:25 }
                text "Password must be at least 8 characters" style:dim
            }
            section "Preferences" {
                checkbox "Subscribe to newsletter"
                checkbox "Accept terms and conditions"
            }
            separator
            row { spacer button "Cancel" button "Create Account" }
        }
    }"#).unwrap());
}

#[test]
fn search_filters() {
    insta::assert_snapshot!(render(r#"layout {
        window "Search" width:45 {
            row { text "Query:" input width:30 button "Search" }
            separator
            section "Filters" {
                row { text "Category:" select "All" width:15 }
                row { text "Date:" select "Any time" width:15 }
                checkbox "Include archived" checked
                checkbox "Exact match"
            }
            separator
            row { text "0 results" spacer button "Clear" button "Apply" }
        }
    }"#).unwrap());
}

#[test]
fn wizard_dialog() {
    insta::assert_snapshot!(render(r#"layout {
        window "Setup Wizard - Step 2 of 4" width:50 {
            progress 50
            separator
            section "Select Installation Type" {
                radio "Standard (recommended)" selected
                radio "Custom"
                radio "Minimal"
            }
            spacer
            text "Standard installation includes all features."
            separator
            row { button "Back" spacer button "Next" }
        }
    }"#).unwrap());
}

// ========== Stress tests ==========

#[test]
fn many_rows() {
    insta::assert_snapshot!(render(r#"layout {
        window width:40 {
            row { text "Row 1" input width:10 }
            row { text "Row 2" input width:10 }
            row { text "Row 3" input width:10 }
            row { text "Row 4" input width:10 }
            row { text "Row 5" input width:10 }
            row { text "Row 6" input width:10 }
            row { text "Row 7" input width:10 }
            row { text "Row 8" input width:10 }
            row { text "Row 9" input width:10 }
            row { text "Row 10" input width:10 }
        }
    }"#).unwrap());
}

#[test]
fn many_buttons_in_row() {
    insta::assert_snapshot!(render(r#"layout {
        row {
            button "A"
            button "B"
            button "C"
            button "D"
            button "E"
            button "F"
            button "G"
        }
    }"#).unwrap());
}

#[test]
fn many_checkboxes() {
    insta::assert_snapshot!(render(r#"layout {
        window "Options" {
            checkbox "Option 1" checked
            checkbox "Option 2"
            checkbox "Option 3" checked
            checkbox "Option 4"
            checkbox "Option 5" checked
            checkbox "Option 6"
            checkbox "Option 7"
            checkbox "Option 8" checked
            checkbox "Option 9"
            checkbox "Option 10"
        }
    }"#).unwrap());
}

#[test]
fn long_list() {
    insta::assert_snapshot!(render(r#"layout {
        list {
            item "Item 1"
            item "Item 2"
            item "Item 3"
            item "Item 4"
            item "Item 5" selected
            item "Item 6"
            item "Item 7"
            item "Item 8"
            item "Item 9"
            item "Item 10"
            item "Item 11"
            item "Item 12"
        }
    }"#).unwrap());
}

// ========== Mixed content ==========

#[test]
fn dashboard() {
    insta::assert_snapshot!(render(r#"layout {
        window "Dashboard" width:60 {
            row {
                box "Stats" {
                    text "Users: 1,234"
                    text "Active: 567"
                    text "New: 89"
                }
                box "Health" {
                    progress 85
                    text "System OK" style:bold
                }
            }
            separator
            section "Recent Activity" {
                table {
                    header {
                        col "Time" width:10
                        col "User" width:15
                        col "Action" width:20
                    }
                    tr { td "10:30" td "alice" td "Login" }
                    tr { td "10:25" td "bob" td "Upload file" }
                    tr { td "10:20" td "charlie" td "Edit profile" }
                }
            }
            row { spacer button "Refresh" button "Settings" }
        }
    }"#).unwrap());
}

#[test]
fn error_dialog_with_details() {
    insta::assert_snapshot!(render(r#"layout {
        window "Error" width:50 {
            alert type:error {
                text "Failed to save document"
            }
            separator
            section "Details" {
                code "Error: ENOSPC - No space left on device"
                text "Location: /home/user/documents/file.txt"
            }
            separator
            row { checkbox "Show technical details" spacer button "OK" }
        }
    }"#).unwrap());
}

#[test]
fn mixed_controls_row() {
    insta::assert_snapshot!(render(r#"layout {
        window width:60 {
            row {
                text "Status:"
                checkbox "Active" checked
                select "Priority" width:12
                button "Save"
                button "Cancel"
            }
        }
    }"#).unwrap());
}

// ========== Padding and gaps ==========

#[test]
fn window_no_padding() {
    insta::assert_snapshot!(render(r#"layout {
        window "No Padding" padding:0 {
            text "Tight fit"
        }
    }"#).unwrap());
}

#[test]
fn window_large_padding() {
    insta::assert_snapshot!(render(r#"layout {
        window "Large Padding" padding:3 {
            text "Spacious"
        }
    }"#).unwrap());
}

#[test]
fn row_custom_gap() {
    insta::assert_snapshot!(render(r#"layout {
        row gap:5 { text "A" text "B" text "C" }
    }"#).unwrap());
}

#[test]
fn row_no_gap() {
    insta::assert_snapshot!(render(r#"layout {
        row gap:0 { button "A" button "B" button "C" }
    }"#).unwrap());
}

// ========== Grid / Masonry layouts ==========

#[test]
fn grid_simple_2x2() {
    // Basic 2x2 grid of equal-sized boxes
    insta::assert_snapshot!(render(r#"layout {
        window {
            row gap:0 { box "A" width:21 {} box "B" width:21 {} }
            row gap:0 { box "C" width:21 {} box "D" width:21 {} }
        }
    }"#).unwrap());
}

#[test]
fn grid_masonry_3_row() {
    // Masonry layout: varying column widths across rows
    // Row 1: F, G (2 equal)
    // Row 2: C, D, E (3 columns, D wider)
    // Row 3: A, B (2 equal)
    insta::assert_snapshot!(render(r#"layout {
        window {
            row gap:0 {
                box "F" width:25 { text "Panel F" }
                box "G" width:25 { text "Panel G" }
            }
            row gap:0 {
                box "C" width:15 { text "C" }
                box "D" width:20 { text "D" }
                box "E" width:15 { text "E" }
            }
            row gap:0 {
                box "A" width:25 { text "Panel A" }
                box "B" width:25 { text "Panel B" }
            }
        }
    }"#).unwrap());
}

#[test]
fn grid_dashboard_complex() {
    // Complex dashboard grid with nested content
    insta::assert_snapshot!(render(r#"layout {
        window "Dashboard" {
            row gap:0 {
                box "Metrics" width:30 {
                    text "Users: 1,234"
                    text "Sessions: 5,678"
                    progress 75
                }
                box "Status" width:30 {
                    text "API: Online" style:bold
                    text "DB: Online" style:bold
                    text "Cache: Degraded" style:danger
                }
            }
            row gap:0 {
                box "A" width:20 { text "Widget A" }
                box "B" width:20 { text "Widget B" }
                box "C" width:20 { text "Widget C" }
            }
            row gap:0 {
                box "Logs" width:40 {
                    code "[10:30] Request received"
                    code "[10:31] Processing..."
                }
                box "Quick Actions" width:20 {
                    button "Refresh"
                    button "Export"
                }
            }
        }
    }"#).unwrap());
}

// ========== Code blocks ==========

#[test]
fn multiline_code() {
    insta::assert_snapshot!(render(r#"layout {
        code ```
fn factorial(n: u64) -> u64 {
    match n {
        0 | 1 => 1,
        _ => n * factorial(n - 1),
    }
}
```
    }"#).unwrap());
}

#[test]
fn code_in_box() {
    insta::assert_snapshot!(render(r#"layout {
        box "Example" {
            code ```
let x = 42;
println!("{}", x);
```
        }
    }"#).unwrap());
}

// ========== Real-world scenarios ==========

#[test]
fn git_commit_dialog() {
    insta::assert_snapshot!(render(r#"layout {
        window "Commit Changes" width:55 {
            section "Changed Files" {
                list {
                    item "src/main.rs" selected
                    item "src/lib.rs"
                    item "Cargo.toml"
                }
            }
            separator
            text "Commit message:"
            input width:45 placeholder:"Enter commit message..."
            separator
            checkbox "Amend previous commit"
            checkbox "Sign commit"
            separator
            row { button "Cancel" spacer button "Commit" }
        }
    }"#).unwrap());
}

#[test]
fn package_manager() {
    insta::assert_snapshot!(render(r#"layout {
        window "Package Manager" width:60 {
            row { text "Search:" input width:35 button "Search" }
            separator
            table {
                header {
                    col "Package" width:20
                    col "Version" width:10
                    col "Status" width:12
                }
                tr { td "react" td "18.2.0" td "Installed" }
                tr { td "typescript" td "5.0.0" td "Outdated" }
                tr { td "webpack" td "5.88.0" td "Available" }
            }
            separator
            row { text "3 packages" spacer button "Update All" button "Install" }
        }
    }"#).unwrap());
}

#[test]
fn database_query_tool() {
    insta::assert_snapshot!(render(r#"layout {
        window "SQL Query" width:65 {
            text "Enter query:"
            code ```
SELECT id, name, email
FROM users
WHERE active = true
ORDER BY name;
```
            separator
            alert type:info { text "Query executed in 0.023s" }
            table {
                header {
                    col "id" width:5 align:right
                    col "name" width:15
                    col "email" width:25
                }
                tr { td "1" td "Alice" td "alice@example.com" }
                tr { td "2" td "Bob" td "bob@example.com" }
            }
            separator
            row { text "2 rows" spacer button "Export" button "Run" }
        }
    }"#).unwrap());
}

// ========== Sample.md examples ==========

#[test]
fn sample_login_form() {
    insta::assert_snapshot!(render(r#"layout {
  window "Login" width:40 {
    section "Credentials" {
      row { text "Username:" input width:20 }
      row { text "Password:" input width:20 placeholder:"********" }
    }
    separator
    row gap:2 {
      checkbox "Remember me"
      spacer
      button "Cancel"
      button "Sign In" style:bold
    }
  }
}"#).unwrap());
}

#[test]
fn sample_dashboard() {
    insta::assert_snapshot!(render(r#"layout {
  window "Dashboard" width:60 {
    section "Build Status" {
      row {
        text "main"
        progress 100
        text "passing" style:bold
      }
      row {
        text "develop"
        progress 75
        text "building..."
      }
      row {
        text "feature/auth"
        progress 30
        text "failing" style:danger
      }
    }
    separator
    section "Quick Actions" {
      row gap:2 {
        button "New Build"
        button "Settings"
        spacer
        link "View Logs"
      }
    }
    alert type:info { text "Last deploy: 2 hours ago" }
  }
}"#).unwrap());
}

#[test]
fn sample_export_dialog() {
    insta::assert_snapshot!(render(r#"layout {
  window "Export Options" width:45 {
    section "Format" {
      row { text "Type:" select "PDF Document" width:25 }
      row { text "Quality:" select "High (300 DPI)" width:25 }
    }
    separator
    section "Destination" {
      row {
        radio "Local file" selected
      }
      row {
        radio "Cloud storage"
      }
      row {
        radio "Email attachment"
      }
    }
    separator
    row gap:2 {
      spacer
      button "Cancel"
      button "Export" style:bold
    }
  }
}"#).unwrap());
}
