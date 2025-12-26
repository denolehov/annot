import { invoke } from '@tauri-apps/api/core';
import type { JSONContent } from '@tiptap/core';
import type { Range } from '$lib/range';
import type { Line } from '$lib/types';
import { rangeToKey, isLineInRange, validateRange } from '$lib/range';
import { extractContentNodes, isContentEmpty } from '$lib/tiptap';

export interface AnnotationEntry {
  range: Range;
  content: JSONContent;
  sealed: boolean;
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

  function isSealed(key: string): boolean {
    return annotations[key]?.sealed ?? false;
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
      annotations[key] = {
        range: { start: coords.startLine, end: coords.endLine },
        content,
        sealed: annotations[key]?.sealed ?? false
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

  function seal(key: string): void {
    const entry = annotations[key];
    if (entry) {
      annotations[key] = { ...entry, sealed: true };
    }
  }

  function unseal(key: string): void {
    const entry = annotations[key];
    if (entry) {
      annotations[key] = { ...entry, sealed: false };
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

  return {
    get annotations() { return annotations; },
    get,
    getByKey,
    isSealed,
    upsert,
    seal,
    unseal,
    remove,
    getAtLine,
    hasAnnotation,
    allRanges,
    allEntries,
  };
}
