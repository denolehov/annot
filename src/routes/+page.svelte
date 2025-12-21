<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { onMount } from "svelte";
  import type { ContentResponse, ContentNode, Line, JSONContent, ExitMode, Tag, DiffMetadata, DiffLineInfo, HunkInfo, MarkdownMetadata, SectionInfo } from "$lib/types";
  import { rangeToKey, keyToRange, isLineInRange, type Range } from "$lib/range";
  import { extractContentNodes, isContentEmpty, contentNodesToTipTap } from "$lib/tiptap";
  import { ContentTracker, type HunkPayload, type SectionPayload } from "$lib/content-tracker";
  import AnnotationEditor from "$lib/AnnotationEditor.svelte";
  import CopyDropdown from "$lib/CopyDropdown.svelte";
  import { CommandPalette } from "$lib/CommandPalette";

  let lines: Line[] = $state([]);
  let label = $state("");
  let error = $state("");
  let diffMetadata: DiffMetadata | null = $state(null);
  let markdownMetadata: MarkdownMetadata | null = $state(null);
  let ephemeral = $state(false);

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

  // Content tracking (generalized for diff/markdown/future modes)
  let hunkTracker: ContentTracker<HunkPayload> | null = $state(null);
  let sectionTracker: ContentTracker<SectionPayload> | null = $state(null);
  let currentFileIndex = $state(0);
  let currentHunkIndex = $state(0);
  let currentSectionIndex = $state(0);
  let contentEl: HTMLDivElement | null = $state(null);
  let scrollRafId: number | null = null;

  // Current file/hunk derived from indices (diff mode)
  let currentFile = $derived.by(() => {
    if (!diffMetadata || diffMetadata.files.length === 0) return null;
    return diffMetadata.files[currentFileIndex] ?? null;
  });

  let currentHunk = $derived.by(() => {
    if (!currentFile || currentFile.hunks.length === 0) return null;
    return currentFile.hunks[currentHunkIndex] ?? null;
  });

  // Current section derived from index (markdown mode)
  let currentSection = $derived.by(() => {
    if (!markdownMetadata || markdownMetadata.sections.length === 0) return null;
    return markdownMetadata.sections[currentSectionIndex] ?? null;
  });

  // Build breadcrumb for markdown sections
  let sectionBreadcrumb = $derived.by(() => {
    if (!markdownMetadata || currentSectionIndex < 0) return [];
    const sections = markdownMetadata.sections;
    const breadcrumb: SectionInfo[] = [];

    let idx: number | null = currentSectionIndex;
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

  // Build ContentTracker for diff mode
  function buildHunkTracker(meta: DiffMetadata): ContentTracker<HunkPayload> {
    const boundaries: { line: number; data: HunkPayload }[] = [];
    for (let fi = 0; fi < meta.files.length; fi++) {
      const file = meta.files[fi];
      for (let hi = 0; hi < file.hunks.length; hi++) {
        boundaries.push({
          line: file.hunks[hi].display_line,
          data: { fileIndex: fi, hunkIndex: hi },
        });
      }
    }
    return new ContentTracker(boundaries);
  }

  // Build ContentTracker for markdown mode
  function buildSectionTracker(meta: MarkdownMetadata): ContentTracker<SectionPayload> {
    const boundaries = meta.sections.map((section, i) => ({
      line: section.source_line,
      data: { sectionIndex: i },
    }));
    return new ContentTracker(boundaries);
  }

  function updateCurrentPosition() {
    if (!contentEl) return;

    const lineEls = contentEl.querySelectorAll('.line');
    const scrollTop = contentEl.scrollTop;

    for (const el of lineEls) {
      const htmlEl = el as HTMLElement;
      if (htmlEl.offsetTop >= scrollTop) {
        const lineNum = parseInt(htmlEl.dataset.line ?? '1', 10);

        // Update diff tracking
        if (hunkTracker) {
          const boundary = hunkTracker.findAt(lineNum);
          if (boundary) {
            currentFileIndex = boundary.data.fileIndex;
            currentHunkIndex = boundary.data.hunkIndex;
          }
        }

        // Update markdown tracking
        if (sectionTracker) {
          const boundary = sectionTracker.findAt(lineNum);
          if (boundary) {
            currentSectionIndex = boundary.data.sectionIndex;
          }
        }

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

  // Helper to get diff line info
  function getDiffLineInfo(lineNum: number): DiffLineInfo | null {
    if (!diffMetadata) return null;
    return diffMetadata.lines[lineNum] ?? null;
  }

  // Check if a line is selectable (not a header in diff mode)
  function isLineSelectable(lineNum: number): boolean {
    if (!diffMetadata) return true; // Non-diff mode: all lines selectable
    const info = getDiffLineInfo(lineNum);
    return info ? info.kind !== 'header' : true;
  }

  // Get hunk bounds for a line (returns null if line is a header or not in diff mode)
  function getHunkBounds(lineNum: number): { start: number; end: number } | null {
    if (!diffMetadata || !hunkTracker || hunkTracker.length === 0) return null;

    const boundary = hunkTracker.findAt(lineNum);
    if (!boundary) return null;

    const boundaries = hunkTracker.all();

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

  // CommandPalette state
  let commandPaletteOpen = $state(false);
  let tags: Tag[] = $state([]);

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

  // CommandPalette handlers
  function handleCommandPaletteClose() {
    commandPaletteOpen = false;
  }

  function handleSetExitModeFromPalette(modeId: string) {
    const idx = exitModes.findIndex(m => m.id === modeId);
    if (idx >= 0) {
      selectedModeIndex = idx;
      invoke('set_exit_mode', { modeId });
    }
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
    const currentIds = new Set(exitModes.map(m => m.id));
    const newIds = new Set(newModes.map(m => m.id));

    // Check for deleted modes
    for (const mode of exitModes) {
      if (!newIds.has(mode.id)) {
        await invoke('delete_exit_mode', { id: mode.id });
      }
    }

    // Check for added/updated modes
    for (const mode of newModes) {
      const existing = exitModes.find(m => m.id === mode.id);
      if (!existing || existing.name !== mode.name || existing.instruction !== mode.instruction ||
          existing.color !== mode.color || existing.order !== mode.order) {
        await invoke('upsert_exit_mode', { mode });
      }
    }

    exitModes = newModes;

    // Update selectedModeIndex if mode was deleted
    if (selectedModeIndex !== null && selectedModeIndex >= exitModes.length) {
      selectedModeIndex = exitModes.length > 0 ? exitModes.length - 1 : null;
    }
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
    // Skip header lines in diff mode
    if (!isLineSelectable(lineNum)) return;

    e.preventDefault();
    isDragging = true;
    mouseDownHandled = true;
    selection = { start: lineNum, end: lineNum };
  }

  function handleContentMouseDown(e: MouseEvent) {
    if (!e.shiftKey) return;
    const lineNum = getLineFromEvent(e);
    if (lineNum === null) return;
    // Skip header lines in diff mode
    if (!isLineSelectable(lineNum)) return;

    e.preventDefault();
    isDragging = true;
    selection = { start: lineNum, end: lineNum };
  }

  function handleMouseMove(e: MouseEvent) {
    if (!isDragging || !selection) return;
    const lineNum = getLineFromEvent(e);
    if (lineNum !== null) {
      // Constrain to hunk bounds and skip header lines
      const constrainedLine = constrainToHunkBounds(lineNum, selection.start);
      selection = { start: selection.start, end: constrainedLine };
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
    // Skip header lines in diff mode
    if (!isLineSelectable(lineNum)) return;

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
      // Only cycle exit modes when no editor is active
      if (exitModes.length > 0 && !selection && !sessionEditorOpen && !commandPaletteOpen) {
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
      // Open editor on hovered line (skip header lines)
      if (!isLineSelectable(hoveredLine)) return;
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
    } else if (e.key === ':' && !selection && !sessionEditorOpen && !commandPaletteOpen) {
      // Open CommandPalette (when not in any editor)
      const activeEl = document.activeElement;
      const isInEditor = activeEl?.closest('.annotation-editor, .session-editor');
      const isInInput = activeEl instanceof HTMLInputElement || activeEl instanceof HTMLTextAreaElement;
      if (!isInEditor && !isInInput && !e.metaKey && !e.ctrlKey && !e.altKey) {
        e.preventDefault();
        commandPaletteOpen = true;
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
    // Skip header lines in diff mode
    if (!isLineSelectable(lineNum)) return;

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
      tags = res.tags;
      exitModes = res.exit_modes;
      diffMetadata = res.diff_metadata;
      markdownMetadata = res.markdown_metadata;
      ephemeral = res.ephemeral;

      // Build content trackers for scroll tracking
      if (res.diff_metadata) {
        hunkTracker = buildHunkTracker(res.diff_metadata);
      }
      if (res.markdown_metadata) {
        sectionTracker = buildSectionTracker(res.markdown_metadata);
      }

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
      const unlisten = await window.onCloseRequested(async (event) => {
        event.preventDefault();
        unlisten();  // Remove listener before closing to prevent re-entry
        await invoke('finish_session');
        await window.destroy();
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
        {#if diffMetadata && currentFile}
          <!-- Diff mode: show hunk metadata -->
          {@const fileName = currentFile.new_name ?? currentFile.old_name ?? 'unknown'}
          {@const fileCount = diffMetadata.files.length}
          <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
          <span class="diff-header-info">
            <span
              class="diff-header-file"
              class:has-comment={sessionComment !== undefined}
              onclick={openSessionEditor}
            >
              {fileName}
              {#if fileCount > 1}
                <span class="diff-header-counter">({currentFileIndex + 1}/{fileCount})</span>
              {/if}
            </span>
            {#if currentHunk}
              <span class="diff-header-sep">·</span>
              <span class="diff-header-range">
                <span class="diff-header-old">-{currentHunk.old_start},{currentHunk.old_count}</span>
                <span class="diff-header-new">+{currentHunk.new_start},{currentHunk.new_count}</span>
              </span>
              {#if currentHunk.function_context}
                <span class="diff-header-fn">
                  {#if currentHunk.function_context_html}
                    {@html currentHunk.function_context_html}
                  {:else}
                    {currentHunk.function_context}
                  {/if}
                </span>
              {/if}
            {/if}
          </span>
        {:else if markdownMetadata && sectionBreadcrumb.length > 0}
          <!-- Markdown mode: depth-based breadcrumb -->
          <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
          <span class="md-header-info">
            <!-- Filename -->
            <span
              class="md-header-file"
              class:has-comment={sessionComment !== undefined}
              onclick={openSessionEditor}
            ><span class="md-header-title">{label}</span></span>

            <!-- H1 (root) - always shown -->
            {#if headerRootSection}
              <span class="md-header-sep">·</span>
              <span class="md-header-section md-header-root">
                <span class="md-header-level">#</span>
                <span class="md-header-title">{headerRootSection.title}</span>
              </span>
            {/if}

            <!-- H2 shown only when current depth is exactly 2 -->
            {#if headerCurrentDepth === 2 && headerH2Section}
              <span class="md-header-sep">·</span>
              <span class="md-header-section md-header-current">
                <span class="md-header-level">##</span>
                <span class="md-header-title">{headerH2Section.title}</span>
              </span>
            {/if}

            <!-- Ellipsis + current section when depth >= 3 -->
            {#if headerCurrentDepth >= 3 && headerCurrentSection}
              <span class="md-header-sep">·</span>
              <span class="md-header-ellipsis">…</span>
              <span class="md-header-sep">·</span>
              <span class="md-header-section md-header-current">
                <span class="md-header-level">{'#'.repeat(headerCurrentSection.level)}</span>
                <span class="md-header-title">{headerCurrentSection.title}</span>
              </span>
            {/if}
          </span>
        {:else}
          <!-- Normal mode: show filename -->
          <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
          <span
            class="file-name"
            class:has-comment={sessionComment !== undefined}
            onclick={openSessionEditor}
          >{label}</span>
        {/if}
      </div>
      <div class="header-right">
        <CopyDropdown {showToast} />
      </div>
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
          {tags}
          {ephemeral}
          onImagePasteBlocked={handleImagePasteBlocked}
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
      class:diff-mode={diffMetadata !== null}
      bind:this={contentEl}
      onscroll={handleContentScroll}
      onmousedown={handleContentMouseDown}
      onmousemove={handleMouseMove}
      onmouseup={handleMouseUp}
      role="presentation"
    >
      {#each lines as line}
        {@const diffLine = getDiffLineInfo(line.number)}
        <div
          class="line"
          class:selected={isSelected(line.number)}
          class:annotated={hasAnnotation(line.number)}
          class:diff-added={diffLine?.kind === 'added'}
          class:diff-deleted={diffLine?.kind === 'deleted'}
          class:diff-context={diffLine?.kind === 'context'}
          class:diff-header={diffLine?.kind === 'header'}
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
            {#if diffMetadata}
              <span class="diff-gutter-old">{diffLine?.old_line_num ?? ''}</span>
              <span class="diff-gutter-new">{diffLine?.new_line_num ?? ''}</span>
            {:else}
              {line.number}
            {/if}
          </span>
          <span class="code">{#if line.html}{@html line.html}{:else}{line.content}{/if}</span>
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
              {tags}
              {ephemeral}
              onImagePasteBlocked={handleImagePasteBlocked}
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
      <span class="kbd-hint"><kbd>⌘w</kbd> save and close</span>
    </div>
  </footer>
</main>

{#if commandPaletteOpen}
  <CommandPalette
    {tags}
    {exitModes}
    onClose={handleCommandPaletteClose}
    onSetExitMode={handleSetExitModeFromPalette}
    onTagsChange={handleTagsChange}
    onExitModesChange={handleExitModesChange}
    {showToast}
  />
{/if}

{#if toastMessage}
  <div class="toast" class:exiting={toastExiting}>{toastMessage}</div>
{/if}

<style>
  /* Page-specific styles only - see src/styles/ for the design system */

  :global(body) {
    overflow: hidden;
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
