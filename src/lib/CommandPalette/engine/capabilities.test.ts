import { describe, it, expect } from 'vitest';
import {
  canCreate,
  canUpdate,
  canDelete,
  canReorder,
  isItemEditable,
  getFilterPlaceholder,
  type Namespace,
  type Item,
} from './types';

// === Test Namespaces ===

const crudNamespace: Namespace = {
  id: 'tags',
  label: 'Tags',
  icon: 'hashtag',
  fields: [{ key: 'name', label: 'Name', type: 'text' }],
};

const actionOnlyNamespace: Namespace = {
  id: 'theme',
  label: 'Theme',
  icon: 'sun',
  fields: [],
};

const noCreateNamespace: Namespace = {
  id: 'bookmarks',
  label: 'Bookmarks',
  icon: 'bookmark',
  fields: [{ key: 'label', label: 'Label', type: 'text' }],
  capabilities: { create: false },
};

const reorderableNamespace: Namespace = {
  id: 'exit-modes',
  label: 'Exit Modes',
  icon: 'exit',
  fields: [{ key: 'name', label: 'Name', type: 'text' }],
  capabilities: { reorder: true },
};

const fullCapabilitiesNamespace: Namespace = {
  id: 'custom',
  label: 'Custom',
  icon: 'star',
  fields: [],
  capabilities: { create: true, update: true, delete: false, reorder: true },
};

// === canCreate ===

describe('canCreate', () => {
  it('returns true for namespace with fields (default)', () => {
    expect(canCreate(crudNamespace)).toBe(true);
  });

  it('returns false for namespace without fields (default)', () => {
    expect(canCreate(actionOnlyNamespace)).toBe(false);
  });

  it('returns false when explicitly disabled', () => {
    expect(canCreate(noCreateNamespace)).toBe(false);
  });

  it('returns true when explicitly enabled even without fields', () => {
    expect(canCreate(fullCapabilitiesNamespace)).toBe(true);
  });
});

// === canUpdate ===

describe('canUpdate', () => {
  it('returns true for namespace with fields (default)', () => {
    expect(canUpdate(crudNamespace)).toBe(true);
  });

  it('returns false for namespace without fields (default)', () => {
    expect(canUpdate(actionOnlyNamespace)).toBe(false);
  });

  it('returns true when explicitly enabled', () => {
    expect(canUpdate(fullCapabilitiesNamespace)).toBe(true);
  });
});

// === canDelete ===

describe('canDelete', () => {
  it('returns true by default', () => {
    expect(canDelete(crudNamespace)).toBe(true);
    expect(canDelete(actionOnlyNamespace)).toBe(true);
  });

  it('returns false when explicitly disabled', () => {
    expect(canDelete(fullCapabilitiesNamespace)).toBe(false);
  });
});

// === canReorder ===

describe('canReorder', () => {
  it('returns false by default', () => {
    expect(canReorder(crudNamespace)).toBe(false);
    expect(canReorder(actionOnlyNamespace)).toBe(false);
  });

  it('returns true when explicitly enabled', () => {
    expect(canReorder(reorderableNamespace)).toBe(true);
    expect(canReorder(fullCapabilitiesNamespace)).toBe(true);
  });
});

// === isItemEditable ===

describe('isItemEditable', () => {
  it('returns true for regular items', () => {
    const item: Item = { id: '1', name: 'Test', values: {} };
    expect(isItemEditable(item)).toBe(true);
  });

  it('returns false for readonly items', () => {
    const item: Item = { id: '1', name: 'Test', values: {}, readonly: true };
    expect(isItemEditable(item)).toBe(false);
  });

  it('returns false for ephemeral items', () => {
    const item: Item = { id: '1', name: 'Test', values: {}, isEphemeral: true };
    expect(isItemEditable(item)).toBe(false);
  });

  it('returns false when both readonly and ephemeral', () => {
    const item: Item = { id: '1', name: 'Test', values: {}, readonly: true, isEphemeral: true };
    expect(isItemEditable(item)).toBe(false);
  });

  it('returns true for items with actions (action is orthogonal)', () => {
    const item: Item = {
      id: '1',
      name: 'Test',
      values: {},
      action: { type: 'EMIT_EVENT', event: 'test', payload: null },
    };
    expect(isItemEditable(item)).toBe(true);
  });
});

// === getFilterPlaceholder ===

describe('getFilterPlaceholder', () => {
  it('returns "Filter..." for namespace that cannot create', () => {
    expect(getFilterPlaceholder(actionOnlyNamespace, 0)).toBe('Filter...');
    expect(getFilterPlaceholder(actionOnlyNamespace, 5)).toBe('Filter...');
    expect(getFilterPlaceholder(noCreateNamespace, 0)).toBe('Filter...');
  });

  it('returns "Type to create..." when can create and no items', () => {
    expect(getFilterPlaceholder(crudNamespace, 0)).toBe('Type to create...');
  });

  it('returns "Filter or create..." when can create and has items', () => {
    expect(getFilterPlaceholder(crudNamespace, 1)).toBe('Filter or create...');
    expect(getFilterPlaceholder(crudNamespace, 100)).toBe('Filter or create...');
  });
});
