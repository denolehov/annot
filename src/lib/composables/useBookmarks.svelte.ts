import { invoke } from '@tauri-apps/api/core';
import type { Bookmark } from '$lib/types';

export interface LineRange {
  start: number;
  end: number;
  id: string;
}

export interface UseBookmarks {
  /** All bookmarks (reactive). */
  readonly all: Bookmark[];
  /** Lookup by ID. */
  readonly byId: Map<string, Bookmark>;
  /** Selection bookmarks as line ranges for visual indicators. */
  readonly lineRanges: LineRange[];

  /** Current session bookmark state. */
  readonly isSessionBookmarked: boolean;
  readonly sessionBookmarkId: string | null;
  /** Last created bookmark ID (for [e] edit flow). */
  readonly lastCreatedId: string | null;

  // CRUD operations
  createSession(label?: string): Promise<Bookmark>;
  createSelection(start: number, end: number, label?: string): Promise<Bookmark>;
  update(id: string, label: string): Promise<void>;
  delete(id: string): Promise<void>;

  // Session bookmark toggle (used by header button)
  toggleSession(): Promise<void>;
  // Selection bookmark toggle (create or delete if same range)
  toggleSelection(start: number, end: number): Promise<void>;

  // Query
  findByLineRange(start: number, end: number): Bookmark | undefined;
  isLineInBookmarkedRange(displayIdx: number): boolean;
  isFirstLineOfBookmark(displayIdx: number): boolean;
  getBookmarkIdAtStart(displayIdx: number): string | undefined;

  // Clear last created (for edit flow)
  clearLastCreated(): void;
}

export function useBookmarks(initialBookmarks: Bookmark[] = []): UseBookmarks {
  let bookmarks = $state<Bookmark[]>(initialBookmarks);

  // Session bookmark tracking
  let sessionBookmarkId = $state<string | null>(null);
  let lastCreatedId = $state<string | null>(null);

  // Line ranges for selection bookmarks (tracked client-side since backend doesn't store them)
  // This is ephemeral — only populated for bookmarks created in this session
  let lineRangesMap = $state<Map<string, { start: number; end: number }>>(new Map());

  // Derived: lookup map
  const byId = $derived(new Map(bookmarks.map((b) => [b.id, b])));

  // Derived: selection bookmark line ranges as array
  const lineRanges = $derived.by(() => {
    const ranges: LineRange[] = [];
    for (const [id, range] of lineRangesMap) {
      ranges.push({ ...range, id });
    }
    return ranges;
  });

  // Derived: is session bookmarked
  const isSessionBookmarked = $derived(sessionBookmarkId !== null);

  // CRUD
  async function createSession(label?: string): Promise<Bookmark> {
    const bookmark = await invoke<Bookmark>('create_bookmark', {
      label: label ?? null,
    });
    bookmarks = [...bookmarks, bookmark];
    sessionBookmarkId = bookmark.id;
    lastCreatedId = bookmark.id;
    return bookmark;
  }

  async function createSelection(
    start: number,
    end: number,
    label?: string
  ): Promise<Bookmark> {
    const normalizedStart = Math.min(start, end);
    const normalizedEnd = Math.max(start, end);

    const bookmark = await invoke<Bookmark>('create_selection_bookmark', {
      startLine: normalizedStart,
      endLine: normalizedEnd,
      label: label ?? null,
    });
    bookmarks = [...bookmarks, bookmark];
    lastCreatedId = bookmark.id;

    // Track line range for visual indicator
    lineRangesMap = new Map(lineRangesMap).set(bookmark.id, {
      start: normalizedStart,
      end: normalizedEnd,
    });

    return bookmark;
  }

  async function update(id: string, label: string): Promise<void> {
    await invoke('update_bookmark', { id, label });
    bookmarks = bookmarks.map((b) => (b.id === id ? { ...b, label } : b));
  }

  async function deleteBookmark(id: string): Promise<void> {
    await invoke('delete_bookmark', { id });
    bookmarks = bookmarks.filter((b) => b.id !== id);
    if (sessionBookmarkId === id) {
      sessionBookmarkId = null;
    }
    if (lastCreatedId === id) {
      lastCreatedId = null;
    }
    // Remove from line ranges if it was a selection bookmark
    if (lineRangesMap.has(id)) {
      const newMap = new Map(lineRangesMap);
      newMap.delete(id);
      lineRangesMap = newMap;
    }
  }

  // Toggle session bookmark
  async function toggleSession(): Promise<void> {
    if (sessionBookmarkId) {
      await deleteBookmark(sessionBookmarkId);
    } else {
      await createSession();
    }
  }

  // Toggle selection bookmark (delete if same range exists)
  async function toggleSelection(start: number, end: number): Promise<void> {
    const normalizedStart = Math.min(start, end);
    const normalizedEnd = Math.max(start, end);

    const existing = lineRanges.find(
      (r) => r.start === normalizedStart && r.end === normalizedEnd
    );

    if (existing) {
      await deleteBookmark(existing.id);
    } else {
      await createSelection(normalizedStart, normalizedEnd);
    }
  }

  // Query
  function findByLineRange(start: number, end: number): Bookmark | undefined {
    const normalizedStart = Math.min(start, end);
    const normalizedEnd = Math.max(start, end);

    const range = lineRanges.find(
      (r) => r.start === normalizedStart && r.end === normalizedEnd
    );
    return range ? byId.get(range.id) : undefined;
  }

  function isLineInBookmarkedRange(displayIdx: number): boolean {
    return lineRanges.some(
      (range) => displayIdx >= range.start && displayIdx <= range.end
    );
  }

  function isFirstLineOfBookmark(displayIdx: number): boolean {
    return lineRanges.some((range) => displayIdx === range.start);
  }

  function getBookmarkIdAtStart(displayIdx: number): string | undefined {
    return lineRanges.find((range) => displayIdx === range.start)?.id;
  }

  function clearLastCreated(): void {
    lastCreatedId = null;
  }

  return {
    get all() {
      return bookmarks;
    },
    get byId() {
      return byId;
    },
    get lineRanges() {
      return lineRanges;
    },
    get isSessionBookmarked() {
      return isSessionBookmarked;
    },
    get sessionBookmarkId() {
      return sessionBookmarkId;
    },
    get lastCreatedId() {
      return lastCreatedId;
    },

    createSession,
    createSelection,
    update,
    delete: deleteBookmark,
    toggleSession,
    toggleSelection,
    findByLineRange,
    isLineInBookmarkedRange,
    isFirstLineOfBookmark,
    getBookmarkIdAtStart,
    clearLastCreated,
  };
}
