import type { Line } from '$lib/types';

export interface SearchMatch {
  displayIndex: number;
  ranges: Array<{ start: number; end: number }>;
}

interface SearchState {
  isOpen: boolean;
  query: string;
  matches: SearchMatch[];
  currentMatchIndex: number; // index into matches array, -1 if none
}

/**
 * Extract text content from HTML.
 * This ensures we search the same text that gets rendered in the DOM.
 */
function getRenderedText(line: Line): string {
  if (!line.html) return line.content;

  // Use a temporary element to extract text from HTML
  const div = document.createElement('div');
  if (line.html.type === 'full') {
    div.innerHTML = line.html.value;
  } else if (line.html.type === 'cells') {
    // Join without spaces to match DOM structure (cells are adjacent in TreeWalker)
    div.innerHTML = line.html.value.join('');
  } else {
    // Exhaustive check: if a new LineHtml type is added, TypeScript will error here
    const _exhaustive: never = line.html;
    return line.content;
  }
  return div.textContent ?? line.content;
}

/**
 * useSearch - Manages in-file search state.
 *
 * @param getLines - Function returning current lines array
 * @param scrollToLine - Callback to scroll viewport to a display index
 */
export function useSearch(
  getLines: () => Line[],
  scrollToLine: (displayIndex: number) => void
) {
  let state = $state<SearchState>({
    isOpen: false,
    query: '',
    matches: [],
    currentMatchIndex: -1,
  });

  function findMatches(lines: Line[], query: string): SearchMatch[] {
    if (!query) return [];

    const queryLower = query.toLowerCase();
    const matches: SearchMatch[] = [];

    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];
      // Search rendered text to match DOM text node offsets
      const textLower = getRenderedText(line).toLowerCase();
      const ranges: Array<{ start: number; end: number }> = [];

      let pos = 0;
      let idx: number;
      while ((idx = textLower.indexOf(queryLower, pos)) !== -1) {
        ranges.push({ start: idx, end: idx + query.length });
        pos = idx + 1;
      }

      if (ranges.length > 0) {
        matches.push({
          displayIndex: i + 1, // 1-indexed
          ranges,
        });
      }
    }

    return matches;
  }

  function open() {
    state.isOpen = true;
  }

  function close() {
    state.isOpen = false;
    state.query = '';
    state.matches = [];
    state.currentMatchIndex = -1;
  }

  function setQuery(query: string) {
    state.query = query;
    state.matches = findMatches(getLines(), query);
    state.currentMatchIndex = state.matches.length > 0 ? 0 : -1;

    if (state.currentMatchIndex >= 0) {
      scrollToLine(state.matches[state.currentMatchIndex].displayIndex);
    }
  }

  function nextMatch() {
    if (state.matches.length === 0) return;

    state.currentMatchIndex = (state.currentMatchIndex + 1) % state.matches.length;
    scrollToLine(state.matches[state.currentMatchIndex].displayIndex);
  }

  function prevMatch() {
    if (state.matches.length === 0) return;

    state.currentMatchIndex =
      (state.currentMatchIndex - 1 + state.matches.length) % state.matches.length;
    scrollToLine(state.matches[state.currentMatchIndex].displayIndex);
  }

  function getCurrentMatch(): SearchMatch | null {
    if (state.currentMatchIndex < 0 || state.currentMatchIndex >= state.matches.length) {
      return null;
    }
    return state.matches[state.currentMatchIndex];
  }

  return {
    get isOpen() {
      return state.isOpen;
    },
    get query() {
      return state.query;
    },
    get matches() {
      return state.matches;
    },
    get currentMatchIndex() {
      return state.currentMatchIndex;
    },
    get totalMatches() {
      return state.matches.length;
    },
    getCurrentMatch,
    open,
    close,
    setQuery,
    nextMatch,
    prevMatch,
  };
}

export type SearchContext = ReturnType<typeof useSearch>;
