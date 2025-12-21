// Obsidian namespace for CommandPalette
// Hybrid namespace: vault CRUD + export action

import type { Namespace, Item } from '../engine/types';

export const obsidianNamespace: Namespace = {
  id: 'obsidian',
  label: 'Obsidian',
  icon: 'obsidian',
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

  // Add "Add Vault" item at the end
  const addVaultItem: Item = {
    id: '__add_vault__',
    name: '+ Add Vault',
    values: {},
    // No action — selecting this triggers CREATE_FORM via reducer
  };

  return [...exportItems, addVaultItem];
}

/**
 * Set vault items from config (called on init)
 */
export function setObsidianVaults(vaults: string[]): void {
  vaultItems = vaults.map((name) => ({
    id: generateVaultId(),
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
  const items = getObsidianItems();
  if (!query) return items;
  const q = query.toLowerCase();
  return items.filter((item) => item.name.toLowerCase().includes(q));
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

/**
 * Generate a unique ID for new vaults
 */
export function generateVaultId(): string {
  return Math.random().toString(36).substring(2, 14);
}
