# Spec: Structured Feedback Protocol (SFP)

## Goal

Define a standard format for structured human→agent feedback that enables auditable oversight, cross-tool interoperability, and machine-readable intent transfer — completing the human-AI collaboration stack alongside MCP (discovery) and A2UI (agent→human presentation).

---

## Design

### Protocol Position

```
┌─────────────────────────────────────────────────────────────┐
│  Human ↔ Agent Interface Stack                             │
├─────────────────────────────────────────────────────────────┤
│  MCP          │ Discovery & invocation layer               │
│  A2UI         │ Agent → Human (structured choice)          │
│  SFP          │ Human → Agent (structured judgment)        │
└─────────────────────────────────────────────────────────────┘
```

### Message Structure

```
SFP Message
├── Header        // Version + capabilities
├── Legend        // Tag definitions (id → instruction)
├── Session       // Context + exit signal
└── Annotations[] // Location-bound feedback
```

### Data Types

```
Header {
  sfp_version: string           // "1.0"
  capabilities: string[]        // ["named_location", "media_node"]
}

Location =
  | LineRange { path?: string, start: u32, end: u32 }
  | CharRange { start: u32, end: u32 }
  | Ref { uri: string, selector?: string }

Tag {
  id: string                    // Stable 12-char identifier
  name: string                  // Hierarchical: SECURITY.SQL_INJECTION
  instruction: string           // LLM-facing prompt text
  scope?: GLOBAL | ORG | PROJECT | SESSION
}

ContentNode =
  | Text { text: string }
  | Tag { id: string }
  | Media { mime: string, data: string }
  | Custom { namespace: string, type: string, data: any }

Annotation {
  location: Location
  content: ContentNode[]
}

ExitSignal {
  id: string                    // "apply", "reject", "revise"
  name: string                  // Display name
  instruction: string           // LLM-facing guidance
}

Session {
  context?: string              // High-level reviewer comment
  exit: ExitSignal
}
```

### Wire Format: Text (Primary)

```
SFP/1.0
Capabilities: named_location

LEGEND:
  [# TAG_NAME] instruction text

SESSION:
  Context: optional high-level comment
  Exit: ExitName
    Exit instruction text

---

path/file.ext:start-end:
    NN | source line
    > [# TAG] annotation text
    > additional text
```

### Wire Format: JSON (Secondary)

```json
{
  "sfp_version": "1.0",
  "capabilities": [],
  "legend": [{ "id": "...", "name": "...", "instruction": "..." }],
  "session": {
    "context": "...",
    "exit": { "id": "...", "name": "...", "instruction": "..." }
  },
  "annotations": [{
    "location": { "type": "line_range", "start": 45, "end": 52 },
    "content": [{ "type": "tag", "id": "..." }, { "type": "text", "text": "..." }]
  }]
}
```

**Canonical rule**: Text is authoritative. JSON must round-trip losslessly to text.

### Extension Mechanism

Custom content nodes use namespaced types:
```
{ "type": "custom", "namespace": "annot", "type": "excalidraw", "data": {...} }
```

Unknown namespaces are ignored (not rejected). Forward compatibility preserved.

### Versioning

- `sfp_version`: Major.minor, breaking changes increment major
- `capabilities`: Feature flags for optional extensions
- Consumers process known capabilities, ignore unknown

---

## Decisions

- **Text-first format**: LLMs parse text natively; JSON is for tooling. Text wins if formats diverge. Rationale: follows Markdown→AST, SQL→query plan precedent.

- **URI-based locations**: `Ref { uri, selector }` allows addressing non-code content (Figma nodes, form fields, UI states) without breaking parsers. Extensible by design.

- **Hierarchical tag names**: `SECURITY.SQL_INJECTION` enables filtering by prefix while maintaining flat ID lookup. Scopes (GLOBAL/ORG/PROJECT/SESSION) support organizational rollout.

- **Capabilities over versions**: Fine-grained feature flags prevent version fragmentation. Tools declare what they produce; consumers parse what they understand.

- **Companion to A2UI, not subordinate**: SFP is orthogonal — structured judgment vs. structured choice. They compose but neither depends on the other.

---

## Open Questions

- **Identity binding**: Lightweight auth for audit trails (OAuth? Signing keys? Anonymous?)
- **Immutability guarantees**: Append-only logs vs. convention-based
- **Consent model**: Per-session opt-in vs. global toggle for learning use cases
- **Governance**: Spec ownership (IETF RFC? Anthropic? Community consortium?)

---

## Scope

### In
- Core message structure (Header, Legend, Session, Annotations)
- Location types (LineRange, CharRange, Ref)
- Content nodes (Text, Tag, Media, Custom)
- Text and JSON wire formats
- Extension mechanism (namespaced custom types)
- Versioning strategy (version + capabilities)

### Out
- Transport layer (handled by MCP)
- UI/UX for annotation tools (implementation detail)
- Specific tag libraries (domain-specific, not protocol concern)
- Identity/auth implementation (noted as open question)
- Storage/persistence format (tool-specific)
- A2UI integration specifics (separate spec)