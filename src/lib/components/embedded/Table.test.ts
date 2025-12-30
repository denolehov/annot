import { describe, it, expect, vi, beforeAll } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import Table from './Table.svelte';
import type { Line, LineHtml, MarkdownMetadata } from '$lib/types';
import type { Range } from '$lib/range';

// Mock ResizeObserver (not available in jsdom)
beforeAll(() => {
  globalThis.ResizeObserver = class ResizeObserver {
    observe() {}
    unobserve() {}
    disconnect() {}
  };
});

// Helper to create a table row line with per-cell HTML
function makeTableLine(
  num: number,
  content: string,
  htmlCells: string[] | null = null
): Line {
  return {
    content,
    html: htmlCells ? { type: 'cells', value: htmlCells } : null,
    origin: { type: 'source', path: 'test.md', line: num },
    semantics: { type: 'markdown', kind: 'table_row' },
  };
}

// Minimal props for Table component
function createTableProps(lines: Array<{ line: Line; displayIndex: number }>) {
  return {
    lines,
    selection: null as Range | null,
    isDragging: false,
    hoveredDisplayIdx: null as number | null,
    markdownMetadata: null as MarkdownMetadata | null,
    annotations: new Map(),
    lastSelectedLine: null as number | null,
    onGutterMouseDown: vi.fn(),
    onGutterClick: vi.fn(),
    onAddMouseDown: vi.fn(),
    onMouseEnter: vi.fn(),
    onMouseLeave: vi.fn(),
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    annotationSlot: (() => null) as any,
  };
}

describe('Table component', () => {
  describe('inline formatting in cells', () => {
    it('renders highlights (==text==) in table cells', async () => {
      // Backend sends per-cell HTML via line.html = { type: 'cells', value: [...] }

      const lines = [
        { line: makeTableLine(1, '| Name | Status |', ['Name', 'Status']), displayIndex: 0 },
        { line: makeTableLine(2, '|---|---|', null), displayIndex: 1 }, // separator has no cells
        { line: makeTableLine(3, '| Item | ==Important== |', ['Item', '<mark class="hl">Important</mark>']), displayIndex: 2 },
      ];

      render(Table, { props: createTableProps(lines) });

      // The highlight should be rendered with the <mark> tag
      const markElement = document.querySelector('mark.hl');
      expect(markElement).not.toBeNull();
      expect(markElement?.textContent).toBe('Important');
    });

    it('renders bold (**text**) in table cells', async () => {
      const lines = [
        { line: makeTableLine(1, '| Name | Status |', ['Name', 'Status']), displayIndex: 0 },
        { line: makeTableLine(2, '|---|---|', null), displayIndex: 1 },
        { line: makeTableLine(3, '| Item | **Bold** |', ['Item', '<strong>Bold</strong>']), displayIndex: 2 },
      ];

      render(Table, { props: createTableProps(lines) });

      const strongElement = document.querySelector('strong');
      expect(strongElement).not.toBeNull();
      expect(strongElement?.textContent).toBe('Bold');
    });

    it('renders inline code (`code`) in table cells', async () => {
      const lines = [
        { line: makeTableLine(1, '| Name | Value |', ['Name', 'Value']), displayIndex: 0 },
        { line: makeTableLine(2, '|---|---|', null), displayIndex: 1 },
        { line: makeTableLine(3, '| Key | `value` |', ['Key', '<code>value</code>']), displayIndex: 2 },
      ];

      render(Table, { props: createTableProps(lines) });

      const codeElement = document.querySelector('code');
      expect(codeElement).not.toBeNull();
      expect(codeElement?.textContent).toBe('value');
    });

    it('renders multiple inline formats in same cell', async () => {
      const lines = [
        { line: makeTableLine(1, '| Description |', ['Description']), displayIndex: 0 },
        { line: makeTableLine(2, '|---|', null), displayIndex: 1 },
        { line: makeTableLine(3, '| **Bold** and ==highlighted== |', ['<strong>Bold</strong> and <mark class="hl">highlighted</mark>']), displayIndex: 2 },
      ];

      render(Table, { props: createTableProps(lines) });

      expect(document.querySelector('strong')?.textContent).toBe('Bold');
      expect(document.querySelector('mark.hl')?.textContent).toBe('highlighted');
    });
  });
});
