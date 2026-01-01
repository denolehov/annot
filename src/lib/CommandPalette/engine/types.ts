// CommandPalette engine types
// Pure domain types with no DOM dependencies

import type { Component } from 'svelte';

// === Item Component Props ===

/**
 * Visual state for an item in the command palette.
 * - idle: Not selected
 * - preselected: Selected while filtering (user typing, outline style)
 * - selected: Selected while navigating (arrow keys, solid style)
 * - pending-delete: Awaiting delete confirmation (dd)
 */
export type ItemSelectionState = 'idle' | 'preselected' | 'selected' | 'pending-delete';

export interface ItemComponentProps {
  item: Item;
  selectionState: ItemSelectionState;
}

// === Domain ===

/**
 * Namespace-specific hotkey (only active in ITEM_FILTER navigating mode)
 */
export interface Hotkey {
  key: string;              // keyboard key to trigger (e.g., 'd')
  display?: string;         // what to show in footer (e.g., 'dd'), defaults to key
  label: string;            // footer hint label (e.g., 'delete')
  action: Action['type'];   // action to dispatch (e.g., 'DELETE')
}

/** CRUD capabilities for a namespace. Unset values use sensible defaults. */
export interface Capabilities {
  /** Show "Create X" option. Default: fields.length > 0 */
  create?: boolean;
  /** Allow editing items via form. Default: fields.length > 0 */
  update?: boolean;
  /** Allow deleting items. Default: true */
  delete?: boolean;
  /** Allow reordering items. Default: false */
  reorder?: boolean;
}

export interface Namespace {
  id: string;
  label: string;
  icon: string;
  /** Component used to render items in this namespace */
  ItemComponent: Component<ItemComponentProps>;
  fields: Field[];
  hotkeys?: Hotkey[];
  /** Example values shown as placeholders in CREATE_FORM (random one picked per form open) */
  examples?: Array<Record<string, string>>;
  /** Explicit CRUD capabilities. Unset values derived from fields presence. */
  capabilities?: Capabilities;
}

// === Capability Resolvers ===

export function canCreate(ns: Namespace): boolean {
  return ns.capabilities?.create ?? ns.fields.length > 0;
}

export function canUpdate(ns: Namespace): boolean {
  return ns.capabilities?.update ?? ns.fields.length > 0;
}

export function canDelete(ns: Namespace): boolean {
  return ns.capabilities?.delete ?? true;
}

export function canReorder(ns: Namespace): boolean {
  return ns.capabilities?.reorder ?? false;
}

/** Check if a specific item can be modified (edited/deleted) */
export function isItemEditable(item: Item): boolean {
  return !item.readonly && !item.isEphemeral;
}

/** Get placeholder text for item filter input */
export function getFilterPlaceholder(ns: Namespace, totalItems: number): string {
  if (!canCreate(ns)) {
    return 'Filter...';
  }
  return totalItems === 0 ? 'Type to create...' : 'Filter or create...';
}

export type Field =
  | { key: string; label: string; type: 'text'; placeholder?: string; required?: boolean; readOnlyInEdit?: boolean }
  | { key: string; label: string; type: 'textarea'; placeholder?: string; required?: boolean; readOnlyInEdit?: boolean }
  | { key: string; label: string; type: 'select'; options: string[]; required?: boolean; readOnlyInEdit?: boolean };

export interface Item {
  id: string;
  name: string;
  values: Record<string, string>;
  isEphemeral?: boolean; // True if injected by agent (session-scoped, not editable)
  action?: Command; // If present: execute on ENTER instead of edit form
  readonly?: boolean; // Blocks CRUD operations (delete, edit) for this specific item
}

// Pending item — being created, no ID yet
export interface PendingItem {
  name: string;
  values: Record<string, string>;
}

// === State ===

export type InputMode = 'filtering' | 'navigating';

export type State =
  | { type: 'IDLE' }
  | { type: 'NAMESPACE_FILTER'; query: string; selectedIndex: number; inputMode: InputMode }
  | { type: 'ITEM_FILTER'; namespace: Namespace; query: string; selectedIndex: number; pendingDelete: boolean; inputMode: InputMode }
  | {
      type: 'EDIT_FORM';
      namespace: Namespace;
      item: Item;
      values: Record<string, string>;
      focusedField: number;
      closeOnSave?: boolean; // If true, close CP after save instead of returning to ITEM_FILTER
    }
  | {
      type: 'CREATE_FORM';
      namespace: Namespace;
      values: Record<string, string>;
      focusedField: number;
      closeOnSave?: boolean; // If true, close CP after save instead of returning to ITEM_FILTER
    }
  | {
      type: 'ITEM_REORDER';
      namespace: Namespace;
      items: Item[]; // Mutable copy of items being reordered
      selectedIndex: number;
    };

// === Actions ===

export interface InitialState {
  namespace: 'tags' | 'exit-modes' | 'bookmarks';
  mode: 'create' | 'edit' | 'filter';
  itemId?: string; // For edit mode - which item to edit
  prefill?: Record<string, string>;
}

export type Action =
  | { type: 'OPEN'; initialState?: InitialState }
  | { type: 'CLOSE' }
  | { type: 'INPUT'; char: string }
  | { type: 'SET_FIELD'; key: string; value: string }
  | { type: 'BACKSPACE' }
  | { type: 'ARROW_UP' }
  | { type: 'ARROW_DOWN' }
  | { type: 'TAB' }
  | { type: 'ENTER'; formValues?: Record<string, string> }
  | { type: 'ESCAPE' }
  | { type: 'DELETE' }
  | { type: 'EDIT' }
  | { type: 'SET' }
  | { type: 'REORDER' } // Enter reorder mode
  | { type: 'MOVE_UP' } // Move focused item up (in reorder mode)
  | { type: 'MOVE_DOWN' } // Move focused item down (in reorder mode)
  | { type: 'SELECT'; index: number }; // Click to select and activate

// === Commands ===

export type Command =
  | { type: 'CREATE_ITEM'; namespace: string; pending: PendingItem }
  | { type: 'UPDATE_ITEM'; namespace: string; item: Item }
  | { type: 'DELETE_ITEM'; namespace: string; itemId: string }
  | { type: 'SET_MODE'; namespace: string; itemId: string }
  | { type: 'REORDER_ITEMS'; namespace: string; orderedIds: string[] }
  | { type: 'EMIT_EVENT'; event: string; payload: unknown }
  | { type: 'COPY_TO_CLIPBOARD'; mode: 'content' | 'annotations' | 'all' }
  | { type: 'OPEN_SAVE_MODAL' }
  | { type: 'EXPORT_TO_OBSIDIAN'; vault: string };

// === Query Context ===

export interface QueryContext {
  namespaces: Namespace[];
  filterNamespaces(query: string): Namespace[];
  getItems(namespace: Namespace): Item[];
  filterItems(namespace: Namespace, query: string): Item[];
}

// === Reducer Result ===

export interface ReduceResult {
  state: State;
  commands: Command[];
}
