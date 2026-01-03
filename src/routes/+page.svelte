<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen, emit } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { onMount } from "svelte";
  import type { ContentResponse, ContentNode, ContentMetadata, Line, JSONContent, ExitMode, Tag, DiffMetadata, HunkInfo, MarkdownMetadata, SectionInfo, ConfigSnapshot } from "$lib/types";
  import { getLineNumber, getDiffKind, isSelectable, isPortalLine, isCodeBlockLine, isCodeBlockFence, isTableLine, isHorizontalRule, getFilePath } from "$lib/line-utils";
  import { rangeToKey, keyToRange, isLineInRange, validateRange, type Range } from "$lib/range";
  import { extractContentNodes, isContentEmpty, contentNodesToTipTap, findExcalidrawChip } from "$lib/tiptap";
  import { ContentTracker, type HunkPayload, type SectionPayload } from "$lib/content-tracker";
  import AnnotationSlot from "$lib/components/AnnotationSlot.svelte";
  import CopyDropdown from "$lib/CopyDropdown.svelte";
  import { CommandPalette } from "$lib/CommandPalette";
  import SaveModal from "$lib/SaveModal.svelte";
  import HelpOverlay from "$lib/HelpOverlay.svelte";
  import Portal from "$lib/components/embedded/Portal.svelte";
  import CodeBlock from "$lib/components/embedded/CodeBlock.svelte";
  import Table from "$lib/components/embedded/Table.svelte";
  import RegularLines from "$lib/components/embedded/RegularLines.svelte";
  import { Header, StatusBar, SessionEditor } from "$lib/components";
  import { useExitModes } from "$lib/composables/useExitModes.svelte";
  import { useContentTracking } from "$lib/composables/useContentTracking.svelte";
  import { useInteraction } from "$lib/composables/useInteraction.svelte";
  import { useAnnotations } from "$lib/composables/useAnnotations.svelte";
  import { useKeyboard } from "$lib/composables/useKeyboard.svelte";
  import { useSelectionBounds } from "$lib/composables/useSelectionBounds.svelte";
  import { useMermaid } from "$lib/composables/useMermaid.svelte";
  import { useLineSegments } from "$lib/composables/useLineSegments.svelte";
  import { useSearch } from "$lib/composables/useSearch.svelte";
  import { useBookmarks } from "$lib/composables/useBookmarks.svelte";
  import { useTerraformRegions } from "$lib/composables/useTerraformRegions.svelte";
  import SearchBar from "$lib/components/SearchBar.svelte";
  import { AnnotProvider } from "$lib/context";
  import type { SaveContentResponse } from "$lib/types";
  import { initTheme, setTheme, type ThemePreference } from "$lib/theme";
  import { convertMermaidToExcalidraw } from "$lib/mermaid-to-excalidraw";
  import { isMermaidExcalidrawSupported } from "$lib/mermaid-loader";

  let lines: Line[] = $state([]);
  let label = $state("");
  let error = $state("");
  let metadata = $state<ContentMetadata>({ type: 'plain' });
  let allowsImagePaste = $state(false);

  // Derived metadata for backwards compatibility
  let diffMetadata = $derived(metadata.type === 'diff' ? metadata : null);

  // =============================================================================
  // Coordinate System (Display Index)
  // =============================================================================
  // All selection coordinates use display indices (1-indexed positions in the
  // lines array). Display indices are inherently unique across all files/content.
  //
  // Source coordinates (path + line numbers) are extracted at the backend
  // boundary via validateRange() when calling Tauri commands.
  // =============================================================================

  let markdownMetadata = $derived(metadata.type === 'markdown' ? metadata : null);

  // Toast state
  let toastMessage = $state<string | null>(null);
  let toastExiting = $state(false);
  let toastTimeout: ReturnType<typeof setTimeout> | null = null;

  // Bookmark ID to edit when opening command palette (set by e key, cleared after use)
  let editBookmarkId = $state<string | null>(null);

  // Bookmarks composable (initialized in onMount after data is loaded)
  let bookmarkState: ReturnType<typeof useBookmarks> | null = $state(null);

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

  // Content tracking (composable)
  const contentTracking = useContentTracking();
  let contentEl: HTMLDivElement | null = $state(null);
  let scrollRafId: number | null = null;

  // Current file/hunk derived from indices (diff mode)
  let currentFile = $derived.by(() => {
    if (!diffMetadata || diffMetadata.files.length === 0) return null;
    return diffMetadata.files[contentTracking.currentFileIndex] ?? null;
  });

  let currentHunk = $derived.by(() => {
    if (!currentFile || currentFile.hunks.length === 0) return null;
    return currentFile.hunks[contentTracking.currentHunkIndex] ?? null;
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

    const lineEls = contentEl.querySelectorAll('.line');
    const scrollTop = contentEl.scrollTop;

    for (const el of lineEls) {
      const htmlEl = el as HTMLElement;
      if (htmlEl.offsetTop >= scrollTop) {
        const displayIdx = parseInt(htmlEl.dataset.displayIdx ?? '1', 10);
        const line = lines[displayIdx - 1];
        const sourceLineNum = line ? getLineNumber(line) : null;
        if (sourceLineNum === null) continue;

        contentTracking.updateFromLine(sourceLineNum);
        break;
      }
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
    const line = lines[displayIdx - 1];
    return line ? isSelectable(line) : false;
  }

  // Selection bounds (composable) — hunk/portal/codeblock boundary logic
  const selectionBounds = useSelectionBounds({
    getLines: () => lines,
    getDiffMetadata: () => diffMetadata,
    getHunkTracker: () => contentTracking.hunkTracker,
  });

  // Interaction state (composable) — unified hover/selection state machine
  const interaction = useInteraction({
    isLineSelectable,
    constrainToBounds: selectionBounds.constrainToSelectionBounds,
    onImmediateBookmark: async (context) => {
      // Called when 'b' was held during drag — create bookmark immediately
      if (!bookmarkState) return;
      await bookmarkState.toggleSelection(context.start, context.end);
      const shortId = bookmarkState.lastCreatedId?.slice(0, 3) ?? '';
      showToast(`Bookmarked as ${shortId} · [e] edit`);
    },
  });

  // Annotation state (composable)
  const annotationState = useAnnotations({
    getLines: () => lines,
  });

  // Exit mode state (composable)
  const exitModeState = useExitModes();

  // Mermaid diagram handling (composable)
  const mermaid = useMermaid({
    getLines: () => lines,
    getLabel: () => label,
    getMarkdownMetadata: () => markdownMetadata,
  });

  // Terraform regions (composable)
  const terraform = useTerraformRegions({
    getLines: () => lines,
  });

  // Line segmentation (composable)
  const lineSegmentation = useLineSegments(() => lines);

  // Search (composable)
  function scrollToDisplayIndex(displayIndex: number) {
    if (!contentEl) return;
    const targetY = (displayIndex - 1) * LINE_HEIGHT;
    const viewportCenter = viewportHeight / 2;
    contentEl.scrollTop = Math.max(0, targetY - viewportCenter);
  }
  const search = useSearch(() => lines, scrollToDisplayIndex);

  // Session comment state (global/file-level comment)
  let sessionComment: JSONContent | undefined = $state(undefined);
  let sessionEditorOpen = $state(false);

  // CommandPalette state
  let commandPaletteOpen = $state(false);
  let commandPaletteInitialState = $state<{ namespace: 'exit-modes'; mode: 'filter' } | undefined>(undefined);
  let tags: Tag[] = $state([]);

  // Tag creation from selection state
  let pendingTagCreation = $state<{
    editorKey: string;  // 'session' or rangeKey
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

  // Help overlay state
  let helpOverlayOpen = $state(false);

  // Content zoom state
  let contentZoom = $state(1.0);

  // Sync zoom to CSS variable for portal elements (tooltips, etc.)
  $effect(() => {
    document.documentElement.style.setProperty('--content-zoom', String(contentZoom));
  });

  // Virtual scrolling state
  const LINE_HEIGHT = 22;
  const BUFFER_LINES = 10;
  let scrollTop = $state(0);
  let viewportHeight = $state(700);

  // Virtual scroll computed values
  let startIndex = $derived(Math.max(0, Math.floor(scrollTop / LINE_HEIGHT) - BUFFER_LINES));
  let endIndex = $derived(Math.min(lines.length, Math.ceil((scrollTop + viewportHeight) / LINE_HEIGHT) + BUFFER_LINES));
  let visibleLines = $derived(lines.slice(startIndex, endIndex));
  let translateY = $derived(startIndex * LINE_HEIGHT);
  let totalHeight = $derived(lines.length * LINE_HEIGHT);

  // Get all annotation ranges for overlay rendering
  let annotationRanges = $derived(annotationState.allRanges());

  // Active editor range (for positioning the editor overlay)
  let activeEditorRange = $derived.by(() => {
    const sel = interaction.range;
    if (!sel || interaction.phase === 'selecting') return null;
    // Check if there's an existing annotation at the last selected line
    const lastLine = Math.max(sel.start, sel.end);
    const existing = annotationState.getAtLine(lastLine);
    if (existing) {
      const range = keyToRange(existing.key);
      return { key: existing.key, start: range.start, end: range.end };
    }
    // New annotation at selection
    const start = Math.min(sel.start, sel.end);
    const end = Math.max(sel.start, sel.end);
    return { key: rangeToKey({ start, end }), start, end };
  });

  async function updateAnnotation(rangeKey: string, content: JSONContent | null) {
    const range = keyToRange(rangeKey);
    await annotationState.upsert(range, content);
  }

  function sealCurrentAnnotation() {
    const sel = interaction.range;
    if (!sel) return;

    // Don't seal if we're creating a tag from this editor - user will return after CP closes
    if (pendingTagCreation) return;

    const key = rangeToKey(sel);
    const entry = annotationState.getByKey(key);
    if (entry) {
      annotationState.seal(key);
    } else {
      // Remove empty annotation
      annotationState.remove(key);
    }
    interaction.clearSelection();
  }

  // Session comment handlers
  function openSessionEditor() {
    sessionEditorOpen = true;
  }

  function closeSessionEditor() {
    // Don't close if we're creating a tag from this editor - user will return after CP closes
    if (pendingTagCreation?.editorKey === 'session') return;

    sessionEditorOpen = false;
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

  // Bookmark toggle handler
  async function handleToggleBookmark() {
    if (!bookmarkState) return;
    const wasBookmarked = bookmarkState.isSessionBookmarked;
    await bookmarkState.toggleSession();
    if (wasBookmarked) {
      showToast('Bookmark removed');
    } else {
      const shortId = bookmarkState.lastCreatedId?.slice(0, 3) ?? '';
      showToast(`Bookmarked as ${shortId} · [e] edit`);
    }
  }

  // Create or toggle selection bookmark handler
  async function handleCreateSelectionBookmark(context: { start: number; end: number }) {
    if (!bookmarkState) return;
    const existing = bookmarkState.findByLineRange(context.start, context.end);
    await bookmarkState.toggleSelection(context.start, context.end);
    if (existing) {
      showToast('Bookmark removed');
    } else {
      const shortId = bookmarkState.lastCreatedId?.slice(0, 3) ?? '';
      showToast(`Bookmarked as ${shortId} · [e] edit`);
    }
  }

  // Check if a display index is in any bookmarked range
  function isLineBookmarked(displayIdx: number): boolean {
    return bookmarkState?.isLineInBookmarkedRange(displayIdx) ?? false;
  }

  // Check if a display index is the first line of any bookmark
  function isFirstLineOfBookmark(displayIdx: number): boolean {
    return bookmarkState?.isFirstLineOfBookmark(displayIdx) ?? false;
  }

  // Delete bookmark by display index (for inline delete button)
  function deleteBookmarkAtLine(displayIdx: number): void {
    const id = bookmarkState?.getBookmarkIdAtStart(displayIdx);
    if (id) {
      bookmarkState?.delete(id);
      showToast('Bookmark removed');
    }
  }

  // Edit last created bookmark handler
  function handleEditLastBookmark() {
    if (bookmarkState?.lastCreatedId) {
      editBookmarkId = bookmarkState.lastCreatedId;
      commandPaletteOpen = true;
    }
  }

  // CommandPalette handlers
  function handleCommandPaletteClose() {
    commandPaletteOpen = false;
    // Clear pending states
    pendingTagCreation = null;
    editBookmarkId = null;
    commandPaletteInitialState = undefined;
  }

  async function handleBookmarkDeleted(id: string) {
    // Composable handles all state updates
    await bookmarkState?.delete(id);
  }

  async function handleBookmarkUpdated(id: string, label: string) {
    // Capture before await (onClose may clear editBookmarkId while we await)
    const wasEditTriggered = editBookmarkId === id;

    // Composable handles state update
    await bookmarkState?.update(id, label);

    // Show toast if edit was triggered via 'e' key
    if (wasEditTriggered) {
      const shortId = id.slice(0, 3);
      const displayLabel = label ? `"${label}"` : '(no label)';
      showToast(`${shortId} → ${displayLabel}`);
    }
  }

  // Handle events from CommandPalette (e.g., theme change)
  function handleCommandPaletteEvent(event: string, payload: unknown) {
    if (event === 'SET_THEME') {
      setTheme(payload as ThemePreference);
      commandPaletteOpen = false;
    }
  }

  // Handle request to create tag from selected text in an editor
  function handleRequestCreateTag(editorKey: string, text: string, from: number, to: number) {
    pendingTagCreation = { editorKey, text, from, to };
    commandPaletteOpen = true;
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

  // Handle reporting a mermaid syntax error as an annotation
  async function handleReportMermaidError(displayRange: Range, errorMessage: string) {
    // Check if annotation already exists at this range
    const rangeKey = rangeToKey(displayRange);
    const existing = annotationState.getByKey(rangeKey);

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

    await annotationState.upsert(displayRange, newContent);
    annotationState.seal(rangeKey);
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
    const rangeKey = `${annotationRange.start}-${annotationRange.end}`;
    const existing = annotationState.getByKey(rangeKey);

    // If annotation exists with a chip, ask AnnotationEditor to open it
    // This reads from TipTap directly, avoiding stale annotationState reads
    if (existing?.content && findExcalidrawChip(existing.content)) {
      await emit('mermaid-open-excalidraw', { rangeKey });
      return;
    }

    // No existing chip - convert mermaid fresh
    const source = mermaid.getMermaidContent(sourceBlock.start_line, sourceBlock.end_line);
    try {
      const elements = await convertMermaidToExcalidraw(source);
      await invoke('open_excalidraw_window', {
        elements,
        rangeKey,
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
      const line = lines[i - 1]; // Convert to 0-indexed
      if (line) {
        rangeLines.push(line.content);
      }
    }
    return rangeLines.join('\n');
  }

  // Shared props for AnnotationSlot component (context provides most state)
  let annotationSlotProps = $derived({
    pendingTagInsertion,
    onUpdate: updateAnnotation,
    onDismiss: sealCurrentAnnotation,
    onRequestCreateTag: handleRequestCreateTag,
    onImagePasteBlocked: handleImagePasteBlocked,
  });

  // Keyboard handling (composable)
  const keyboard = useKeyboard(
    {
      onShiftDown: () => interaction.handleShiftKeyDown(),
      onShiftUp: () => interaction.handleShiftKeyUp(),
      onTabCycle: (dir) => dir === 'forward' ? exitModeState.cycleForward() : exitModeState.cycleBackward(),
      onOpenSessionEditor: openSessionEditor,
      onOpenCommandPalette: () => commandPaletteOpen = true,
      onOpenCommandPaletteWithNamespace: (namespace) => {
        commandPaletteInitialState = { namespace, mode: 'filter' };
        commandPaletteOpen = true;
      },
      onOpenSaveModal: openSaveModal,
      onOpenSearch: () => search.open(),
      onOpenHelp: () => helpOverlayOpen = true,
      onCreateSessionBookmark: handleToggleBookmark,
      onCreateSelectionBookmark: handleCreateSelectionBookmark,
      onEditLastBookmark: handleEditLastBookmark,
      onZoomIn: () => contentZoom = Math.min(contentZoom + 0.1, 3.0),
      onZoomOut: () => contentZoom = Math.max(contentZoom - 0.1, 0.5),
      onZoomReset: () => contentZoom = 1.0,
      onCommentHoveredLine: () => {
        if (interaction.hoverLine !== null) {
          interaction.selectLine(interaction.hoverLine);
        }
      },
      onTerraformHoveredLine: () => {
        if (interaction.hoverLine !== null) {
          interaction.selectLine(interaction.hoverLine);
          interaction.openTerraform();
        }
      },
      onDragModifierPress: (key) => interaction.setDragModifier(key),
      onConfirmChoice: (action) => interaction.confirmChoice(action),
      onCancelChoice: () => interaction.cancelChoice(),
    },
    {
      isEditorActive: () => !!interaction.range || sessionEditorOpen,
      isCommandPaletteOpen: () => commandPaletteOpen,
      isSaveModalOpen: () => saveModalOpen,
      isHelpOverlayOpen: () => helpOverlayOpen,
      isSearchOpen: () => search.isOpen,
      hasHoveredLine: () => interaction.hoverLine !== null,
      hasExitModes: () => exitModeState.modes.length > 0,
      isHoveredLineSelectable: () => interaction.hoverLine !== null && isLineSelectable(interaction.hoverLine),
      hasLastCreatedBookmark: () => !!bookmarkState?.lastCreatedId,
      getBookmarkContext: () => interaction.getBookmarkContext(),
      getPhase: () => interaction.phase,
      isShiftHeld: () => interaction.isShiftHeld,
      isPendingChoice: () => interaction.pendingChoice,
    }
  );

  onMount(async () => {
    const window = getCurrentWindow();

    // Apply theme before any content renders (prevents flash)
    await initTheme();

    try {
      const res = await invoke<ContentResponse>("get_content");
      label = res.label;
      lines = res.lines;
      tags = res.tags;
      bookmarkState = useBookmarks(res.bookmarks);
      exitModeState.initialize(res.exit_modes, res.selected_exit_mode_id);
      metadata = res.metadata;
      allowsImagePaste = res.allows_image_paste;

      // Build content trackers for scroll tracking
      if (res.metadata.type === 'diff') {
        contentTracking.initializeDiff(res.metadata);
      }
      if (res.metadata.type === 'markdown') {
        contentTracking.initializeMarkdown(res.metadata);
      }

      // Hydrate session comment from backend
      if (res.session_comment) {
        sessionComment = contentNodesToTipTap(res.session_comment);
      }

      // Load terraform regions for visual indicators
      const firstSourceLine = lines.find(l => l.origin.type === 'source' || l.origin.type === 'diff');
      const firstPath = firstSourceLine ? getFilePath(firstSourceLine) : null;
      if (firstPath) {
        terraform.loadAll(firstPath);
      }

      // Listen for window close - this triggers output and exit
      const unlisten = await window.onCloseRequested(async (event) => {
        event.preventDefault();
        unlisten();  // Remove listener before closing to prevent re-entry

        try {
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
        const rangeKey = rangeToKey(range);

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
        annotationState.upsert(range, newContent);
        annotationState.seal(rangeKey);
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
        bookmarkState?.reloadFromSnapshot(snapshot.bookmarks);
      } catch {
        // Ignore errors - reload is best-effort
      }
    });
  });
</script>

<svelte:window onkeydown={keyboard.handleKeyDown} onkeyup={keyboard.handleKeyUp} />

<main class="viewer" style:--mode-color={exitModeState.selectedMode?.color ?? 'transparent'}>
  {#if error}
    <div class="error">{error}</div>
  {:else if !bookmarkState || lines.length === 0}
    <div class="loading">Loading...</div>
  {:else}
  <AnnotProvider
    {lines}
    {metadata}
    {tags}
    {allowsImagePaste}
    {contentZoom}
    interaction={interaction}
    annotations={annotationState}
    exitModes={exitModeState}
    {search}
    {mermaid}
    bookmarks={bookmarkState}
    {terraform}
    {showToast}
    {isLineSelectable}
    {getOriginalLinesForRange}
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
      onCreateBookmark={handleToggleBookmark}
      zoomLevel={contentZoom}
    />
    <div style:zoom={contentZoom}>
      <SessionEditor
        content={sessionComment}
        isOpen={sessionEditorOpen}
        pendingTagInsertion={pendingTagInsertion?.editorKey === 'session' ? { from: pendingTagInsertion.from, to: pendingTagInsertion.to, tag: pendingTagInsertion.tag } : null}
        onUpdate={updateSessionComment}
        onOpen={openSessionEditor}
        onClose={closeSessionEditor}
        onRequestCreateTag={(text, from, to) => handleRequestCreateTag('session', text, from, to)}
        onImagePasteBlocked={handleImagePasteBlocked}
      />
    </div>
  </div>

    <div
      class="content"
      class:shift-held={interaction.isShiftHeld}
      class:phase-idle={interaction.phase === 'idle'}
      class:phase-hovering={interaction.phase === 'hovering'}
      class:phase-selecting={interaction.phase === 'selecting'}
      class:phase-committed={interaction.phase === 'committed'}
      class:phase-editing={interaction.phase === 'editing'}
      class:diff-mode={diffMetadata !== null}
      bind:this={contentEl}
      onscroll={handleContentScroll}
      onpointerdown={interaction.handleContentPointerDown}
      onpointermove={interaction.handlePointerMove}
      onpointerup={interaction.handleGlobalPointerUp}
      onmouseleave={interaction.handleContentLeave}
      role="presentation"
    >
      <div
        class="content-inner"
        style:zoom={contentZoom}
      >
      {#each lineSegmentation.segments as segment}
        {#if segment.type === 'portal'}
          <Portal lines={segment.lines} {isLineBookmarked} {isFirstLineOfBookmark} {deleteBookmarkAtLine}>
            {#snippet annotationSlot(displayIndex, rangeKey)}
              <AnnotationSlot {rangeKey} {...annotationSlotProps} />
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
            {isLineBookmarked}
            {isFirstLineOfBookmark}
            {deleteBookmarkAtLine}
            onMermaidOpen={mermaidBlock && !mermaidError ? () => mermaid.openMermaidWindow(mermaidBlock) : undefined}
            onExcalidrawOpen={mermaidBlock ? () => openExcalidrawFromMermaid(
              mermaidBlock,  // source block for content extraction
              annotationRange!
            ) : undefined}
            {excalidrawSupported}
            {mermaidError}
            onReportMermaidError={annotationRange ? (error) => handleReportMermaidError(annotationRange, error) : undefined}
          >
            {#snippet annotationSlot(displayIndex, rangeKey)}
              <AnnotationSlot {rangeKey} {...annotationSlotProps} />
            {/snippet}
          </CodeBlock>
        {:else if segment.type === 'table'}
          <Table lines={segment.lines} {isLineBookmarked} {isFirstLineOfBookmark} {deleteBookmarkAtLine}>
            {#snippet annotationSlot(displayIndex, rangeKey)}
              <AnnotationSlot {rangeKey} {...annotationSlotProps} />
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
            {isLineBookmarked}
            {isFirstLineOfBookmark}
            {deleteBookmarkAtLine}
            {annotationSlotProps}
          />
        {/if}
      {/each}
      </div>
    </div>

  <!-- Footer / Status Bar -->
  <div style:zoom={contentZoom}>
    <StatusBar />
  </div>
  </AnnotProvider>
  {/if}
</main>

<div style:zoom={contentZoom}>
  <SearchBar {search} />
</div>

{#if commandPaletteOpen && bookmarkState}
  <CommandPalette
    {tags}
    bookmarks={bookmarkState.all}
    exitModes={exitModeState.modes}
    zoomLevel={contentZoom}
    onClose={handleCommandPaletteClose}
    onSetExitMode={handleSetExitModeFromPalette}
    onTagsChange={handleTagsChange}
    onExitModesChange={handleExitModesChange}
    onBookmarkDeleted={handleBookmarkDeleted}
    onBookmarkUpdated={handleBookmarkUpdated}
    {showToast}
    onOpenSaveModal={openSaveModal}
    initialState={pendingTagCreation
      ? { namespace: 'tags', mode: 'create', prefill: { instruction: pendingTagCreation.text } }
      : editBookmarkId
        ? { namespace: 'bookmarks', mode: 'edit', itemId: editBookmarkId }
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

{#if helpOverlayOpen}
  <HelpOverlay onClose={() => helpOverlayOpen = false} />
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
    font-size: 18px;
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
    font-size: 13px;
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
