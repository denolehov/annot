# Dark Theme Implementation Notes

## What We Built

A warm dark theme for annot that preserves the light theme's "ephemeral, physical" character. The CSS is complete and ready—just needs runtime theme switching.

---

## Color Philosophy

**Light theme identity:** Warm zinc grays + cream tones + amber accents ("whisper not scream")

**Dark theme translation:**
- Pure black → lifted warm darks (`#13120f`, `#1a1916`)
- Pure white text → cream off-white (`#e8e4dc`)
- Neutral grays → brown-undertone grays
- Patterns use `rgba(200,180,140,...)` instead of white

---

## Token Architecture

```
:root {
  /* Light primitives */
  --light-zinc-*
  --light-amber-*
  
  /* Dark primitives */  
  --dark-zinc-*
  --dark-amber-*
  --dark-code-*  /* syntax highlighting */
  
  /* Semantic tokens (theme-aware) */
  --bg-main, --bg-window, --bg-portal
  --text-primary, --text-secondary, --text-muted
  --border-subtle, --border-strong
  --hl-edge, --hl-peak, --hl-mid  /* highlight gradient */
}

[data-theme="dark"] {
  /* Remap semantic tokens to dark primitives */
}
```

---

## Key Discoveries

### Highlights need gradients
The highlighter pen effect requires a 3-stop gradient, not flat color:
```css
background-image: linear-gradient(
  to right,
  var(--hl-edge),   /* 6% light / 15% dark */
  var(--hl-peak) 4%, /* 20% light / 50% dark */
  var(--hl-mid)     /* 10% light / 30% dark */
);
```

### Hardcoded colors break theming
Found in:
- `code-viewer.css` - highlight gradient had hardcoded rgba
- `Portal.svelte` - portal glow had hardcoded rgba

**Fix:** Replace with CSS variables, define in both themes.

### Dark theme needs MORE contrast for highlights
Light theme: subtle 6-20% opacity amber
Dark theme: needs 15-50% opacity to be visible

---

## Excalidraw & Mermaid (TODO)

Both need theme-aware initialization:

**Excalidraw:**
- Pass `theme: 'dark' | 'light'` prop
- Set `viewBackgroundColor` in `initialData.appState`
- May need `updateScene()` after mount
- Separate window needs `data-theme` attribute synced

**Mermaid:**
- Re-initialize with `theme: 'dark'` when theme changes
- Track `lastTheme` to detect changes

---

## Files Changed

### Tokens & Variables
- `src/styles/tokens.css` — dark primitives + semantic remapping

### CSS Fixes  
- `src/styles/components/code-viewer.css` — highlight using variables
- `src/lib/components/embedded/Portal.svelte` — portal glow variable

### Reverted (for proper implementation later)
- `src/routes/+layout.svelte`
- `src/lib/excalidraw-loader.ts`
- `src/lib/mermaid-loader.ts`

---

## To Implement Theme Switching

1. Add theme state (localStorage + system preference)
2. Set `document.documentElement.dataset.theme` on all windows
3. Add `html.dark` class for Excalidraw compatibility
4. Re-render Mermaid diagrams on theme change
5. Pass theme to Excalidraw window via backend context