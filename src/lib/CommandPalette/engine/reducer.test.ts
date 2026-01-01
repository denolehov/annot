import { describe, it, expect } from 'vitest';
import type { Component } from 'svelte';
import { reduce, computeItemList } from './reducer';
import type { State, Action, QueryContext, Namespace, Item, Command, ItemComponentProps } from './types';

// Mock component for tests (not rendered, just needed for type satisfaction)
const MockItemComponent = null as unknown as Component<ItemComponentProps>;

// Test namespace with regular items
const tagsNamespace: Namespace = {
  id: 'tags',
  label: 'Tags',
  icon: 'hashtag',
  ItemComponent: MockItemComponent,
  fields: [{ key: 'name', label: 'Name', type: 'text', required: true }],
  hotkeys: [
    { key: 'd', display: 'dd', label: 'delete', action: 'DELETE' },
    { key: 'e', label: 'edit', action: 'EDIT' },
  ],
};

// Test namespace with action items (no fields = action-only, no CRUD)
const copyNamespace: Namespace = {
  id: 'copy',
  label: 'Copy',
  icon: 'copy',
  ItemComponent: MockItemComponent,
  fields: [],
  hotkeys: [],
  capabilities: { delete: false },
};

// Test namespace with single action item (should auto-execute)
const saveNamespace: Namespace = {
  id: 'save',
  label: 'Save',
  icon: 'save',
  ItemComponent: MockItemComponent,
  fields: [],
  hotkeys: [],
};

const singleActionItem: Item[] = [
  { id: 'save-to-file', name: 'Save to file', values: {}, action: { type: 'OPEN_SAVE_MODAL' as const } },
];

const regularItems: Item[] = [
  { id: 'tag-1', name: 'TODO', values: { name: 'TODO' } },
  { id: 'tag-2', name: 'FIXME', values: { name: 'FIXME' } },
];

const actionItems: Item[] = [
  { id: 'copy-content', name: 'Content', values: {}, action: { type: 'COPY_TO_CLIPBOARD', mode: 'content' } },
  { id: 'copy-annotations', name: 'Annotations', values: {}, action: { type: 'COPY_TO_CLIPBOARD', mode: 'annotations' } },
  { id: 'copy-both', name: 'Both', values: {}, action: { type: 'COPY_TO_CLIPBOARD', mode: 'all' } },
];

function createMockContext(namespaces: Namespace[], itemsMap: Record<string, Item[]>): QueryContext {
  return {
    namespaces,
    filterNamespaces(query: string): Namespace[] {
      if (!query) return namespaces;
      const q = query.toLowerCase();
      return namespaces.filter((ns) => ns.label.toLowerCase().includes(q));
    },
    getItems(namespace: Namespace): Item[] {
      return itemsMap[namespace.id] || [];
    },
    filterItems(namespace: Namespace, query: string): Item[] {
      const items = itemsMap[namespace.id] || [];
      if (!query) return items;
      const q = query.toLowerCase();
      return items.filter((item) => item.name.toLowerCase().includes(q));
    },
  };
}

describe('reducer: action items', () => {
  const ctx = createMockContext(
    [tagsNamespace, copyNamespace],
    { tags: regularItems, copy: actionItems }
  );

  describe('ENTER on action item', () => {
    it('executes action and returns to IDLE', () => {
      const state: State = {
        type: 'ITEM_FILTER',
        namespace: copyNamespace,
        query: '',
        selectedIndex: 0, // "Content" action item
        pendingDelete: false,
        inputMode: 'navigating',
      };

      const result = reduce(state, { type: 'ENTER' }, ctx);

      expect(result.state.type).toBe('IDLE');
      expect(result.commands).toHaveLength(1);
      expect(result.commands[0]).toEqual({
        type: 'COPY_TO_CLIPBOARD',
        mode: 'content',
      });
    });

    it('executes correct action based on selected index', () => {
      const state: State = {
        type: 'ITEM_FILTER',
        namespace: copyNamespace,
        query: '',
        selectedIndex: 2, // "Both" action item
        pendingDelete: false,
        inputMode: 'navigating',
      };

      const result = reduce(state, { type: 'ENTER' }, ctx);

      expect(result.commands[0]).toEqual({
        type: 'COPY_TO_CLIPBOARD',
        mode: 'all',
      });
    });
  });

  describe('ENTER on regular item', () => {
    it('opens edit form for regular items', () => {
      const state: State = {
        type: 'ITEM_FILTER',
        namespace: tagsNamespace,
        query: '',
        selectedIndex: 0, // "TODO" regular item
        pendingDelete: false,
        inputMode: 'navigating',
      };

      const result = reduce(state, { type: 'ENTER' }, ctx);

      expect(result.state.type).toBe('EDIT_FORM');
      expect(result.commands).toHaveLength(0);
    });
  });

  describe('DELETE on action item', () => {
    it('ignores delete for action items', () => {
      const state: State = {
        type: 'ITEM_FILTER',
        namespace: copyNamespace,
        query: '',
        selectedIndex: 0,
        pendingDelete: false,
        inputMode: 'navigating',
      };

      const result = reduce(state, { type: 'DELETE' }, ctx);

      // Should not arm pendingDelete
      expect(result.state).toEqual(state);
      expect(result.commands).toHaveLength(0);
    });
  });

  describe('EDIT on action item', () => {
    it('ignores edit for action items', () => {
      const state: State = {
        type: 'ITEM_FILTER',
        namespace: copyNamespace,
        query: '',
        selectedIndex: 0,
        pendingDelete: false,
        inputMode: 'navigating',
      };

      const result = reduce(state, { type: 'EDIT' }, ctx);

      // Should stay in same state
      expect(result.state).toEqual(state);
      expect(result.commands).toHaveLength(0);
    });
  });

  describe('DELETE on regular item', () => {
    it('arms pendingDelete on first delete', () => {
      const state: State = {
        type: 'ITEM_FILTER',
        namespace: tagsNamespace,
        query: '',
        selectedIndex: 0,
        pendingDelete: false,
        inputMode: 'navigating',
      };

      const result = reduce(state, { type: 'DELETE' }, ctx);

      expect(result.state.type).toBe('ITEM_FILTER');
      if (result.state.type === 'ITEM_FILTER') {
        expect(result.state.pendingDelete).toBe(true);
      }
    });
  });
});

describe('computeItemList', () => {
  const ctx = createMockContext(
    [tagsNamespace, copyNamespace],
    { tags: regularItems, copy: actionItems }
  );

  it('shows create option for namespaces with fields', () => {
    const state: State = {
      type: 'ITEM_FILTER',
      namespace: tagsNamespace,
      query: 'new',
      selectedIndex: 0,
      pendingDelete: false,
      inputMode: 'filtering',
    };

    const result = computeItemList(state, ctx);

    expect(result.showCreate).toBe(true);
  });

  it('hides create option for namespaces without fields', () => {
    const state: State = {
      type: 'ITEM_FILTER',
      namespace: copyNamespace,
      query: 'new',
      selectedIndex: 0,
      pendingDelete: false,
      inputMode: 'filtering',
    };

    const result = computeItemList(state, ctx);

    expect(result.showCreate).toBe(false);
  });
});

describe('reducer: obsidian namespace special handling', () => {
  // Obsidian namespace has fields (for vault CRUD) AND items with actions (for export)
  const obsidianNamespace: Namespace = {
    id: 'obsidian',
    label: 'Obsidian',
    icon: 'obsidian',
    ItemComponent: MockItemComponent,
    fields: [{ key: 'name', label: 'Vault Name', type: 'text', required: true }],
    hotkeys: [
      { key: 'd', display: 'dd', label: 'delete', action: 'DELETE' },
      { key: 'e', label: 'edit', action: 'EDIT' },
    ],
  };

  // Obsidian vault items have both values (for CRUD) and action (for export)
  const obsidianItems: Item[] = [
    {
      id: 'vault-1',
      name: 'Export to: Work Notes',
      values: { name: 'Work Notes' },
      action: { type: 'EXPORT_TO_OBSIDIAN', vault: 'Work Notes' },
    },
    {
      id: 'vault-2',
      name: 'Export to: Personal',
      values: { name: 'Personal' },
      action: { type: 'EXPORT_TO_OBSIDIAN', vault: 'Personal' },
    },
  ];

  const ctx = createMockContext(
    [obsidianNamespace, copyNamespace],
    { obsidian: obsidianItems, copy: actionItems }
  );

  describe('DELETE on obsidian item with action', () => {
    it('arms pendingDelete on first delete (unlike other action items)', () => {
      const state: State = {
        type: 'ITEM_FILTER',
        namespace: obsidianNamespace,
        query: '',
        selectedIndex: 0,
        pendingDelete: false,
        inputMode: 'navigating',
      };

      const result = reduce(state, { type: 'DELETE' }, ctx);

      expect(result.state.type).toBe('ITEM_FILTER');
      if (result.state.type === 'ITEM_FILTER') {
        expect(result.state.pendingDelete).toBe(true);
      }
    });

    it('emits DELETE_ITEM command on second delete', () => {
      const state: State = {
        type: 'ITEM_FILTER',
        namespace: obsidianNamespace,
        query: '',
        selectedIndex: 0,
        pendingDelete: true, // Already armed
        inputMode: 'navigating',
      };

      const result = reduce(state, { type: 'DELETE' }, ctx);

      expect(result.commands).toContainEqual({
        type: 'DELETE_ITEM',
        namespace: 'obsidian',
        itemId: 'vault-1',
      });
    });
  });

  describe('EDIT on obsidian item with action', () => {
    it('opens edit form (unlike other action items)', () => {
      const state: State = {
        type: 'ITEM_FILTER',
        namespace: obsidianNamespace,
        query: '',
        selectedIndex: 0,
        pendingDelete: false,
        inputMode: 'navigating',
      };

      const result = reduce(state, { type: 'EDIT' }, ctx);

      expect(result.state.type).toBe('EDIT_FORM');
      if (result.state.type === 'EDIT_FORM') {
        expect(result.state.item.id).toBe('vault-1');
        expect(result.state.values.name).toBe('Work Notes');
      }
    });
  });

  describe('ENTER on obsidian item with action', () => {
    it('executes the export action and closes', () => {
      const state: State = {
        type: 'ITEM_FILTER',
        namespace: obsidianNamespace,
        query: '',
        selectedIndex: 0,
        pendingDelete: false,
        inputMode: 'navigating',
      };

      const result = reduce(state, { type: 'ENTER' }, ctx);

      expect(result.state.type).toBe('IDLE');
      expect(result.commands).toContainEqual({
        type: 'EXPORT_TO_OBSIDIAN',
        vault: 'Work Notes',
      });
    });
  });

  describe('DELETE on copy namespace (non-obsidian) with action', () => {
    it('still blocks delete for action items', () => {
      const state: State = {
        type: 'ITEM_FILTER',
        namespace: copyNamespace,
        query: '',
        selectedIndex: 0,
        pendingDelete: false,
        inputMode: 'navigating',
      };

      const result = reduce(state, { type: 'DELETE' }, ctx);

      // Should not arm pendingDelete
      expect(result.state).toEqual(state);
      expect(result.commands).toHaveLength(0);
    });
  });
});

describe('reducer: arrow navigation cycling', () => {
  const ctx = createMockContext(
    [tagsNamespace, copyNamespace, saveNamespace],
    { tags: regularItems, copy: actionItems, save: singleActionItem }
  );

  describe('NAMESPACE_FILTER cycling', () => {
    it('arrow down from filtering goes to first item', () => {
      const state: State = {
        type: 'NAMESPACE_FILTER',
        query: '',
        selectedIndex: 0,
        inputMode: 'filtering',
      };

      const result = reduce(state, { type: 'ARROW_DOWN' }, ctx);

      expect(result.state.type).toBe('NAMESPACE_FILTER');
      if (result.state.type === 'NAMESPACE_FILTER') {
        expect(result.state.selectedIndex).toBe(0);
        expect(result.state.inputMode).toBe('navigating');
      }
    });

    it('arrow up from filtering goes to last item', () => {
      const state: State = {
        type: 'NAMESPACE_FILTER',
        query: '',
        selectedIndex: 0,
        inputMode: 'filtering',
      };

      const result = reduce(state, { type: 'ARROW_UP' }, ctx);

      expect(result.state.type).toBe('NAMESPACE_FILTER');
      if (result.state.type === 'NAMESPACE_FILTER') {
        expect(result.state.selectedIndex).toBe(2); // Last namespace (save)
        expect(result.state.inputMode).toBe('navigating');
      }
    });

    it('arrow up at first item returns to filtering', () => {
      const state: State = {
        type: 'NAMESPACE_FILTER',
        query: '',
        selectedIndex: 0,
        inputMode: 'navigating',
      };

      const result = reduce(state, { type: 'ARROW_UP' }, ctx);

      expect(result.state.type).toBe('NAMESPACE_FILTER');
      if (result.state.type === 'NAMESPACE_FILTER') {
        expect(result.state.inputMode).toBe('filtering');
      }
    });

    it('arrow down at last item returns to filtering', () => {
      const state: State = {
        type: 'NAMESPACE_FILTER',
        query: '',
        selectedIndex: 2, // Last item
        inputMode: 'navigating',
      };

      const result = reduce(state, { type: 'ARROW_DOWN' }, ctx);

      expect(result.state.type).toBe('NAMESPACE_FILTER');
      if (result.state.type === 'NAMESPACE_FILTER') {
        expect(result.state.inputMode).toBe('filtering');
      }
    });

    it('arrow down in middle moves to next item', () => {
      const state: State = {
        type: 'NAMESPACE_FILTER',
        query: '',
        selectedIndex: 1,
        inputMode: 'navigating',
      };

      const result = reduce(state, { type: 'ARROW_DOWN' }, ctx);

      expect(result.state.type).toBe('NAMESPACE_FILTER');
      if (result.state.type === 'NAMESPACE_FILTER') {
        expect(result.state.selectedIndex).toBe(2);
        expect(result.state.inputMode).toBe('navigating');
      }
    });

    it('arrow up in middle moves to previous item', () => {
      const state: State = {
        type: 'NAMESPACE_FILTER',
        query: '',
        selectedIndex: 1,
        inputMode: 'navigating',
      };

      const result = reduce(state, { type: 'ARROW_UP' }, ctx);

      expect(result.state.type).toBe('NAMESPACE_FILTER');
      if (result.state.type === 'NAMESPACE_FILTER') {
        expect(result.state.selectedIndex).toBe(0);
        expect(result.state.inputMode).toBe('navigating');
      }
    });
  });

  describe('ITEM_FILTER cycling', () => {
    it('arrow down from filtering goes to first item', () => {
      const state: State = {
        type: 'ITEM_FILTER',
        namespace: copyNamespace,
        query: '',
        selectedIndex: 0,
        pendingDelete: false,
        inputMode: 'filtering',
      };

      const result = reduce(state, { type: 'ARROW_DOWN' }, ctx);

      expect(result.state.type).toBe('ITEM_FILTER');
      if (result.state.type === 'ITEM_FILTER') {
        expect(result.state.selectedIndex).toBe(0);
        expect(result.state.inputMode).toBe('navigating');
      }
    });

    it('arrow up from filtering goes to last item', () => {
      const state: State = {
        type: 'ITEM_FILTER',
        namespace: copyNamespace,
        query: '',
        selectedIndex: 0,
        pendingDelete: false,
        inputMode: 'filtering',
      };

      const result = reduce(state, { type: 'ARROW_UP' }, ctx);

      expect(result.state.type).toBe('ITEM_FILTER');
      if (result.state.type === 'ITEM_FILTER') {
        expect(result.state.selectedIndex).toBe(2); // Last item (3 items: 0, 1, 2)
        expect(result.state.inputMode).toBe('navigating');
      }
    });

    it('arrow up at first item returns to filtering', () => {
      const state: State = {
        type: 'ITEM_FILTER',
        namespace: copyNamespace,
        query: '',
        selectedIndex: 0,
        pendingDelete: false,
        inputMode: 'navigating',
      };

      const result = reduce(state, { type: 'ARROW_UP' }, ctx);

      expect(result.state.type).toBe('ITEM_FILTER');
      if (result.state.type === 'ITEM_FILTER') {
        expect(result.state.inputMode).toBe('filtering');
      }
    });

    it('arrow down at last item returns to filtering', () => {
      const state: State = {
        type: 'ITEM_FILTER',
        namespace: copyNamespace,
        query: '',
        selectedIndex: 2, // Last item
        pendingDelete: false,
        inputMode: 'navigating',
      };

      const result = reduce(state, { type: 'ARROW_DOWN' }, ctx);

      expect(result.state.type).toBe('ITEM_FILTER');
      if (result.state.type === 'ITEM_FILTER') {
        expect(result.state.inputMode).toBe('filtering');
      }
    });

    it('arrow navigation clears pendingDelete', () => {
      const state: State = {
        type: 'ITEM_FILTER',
        namespace: tagsNamespace,
        query: '',
        selectedIndex: 0,
        pendingDelete: true,
        inputMode: 'navigating',
      };

      const result = reduce(state, { type: 'ARROW_DOWN' }, ctx);

      expect(result.state.type).toBe('ITEM_FILTER');
      if (result.state.type === 'ITEM_FILTER') {
        expect(result.state.pendingDelete).toBe(false);
      }
    });
  });
});

describe('reducer: single-action namespace auto-execute', () => {
  // Context with save namespace (single action item, no fields)
  const ctx = createMockContext(
    [tagsNamespace, copyNamespace, saveNamespace],
    { tags: regularItems, copy: actionItems, save: singleActionItem }
  );

  // Also test context where tags has just 1 item (should NOT auto-execute)
  const singleTagItems: Item[] = [{ id: 'tag-1', name: 'TODO', values: { name: 'TODO' } }];
  const ctxSingleTag = createMockContext(
    [tagsNamespace, saveNamespace],
    { tags: singleTagItems, save: singleActionItem }
  );

  describe('ENTER on single-action namespace', () => {
    it('auto-executes action and returns to IDLE', () => {
      const state: State = {
        type: 'NAMESPACE_FILTER',
        query: '',
        selectedIndex: 2, // saveNamespace
        inputMode: 'navigating',
      };

      const result = reduce(state, { type: 'ENTER' }, ctx);

      expect(result.state.type).toBe('IDLE');
      expect(result.commands).toHaveLength(1);
      expect(result.commands[0]).toEqual({ type: 'OPEN_SAVE_MODAL' });
    });
  });

  describe('SELECT on single-action namespace', () => {
    it('auto-executes action and returns to IDLE', () => {
      const state: State = {
        type: 'NAMESPACE_FILTER',
        query: '',
        selectedIndex: 0,
        inputMode: 'navigating',
      };

      const result = reduce(state, { type: 'SELECT', index: 2 }, ctx); // saveNamespace

      expect(result.state.type).toBe('IDLE');
      expect(result.commands).toHaveLength(1);
      expect(result.commands[0]).toEqual({ type: 'OPEN_SAVE_MODAL' });
    });
  });

  describe('ENTER on multi-action namespace', () => {
    it('transitions to ITEM_FILTER (does not auto-execute)', () => {
      const state: State = {
        type: 'NAMESPACE_FILTER',
        query: '',
        selectedIndex: 1, // copyNamespace (3 items)
        inputMode: 'navigating',
      };

      const result = reduce(state, { type: 'ENTER' }, ctx);

      expect(result.state.type).toBe('ITEM_FILTER');
      // Should not have executed any action
      expect(result.commands.find(c => c.type === 'COPY_TO_CLIPBOARD')).toBeUndefined();
    });
  });

  describe('ENTER on editable namespace with single item', () => {
    it('transitions to ITEM_FILTER (does not auto-execute)', () => {
      const state: State = {
        type: 'NAMESPACE_FILTER',
        query: '',
        selectedIndex: 0, // tagsNamespace (has fields, so editable)
        inputMode: 'navigating',
      };

      const result = reduce(state, { type: 'ENTER' }, ctxSingleTag);

      // Should go to ITEM_FILTER, not auto-execute
      expect(result.state.type).toBe('ITEM_FILTER');
      expect(result.commands.find(c => c.type === 'OPEN_SAVE_MODAL')).toBeUndefined();
    });
  });
});

describe('reducer: initial state', () => {
  const ctx = createMockContext(
    [tagsNamespace, copyNamespace],
    { tags: regularItems, copy: actionItems }
  );

  describe('OPEN with edit initialState', () => {
    it('opens directly in EDIT_FORM for valid item', () => {
      const state: State = { type: 'IDLE' };
      const result = reduce(
        state,
        { type: 'OPEN', initialState: { namespace: 'tags', mode: 'edit', itemId: 'tag-1' } },
        ctx
      );

      expect(result.state.type).toBe('EDIT_FORM');
      if (result.state.type === 'EDIT_FORM') {
        expect(result.state.item.id).toBe('tag-1');
        expect(result.state.namespace.id).toBe('tags');
        expect(result.state.focusedField).toBe(0);
      }
    });

    it('falls back to NAMESPACE_FILTER if item not found', () => {
      const state: State = { type: 'IDLE' };
      const result = reduce(
        state,
        { type: 'OPEN', initialState: { namespace: 'tags', mode: 'edit', itemId: 'nonexistent' } },
        ctx
      );

      expect(result.state.type).toBe('NAMESPACE_FILTER');
    });

    it('falls back to NAMESPACE_FILTER if namespace not found', () => {
      const state: State = { type: 'IDLE' };
      const result = reduce(
        state,
        { type: 'OPEN', initialState: { namespace: 'bookmarks', mode: 'edit', itemId: 'tag-1' } },
        ctx
      );

      expect(result.state.type).toBe('NAMESPACE_FILTER');
    });

    it('falls back to NAMESPACE_FILTER if item is not editable', () => {
      const ctxWithReadonly = createMockContext(
        [tagsNamespace],
        { tags: [{ id: 'tag-1', name: 'TODO', values: { name: 'TODO' }, readonly: true }] }
      );
      const state: State = { type: 'IDLE' };
      const result = reduce(
        state,
        { type: 'OPEN', initialState: { namespace: 'tags', mode: 'edit', itemId: 'tag-1' } },
        ctxWithReadonly
      );

      expect(result.state.type).toBe('NAMESPACE_FILTER');
    });
  });
});
