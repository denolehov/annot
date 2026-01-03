/**
 * Generic content position tracker using binary search.
 *
 * Used for:
 * - Diff mode: tracking current hunk as user scrolls
 * - Markdown mode: tracking current section for breadcrumb
 * - Future: tracking current function/class in code
 */

export interface Boundary<T> {
  /** Line number where this boundary starts */
  line: number;
  /** Mode-specific payload data */
  data: T;
}

export class ContentTracker<T> {
  private boundaries: Boundary<T>[] = [];

  constructor(items: Boundary<T>[]) {
    // Sort by line number for binary search
    this.boundaries = [...items].sort((a, b) => a.line - b.line);
  }

  /**
   * Find the boundary that contains the given line.
   * Uses binary search to find the largest boundary.line <= targetLine.
   */
  findAt(targetLine: number): Boundary<T> | null {
    if (this.boundaries.length === 0) return null;

    let lo = 0;
    let hi = this.boundaries.length - 1;
    let result: Boundary<T> | null = null;

    while (lo <= hi) {
      const mid = (lo + hi) >> 1;
      if (this.boundaries[mid].line <= targetLine) {
        result = this.boundaries[mid];
        lo = mid + 1;
      } else {
        hi = mid - 1;
      }
    }

    return result;
  }

  /**
   * Get all boundaries (for iteration).
   */
  all(): readonly Boundary<T>[] {
    return this.boundaries;
  }

  /**
   * Get the number of boundaries.
   */
  get length(): number {
    return this.boundaries.length;
  }
}

// Type aliases for specific use cases

/** Payload for diff hunk tracking */
export interface HunkPayload {
  fileIndex: number;
  hunkIndex: number;
}

/** Payload for markdown section tracking */
export interface SectionPayload {
  sectionIndex: number;
}

/** Payload for code symbol tracking (future) */
export interface SymbolPayload {
  symbolIndex: number;
  kind: 'function' | 'class' | 'method';
}
