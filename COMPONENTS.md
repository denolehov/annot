# Component Inventory

Quick reference for all UI components in annot. Use this to understand what exists before creating or modifying components.

---

## Core Components

### AnnotationEditor
The rich text editor for creating and editing annotations.

| Property | Value |
|----------|-------|
| **File** | `src/lib/AnnotationEditor.svelte` |
| **Styles** | `src/styles/components/annotation-editor.css` |
| **Triggers** | Click line gutter, edit existing annotation |

**Key States:**
- Empty (placeholder visible)
- Editing (cursor active)
- With tags (inline tag chips)
- With media (image/diagram chips)
- Sealed (read-only, solid border)

**Features:**
- TipTap-based rich text editing
- `/` triggers tag autocomplete (slash commands)
- `#` triggers inline tag suggestions
- Image paste handling (Cmd+V)
- Excalidraw diagram embedding
- Selection-based tag creation

---

### CommandPalette
Keyboard-driven modal for managing tags, exit modes, and utilities.

| Property | Value |
|----------|-------|
| **File** | `src/lib/CommandPalette/CommandPalette.svelte` |
| **Styles** | `src/styles/components/command-palette.css` |
| **Trigger** | `Ctrl+K` (or `Cmd+K` on macOS) |

**Namespaces:**
- `tags` — Create, edit, delete, reorder tags
- `exit-modes` — Manage exit mode buttons
- `copy` — Copy output in different formats
- `obsidian` — Obsidian plugin integration

**Key States:**
- Filter view (searching/selecting items)
- Form view (creating/editing an item)
- Reorder view (drag to reorder)

**Sub-components:**
- `Icon.svelte` — Renders icons for menu items

---

### CopyDropdown
Dropdown for selecting output format when copying annotations.

| Property | Value |
|----------|-------|
| **File** | `src/lib/CopyDropdown.svelte` |
| **Styles** | Inline `<style>` block |
| **Trigger** | Click copy button in header |

**Formats:**
- Structured text (default)
- JSON
- Markdown

---

### SaveModal
File save dialog for exporting annotations to disk.

| Property | Value |
|----------|-------|
| **File** | `src/lib/SaveModal.svelte` |
| **Styles** | Inline `<style>` block (uses tokens) |
| **Trigger** | Click save button in header |

**Key States:**
- Input (path entry)
- Saving (disabled, loading)
- Error (validation message)

---

### ExcalidrawModal
Full-screen modal for creating/editing Excalidraw diagrams.

| Property | Value |
|----------|-------|
| **File** | `src/lib/ExcalidrawModal.svelte` |
| **Styles** | Inline `<style>` block (uses tokens) |
| **Trigger** | Click Excalidraw chip in editor |

**Key States:**
- Loading (Excalidraw initializing)
- Ready (canvas active)
- Confirm discard (unsaved changes warning)

---

## Page Components

### Main Viewer (`+page.svelte`)
The primary view displaying file/diff content with annotations.

| Property | Value |
|----------|-------|
| **File** | `src/routes/+page.svelte` |
| **Styles** | Inline `<style>` block + global CSS |

**Responsibilities:**
- Renders line-by-line content
- Manages line selection (gutter clicks)
- Hosts annotation editors
- Displays exit mode bar
- Handles keyboard shortcuts (`g`, `Tab`, `Escape`)

---

### Layout (`+layout.svelte`)
Root layout wrapper for all pages.

| Property | Value |
|----------|-------|
| **File** | `src/routes/+layout.svelte` |

**Responsibilities:**
- Imports global styles (`src/styles/index.css`)
- Provides slot for page content

---

### Mermaid Viewer (`mermaid/+page.svelte`)
Separate page for rendering Mermaid diagrams.

| Property | Value |
|----------|-------|
| **File** | `src/routes/mermaid/+page.svelte` |

**Note:** Opens in separate window for diagram rendering.

---

## Style Dependencies

| Component | CSS File |
|-----------|----------|
| AnnotationEditor | `components/annotation-editor.css` |
| CommandPalette | `components/command-palette.css` |
| Tag/Media chips | `components/chips.css` |
| Code viewer | `components/code-viewer.css` |
| Status bar | `components/status-bar.css` |
| Keyboard hints | `components/kbd.css` |

---

## Adding a New Component

1. Create `.svelte` file in `src/lib/` (or subfolder for complex components)
2. If component needs styles beyond scoped `<style>`:
   - Create `src/styles/components/[name].css`
   - Add import to `src/styles/index.css`
3. Use design tokens from `tokens.css` — never hardcode colors
4. Update this file with component documentation
