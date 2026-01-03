/**
 * Selection bounds logic for constraining line selections.
 *
 * Handles bounds detection for:
 * - Diff hunks: selections stay within a single hunk
 * - Portals: selections stay within a single embedded code block
 * - Code blocks: selections stay within fence boundaries
 */

import type { Line, DiffMetadata } from '$lib/types';
import { ContentTracker, type HunkPayload } from '$lib/content-tracker';
import { isPortalLine, isCodeBlockLine, getFilePath, getLineNumber } from '$lib/line-utils';

export interface Bounds {
  start: number;
  end: number;
}

export interface UseSelectionBoundsDeps {
  getLines: () => Line[];
  getDiffMetadata: () => DiffMetadata | null;
  getHunkTracker: () => ContentTracker<HunkPayload> | null;
}

export function useSelectionBounds(deps: UseSelectionBoundsDeps) {
  const { getLines, getDiffMetadata, getHunkTracker } = deps;

  /**
   * Get hunk bounds for a line (returns null if line is a header or not in diff mode).
   * The bounds define the selectable region within a diff hunk.
   */
  function getHunkBounds(lineNum: number): Bounds | null {
    const tracker = getHunkTracker();
    const diffMetadata = getDiffMetadata();
    const lines = getLines();

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

  /**
   * Constrain a line number to valid hunk bounds.
   */
  function constrainToHunkBounds(lineNum: number, anchorLine: number): number {
    const bounds = getHunkBounds(anchorLine);
    if (!bounds) return lineNum; // No bounds in non-diff mode

    // Clamp to hunk bounds
    return Math.max(bounds.start, Math.min(bounds.end, lineNum));
  }

  /**
   * Get portal bounds for a line (returns null if line is not in a portal).
   * Boundary detection: uses semantics and line number discontinuity.
   */
  function getPortalBounds(lineNum: number): Bounds | null {
    const lines = getLines();
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

  /**
   * Get code block bounds for a line (returns null if line is not in a code block).
   */
  function getCodeBlockBounds(lineNum: number): Bounds | null {
    const lines = getLines();
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

  /**
   * Constrain a line number to valid selection bounds.
   * Combines hunk bounds (for diff mode) and portal bounds.
   */
  function constrainToSelectionBounds(lineNum: number, anchorLine: number): number {
    const lines = getLines();

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

  return {
    getHunkBounds,
    getPortalBounds,
    getCodeBlockBounds,
    constrainToHunkBounds,
    constrainToSelectionBounds,
  };
}
