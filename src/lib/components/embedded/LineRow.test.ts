import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import { createRawSnippet } from 'svelte';
import LineRow from './LineRow.svelte';

const handlers = {
  isLineHighlighted: vi.fn(() => false),
  hasAnnotation: vi.fn(() => false),
  handleLineEnter: vi.fn(),
  handleLineLeave: vi.fn(),
  handlePointerDown: vi.fn(),
  handleGutterClick: vi.fn(),
};

vi.mock('$lib/context', () => ({
  getAnnotContext: () => ({
    interaction: {
      isLineHighlighted: handlers.isLineHighlighted,
      handleLineEnter: handlers.handleLineEnter,
      handleLineLeave: handlers.handleLineLeave,
      handlePointerDown: handlers.handlePointerDown,
      handleGutterClick: handlers.handleGutterClick,
    },
    annotations: {
      hasAnnotation: handlers.hasAnnotation,
    },
    markdownMetadata: null,
  }),
}));

const textSnippet = (text: string) =>
  createRawSnippet(() => ({
    render: () => `<span>${text}</span>`,
  }));

describe('LineRow interactive gating', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders the add-annotation button and a clickable gutter when interactive', () => {
    render(LineRow, {
      props: {
        interactive: true,
        displayIndex: 3,
        gutter: textSnippet('3'),
        code: textSnippet('const x = 1;'),
      },
    });

    expect(screen.getByLabelText('Add annotation')).toBeInTheDocument();
    const gutter = document.querySelector('.gutter');
    expect(gutter).toHaveAttribute('role', 'button');
    expect(gutter?.getAttribute('data-display-idx')).toBeNull();
    expect(document.querySelector('[data-display-idx="3"]')).not.toBeNull();
  });

  it('omits the add-btn and click wiring, and never queries selection/annotation, when non-interactive', () => {
    render(LineRow, {
      props: {
        interactive: false,
        gutter: textSnippet('⋯'),
        code: textSnippet(''),
      },
    });

    expect(screen.queryByLabelText('Add annotation')).toBeNull();
    const gutter = document.querySelector('.gutter');
    expect(gutter).toHaveAttribute('role', 'presentation');
    expect(document.querySelector('[data-display-idx]')).toBeNull();
    expect(handlers.isLineHighlighted).not.toHaveBeenCalled();
    expect(handlers.hasAnnotation).not.toHaveBeenCalled();
  });
});
