import { describe, it, expect } from 'vitest';
import { flushSync } from 'svelte';
import { useContentTracking } from './useContentTracking.svelte';
import type { DiffMetadata, MarkdownMetadata } from '$lib/types';

describe('useContentTracking', () => {
  it('starts with default indices', () => {
    const tracking = useContentTracking();
    expect(tracking.currentFileIndex).toBe(0);
    expect(tracking.currentHunkIndex).toBe(0);
    expect(tracking.currentSectionIndex).toBe(0);
    expect(tracking.hunkTracker).toBeNull();
  });

  it('initializes diff tracker from metadata', () => {
    const tracking = useContentTracking();
    const meta: DiffMetadata = {
      files: [
        {
          old_name: 'a.rs',
          new_name: 'a.rs',
          language: 'rust',
          start_line: 1,
          end_line: 20,
          hunks: [
            { display_line: 2, old_start: 1, old_count: 5, new_start: 1, new_count: 6, function_context: null, function_context_html: null },
            { display_line: 10, old_start: 10, old_count: 3, new_start: 11, new_count: 4, function_context: null, function_context_html: null },
          ],
        },
        {
          old_name: 'b.rs',
          new_name: 'b.rs',
          language: 'rust',
          start_line: 21,
          end_line: 40,
          hunks: [
            { display_line: 22, old_start: 1, old_count: 5, new_start: 1, new_count: 5, function_context: null, function_context_html: null },
          ],
        },
      ],
    };

    flushSync(() => {
      tracking.initializeDiff(meta);
    });

    expect(tracking.hunkTracker).not.toBeNull();
    expect(tracking.hunkTracker?.length).toBe(3); // 2 + 1 hunks
  });

  it('updates position from line number in diff mode', () => {
    const tracking = useContentTracking();
    const meta: DiffMetadata = {
      files: [
        {
          old_name: 'a.rs',
          new_name: 'a.rs',
          language: 'rust',
          start_line: 1,
          end_line: 20,
          hunks: [
            { display_line: 2, old_start: 1, old_count: 5, new_start: 1, new_count: 6, function_context: null, function_context_html: null },
            { display_line: 10, old_start: 10, old_count: 3, new_start: 11, new_count: 4, function_context: null, function_context_html: null },
          ],
        },
      ],
    };

    flushSync(() => {
      tracking.initializeDiff(meta);
    });

    // Line 5 is in the first hunk (starts at line 2)
    flushSync(() => {
      tracking.updateFromLine(5);
    });
    expect(tracking.currentFileIndex).toBe(0);
    expect(tracking.currentHunkIndex).toBe(0);

    // Line 15 is in the second hunk (starts at line 10)
    flushSync(() => {
      tracking.updateFromLine(15);
    });
    expect(tracking.currentFileIndex).toBe(0);
    expect(tracking.currentHunkIndex).toBe(1);
  });

  it('initializes markdown tracker from metadata', () => {
    const tracking = useContentTracking();
    const meta: MarkdownMetadata = {
      sections: [
        { title: 'Intro', level: 1, source_line: 1, parent_index: null },
        { title: 'Details', level: 2, source_line: 10, parent_index: 0 },
        { title: 'More', level: 2, source_line: 25, parent_index: 0 },
      ],
      code_blocks: [],
      tables: [],
    };

    flushSync(() => {
      tracking.initializeMarkdown(meta);
    });

    // We don't expose sectionTracker directly, but we can test via updateFromLine
    flushSync(() => {
      tracking.updateFromLine(15);
    });
    expect(tracking.currentSectionIndex).toBe(1); // "Details" section
  });

  it('updates section index from line number in markdown mode', () => {
    const tracking = useContentTracking();
    const meta: MarkdownMetadata = {
      sections: [
        { title: 'A', level: 1, source_line: 1, parent_index: null },
        { title: 'B', level: 1, source_line: 20, parent_index: null },
        { title: 'C', level: 1, source_line: 40, parent_index: null },
      ],
      code_blocks: [],
      tables: [],
    };

    flushSync(() => {
      tracking.initializeMarkdown(meta);
    });

    flushSync(() => {
      tracking.updateFromLine(5);
    });
    expect(tracking.currentSectionIndex).toBe(0);

    flushSync(() => {
      tracking.updateFromLine(25);
    });
    expect(tracking.currentSectionIndex).toBe(1);

    flushSync(() => {
      tracking.updateFromLine(50);
    });
    expect(tracking.currentSectionIndex).toBe(2);
  });
});
