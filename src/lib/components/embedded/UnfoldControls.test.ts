import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import UnfoldControls from './UnfoldControls.svelte';

describe('UnfoldControls', () => {
  it('collapses a sub-step gap to a single expand-all icon carrying the count in its tooltip', () => {
    render(UnfoldControls, {
      props: { size: 5, showUp: true, showDown: true, onExpand: vi.fn() },
    });

    const buttons = screen.getAllByRole('button');
    expect(buttons).toHaveLength(1);
    expect(buttons[0]).toHaveAttribute('title', 'Expand all — 5 unchanged lines');
  });

  it('stacks ▼ over ▲ (GitHub order) for a gap larger than one step', () => {
    render(UnfoldControls, {
      props: { size: 40, showUp: true, showDown: true, onExpand: vi.fn() },
    });

    const buttons = screen.getAllByRole('button');
    expect(buttons).toHaveLength(2);
    expect(buttons[0]).toHaveAttribute('title', 'Expand down — 40 unchanged lines');
    expect(buttons[1]).toHaveAttribute('title', 'Expand up — 40 unchanged lines');
  });

  it('shows a single arrow for a one-sided gap', () => {
    render(UnfoldControls, {
      props: { size: 40, showUp: false, showDown: true, onExpand: vi.fn() },
    });

    const buttons = screen.getAllByRole('button');
    expect(buttons).toHaveLength(1);
    expect(buttons[0]).toHaveAttribute('title', 'Expand down — 40 unchanged lines');
  });

  it('never surfaces the count as always-visible text', () => {
    render(UnfoldControls, {
      props: { size: 40, showUp: true, showDown: true, onExpand: vi.fn() },
    });

    expect(screen.queryByText(/unchanged/)).toBeNull();
  });

  it('invokes onExpand with the right direction/amount and reports failure via the tooltip, not text', async () => {
    const onExpand = vi.fn().mockRejectedValue(new Error('boom'));
    render(UnfoldControls, {
      props: { size: 40, showUp: true, showDown: false, onExpand },
    });

    const [upBtn] = screen.getAllByRole('button');
    upBtn.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    expect(onExpand).toHaveBeenCalledWith({ direction: 'up', amount: 'step' });

    await Promise.resolve();
    await Promise.resolve();
    await vi.waitFor(() =>
      expect(screen.getByRole('button')).toHaveAttribute('title', "couldn't expand — try again"),
    );
    expect(screen.queryByText(/couldn't expand/)).toBeNull();
  });
});
// Propagation into the host gutter's selection handlers is covered in
// RegularLines.test.ts — Svelte delegates onclick/onpointerdown to the mount
// root, so only an integration through LineRow exercises the real ordering.
