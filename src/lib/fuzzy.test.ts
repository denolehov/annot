import { describe, it, expect } from 'vitest';
import { fuzzySearch } from './fuzzy';

interface Tag {
  id: string;
  name: string;
  instruction: string;
}

const tags: Tag[] = [
  { id: '1', name: 'SECURITY', instruction: 'Security-sensitive code requiring careful review' },
  { id: '2', name: 'TODO', instruction: 'Mark for future implementation' },
  { id: '3', name: 'REFACTOR', instruction: 'Code that should be refactored' },
  { id: '4', name: 'BUG', instruction: 'Potential bug or issue' },
  { id: '5', name: 'INSECURE', instruction: 'Known security vulnerability' },
];

const keys = [
  { name: 'name' as const, weight: 2 },
  { name: 'instruction' as const, weight: 1 },
];

describe('fuzzySearch', () => {
  it('returns all items when query is empty', () => {
    const result = fuzzySearch(tags, '', keys);
    expect(result).toEqual(tags);
  });

  it('returns limited items when query is empty and limit is set', () => {
    const result = fuzzySearch(tags, '', keys, 2);
    expect(result).toHaveLength(2);
    expect(result).toEqual(tags.slice(0, 2));
  });

  it('finds exact matches', () => {
    const result = fuzzySearch(tags, 'SECURITY', keys);
    expect(result[0].name).toBe('SECURITY');
  });

  it('finds partial matches', () => {
    const result = fuzzySearch(tags, 'sec', keys);
    expect(result.length).toBeGreaterThan(0);
    // SECURITY should be in results
    expect(result.some((t) => t.name === 'SECURITY')).toBe(true);
  });

  it('tolerates typos', () => {
    const result = fuzzySearch(tags, 'secruity', keys);
    // Should still find SECURITY despite typo
    expect(result.some((t) => t.name === 'SECURITY')).toBe(true);
  });

  it('searches instruction field', () => {
    const result = fuzzySearch(tags, 'vulnerability', keys);
    // Should find INSECURE which has "vulnerability" in instruction
    expect(result.some((t) => t.name === 'INSECURE')).toBe(true);
  });

  it('ranks name matches higher than instruction matches', () => {
    // "sec" appears in SECURITY (name) and INSECURE (name) and instruction
    const result = fuzzySearch(tags, 'sec', keys);
    // Both SECURITY and INSECURE should be near the top
    const topNames = result.slice(0, 2).map((t) => t.name);
    expect(topNames).toContain('SECURITY');
  });

  it('respects limit parameter', () => {
    const result = fuzzySearch(tags, 'sec', keys, 1);
    expect(result).toHaveLength(1);
  });

  it('returns empty array when no matches', () => {
    const result = fuzzySearch(tags, 'zzzzzzzzz', keys);
    expect(result).toEqual([]);
  });

  it('is case insensitive', () => {
    const lower = fuzzySearch(tags, 'security', keys);
    const upper = fuzzySearch(tags, 'SECURITY', keys);
    expect(lower[0].name).toBe('SECURITY');
    expect(upper[0].name).toBe('SECURITY');
  });
});
