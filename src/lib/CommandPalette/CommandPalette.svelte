<script lang="ts">
  import { onMount, setContext } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { reduce, computeItemList } from './engine/reducer';
  import { createQueryContext, setTagItems, setExitModeItems, setBookmarkItems, bookmarkToItem, saveTagItem, deleteTagItem, saveExitModeItem, deleteExitModeItem, saveBookmarkItem, deleteBookmarkItem, reorderExitModeItems, generateTagId, generateExitModeId, setObsidianVaults, saveObsidianVault, deleteObsidianVault, getVaultNames, generateVaultId } from './namespaces';
  import type { State, Action, Command, Item, Namespace, InitialState } from './engine/types';
  import { getFilterPlaceholder } from './engine/types';
  import type { Tag, ExitMode, Bookmark } from '$lib/types';
  import Icon from './Icon.svelte';

  // Config type matching Rust
  interface Config {
    obsidian: {
      vaults: string[];
    };
  }

  interface Props {
    tags: Tag[];
    exitModes: ExitMode[];
    bookmarks: Bookmark[];
    onClose: () => void;
    onSetExitMode: (modeId: string) => void;
    onTagsChange?: (tags: Tag[]) => void;
    onExitModesChange?: (modes: ExitMode[]) => void;
    onBookmarkDeleted?: (id: string) => void;
    onBookmarkUpdated?: (id: string, label: string) => void;
    showToast?: (message: string) => void;
    onOpenSaveModal?: () => void;
    initialState?: InitialState;
    onItemCreated?: (item: Item, namespace: string) => void;
    onEvent?: (event: string, payload: unknown) => void;
  }

  let { tags, exitModes, bookmarks, onClose, onSetExitMode, onTagsChange, onExitModesChange, onBookmarkDeleted, onBookmarkUpdated, showToast, onOpenSaveModal, initialState, onItemCreated, onEvent }: Props = $props();

  // Convert domain types to Item format
  function tagToItem(tag: Tag): Item {
    return { id: tag.id, name: tag.name, values: { name: tag.name, instruction: tag.instruction } };
  }

  function exitModeToItem(mode: ExitMode): Item {
    return { id: mode.id, name: mode.name, values: { name: mode.name, instruction: mode.instruction }, isEphemeral: mode.origin === 'transient' };
  }

  function itemToTag(item: Item): Tag {
    return { id: item.id, name: item.values.name || item.name, instruction: item.values.instruction || '' };
  }

  function itemToExitMode(item: Item, original?: ExitMode): ExitMode {
    return {
      id: item.id,
      name: item.values.name || item.name,
      instruction: item.values.instruction || '',
      color: original?.color || '#888888',
      order: original?.order ?? 0,
      origin: original?.origin ?? 'persisted',
    };
  }

  // Initialize namespace stores from props
  $effect(() => {
    setTagItems(tags.map(tagToItem));
  });

  $effect(() => {
    setExitModeItems(exitModes.map(exitModeToItem));
  });

  $effect(() => {
    setBookmarkItems(bookmarks.map(bookmarkToItem));
  });

  // State machine
  let machineState: State = $state({ type: 'IDLE' });
  let ctx = $derived(createQueryContext());

  // Get icon for namespace (theme namespace shows sun/moon based on current theme)
  function getNamespaceIcon(namespace: Namespace): string {
    if (namespace.id === 'theme') {
      const currentTheme = document.documentElement.getAttribute('data-theme');
      return currentTheme === 'dark' ? 'moon' : 'sun';
    }
    return namespace.icon;
  }

  // Modal element reference
  let modalEl: HTMLDivElement | undefined = $state(undefined);
  let inputEl: HTMLInputElement | undefined = $state(undefined);
  let listEl: HTMLUListElement | undefined = $state(undefined);

  // Current theme preference (for showing checkmark in theme namespace)
  let currentThemePreference: string = $state('system');

  // Expose to ThemeItem via context
  setContext('currentThemeId', () => `theme-${currentThemePreference}`);

  // Fetch current theme on mount
  onMount(() => {
    invoke<string>('get_theme').then((theme) => {
      currentThemePreference = theme;
    }).catch(() => {
      // Ignore errors, default to 'system'
    });
  });

  function dispatch(action: Action) {
    const result = reduce(machineState, action, ctx);

    // Execute commands
    for (const cmd of result.commands) {
      executeCommand(cmd);
    }

    // Update state
    machineState = result.state;

    // Close if IDLE
    if (machineState.type === 'IDLE') {
      onClose();
    }
  }

  function executeCommand(cmd: Command) {
    switch (cmd.type) {
      case 'SET_MODE':
        onSetExitMode(cmd.itemId);
        break;

      case 'CREATE_ITEM': {
        if (cmd.namespace === 'tags') {
          const newItem: Item = {
            id: generateTagId(),
            name: cmd.pending.name,
            values: cmd.pending.values,
          };
          saveTagItem(newItem);
          onTagsChange?.(ctx.getItems({ id: 'tags' } as Namespace).map(itemToTag));
          onItemCreated?.(newItem, 'tags');
        } else if (cmd.namespace === 'exit-modes') {
          const newItem: Item = {
            id: generateExitModeId(cmd.pending.name),
            name: cmd.pending.name,
            values: cmd.pending.values,
          };
          saveExitModeItem(newItem);
          onExitModesChange?.(ctx.getItems({ id: 'exit-modes' } as Namespace).map((i) => itemToExitMode(i)));
        }
        break;
      }

      case 'UPDATE_ITEM': {
        if (cmd.namespace === 'tags') {
          saveTagItem(cmd.item);
          onTagsChange?.(ctx.getItems({ id: 'tags' } as Namespace).map(itemToTag));
        } else if (cmd.namespace === 'exit-modes') {
          const original = exitModes.find((m) => m.id === cmd.item.id);
          saveExitModeItem(cmd.item);
          onExitModesChange?.(ctx.getItems({ id: 'exit-modes' } as Namespace).map((i) => {
            const orig = exitModes.find((m) => m.id === i.id);
            return itemToExitMode(i, orig);
          }));
        } else if (cmd.namespace === 'bookmarks') {
          saveBookmarkItem(cmd.item);
          onBookmarkUpdated?.(cmd.item.id, cmd.item.values.label || cmd.item.name);
          // Force state update to trigger itemListData recompute
          machineState = { ...machineState };
        }
        break;
      }

      case 'DELETE_ITEM': {
        if (cmd.namespace === 'tags') {
          deleteTagItem(cmd.itemId);
          onTagsChange?.(ctx.getItems({ id: 'tags' } as Namespace).map(itemToTag));
        } else if (cmd.namespace === 'exit-modes') {
          deleteExitModeItem(cmd.itemId);
          onExitModesChange?.(ctx.getItems({ id: 'exit-modes' } as Namespace).map((i) => {
            const orig = exitModes.find((m) => m.id === i.id);
            return itemToExitMode(i, orig);
          }));
        } else if (cmd.namespace === 'bookmarks') {
          deleteBookmarkItem(cmd.itemId);
          onBookmarkDeleted?.(cmd.itemId);
        }
        inputEl?.focus();
        break;
      }

      case 'REORDER_ITEMS': {
        if (cmd.namespace === 'exit-modes') {
          reorderExitModeItems(cmd.orderedIds);
          onExitModesChange?.(ctx.getItems({ id: 'exit-modes' } as Namespace).map((i) => {
            const orig = exitModes.find((m) => m.id === i.id);
            return itemToExitMode(i, orig);
          }));
        }
        break;
      }

      case 'COPY_TO_CLIPBOARD': {
        const labels: Record<string, string> = {
          content: 'Content',
          annotations: 'Annotations',
          all: 'Content + Annotations',
        };
        invoke('copy_to_clipboard', { mode: cmd.mode })
          .then(() => showToast?.(`${labels[cmd.mode]} copied!`))
          .catch((e) => showToast?.(`Failed to copy: ${e}`));
        break;
      }

      case 'OPEN_SAVE_MODAL':
        onOpenSaveModal?.();
        break;

      case 'EXPORT_TO_OBSIDIAN': {
        invoke<{ url: string }>('export_to_obsidian', { vaultName: cmd.vault })
          .then(async (result) => {
            // Content already copied to clipboard by Rust
            // Open Obsidian via plugin opener (handles custom protocols)
            await openUrl(result.url);
            showToast?.('Opening in Obsidian...');
          })
          .catch((e) => showToast?.(`Export failed: ${e}`));
        break;
      }

      case 'EMIT_EVENT':
        onEvent?.(cmd.event, cmd.payload);
        break;
    }

    // Handle obsidian namespace CRUD
    if (cmd.type === 'CREATE_ITEM' && cmd.namespace === 'obsidian') {
      const newItem: Item = {
        id: generateVaultId(),
        name: cmd.pending.values.name || cmd.pending.name,
        values: cmd.pending.values,
      };
      saveObsidianVault(newItem);
      persistObsidianConfig();
    } else if (cmd.type === 'UPDATE_ITEM' && cmd.namespace === 'obsidian') {
      saveObsidianVault(cmd.item);
      persistObsidianConfig();
    } else if (cmd.type === 'DELETE_ITEM' && cmd.namespace === 'obsidian') {
      deleteObsidianVault(cmd.itemId);
      persistObsidianConfig();
    }
  }

  // Persist obsidian vaults to config file
  async function persistObsidianConfig() {
    try {
      const config: Config = {
        obsidian: {
          vaults: getVaultNames(),
        },
      };
      await invoke('save_config', { config });
    } catch (e) {
      console.error('Failed to save config:', e);
      showToast?.(`Failed to save config: ${e}`);
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    const target = e.target as HTMLElement;
    const inTextarea = target instanceof HTMLTextAreaElement;
    const inInput = target instanceof HTMLInputElement;
    const inForm = machineState.type === 'EDIT_FORM' || machineState.type === 'CREATE_FORM';

    // Form states: only handle Escape and Cmd+Enter globally
    if (inForm && (inTextarea || inInput)) {
      if (e.key === 'Escape') {
        e.preventDefault();
        dispatch({ type: 'ESCAPE' });
        return;
      }
      if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
        e.preventDefault();
        const formValues = readFormValues();
        dispatch({ type: 'ENTER', formValues });
        return;
      }
      if (e.key === 'Tab' && !e.shiftKey) {
        e.preventDefault();
        dispatch({ type: 'TAB' });
        return;
      }
      return; // Let input handle other keys
    }

    // Reorder state: special handling
    if (machineState.type === 'ITEM_REORDER') {
      if (e.key === 'Escape') {
        e.preventDefault();
        dispatch({ type: 'ESCAPE' });
      } else if (e.key === 'Enter') {
        e.preventDefault();
        dispatch({ type: 'ENTER' });
      } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        if (e.metaKey && e.altKey) {
          dispatch({ type: 'MOVE_UP' });
        } else {
          dispatch({ type: 'ARROW_UP' });
        }
      } else if (e.key === 'ArrowDown') {
        e.preventDefault();
        if (e.metaKey && e.altKey) {
          dispatch({ type: 'MOVE_DOWN' });
        } else {
          dispatch({ type: 'ARROW_DOWN' });
        }
      }
      return;
    }

    // Filter states
    if (e.key === 'Escape') {
      e.preventDefault();
      dispatch({ type: 'ESCAPE' });
    } else if (e.key === 'Enter') {
      e.preventDefault();
      dispatch({ type: 'ENTER' });
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      dispatch({ type: 'ARROW_UP' });
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      dispatch({ type: 'ARROW_DOWN' });
    } else if (e.key === 'Backspace') {
      // Only dispatch if input is empty or not in input
      if (inInput) {
        const input = target as HTMLInputElement;
        if (input.value === '') {
          e.preventDefault();
          dispatch({ type: 'BACKSPACE' });
        }
      } else {
        e.preventDefault();
        dispatch({ type: 'BACKSPACE' });
      }
    } else if (machineState.type === 'ITEM_FILTER' && machineState.inputMode === 'navigating') {
      // Check namespace hotkeys
      const hotkey = machineState.namespace.hotkeys?.find((h) => h.key === e.key);
      if (hotkey) {
        e.preventDefault();
        dispatch({ type: hotkey.action } as Action);
      } else if (/^[a-zA-Z0-9]$/.test(e.key) && !e.metaKey && !e.ctrlKey && !e.altKey) {
        // Switch to filtering mode on printable character
        e.preventDefault();
        dispatch({ type: 'INPUT', char: e.key });
      }
    } else if (machineState.type === 'NAMESPACE_FILTER' && machineState.inputMode === 'navigating') {
      if (/^[a-zA-Z0-9]$/.test(e.key) && !e.metaKey && !e.ctrlKey && !e.altKey) {
        e.preventDefault();
        dispatch({ type: 'INPUT', char: e.key });
      }
    }
  }

  function handleInput(e: Event) {
    const target = e.target as HTMLInputElement;
    const value = target.value;

    // Get current query from state
    let currentQuery = '';
    if (machineState.type === 'NAMESPACE_FILTER' || machineState.type === 'ITEM_FILTER') {
      currentQuery = machineState.query;
    }

    // Compute the diff and dispatch INPUT or BACKSPACE
    if (value.length > currentQuery.length) {
      const newChar = value.slice(currentQuery.length);
      for (const char of newChar) {
        dispatch({ type: 'INPUT', char });
      }
    } else if (value.length < currentQuery.length) {
      const deleteCount = currentQuery.length - value.length;
      for (let i = 0; i < deleteCount; i++) {
        dispatch({ type: 'BACKSPACE' });
      }
    }
  }

  function readFormValues(): Record<string, string> {
    const form = modalEl?.querySelector('form');
    if (!form) return {};
    const data = new FormData(form);
    const values: Record<string, string> = {};
    for (const [key, val] of data.entries()) {
      values[key] = val as string;
    }
    return values;
  }

  // Load config and open on mount
  onMount(async () => {
    // Load obsidian vaults from config
    try {
      const config = await invoke<Config>('get_config');
      setObsidianVaults(config.obsidian?.vaults || []);
    } catch (e) {
      console.error('Failed to load config:', e);
      setObsidianVaults([]);
    }
    dispatch({ type: 'OPEN', initialState });
  });

  // Focus management for filter states
  $effect(() => {
    if (machineState.type === 'NAMESPACE_FILTER' || machineState.type === 'ITEM_FILTER') {
      if (machineState.inputMode === 'filtering') {
        inputEl?.focus();
      } else {
        // In navigating mode: blur input, focus modal for key capture
        inputEl?.blur();
        modalEl?.focus();
      }
    }
  });

  // Focus correct field in form states
  $effect(() => {
    if (machineState.type === 'EDIT_FORM' || machineState.type === 'CREATE_FORM') {
      // Capture values before async callback
      const fieldKey = machineState.namespace.fields[machineState.focusedField]?.key;
      if (fieldKey) {
        // Small delay to ensure DOM is ready
        requestAnimationFrame(() => {
          const el = modalEl?.querySelector(`[name="${fieldKey}"]`) as HTMLInputElement | HTMLTextAreaElement;
          el?.focus();
        });
      }
    }
  });

  // Focus modal in reorder mode for key capture
  $effect(() => {
    if (machineState.type === 'ITEM_REORDER') {
      requestAnimationFrame(() => {
        modalEl?.focus();
      });
    }
  });

  // Scroll selected item into view when selection changes
  $effect(() => {
    if (machineState.type === 'NAMESPACE_FILTER' || machineState.type === 'ITEM_FILTER' || machineState.type === 'ITEM_REORDER') {
      const idx = machineState.selectedIndex;
      requestAnimationFrame(() => {
        // Use nth-child to find the selected li - works for all filter/reorder states
        // regardless of whether selection is indicated via .selected class or data-state
        const selected = listEl?.querySelector(`li:nth-child(${idx + 1})`) as HTMLElement | null;
        selected?.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
      });
    }
  });

  // Computed values for rendering
  let filteredNamespaces = $derived.by(() => {
    if (machineState.type !== 'NAMESPACE_FILTER') return [];
    return ctx.filterNamespaces(machineState.query);
  });

  let itemListData = $derived.by(() => {
    if (machineState.type !== 'ITEM_FILTER') return { matches: [], showCreate: false, createIndex: -1, totalItems: 0 };
    const result = computeItemList(machineState, ctx);
    const totalItems = ctx.getItems(machineState.namespace).length;
    return { ...result, totalItems };
  });

  // Get current query for display
  let currentQuery = $derived.by(() => {
    if (machineState.type === 'NAMESPACE_FILTER' || machineState.type === 'ITEM_FILTER') {
      return machineState.query;
    }
    return '';
  });

  // Check if in navigating mode (for visual feedback)
  let isNavigating = $derived.by(() => {
    if (machineState.type === 'NAMESPACE_FILTER' || machineState.type === 'ITEM_FILTER') {
      return machineState.inputMode === 'navigating';
    }
    return false;
  });

  // Example placeholders for CREATE_FORM (pick random example from namespace)
  let examplePlaceholders: Record<string, string> = $state({});
  $effect(() => {
    if (machineState.type === 'CREATE_FORM') {
      const examples = machineState.namespace.examples;
      if (examples && examples.length > 0) {
        examplePlaceholders = examples[Math.floor(Math.random() * examples.length)];
      } else {
        examplePlaceholders = {};
      }
    }
  });

  // Get footer hints (matching hl poly editor style with Unicode symbols)
  let footerHints = $derived.by(() => {
    if (machineState.type === 'NAMESPACE_FILTER') {
      return [
        { key: '↑↓', label: 'navigate' },
        { key: '↵', label: 'select' },
        { key: 'Esc', label: 'close' },
      ];
    }
    if (machineState.type === 'ITEM_FILTER') {
      const hints: Array<{ key: string; label: string }> = [{ key: '↑↓', label: 'navigate' }];
      if (machineState.inputMode === 'navigating' && machineState.namespace.hotkeys) {
        for (const hk of machineState.namespace.hotkeys) {
          hints.push({ key: hk.display || hk.key, label: hk.label });
        }
      }
      hints.push({ key: '↵', label: 'select' });
      hints.push({ key: 'Esc', label: 'back' });
      return hints;
    }
    if (machineState.type === 'ITEM_REORDER') {
      return [
        { key: '↑↓', label: 'nav' },
        { key: '⌘⌥↑↓', label: 'move' },
        { key: '↵', label: 'save' },
      ];
    }
    if (machineState.type === 'EDIT_FORM' || machineState.type === 'CREATE_FORM') {
      return [
        { key: '⌘↵', label: 'save' },
        { key: 'Tab', label: 'next field' },
        { key: 'Esc', label: 'cancel' },
      ];
    }
    return [];
  });
</script>

<div
  class="backdrop"
  role="presentation"
  onclick={() => dispatch({ type: 'CLOSE' })}
  onkeydown={() => {}}
></div>

<div
  class="modal"
  role="dialog"
  aria-modal="true"
  bind:this={modalEl}
  onkeydown={handleKeyDown}
  tabindex="-1"
>
  {#if machineState.type === 'NAMESPACE_FILTER'}
    <div class="filter-view">
      <div class="input-row">
        <span class="search-icon"><Icon name="search" /></span>
        <input
          bind:this={inputEl}
          type="text"
          class="inline-input"
          class:navigating={isNavigating}
          placeholder="Filter namespaces..."
          value={currentQuery}
          oninput={handleInput}
        />
      </div>
      <ul class="item-list" class:filtering={!isNavigating} role="listbox" bind:this={listEl}>
        {#each filteredNamespaces as ns, i}
          <li
            class="item"
            class:selected={machineState.selectedIndex === i}
            role="option"
            aria-selected={machineState.selectedIndex === i}
            onclick={() => dispatch({ type: 'SELECT', index: i })}
            onkeydown={() => {}}
          >
            <span class="icon"><Icon name={getNamespaceIcon(ns)} /></span>
            <span class="label">{ns.label}</span>
          </li>
        {/each}
      </ul>
    </div>

  {:else if machineState.type === 'ITEM_FILTER'}
    <div class="filter-view">
      <div class="input-row">
        <span class="search-icon"><Icon name="search" /></span>
        <span class="ns-prefix"><Icon name={getNamespaceIcon(machineState.namespace)} /> {machineState.namespace.label}</span>
        <span class="separator">›</span>
        <input
          bind:this={inputEl}
          type="text"
          class="inline-input"
          class:navigating={isNavigating}
          placeholder={getFilterPlaceholder(machineState.namespace, itemListData.totalItems)}
          value={currentQuery}
          oninput={handleInput}
        />
      </div>
      {#if itemListData.matches.length > 0 || itemListData.showCreate}
        <ul class="item-list" class:filtering={!isNavigating} role="listbox" bind:this={listEl}>
          {#each itemListData.matches as item, i}
            {@const ItemComponent = machineState.namespace.ItemComponent}
            {@const isSelected = machineState.selectedIndex === i}
            {@const itemState = !isSelected
              ? 'idle'
              : machineState.pendingDelete
                ? 'pending-delete'
                : machineState.inputMode === 'filtering'
                  ? 'preselected'
                  : 'selected'}
            <li
              role="option"
              aria-selected={isSelected}
              onclick={() => dispatch({ type: 'SELECT', index: i })}
              onkeydown={() => {}}
            >
              <ItemComponent {item} selectionState={itemState} />
            </li>
          {/each}
          {#if itemListData.showCreate}
          <li
            class="item create-item"
            class:selected={machineState.selectedIndex === itemListData.createIndex}
            role="option"
            aria-selected={machineState.selectedIndex === itemListData.createIndex}
            onclick={() => dispatch({ type: 'SELECT', index: itemListData.createIndex })}
            onkeydown={() => {}}
          >
            <span class="icon">+</span>
            <span class="create-label">Create "{machineState.query}"</span>
          </li>
          {/if}
        </ul>
      {/if}
    </div>

  {:else if machineState.type === 'ITEM_REORDER'}
    <div class="reorder-view">
      <div class="input-row">
        <span class="search-icon"><Icon name="reorder" /></span>
        <span class="ns-prefix"><Icon name={getNamespaceIcon(machineState.namespace)} /> {machineState.namespace.label}</span>
        <span class="separator">›</span>
        <span class="mode-prefix">Reorder</span>
      </div>
      <ul class="item-list" bind:this={listEl}>
        {#each machineState.items as item, i}
          <li
            class="item"
            class:selected={machineState.selectedIndex === i}
          >
            <span class="drag-handle">=</span>
            <span class="name">{item.name}</span>
          </li>
        {/each}
      </ul>
    </div>

  {:else if machineState.type === 'EDIT_FORM' || machineState.type === 'CREATE_FORM'}
    <div class="form-view">
      <div class="input-row">
        <span class="search-icon"><Icon name={machineState.type === 'CREATE_FORM' ? 'plus' : 'edit'} /></span>
        <span class="ns-prefix"><Icon name={getNamespaceIcon(machineState.namespace)} /> {machineState.namespace.label}</span>
        <span class="separator">›</span>
        <span class="mode-prefix">{machineState.type === 'CREATE_FORM' ? 'New' : 'Edit'}</span>
      </div>
      <form>
        {#each machineState.namespace.fields as field, i}
          <div class="field" class:focused={machineState.focusedField === i}>
            <label for={field.key}>{field.label}</label>
            {#if field.type === 'text'}
              {@const placeholder = machineState.type === 'CREATE_FORM' ? examplePlaceholders[field.key] || field.placeholder : field.placeholder}
              <input
                type="text"
                id={field.key}
                name={field.key}
                value={machineState.values[field.key] || ''}
                {placeholder}
                readonly={machineState.type === 'EDIT_FORM' && field.readOnlyInEdit}
              />
            {:else if field.type === 'textarea'}
              {@const placeholder = machineState.type === 'CREATE_FORM' ? examplePlaceholders[field.key] || field.placeholder : field.placeholder}
              <textarea
                id={field.key}
                name={field.key}
                {placeholder}
                readonly={machineState.type === 'EDIT_FORM' && field.readOnlyInEdit}
              >{machineState.values[field.key] || ''}</textarea>
            {:else if field.type === 'select'}
              <select id={field.key} name={field.key} value={machineState.values[field.key] || ''}>
                {#each field.options as option}
                  <option value={option}>{option}</option>
                {/each}
              </select>
            {/if}
          </div>
        {/each}
      </form>
    </div>
  {/if}

  <!-- Footer with keyboard hints -->
  {#if footerHints.length > 0}
    <div class="footer">
      {#each footerHints as hint}
        <span class="kbd-hint">
          <kbd>{hint.key}</kbd>
          <span>{hint.label}</span>
        </span>
      {/each}
    </div>
  {/if}
</div>

<style>
  /* Component styles - see src/styles/command-palette.css for shared styles */
</style>
