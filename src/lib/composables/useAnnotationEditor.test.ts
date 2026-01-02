import { describe, it, expect, vi, beforeEach } from 'vitest';
import { flushSync } from 'svelte';
// @ts-expect-error - internal Svelte API for testing effects
import { effect_root } from 'svelte/internal/client';

// Mock TipTap Editor - it requires a real DOM environment
vi.mock('@tiptap/core', () => {
  class MockEditor {
    commands = {
      blur: vi.fn(),
      focus: vi.fn(),
      setContent: vi.fn(),
    };
    chain = vi.fn(() => ({
      focus: vi.fn().mockReturnThis(),
      deleteRange: vi.fn().mockReturnThis(),
      insertContent: vi.fn().mockReturnThis(),
      insertContentAt: vi.fn().mockReturnThis(),
      run: vi.fn(),
    }));
    destroy = vi.fn();
    getJSON = vi.fn(() => ({ type: 'doc', content: [] }));
    setEditable = vi.fn();
    on = vi.fn();
    off = vi.fn();

    constructor(public options: Record<string, unknown>) {}
  }

  return {
    Editor: MockEditor,
  };
});

vi.mock('@tiptap/starter-kit', () => ({
  default: { configure: vi.fn(() => ({})) },
}));

vi.mock('@tiptap/extension-placeholder', () => ({
  default: { configure: vi.fn(() => ({})) },
}));

// Mock the tiptap utilities
vi.mock('../tiptap', () => ({
  trimContent: vi.fn((content) => content),
  isContentEmpty: vi.fn(() => true),
  ImagePasteHandler: { configure: vi.fn(() => ({})) },
  TextPasteHandler: {},
  SlashCommands: { configure: vi.fn(() => ({})) },
  EditorShortcuts: { configure: vi.fn(() => ({})) },
  createSlashSuggestion: vi.fn(() => ({})),
  createSuggestionRender: vi.fn(() => () => ({
    onStart: vi.fn(),
    onUpdate: vi.fn(),
    onKeyDown: vi.fn(),
    onExit: vi.fn(),
  })),
  parseFenceFromJson: vi.fn(() => null),
  transformReplaceFenceToPreview: vi.fn((json) => json),
  transformReplacePreviewToFence: vi.fn((json) => json),
}));

// Mock the new tiptap extensions
vi.mock('../tiptap/extensions', () => ({
  ErrorChip: {},
  TagChip: { configure: vi.fn(() => ({})) },
  PasteChip: {},
  MediaChip: {},
  BookmarkChip: { configure: vi.fn(() => ({})) },
  ReplacePreview: {},
  ExcalidrawChip: {},
  ExcalidrawPlaceholder: {},
}));

import { useAnnotationEditor } from './useAnnotationEditor.svelte';

describe('useAnnotationEditor', () => {
  const createOptions = (overrides = {}) => ({
    element: () => undefined,
    getContent: () => undefined,
    getSealed: () => false,
    getTags: () => [],
    getBookmarks: () => [],
    getAllowsImagePaste: () => true,
    getOnUpdate: () => vi.fn(),
    getOnDismiss: () => vi.fn(),
    getOnImagePasteBlocked: () => undefined,
    ...overrides,
  });

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('starts with null editor when no element provided', () => {
    let state: ReturnType<typeof useAnnotationEditor>;
    const cleanup = effect_root(() => {
      state = useAnnotationEditor(createOptions());
    });
    expect(state!.editor).toBeNull();
    cleanup();
  });

  it('starts with inactive suggestion states', () => {
    let state: ReturnType<typeof useAnnotationEditor>;
    const cleanup = effect_root(() => {
      state = useAnnotationEditor(createOptions());
    });
    expect(state!.tagSuggestion.active).toBe(false);
    expect(state!.tagSuggestion.items).toEqual([]);
    expect(state!.slashSuggestion.active).toBe(false);
    expect(state!.slashSuggestion.items).toEqual([]);
    cleanup();
  });

  it('tracks Excalidraw modal state', () => {
    let state: ReturnType<typeof useAnnotationEditor>;
    const cleanup = effect_root(() => {
      state = useAnnotationEditor(createOptions());
    });

    expect(state!.isExcalidrawModalOpen).toBe(false);

    state!.setExcalidrawModalOpen(true);
    expect(state!.isExcalidrawModalOpen).toBe(true);

    state!.setExcalidrawModalOpen(false);
    expect(state!.isExcalidrawModalOpen).toBe(false);

    cleanup();
  });

  it('creates editor when element is provided', () => {
    const mockElement = document.createElement('div');

    let state: ReturnType<typeof useAnnotationEditor>;
    const cleanup = effect_root(() => {
      state = useAnnotationEditor(createOptions({
        element: () => mockElement,
      }));
    });

    // Run effects
    flushSync(() => {});

    expect(state!.editor).not.toBeNull();
    cleanup();
  });

  it('passes initial content to editor', () => {
    const mockElement = document.createElement('div');
    const initialContent = { type: 'doc', content: [{ type: 'paragraph' }] };

    let state: ReturnType<typeof useAnnotationEditor>;
    const cleanup = effect_root(() => {
      state = useAnnotationEditor(createOptions({
        element: () => mockElement,
        getContent: () => initialContent,
      }));
    });

    flushSync(() => {});

    // Check the editor was created with the correct content
    expect(state!.editor).not.toBeNull();
    expect((state!.editor as any).options.content).toEqual(initialContent);
    cleanup();
  });

  it('sets editable based on sealed state', () => {
    const mockElement = document.createElement('div');

    let state: ReturnType<typeof useAnnotationEditor>;
    const cleanup = effect_root(() => {
      state = useAnnotationEditor(createOptions({
        element: () => mockElement,
        getSealed: () => true,
      }));
    });

    flushSync(() => {});

    // Check the editor was created with editable: false
    expect(state!.editor).not.toBeNull();
    expect((state!.editor as any).options.editable).toBe(false);
    cleanup();
  });
});
