# extractContentNodes Refactoring Analysis

## Current State

The function in [tiptap.ts](src/lib/tiptap.ts#L1404-L1618) has ~150 lines with these problems:

<!-- portal: /Users/denolehov/_p/rust/annot/src/lib/tiptap.ts#L1404-L1618 -->
```typescript
export function extractContentNodes(json: JSONContent): ContentNode[] {
  if (!json.content || json.content.length === 0) {
    return [];
  }

  const nodes: ContentNode[] = [];
  let pendingText = '';

  // Track list context for proper markdown formatting
  type ListContext = { type: 'bullet' | 'ordered'; index: number };
  const listStack: ListContext[] = [];

  function flushText() {
    if (!pendingText) return;

    // Parse for ```replace blocks
    // Format: ```replace\n{original}\n---\n{replacement}\n```
    const replacePattern = /```replace\n([\s\S]*?)\n---\n([\s\S]*?)\n```/g;
    let lastIndex = 0;
    let match;

    while ((match = replacePattern.exec(pendingText)) !== null) {
      // Add text before the match
      if (match.index > lastIndex) {
        const beforeText = pendingText.slice(lastIndex, match.index);
        if (beforeText.trim()) {
          nodes.push({ type: 'text', text: beforeText });
        }
      }

      // Add the replace node
      const original = match[1];
      const replacement = match[2];
      nodes.push({ type: 'replace', original, replacement });

      lastIndex = match.index + match[0].length;
    }

    // Add remaining text after last match
    if (lastIndex < pendingText.length) {
      const afterText = pendingText.slice(lastIndex);
      if (afterText.trim()) {
        nodes.push({ type: 'text', text: afterText });
      }
    } else if (lastIndex === 0) {
      // No matches found, add as plain text
      nodes.push({ type: 'text', text: pendingText });
    }

    pendingText = '';
  }

  function getListPrefix(): string {
    if (listStack.length === 0) return '';
    const indent = '  '.repeat(listStack.length - 1);
    const ctx = listStack[listStack.length - 1];
    if (ctx.type === 'bullet') {
      return `${indent}- `;
    } else {
      return `${indent}${ctx.index}. `;
    }
  }

  function walkNode(node: JSONContent) {
    if (node.type === 'text' && node.text) {
      // Apply marks as markdown (StarterKit v3 includes: bold, italic, strike, code, underline, link)
      let text = node.text;
      let linkHref: string | null = null;
      if (node.marks) {
        for (const mark of node.marks) {
          switch (mark.type) {
            case 'bold':
              text = `**${text}**`;
              break;
            case 'italic':
              text = `*${text}*`;
              break;
            case 'strike':
              text = `~~${text}~~`;
              break;
            case 'code':
              text = `\`${text}\``;
              break;
            case 'underline':
              // No standard markdown for underline, use HTML
              text = `<u>${text}</u>`;
              break;
            case 'link':
              // Capture href, apply after other marks
              linkHref = mark.attrs?.href ?? null;
              break;
          }
        }
        // Apply link last so it wraps the formatted text
        if (linkHref) {
          text = `[${text}](${linkHref})`;
        }
      }
      pendingText += text;
    } else if (node.type === 'tagChip' && node.attrs) {
      flushText();
      nodes.push({
        type: 'tag',
        id: node.attrs.id,
        name: node.attrs.name,
        instruction: node.attrs.instruction,
      });
    } else if (node.type === 'mediaChip' && node.attrs) {
      flushText();
      nodes.push({
        type: 'media',
        image: node.attrs.image,
        mime_type: node.attrs.mimeType,
      });
    } else if (node.type === 'excalidrawChip' && node.attrs) {
      flushText();
      nodes.push({
        type: 'excalidraw',
        elements: node.attrs.elements,
        image: node.attrs.image,
      });
    } else if (node.type === 'replacePreview' && node.attrs) {
      flushText();
      nodes.push({
        type: 'replace',
        original: node.attrs.original,
        replacement: node.attrs.replacement,
      });
    } else if (node.type === 'errorChip' && node.attrs) {
      flushText();
      nodes.push({
        type: 'error',
        source: node.attrs.source,
        message: node.attrs.message,
      });
    } else if (node.type === 'pasteChip' && node.attrs) {
      flushText();
      nodes.push({
        type: 'paste',
        content: node.attrs.content,
      });
    } else if (node.type === 'bulletList') {
      // Push bullet list context
      listStack.push({ type: 'bullet', index: 0 });
      if (node.content) {
        node.content.forEach(walkNode);
      }
      listStack.pop();
    } else if (node.type === 'orderedList') {
      // Push ordered list context (start from attrs or default to 1)
      const start = node.attrs?.start ?? 1;
      listStack.push({ type: 'ordered', index: start - 1 });
      if (node.content) {
        node.content.forEach(walkNode);
      }
      listStack.pop();
    } else if (node.type === 'listItem') {
      // Increment index for ordered lists
      if (listStack.length > 0) {
        listStack[listStack.length - 1].index++;
      }
      // Add newline before list item (except first item at top level)
      if (pendingText || nodes.length > 0) {
        pendingText += '\n';
      }
      // Add list marker
      pendingText += getListPrefix();
      // Walk children but handle nested lists specially
      if (node.content) {
        for (const child of node.content) {
          if (child.type === 'paragraph') {
            // Don't add newline for first paragraph in list item
            if (child.content) {
              child.content.forEach(walkNode);
            }
          } else if (child.type === 'bulletList' || child.type === 'orderedList') {
            // Nested list - walk it
            walkNode(child);
          } else {
            walkNode(child);
          }
        }
      }
    } else if (node.type === 'hardBreak') {
      // Hard break within a paragraph - preserve as newline
      pendingText += '\n';
    } else if (node.type === 'paragraph') {
      // Add newline between paragraphs (except first)
      if (pendingText || nodes.length > 0) {
        pendingText += '\n';
      }
      if (node.content) {
        node.content.forEach(walkNode);
      }
    } else if (node.content) {
      node.content.forEach(walkNode);
    }
  }

  json.content.forEach(walkNode);
  flushText();

  // Trim trailing whitespace from last text node
  if (nodes.length > 0) {
    const last = nodes[nodes.length - 1];
    if (last.type === 'text') {
      last.text = last.text.trimEnd();
      if (!last.text) {
        nodes.pop();
      }
    }
  }

  return nodes;
}
```

1. **14-branch if/else chain** — Open-Closed Principle violation
2. **Mixed responsibilities** — text accumulation, list formatting, chip extraction, replace parsing
3. **Mutable shared state** — `pendingText`, `listStack`, `nodes` mutated across recursive calls
4. **Copy-paste chip handlers** — all 6 follow the same pattern
5. **Stringly-typed** — no TypeScript safety on node types

## Proposed Architecture

### 1. Mark-to-Markdown (pure function)

```typescript
const MARK_WRAPPERS: Record<string, (text: string) => string> = {
  bold: (t) => `**${t}**`,
  italic: (t) => `*${t}*`,
  strike: (t) => `~~${t}~~`,
  code: (t) => `\`${t}\``,
  underline: (t) => `<u>${t}</u>`,
};

function applyMarks(text: string, marks?: JSONContent['marks']): string {
  // Handle marks via table + link specially
}
```

### 2. Chip Extractors (registry pattern)

```typescript
const CHIP_EXTRACTORS: Record<string, ChipExtractor> = {
  tagChip: (attrs) => ({ type: 'tag', id: attrs.id, ... }),
  mediaChip: (attrs) => ({ type: 'media', ... }),
  replacePreview: (attrs) => ({ type: 'replace', ... }),
  // Adding a new chip = add one entry here
};
```

### 3. TextAccumulator class

Encapsulates `pendingText` + `nodes` + replace block parsing:

```typescript
class TextAccumulator {
  append(text: string): void;
  pushNode(node: ContentNode): void;
  flush(): void;
  getNodes(): ContentNode[];
}
```

### 4. ListContext class

Encapsulates list stack with clean interface:

```typescript
class ListContext {
  enter(type: 'bullet' | 'ordered', start?: number): void;
  exit(): void;
  incrementIndex(): void;
  getPrefix(): string;
}
```

### 5. Main function becomes orchestrator

```typescript
function walk(node: JSONContent): void {
  // Text node? → applyMarks + accumulator.append
  // Chip node? → CHIP_EXTRACTORS[type] → accumulator.pushNode
  // Structural? → switch on 6 cases (list, listItem, paragraph, etc.)
}
```

## Comparison

| Aspect      | Before          | After                  |
| ----------- | --------------- | ---------------------- |
| Lines       | ~150 monolithic | ~120 split into units  |
| Adding chip | Edit walkNode   | Add to CHIP_EXTRACTORS |
| Testability | Hard            | Easy (pure functions)  |
| State       | 3 mutable vars  | Encapsulated classes   |

## Questions

1. **Scope**: Full refactor now, or incremental?
2. **Testing**: Existing tests in [tiptap.test.ts](src/lib/tiptap.test.ts) should cover behavior — verify first?
3. **Type safety**: Add discriminated unions for node types?
