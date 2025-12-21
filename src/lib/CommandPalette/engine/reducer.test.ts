import { describe, it, expect } from 'vitest';
import { reduce, computeItemList } from './reducer';
import type { State, Action, QueryContext, Namespace, Item, Command } from './types';

// Test namespace with regular items
const tagsNamespace: Namespace = {
  id: 'tags',
  label: 'Tags',
  icon: 'hashtag',
  fields: [{ key: 'name', label: 'Name', type: 'text', required: true }],
  hotkeys: [
    { key: 'd', display: 'dd', label: 'delete', action: 'DELETE' },
    { key: 'e', label: 'edit', action: 'EDIT' },
  ],
};

// Test namespace with action items (no fields = action-only)
const copyNamespace: Namespace = {
  id: 'copy',
  label: 'Copy',
  icon: 'copy',
  fields: [],
  hotkeys: [],
};

// Test namespace with single action item (should auto-execute)
const saveNamespace: Namespace = {
  id: 'save',
  label: 'Save',
  icon: 'save',
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
