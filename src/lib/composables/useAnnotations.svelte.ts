import { invoke } from '@tauri-apps/api/core';
import type { JSONContent } from '@tiptap/core';
import type { Range } from '$lib/range';
import type { Line } from '$lib/types';
import { rangeToKey, isLineInRange, validateRange } from '$lib/range';
import { extractContentNodes, isContentEmpty } from '$lib/tiptap';

export interface AnnotationEntry {
  range: Range;
  content: JSONContent;
}

export interface UseAnnotationsOptions {
  /** Lines array for validating ranges and resolving paths */
  getLines: () => Line[];
}

export function useAnnotations(options: UseAnnotationsOptions) {
  let annotations: Record<string, AnnotationEntry> = $state({});

  function get(range: Range): JSONContent | undefined {
    return annotations[rangeToKey(range)]?.content;
  }

  function getByKey(key: string): AnnotationEntry | undefined {
    return annotations[key];
  }

  async function upsert(range: Range, content: JSONContent | null): Promise<void> {
    const key = rangeToKey(range);
    const lines = options.getLines();
    const coords = validateRange(range, lines);

    if (!coords) {
      console.warn('Invalid range for annotation:', range);
      return;
    }

    if (content && !isContentEmpty(content)) {
      // Store display indices (from range param), not source line numbers (from coords)
      // Display indices are used for UI lookups (getAtLine, hasAnnotation)
      // Source coords are only for the backend API call
      const normalizedRange = {
        start: Math.min(range.start, range.end),
        end: Math.max(range.start, range.end)
      };
      annotations[key] = {
        range: normalizedRange,
        content,
      };
      const nodes = extractContentNodes(content);
      await invoke('upsert_annotation', {
        path: coords.path,
        startLine: coords.startLine,
        endLine: coords.endLine,
        content: nodes
      });
    } else {
      delete annotations[key];
      await invoke('delete_annotation', {
        path: coords.path,
        startLine: coords.startLine,
        endLine: coords.endLine
      });
    }
  }

  function remove(key: string): void {
    delete annotations[key];
  }

  function getAtLine(displayIdx: number): { key: string; content: JSONContent } | null {
    for (const [key, entry] of Object.entries(annotations)) {
      if (entry.range.end === displayIdx) {
        return { key, content: entry.content };
      }
    }
    return null;
  }

  function hasAnnotation(displayIdx: number): boolean {
    for (const entry of Object.values(annotations)) {
      if (isLineInRange(displayIdx, entry.range)) {
        return true;
      }
    }
    return false;
  }

  function allRanges(): Array<{ key: string; start: number; end: number }> {
    return Object.entries(annotations).map(([key, entry]) => ({
      key,
      start: entry.range.start,
      end: entry.range.end,
    }));
  }

  function allEntries(): Record<string, AnnotationEntry> {
    return annotations;
  }

  /**
   * Replace all annotations with new data (used for undo/redo).
   * Does NOT sync to backend - caller is responsible for that.
   */
  function replaceAll(newAnnotations: Record<string, AnnotationEntry>): void {
    // Clear existing
    for (const key of Object.keys(annotations)) {
      delete annotations[key];
    }
    // Add new
    for (const [key, entry] of Object.entries(newAnnotations)) {
      annotations[key] = {
        range: { ...entry.range },
        content: JSON.parse(JSON.stringify(entry.content)),
      };
    }
  }

  return {
    get annotations() { return annotations; },
    /** Alias for annotations getter (for history system) */
    get all() { return annotations; },
    get,
    getByKey,
    upsert,
    remove,
    getAtLine,
    hasAnnotation,
    allRanges,
    allEntries,
    replaceAll,
  };
}
