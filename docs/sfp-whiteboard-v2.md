# Structured Feedback Protocol (SFP) — v2

> "If A2UI is how agents *ask*, SFP is how humans *answer precisely*."

---

## The Protocol Stack

```
┌─────────────────────────────────────────────────────────────┐
│  Complete Human ↔ Agent Interface                          │
├────────────────────────┬────────────────────────────────────┤
│  Agent → Human         │  Human → Agent                     │
│  ─────────────         │  ─────────────                     │
│  A2UI                  │  SFP                               │
│  • Structured choice   │  • Structured judgment             │
│  • Dynamic UI gen      │  • Precise feedback                │
│  • Component catalog   │  • Semantic metadata               │
├────────────────────────┴────────────────────────────────────┤
│  MCP (Model Context Protocol)                               │
│  • Discovery & invocation layer                             │
│  • A2UI: how tools present    SFP: what tools return        │
└─────────────────────────────────────────────────────────────┘
```

---

## What SFP Enables (Oracle Synthesis)

### 1. Auditable Human Oversight
- **Regulatory readiness**: EU AI Act Article 14 mandates records of oversight
- **Evidence generation**: Each annotation is a verifiable human assertion
- **Requirements**: Cryptographic timestamps, identity binding, immutability

### 2. Reproducible Reviews  
- Not deterministic outputs — *auditable methodology*
- Same tag set + instructions = comparable coverage across reviewers
- Enables cross-agent handoffs, agent self-review loops

### 3. Human-in-the-Loop Learning
- Tags = labeled training examples at sub-document granularity
- `(code_snippet, tag_instruction, human_judgment)` tuples
- Requires: Consent model, local-first option, poisoning mitigation

### 4. Feedback-as-Data
- Queryable corpus of human judgment
- Org analytics: tag frequency reveals blindspots
- Feedback graphs: annotations cite prior annotations

### 5. Cross-Tool Interoperability
- MCP + SFP = LSP for human feedback
- Critical mass: 2-3 agent frameworks + 5-7 tools
- Extension via namespaced custom types

---

## Data Model (Refined)

### Location (URI-based, extensible)

```
Location =
  | LineRange { path?, start: u32, end: u32 }
  | CharRange { start: u32, end: u32 }
  | Ref { uri: string, selector?: string }

Examples:
  LineRange { path: "src/auth.rs", start: 45, end: 52 }
  Ref { uri: "figma://file/abc", selector: "node=123" }
  Ref { uri: "form://signup", selector: "field=email" }
  Ref { uri: "ui://checkout", selector: "step=payment" }
```

### Tags (Scoped Namespaces)

```
Tag {
  id: string              // Stable 12-char ID
  name: string            // Hierarchical: SECURITY.SQL_INJECTION
  instruction: string     // LLM prompt (immutable per version)
  scope: Scope            // Where tag is defined
}

Scope = GLOBAL | ORG | PROJECT | SESSION

Examples:
  SECURITY                      // Global
  SECURITY.SQL_INJECTION        // Global, specific
  ORG.COMPLIANCE.PII            // Organization-wide
  PROJECT.AUTH_FLOW             // Project-local
  SESSION.EXPERIMENTAL          // Ephemeral
```

### Versioning (Protocol + Capabilities)

```
Header {
  sfp_version: "1.0"
  capabilities: ["named_location", "media_node", "json_projection"]
}
```

Consumers parse what they understand, ignore unknown capabilities.

---

## Wire Formats

### Text (Primary — LLMs parse natively)

```
SFP/1.0
Capabilities: named_location, media_node

LEGEND:
  [# SECURITY] Review for injection vulnerabilities
  [# SECURITY.SQL_INJECTION] Parameterize this query

SESSION:
  Context: Focus on auth module security
  Exit: Revise
    Address security annotations before approval

---

src/auth.rs:45-52:
    45 | fn authenticate(input: &str) {
    46 |     db.query(format!("SELECT * WHERE name = {}", input));
    > [# SECURITY.SQL_INJECTION] Use parameterized query
    > Also add rate limiting

figma://file/abc#node=123:
    > [# UX] Button placement feels off — see attached sketch
    > [media:image/png;base64,...]
```

### JSON (Secondary — programmatic consumers)

```json
{
  "sfp_version": "1.0",
  "capabilities": ["named_location"],
  "legend": [
    { "id": "sec_001", "name": "SECURITY", "instruction": "..." }
  ],
  "session": {
    "context": "Focus on auth module security",
    "exit": { "id": "revise", "name": "Revise", "instruction": "..." }
  },
  "annotations": [
    {
      "location": { "type": "line_range", "path": "src/auth.rs", "start": 45, "end": 52 },
      "content": [
        { "type": "tag", "id": "sec_001" },
        { "type": "text", "text": "Use parameterized query" }
      ]
    }
  ]
}
```

**Rule**: If text and JSON diverge, text wins. (Markdown→AST parallel)

---

## Adoption Strategy

```
Tier 1 (Frameworks)     Tier 2 (Tools)           Tier 3 (Consumers)
─────────────────────   ─────────────────────    ─────────────────────
• Claude/MCP            • annot/hl               • LLM providers
• LangChain             • Cursor                 • Fine-tuning pipelines
• AutoGPT               • Aider                  • Analytics platforms
                        • IDE extensions

      ↓ Distribution          ↓ Production              ↓ Consumption
```

**Critical path**: MCP blesses SFP → annot implements → one framework adopts → tipping point

---

## Extension Mechanism

Namespaced custom types (ActivityPub pattern):

```
ContentNode =
  | Text { text }
  | Tag { id, name, instruction }
  | Media { mime, data }
  | Custom { namespace: "annot", type: "excalidraw", data: {...} }
```

Unknown namespaces are ignored, not rejected. Forward compatibility preserved.

---

## Open Questions (Remaining)

1. **[?] Identity binding**: How lightweight? OAuth? Signing keys? Anonymous-with-proof-of-work?
2. **[?] Immutability**: Append-only log? Merkle tree? Or just "don't mutate" convention?
3. **[?] Consent UX**: Per-session opt-in? Global toggle? Granular (tags yes, code no)?
4. **[?] Governance**: Who owns the spec? IETF? Anthropic? Community consortium?

---

## Summary: SFP is Infrastructure

| Layer                  | Standard   | Status               |
| ---------------------- | ---------- | -------------------- |
| Discovery/Invocation   | MCP        | Emerging (Anthropic) |
| Agent → Human UI       | A2UI       | Emerging (Google)    |
| Human → Agent Feedback | **SFP**    | **Proposed**         |

SFP completes the stack. Without it, the human→agent channel remains unstructured prose — the bottleneck in human-AI collaboration.