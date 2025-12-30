# Concept: Invisible Enrichment

> Status: Conceptual — saved for future exploration

## The Idea

AI sends content + metadata sideband. The metadata guides human attention without cluttering the visible document.

```
review_content(
  content: "## Architecture\nUse microservices...",
  enrichments: [
    { type: "alternatives", heading: "## Architecture", options: ["Monolith", "Serverless"] },
    { type: "uncertain", lines: [45, 67], reason: "I'm guessing here" },
    { type: "worth_revisiting", lines: [80-95], reason: "Potential feature idea" }
  ]
)
```

## Why This Matters

The AI's metacognition becomes visible without feeling "pre-annotated." The human's first impression is clean content; the enrichments surface contextually.

## Enrichment Types (Brainstorm)

| Type | AI signals | UI surfaces as |
|------|-----------|----------------|
| `alternatives` | "There are other options here" | Branch icon on heading |
| `uncertain` | "I'm not confident about this" | Faint background tint |
| `worth_revisiting` | "This might be worth bookmarking" | Subtle bookmark suggestion on hover |
| `attention` | "This is a key decision point" | Gutter highlight |
| `suggested_tags` | "If you annotate here, consider these tags" | Pre-populated tag suggestions |

## Visibility Model Options

1. **Ghost layer** — invisible by default, subtle indicators that something exists, reveal on hover
2. **Ambient presence** — subtle visual hints always visible (background tint, gutter dots)
3. **On-demand** — toggle: "Show AI hints" / "Clean view"
4. **Contextual** — surface only when relevant (e.g., suggested tags appear when annotating that line)

Lean toward **contextual surfacing** with **ghost indicators**.

## Design Questions

1. **Manipulation tension** — AI guides attention. Is this helpful or paternalistic?
2. **Output behavior** — Do enrichments appear in output? Only if interacted with?
3. **Trust/transparency** — Should user see that AI provided enrichments?

## API Shape (Sketch)

```typescript
interface Enrichment {
  type: "alternatives" | "uncertain" | "worth_revisiting" | "attention" | "suggested_tags";
  anchor: LineAnchor | HeadingAnchor | RangeAnchor;
  // Type-specific payload
}

// MCP tool signature extension
review_content(
  content: string,
  label: string,
  exit_modes?: ExitMode[],
  enrichments?: Enrichment[]  // NEW
)
```

## Connection to Manifesto

From the directional model:
- **AI → Human**: Tools that help humans comprehend what AI presents
- Enrichments fit here: portals, highlights, callouts, diagrams... and now **sideband metadata**

The enrichment is the AI saying "here's where your attention might matter most" without dictating what the human should think.

---

*Saved: 2024-12-30. Revisit when ready to implement.*
