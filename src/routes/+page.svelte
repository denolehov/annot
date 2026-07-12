<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen, emit } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { onMount, tick } from "svelte";
  import type { ContentResponse, ContentNode, ContentMetadata, DiffDocument, Line, JSONContent, ExitMode, Tag, MarkdownMetadata, SectionInfo, ConfigSnapshot } from "$lib/types";
  import { getLineNumber, isSelectable, isPortalLine, isCodeBlockLine, isCodeBlockFence, isTableLine, isHorizontalRule } from "$lib/line-utils";
  import { type Range } from "$lib/range";
  import { selectionToAnchor, type Anchor, type Side, type SlotRef } from "$lib/anchor";
  import { extractContentNodes, isContentEmpty, contentNodesToTipTap, findExcalidrawChip } from "$lib/tiptap";
  import AnnotationSlot from "$lib/components/AnnotationSlot.svelte";
  import CopyDropdown from "$lib/CopyDropdown.svelte";
  import { CommandPalette } from "$lib/CommandPalette";
  import SaveModal from "$lib/SaveModal.svelte";
  import HelpOverlay from "$lib/HelpOverlay.svelte";
  import Portal from "$lib/components/embedded/Portal.svelte";
  import CodeBlock from "$lib/components/embedded/CodeBlock.svelte";
  import Table from "$lib/components/embedded/Table.svelte";
  import RegularLines from "$lib/components/embedded/RegularLines.svelte";
  import { Header, StatusBar, SessionEditor, WindowResizeHandles } from "$lib/components";
  import { PaneGroup, Pane, PaneResizer } from "paneforge";
  import FileTree from "$lib/components/FileTree.svelte";
  import { deriveDisplay, selectionToDiffAnchor, type DiffViewMode } from "$lib/display-rows";
  import { useFileTree } from "$lib/composables/useFileTree.svelte";
  import { useFileCollapse } from "$lib/composables/useFileCollapse.svelte";
  import { useExitModes } from "$lib/composables/useExitModes.svelte";
  import { useContentTracking } from "$lib/composables/useContentTracking.svelte";
  import { useInteraction, type EditorKind } from "$lib/composables/useInteraction.svelte";
  import { useAnnotations } from "$lib/composables/useAnnotations.svelte";
  import { useKeyboard } from "$lib/composables/useKeyboard.svelte";
  import { useSelectionBounds } from "$lib/composables/useSelectionBounds.svelte";
  import { useMermaid } from "$lib/composables/useMermaid.svelte";
  import { useLineSegments } from "$lib/composables/useLineSegments.svelte";
  import { useSearch } from "$lib/composables/useSearch.svelte";
  import { useOverlay } from "$lib/composables/useOverlay.svelte";
  import { useHistory, emptySessionData, type SessionData } from "$lib/composables/useHistory.svelte";
  import SearchBar from "$lib/components/SearchBar.svelte";
  import { AnnotProvider } from "$lib/context";
  import type { SaveContentResponse } from "$lib/types";
  import { initTheme, setTheme, type ThemePreference } from "$lib/theme";
  import { convertMermaidToExcalidraw } from "$lib/mermaid-to-excalidraw";
  import { isMermaidExcalidrawSupported } from "$lib/mermaid-loader";

  let lines: Line[] = $state([]);
  let diffDocs = $state<DiffDocument[] | null>(null);
  let loaded = $state(false);
  let label = $state("");
  let error = $state("");
  let metadata = $state<ContentMetadata>({ type: 'plain' });
  let allowsImagePaste = $state(false);

  // =============================================================================
  // Coordinate System (Display Index)
  // =============================================================================
  // Selection coordinates use display indices (1-indexed positions in the
  // lines array) — ephemeral UI state only. Annotation identity is an id; its
  // position is an Anchor in source coordinates, computed from the selection
  // at creation (selectionToAnchor) and resolved back to display rows at
  // render time (useAnnotations). No display index is ever persisted.
  // =============================================================================

  let markdownMetadata = $derived(metadata.type === 'markdown' ? metadata : null);

  // Toast state
  let toastMessage = $state<string | null>(null);
  let toastExiting = $state(false);
  let toastTimeout: ReturnType<typeof setTimeout> | null = null;

  function showToast(message: string, duration = 3000) {
    if (toastTimeout) clearTimeout(toastTimeout);
    toastMessage = message;
    toastExiting = false;
    toastTimeout = setTimeout(() => {
      toastExiting = true;
      // Wait for exit animation to complete
      setTimeout(() => {
        toastMessage = null;
        toastExiting = false;
      }, 200);
    }, duration);
  }

  // The DisplayRow spine: single source of display truth for diff mode.
  let diffDisplay = $derived(diffDocs ? deriveDisplay(diffDocs) : null);

  // content-visibility: auto only pays for itself on big documents (its win
  // is skipping re-wrap of off-screen lines during resize, which scales with
  // line count). Below this, lines render fully live — no scroll-in pop, no
  // skipped-layout invalidation edge cases (see code-viewer.css).
  const VIRTUALIZE_LINE_THRESHOLD = 2000;
  let virtualizeLines = $derived(
    (diffDisplay ? diffDisplay.rows.length : lines.length) > VIRTUALIZE_LINE_THRESHOLD
  );

  // Diff view projection (unified default). Session-scoped by design —
  // persistence is parked (see .specs.local/diff-redesign/s4-split-view.md).
  let diffView = $state<DiffViewMode>('unified');

  function setDiffView(mode: DiffViewMode) {
    diffView = mode;
  }

  // Content tracking (composable)
  const contentTracking = useContentTracking(() => diffDisplay);
  let contentEl: HTMLDivElement | null = $state(null);
  let scrollRafId: number | null = null;

  // File tree sidebar (composable) — diff mode only
  const fileTree = useFileTree();

  // While the window is GROWING, suspend rendering of the content tree
  // (content-visibility: hidden via the `.resizing` class). On a 10k-line file
  // the reflow can't fill the newly-exposed area fast enough, so content
  // visibly trails the growing edge; freezing the last-painted frame until the
  // drag settles hides that entirely.
  //
  // Only on grow, not shrink: shrinking has no trailing edge (existing content
  // already covers the smaller window), and freezing a fast shrink outran the
  // browser's cached paint and flashed blank. Live content on shrink is fine.
  //
  // No per-frame resize signal exists, so we debounce: set on a resize that
  // grew either dimension, clear ~120ms after the last resize event.
  let resizing = $state(false);
  let resizeTimer: number | null = null;
  let lastW = globalThis.innerWidth;
  let lastH = globalThis.innerHeight;
  function handleWindowResize() {
    const grew = globalThis.innerWidth > lastW || globalThis.innerHeight > lastH;
    lastW = globalThis.innerWidth;
    lastH = globalThis.innerHeight;
    if (grew && !resizing) resizing = true;
    if (resizeTimer) clearTimeout(resizeTimer);
    resizeTimer = window.setTimeout(() => { resizing = false; resizeTimer = null; }, 120);
  }

  // Current file/hunk derived from indices (diff mode)
  let currentFile = $derived(diffDisplay?.docs[contentTracking.currentFileIndex] ?? null);

  let currentHunk = $derived.by(() => {
    const hunks = currentFile?.doc.hunks;
    if (!hunks || hunks.length === 0) return null;
    return hunks[contentTracking.currentHunkIndex] ?? null;
  });

  // Current section derived from index (markdown mode)
  let currentSection = $derived.by(() => {
    if (!markdownMetadata || markdownMetadata.sections.length === 0) return null;
    return markdownMetadata.sections[contentTracking.currentSectionIndex] ?? null;
  });

  // Build breadcrumb for markdown sections
  let sectionBreadcrumb = $derived.by(() => {
    if (!markdownMetadata || contentTracking.currentSectionIndex < 0) return [];
    const sections = markdownMetadata.sections;
    const breadcrumb: SectionInfo[] = [];

    let idx: number | null = contentTracking.currentSectionIndex;
    while (idx !== null && idx >= 0 && idx < sections.length) {
      breadcrumb.unshift(sections[idx]);
      idx = sections[idx].parent_index;
    }

    return breadcrumb;
  });

  // Header display: show only the current (deepest) section
  let headerCurrentSection = $derived(sectionBreadcrumb.at(-1) ?? null);

  function updateCurrentPosition() {
    if (!contentEl) return;

    // Find the line at the top of the visible area by hit-testing. Robust to code
    // blocks / portals whose lines have a different offsetParent — offsetTop is not
    // globally monotonic, so reading/searching it picks the wrong line. This is one
    // O(1)-per-header hit test instead of reading offsetTop on all ~10k lines every frame.
    const rect = contentEl.getBoundingClientRect();
    const x = rect.left + 12;
    let lineEl: HTMLElement | null = null;
    let headerEl: HTMLElement | null = null;
    // Pierce the whole stack: a run of collapsed (header-only) files sticks its
    // headers one after another at the top edge. Hop past each by its own
    // rendered height — rather than a fixed probe depth, which runs out before
    // reaching real content when several collapsed files stack up — landing on
    // the covered line row underneath. Keep the last (bottommost) header as the
    // fallback when we never reach one (i.e. we're at the very start of a file).
    let y = rect.top + 1;
    let guard = 0;
    while (y < rect.bottom && !lineEl && guard++ < 64) {
      const row = document
        .elementsFromPoint(x, y)
        .reduce<HTMLElement | null>((found, el) => found ?? (el.closest('[data-display-idx]') as HTMLElement | null), null);
      if (!row) break;
      if (row.classList.contains('file-header-line')) {
        headerEl = row;
        y = row.getBoundingClientRect().bottom + 1;
      } else {
        lineEl = row;
      }
    }
    lineEl ??= headerEl;
    if (!lineEl) return;

    const displayIdx = parseInt(lineEl.dataset.displayIdx ?? '1', 10);
    if (diffDisplay) {
      // Diff mode: hunk boundaries live in walk display space
      contentTracking.updateFromLine(displayIdx);
    } else {
      // Markdown/source mode: section boundaries use source_line from the file
      const line = lines[displayIdx - 1];
      const sourceLineNum = line ? getLineNumber(line) : null;
      if (sourceLineNum !== null) contentTracking.updateFromLine(sourceLineNum);
    }
  }

  function handleContentScroll() {
    if (scrollRafId) return;
    scrollRafId = requestAnimationFrame(() => {
      scrollRafId = null;
      updateCurrentPosition();
    });
  }

  // Check if a line at the given display index is selectable.
  function isLineSelectable(displayIdx: number): boolean {
    // Diff mode: only walk rows are selectable — headers are structure.
    if (diffDisplay) return diffDisplay.byIndex.get(displayIdx)?.kind === 'row';
    const line = lines[displayIdx - 1];
    return line ? isSelectable(line) : false;
  }

  /**
   * Selection → anchor, routed by mode: the walk owns diff coordinates.
   * `side` scopes a split-view drag to one column; null (unified/flat)
   * keeps every row in the range.
   */
  function anchorForRange(range: Range, side: Side | null): Anchor | null {
    return diffDisplay ? selectionToDiffAnchor(range, diffDisplay, side) : selectionToAnchor(range, lines);
  }

  /** Side-less variant for programmatic display ranges (mermaid, excalidraw). */
  function anchorForSelection(range: Range): Anchor | null {
    return anchorForRange(range, null);
  }

  // Selection bounds (composable) — hunk/portal/codeblock boundary logic
  const selectionBounds = useSelectionBounds({
    getLines: () => lines,
    getDiffDisplay: () => diffDisplay,
  });

  // Draft slot: a new annotation's identity, minted the moment its slot comes
  // into existence (selection commit, gutter click, hover-comment, …) so the
  // draft→saved transition never changes the slot's id — the editor must not
  // remount on the first keystroke. Cleared when interaction returns to idle.
  let draft = $state<SlotRef | null>(null);

  function handleSelectionChange(anchor: Anchor | null) {
    if (!anchor) {
      draft = null;
      return;
    }
    // An existing annotation at the selection's end row claims the slot. The
    // draft shadows it anyway: emptying the editor deletes the entry mid-edit,
    // and the shadow is what keeps the slot (and the live editor) mounted
    // until the editor is dismissed.
    const span = annotationState.spanOfAnchor(anchor);
    const existing = span ? annotationState.atEndRow(span.end) : null;
    draft = existing ? { id: existing.id, anchor: existing.anchor } : { id: crypto.randomUUID(), anchor };
  }

  /** Open an annotation's editor, shadowing it as the active draft (see above). */
  function openAnnotationEditor(slot: SlotRef) {
    draft = { id: slot.id, anchor: slot.anchor };
    interaction.openEditor({ kind: 'annotation', id: slot.id });
  }

  // dispatch() calls onSelectionChange synchronously before this runs, so the
  // draft already reflects the just-committed selection by the time this reads it.
  function editorForSelection(): EditorKind | null {
    return draft ? { kind: 'annotation', id: draft.id } : null;
  }

  /** The id's anchor: a saved entry's if it has one, else the shadowing draft's. */
  function anchorForId(id: string): Anchor | null {
    return annotationState.getById(id)?.anchor ?? (draft?.id === id ? draft.anchor : null);
  }

  function spanForAnnotation(id: string): Range | null {
    // Prefer the store's memoized span; only a not-yet-saved draft needs
    // resolving here.
    const saved = annotationState.spanOf(id);
    if (saved) return saved;
    const anchor = anchorForId(id);
    return anchor ? annotationState.spanOfAnchor(anchor) : null;
  }

  // Interaction state (composable) — unified hover/selection state machine
  const interaction = useInteraction({
    isLineSelectable,
    constrainToBounds: selectionBounds.constrainToSelectionBounds,
    spanForAnnotation,
    anchorForRange,
    spanForDraft: (anchor) => annotationState.spanOfAnchor(anchor),
    anchorForAnnotation: anchorForId,
    editorForSelection,
    onSelectionChange: handleSelectionChange,
  });

  // Per-file collapse (composable) — diff mode only
  const fileCollapse = useFileCollapse(() => diffDisplay?.docs ?? [], {
    // Spec says "move selection to the header row", but header rows are not
    // selectable — clear instead (GitHub behavior).
    onCollapse: async (dv, opts) => {
      const r = interaction.range;
      if (r && r.end >= dv.headerDisplayIndex && r.start <= dv.endDisplayIndex) {
        if (interaction.phase === 'editing') interaction.closeEditor();
        interaction.clearSelection();
      }
      const h = interaction.hoverLine;
      if (h !== null && h > dv.headerDisplayIndex && h <= dv.endDisplayIndex) {
        interaction.handleLineLeave();
      }

      // Collapsing the file you're currently reading advances to the next one,
      // same as picking it from the file tree — collapsing means "done with this
      // file". Never fires for collapseAll (opts.bulk) or for a file collapsed
      // while positioned elsewhere, and never wraps past the last file.
      if (!opts?.bulk && contentTracking.currentFileIndex === dv.index) {
        const next = diffDisplay?.docs[dv.index + 1];
        if (next) {
          // Collapsing shrinks the current file's DOM extent; wait for that reflow
          // before measuring the next header's position, or scrollToDisplayIndex
          // computes its scroll delta against the stale (pre-collapse) layout and
          // overshoots into the next file's body instead of landing on its header.
          await tick();
          jumpToFile(next.headerDisplayIndex);
        }
      }
    },
  });

  // Annotation state (composable)
  const annotationState = useAnnotations({
    getLines: () => lines,
    getDisplay: () => diffDisplay,
  });

  // Exit mode state (composable)
  const exitModeState = useExitModes();

  // Mermaid diagram handling (composable)
  const mermaid = useMermaid({
    getLines: () => lines,
    getLabel: () => label,
    getMarkdownMetadata: () => markdownMetadata,
  });

  // Line segmentation (composable)
  const lineSegmentation = useLineSegments(() => lines);

  // Search (composable)
  async function scrollToDisplayIndex(displayIndex: number, block: ScrollLogicalPosition = 'center') {
    // Jumps into a collapsed file expand it first (GitHub behavior); the header
    // row itself is always rendered, so it never triggers an expand.
    const walkEntry = diffDisplay?.byIndex.get(displayIndex);
    const dv = walkEntry ? (diffDisplay?.docs[walkEntry.docIdx] ?? null) : null;
    if (dv && displayIndex > dv.headerDisplayIndex && fileCollapse.isCollapsed(dv.index)) {
      fileCollapse.expand(dv.index);
      await tick();
    }
    const target =
      contentEl?.querySelector(`[data-display-idx="${displayIndex}"]`) ??
      // Fall back to the file's header when the row itself isn't in the DOM.
      (dv ? contentEl?.querySelector(`[data-display-idx="${dv.headerDisplayIndex}"]`) : null);
    if (!target || !contentEl) return;
    if (block === 'start') {
      // Native scrollIntoView misjudges targets that are `position: sticky`
      // (file header rows) against their sticky siblings, especially when
      // jumping backward — compute the delta ourselves instead.
      const delta = target.getBoundingClientRect().top - contentEl.getBoundingClientRect().top;
      contentEl.scrollTop += delta;
    } else {
      target.scrollIntoView({ block });
    }
  }

  // File jumps land the file header at the top of the viewport. Centering it would
  // leave the *previous* file at the top, which is what current-file tracking reads.
  async function jumpToFile(startLine: number) {
    const dv = diffDisplay?.docs.find((d) => d.headerDisplayIndex === startLine);
    if (dv && fileCollapse.isCollapsed(dv.index)) {
      fileCollapse.expand(dv.index);
      await tick();
    }
    // Set current-file tracking directly rather than waiting on the scroll-driven
    // hit-test to re-derive it — expanding a collapsed file churns the DOM right as
    // we scroll, which can race that heuristic onto stale state.
    contentTracking.updateFromLine(startLine);
    scrollToDisplayIndex(startLine, 'start');
  }
  const search = useSearch(() => lines, scrollToDisplayIndex, () => diffDisplay);

  // Session comment state (global/file-level comment)
  let sessionComment: JSONContent | undefined = $state(undefined);

  // Overlay state (command palette, help, timeline are mutually exclusive)
  const overlay = useOverlay();
  let commandPaletteInitialState = $state<{ namespace: 'exit-modes'; mode: 'filter' } | undefined>(undefined);
  let tags: Tag[] = $state([]);

  // Tag creation from selection state
  let pendingTagCreation = $state<{
    editorKey: string;  // 'session' or annotation id
    from: number;
    to: number;
    text: string;
  } | null>(null);

  let pendingTagInsertion = $state<{
    editorKey: string;
    from: number;
    to: number;
    tag: Tag;
  } | null>(null);

  // Save modal state
  let saveModalOpen = $state(false);

  // Help overlay state is now managed by useOverlay()

  // --- History / Undo System ---

  /**
   * Capture current session state as a SessionData snapshot.
   */
  function captureSessionData(): SessionData {
    return {
      annotations: { ...annotationState.all },
      sessionComment: sessionComment ? JSON.parse(JSON.stringify(sessionComment)) : null,
      selectedExitMode: exitModeState.selectedId,
    };
  }

  /**
   * Restore session state from a SessionData snapshot.
   * Called on undo/redo.
   */
  async function restoreSessionData(data: SessionData): Promise<void> {
    // Restore annotations (diffs by id and syncs the backend)
    annotationState.restore(data.annotations);

    // Restore session comment
    sessionComment = data.sessionComment ? JSON.parse(JSON.stringify(data.sessionComment)) : undefined;

    // Restore exit mode
    if (data.selectedExitMode) {
      exitModeState.select(data.selectedExitMode);
    } else {
      exitModeState.clearSelection();
    }
  }

  // History composable for undo/redo
  const history = useHistory({
    onStateChange: async (data, label) => {
      if (label === 'Undo' || label === 'Redo') {
        await restoreSessionData(data);
      }
    },
  });

  /**
   * Push current state to history before a mutation.
   * Call this before making any change to session state.
   */
  function pushHistory(label: string): void {
    history.push(captureSessionData(), label);
  }

  // Content zoom state
  let contentZoom = $state(1.0);

  // Sync zoom to CSS variable for portal elements (tooltips, etc.)
  let appliedZoom = 1.0;
  $effect(() => {
    document.documentElement.style.setProperty('--content-zoom', String(contentZoom));
    if (contentZoom !== appliedZoom) {
      appliedZoom = contentZoom;
      // WebKit keeps stale cached layout inside content-visibility:auto
      // skipped lines when an inherited font-size changes: scrolled into
      // view later, their token boxes still sit at the old zoom's advance
      // widths while glyphs paint at the new size. Disable skipping for one
      // forced synchronous reflow so every line re-lays-out at the new
      // zoom, then restore it (all pre-paint, so nothing flashes).
      const root = document.documentElement;
      root.classList.add('zoom-relayout');
      void document.body.offsetHeight;
      root.classList.remove('zoom-relayout');
    }
  });

  async function updateAnnotation(id: string, content: JSONContent | null) {
    const anchor = anchorForId(id);
    if (!anchor) return;
    annotationState.upsert(id, anchor, content);
  }

  function closeCurrentEditor() {
    // Don't close if we're creating a tag from this editor - user will return after CP closes
    if (pendingTagCreation) return;
    if (interaction.phase !== 'editing') return;

    // An empty draft simply dies with the slot: no entry was ever created and
    // the draft itself clears via onSelectionChange when we return to idle.
    interaction.closeEditor();
  }

  // Session comment handlers
  function openSessionEditor() {
    interaction.openEditor({ kind: 'session' });
  }

  function closeSessionEditor() {
    // Don't close if we're creating a tag from this editor - user will return after CP closes
    if (pendingTagCreation?.editorKey === 'session') return;

    interaction.closeEditor();
  }

  async function updateSessionComment(content: JSONContent | null) {
    sessionComment = content ?? undefined;
    // Sync to backend
    const nodes = content ? extractContentNodes(content) : null;
    await invoke('set_session_comment', { content: nodes });
  }

  // Save modal handlers
  function openSaveModal() {
    saveModalOpen = true;
  }

  function closeSaveModal() {
    saveModalOpen = false;
  }

  async function handleSave(path: string) {
    const response = await invoke<SaveContentResponse>('save_content', { path });
    label = response.new_label;
    closeSaveModal();
    showToast(`Saved to ${response.saved_path}`);
  }

  // CommandPalette handlers
  function handleCommandPaletteClose() {
    overlay.close();
    // Clear pending states
    pendingTagCreation = null;
    commandPaletteInitialState = undefined;
  }

  // Handle events from CommandPalette (e.g., theme change)
  function handleCommandPaletteEvent(event: string, payload: unknown) {
    if (event === 'SET_THEME') {
      setTheme(payload as ThemePreference);
      overlay.close();
    } else if (event === 'JUMP_TO_FILE') {
      overlay.close();
      jumpToFile(payload as number);
    } else if (event === 'SET_DIFF_VIEW') {
      setDiffView(payload as DiffViewMode);
      overlay.close();
    }
  }

  // Handle request to create tag from selected text in an editor
  function handleRequestCreateTag(editorKey: string, text: string, from: number, to: number) {
    pendingTagCreation = { editorKey, text, from, to };
    overlay.openCommandPalette();
  }

  // Handle tag created via CommandPalette - trigger chip insertion
  function handleItemCreated(item: { id: string; name: string; values: Record<string, string> }, namespace: string) {
    if (namespace === 'tags' && pendingTagCreation) {
      const tag: Tag = {
        id: item.id,
        name: item.values.name || item.name,
        instruction: item.values.instruction || '',
      };
      pendingTagInsertion = {
        editorKey: pendingTagCreation.editorKey,
        from: pendingTagCreation.from,
        to: pendingTagCreation.to,
        tag,
      };
      pendingTagCreation = null;
      // Clear pending insertion after a tick to allow the editor to react
      setTimeout(() => {
        pendingTagInsertion = null;
      }, 0);
    }
  }

  function handleSetExitModeFromPalette(modeId: string) {
    exitModeState.selectById(modeId);
  }

  async function handleTagsChange(newTags: Tag[]) {
    // Find changed tag by comparing with current state
    const currentIds = new Set(tags.map(t => t.id));
    const newIds = new Set(newTags.map(t => t.id));

    // Check for deleted tags
    for (const tag of tags) {
      if (!newIds.has(tag.id)) {
        await invoke('delete_tag', { id: tag.id });
      }
    }

    // Check for added/updated tags
    for (const tag of newTags) {
      const existing = tags.find(t => t.id === tag.id);
      if (!existing || existing.name !== tag.name || existing.instruction !== tag.instruction) {
        await invoke('upsert_tag', { tag });
      }
    }

    tags = newTags;
  }

  function handleImagePasteBlocked() {
    showToast('Image paste is only supported in MCP mode');
  }

  /**
   * Upsert `content` under whichever annotation already spans `range` exactly,
   * or a freshly minted one. Returns false if the range isn't anchorable.
   */
  function upsertAtSpan(range: Range, content: JSONContent): boolean {
    const existing = annotationState.atSpan(range);
    const anchor = existing?.anchor ?? anchorForSelection(range);
    if (!anchor) return false;
    annotationState.upsert(existing?.id ?? crypto.randomUUID(), anchor, content);
    return true;
  }

  function handleFileRefCopied(path: string) {
    const alreadyShowing = toastMessage != null;
    showToast(alreadyShowing ? `New copied path: "${path}"` : `Copied: "${path}"`);
  }

  // Handle reporting a mermaid syntax error as an annotation
  async function handleReportMermaidError(displayRange: Range, errorMessage: string) {
    // Check if an annotation already spans exactly this range
    const existing = annotationState.atSpan(displayRange);

    if (existing?.content) {
      // Check if error node already exists (TipTap uses 'errorChip' type)
      const hasError = JSON.stringify(existing.content).includes('"type":"errorChip"');
      if (hasError) {
        // Highlight existing annotation
        interaction.setSelection(displayRange);
        showToast('Error already reported');
        return;
      }
    }

    // Create error content node
    const errorNode = {
      type: 'errorChip',
      attrs: { source: 'mermaid', message: errorMessage }
    };

    // Create or update annotation with error node
    const newContent: JSONContent = existing?.content ? {
      ...existing.content,
      content: [
        ...(existing.content.content || []),
        { type: 'paragraph', content: [errorNode] }
      ]
    } : {
      type: 'doc',
      content: [
        { type: 'paragraph', content: [errorNode] }
      ]
    };

    if (!upsertAtSpan(displayRange, newContent)) return;
    showToast('Error added to feedback');
  }

  async function handleExitModesChange(newModes: ExitMode[]) {
    // Find changed modes by comparing with current state
    const currentModes = exitModeState.modes;
    const newIds = new Set(newModes.map(m => m.id));

    // Check for deleted modes
    for (const mode of currentModes) {
      if (!newIds.has(mode.id)) {
        await invoke('delete_exit_mode', { id: mode.id });
      }
    }

    // Check for added/updated modes
    for (const mode of newModes) {
      const existing = currentModes.find(m => m.id === mode.id);
      if (!existing || existing.name !== mode.name || existing.instruction !== mode.instruction ||
          existing.color !== mode.color || existing.order !== mode.order) {
        await invoke('upsert_exit_mode', { mode });
      }
    }

    // Update composable state (handles index clamping)
    exitModeState.setModes(newModes);
  }

  // Open excalidraw from a mermaid code block (keeps annotation coupling here)
  async function openExcalidrawFromMermaid(
    sourceBlock: { start_line: number; end_line: number },
    annotationRange: { start: number; end: number }
  ) {
    // sourceBlock has source line numbers for extracting mermaid content
    // annotationRange has display indices for creating the annotation
    const existing = annotationState.atSpan(annotationRange);

    // If annotation exists with a chip, ask AnnotationEditor to open it
    // This reads from TipTap directly, avoiding stale annotationState reads
    if (existing?.content && findExcalidrawChip(existing.content)) {
      await emit('mermaid-open-excalidraw', { annotationId: existing.id });
      return;
    }

    // No existing chip - convert mermaid fresh
    const source = mermaid.getMermaidContent(sourceBlock.start_line, sourceBlock.end_line);
    try {
      const elements = await convertMermaidToExcalidraw(source);
      await invoke('open_excalidraw_window', {
        elements,
        // Unread for CodeBlock origin — results route back via `origin`, and the
        // annotation (with its id) is only created when the result arrives.
        annotationId: '',
        nodeRef: { type: 'Placeholder', id: `mermaid-${Date.now()}` },
        origin: { type: 'CodeBlock', start_line: annotationRange.start, end_line: annotationRange.end },
      });
    } catch (e) {
      showToast(`Failed to convert mermaid: ${e instanceof Error ? e.message : String(e)}`);
    }
  }

  // Get original lines content for a given range (for /replace command)
  function getOriginalLinesForRange(range: Range): string {
    const start = Math.min(range.start, range.end);
    const end = Math.max(range.start, range.end);
    const rangeLines: string[] = [];
    for (let i = start; i <= end; i++) {
      if (diffDisplay) {
        const entry = diffDisplay.byIndex.get(i);
        if (entry?.kind === 'row') rangeLines.push(entry.row.content);
      } else {
        const line = lines[i - 1]; // Convert to 0-indexed
        if (line) rangeLines.push(line.content);
      }
    }
    return rangeLines.join('\n');
  }

  /**
   * Unfold context around a hunk (S3). The backend grows the session's
   * document (splice + merge live there — one source of truth) and returns
   * the whole updated document; replacing it re-derives the walk.
   */
  async function expandContext(
    docIdx: number,
    hunkIdx: number,
    direction: 'up' | 'down',
    amount: 'step' | 'all',
  ): Promise<void> {
    const updated = await invoke<DiffDocument>('expand_context', {
      fileIndex: docIdx,
      hunkIndex: hunkIdx,
      direction,
      amount,
    });
    if (diffDocs) diffDocs[docIdx] = updated;
  }

  // Shared props for AnnotationSlot component (context provides most state)
  let annotationSlotProps = $derived({
    pendingTagInsertion,
    onUpdate: updateAnnotation,
    onUnseal: openAnnotationEditor,
    onDismiss: closeCurrentEditor,
    onRequestCreateTag: handleRequestCreateTag,
    onImagePasteBlocked: handleImagePasteBlocked,
    onFileRefCopied: handleFileRefCopied,
  });

  // Keyboard handling (composable)
  const keyboard = useKeyboard(
    {
      onShiftDown: () => interaction.handleShiftKeyDown(),
      onShiftUp: () => interaction.handleShiftKeyUp(),
      onTabCycle: (dir) => dir === 'forward' ? exitModeState.cycleForward() : exitModeState.cycleBackward(),
      onOpenSessionEditor: openSessionEditor,
      onOpenCommandPalette: () => overlay.openCommandPalette(),
      onOpenCommandPaletteWithNamespace: (namespace) => {
        commandPaletteInitialState = { namespace, mode: 'filter' };
        overlay.openCommandPalette(namespace);
      },
      onOpenSaveModal: openSaveModal,
      onCloseWindow: () => getCurrentWindow().close(),
      onOpenSearch: () => search.open(),
      onOpenHelp: () => overlay.openHelp(),
      onToggleFileTree: () => { if (diffDisplay) fileTree.toggle(); },
      onZoomIn: () => contentZoom = Math.min(contentZoom + 0.1, 3.0),
      onZoomOut: () => contentZoom = Math.max(contentZoom - 0.1, 0.5),
      onZoomReset: () => contentZoom = 1.0,
      onCommentHoveredLine: () => {
        if (interaction.hoverLine !== null) {
          const line = interaction.hoverLine;
          // selectLine transitions to committed, which mints the draft (or
          // binds the existing annotation) via onSelectionChange.
          interaction.selectLine(line);
          const editor = editorForSelection();
          if (editor) interaction.openEditor(editor);
        }
      },
    },
    {
      isEditorActive: () => interaction.phase === 'editing',
      isCommandPaletteOpen: () => overlay.isCommandPaletteOpen(),
      isSaveModalOpen: () => saveModalOpen,
      isHelpOverlayOpen: () => overlay.isHelpOpen(),
      isSearchOpen: () => search.isOpen,
      hasHoveredLine: () => interaction.hoverLine !== null,
      hasExitModes: () => exitModeState.modes.length > 0,
      isHoveredLineSelectable: () => interaction.hoverLine !== null && isLineSelectable(interaction.hoverLine),
    }
  );

  onMount(async () => {
    const window = getCurrentWindow();

    // Apply theme before any content renders (prevents flash)
    await initTheme();

    try {
      const res = await invoke<ContentResponse>("get_content");
      label = res.label;
      if (res.view.type === 'diff') {
        diffDocs = res.view.documents;
      } else {
        lines = res.view.lines;
      }
      loaded = true;
      tags = res.tags;
      exitModeState.initialize(res.exit_modes, res.selected_exit_mode_id);
      metadata = res.metadata;
      allowsImagePaste = res.allows_image_paste;

      // Build content trackers for scroll tracking
      if (res.view.type === 'diff') {
        fileCollapse.init();
      }
      if (res.metadata.type === 'markdown') {
        contentTracking.initializeMarkdown(res.metadata);
      }

      // Hydrate session comment from backend
      if (res.session_comment) {
        sessionComment = contentNodesToTipTap(res.session_comment);
      }

      // Listen for window close - this triggers output and exit
      const unlisten = await window.onCloseRequested(async (event) => {
        event.preventDefault();
        unlisten();  // Remove listener before closing to prevent re-entry

        try {
          // Flush any debounced annotation writes before the backend reads its
          // in-memory state — otherwise the last keystrokes never reach it.
          await annotationState.flush();
          await invoke('finish_review');
        } catch (e) {
          console.error('Failed to finish review:', e);
          await window.destroy(); // Fallback
        }
      });

      // Listen for Excalidraw results from CodeBlock origin (mermaid → excalidraw)
      interface CodeBlockExcalidrawResult {
        start_line: number;
        end_line: number;
        elements: string;
        png: string;
      }

      // This handler is for FIRST creation from mermaid only.
      // Re-edits use Annotation origin and go through AnnotationEditor → excalidraw-result.
      await listen<CodeBlockExcalidrawResult>('codeblock-excalidraw-result', (event) => {
        const { start_line, end_line, elements, png } = event.payload;
        const range = { start: start_line, end: end_line };

        // Create excalidraw chip node
        const chipNode = {
          type: 'excalidrawChip',
          attrs: { nodeId: crypto.randomUUID(), elements, image: png }
        };

        // Create new annotation with chip
        const newContent: JSONContent = {
          type: 'doc',
          content: [
            { type: 'paragraph', content: [chipNode] }
          ]
        };
        if (!upsertAtSpan(range, newContent)) return;
        showToast('Diagram saved as annotation');
      });
    } catch (e) {
      error = String(e);
    }
    // Show window after content is ready (started hidden to avoid flash)
    await window.show();

    // Reload config and invalidate file cache on window focus
    await listen('tauri://focus', async () => {
      // Invalidate file cache (for @ file references)
      invoke('invalidate_file_cache').catch(() => {
        // Ignore errors - cache invalidation is best-effort
      });

      // Reload config from disk (picks up changes from other windows)
      try {
        const snapshot = await invoke<ConfigSnapshot>('reload_config');
        tags = snapshot.tags;
        exitModeState.setModes(snapshot.exit_modes);
      } catch {
        // Ignore errors - reload is best-effort
      }
    });
  });
</script>

<svelte:window onkeydown={keyboard.handleKeyDown} onkeyup={keyboard.handleKeyUp} onresize={handleWindowResize} />

<WindowResizeHandles />

<main class="viewer" style:--mode-color={exitModeState.selectedMode?.color ?? 'transparent'}>
  {#if error}
    <div class="error">{error}</div>
  {:else if !loaded}
    <div class="loading">Loading...</div>
  {:else}
  <AnnotProvider
    {lines}
    {metadata}
    {tags}
    {allowsImagePaste}
    {contentZoom}
    {diffDisplay}
    {diffView}
    {setDiffView}
    interaction={interaction}
    annotations={annotationState}
    {draft}
    exitModes={exitModeState}
    {fileCollapse}
    {search}
    {mermaid}
    {showToast}
    {isLineSelectable}
    {getOriginalLinesForRange}
    {expandContext}
  >
  <div class="sticky-header">
    <Header
      {label}
      {currentFile}
      currentFileIndex={contentTracking.currentFileIndex}
      {currentHunk}
      {sectionBreadcrumb}
      {headerCurrentSection}
      hasSessionComment={sessionComment !== undefined}
      onOpenSessionEditor={openSessionEditor}
      onOpenSaveModal={openSaveModal}
      zoomLevel={contentZoom}
    />
    <SessionEditor
      content={sessionComment}
      isOpen={interaction.isSessionEditorOpen()}
      pendingTagInsertion={pendingTagInsertion?.editorKey === 'session' ? { from: pendingTagInsertion.from, to: pendingTagInsertion.to, tag: pendingTagInsertion.tag } : null}
      onUpdate={updateSessionComment}
      onOpen={openSessionEditor}
      onClose={closeSessionEditor}
      onRequestCreateTag={(text, from, to) => handleRequestCreateTag('session', text, from, to)}
      onImagePasteBlocked={handleImagePasteBlocked}
      onFileRefCopied={handleFileRefCopied}
    />
  </div>

  <PaneGroup direction="horizontal" class="viewer-body">
    {#if fileTree.isOpen && diffDisplay}
      <Pane order={1} defaultSize={22} minSize={12} maxSize={45} class="file-tree-pane">
        <FileTree
          docs={diffDisplay?.docs ?? []}
          currentIndex={contentTracking.currentFileIndex}
          onJump={jumpToFile}
          isDirExpanded={fileTree.isDirExpanded}
          toggleDir={fileTree.toggleDir}
        />
      </Pane>
      <PaneResizer class="file-tree-resizer" />
    {/if}

    <Pane order={2} class="content-pane">
    <div
      class="content"
      class:virtualized={virtualizeLines}
      class:resizing={resizing}
      class:shift-held={interaction.isShiftHeld}
      class:phase-idle={interaction.phase === 'idle'}
      class:phase-selecting={interaction.phase === 'selecting'}
      class:phase-committed={interaction.phase === 'committed'}
      class:phase-editing={interaction.phase === 'editing'}
      class:diff-mode={diffDisplay !== null}
      bind:this={contentEl}
      onscroll={handleContentScroll}
      onpointerdown={interaction.handleContentPointerDown}
      onpointermove={interaction.handlePointerMove}
      onpointerup={interaction.handleGlobalPointerUp}
      onmouseleave={interaction.handleContentLeave}
      role="presentation"
    >
      <div class="content-inner">
      {#if diffDisplay}
        <!-- Diff mode: RegularLines renders the walk; there are no flat lines. -->
        <RegularLines {annotationSlotProps} />
      {:else}
      {#each lineSegmentation.segments as segment}
        {#if segment.type === 'portal'}
          <Portal lines={segment.lines}>
            {#snippet annotationSlot(displayIndex, slot)}
              <AnnotationSlot slotRef={slot} {...annotationSlotProps} />
            {/snippet}
          </Portal>
        {:else if segment.type === 'codeblock'}
          {@const firstLineNum = getLineNumber(segment.lines[0]?.line)}
          {@const mermaidBlock = firstLineNum !== null ? mermaid.getMermaidBlockAt(firstLineNum) : null}
          {@const mermaidSource = mermaidBlock ? mermaid.getMermaidContent(mermaidBlock.start_line, mermaidBlock.end_line) : null}
          {@const excalidrawSupported = mermaidSource ? isMermaidExcalidrawSupported(mermaidSource) : true}
          {@const mermaidError = mermaidBlock ? mermaid.getMermaidError(mermaidBlock.start_line, mermaidBlock.end_line) : null}
          {@const annotationRange = mermaidBlock ? {
            start: segment.lines[1]?.displayIndex ?? segment.lines[0].displayIndex,
            end: segment.lines[segment.lines.length - 2]?.displayIndex ?? segment.lines[segment.lines.length - 1].displayIndex
          } : null}
          <CodeBlock
            lines={segment.lines}
            language={segment.language}
            color={segment.color}
            onMermaidOpen={mermaidBlock && !mermaidError ? () => mermaid.openMermaidWindow(mermaidBlock) : undefined}
            onExcalidrawOpen={mermaidBlock ? () => openExcalidrawFromMermaid(
              mermaidBlock,  // source block for content extraction
              annotationRange!
            ) : undefined}
            {excalidrawSupported}
            {mermaidError}
            onReportMermaidError={annotationRange ? (error) => handleReportMermaidError(annotationRange, error) : undefined}
          >
            {#snippet annotationSlot(displayIndex, slot)}
              <AnnotationSlot slotRef={slot} {...annotationSlotProps} />
            {/snippet}
          </CodeBlock>
        {:else if segment.type === 'table'}
          <Table lines={segment.lines}>
            {#snippet annotationSlot(displayIndex, slot)}
              <AnnotationSlot slotRef={slot} {...annotationSlotProps} />
            {/snippet}
          </Table>
        {:else if segment.type === 'separator'}
          <div class="line separator-line">
            <span class="gutter"></span>
            <span class="code"><hr class="separator" /></span>
          </div>
        {:else}
          <RegularLines
            lines={segment.lines}
            {annotationSlotProps}
          />
        {/if}
      {/each}
      {/if}
      </div>
    </div>
    </Pane>
  </PaneGroup>

  <!-- Footer / Status Bar -->
  <StatusBar />
  </AnnotProvider>
  {/if}
</main>

<SearchBar {search} />

{#if overlay.isCommandPaletteOpen()}
  <CommandPalette
    {tags}
    exitModes={exitModeState.modes}
    files={diffDisplay?.docs ?? []}
    diffView={diffDisplay ? diffView : null}
    onClose={handleCommandPaletteClose}
    onSetExitMode={handleSetExitModeFromPalette}
    onTagsChange={handleTagsChange}
    onExitModesChange={handleExitModesChange}
    {showToast}
    onOpenSaveModal={openSaveModal}
    initialState={pendingTagCreation
      ? { namespace: 'tags', mode: 'create', prefill: { instruction: pendingTagCreation.text } }
      : commandPaletteInitialState}
    onItemCreated={handleItemCreated}
    onEvent={handleCommandPaletteEvent}
  />
{/if}

{#if toastMessage}
  <div class="toast" class:exiting={toastExiting}>{toastMessage}</div>
{/if}

{#if saveModalOpen}
  <SaveModal
    defaultPath={label}
    onSave={handleSave}
    onCancel={closeSaveModal}
  />
{/if}

{#if overlay.isHelpOpen()}
  <HelpOverlay onClose={() => overlay.close()} />
{/if}

<style>
  /* Page-specific styles only - see src/styles/ for the design system */

  :global(body) {
    overflow: hidden;
  }

  :global(.header-btn) {
    display: inline-flex;
    align-items: center;
    padding: 4px 6px;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 6px;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 18px; /* unscaled: chrome */
  }

  :global(.header-btn:hover) {
    background: var(--bg-window);
    border-color: var(--border-subtle);
    color: var(--text-primary);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.05);
  }

  :global(.header-btn:focus-visible) {
    outline: none;
    border-color: var(--focus-ring);
  }

  /* Toggled-on state (e.g. split view active) */
  :global(.header-btn.active) {
    background: var(--bg-window);
    border-color: var(--border-subtle);
    color: var(--accent);
  }

  :global(.header-btn svg) {
    display: block;
  }

  .toast {
    position: fixed;
    bottom: 48px;
    left: 50%;
    transform: translateX(-50%);
    background: var(--text-primary);
    color: white;
    padding: 8px 16px;
    border-radius: 6px;
    font-size: 13px; /* unscaled: chrome */
    font-family: var(--font-ui);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
    z-index: 9999;
    animation: toast-in 0.2s ease forwards;
  }

  :global([data-theme="dark"]) .toast {
    color: var(--bg-main);
  }

  .toast.exiting {
    animation: toast-out 0.2s ease forwards;
  }

  @keyframes toast-in {
    from {
      opacity: 0;
      transform: translateX(-50%) translateY(8px);
    }
    to {
      opacity: 1;
      transform: translateX(-50%) translateY(0);
    }
  }

  @keyframes toast-out {
    from {
      opacity: 1;
      transform: translateX(-50%) translateY(0);
    }
    to {
      opacity: 0;
      transform: translateX(-50%) translateY(-8px);
    }
  }
</style>
