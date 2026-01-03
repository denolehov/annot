import { ContentTracker, type HunkPayload, type SectionPayload } from '$lib/content-tracker';
import type { DiffMetadata, MarkdownMetadata } from '$lib/types';

export function useContentTracking() {
  let hunkTracker: ContentTracker<HunkPayload> | null = $state(null);
  let sectionTracker: ContentTracker<SectionPayload> | null = $state(null);
  let currentFileIndex = $state(0);
  let currentHunkIndex = $state(0);
  let currentSectionIndex = $state(0);
  let scrollRafId: number | null = null;

  function initializeDiff(meta: DiffMetadata): void {
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
    hunkTracker = new ContentTracker(boundaries);
  }

  function initializeMarkdown(meta: MarkdownMetadata): void {
    const boundaries = meta.sections.map((section, i) => ({
      line: section.source_line,
      data: { sectionIndex: i },
    }));
    sectionTracker = new ContentTracker(boundaries);
  }

  function updateFromLine(lineNum: number): void {
    if (hunkTracker) {
      const boundary = hunkTracker.findAt(lineNum);
      if (boundary) {
        currentFileIndex = boundary.data.fileIndex;
        currentHunkIndex = boundary.data.hunkIndex;
      }
    }
    if (sectionTracker) {
      const boundary = sectionTracker.findAt(lineNum);
      if (boundary) {
        currentSectionIndex = boundary.data.sectionIndex;
      }
    }
  }

  function handleScroll(contentEl: HTMLElement): void {
    if (scrollRafId) return;
    scrollRafId = requestAnimationFrame(() => {
      scrollRafId = null;
      updateCurrentPosition(contentEl);
    });
  }

  function updateCurrentPosition(contentEl: HTMLElement): void {
    const lineEls = contentEl.querySelectorAll('.line');
    const scrollTop = contentEl.scrollTop;

    for (const el of lineEls) {
      const htmlEl = el as HTMLElement;
      if (htmlEl.offsetTop >= scrollTop) {
        const lineNum = parseInt(htmlEl.dataset.line ?? '1', 10);
        updateFromLine(lineNum);
        break;
      }
    }
  }

  return {
    get hunkTracker() { return hunkTracker; },
    get currentFileIndex() { return currentFileIndex; },
    get currentHunkIndex() { return currentHunkIndex; },
    get currentSectionIndex() { return currentSectionIndex; },
    initializeDiff,
    initializeMarkdown,
    updateFromLine,
    handleScroll,
  };
}
