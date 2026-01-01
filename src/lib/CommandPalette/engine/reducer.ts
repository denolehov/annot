// Pure reducer for CommandPalette state machine
// No side effects, no DOM dependencies

import type { State, Action, Command, QueryContext, ReduceResult, Namespace, Item, PendingItem } from './types';
import { canCreate, canUpdate, canDelete, canReorder, isItemEditable } from './types';

/**
 * Compute the item list for ITEM_FILTER state
 * Returns matches, whether to show Create option, and Create's index
 */
export function computeItemList(
  state: { namespace: Namespace; query: string },
  ctx: QueryContext
): { matches: Item[]; showCreate: boolean; createIndex: number } {
  const matches = ctx.filterItems(state.namespace, state.query);
  const queryTrimmed = state.query.trim();
  const hasExactMatch = matches.some(
    (m) => m.name.toLowerCase() === queryTrimmed.toLowerCase()
  );
  // Show Create only if: namespace allows it, query is non-empty, and no exact match
  const showCreate = canCreate(state.namespace) && queryTrimmed !== '' && !hasExactMatch;
  const createIndex = showCreate ? matches.length : -1;

  return { matches, showCreate, createIndex };
}

/**
 * Pure reducer function
 * Takes current state and action, returns new state and commands to execute
 */
export function reduce(state: State, action: Action, ctx: QueryContext): ReduceResult {
  const commands: Command[] = [];

  switch (state.type) {
    case 'IDLE': {
      if (action.type === 'OPEN') {
        commands.push({ type: 'EMIT_EVENT', event: 'commandpalette:open', payload: undefined });

        // If initialState provided, jump directly to appropriate form
        if (action.initialState?.mode === 'create') {
          const namespace = ctx.namespaces.find((ns) => ns.id === action.initialState!.namespace);
          if (namespace) {
            return {
              state: {
                type: 'CREATE_FORM',
                namespace,
                values: action.initialState.prefill ?? {},
                focusedField: 0,
                closeOnSave: true, // Close CP after save when opened from editor
              },
              commands,
            };
          }
        }

        // If initialState is edit mode, jump directly to EDIT_FORM
        if (action.initialState?.mode === 'edit' && action.initialState.itemId) {
          const namespace = ctx.namespaces.find((ns) => ns.id === action.initialState!.namespace);
          if (namespace) {
            const items = ctx.getItems(namespace);
            const item = items.find((i) => i.id === action.initialState!.itemId);
            if (item && isItemEditable(item)) {
              return {
                state: {
                  type: 'EDIT_FORM',
                  namespace,
                  item,
                  values: { ...item.values },
                  focusedField: 0,
                },
                commands,
              };
            }
          }
        }

        return {
          state: { type: 'NAMESPACE_FILTER', query: '', selectedIndex: 0, inputMode: 'filtering' },
          commands,
        };
      }
      return { state, commands };
    }

    case 'NAMESPACE_FILTER': {
      if (action.type === 'ESCAPE' || action.type === 'CLOSE') {
        commands.push({ type: 'EMIT_EVENT', event: 'commandpalette:close', payload: undefined });
        return { state: { type: 'IDLE' }, commands };
      }

      if (action.type === 'INPUT') {
        return {
          state: { ...state, query: state.query + action.char, selectedIndex: 0, inputMode: 'filtering' },
          commands,
        };
      }

      if (action.type === 'BACKSPACE') {
        return {
          state: { ...state, query: state.query.slice(0, -1), selectedIndex: 0, inputMode: 'filtering' },
          commands,
        };
      }

      if (action.type === 'ARROW_DOWN' || action.type === 'ARROW_UP') {
        const matches = ctx.filterNamespaces(state.query);
        const maxIndex = Math.max(0, matches.length - 1);

        // Cycle: filtering ↔ nav[0] ... nav[n] ↔ filtering
        if (state.inputMode === 'filtering') {
          if (action.type === 'ARROW_DOWN') {
            return { state: { ...state, selectedIndex: 0, inputMode: 'navigating' }, commands };
          }
          // Arrow up from filtering: go to last item
          return { state: { ...state, selectedIndex: maxIndex, inputMode: 'navigating' }, commands };
        }

        // Arrow up at first item: return to filtering
        if (action.type === 'ARROW_UP' && state.selectedIndex === 0) {
          return { state: { ...state, inputMode: 'filtering' }, commands };
        }

        // Arrow down at last item: return to filtering
        if (action.type === 'ARROW_DOWN' && state.selectedIndex === maxIndex) {
          return { state: { ...state, inputMode: 'filtering' }, commands };
        }

        const delta = action.type === 'ARROW_DOWN' ? 1 : -1;
        const nextIndex = state.selectedIndex + delta;

        return { state: { ...state, selectedIndex: nextIndex, inputMode: 'navigating' }, commands };
      }

      if (action.type === 'ENTER') {
        const matches = ctx.filterNamespaces(state.query);
        const ns = matches[state.selectedIndex];
        if (!ns) {
          return { state, commands };
        }
        // Check for action-only namespace with single item (e.g., Save)
        // Action namespaces have no fields (not editable)
        const items = ctx.getItems(ns);
        if (ns.fields.length === 0 && items.length === 1 && items[0].action) {
          // Execute immediately and close
          return { state: { type: 'IDLE' }, commands: [items[0].action] };
        }
        commands.push({
          type: 'EMIT_EVENT',
          event: 'commandpalette:namespace-locked',
          payload: { namespace: ns.id },
        });
        return {
          state: { type: 'ITEM_FILTER', namespace: ns, query: '', selectedIndex: 0, pendingDelete: false, inputMode: 'filtering' },
          commands,
        };
      }

      // SELECT: click to select and activate (behaves like setting index then ENTER)
      if (action.type === 'SELECT') {
        const matches = ctx.filterNamespaces(state.query);
        const ns = matches[action.index];
        if (!ns) {
          return { state, commands };
        }
        // Check for action-only namespace with single item (e.g., Save)
        // Action namespaces have no fields (not editable)
        const items = ctx.getItems(ns);
        if (ns.fields.length === 0 && items.length === 1 && items[0].action) {
          // Execute immediately and close
          return { state: { type: 'IDLE' }, commands: [items[0].action] };
        }
        commands.push({
          type: 'EMIT_EVENT',
          event: 'commandpalette:namespace-locked',
          payload: { namespace: ns.id },
        });
        return {
          state: { type: 'ITEM_FILTER', namespace: ns, query: '', selectedIndex: 0, pendingDelete: false, inputMode: 'filtering' },
          commands,
        };
      }

      return { state, commands };
    }

    case 'ITEM_FILTER': {
      // Helper to clear pendingDelete when returning to this state
      const clearPending = (s: typeof state) => ({ ...s, pendingDelete: false });
      // ESCAPE: if pendingDelete is armed, just disarm it; otherwise go back to NAMESPACE_FILTER
      if (action.type === 'ESCAPE') {
        if (state.pendingDelete) {
          return { state: clearPending(state), commands };
        }
        return {
          state: { type: 'NAMESPACE_FILTER', query: '', selectedIndex: 0, inputMode: 'filtering' },
          commands,
        };
      }

      // CLOSE (backdrop click) closes completely
      if (action.type === 'CLOSE') {
        commands.push({ type: 'EMIT_EVENT', event: 'commandpalette:close', payload: undefined });
        return { state: { type: 'IDLE' }, commands };
      }

      if (action.type === 'BACKSPACE') {
        if (state.query === '') {
          // Go back to namespace filter
          return {
            state: { type: 'NAMESPACE_FILTER', query: '', selectedIndex: 0, inputMode: 'filtering' },
            commands,
          };
        }
        // Backspace switches to filtering mode (user is editing the filter)
        return {
          state: clearPending({ ...state, query: state.query.slice(0, -1), selectedIndex: 0, inputMode: 'filtering' }),
          commands,
        };
      }

      if (action.type === 'INPUT') {
        // Typing switches to filtering mode
        return {
          state: clearPending({ ...state, query: state.query + action.char, selectedIndex: 0, inputMode: 'filtering' }),
          commands,
        };
      }

      if (action.type === 'ARROW_DOWN' || action.type === 'ARROW_UP') {
        const { matches, showCreate } = computeItemList(state, ctx);
        const totalItems = matches.length + (showCreate ? 1 : 0);
        const maxIndex = Math.max(0, totalItems - 1);

        // Cycle: filtering ↔ nav[0] ... nav[n] ↔ filtering
        if (state.inputMode === 'filtering') {
          if (action.type === 'ARROW_DOWN') {
            return { state: clearPending({ ...state, selectedIndex: 0, inputMode: 'navigating' }), commands };
          }
          // Arrow up from filtering: go to last item
          return { state: clearPending({ ...state, selectedIndex: maxIndex, inputMode: 'navigating' }), commands };
        }

        // Arrow up at first item: return to filtering
        if (action.type === 'ARROW_UP' && state.selectedIndex === 0) {
          return { state: clearPending({ ...state, inputMode: 'filtering' }), commands };
        }

        // Arrow down at last item: return to filtering
        if (action.type === 'ARROW_DOWN' && state.selectedIndex === maxIndex) {
          return { state: clearPending({ ...state, inputMode: 'filtering' }), commands };
        }

        const delta = action.type === 'ARROW_DOWN' ? 1 : -1;
        const nextIndex = state.selectedIndex + delta;

        // Arrow navigation in navigating mode
        return { state: clearPending({ ...state, selectedIndex: nextIndex, inputMode: 'navigating' }), commands };
      }

      if (action.type === 'ENTER') {
        const { matches, showCreate, createIndex } = computeItemList(state, ctx);

        // Selected Create option
        if (showCreate && state.selectedIndex === createIndex) {
          return {
            state: {
              type: 'CREATE_FORM',
              namespace: state.namespace,
              values: { name: state.query },
              focusedField: 1, // Skip name since it's pre-filled
            },
            commands,
          };
        }

        // Selected an existing item
        const item = matches[state.selectedIndex];
        if (item) {
          // Executable item — run action, close palette
          if (item.action) {
            commands.push(item.action);
            return {
              state: { type: 'IDLE' },
              commands,
            };
          }

          // Regular item — open edit form
          return {
            state: {
              type: 'EDIT_FORM',
              namespace: state.namespace,
              item,
              values: { ...item.values },
              focusedField: 0,
            },
            commands,
          };
        }

        return { state, commands };
      }

      // DELETE only works in navigating mode
      if (action.type === 'DELETE' && state.inputMode === 'navigating') {
        // Check namespace capability
        if (!canDelete(state.namespace)) {
          return { state, commands };
        }

        const { matches, showCreate, createIndex } = computeItemList(state, ctx);

        // Don't delete if on Create option
        if (showCreate && state.selectedIndex === createIndex) {
          return { state, commands };
        }

        // Check if item can be deleted
        const selectedItem = matches[state.selectedIndex];
        if (!selectedItem || !isItemEditable(selectedItem)) {
          return { state, commands };
        }

        // Vim-style dd: first d sets pendingDelete, second d confirms
        if (!state.pendingDelete) {
          // First d - arm the delete
          return { state: { ...state, pendingDelete: true }, commands };
        }

        // Second d - actually delete
        commands.push({
          type: 'DELETE_ITEM',
          namespace: state.namespace.id,
          itemId: selectedItem.id,
        });
        commands.push({
          type: 'EMIT_EVENT',
          event: 'commandpalette:item-deleted',
          payload: { namespace: state.namespace.id, itemId: selectedItem.id },
        });
        return { state: clearPending(state), commands };
      }

      // EDIT only works in navigating mode - opens edit form for selected item
      if (action.type === 'EDIT' && state.inputMode === 'navigating') {
        // Check namespace capability
        if (!canUpdate(state.namespace)) {
          return { state, commands };
        }

        const { matches, showCreate, createIndex } = computeItemList(state, ctx);

        // Don't edit if on Create option
        if (showCreate && state.selectedIndex === createIndex) {
          return { state, commands };
        }

        // Check if item can be edited
        const item = matches[state.selectedIndex];
        if (!item || !isItemEditable(item)) {
          return { state, commands };
        }

        return {
          state: {
            type: 'EDIT_FORM',
            namespace: state.namespace,
            item,
            values: { ...item.values },
            focusedField: 0,
          },
          commands,
        };
      }

      // SET only works in navigating mode - sets the selected item as active and closes
      if (action.type === 'SET' && state.inputMode === 'navigating') {
        const { matches, showCreate, createIndex } = computeItemList(state, ctx);

        // Don't set if on Create option
        if (showCreate && state.selectedIndex === createIndex) {
          return { state, commands };
        }

        const item = matches[state.selectedIndex];
        if (item) {
          commands.push({
            type: 'SET_MODE',
            namespace: state.namespace.id,
            itemId: item.id,
          });
          commands.push({ type: 'EMIT_EVENT', event: 'commandpalette:close', payload: undefined });
          return { state: { type: 'IDLE' }, commands };
        }

        return { state, commands };
      }

      // REORDER enters reorder mode (only in navigating mode with items)
      if (action.type === 'REORDER' && state.inputMode === 'navigating') {
        if (!canReorder(state.namespace)) {
          return { state, commands };
        }
        const items = ctx.getItems(state.namespace);
        if (items.length < 2) {
          return { state, commands }; // Need at least 2 items to reorder
        }
        return {
          state: {
            type: 'ITEM_REORDER',
            namespace: state.namespace,
            items: [...items], // Mutable copy
            selectedIndex: state.selectedIndex,
          },
          commands,
        };
      }

      // SELECT: click to select and activate (behaves like setting index then ENTER)
      if (action.type === 'SELECT') {
        const { matches, showCreate, createIndex } = computeItemList(state, ctx);

        // Selected Create option
        if (showCreate && action.index === createIndex) {
          return {
            state: {
              type: 'CREATE_FORM',
              namespace: state.namespace,
              values: { name: state.query },
              focusedField: 1, // Skip name since it's pre-filled
            },
            commands,
          };
        }

        // Selected an existing item
        const item = matches[action.index];
        if (item) {
          // Executable item — run action, close palette
          if (item.action) {
            commands.push(item.action);
            return {
              state: { type: 'IDLE' },
              commands,
            };
          }

          // Regular item — open edit form
          return {
            state: {
              type: 'EDIT_FORM',
              namespace: state.namespace,
              item,
              values: { ...item.values },
              focusedField: 0,
            },
            commands,
          };
        }

        return { state, commands };
      }

      return { state, commands };
    }

    case 'ITEM_REORDER': {
      // ESCAPE or ENTER exits reorder mode and saves the new order
      if (action.type === 'ESCAPE' || action.type === 'ENTER') {
        // Emit reorder command with new order
        commands.push({
          type: 'REORDER_ITEMS',
          namespace: state.namespace.id,
          orderedIds: state.items.map((item) => item.id),
        });
        return {
          state: {
            type: 'ITEM_FILTER',
            namespace: state.namespace,
            query: '',
            selectedIndex: state.selectedIndex, // Preserve selection
            pendingDelete: false,
            inputMode: 'navigating',
          },
          commands,
        };
      }

      // ARROW_UP navigates focus up (does not swap)
      if (action.type === 'ARROW_UP') {
        if (state.selectedIndex <= 0) {
          return { state, commands }; // Already at top
        }
        return {
          state: { ...state, selectedIndex: state.selectedIndex - 1 },
          commands,
        };
      }

      // ARROW_DOWN navigates focus down (does not swap)
      if (action.type === 'ARROW_DOWN') {
        if (state.selectedIndex >= state.items.length - 1) {
          return { state, commands }; // Already at bottom
        }
        return {
          state: { ...state, selectedIndex: state.selectedIndex + 1 },
          commands,
        };
      }

      // MOVE_UP swaps focused item up (triggered by Cmd+Alt+Arrow)
      if (action.type === 'MOVE_UP') {
        if (state.selectedIndex <= 0) {
          return { state, commands }; // Already at top
        }
        const newItems = [...state.items];
        const idx = state.selectedIndex;
        [newItems[idx - 1], newItems[idx]] = [newItems[idx], newItems[idx - 1]];
        return {
          state: {
            ...state,
            items: newItems,
            selectedIndex: idx - 1,
          },
          commands,
        };
      }

      // MOVE_DOWN swaps focused item down (triggered by Cmd+Alt+Arrow)
      if (action.type === 'MOVE_DOWN') {
        if (state.selectedIndex >= state.items.length - 1) {
          return { state, commands }; // Already at bottom
        }
        const newItems = [...state.items];
        const idx = state.selectedIndex;
        [newItems[idx], newItems[idx + 1]] = [newItems[idx + 1], newItems[idx]];
        return {
          state: {
            ...state,
            items: newItems,
            selectedIndex: idx + 1,
          },
          commands,
        };
      }

      return { state, commands };
    }

    case 'EDIT_FORM': {
      if (action.type === 'ESCAPE') {
        return {
          state: {
            type: 'ITEM_FILTER',
            namespace: state.namespace,
            query: '',
            selectedIndex: 0,
            pendingDelete: false,
            inputMode: 'filtering',
          },
          commands,
        };
      }

      if (action.type === 'SET_FIELD') {
        return {
          state: {
            ...state,
            values: { ...state.values, [action.key]: action.value },
          },
          commands,
        };
      }

      if (action.type === 'TAB') {
        const fieldCount = state.namespace.fields.length;
        const nextField = (state.focusedField + 1) % fieldCount;
        return { state: { ...state, focusedField: nextField }, commands };
      }

      if (action.type === 'ENTER') {
        // Use formValues if provided (from DOM), otherwise use state values
        const finalValues = action.formValues ?? state.values;
        const updatedItem: Item = {
          ...state.item,
          name: finalValues.name || state.item.name, // Update top-level name from form
          values: finalValues,
        };
        commands.push({
          type: 'UPDATE_ITEM',
          namespace: state.namespace.id,
          item: updatedItem,
        });
        return {
          state: {
            type: 'ITEM_FILTER',
            namespace: state.namespace,
            query: '',
            selectedIndex: 0,
            pendingDelete: false,
            inputMode: 'filtering',
          },
          commands,
        };
      }

      return { state, commands };
    }

    case 'CREATE_FORM': {
      if (action.type === 'ESCAPE') {
        return {
          state: {
            type: 'ITEM_FILTER',
            namespace: state.namespace,
            query: '',
            selectedIndex: 0,
            pendingDelete: false,
            inputMode: 'filtering',
          },
          commands,
        };
      }

      if (action.type === 'SET_FIELD') {
        return {
          state: {
            ...state,
            values: { ...state.values, [action.key]: action.value },
          },
          commands,
        };
      }

      if (action.type === 'TAB') {
        const fieldCount = state.namespace.fields.length;
        const nextField = (state.focusedField + 1) % fieldCount;
        return { state: { ...state, focusedField: nextField }, commands };
      }

      if (action.type === 'ENTER') {
        // Use formValues if provided (from DOM), otherwise use state values
        const finalValues = action.formValues ?? state.values;
        const pending: PendingItem = {
          name: finalValues.name || '',
          values: finalValues,
        };
        commands.push({
          type: 'CREATE_ITEM',
          namespace: state.namespace.id,
          pending,
        });

        // If opened from editor (closeOnSave), close CP; otherwise return to ITEM_FILTER
        if (state.closeOnSave) {
          commands.push({ type: 'EMIT_EVENT', event: 'commandpalette:close', payload: undefined });
          return { state: { type: 'IDLE' }, commands };
        }

        return {
          state: {
            type: 'ITEM_FILTER',
            namespace: state.namespace,
            query: '',
            selectedIndex: 0,
            pendingDelete: false,
            inputMode: 'filtering',
          },
          commands,
        };
      }

      return { state, commands };
    }
  }
}
