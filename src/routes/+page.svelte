<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { onMount } from "svelte";
  import type { ContentResponse, ContentNode, ContentMetadata, Line, JSONContent, ExitMode, Tag, DiffMetadata, HunkInfo, MarkdownMetadata, SectionInfo } from "$lib/types";
  import { getLineNumber, getDiffKind, isSelectable, isPortalLine, isCodeBlockLine, isCodeBlockFence, isTableLine, isHorizontalRule, getFilePath, extractCodeBlockContent } from "$lib/line-utils";
  import { rangeToKey, keyToRange, isLineInRange, validateRange, type Range } from "$lib/range";
  import { extractContentNodes, isContentEmpty, contentNodesToTipTap } from "$lib/tiptap";
  import { ContentTracker, type HunkPayload, type SectionPayload } from "$lib/content-tracker";
  import AnnotationEditor from "$lib/AnnotationEditor.svelte";
  import CopyDropdown from "$lib/CopyDropdown.svelte";
  import { CommandPalette } from "$lib/CommandPalette";
  import SaveModal from "$lib/SaveModal.svelte";
  import Portal from "$lib/components/embedded/Portal.svelte";
  import CodeBlock from "$lib/components/embedded/CodeBlock.svelte";
  import Table from "$lib/components/embedded/Table.svelte";
  import { Header, StatusBar, SessionEditor } from "$lib/components";
  import { useExitModes } from "$lib/composables/useExitModes.svelte";
  import { useContentTracking } from "$lib/composables/useContentTracking.svelte";
  import { useInteraction } from "$lib/composables/useInteraction.svelte";
  import { useAnnotations } from "$lib/composables/useAnnotations.svelte";
  import { useKeyboard } from "$lib/composables/useKeyboard.svelte";
  import type { SaveContentResponse } from "$lib/types";
  import { initTheme, setTheme, type ThemePreference } from "$lib/theme";

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

  // Depth-based header display: H1 always, H2 only at depth 2, else ellipsis + current
  let headerRootSection = $derived(sectionBreadcrumb.find(s => s.level === 1) ?? null);
  let headerH2Section = $derived(sectionBreadcrumb.find(s => s.level === 2) ?? null);
  let headerCurrentSection = $derived(sectionBreadcrumb.at(-1) ?? null);
  let headerCurrentDepth = $derived(headerCurrentSection?.level ?? 0);

  // =============================================================================
  // Line Segmentation (for portal/codeblock-aware rendering)
  // =============================================================================

  /** Segment type for rendering: regular lines, portal group, code block group, table group, or separator */
  type LineSegment =
    | { type: 'regular'; lines: { line: Line; displayIndex: number }[] }
    | { type: 'portal'; lines: { line: Line; displayIndex: number }[] }
    | { type: 'codeblock'; lines: { line: Line; displayIndex: number }[]; language: string | null }
    | { type: 'table'; lines: { line: Line; displayIndex: number }[] }
    | { type: 'separator'; lines: { line: Line; displayIndex: number }[] };

  /** Get the language from a code block start line */
  function getCodeBlockLanguage(line: Line): string | null {
    if (line.semantics.type === 'markdown' && line.semantics.kind === 'code_block_start') {
      return line.semantics.language;
    }
    return null;
  }

  /** Group lines into segments for portal/codeblock/table-aware rendering */
  let lineSegments = $derived.by((): LineSegment[] => {
    const segments: LineSegment[] = [];
    let currentSegment: LineSegment | null = null;

    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];
      const displayIndex = i + 1;
      const isPortal = isPortalLine(line);
      const isCodeBlock = isCodeBlockLine(line);
      const isTable = isTableLine(line);
      const isSeparator = isHorizontalRule(line);

      if (isSeparator) {
        // Horizontal rule - standalone separator segment
        if (currentSegment) segments.push(currentSegment);
        segments.push({ type: 'separator', lines: [{ line, displayIndex }] });
        currentSegment = null;
      } else if (isPortal) {
        // Portal line - portals take priority over code blocks and tables
        const isPortalHeader = line.semantics.type === 'portal' && line.semantics.kind === 'header';
        if (currentSegment?.type === 'portal' && !isPortalHeader) {
          // Continue current portal segment (unless this is a new portal header)
          currentSegment.lines.push({ line, displayIndex });
        } else {
          // Start new portal segment
          if (currentSegment) segments.push(currentSegment);
          currentSegment = { type: 'portal', lines: [{ line, displayIndex }] };
        }
      } else if (isCodeBlock) {
        // Code block line
        const isCodeBlockStart = line.semantics.type === 'markdown' && line.semantics.kind === 'code_block_start';
        if (currentSegment?.type === 'codeblock' && !isCodeBlockStart) {
          // Continue current code block segment (unless this is a new code block)
          currentSegment.lines.push({ line, displayIndex });
        } else {
          // Start new code block segment
          if (currentSegment) segments.push(currentSegment);
          const language = getCodeBlockLanguage(line);
          currentSegment = { type: 'codeblock', lines: [{ line, displayIndex }], language };
        }
      } else if (isTable) {
        // Table row line
        if (currentSegment?.type === 'table') {
          // Continue current table segment
          currentSegment.lines.push({ line, displayIndex });
        } else {
          // Start new table segment
          if (currentSegment) segments.push(currentSegment);
          currentSegment = { type: 'table', lines: [{ line, displayIndex }] };
        }
      } else {
        // Regular line
        if (currentSegment?.type === 'regular') {
          // Continue current regular segment
          currentSegment.lines.push({ line, displayIndex });
        } else {
          // Start new regular segment
          if (currentSegment) segments.push(currentSegment);
          currentSegment = { type: 'regular', lines: [{ line, displayIndex }] };
        }
      }
    }

    if (currentSegment) segments.push(currentSegment);
    return segments;
  });

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

  // Get hunk bounds for a line (returns null if line is a header or not in diff mode)
  function getHunkBounds(lineNum: number): { start: number; end: number } | null {
    const tracker = contentTracking.hunkTracker;
    if (!diffMetadata || !tracker || tracker.length === 0) return null;

    const boundary = tracker.findAt(lineNum);
    if (!boundary) return null;

    const boundaries = tracker.all();

    // Find this boundary's index
    const boundaryIdx = boundaries.findIndex(
      b => b.data.fileIndex === boundary.data.fileIndex && b.data.hunkIndex === boundary.data.hunkIndex
    );

    // The hunk starts at the @@ line (header), but selectable content starts on next line
    const hunkStart = boundary.line + 1;

    // Find end: next hunk in SAME file, or file's end
    let hunkEnd: number;
    const nextBoundary = boundaries[boundaryIdx + 1];

    if (nextBoundary && nextBoundary.data.fileIndex === boundary.data.fileIndex) {
      // Next hunk in same file - end before its header line
      hunkEnd = nextBoundary.line - 1;
    } else {
      // Last hunk in this file - end at file's end_line
      const file = diffMetadata.files[boundary.data.fileIndex];
      hunkEnd = file?.end_line ?? lines.length;
    }

    return { start: hunkStart, end: hunkEnd };
  }

  // Constrain a line number to valid hunk bounds
  function constrainToHunkBounds(lineNum: number, anchorLine: number): number {
    const bounds = getHunkBounds(anchorLine);
    if (!bounds) return lineNum; // No bounds in non-diff mode

    // Clamp to hunk bounds
    return Math.max(bounds.start, Math.min(bounds.end, lineNum));
  }

  // Get portal bounds for a line (returns null if line is not in a portal)
  // Boundary detection: uses semantics and line number discontinuity
  function getPortalBounds(lineNum: number): { start: number; end: number } | null {
    const line = lines[lineNum - 1];
    if (!line) return null;

    // Check if this line is in a portal
    if (!isPortalLine(line)) return null;

    const currentPath = getFilePath(line);
    const currentLineNum = getLineNumber(line);

    let start = lineNum;
    let end = lineNum;

    // Scan backwards to find start
    // Stop at: non-portal line, different path, or line number gap > 1
    let prevLineNum = currentLineNum;
    for (let i = lineNum - 2; i >= 0; i--) {
      const prevLine = lines[i];
      if (!isPortalLine(prevLine)) break;

      const path = getFilePath(prevLine);
      const num = getLineNumber(prevLine);

      // Different path means different portal
      if (path !== currentPath) break;

      // Line number gap > 1 indicates portal boundary (unless virtual)
      if (prevLineNum !== null && num !== null && Math.abs(prevLineNum - num) > 1) break;

      start = i + 1; // 1-indexed
      prevLineNum = num;
    }

    // Scan forwards to find end
    prevLineNum = currentLineNum;
    for (let i = lineNum; i < lines.length; i++) {
      const nextLine = lines[i];
      if (!isPortalLine(nextLine)) break;

      const path = getFilePath(nextLine);
      const num = getLineNumber(nextLine);

      // Different path means different portal
      if (path !== currentPath) break;

      // Line number gap > 1 indicates portal boundary
      if (prevLineNum !== null && num !== null && Math.abs(num - prevLineNum) > 1) break;

      end = i + 1; // 1-indexed
      prevLineNum = num;
    }

    return { start, end };
  }

  // Get code block bounds for a line (returns null if line is not in a code block)
  function getCodeBlockBounds(lineNum: number): { start: number; end: number } | null {
    const line = lines[lineNum - 1];
    if (!line) return null;

    // Check if this line is in a code block
    if (!isCodeBlockLine(line)) return null;

    let start = lineNum;
    let end = lineNum;

    // Scan backwards to find start (code_block_start fence)
    for (let i = lineNum - 2; i >= 0; i--) {
      const prevLine = lines[i];
      if (!isCodeBlockLine(prevLine)) break;

      start = i + 1; // 1-indexed

      // If we hit the start fence, stop
      if (prevLine.semantics.type === 'markdown' && prevLine.semantics.kind === 'code_block_start') {
        break;
      }
    }

    // Scan forwards to find end (code_block_end fence)
    for (let i = lineNum; i < lines.length; i++) {
      const nextLine = lines[i];
      if (!isCodeBlockLine(nextLine)) break;

      end = i + 1; // 1-indexed

      // If we hit the end fence, stop
      if (nextLine.semantics.type === 'markdown' && nextLine.semantics.kind === 'code_block_end') {
        break;
      }
    }

    return { start, end };
  }

  // Constrain a line number to valid selection bounds (combines hunk + portal bounds)
  function constrainToSelectionBounds(lineNum: number, anchorLine: number): number {
    // First apply hunk bounds (for diff mode)
    let constrained = constrainToHunkBounds(lineNum, anchorLine);

    // Then check portal bounds (if anchor is in a portal, stay within it)
    const portalBounds = getPortalBounds(anchorLine);
    if (portalBounds) {
      constrained = Math.max(portalBounds.start, Math.min(portalBounds.end, constrained));
    }

    // If target would cross into a portal from outside, clamp to anchor
    const anchorLine_ = lines[anchorLine - 1];
    const targetLine = lines[lineNum - 1];
    const anchorIsPortal = anchorLine_ ? isPortalLine(anchorLine_) : false;
    const targetIsPortal = targetLine ? isPortalLine(targetLine) : false;

    if (anchorIsPortal !== targetIsPortal) {
      // Crossing portal boundary - prevent it
      return anchorLine;
    }

    return constrained;
  }

  // Interaction state (composable) — unified hover/selection state machine
  const interaction = useInteraction({
    isLineSelectable,
    constrainToBounds: constrainToSelectionBounds,
  });

  // Derived values for component compatibility (temporary during migration)
  let interactionSelection = $derived(interaction.range);
  let interactionIsDragging = $derived(interaction.phase === 'selecting');
  let interactionHoveredIdx = $derived(interaction.hoverLine);

  // Adapter handlers for embedded components (MouseEvent → PointerEvent)
  function handleGutterPointerDown(displayIdx: number, e: MouseEvent) {
    interaction.handlePointerDown(displayIdx, e as PointerEvent);
  }

  function handleAddPointerDown(displayIdx: number, e: MouseEvent) {
    interaction.handlePointerDown(displayIdx, e as PointerEvent);
  }

  // Annotation state (composable)
  const annotationState = useAnnotations({
    getLines: () => lines,
  });

  // Derived Map for Portal compatibility (converts Record to Map)
  let annotationsMap = $derived.by(() => {
    const map = new Map<string, JSONContent>();
    for (const [key, entry] of Object.entries(annotationState.annotations)) {
      map.set(key, entry.content);
    }
    return map;
  });

  // Exit mode state (composable)
  const exitModeState = useExitModes();

  // Session comment state (global/file-level comment)
  let sessionComment: JSONContent | undefined = $state(undefined);
  let sessionEditorOpen = $state(false);

  // CommandPalette state
  let commandPaletteOpen = $state(false);
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

  // Content zoom state
  let contentZoom = $state(1.0);

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

  // Derived: last line of current selection (for positioning editor)
  let lastSelectedLine = $derived.by(() => {
    const sel = interaction.range;
    if (!sel) return null;
    return Math.max(sel.start, sel.end);
  });

  async function updateAnnotation(content: JSONContent | null) {
    const sel = interaction.range;
    if (!sel) return;
    await annotationState.upsert(sel, content);
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

  // CommandPalette handlers
  function handleCommandPaletteClose() {
    commandPaletteOpen = false;
    // Clear pending tag creation if cancelled
    pendingTagCreation = null;
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

  // Get annotation info for a specific display index (is it the last line of any annotation?)
  function getAnnotationAtLine(displayIdx: number): { key: string; content: JSONContent } | null {
    return annotationState.getAtLine(displayIdx);
  }

  // Check if a display index is selected (full selection highlight)
  function isSelected(displayIdx: number): boolean {
    const sel = interaction.range;
    if (!sel) return false;
    // Show selection highlight for selecting/committed/editing phases
    const phase = interaction.phase;
    if (phase === 'idle' || phase === 'hovering') return false;
    return isLineInRange(displayIdx, sel);
  }

  // Check if a display index is in preview state (hover - lighter highlight)
  function isPreview(displayIdx: number): boolean {
    return interaction.isLinePreview(displayIdx);
  }

  // Check if a line starts a mermaid code block
  function getMermaidBlockAt(lineNum: number) {
    if (!markdownMetadata?.code_blocks) return null;
    return markdownMetadata.code_blocks.find(
      b => b.start_line === lineNum && b.language === 'mermaid'
    ) ?? null;
  }

  // Extract mermaid content from a code block (excluding fence lines)
  function getMermaidContent(startLine: number, endLine: number): string {
    return extractCodeBlockContent(lines, startLine, endLine, label);
  }

  async function openMermaidWindow(block: { start_line: number; end_line: number }) {
    const source = getMermaidContent(block.start_line, block.end_line);
    try {
      await invoke('open_mermaid_window', {
        source,
        filePath: label,
        startLine: block.start_line,
        endLine: block.end_line,
      });
    } catch (e) {
      console.error('Failed to open mermaid window:', e);
    }
  }

  // Check if a display index has an annotation
  function hasAnnotation(displayIdx: number): boolean {
    return annotationState.hasAnnotation(displayIdx);
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

  // Keyboard handling (composable)
  const keyboard = useKeyboard(
    {
      onShiftDown: () => interaction.handleShiftKeyDown(),
      onShiftUp: () => interaction.handleShiftKeyUp(),
      onTabCycle: (dir) => dir === 'forward' ? exitModeState.cycleForward() : exitModeState.cycleBackward(),
      onOpenSessionEditor: openSessionEditor,
      onOpenCommandPalette: () => commandPaletteOpen = true,
      onOpenSaveModal: openSaveModal,
      onZoomIn: () => contentZoom = Math.min(contentZoom + 0.1, 3.0),
      onZoomOut: () => contentZoom = Math.max(contentZoom - 0.1, 0.5),
      onZoomReset: () => contentZoom = 1.0,
      onCommentHoveredLine: () => {
        if (interaction.hoverLine !== null) {
          interaction.selectLine(interaction.hoverLine);
        }
      },
    },
    {
      isEditorActive: () => !!interaction.range || sessionEditorOpen,
      isCommandPaletteOpen: () => commandPaletteOpen,
      isSaveModalOpen: () => saveModalOpen,
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
      lines = res.lines;
      tags = res.tags;
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
    } catch (e) {
      error = String(e);
    }
    // Show window after content is ready (started hidden to avoid flash)
    await window.show();
  });
</script>

<svelte:window onkeydown={keyboard.handleKeyDown} onkeyup={keyboard.handleKeyUp} />

<main class="viewer" style:--mode-color={exitModeState.selectedMode?.color ?? 'transparent'}>
  <div class="sticky-header">
    <Header
      {label}
      {metadata}
      {currentFile}
      currentFileIndex={contentTracking.currentFileIndex}
      {currentHunk}
      {sectionBreadcrumb}
      {headerRootSection}
      {headerH2Section}
      {headerCurrentSection}
      {headerCurrentDepth}
      hasSessionComment={sessionComment !== undefined}
      onOpenSessionEditor={openSessionEditor}
      onOpenSaveModal={openSaveModal}
      {showToast}
    />
    <SessionEditor
      content={sessionComment}
      isOpen={sessionEditorOpen}
      {tags}
      {allowsImagePaste}
      pendingTagInsertion={pendingTagInsertion?.editorKey === 'session' ? { from: pendingTagInsertion.from, to: pendingTagInsertion.to, tag: pendingTagInsertion.tag } : null}
      onUpdate={updateSessionComment}
      onOpen={openSessionEditor}
      onClose={closeSessionEditor}
      onRequestCreateTag={(text, from, to) => handleRequestCreateTag('session', text, from, to)}
      onImagePasteBlocked={handleImagePasteBlocked}
    />
  </div>

  {#if error}
    <div class="error">{error}</div>
  {:else if lines.length === 0}
    <div class="loading">Loading...</div>
  {:else}
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
        style:transform="scale({contentZoom})"
        style:width="calc(100% / {contentZoom})"
      >
      {#each lineSegments as segment}
        {#if segment.type === 'portal'}
          <Portal
            lines={segment.lines}
            selection={interactionSelection}
            isDragging={interactionIsDragging}
            hoveredDisplayIdx={interactionHoveredIdx}
            {markdownMetadata}
            annotations={annotationsMap}
            {lastSelectedLine}
            onGutterMouseDown={handleGutterPointerDown}
            onGutterClick={interaction.handleGutterClick}
            onAddMouseDown={handleAddPointerDown}
            onMouseEnter={interaction.handleLineEnter}
            onMouseLeave={interaction.handleLineLeave}
          >
            {#snippet annotationSlot(displayIndex, rangeKey)}
              {#if rangeKey}
                {#key rangeKey}
                  <AnnotationEditor
                    content={annotationState.getByKey(rangeKey)?.content}
                    sealed={annotationState.isSealed(rangeKey)}
                    onUpdate={updateAnnotation}
                    onUnseal={() => {
                      interaction.setSelection(keyToRange(rangeKey));
                      annotationState.unseal(rangeKey);
                    }}
                    onDismiss={sealCurrentAnnotation}
                    {tags}
                    {allowsImagePaste}
                    onImagePasteBlocked={handleImagePasteBlocked}
                    onRequestCreateTag={(text, from, to) => handleRequestCreateTag(rangeKey, text, from, to)}
                    pendingTagInsertion={pendingTagInsertion?.editorKey === rangeKey ? { from: pendingTagInsertion.from, to: pendingTagInsertion.to, tag: pendingTagInsertion.tag } : null}
                    getOriginalLines={() => getOriginalLinesForRange(keyToRange(rangeKey))}
                  />
                {/key}
              {/if}
            {/snippet}
          </Portal>
        {:else if segment.type === 'codeblock'}
          {@const firstLineNum = getLineNumber(segment.lines[0]?.line)}
          {@const mermaidBlock = firstLineNum !== null ? getMermaidBlockAt(firstLineNum) : null}
          <CodeBlock
            lines={segment.lines}
            language={segment.language}
            selection={interactionSelection}
            isDragging={interactionIsDragging}
            hoveredDisplayIdx={interactionHoveredIdx}
            {markdownMetadata}
            annotations={annotationsMap}
            {lastSelectedLine}
            onGutterMouseDown={handleGutterPointerDown}
            onGutterClick={interaction.handleGutterClick}
            onAddMouseDown={handleAddPointerDown}
            onMouseEnter={interaction.handleLineEnter}
            onMouseLeave={interaction.handleLineLeave}
            onMermaidOpen={mermaidBlock ? () => openMermaidWindow(mermaidBlock) : undefined}
          >
            {#snippet annotationSlot(displayIndex, rangeKey)}
              {#if rangeKey}
                {#key rangeKey}
                  <AnnotationEditor
                    content={annotationState.getByKey(rangeKey)?.content}
                    sealed={annotationState.isSealed(rangeKey)}
                    onUpdate={updateAnnotation}
                    onUnseal={() => {
                      interaction.setSelection(keyToRange(rangeKey));
                      annotationState.unseal(rangeKey);
                    }}
                    onDismiss={sealCurrentAnnotation}
                    {tags}
                    {allowsImagePaste}
                    onImagePasteBlocked={handleImagePasteBlocked}
                    onRequestCreateTag={(text, from, to) => handleRequestCreateTag(rangeKey, text, from, to)}
                    pendingTagInsertion={pendingTagInsertion?.editorKey === rangeKey ? { from: pendingTagInsertion.from, to: pendingTagInsertion.to, tag: pendingTagInsertion.tag } : null}
                    getOriginalLines={() => getOriginalLinesForRange(keyToRange(rangeKey))}
                  />
                {/key}
              {/if}
            {/snippet}
          </CodeBlock>
        {:else if segment.type === 'table'}
          <Table
            lines={segment.lines}
            selection={interactionSelection}
            isDragging={interactionIsDragging}
            hoveredDisplayIdx={interactionHoveredIdx}
            {markdownMetadata}
            annotations={annotationsMap}
            {lastSelectedLine}
            onGutterMouseDown={handleGutterPointerDown}
            onGutterClick={interaction.handleGutterClick}
            onAddMouseDown={handleAddPointerDown}
            onMouseEnter={interaction.handleLineEnter}
            onMouseLeave={interaction.handleLineLeave}
          >
            {#snippet annotationSlot(displayIndex, rangeKey)}
              {#if rangeKey}
                {#key rangeKey}
                  <AnnotationEditor
                    content={annotationState.getByKey(rangeKey)?.content}
                    sealed={annotationState.isSealed(rangeKey)}
                    onUpdate={updateAnnotation}
                    onUnseal={() => {
                      interaction.setSelection(keyToRange(rangeKey));
                      annotationState.unseal(rangeKey);
                    }}
                    onDismiss={sealCurrentAnnotation}
                    {tags}
                    {allowsImagePaste}
                    onImagePasteBlocked={handleImagePasteBlocked}
                    onRequestCreateTag={(text, from, to) => handleRequestCreateTag(rangeKey, text, from, to)}
                    pendingTagInsertion={pendingTagInsertion?.editorKey === rangeKey ? { from: pendingTagInsertion.from, to: pendingTagInsertion.to, tag: pendingTagInsertion.tag } : null}
                    getOriginalLines={() => getOriginalLinesForRange(keyToRange(rangeKey))}
                  />
                {/key}
              {/if}
            {/snippet}
          </Table>
        {:else if segment.type === 'separator'}
          <div class="line separator-line">
            <span class="gutter"></span>
            <span class="code"><hr class="separator" /></span>
          </div>
        {:else}
          {#each segment.lines as { line, displayIndex }}
            {@const sourceLineNum = getLineNumber(line)}
            {@const diffKind = getDiffKind(line)}
            {@const mermaidBlock = sourceLineNum !== null ? getMermaidBlockAt(sourceLineNum) : null}
            <div
              class="line"
              class:selected={isSelected(displayIndex)}
              class:annotated={hasAnnotation(displayIndex)}
              class:preview={isPreview(displayIndex)}
              class:diff-added={diffKind === 'added'}
              class:diff-deleted={diffKind === 'deleted'}
              class:diff-context={diffKind === 'context'}
              class:diff-header={diffKind === 'file_header' || diffKind === 'hunk_header'}
              data-display-idx={displayIndex}
              onmouseenter={() => interaction.handleLineEnter(displayIndex)}
              onmouseleave={() => interaction.handleLineLeave()}
              role="presentation"
            >
              <button
                class="add-btn"
                onpointerdown={(e) => interaction.handlePointerDown(displayIndex, e)}
                aria-label="Add annotation"
              >+</button>
              <!-- svelte-ignore a11y_click_events_have_key_events -->
              <span
                class="gutter"
                class:selected={isSelected(displayIndex)}
                onpointerdown={(e) => interaction.handlePointerDown(displayIndex, e)}
                onclick={() => interaction.handleGutterClick(displayIndex)}
                role="button"
                tabindex="-1"
              >
                {#if line.origin.type === 'diff'}
                  <span class="diff-gutter-old">{line.origin.old_line ?? ''}</span>
                  <span class="diff-gutter-new">{line.origin.new_line ?? ''}</span>
                {:else if sourceLineNum !== null}
                  {sourceLineNum}
                {/if}
              </span>
              <span class="code" class:md={markdownMetadata}>{#if line.html}{@html line.html}{:else}{line.content}{/if}</span>
              {#if mermaidBlock}
                <button
                  class="mermaid-view-btn"
                  onclick={() => openMermaidWindow(mermaidBlock)}
                  title="View diagram"
                >
                  <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" width="14" height="14">
                    <path stroke-linecap="round" stroke-linejoin="round" d="M2.25 7.125C2.25 6.504 2.754 6 3.375 6h6c.621 0 1.125.504 1.125 1.125v3.75c0 .621-.504 1.125-1.125 1.125h-6a1.125 1.125 0 0 1-1.125-1.125v-3.75ZM14.25 8.625c0-.621.504-1.125 1.125-1.125h5.25c.621 0 1.125.504 1.125 1.125v8.25c0 .621-.504 1.125-1.125 1.125h-5.25a1.125 1.125 0 0 1-1.125-1.125v-8.25ZM3.75 16.125c0-.621.504-1.125 1.125-1.125h5.25c.621 0 1.125.504 1.125 1.125v2.25c0 .621-.504 1.125-1.125 1.125h-5.25a1.125 1.125 0 0 1-1.125-1.125v-2.25Z" />
                  </svg>
                </button>
              {/if}
            </div>
            {@const annotationAtLine = getAnnotationAtLine(displayIndex)}
            {@const isLastSelectedLine = displayIndex === lastSelectedLine && interaction.range && interaction.phase !== 'selecting'}
            {@const rangeKey = annotationAtLine?.key ?? (isLastSelectedLine && interaction.range ? rangeToKey(interaction.range) : null)}
            {#if rangeKey}
              {#key rangeKey}
                <AnnotationEditor
                  content={annotationState.getByKey(rangeKey)?.content}
                  sealed={annotationState.isSealed(rangeKey)}
                  onUpdate={updateAnnotation}
                  onUnseal={() => {
                    interaction.setSelection(keyToRange(rangeKey));
                    annotationState.unseal(rangeKey);
                  }}
                  onDismiss={sealCurrentAnnotation}
                  {tags}
                  {allowsImagePaste}
                  onImagePasteBlocked={handleImagePasteBlocked}
                  onRequestCreateTag={(text, from, to) => handleRequestCreateTag(rangeKey, text, from, to)}
                  pendingTagInsertion={pendingTagInsertion?.editorKey === rangeKey ? { from: pendingTagInsertion.from, to: pendingTagInsertion.to, tag: pendingTagInsertion.tag } : null}
                  getOriginalLines={() => getOriginalLinesForRange(keyToRange(rangeKey))}
                />
              {/key}
            {/if}
          {/each}
        {/if}
      {/each}
      </div>
    </div>
  {/if}

  <!-- Footer / Status Bar -->
  <StatusBar selectedMode={exitModeState.selectedMode} onCycleMode={exitModeState.cycleForward} />
</main>

{#if commandPaletteOpen}
  <CommandPalette
    {tags}
    exitModes={exitModeState.modes}
    onClose={handleCommandPaletteClose}
    onSetExitMode={handleSetExitModeFromPalette}
    onTagsChange={handleTagsChange}
    onExitModesChange={handleExitModesChange}
    {showToast}
    onOpenSaveModal={openSaveModal}
    initialState={pendingTagCreation ? { namespace: 'tags', mode: 'create', prefill: { instruction: pendingTagCreation.text } } : undefined}
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

  .mermaid-view-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    margin-left: 8px;
    padding: 2px 4px;
    background: var(--bg-window);
    border: 1px solid var(--border-subtle);
    border-radius: 4px;
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .mermaid-view-btn:hover {
    background: var(--bg-panel);
    color: var(--text-primary);
    border-color: var(--border-strong);
  }

  .mermaid-view-btn:focus-visible {
    outline: none;
    border-color: var(--focus-ring);
  }

  .mermaid-view-btn svg {
    display: block;
  }
</style>
