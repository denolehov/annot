import type { JSONContent } from '@tiptap/core';
import type { TerraformRegion } from '$lib/types';
import type { AnnotationEntry } from './useAnnotations.svelte';

/**
 * SessionData represents the undoable state of a session.
 * This is what gets stored in the history stack.
 */
export interface SessionData {
  /** Annotations keyed by range string (e.g., "10-15") */
  annotations: Record<string, AnnotationEntry>;
  /** Terraform regions for the current file */
  terraform: TerraformRegion[];
  /** Session-level comment (TipTap JSON) */
  sessionComment: JSONContent | null;
  /** Selected exit mode ID */
  selectedExitMode: string | null;
}

/**
 * NarrativeEntry is a human-readable log entry for debugging.
 * NOT used for undo logic — purely for visibility.
 */
export interface NarrativeEntry {
  /** Homerow ID for display: "aa", "as", etc. */
  id: string;
  /** Timestamp when action occurred */
  timestamp: number;
  /** Human-readable description */
  label: string;
  /** Which history index this corresponds to */
  historyIndex: number;
}

const HOMEROW = 'asdfjkl';
const MAX_HISTORY = 100;

/**
 * Generate a homerow-friendly ID from an index.
 * IDs are stable and never recycled within a session.
 */
function generateId(index: number): string {
  const first = HOMEROW[Math.floor(index / HOMEROW.length) % HOMEROW.length];
  const second = HOMEROW[index % HOMEROW.length];
  return `${first}${second}`;
}

/**
 * Deep clone session data for immutable storage.
 */
function cloneSessionData(data: SessionData): SessionData {
  return {
    annotations: Object.fromEntries(
      Object.entries(data.annotations).map(([key, entry]) => [
        key,
        {
          range: { ...entry.range },
          content: JSON.parse(JSON.stringify(entry.content)),
          sealed: entry.sealed,
        },
      ])
    ),
    terraform: data.terraform.map(r => ({
      start_line: r.start_line,
      end_line: r.end_line,
      intent: JSON.parse(JSON.stringify(r.intent)),
    })),
    sessionComment: data.sessionComment ? JSON.parse(JSON.stringify(data.sessionComment)) : null,
    selectedExitMode: data.selectedExitMode,
  };
}

/**
 * Create an empty session data object.
 */
export function emptySessionData(): SessionData {
  return {
    annotations: {},
    terraform: [],
    sessionComment: null,
    selectedExitMode: null,
  };
}

export interface UseHistoryOptions {
  /** Called when state changes (for syncing to backend) */
  onStateChange?: (data: SessionData, label: string) => void;
}

/**
 * History composable implementing immutable state stack for undo/redo.
 *
 * Core principle: history[historyIndex] is always the current state.
 * - Undo = historyIndex--
 * - Redo = historyIndex++
 * - Mutation = push new state, truncate future
 */
export function useHistory(options: UseHistoryOptions = {}) {
  // History stack - array of immutable snapshots
  let history = $state<SessionData[]>([emptySessionData()]);
  let historyIndex = $state(0);

  // Narrative log for debugging (not authoritative)
  let narrative = $state<NarrativeEntry[]>([]);
  let narrativeIdCounter = $state(0);

  /**
   * Get the current state (read-only view).
   */
  function current(): SessionData {
    return history[historyIndex];
  }

  /**
   * Push a new state to history, truncating any future states.
   * @param newState The new state to push
   * @param label Human-readable description for the narrative
   */
  function push(newState: SessionData, label: string): void {
    // Clone to ensure immutability
    const cloned = cloneSessionData(newState);

    // Truncate any "future" states (redo history)
    history = history.slice(0, historyIndex + 1);

    // Push new state
    history.push(cloned);
    historyIndex++;

    // Cap history size
    if (history.length > MAX_HISTORY) {
      const excess = history.length - MAX_HISTORY;
      history = history.slice(excess);
      historyIndex -= excess;
    }

    // Record in narrative
    narrative.push({
      id: generateId(narrativeIdCounter++),
      timestamp: Date.now(),
      label,
      historyIndex,
    });

    // Notify listeners
    options.onStateChange?.(cloned, label);
  }

  /**
   * Undo: move back one state in history.
   * @returns true if undo was performed, false if at beginning
   */
  function undo(): boolean {
    if (historyIndex > 0) {
      historyIndex--;
      const state = current();
      options.onStateChange?.(state, 'Undo');
      return true;
    }
    return false;
  }

  /**
   * Redo: move forward one state in history.
   * @returns true if redo was performed, false if at end
   */
  function redo(): boolean {
    if (historyIndex < history.length - 1) {
      historyIndex++;
      const state = current();
      options.onStateChange?.(state, 'Redo');
      return true;
    }
    return false;
  }

  /**
   * Check if undo is available.
   */
  function canUndo(): boolean {
    return historyIndex > 0;
  }

  /**
   * Check if redo is available.
   */
  function canRedo(): boolean {
    return historyIndex < history.length - 1;
  }

  /**
   * Initialize history with a starting state.
   * Used when loading existing session data.
   */
  function initialize(initialState: SessionData): void {
    history = [cloneSessionData(initialState)];
    historyIndex = 0;
    narrative = [];
    narrativeIdCounter = 0;
  }

  /**
   * Get the narrative log for debugging/display.
   */
  function getNarrative(): NarrativeEntry[] {
    return narrative;
  }

  /**
   * Get history length (for debugging).
   */
  function getHistoryLength(): number {
    return history.length;
  }

  return {
    // State access
    get current() { return current(); },

    // Mutations
    push,
    undo,
    redo,
    initialize,

    // Queries
    canUndo,
    canRedo,
    getNarrative,
    getHistoryLength,

    // For debugging
    get historyIndex() { return historyIndex; },
  };
}

export type UseHistoryReturn = ReturnType<typeof useHistory>;
