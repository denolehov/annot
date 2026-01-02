# asciiscript v1 Specification

A declarative DSL for agents to describe UI mockups. Compiles to ASCII box art.

---

## Grammar

```ebnf
program     = layout
layout      = "layout" block
block       = "{" statement* "}"
statement   = container | primitive | comment
comment     = "#" [^\n]*              (* must be on its own line *)

(* Containers *)
container   = (window | box | section | row | column) modifiers? block
window      = "window" STRING?
box         = "box" STRING?
section     = "section" STRING
row         = "row"
column      = "column"

(* Primitives *)
primitive   = text | input | checkbox | radio | select
            | button | link | code | raw
            | separator | spacer | progress | alert
            | table | list

text        = "text" STRING modifiers?
input       = "input" modifiers
checkbox    = "checkbox" STRING flag?
radio       = "radio" STRING flag?
select      = "select" STRING modifiers?
button      = "button" STRING modifiers?
link        = "link" STRING
code        = "code" (STRING | MULTILINE)
raw         = "raw" (STRING | MULTILINE)
separator   = "separator"
spacer      = "spacer"
progress    = "progress" NUMBER
alert       = "alert" modifiers block

(* Table — uses `tr`/`td` to avoid collision with layout `row` *)
table       = "table" modifiers? "{" header? tr* "}"
header      = "header" "{" col+ "}"
col         = "col" STRING modifiers
tr          = "tr" "{" td+ "}"
td          = "td" STRING modifiers?

(* List *)
list        = "list" "{" item+ "}"
item        = "item" STRING flag?

(* Modifiers = attrs + flags *)
modifiers   = (attr | flag)+
attr        = IDENT ":" VALUE
flag        = IDENT                     (* restricted to known set *)

(* Tokens *)
VALUE       = STRING | NUMBER | IDENT
STRING      = '"' [^"]* '"'
MULTILINE   = '```' (?:(?!```).)*  '```'   (* no ``` inside; no lang tag *)
NUMBER      = [0-9]+
IDENT       = [a-z_]+
```

---

## Typed Attributes

| Attribute | Type | Allowed On | Default |
|-----------|------|------------|---------|
| `width` | int >= 1 | window, box, column, col, input, select | auto |
| `height` | int >= 1 | box | auto |
| `padding` | int >= 0 | window, box | 1 |
| `gap` | int >= 0 | row, column | 1 for row, 0 for column |
| `align` | left/center/right | text, col, td | left |
| `style` | bold/dim/danger | text, button | — |
| `type` | error/warn/info | alert | — |
| `placeholder` | string | input | — |

Unknown attrs/flags MUST produce a parse error with line/column.

---

## Known Flags

| Primitive | Allowed Flags |
|-----------|---------------|
| `checkbox` | `checked` |
| `radio` | `selected` |
| `item` | `selected` |

---

## Width Definitions

Two width concepts:

| Term | Meaning |
|------|---------|
| **Outer width** | Total characters including borders |
| **Content width** | Inside borders, after padding |

Width semantics per element:

| Element | `width:N` means |
|---------|-----------------|
| `window`, `box` | Outer width |
| `column` | Allocated width for children (content width) |
| `col` (table) | Cell content width (excludes `\| ` chrome) |
| `input` | Outer width including `[ ]` brackets |
| `select` | Outer width including `[ v]` chrome |

**Character counting**: Each Unicode codepoint = 1 column. Box-drawing characters = 1 column. (Implementation may use wcwidth for accuracy.)

---

## Containers

| Container | Border | Children | Spacer Behavior |
|-----------|--------|----------|-----------------|
| `layout` | None | Vertical | — |
| `window "Title"` | `+--+` | Vertical | Vertical push |
| `box "Label"` | `+--+` | Vertical | Vertical push |
| `section "Title"` | None, 2-char indent | Vertical | Vertical push |
| `row` | None | Horizontal | Horizontal expand |
| `column` | None | Vertical | Vertical expand |

### Section

- Emits title line: `## {Title}`
- Children indented 2 spaces
- Title contributes to auto-width
- Only valid inside vertical contexts (layout/window/box/column)

---

## Layout Algorithm

### Width Resolution Order

1. **Explicit**: Element has `width:N`
2. **Inherited**: Parent has resolved width → child gets parent's content width
3. **Auto**: Expand to fit widest child

### Row Allocation

1. Determine parent content width (must be resolved for spacer expansion)
2. Compute each child's **natural width**:
   - text/link/checkbox/radio/button/select/input/progress/list/table/code/raw/alert → rendered width
   - column with `width` → that width
   - container without width → recursive auto-width
   - spacer → 0 initially
3. Add gaps between children
4. If parent width resolved and remaining space > 0: distribute to spacers equally
5. If remaining space < 0: truncate children left-to-right (see Overflow)

### Spacer Behavior

**Spacer only expands when parent has a resolved width.**

- **Resolved parent**: Spacers divide remaining space equally
- **Unresolved parent**: Spacer renders as single space (minimum visibility)

```asciiscript
# Resolved parent → spacer expands
window width:40 {
  row { text "L" spacer text "R" }
}
# Output: | L                                  R |

# Unresolved parent → spacer = 1 space
row { text "L" spacer text "R" }
# Output: L R
```

### Text Alignment

`align` positions text within the **allocated cell width**:

- In `row`: allocated width from row distribution
- In `column`: column's resolved width
- In `td`: column's `width` attr
- No resolved width: alignment is no-op

---

## Overflow & Truncation

**Global rule**: Any primitive rendered into an allocated width truncates with `…` when content exceeds available space.

- Minimum render width: 3 characters (1 char + `…` + 1 padding)
- Borders always rendered intact
- Truncation priority in rows (left-to-right): rightmost elements truncated first

---

## Primitives

### Text

```asciiscript
text "Hello"
text "Centered" align:center
text "Muted" style:dim
```

### Input

```asciiscript
input width:10
input width:15 placeholder:"email"
```

Output: `[__________]` or `[email__________]`

`width` is required and includes brackets.

### Checkbox / Radio

```asciiscript
checkbox "Enable feature"
checkbox "Dark mode" checked
radio "Option A"
radio "Option B" selected
```

Output: `[ ] Label`, `[x] Label`, `( ) Label`, `(o) Label`

### Select

```asciiscript
select "Current" width:12
```

Output: `[Current    v]`

### Button

```asciiscript
button "OK"
button "Cancel"
button "Delete" style:danger
```

Output: `[ OK ]`, `[ Cancel ]`, `[ Delete ]`

Brackets added automatically.

### Separator

```asciiscript
separator
```

Full-width horizontal rule: `|--------|`

### Spacer

```asciiscript
row { text "Left" spacer text "Right" }
```

See "Spacer Behavior" above.

### Progress

```asciiscript
progress 75
```

Output: `[████████████░░░░] 75%`

Bar scales to allocated width. Percentage appended.

### Code / Raw

```asciiscript
code "fn main() {}"

code ```
fn main() {
    println!("Hello");
}
```

raw ```
┌───┐
│ A │
└───┘
```
```

**Multiline rules:**
- Opening ``` must be at token boundary (no language tag)
- Leading/trailing newlines trimmed
- Internal whitespace preserved exactly
- Cannot contain ``` sequence inside

### Alert

```asciiscript
alert type:error {
  text "Build failed"
  text "src/main.rs:42"
}
```

| Type | Border | Header |
|------|--------|--------|
| `error` | Double (`╔═╗`) | `ERROR` |
| `warn` | Single (`┌─┐`) | `WARNING` |
| `info` | Single (`┌─┐`) | `INFO` |

Alert width: `min(parent_content_width, max(child_widths + chrome))` when parent resolved; otherwise auto-width.

---

## Table

```asciiscript
table {
  header { col "Name" width:15 col "Size" width:8 align:right }
  tr { td "main.rs" td "2.4 KB" }
  tr { td "lib.rs" td "1.1 KB" }
}
```

Output:
```
+-----------------+----------+
| Name            |     Size |
+-----------------+----------+
| main.rs         |   2.4 KB |
| lib.rs          |   1.1 KB |
+-----------------+----------+
```

- `col width` is required
- Cells padded with spaces to fill width
- Overflow truncated with `…`

---

## List

```asciiscript
list {
  item "First"
  item "Second" selected
  item "Third"
}
```

Output:
```
   First
 > Second
   Third
```

---

## Examples

### Login Form

```asciiscript
layout {
  window "Login" {
    row { text "Username:" input width:20 }
    row { text "Password:" input width:20 }
    separator
    row { spacer button "Cancel" button "Login" }
  }
}
```

```
+-- Login ----------------------------+
| Username: [____________________]    |
| Password: [____________________]    |
|-------------------------------------|
|             [ Cancel ] [ Login ]    |
+-------------------------------------+
```

### Settings Panel

```asciiscript
layout {
  window "Preferences" width:45 {
    section "Appearance" {
      row { text "Theme:" radio "Dark" selected radio "Light" }
      checkbox "Line numbers" checked
      checkbox "Word wrap"
    }
    section "Editor" {
      row { text "Tab size:" input width:4 placeholder:"4" }
      row { text "Font:" select "Mono" width:12 }
    }
    separator
    row { spacer button "Apply" }
  }
}
```

```
+-- Preferences ------------------------------+
| ## Appearance                               |
|   Theme: (o) Dark  ( ) Light                |
|   [x] Line numbers                          |
|   [ ] Word wrap                             |
| ## Editor                                   |
|   Tab size: [4___]                          |
|   Font: [Mono        v]                     |
|--------------------------------------------|
|                               [ Apply ]    |
+--------------------------------------------+
```

### Build Status

```asciiscript
layout {
  window "Build" width:35 {
    text "Compiling project..."
    progress 65
    separator
    alert type:warn {
      text "unused variable 'foo'"
      text "  --> src/main.rs:42"
    }
  }
}
```

```
+-- Build ----------------------------+
| Compiling project...               |
| [████████████░░░░] 65%             |
|-------------------------------------|
| ┌─ WARNING ───────────────────────┐|
| │ unused variable 'foo'           │|
| │   --> src/main.rs:42            │|
| └─────────────────────────────────┘|
+-------------------------------------+
```

---

## Rendering Summary

| Rule | Behavior |
|------|----------|
| Width resolution | Explicit → Inherited → Auto |
| Spacer | Expands with resolved parent; otherwise 1 space |
| Overflow | Truncate with `…`, min 3 chars |
| Padding | Default 1 for window/box |
| Gap | Default 1 for row, 0 for column |
| Borders | `+`, `-`, `|` standard; `╔`, `═`, `║` for error |
| Output | Single trailing newline |

---

## Scope

**In v1:**
- Containers: layout, window, box, section, row, column
- Primitives: text, input, checkbox, radio, select, button, link, code, raw, separator, spacer, progress, alert, table (tr/td), list
- Attributes: width, height, align, style, gap, padding, type, placeholder
- Flags: checked, selected

**Out v1:**
- tabs, breadcrumb, menu, steps, tree, diagram
- Percentage widths
- Text wrapping
- Border customization
- Vertical separator
- Conditional rendering
- `disabled` flag (reserved for v1.1)

---

## Implementation Notes

### Render Pipeline

1. **Parse**: Source → AST (with line/col for errors)
2. **Validate**: Type-check attrs, verify required attrs, check flag validity
3. **Measure**: Bottom-up natural width computation
4. **Resolve**: Top-down width assignment (explicit → inherited)
5. **Allocate**: Distribute space in rows, expand spacers
6. **Render**: Generate ASCII lines
7. **Clip**: Apply truncation where needed

### Error Handling

- Unknown attr/flag: Parse error with line/col
- Type mismatch (e.g., `width:abc`): Validation error
- Missing required attr (e.g., `input` without `width`): Validation error
- Invalid structure (e.g., `section` inside `row`): Validation error

### Future (v1.1 candidates)

- `wrap:true` on text
- `width:50%` percentage widths
- `vseparator` vertical separator
- `justify:space_between` for row
- `disabled` flag on inputs/buttons
- `id:"name"` for agent references
