import { invoke } from '@tauri-apps/api/core';
import type { JSONContent } from '@tiptap/core';
import type { Range } from '$lib/range';
import type { Line } from '$lib/types';
import { type Anchor, anchorKeys, endpointKeys } from '$lib/anchor';
import type { DiffDisplay } from '$lib/display-rows';
import { extractContentNodes, isContentEmpty } from '$lib/tiptap';

export interface AnnotationEntry {
  id: string;
  anchor: Anchor;
  content: JSONContent;
}

/** Deep-clone an entry for immutable storage (history stack, restore snapshots). */
export function cloneAnnotationEntry(entry: AnnotationEntry): AnnotationEntry {
  return {
    id: entry.id,
    anchor: JSON.parse(JSON.stringify(entry.anchor)),
    content: JSON.parse(JSON.stringify(entry.content)),
  };
}

export interface UseAnnotationsOptions {
  /** Lines array for resolving anchors to display rows (non-diff modes) */
  getLines: () => Line[];
  /** The display walk; owns diff coordinate resolution when present */
  getDisplay?: () => DiffDisplay | null;
}

export function useAnnotations(options: UseAnnotationsOptions) {
  let annotations: Record<string, AnnotationEntry> = $state({});

  // Anchors live in source coordinates; display rows are resolved through this
  // map. Diff mode reads the walk's byEndpoint (context rows answer on both
  // sides); other modes build the map from the lines array.
  const endpointToRow = $derived.by(() => {
    const display = options.getDisplay?.() ?? null;
    if (display) return display.byEndpoint;

    const map = new Map<string, number>();
    options.getLines().forEach((line, i) => {
      for (const key of endpointKeys(line)) map.set(key, i + 1);
    });
    return map;
  });

  function resolveAnchor(anchor: Anchor): Range | null {
    const [startKey, endKey] = anchorKeys(anchor);
    const start = endpointToRow.get(startKey);
    const end = endpointToRow.get(endKey);
    if (start === undefined || end === undefined) return null;
    return { start: Math.min(start, end), end: Math.max(start, end) };
  }

  // Resolved display span per annotation id. Reads only entry anchors, so
  // in-place content edits (the per-keystroke case) don't invalidate it —
  // same contract as the row sets below.
  const spans = $derived.by(() => {
    const map = new Map<string, Range>();
    for (const entry of Object.values(annotations)) {
      const span = resolveAnchor(entry.anchor);
      if (span) {
        map.set(entry.id, span);
      } else {
        // Impossible pre-unfold/per-file-docs; defined fold-away behavior:
        // the annotation is hidden, not crashed on.
        console.warn('Annotation anchor does not resolve against current lines:', entry.id, entry.anchor);
      }
    }
    return map;
  });

  // Display rows covered by any annotation. Rebuilt once when the entry set
  // changes, so per-line `hasAnnotation` is an O(1) Set lookup. Without this,
  // adding one annotation re-scans every entry for all ~10k lines (O(N·A)) and
  // stalls the reactive flush — the dominant cost while annotating large files.
  const annotatedRows = $derived.by(() => {
    const set = new Set<number>();
    for (const span of spans.values()) {
      for (let i = span.start; i <= span.end; i++) {
        set.add(i);
      }
    }
    return set;
  });

  // Maps each annotation's resolved end row to its entry — the end row hosts
  // the editor slot. One annotation per end row (invariant). `atEndRow` is
  // called per line render, so this must stay an O(1) lookup.
  const byEndRow = $derived.by(() => {
    const map = new Map<number, AnnotationEntry>();
    for (const [id, span] of spans) {
      const entry = annotations[id];
      if (entry) map.set(span.end, entry);
    }
    return map;
  });

  function getById(id: string): AnnotationEntry | undefined {
    return annotations[id];
  }

  // Backend syncs pending a flush, coalesced per annotation id. The editor's
  // onUpdate fires once per keystroke; local `annotations` state updates
  // immediately (so the UI stays reactive), but the IPC — which serializes the
  // whole content tree across the JS↔Rust bridge every call — is debounced.
  // The backend keeps annotations in memory and only reads them at
  // finish_review, so `flush()` MUST run before the window closes or the last
  // keystrokes are lost (wired in +page.svelte's onCloseRequested).
  type PendingSync =
    | { op: 'upsert'; id: string; path: string; anchor: Anchor; content: ReturnType<typeof extractContentNodes> }
    | { op: 'delete'; path: string; id: string };

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
              id: op.id,
              path: op.path,
              anchor: op.anchor,
              content: op.content
            })
          : invoke('delete_annotation', {
              path: op.path,
              id: op.id
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

  function upsert(id: string, anchor: Anchor, content: JSONContent | null): void {
    if (content && !isContentEmpty(content)) {
      // Mutate content in place for an existing annotation, so editing (which
      // fires per keystroke) neither changes the store's key set nor touches
      // the anchor — the resolution maps above stay valid while typing.
      const existing = annotations[id];
      if (existing) {
        existing.content = content;
      } else {
        annotations[id] = { id, anchor, content };
      }
      pending.set(id, {
        op: 'upsert',
        id,
        path: anchor.path,
        anchor,
        content: extractContentNodes(content)
      });
    } else {
      const existing = annotations[id];
      delete annotations[id];
      if (existing) {
        pending.set(id, { op: 'delete', path: anchor.path, id });
      } else {
        // Never synced — nothing to delete on the backend, and this cancels
        // any not-yet-flushed pending upsert.
        pending.delete(id);
      }
    }
    scheduleFlush();
  }

  function remove(id: string): void {
    delete annotations[id];
  }

  function atEndRow(displayIdx: number): AnnotationEntry | null {
    return byEndRow.get(displayIdx) ?? null;
  }

  /** The entry whose resolved span exactly matches `range`, if any. */
  function atSpan(range: Range): AnnotationEntry | null {
    const start = Math.min(range.start, range.end);
    const end = Math.max(range.start, range.end);
    for (const [id, span] of spans) {
      if (span.start === start && span.end === end) return annotations[id] ?? null;
    }
    return null;
  }

  function hasAnnotation(displayIdx: number): boolean {
    return annotatedRows.has(displayIdx);
  }

  function spanOf(id: string): Range | null {
    return spans.get(id) ?? null;
  }

  function spanOfAnchor(anchor: Anchor): Range | null {
    return resolveAnchor(anchor);
  }

  function allEntries(): Record<string, AnnotationEntry> {
    return annotations;
  }

  /**
   * Replace all annotations (undo/redo). Diffs by id against the current
   * store and syncs the backend through the pending queue: ids that vanish
   * are deleted, every snapshot entry is re-upserted (idempotent, debounced).
   */
  function restore(snapshot: Record<string, AnnotationEntry>): void {
    for (const [id, entry] of Object.entries(annotations)) {
      if (!snapshot[id]) {
        pending.set(id, { op: 'delete', path: entry.anchor.path, id });
      }
    }
    for (const id of Object.keys(annotations)) {
      delete annotations[id];
    }
    for (const [id, entry] of Object.entries(snapshot)) {
      annotations[id] = cloneAnnotationEntry(entry);
      pending.set(id, {
        op: 'upsert',
        id,
        path: entry.anchor.path,
        anchor: entry.anchor,
        content: extractContentNodes(entry.content)
      });
    }
    scheduleFlush();
  }

  return {
    get annotations() { return annotations; },
    /** Alias for annotations getter (for history system) */
    get all() { return annotations; },
    getById,
    upsert,
    flush,
    remove,
    atEndRow,
    atSpan,
    hasAnnotation,
    spanOf,
    spanOfAnchor,
    allEntries,
    restore,
  };
}
