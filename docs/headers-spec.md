# Spec: Smart Header Truncation

## Goal

Implement breadcrumb display in the markdown header that shows relevant context based on heading depth, with text truncation for long titles.

## Design

### Display Rules (Based on Heading Depth)

```
At H1:          sample.md · # Auth Service
At H2:          sample.md · # Auth Service · ## Test Cases  
At H3+:         sample.md · # Auth Service · … · ### Current Section
```

**Logic:**
- Always show: filename + H1 (root)
- At H2 depth: show H2 as well (no ellipsis needed)
- At H3-H6 depth: collapse H2 through parent into `…`, show only current

### Examples

```
Depth 1 (# only):
  readme.md · # Getting Started

Depth 2 (## ):
  readme.md · # Getting Started · ## Installation

Depth 3 (### ):
  readme.md · # Getting Started · … · ### macOS

Depth 4 (#### ):
  spec.md · # API Reference · … · #### Token Refresh

Depth 6 (###### ):
  spec.md · # API Reference · … · ###### Edge Cases
```

### Layout Structure

```
┌─────────────────────────────────────────────────────────────────┐
│ [BREADCRUMB ←────────── flex: 1, min-width: 0 ──→] [ICONS 80px] │
└─────────────────────────────────────────────────────────────────┘
```

### Component Structure

```svelte
<div class="header-breadcrumb">
  <!-- Filename -->
  <span class="crumb filename">{label}</span>
  <span class="sep">·</span>
  
  <!-- H1 (root) - always shown -->
  {#if rootSection}
    <span class="crumb root">
      <span class="level">#</span>
      <span class="title">{rootSection.title}</span>
    </span>
  {/if}
  
  <!-- H2 shown only when current depth is exactly 2 -->
  {#if currentDepth === 2 && h2Section}
    <span class="sep">·</span>
    <span class="crumb h2">
      <span class="level">##</span>
      <span class="title">{h2Section.title}</span>
    </span>
  {/if}
  
  <!-- Ellipsis + current section when depth >= 3 -->
  {#if currentDepth >= 3 && currentSection}
    <span class="sep">·</span>
    <span class="collapsed-indicator">…</span>
    <span class="sep">·</span>
    <span class="crumb current">
      <span class="level">{'#'.repeat(currentSection.level)}</span>
      <span class="title">{currentSection.title}</span>
    </span>
  {/if}
</div>
```

### Logic

```typescript
// Derived from breadcrumb
let rootSection = $derived(sectionBreadcrumb.find(s => s.level === 1) ?? null);
let h2Section = $derived(sectionBreadcrumb.find(s => s.level === 2) ?? null);
let currentSection = $derived(sectionBreadcrumb.at(-1) ?? null);
let currentDepth = $derived(currentSection?.level ?? 0);
```

### CSS

```css
.header-breadcrumb {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  flex: 1;
  min-width: 0;
  overflow: hidden;
}

.crumb {
  display: flex;
  align-items: center;
  gap: 0.25rem;
  white-space: nowrap;
  min-width: 0;
}

.crumb .title {
  text-overflow: ellipsis;
  overflow: hidden;
}

/* Current section prefers not to shrink */
.crumb.current {
  flex-shrink: 0.5;
}

/* Other crumbs shrink more readily */
.crumb.filename,
.crumb.root,
.crumb.h2 {
  flex-shrink: 1;
}

.collapsed-indicator {
  color: var(--text-muted);
  flex-shrink: 0;
}

.sep {
  color: var(--text-muted);
  flex-shrink: 0;
}
```

## Decisions

- **Depth-based display**: Structure determined by heading level, not available width
- **H1 always visible**: Root section provides document context
- **H2 shown only at depth 2**: When deeper, collapses into `…`
- **CSS text truncation**: All titles can truncate with ellipsis when space is tight
- **80px icon reservation**: Space on right for future icons

## Scope

### In
- Depth-based breadcrumb: filename + H1 + (H2 or `…` + current)
- CSS text truncation on all titles
- Space reservation for header action icons

### Out
- Width-based collapse thresholds
- Animation (not needed since structure is stable per scroll position)
- Click-to-expand
- Hover preview
