# Spec: Paste-as-Chip

## Goal
When users paste large content into an annotation editor, collapse it into an atomic chip that shows a preview on hover — keeping annotations readable while preserving the full pasted content.

## Design

### Threshold Heuristic

Focus on **screen real estate** — chip when content would visually dominate:

```typescript
export function shouldChip(text: string): boolean {
  if (!text) return false;
  
  const lines = text.split('\n');
  const lineCount = lines.length;
  const charCount = text.length;
  
  // Vertical sprawl: takes up too much height
  if (lineCount >= 6) return true;
  
  // Horizontal sprawl: massive single/double line (minified, tokens, URLs)
  if (lineCount <= 2 && charCount >= 400) return true;
  
  return false;
}
```

| Content Type     | Lines   | Chars   | Result   | Reason            |
| ---------------- | ------- | ------- | -------- | ----------------- |
| Code block       | 10      | 200     | CHIP     | Vertical sprawl   |
| Long paragraph   | 3       | 450     | INLINE   | Under thresholds  |
| Normal paragraph | 3       | 180     | INLINE   | Under thresholds  |
| Minified JSON    | 1       | 600     | CHIP     | Horizontal sprawl |
| Stack trace      | 20      | 1000    | CHIP     | Vertical sprawl   |
| Short URL        | 1       | 80      | INLINE   | Under thresholds  |
| Long URL         | 1       | 450     | CHIP     | Horizontal sprawl |
| 2 sentences      | 2       | 180     | INLINE   | Under thresholds  |

### Data Model

**TipTap node attributes:**
```typescript
{
  content: string,      // Full pasted text
  lineCount: number,    // For display label
}
```

**ContentNode output:**
```typescript
interface PasteNode {
  type: 'paste';
  content: string;
}
```

### Visual Design

**Chip display:**
```
┌─────────────────────┐
│ 📋 Pasted (8 lines) │
└─────────────────────┘
```

Label format:
- Multi-line: `📋 Pasted (N lines)`
- Single line: `📋 Pasted text`

**Hover preview:**
- Render in `<pre>` to preserve whitespace
- Max 10 lines, truncate with `+N more lines` if needed
- Monospace font
- Max-width: 500px

**Styling:**
- New variant: `.paste-chip`
- Color tint: blue-ish (like `.media-chip`)
- Follow existing chip base styles

### Paste Handler

Extend or modify existing `ImagePasteHandler` to also handle `text/plain`:

```typescript
// Pseudo-code
if (clipboardItem.type === 'text/plain') {
  const text = await clipboardItem.text();
  if (shouldChip(text)) {
    insertPasteChip(editor, text);
    return true; // consume event
  }
  return false; // let default paste handle
}
```

## Decisions

- **Screen real estate focus**: Density-based heuristics are fragile (code vs prose). Focus on vertical (≥6 lines) and horizontal (≥400 chars on ≤2 lines) sprawl.
- **No editable labels**: Labels are for human orientation only. LLMs receive the raw content. Keeping labels automatic reduces friction in an ephemeral tool.
- **No user override (for now)**: Undo-to-unwrap is a future improvement. For v1, thresholds are fixed.
- **`<pre>` for preview**: Preserve whitespace formatting in hover tooltip.

## Open Questions
(None — all questions resolved)

## Scope

**In:**
- `shouldChip()` threshold function
- `PasteChip` TipTap node (atomic, inline)
- `TextPasteHandler` extension (or extend `ImagePasteHandler`)
- Hover preview with Floating UI
- `.paste-chip` styling
- `PasteNode` ContentNode type
- `extractContentNodes()` integration
- Unit tests for threshold logic

**Out:**
- Undo to unwrap chip → inline text
- Smart labels (detect JSON, error logs, code language)
- Click to copy chip content
- Configurable thresholds
- Modifier key overrides