import { invoke } from '@tauri-apps/api/core';
import type { JSONContent } from '@tiptap/core';
import type { Range } from '$lib/range';
import type { Line } from '$lib/types';
import { rangeToKey, validateRange } from '$lib/range';
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

  // Display indices covered by any annotation. Rebuilt once when `annotations`
  // changes, so per-line `hasAnnotation` is an O(1) Set lookup. Without this,
  // adding one annotation re-scans every entry for all ~10k lines (O(N·A)) and
  // stalls the reactive flush — the dominant cost while annotating large files.
  const annotatedLines = $derived.by(() => {
    const set = new Set<number>();
    for (const entry of Object.values(annotations)) {
      for (let i = entry.range.start; i <= entry.range.end; i++) {
        set.add(i);
      }
    }
    return set;
  });

  // Maps each annotation's end line to its entry. `getAtLine` is called per
  // line render (via getRangeKeyForLine), so a linear scan is O(N·A) across a
  // full render — same shape as the hasAnnotation cost above. In-place content
  // edits don't change which end lines exist, so this stays valid while typing.
  const byEndLine = $derived.by(() => {
    const map = new Map<number, { key: string; content: JSONContent }>();
    for (const [key, entry] of Object.entries(annotations)) {
      map.set(entry.range.end, { key, content: entry.content });
    }
    return map;
  });

  function get(range: Range): JSONContent | undefined {
    return annotations[rangeToKey(range)]?.content;
  }

  function getByKey(key: string): AnnotationEntry | undefined {
    return annotations[key];
  }

  // Backend syncs pending a flush, coalesced per annotation key. The editor's
  // onUpdate fires once per keystroke; local `annotations` state updates
  // immediately (so the UI stays reactive), but the IPC — which serializes the
  // whole content tree across the JS↔Rust bridge every call — is debounced.
  // The backend keeps annotations in memory and only reads them at
  // finish_review, so `flush()` MUST run before the window closes or the last
  // keystrokes are lost (wired in +page.svelte's onCloseRequested).
  type PendingSync =
    | { op: 'upsert'; path: string; startLine: number; endLine: number; content: ReturnType<typeof extractContentNodes> }
    | { op: 'delete'; path: string; startLine: number; endLine: number };

  const pending = new Map<string, PendingSync>();
  let flushTimer: ReturnType<typeof setTimeout> | null = null;
  const FLUSH_DELAY_MS = 250;

  function cancelFlush(): void {
    if (flushTimer !== null) {
      clearTimeout(flushTimer);
      flushTimer = null;
    }
  }

  async function flush(): Promise<void> {
    cancelFlush();
    if (pending.size === 0) return;
    // Snapshot and clear synchronously so keystrokes landing during the await
    // accumulate into a fresh batch rather than being dropped.
    const ops = [...pending.values()];
    pending.clear();
    await Promise.all(
      ops.map((op) =>
        op.op === 'upsert'
          ? invoke('upsert_annotation', {
              path: op.path,
              startLine: op.startLine,
              endLine: op.endLine,
              content: op.content
            })
          : invoke('delete_annotation', {
              path: op.path,
              startLine: op.startLine,
              endLine: op.endLine
            })
      )
    );
  }

  function scheduleFlush(): void {
    cancelFlush();
    flushTimer = setTimeout(() => {
      flushTimer = null;
      flush().catch((e) => console.error('Failed to sync annotations:', e));
    }, FLUSH_DELAY_MS);
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
      // Mutate content in place for an existing annotation, so editing (which fires
      // per keystroke) doesn't change the `annotations` key set — `annotatedLines`
      // only reads ranges, so it stays valid and hasAnnotation doesn't re-run on all
      // ~10k lines. Only creating a new annotation changes the key set.
      const existing = annotations[key];
      if (existing) {
        existing.content = content;
      } else {
        annotations[key] = { range: normalizedRange, content };
      }
      pending.set(key, {
        op: 'upsert',
        path: coords.path,
        startLine: coords.startLine,
        endLine: coords.endLine,
        content: extractContentNodes(content)
      });
    } else {
      delete annotations[key];
      pending.set(key, {
        op: 'delete',
        path: coords.path,
        startLine: coords.startLine,
        endLine: coords.endLine
      });
    }
    scheduleFlush();
  }

  function remove(key: string): void {
    delete annotations[key];
  }

  function getAtLine(displayIdx: number): { key: string; content: JSONContent } | null {
    return byEndLine.get(displayIdx) ?? null;
  }

  function hasAnnotation(displayIdx: number): boolean {
    return annotatedLines.has(displayIdx);
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
    flush,
    remove,
    getAtLine,
    hasAnnotation,
    allRanges,
    allEntries,
    replaceAll,
  };
}
