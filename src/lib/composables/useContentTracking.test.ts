import { describe, it, expect } from 'vitest';
import { flushSync } from 'svelte';
import { useContentTracking } from './useContentTracking.svelte';
import { deriveDisplay } from '$lib/display-rows';
import type { DiffDocument, MarkdownMetadata } from '$lib/types';

function doc(path: string, rowCounts: number[]): DiffDocument {
  return {
    path,
    old_path: null,
    status: 'modified',
    unavailable: false,
    conflicted: false,
    new_len: null,
    language: 'rs',
    hunks: rowCounts.map((rowCount) => ({
      old_range: { start: 1, end: 1 + rowCount },
      new_range: { start: 1, end: 1 + rowCount },
      function_context: null,
      function_context_html: null,
      rows: Array.from({ length: rowCount }, (_, i) => ({
        old_line: i + 1,
        new_line: i + 1,
        content: ` line ${i}`,
        html: null,
      })),
    })),
  };
}

describe('useContentTracking', () => {
  it('starts with default indices', () => {
    const tracking = useContentTracking();
    expect(tracking.currentFileIndex).toBe(0);
    expect(tracking.currentHunkIndex).toBe(0);
    expect(tracking.currentSectionIndex).toBe(0);
  });

  it('updates position from display index in diff mode', () => {
    // a.rs: header 1, hunks at 2 (rows 3–9) and 10 (rows 11–15)
    const display = deriveDisplay([doc('a.rs', [7, 5])]);
    const tracking = useContentTracking(() => display);

    flushSync(() => {
      tracking.updateFromLine(5);
    });
    expect(tracking.currentFileIndex).toBe(0);
    expect(tracking.currentHunkIndex).toBe(0);

    flushSync(() => {
      tracking.updateFromLine(15);
    });
    expect(tracking.currentFileIndex).toBe(0);
    expect(tracking.currentHunkIndex).toBe(1);
  });

  it('resolves a file header line to its own file, not the file above it', () => {
    // lib.rs: header 1, hunk 2 (rows 3–17); main.rs: header 18, hunk 19 (rows 20–29)
    const display = deriveDisplay([doc('lib.rs', [15]), doc('main.rs', [10])]);
    const tracking = useContentTracking(() => display);

    flushSync(() => {
      tracking.updateFromLine(18);
    });
    expect(tracking.currentFileIndex).toBe(1);
    expect(tracking.currentHunkIndex).toBe(0);

    flushSync(() => {
      tracking.updateFromLine(25);
    });
    expect(tracking.currentFileIndex).toBe(1);
    expect(tracking.currentHunkIndex).toBe(0);
  });

  it('initializes markdown tracker from metadata', () => {
    const tracking = useContentTracking();
    const meta: MarkdownMetadata = {
      sections: [
        { title: 'Intro', level: 1, source_line: 1, parent_index: null, end_line: 9 },
        { title: 'Details', level: 2, source_line: 10, parent_index: 0, end_line: 24 },
        { title: 'More', level: 2, source_line: 25, parent_index: 0, end_line: 50 },
      ],
      code_blocks: [],
      tables: [],
    };

    flushSync(() => {
      tracking.initializeMarkdown(meta);
    });

    flushSync(() => {
      tracking.updateFromLine(15);
    });
    expect(tracking.currentSectionIndex).toBe(1); // "Details" section
  });

  it('updates section index from line number in markdown mode', () => {
    const tracking = useContentTracking();
    const meta: MarkdownMetadata = {
      sections: [
        { title: 'A', level: 1, source_line: 1, parent_index: null, end_line: 19 },
        { title: 'B', level: 1, source_line: 20, parent_index: null, end_line: 39 },
        { title: 'C', level: 1, source_line: 40, parent_index: null, end_line: 100 },
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
