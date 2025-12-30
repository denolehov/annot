/**
 * Line segmentation composable for portal/codeblock/table-aware rendering.
 *
 * Groups consecutive lines by type (portal, codeblock, table, regular, separator)
 * for efficient rendering with specialized components.
 */

import type { Line } from '$lib/types';
import { isPortalLine, isCodeBlockLine, isTableLine, isHorizontalRule } from '$lib/line-utils';

/** A line with its 1-indexed display position */
export interface DisplayLine {
  line: Line;
  displayIndex: number;
}

/** Segment type for rendering */
export type LineSegment =
  | { type: 'regular'; lines: DisplayLine[] }
  | { type: 'portal'; lines: DisplayLine[] }
  | { type: 'codeblock'; lines: DisplayLine[]; language: string | null }
  | { type: 'table'; lines: DisplayLine[] }
  | { type: 'separator'; lines: DisplayLine[] };

/** Get the language from a code block start line */
function getCodeBlockLanguage(line: Line): string | null {
  if (line.semantics.type === 'markdown' && line.semantics.kind === 'code_block_start') {
    return line.semantics.language;
  }
  return null;
}

export function useLineSegments(getLines: () => Line[]) {
  /** Group lines into segments for portal/codeblock/table-aware rendering */
  let segments = $derived.by((): LineSegment[] => {
    const lines = getLines();
    const result: LineSegment[] = [];
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
        if (currentSegment) result.push(currentSegment);
        result.push({ type: 'separator', lines: [{ line, displayIndex }] });
        currentSegment = null;
      } else if (isPortal) {
        // Portal line - portals take priority over code blocks and tables
        const isPortalHeader = line.semantics.type === 'portal' && line.semantics.kind === 'header';
        if (currentSegment?.type === 'portal' && !isPortalHeader) {
          // Continue current portal segment (unless this is a new portal header)
          currentSegment.lines.push({ line, displayIndex });
        } else {
          // Start new portal segment
          if (currentSegment) result.push(currentSegment);
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
          if (currentSegment) result.push(currentSegment);
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
          if (currentSegment) result.push(currentSegment);
          currentSegment = { type: 'table', lines: [{ line, displayIndex }] };
        }
      } else {
        // Regular line
        if (currentSegment?.type === 'regular') {
          // Continue current regular segment
          currentSegment.lines.push({ line, displayIndex });
        } else {
          // Start new regular segment
          if (currentSegment) result.push(currentSegment);
          currentSegment = { type: 'regular', lines: [{ line, displayIndex }] };
        }
      }
    }

    if (currentSegment) result.push(currentSegment);
    return result;
  });

  return {
    get segments() { return segments; },
  };
}
