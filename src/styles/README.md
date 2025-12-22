# Style System

This directory contains annot's design system — tokens, base styles, and component CSS.

## Directory Structure

```
styles/
├── tokens.css          # Design tokens (colors, spacing, typography)
├── base.css            # Global resets, scrollbars, focus states
├── fonts.css           # @fontsource imports
├── syntax.css          # Code syntax highlighting
├── index.css           # Master import file
└── components/         # Component-specific styles
    ├── annotation-editor.css
    ├── chips.css
    ├── code-viewer.css
    ├── command-palette.css
    ├── kbd.css
    └── status-bar.css
```

## Token Categories

All design values are defined in `tokens.css`. Use these variables — **never hardcode colors or sizes**.

### Colors

| Prefix | Usage |
|--------|-------|
| `--bg-*` | Backgrounds (`--bg-main`, `--bg-panel`, `--bg-hover`) |
| `--text-*` | Text colors (`--text-primary`, `--text-secondary`, `--text-muted`) |
| `--border-*` | Borders (`--border-subtle`, `--border-strong`, `--border-hover`) |
| `--selection-*` | Selection highlights |
| `--tag-*` | Tag chip colors |
| `--error-*` | Error states |
| `--accent-*` | Interactive accents (amber family) |
| `--code-*` | Syntax highlighting |

### Sizing

| Token | Usage |
|-------|-------|
| `--radius-sm/md/lg/xl` | Border radii (4px, 6px, 8px, 12px) |
| `--shadow-sm/md/lg` | Box shadows |
| `--chip-*` | Tag chip dimensions |
| `--gutter-width` | Line number gutter width |

### Typography

| Token | Usage |
|-------|-------|
| `--font-ui` | UI text (Inter) |
| `--font-mono` | Code (JetBrains Mono) |

### Transitions

| Token | Usage |
|-------|-------|
| `--transition-fast` | Quick state changes (80ms) |
| `--transition-normal` | Standard transitions (150ms) |

## Dark Mode

Tokens are structured for easy dark mode support:

```css
/* Primitives (light) */
--light-zinc-900: #18181b;

/* Primitives (dark) - ready but not active */
--dark-zinc-900: #fafaf9;

/* Semantic tokens point to primitives */
--text-primary: var(--light-zinc-900);
```

When implementing dark mode:
1. Add `[data-theme="dark"]` selector to `tokens.css`
2. Remap semantic tokens to dark primitives
3. Components need zero changes

## Adding Component Styles

1. **Create file:** `components/[component-name].css`
2. **Add import:** Add to `index.css` in the Components section
3. **Use tokens:** Reference `var(--token-name)` for all values
4. **Document:** Update `COMPONENTS.md` with style file path

### Example

```css
/* components/my-component.css */
.my-component {
  background: var(--bg-panel);
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-md);
  padding: 16px;
  font-family: var(--font-ui);
  color: var(--text-primary);
  transition: background var(--transition-fast);
}

.my-component:hover {
  background: var(--bg-hover);
}
```

## Import Order

The order in `index.css` matters:

1. **Fonts** — Load typefaces first
2. **Tokens** — Define all variables
3. **Base** — Global resets (depends on tokens)
4. **Syntax** — Code highlighting (depends on tokens)
5. **Components** — Component styles (depends on all above)

## Common Patterns

### Popovers/Modals
```css
.modal {
  background: var(--bg-portal);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-xl);
  box-shadow: var(--shadow-md);
}
```

### Buttons
```css
.btn-primary {
  background: var(--accent-primary);
  color: white;
  border-radius: var(--radius-md);
}

.btn-secondary {
  background: var(--bg-panel);
  color: var(--text-primary);
  border: 1px solid var(--border-strong);
}

.btn-danger {
  background: var(--danger);
  color: white;
}
```

### Form Inputs
```css
.input {
  background: var(--bg-input);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-md);
  color: var(--text-primary);
}

.input:focus {
  border-color: var(--accent-primary);
  box-shadow: 0 0 0 3px var(--focus-glow);
}
```

### Scrollbars
```css
/* Already defined in base.css — inherits from tokens */
::-webkit-scrollbar-thumb {
  background: var(--border-strong);
}
::-webkit-scrollbar-thumb:hover {
  background: var(--border-hover);
}
```

## Accessibility

- **Contrast:** `--text-muted` meets WCAG AA (4.51:1)
- **Focus:** All interactive elements have `:focus-visible` styles
- **No color-only info:** Diff lines marked by position, not just color
