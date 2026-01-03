/**
 * Fuzzy search utility wrapping fuse.js.
 * Used by tag autocomplete (#), slash commands (/), and command palette (:).
 */
import Fuse, { type FuseOptionKey, type IFuseOptions } from 'fuse.js';

const DEFAULTS: IFuseOptions<unknown> = {
  threshold: 0.4, // Tolerates 1-2 typos in short strings
  ignoreLocation: true, // Match anywhere in string
  shouldSort: true, // Sort by score (best matches first)
  minMatchCharLength: 1,
};

/**
 * Fuzzy search items by query.
 * Returns all items when query is empty (preserves current behavior).
 *
 * @param items - Array of items to search
 * @param query - Search query string
 * @param keys - Fuse.js keys to search (with optional weights)
 * @param limit - Optional max number of results
 */
export function fuzzySearch<T>(
  items: T[],
  query: string,
  keys: FuseOptionKey<T>[],
  limit?: number
): T[] {
  if (!query) {
    return limit ? items.slice(0, limit) : items;
  }

  const fuse = new Fuse(items, { ...DEFAULTS, keys });
  const results = fuse.search(query);
  const mapped = results.map((r) => r.item);

  return limit ? mapped.slice(0, limit) : mapped;
}
