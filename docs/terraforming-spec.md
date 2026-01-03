# Spec: Terraforming

## Goal

Give users structured controls to tell the AI *how* content should change in the next iteration, not just *what's* wrong. UI captures intent via sliders and toggles; a translation layer converts to natural language prose the LLM can directly consume.

---

## Design

### The Four Axes

| Axis          | Question       | Spectrum                              |
| ------------- | -------------- | ------------------------------------- |
| **Form**      | What shape?    | table · list · prose · diagram · code |
| **Mass**      | How much?      | expand ←→ condense (+ remove)         |
| **Gravity**   | How important? | pin ← focus ←→ blur → dissolve        |
| **Direction** | Right track?   | lean-in ←→ move-away (+ reframe)      |

### Neutral State Semantics

Each slider axis has a center position that means "no change requested":

| Axis      | Neutral Meaning            | Output When Neutral |
| --------- | -------------------------- | ------------------- |
| Mass      | No change in length        | *Omit entirely*     |
| Gravity   | Keep prominence unchanged  | *Omit entirely*     |
| Direction | No directional correction  | *Omit entirely*     |

**Rule**: If an axis is at center (neutral), omit that sentence from the output. This keeps prompts short and reduces unintended drift.

### Selection Granularity

Selection operates at **line level only**. Users select one or more complete lines.

- Minimum selection: 1 line
- Selection always includes full lines (no partial/character selection)
- For sub-line precision, users quote specific text in their annotation

### Entry Flow

```
Shift-drag (or drag from +) → release → [c] annotate [b] bookmark [t] terraform
                                                                       ↓
                                                              Terraform palette opens
```

### Palette Layout

```
┌─────────────────────────────────────────────────────────────────┐
│  TERRAFORM                                               [esc]  │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  FORM      [1]table  [2]list  [3]prose  [4]diagram  [5]code    │
│                 ▣              ▣         (multi-select)         │
│                                                                 │
│  ─────────────────────────────────────────────────────────────  │
│                                                                 │
│  MASS      ◀━━━━━━━━●━━━━━━━━▶                    [x] remove   │
│            expand   ↑   condense                                │
│                "a bit"                                          │
│                                                                 │
│  GRAVITY   [p]pin  ◀━━━●━━━━━▶  [d]dissolve                    │
│                focus ↑  blur                                    │
│               "moderately"                                      │
│                                                                 │
│  DIRECTION [r]reframe  ◀━━━━●━━━━━━▶                           │
│                   lean-in   move-away                           │
│                      ↑                                          │
│                 "slightly"                                      │
│                                                                 │
├─────────────────────────────────────────────────────────────────┤
│  ┌───────────────────────────────────────────────────────────┐  │
│  │ "Restructure into table and prose. Expand a bit..."      │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                            [a] annotate  [⏎]    │
└─────────────────────────────────────────────────────────────────┘
```

### Intensity Scale (5 levels)

| Level | Visual | Adverb          | Level 5 Verb Upgrade |
| ----- | ------ | --------------- | -------------------- |
| 1     | ●○○○○  | "just slightly" | —                    |
| 2     | ●●○○○  | "a bit"         | —                    |
| 3     | ●●●○○  | "moderately"    | —                    |
| 4     | ●●●●○  | "significantly" | —                    |
| 5     | ●●●●●  | "completely"    | Use stronger verbs   |

**Level 5 upgrades** (max intensity uses harder verbs):
- Expand completely → "Rewrite with substantial added depth, examples, and supporting detail."
- Condense completely → "Reduce to a minimal gist — only the essential point."
- Focus completely → "Make this the absolute centerpiece; restructure everything around it."
- Blur completely → "Minimize to bare mention; treat as footnote-level context."
- Lean-in completely → "This is exactly right. Double down and develop fully."
- Move-away completely → "This approach is fundamentally wrong. Replace entirely with an alternative."

### Visual Indicators

**Icons** (SVG, top-right of selection block):

| Icon | Axis      |
| ---- | --------- |
| ⊞    | Form      |
| ↔    | Mass      |
| ★    | Gravity   |
| ⚡   | Direction |

- Hover **icon cluster only** → tooltip shows constructed prompt
- After apply: icons remain, summary on hover (sealed state)

**Gutter**: 3px left border, electric purple `#A855F7`
- Stacks with bookmark red (both visible if both present)

---

## Phrase Construction

### Sentence Emission Order

Always emit sentences in this order (matches human mental model: shape → size → importance → trajectory):

1. **Form** (if any selected)
2. **Mass** (unless neutral)
3. **Gravity** (unless neutral)
4. **Direction** (unless neutral)

Apply conflict precedence before emission (see below).

### Conflict Precedence Rules

When user selects contradictory operations, apply deterministic precedence:

**1. Remove overrides everything**
- If `Remove` is selected, emit only: "Remove this entirely."
- Ignore all other axes.

**2. Pin blocks rewrites**
- If `Pin` is selected, emit only: "Preserve this exactly as written."
- Ignore Mass (expand/condense), Direction (reframe/pivot), and Form.
- Rationale: "preserve exactly" contradicts any rewrite.

**3. Dissolve blocks Form**
- If `Dissolve` is selected, ignore Form selections.
- Rationale: "integrate into surroundings" contradicts structural conversion.
- Still allow Mass and Direction (affects how the dissolved content integrates).

### Form Phrases (Multi-Select)

**Ordering**: Always emit in UI order (table → list → prose → diagram → code).

**Primary/Secondary mapping**:
- First selected = primary format
- Remaining = secondary ("also provide")

| Forms       | Phrase Template |
| ----------- | --------------- |
| table       | "Restructure this into a Markdown table." |
| list        | "Restructure this into a bulleted list." |
| prose       | "Rewrite this as flowing prose." |
| diagram     | "Express this as a Mermaid diagram." |
| code        | "Convert this into code or pseudocode." |

**Multi-select output**:
- 1 form: "{primary phrase}"
- 2 forms: "{primary phrase} Also provide a {secondary} version."
- 3+ forms: "{primary phrase} Also provide {secondary1} and {secondary2} versions."

### Mass Phrases

| State    | Phrase |
| -------- | ------ |
| Expand (1-4) | "Expand {adverb} with more depth and examples." |
| Expand (5) | "Rewrite with substantial added depth, examples, and supporting detail." |
| Condense (1-4) | "Condense {adverb} to essentials." |
| Condense (5) | "Reduce to a minimal gist — only the essential point." |
| Neutral | *Omit* |
| Remove | "Remove this entirely." *(overrides all)* |

### Gravity Phrases

| State    | Phrase |
| -------- | ------ |
| Focus (1-4) | "Make this {adverb} more central/prominent." |
| Focus (5) | "Make this the absolute centerpiece; restructure everything around it." |
| Blur (1-4) | "Reduce prominence {adverb}; treat as supporting context." |
| Blur (5) | "Minimize to bare mention; treat as footnote-level context." |
| Neutral | *Omit* |
| Pin | "Preserve this exactly as written." *(blocks rewrites)* |
| Dissolve | "Remove as a unit; integrate its essential information into surrounding sections." *(blocks Form)* |

### Direction Phrases

| State    | Phrase |
| -------- | ------ |
| Lean-in (1-4) | "You're {adverb} on the right track. Amplify this thinking." |
| Lean-in (5) | "This is exactly right. Double down and develop this fully." |
| Move-away (1-4) | "This approach is {adverb} off-target. Replace with an alternative framing that better supports the surrounding context." |
| Move-away (5) | "This approach is fundamentally wrong. Replace entirely with an alternative." |
| Neutral | *Omit* |
| Reframe | "Keep the facts; change the angle or framing. Do not introduce new claims." |

---

## Output Format

```
TERRAFORM:

file.md:10-15:
    9 | context line
>  10 | selected line 1
>  11 | selected line 2
      └──▷ Restructure into a Markdown table. Expand a bit with more
           depth and examples. Make this moderately more central.
           This approach is slightly off-target. Replace with an
           alternative framing that better supports the surrounding
           context.

file.md:30:
      └──▷ Preserve this exactly as written.

file.md:50:
      └──▷ Remove this entirely.

ANNOTATIONS:

file.md:60:
      └──> [# TODO] Follow up on this
```

---

## Keyboard Reference

| Context   | Key     | Action                     |
| --------- | ------- | -------------------------- |
| Entry     | `t`     | Open palette               |
| Form      | `1-5`   | Toggle form (multi-select) |
| Mass      | `+`/`-` | Nudge expand/condense      |
| Mass      | `x`     | Remove                     |
| Gravity   | `f`/`b` | Nudge focus/blur           |
| Gravity   | `p`     | Pin                        |
| Gravity   | `d`     | Dissolve                   |
| Direction | `>`/`<` | Nudge lean-in/move-away    |
| Direction | `r`     | Reframe                    |
| Actions   | `a`     | Open annotation editor     |
| Actions   | `Enter` | Apply                      |
| Actions   | `Esc`   | Cancel                     |

---

## Decisions

| Decision             | Choice                                   | Rationale                                           |
| -------------------- | ---------------------------------------- | --------------------------------------------------- |
| Output format        | Natural language prose                   | LLMs understand English better than structured tags |
| Intensity levels     | 5                                        | Enough granularity without overwhelming             |
| Form selection       | Multi-select, UI order                   | Users may want multiple formats; predictable output |
| Entry point          | Extends `[c][b]` pattern with `[t]`      | Consistent with existing UX                         |
| Hover target         | Icon cluster only                        | Precise, doesn't interfere with line interaction    |
| Sealed state         | Icons + hover for summary                | Minimal visual noise                                |
| Color                | Electric purple `#A855F7`                | Distinct from bookmark (red) and annotation (amber) |
| Indicator stacking   | Both colors show                         | No information loss                                 |
| Conflict handling    | Deterministic precedence (Remove > Pin > Dissolve) | Predictable behavior users can learn       |
| Neutral state        | Omit from output                         | Keeps prompts short, reduces drift                  |
| Selection granularity| Line-level only                          | Sub-line selection deferred to future iteration     |
| Diagram format       | Mermaid                                  | Widely supported, text-based                        |
| Emission order       | Form → Mass → Gravity → Direction        | Matches human mental model                          |

---

## Scope

**In:**
- Terraform palette UI
- Four axes (Form, Mass, Gravity, Direction)
- Phrase translation layer with conflict precedence
- Neutral state handling (omit if unchanged)
- 5-level intensity with Level 5 verb upgrades
- Visual indicators (icons, gutter border)
- `TERRAFORM:` output section
- Keyboard shortcuts

**Out:**
- Presets/macros (Zoom In, Zoom Out) — deferred
- Sub-line/word-level selection — future iteration
- Relationship operations (link, decouple) — separate "restructure" mode
- Tone/voice controls — handled via existing tags
