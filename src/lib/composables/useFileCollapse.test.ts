import { describe, it, expect, vi } from 'vitest';
import { useFileCollapse, AUTO_COLLAPSE_THRESHOLD } from './useFileCollapse.svelte';
import type { DocView } from '$lib/display-rows';

function entry(index: number, changed = 1): DocView {
  const path = `f${index}.ts`;
  return {
    index,
    doc: { path, old_path: null, status: 'modified', unavailable: false, language: '', hunks: [] },
    path,
    dir: '',
    name: path,
    added: changed,
    deleted: 0,
    headerDisplayIndex: index * 10 + 1,
    endDisplayIndex: index * 10 + 9,
  };
}

describe('useFileCollapse', () => {
  it('toggles collapse state per file', () => {
    const collapse = useFileCollapse(() => [entry(0), entry(1)]);

    expect(collapse.isCollapsed(0)).toBe(false);
    expect(collapse.anyCollapsed).toBe(false);

    collapse.toggle(0);
    expect(collapse.isCollapsed(0)).toBe(true);
    expect(collapse.isCollapsed(1)).toBe(false);
    expect(collapse.anyCollapsed).toBe(true);

    collapse.toggle(0);
    expect(collapse.isCollapsed(0)).toBe(false);
  });

  it('collapseAll / expandAll cover every entry', () => {
    const collapse = useFileCollapse(() => [entry(0), entry(1), entry(2)]);

    collapse.collapseAll();
    expect([0, 1, 2].every((i) => collapse.isCollapsed(i))).toBe(true);

    collapse.expandAll();
    expect(collapse.anyCollapsed).toBe(false);
  });

  it('fires onCollapse only on expanded → collapsed transitions', () => {
    const onCollapse = vi.fn();
    const entries = [entry(0), entry(1)];
    const collapse = useFileCollapse(() => entries, { onCollapse });

    collapse.collapse(0);
    collapse.collapse(0); // already collapsed — no event
    expect(onCollapse).toHaveBeenCalledTimes(1);
    expect(onCollapse).toHaveBeenCalledWith(entries[0]);

    collapse.collapseAll(); // 0 already collapsed, only 1 transitions
    expect(onCollapse).toHaveBeenCalledTimes(2);
    expect(onCollapse).toHaveBeenLastCalledWith(entries[1]);

    collapse.expand(1);
    collapse.toggle(1); // toggle into collapsed fires too
    expect(onCollapse).toHaveBeenCalledTimes(3);
  });

  it('init seeds files above the auto-collapse threshold without firing onCollapse', () => {
    const onCollapse = vi.fn();
    const collapse = useFileCollapse(
      () => [entry(0, AUTO_COLLAPSE_THRESHOLD + 1), entry(1, 3)],
      { onCollapse },
    );

    collapse.init();

    expect(collapse.isCollapsed(0)).toBe(true);
    expect(collapse.isCollapsed(1)).toBe(false);
    expect(onCollapse).not.toHaveBeenCalled();
  });
});
