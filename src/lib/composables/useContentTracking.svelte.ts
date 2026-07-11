import { ContentTracker, type SectionPayload } from '$lib/content-tracker';
import type { MarkdownMetadata } from '$lib/types';
import type { DiffDisplay } from '$lib/display-rows';

export function useContentTracking(getDisplay: () => DiffDisplay | null = () => null) {
  let sectionTracker: ContentTracker<SectionPayload> | null = $state(null);
  let currentFileIndex = $state(0);
  let currentHunkIndex = $state(0);
  let currentSectionIndex = $state(0);

  function initializeMarkdown(meta: MarkdownMetadata): void {
    const boundaries = meta.sections.map((section, i) => ({
      line: section.source_line,
      data: { sectionIndex: i },
    }));
    sectionTracker = new ContentTracker(boundaries);
  }

  function updateFromLine(lineNum: number): void {
    // Diff mode: lineNum is a display index — resolve through the walk.
    // A file header is its own file's position, above its first hunk.
    const entry = getDisplay()?.byIndex.get(lineNum);
    if (entry) {
      currentFileIndex = entry.docIdx;
      currentHunkIndex = entry.kind === 'file-header' ? 0 : entry.hunkIdx;
    }

    // Markdown mode: lineNum is a source line.
    if (sectionTracker) {
      const boundary = sectionTracker.findAt(lineNum);
      if (boundary) {
        currentSectionIndex = boundary.data.sectionIndex;
      }
    }
  }

  return {
    get currentFileIndex() { return currentFileIndex; },
    get currentHunkIndex() { return currentHunkIndex; },
    get currentSectionIndex() { return currentSectionIndex; },
    initializeMarkdown,
    updateFromLine,
  };
}
