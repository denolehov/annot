# Spec: Excalidraw Integration

## Goal

Add diagram drawing capability to annot via Excalidraw, using a bundled React island pattern for offline support. Users can insert diagrams via `/excalidraw` slash command and edit them by clicking.

## Design

### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  TipTap Editor                                              │
│  ├── ExcalidrawPlaceholder (transient, opens modal)         │
│  └── ExcalidrawChip (persistent, click to edit)             │
└───────────────────────┬─────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────────────────┐
│  ExcalidrawModal.svelte                                     │
│  └── <div> ← React mounts here via loader.ts                │
└───────────────────────┬─────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────────────────┐
│  excalidraw-loader.ts                                       │
│  - Lazy imports react-dom/client + @excalidraw/excalidraw   │
│  - Returns handle: { unmount, getElements, exportToPng }    │
└─────────────────────────────────────────────────────────────┘
```

### Data Model

**Frontend (TypeScript):**
```typescript
interface ExcalidrawNode {
  type: 'excalidraw';
  elements: string;      // JSON string of Excalidraw elements
  image?: string;        // base64 PNG for MCP export
}

type ContentNode = TextNode | TagNode | ExcalidrawNode;
```

**Backend (Rust):**
```rust
pub enum ContentNode {
    Text { text: String },
    Tag { id: String, name: String, instruction: String },
    Excalidraw { elements: String, image: Option<String> },
}
```

### User Flows

**Create diagram:**
1. Type `/excalidraw` in editor → inserts placeholder
2. Modal opens immediately, React mounts
3. Draw diagram, click "Save"
4. Placeholder replaced with ExcalidrawChip
5. Modal closes, React unmounts

**Edit diagram:**
1. Click existing ExcalidrawChip
2. Modal opens with existing elements loaded
3. Edit, click "Save"
4. Chip attrs updated via TipTap transaction
5. Modal closes

**Cancel/Dismiss:**
- New diagram: placeholder deleted
- Editing: no changes, modal closes

### Output Format

**CLI:**
```
file.rs:45-52:
  > [EXCALIDRAW]
  > {"elements": [...], "appState": {...}}
```

**MCP:** Base64 PNG included in response for visual representation.

## Decisions

- **Bundled React (not CDN)**: Enables fully offline operation, ~700KB added to bundle but lazy-loaded on first use
- **Click-to-edit only**: No keyboard shortcut for opening chips — click is sufficient for MVP
- **No dark mode sync**: Use Excalidraw's default theme for now
- **No PNG size limits**: Trust Excalidraw's defaults for export resolution
- **Placeholder pattern**: Transient node ensures clean UX if user cancels mid-creation

## Open Questions

None — all resolved in review.

## Scope

**In:**
- `ExcalidrawChip` TipTap node (persistent, stores elements + image)
- `ExcalidrawPlaceholder` TipTap node (transient, auto-opens modal)
- `ExcalidrawModal.svelte` component
- `excalidraw-loader.ts` for React mounting
- Slash command `/excalidraw` integration
- Backend `ContentNode::Excalidraw` variant
- Output formatting for CLI and MCP
- Font assets copied for offline use
- `window.EXCALIDRAW_ASSET_PATH` set for offline fonts

**Out:**
- Collaborative/real-time editing
- Image paste as separate chip type (future feature)
- Excalidraw templates/library
- Dark mode synchronization
- Keyboard navigation to open chips
