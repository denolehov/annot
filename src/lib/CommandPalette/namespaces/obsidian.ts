// Obsidian namespace for CommandPalette
// Hybrid namespace: vault CRUD + export action

import type { Namespace, Item } from '../engine/types';
import { fuzzySearch } from '$lib/fuzzy';
import { SimpleItem } from '../items';
import { generateId } from '$lib/utils/id';

export const obsidianNamespace: Namespace = {
  id: 'obsidian',
  label: 'Obsidian',
  icon: 'obsidian',
  ItemComponent: SimpleItem,
  fields: [{ key: 'name', label: 'Vault Name', type: 'text', required: true }],
  hotkeys: [
    { key: 'd', display: 'dd', label: 'delete', action: 'DELETE' },
    { key: 'e', label: 'edit', action: 'EDIT' },
  ],
  examples: [{ name: 'Personal Notes' }, { name: 'Work' }, { name: 'Research' }],
};

// In-memory storage for vault items
let vaultItems: Item[] = [];

/**
 * Build items list: vault export items + "Add Vault" action
 */
export function getObsidianItems(): Item[] {
  // Each vault becomes an "Export to: VaultName" item with action
  const exportItems: Item[] = vaultItems.map((vault) => ({
    id: vault.id,
    name: `Export to: ${vault.name}`,
    values: vault.values,
    action: { type: 'EXPORT_TO_OBSIDIAN' as const, vault: vault.name },
  }));

  // Add "Add Vault" item at the end (readonly: cannot be deleted/edited)
  const addVaultItem: Item = {
    id: '__add_vault__',
    name: '+ Add Vault',
    values: {},
    readonly: true,
  };

  return [...exportItems, addVaultItem];
}

/**
 * Set vault items from config (called on init)
 */
export function setObsidianVaults(vaults: string[]): void {
  vaultItems = vaults.map((name) => ({
    id: generateId(),
    name,
    values: { name },
  }));
}

/**
 * Get raw vault items (for CRUD operations)
 */
export function getRawVaultItems(): Item[] {
  return vaultItems;
}

/**
 * Filter items by query
 */
export function filterObsidianItems(query: string): Item[] {
  return fuzzySearch(getObsidianItems(), query, [{ name: 'name', weight: 1 }]);
}

/**
 * Save/update a vault item
 */
export function saveObsidianVault(item: Item): void {
  // Update the name from values
  const name = item.values.name || item.name;
  const idx = vaultItems.findIndex((i) => i.id === item.id);
  if (idx >= 0) {
    vaultItems[idx] = { ...item, name, values: { name } };
  } else {
    vaultItems.push({ ...item, name, values: { name } });
  }
  vaultItems = [...vaultItems]; // Trigger reactivity
}

/**
 * Delete a vault item
 */
export function deleteObsidianVault(id: string): void {
  vaultItems = vaultItems.filter((i) => i.id !== id);
}

/**
 * Get vault names for saving to config
 */
export function getVaultNames(): string[] {
  return vaultItems.map((v) => v.name);
}

// Re-export jj-style ID generator for backwards compatibility
export { generateId as generateVaultId };
