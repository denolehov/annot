# Spec: Highlight Syntax (Final)

## Goal

Let agents emphasize spans in markdown content with three intensity levels, styled as skewed highlighter marks.

## Syntax

```markdown
==subtle==      → level 1 (low)
===default===   → level 2 (medium)  
====strong====  → level 3 (high)
```

## Rendering

```html
<mark class="hl hl-1">subtle</mark>
<mark class="hl hl-2">default</mark>
<mark class="hl hl-3">strong</mark>
```

---

## Edge Case Matrix

| Case                                     | Input                | Detection                              | Handling                                           |
| ---------------------------------------- | -------------------- | -------------------------------------- | -------------------------------------------------- |
| **Delimiter mismatch (opener < closer)** | `==text===`          | Count opener `=`, scan for exact match | `<mark class="hl-1">text</mark>=`                  |
| **Delimiter mismatch (opener > closer)** | `===text==`          | Same                                   | `=<mark class="hl-1">text</mark>`                  |
| **Empty content**                        | `====`               | Content between delimiters is empty    | Literal `====`                                     |
| **Nested delimiters**                    | `==a ==b== c==`      | Greedy/outermost match                 | `<mark>a ==b== c</mark>`                           |
| **Adjacent highlights**                  | `==a====b==`         | Close first, reopen                    | `<mark>a</mark><mark>b</mark>`                     |
| **Single = in content**                  | `==a=b==`            | Only `={2,4}` counts as delimiter      | `<mark>a=b</mark>`                                 |
| **Double = in content**                  | `==a==b==`           | First valid closer wins (non-greedy)   | `<mark>a</mark>b==`                                |
| **Inside code span**                     | `` `==text==` ``     | Code takes precedence                  | `<code>==text==</code>`                            |
| **Code inside highlight**                | `` ==`code`== ``     | Highlight wraps atomic code            | `<mark><code>code</code></mark>`                   |
| **Bold/italic inside**                   | `==**bold**==`       | Nesting works                          | `<mark><strong>bold</strong></mark>`               |
| **Highlight inside bold**                | `**==text==**`       | Nesting works                          | `<strong><mark>text</mark></strong>`               |
| **Portal links**                         | `==[ref](file#L1)==` | Nesting works                          | `<mark><span class="portal-ref">...</span></mark>` |
| **Cross-line**                           | `==a\nb==`           | Disallow                               | Literal `==a` + newline + `b==`                    |
| **Escaped opener**                       | `\==text==`          | Backslash escapes                      | Literal `==text==`                                 |

---

## CSS Implementation

```css
/* tokens.css */
:root {
  --hl-1: rgba(253, 224, 71, 0.35);   /* pale yellow */
  --hl-2: rgba(253, 224, 71, 0.55);   /* medium yellow */
  --hl-3: rgba(251, 191, 36, 0.65);   /* strong amber */
}

/* code-viewer.css */
.hl {
  position: relative;
  isolation: isolate;      /* reliable stacking context */
  padding: 0 3px;
}

.hl::before {
  content: "";
  position: absolute;
  inset: 0;
  z-index: -1;
  transform: skewX(-3deg);
  border-radius: 2px;
  pointer-events: none;
}

.hl-1::before { background: var(--hl-1); }
.hl-2::before { background: var(--hl-2); }
.hl-3::before { 
  background: var(--hl-3); 
  transform: skewX(-4deg);
}
.hl-3 { font-weight: 500; }

/* Adjacent highlight gap */
.hl + .hl { margin-left: 1px; }
```

---

## Parsing Implementation

Location: `render_inline()` in [markdown.rs](src-tauri/src/markdown.rs#L720-L805)
markdown.rs — src-tauri/src/markdown.rs#L720-L805
            // Text: escape and emit
            Event::Text(t) => {
                output.push_str(&html_escape(&t));
            }

            // Strong (bold): **text**
            Event::Start(Tag::Strong) => {
                output.push_str("<strong>");
            }
            Event::End(TagEnd::Strong) => {
                output.push_str("</strong>");
            }

            // Emphasis (italic): *text*
            Event::Start(Tag::Emphasis) => {
                output.push_str("<em>");
            }
            Event::End(TagEnd::Emphasis) => {
                output.push_str("</em>");
            }

            // Inline code: `code`
            Event::Code(code) => {
                output.push_str("<code>");
                output.push_str(&html_escape(&code));
                output.push_str("</code>");
            }

            // Links: [text](url)
            // Portal links (with line anchors) get special styling as spans
            Event::Start(Tag::Link { dest_url, .. }) => {
                if parse_line_anchor(&dest_url).is_some() {
                    portal_path = Some(dest_url.to_string());
                    output.push_str("<span class=\"portal-ref\">");
                    output.push_str(PORTAL_REF_ICON);
                } else {
                    output.push_str("<a href=\"");
                    output.push_str(&html_escape(&dest_url));
                    output.push_str("\">");
                }
            }
            Event::End(TagEnd::Link) => {
                if let Some(path) = portal_path.take() {
                    // If no link text was provided, use filename as label
                    // After the icon, output ends with "</svg>" if no text was added
                    if output.ends_with("</svg>") {
                        output.push_str(filename_from_path(&path));
                    }
                    output.push_str("</span>");
                } else {
                    output.push_str("</a>");
                }
            }

            // Strikethrough: ~~text~~
            Event::Start(Tag::Strikethrough) => {
                output.push_str("<del>");
            }
            Event::End(TagEnd::Strikethrough) => {
                output.push_str("</del>");
            }

            // Soft/hard breaks
            Event::SoftBreak | Event::HardBreak => {
                output.push(' ');
            }

            // Skip block elements (paragraph wrappers, etc.)
            Event::Start(Tag::Paragraph)
            | Event::End(TagEnd::Paragraph)
            | Event::Start(Tag::BlockQuote(_))
            | Event::End(TagEnd::BlockQuote(_))
            | Event::Start(Tag::List(_))
            | Event::End(TagEnd::List(_))
            | Event::Start(Tag::Item)
            | Event::End(TagEnd::Item)
            | Event::Start(Tag::Heading { .. })
            | Event::End(TagEnd::Heading(_)) => {}

            // Skip other events
            _ => {}
        }
    }

    output
}


**Approach**: Post-process `Event::Text` events with a state machine.

```rust
fn process_highlights(text: &str) -> String {
    // State: Normal | InHighlight { level, start_idx }
    // 
    // 1. Scan for opener: ={2,4} not preceded by \
    // 2. Record level (count of =)
    // 3. Scan for closer: exact same count of =
    // 4. Emit <mark class="hl hl-{level}">content</mark>
    // 5. If no closer found, emit literal opener + rest
}
```

Key rules:
- Only process in `Event::Text`, not inside `Event::Code`
- Non-greedy: first valid closer wins
- Single-line only: newline aborts open highlight

---

## Decisions

| Choice                | Rationale                                      |
| --------------------- | ---------------------------------------------- |
| Three levels only     | Avoids scope creep, sufficient nuance          |
| `={2,4}` not `={1,4}` | Single `=` too common in prose/code            |
| Non-greedy matching   | Matches markdown convention (first `*` closes) |
| Code sacrosanct       | Preserves literal semantics                    |
| Skewed styling        | User preference for hand-drawn feel            |
| `isolation: isolate`  | Fixes z-index in nested contexts               |

## Scope

**In**: 
- Highlight parsing in markdown renderer
- CSS tokens and component styles
- Text events only (not code spans)

**Out**:
- Semantic names (warn, key, etc.)
- Dark mode variants (defer to later)
- Interactive highlights
- Cross-line spans