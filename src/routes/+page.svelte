<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { onMount } from "svelte";
  import type { ContentResponse, ContentNode, Line, JSONContent, ExitMode } from "$lib/types";
  import { rangeToKey, keyToRange, isLineInRange, type Range } from "$lib/range";
  import { extractContentNodes, isContentEmpty, contentNodesToTipTap } from "$lib/tiptap";
  import AnnotationEditor from "$lib/AnnotationEditor.svelte";

  let lines: Line[] = $state([]);
  let label = $state("");
  let error = $state("");

  // Selection state
  let selection: { start: number; end: number } | null = $state(null);
  let isDragging = $state(false);
  let isShiftHeld = $state(false);
  let mouseDownHandled = false;  // Prevents click from undoing mousedown
  let hoveredLine: number | null = $state(null);

  // Annotation state - Map keyed by "startLine-endLine" → TipTap JSON
  let annotations: Map<string, JSONContent> = $state(new Map());
  let sealedRanges: Set<string> = $state(new Set());

  // Exit mode state (null index = neutral/no mode selected)
  let exitModes: ExitMode[] = $state([]);
  let selectedModeIndex: number | null = $state(null);
  let selectedMode = $derived.by(() =>
    selectedModeIndex !== null && exitModes.length > 0 ? exitModes[selectedModeIndex] : null
  );

  // Session comment state (global/file-level comment)
  let sessionComment: JSONContent | undefined = $state(undefined);
  let sessionEditorOpen = $state(false);

  // Derived: last line of current selection (for positioning editor)
  let lastSelectedLine = $derived.by(() => {
    if (!selection) return null;
    return Math.max(selection.start, selection.end);
  });

  function getAnnotation(sel: Range): JSONContent | undefined {
    return annotations.get(rangeToKey(sel));
  }

  async function updateAnnotation(content: JSONContent | null) {
    if (!selection) return;
    const key = rangeToKey(selection);
    const min = Math.min(selection.start, selection.end);
    const max = Math.max(selection.start, selection.end);

    if (content && !isContentEmpty(content)) {
      annotations.set(key, content);
      // Sync to backend
      const nodes = extractContentNodes(content);
      await invoke('upsert_annotation', {
        startLine: min,
        endLine: max,
        content: nodes
      });
    } else {
      annotations.delete(key);
      // Delete from backend
      await invoke('delete_annotation', {
        startLine: min,
        endLine: max
      });
    }
    annotations = new Map(annotations); // trigger reactivity
  }

  function sealCurrentAnnotation() {
    if (!selection) return;
    const key = rangeToKey(selection);
    const content = annotations.get(key);
    if (content) {
      sealedRanges.add(key);
      sealedRanges = new Set(sealedRanges);
    } else {
      // Remove empty annotation
      annotations.delete(key);
      annotations = new Map(annotations);
    }
    selection = null;
  }

  // Session comment handlers
  function openSessionEditor() {
    sessionEditorOpen = true;
  }

  function closeSessionEditor() {
    sessionEditorOpen = false;
  }

  async function updateSessionComment(content: JSONContent | null) {
    sessionComment = content ?? undefined;
    // Sync to backend
    const nodes = content ? extractContentNodes(content) : null;
    await invoke('set_session_comment', { content: nodes });
  }

  // Get annotation info for a specific line (is it the last line of any annotation?)
  function getAnnotationAtLine(lineNum: number): { key: string; content: JSONContent } | null {
    for (const [key, content] of annotations) {
      const range = keyToRange(key);
      if (range.end === lineNum) {
        return { key, content };
      }
    }
    return null;
  }

  function isSelected(lineNum: number): boolean {
    if (!selection) return false;
    const min = Math.min(selection.start, selection.end);
    const max = Math.max(selection.start, selection.end);
    return lineNum >= min && lineNum <= max;
  }

  function hasAnnotation(lineNum: number): boolean {
    for (const key of annotations.keys()) {
      if (isLineInRange(lineNum, keyToRange(key))) {
        return true;
      }
    }
    return false;
  }

  function getLineFromEvent(e: MouseEvent): number | null {
    const el = e.target as Element;
    const row = el.closest('.line') as HTMLElement | null;
    return row ? parseInt(row.dataset.line ?? '', 10) : null;
  }

  // Mouse handlers for selection
  function handleGutterMouseDown(lineNum: number, e: MouseEvent) {
    e.preventDefault();
    isDragging = true;
    mouseDownHandled = true;
    selection = { start: lineNum, end: lineNum };
  }

  function handleContentMouseDown(e: MouseEvent) {
    if (!e.shiftKey) return;
    const lineNum = getLineFromEvent(e);
    if (lineNum === null) return;
    e.preventDefault();
    isDragging = true;
    selection = { start: lineNum, end: lineNum };
  }

  function handleMouseMove(e: MouseEvent) {
    if (!isDragging || !selection) return;
    const lineNum = getLineFromEvent(e);
    if (lineNum !== null) {
      selection = { start: selection.start, end: lineNum };
    }
  }

  function handleMouseUp() {
    isDragging = false;
  }

  function handleGutterClick(lineNum: number) {
    // Skip if mousedown already handled this interaction
    if (mouseDownHandled) {
      mouseDownHandled = false;
      return;
    }
    // Toggle off if clicking same single-line selection
    if (selection?.start === lineNum && selection?.end === lineNum) {
      selection = null;
    } else {
      selection = { start: lineNum, end: lineNum };
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Shift') {
      isShiftHeld = true;
    } else if (e.key === 'Tab') {
      // Always prevent default Tab behavior
      e.preventDefault();
      // Only cycle exit modes when editor is closed
      if (exitModes.length > 0 && !selection) {
        if (e.shiftKey) {
          // Cycle backward: 0 → null → last → ... → 1 → 0
          if (selectedModeIndex === null) {
            selectedModeIndex = exitModes.length - 1;
          } else if (selectedModeIndex === 0) {
            selectedModeIndex = null;
          } else {
            selectedModeIndex = selectedModeIndex - 1;
          }
        } else {
          // Cycle forward: null → 0 → 1 → ... → last → null
          if (selectedModeIndex === null) {
            selectedModeIndex = 0;
          } else if (selectedModeIndex === exitModes.length - 1) {
            selectedModeIndex = null;
          } else {
            selectedModeIndex = selectedModeIndex + 1;
          }
        }
        // Sync to backend
        const modeId = selectedModeIndex !== null ? exitModes[selectedModeIndex].id : null;
        invoke('set_exit_mode', { modeId });
      }
    } else if (e.key === 'c' && hoveredLine !== null && !selection) {
      // Open editor on hovered line
      e.preventDefault();
      selection = { start: hoveredLine, end: hoveredLine };
    } else if (e.key === 'g' && !selection && !sessionEditorOpen) {
      // Open session comment editor (when not in any editor)
      const activeEl = document.activeElement;
      const isInEditor = activeEl?.closest('.annotation-editor, .session-editor');
      const isInInput = activeEl instanceof HTMLInputElement || activeEl instanceof HTMLTextAreaElement;
      if (!isInEditor && !isInInput) {
        e.preventDefault();
        openSessionEditor();
      }
    }
    // Escape is now handled by the editor's blur handler
  }

  function handleKeyUp(e: KeyboardEvent) {
    if (e.key === 'Shift') {
      isShiftHeld = false;
    }
  }

  function handleAddMouseDown(lineNum: number, e: MouseEvent) {
    e.preventDefault();
    isDragging = true;
    mouseDownHandled = true;
    selection = { start: lineNum, end: lineNum };
  }

  onMount(async () => {
    const window = getCurrentWindow();
    try {
      const res = await invoke<ContentResponse>("get_content");
      label = res.label;
      lines = res.lines;
      exitModes = res.exit_modes;

      // Find index of initially selected mode (if any)
      if (res.selected_exit_mode_id) {
        const idx = exitModes.findIndex(m => m.id === res.selected_exit_mode_id);
        if (idx >= 0) selectedModeIndex = idx;
      }

      // Hydrate session comment from backend
      if (res.session_comment) {
        sessionComment = contentNodesToTipTap(res.session_comment);
      }

      // Listen for window close - this triggers output and exit
      await window.onCloseRequested(async (event) => {
        event.preventDefault();
        await invoke('finish_session');
      });
    } catch (e) {
      error = String(e);
    }
    // Show window after content is ready (started hidden to avoid flash)
    await window.show();
  });
</script>

<svelte:window onkeydown={handleKeyDown} onkeyup={handleKeyUp} />

<main class="viewer" style:--mode-color={selectedMode?.color ?? 'transparent'}>
  <div class="sticky-header">
    <header class="header" data-tauri-drag-region>
      <div class="header-left">
        <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
        <span
          class="file-name"
          class:has-comment={sessionComment !== undefined}
          onclick={openSessionEditor}
        >{label}</span>
      </div>
      <div class="header-right"></div>
    </header>
    <!-- Session editor slot -->
    {#if sessionEditorOpen || sessionComment}
      <div class="session-slot">
        <AnnotationEditor
          content={sessionComment}
          sealed={!sessionEditorOpen}
          onUpdate={updateSessionComment}
          onUnseal={openSessionEditor}
          onDismiss={closeSessionEditor}
        />
      </div>
    {/if}
  </div>

  {#if error}
    <div class="error">{error}</div>
  {:else if lines.length === 0}
    <div class="loading">Loading...</div>
  {:else}
    <div
      class="content"
      class:shift-held={isShiftHeld}
      onmousedown={handleContentMouseDown}
      onmousemove={handleMouseMove}
      onmouseup={handleMouseUp}
      role="presentation"
    >
      {#each lines as line}
        <div
          class="line"
          class:selected={isSelected(line.number)}
          class:annotated={hasAnnotation(line.number)}
          data-line={line.number}
          onmouseenter={() => hoveredLine = line.number}
          onmouseleave={() => hoveredLine = null}
          role="presentation"
        >
          <button
            class="add-btn"
            onmousedown={(e) => handleAddMouseDown(line.number, e)}
            aria-label="Add annotation"
          >+</button>
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <span
            class="gutter"
            class:selected={isSelected(line.number)}
            onmousedown={(e) => handleGutterMouseDown(line.number, e)}
            onclick={() => handleGutterClick(line.number)}
            role="button"
            tabindex="-1"
          >
            {line.number}
          </span>
          <span class="code">
            {#if line.html}
              {@html line.html}
            {:else}
              {line.content}
            {/if}
          </span>
        </div>
        {@const annotationAtLine = getAnnotationAtLine(line.number)}
        {@const isLastSelectedLine = line.number === lastSelectedLine && selection && !isDragging}
        {@const rangeKey = annotationAtLine?.key ?? (isLastSelectedLine && selection ? rangeToKey(selection) : null)}
        {#if rangeKey}
          {#key rangeKey}
            <AnnotationEditor
              content={annotations.get(rangeKey)}
              sealed={sealedRanges.has(rangeKey)}
              onUpdate={updateAnnotation}
              onUnseal={() => {
                selection = keyToRange(rangeKey);
                sealedRanges.delete(rangeKey);
                sealedRanges = new Set(sealedRanges);
              }}
              onDismiss={sealCurrentAnnotation}
            />
          {/key}
        {/if}
      {/each}
    </div>
  {/if}

  <!-- Footer / Status Bar -->
  <footer class="status-bar" style:--mode-color={selectedMode?.color ?? 'transparent'}>
    <div class="status-bar-left">
      <button
        class="exit-mode-btn"
        class:neutral={!selectedMode}
        onclick={() => {
          if (exitModes.length > 0) {
            // Cycle forward including neutral: null → 0 → 1 → ... → last → null
            if (selectedModeIndex === null) {
              selectedModeIndex = 0;
            } else if (selectedModeIndex === exitModes.length - 1) {
              selectedModeIndex = null;
            } else {
              selectedModeIndex = selectedModeIndex + 1;
            }
            const modeId = selectedModeIndex !== null ? exitModes[selectedModeIndex].id : null;
            invoke('set_exit_mode', { modeId });
          }
        }}
      >
        <kbd>Tab</kbd>
        <span class="exit-mode-label">
          {#if selectedMode}
            {selectedMode.name}
            <span class="exit-mode-instruction">({selectedMode.instruction})</span>
          {:else}
            set exit mode
          {/if}
        </span>
      </button>
    </div>
    <div class="status-bar-right">
      <span class="kbd-hint"><kbd>g</kbd> global note</span>
      <span class="kbd-hint"><kbd>⌘W</kbd> close</span>
    </div>
  </footer>
</main>

<style>
  :global(*) {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
  }

  :global(body) {
    font-family: "JetBrains Mono", ui-monospace, "SF Mono", Menlo, Monaco, monospace;
    font-size: 12px;
    line-height: 22px;
    background-color: #fafaf9;
    color: #18181b;
    overflow: hidden;
  }

  .viewer {
    height: 100vh;
    display: flex;
    flex-direction: column;
    position: relative;
  }

  /* Header with frosted glass effect */
  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 40px;
    /* Left padding for traffic lights, slight top padding to align text */
    padding: 2.4px 12px 0 90px;
    flex-shrink: 0;
    border-bottom: 1px solid rgba(0, 0, 0, 0.06);
    background: color-mix(in srgb, #fafaf8 85%, transparent);
    backdrop-filter: blur(20px) saturate(180%);
    -webkit-backdrop-filter: blur(20px) saturate(180%);
    -webkit-app-region: drag;
    /* Soft shadow for depth */
    box-shadow:
      0 1px 3px rgba(0, 0, 0, 0.04),
      0 4px 12px rgba(0, 0, 0, 0.03);
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .header-right {
    display: flex;
    align-items: center;
    gap: 8px;
    -webkit-app-region: no-drag;
  }

  /* Sticky header container */
  .sticky-header {
    position: sticky;
    top: 0;
    z-index: 100;
    background: #fafaf9;
  }

  /* Session slot for global comment editor */
  .session-slot {
    padding: 0 12px 12px 12px;
    background: color-mix(in srgb, #fafaf8 85%, transparent);
    backdrop-filter: blur(20px) saturate(180%);
    -webkit-backdrop-filter: blur(20px) saturate(180%);
  }

  /* Override annotation-editor styles when in session slot */
  .session-slot :global(.annotation-editor) {
    margin: 8px 8px 0 8px;
    max-width: none;
  }

  /* Hide arrow for session editor */
  .session-slot :global(.annotation-editor::after) {
    display: none;
  }

  /* Clickable filename */
  .file-name {
    color: #18181b;
    font-weight: 600;
    font-size: 13px;
    letter-spacing: -0.01em;
    cursor: pointer;
    -webkit-app-region: no-drag;
    transition: color 150ms ease;
  }

  .file-name:hover {
    color: #52525b;
  }

  .file-name.has-comment {
    text-decoration: underline;
    text-decoration-style: dotted;
    text-underline-offset: 3px;
  }

  .content {
    flex: 1;
    overflow: auto;
    padding-bottom: 1rem;
  }

  .line {
    position: relative;
    display: flex;
    white-space: pre;
    tab-size: 4;
  }

  .line:hover {
    background-color: var(--bg-main, #fafaf9);
  }

  /* Selection (amber family - matches hl) */
  .line.selected {
    background-color: var(--selection-bg, #fefce8);
  }

  .line.selected:hover {
    background-color: var(--selection-bg, #fefce8);
  }

  .gutter.selected {
    background-color: var(--selection-bg, #fefce8);
    color: var(--text-secondary);
    border-right-color: var(--selection-border, #fcd34d);
  }

  /* Left accent bar on selected lines */
  .line.selected::before {
    content: "";
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    width: 3px;
    background: var(--selection-border, #fcd34d);
  }

  /* Annotated lines (subtle highlight) */
  .line.annotated {
    background-color: var(--selection-bg, #fefce8);
  }

  .line.annotated::before {
    content: "";
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    width: 3px;
    background: var(--selection-border, #fcd34d);
  }

  /* Shift+drag cursor */
  .content.shift-held .code {
    cursor: pointer;
  }

  /* Floating add button */
  .add-btn {
    position: absolute;
    top: 50%;
    left: calc(var(--gutter-width, 50px) - 9px);
    transform: translateY(-50%);
    width: 18px;
    height: 18px;
    background: var(--selection-border, #fcd34d);
    color: white;
    border: none;
    border-radius: 4px;
    font-size: 16px;
    font-weight: 400;
    cursor: pointer;
    display: none;
    align-items: center;
    justify-content: center;
    box-shadow: 0 2px 4px rgba(0,0,0,0.1);
    padding: 0;
    padding-bottom: 2px;
    line-height: 0;
  }

  .add-btn:hover {
    transform: translateY(-50%) scale(1.1);
    box-shadow: 0 3px 6px rgba(0,0,0,0.15);
  }

  .line:not(.selected):hover .add-btn {
    display: flex;
  }

  .gutter {
    width: var(--gutter-width, 50px);
    flex-shrink: 0;
    padding-right: 12px;
    text-align: right;
    color: var(--text-muted, #71717a);
    -webkit-user-select: none;
    user-select: none;
    cursor: pointer;
    font-variant-numeric: tabular-nums;
    border-right: 1px solid var(--border-subtle, #e4e4e7);
    /* Prevent gutter from being included in text selection */
    pointer-events: auto;
  }

  .gutter:hover {
    color: var(--text-secondary, #52525b);
  }

  .code {
    flex: 1;
    padding-left: 16px;
    -webkit-user-select: text;
    user-select: text;
  }

  .error {
    padding: 2rem;
    color: #dc2626;
  }

  .loading {
    padding: 2rem;
    color: #71717a;
  }

  /* Status Bar / Footer */
  .status-bar {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 16px;
    background: color-mix(in srgb, var(--mode-color, #fafaf9) 15%, #fafaf9);
    backdrop-filter: blur(16px) saturate(150%);
    -webkit-backdrop-filter: blur(16px) saturate(150%);
    border-top: 1px solid color-mix(in srgb, var(--mode-color, transparent) 20%, rgba(0, 0, 0, 0.06));
    font-size: 12px;
    z-index: 10;
    position: relative;
    transition: background 400ms ease, border-top-color 400ms ease;
  }

  .status-bar-left,
  .status-bar-right {
    display: flex;
    align-items: center;
    gap: 16px;
  }

  .exit-mode-btn {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 4px 8px;
    margin: -4px -8px;
    border: none;
    background: transparent;
    border-radius: 4px;
    cursor: pointer;
    font-family: var(--font-ui, "Inter", sans-serif);
    font-size: 11px;
    font-variant: small-caps;
    text-transform: lowercase;
    color: var(--text-muted, #71717a);
    line-height: 1;
    transition: background 150ms ease;
  }

  .exit-mode-btn:hover {
    background: rgba(0, 0, 0, 0.04);
  }

  .exit-mode-btn.neutral {
    color: var(--text-muted, #a1a1aa);
  }

  .exit-mode-label {
    font-weight: 500;
  }

  .exit-mode-instruction {
    font-weight: 400;
    opacity: 0.7;
  }

  /* Global kbd styling */
  :global(kbd) {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-window);
    border: 1px solid var(--border-strong);
    border-radius: 3px;
    padding: 1px 4px;
    font-weight: 500;
    font-size: 9px;
    color: var(--text-secondary);
    box-shadow:
      0 1px 0 rgba(0, 0, 0, 0.1),
      0 2px 4px rgba(0, 0, 0, 0.05),
      inset 0 1px 0 rgba(255, 255, 255, 0.1);
    font-family: var(--font-ui);
    min-height: 16px;
    line-height: 1;
  }

  /* Hint wrapper for kbd + label combinations */
  :global(.kbd-hint) {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-variant: small-caps;
    font-size: 11px;
    color: var(--text-muted);
    font-family: var(--font-ui);
    line-height: 1;
  }

  /* Status bar variant - subtler kbd background */
  .status-bar kbd {
    background: var(--bg-main);
    border-color: var(--border-subtle);
  }

  /* Scrollbar styling */
  .content::-webkit-scrollbar {
    width: 6px;
    height: 6px;
  }

  .content::-webkit-scrollbar-thumb {
    background-color: #d4d4d8;
    border-radius: 99px;
  }

  .content::-webkit-scrollbar-thumb:hover {
    background-color: #a1a1aa;
  }

  .content::-webkit-scrollbar-track {
    background: transparent;
  }

  /* ===========================================
     Design Tokens (matching hl)
     =========================================== */
  :global(:root) {
    /* Backgrounds & Neutrals (zinc family - warmer grays) */
    --bg-main: #fafaf9;          /* warm zinc-50 */
    --bg-window: #fefefe;        /* warm white - subtle cream tint */
    --bg-panel: #fafaf8;         /* warm zinc-50 */
    --bg-portal: #fdfcfa;        /* lighter warm cream for editors */

    /* Borders */
    --border-subtle: #e4e4e7;    /* zinc-200 */
    --border-strong: #d4d4d8;    /* zinc-300 */
    --border-active: #71717a;    /* zinc-500 */

    /* Text (zinc family) */
    --text-primary: #18181b;     /* zinc-900 */
    --text-secondary: #52525b;   /* zinc-600 */
    --text-muted: #71717a;       /* zinc-500 */

    /* Selection (amber family) */
    --selection-bg: #fefce8;     /* amber-50 */
    --selection-border: #fcd34d; /* amber-300 */

    /* Tag colors (zinc family) */
    --tag-bg: #fafafa;           /* zinc-50 */
    --tag-border: #d4d4d8;       /* zinc-300 */
    --tag-text: #52525b;         /* zinc-600 */

    /* Chip sizing */
    --chip-height: 20px;
    --chip-radius: 4px;
    --chip-font: 10px;
    --chip-padding: 0 6px;

    /* Fonts */
    --font-mono: "JetBrains Mono", monospace;
    --font-ui: "Inter", sans-serif;

    /* Layout */
    --gutter-width: 50px;

    /* Code Syntax (GitHub Light) */
    --code-keyword: #d73a49;
    --code-func: #6f42c1;
    --code-string: #032f62;
    --code-comment: #6a737d;
    --code-constant: #005cc5;
    --code-variable: #e36209;
    --code-tag: #22863a;
    --code-op: #d73a49;
    --code-support: #6f42c1;
  }

  /* ===========================================
     Syntax Highlighting (syntect CSS classes)

     syntect uses semantic class names like:
       <span class="storage type function rust">fn</span>
       <span class="string quoted double rust">"hello"</span>
       <span class="comment line double-slash rust">// comment</span>

     Reference: src-tauri/src/highlight.rs::documents_html_structure_and_classes
     =========================================== */

  /* Comments */
  :global(.comment) { color: var(--code-comment); font-style: italic; }

  /* Storage (keywords like fn, let, const, function, var) */
  :global(.storage) { color: var(--code-keyword); }

  /* Keywords (control flow: return, if, else, for, while) */
  :global(.keyword) { color: var(--code-keyword); }

  /* Strings */
  :global(.string) { color: var(--code-string); }

  /* Constants (numbers, booleans, null) */
  :global(.constant) { color: var(--code-constant); }

  /* Entity names (function names, class names, type names) */
  :global(.entity.name) { color: var(--code-func); }
  :global(.entity.name.function) { color: var(--code-func); }
  :global(.entity.name.type) { color: var(--code-func); }
  :global(.entity.name.class) { color: var(--code-func); }

  /* Variables */
  :global(.variable) { color: var(--text-primary); }
  :global(.variable.parameter) { color: var(--code-variable); }
  :global(.variable.other) { color: var(--text-primary); }

  /* Support (macros, built-ins) */
  :global(.support) { color: var(--code-support); }
  :global(.support.macro) { color: var(--code-support); }
  :global(.support.function) { color: var(--code-support); }

  /* Punctuation - keep neutral */
  :global(.punctuation) { color: var(--text-primary); }

  /* Meta - generally inherit, but provide fallback */
  :global(.meta) { color: inherit; }

  /* Source - base scope, inherit color */
  :global(.source) { color: var(--text-primary); }
</style>
