// Theme management utilities

import { invoke } from '@tauri-apps/api/core';

export type ThemePreference = 'system' | 'light' | 'dark';
export type EffectiveTheme = 'light' | 'dark';

/**
 * Resolve a theme preference to an effective theme.
 * "system" checks prefers-color-scheme, others pass through.
 */
export function resolveTheme(preference: ThemePreference): EffectiveTheme {
  if (preference === 'system') {
    return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
  }
  return preference;
}

/**
 * Apply theme to the document by setting the data-theme attribute.
 */
export function applyTheme(theme: EffectiveTheme): void {
  document.documentElement.setAttribute('data-theme', theme);
}

/**
 * Load theme preference from backend and apply it.
 * Returns the effective theme that was applied.
 */
export async function initTheme(): Promise<EffectiveTheme> {
  const preference = await invoke<ThemePreference>('get_theme');
  const effective = resolveTheme(preference);
  applyTheme(effective);
  return effective;
}

/**
 * Set theme preference (persists to backend) and apply it.
 */
export async function setTheme(preference: ThemePreference): Promise<void> {
  await invoke('set_theme', { theme: preference });
  const effective = resolveTheme(preference);
  applyTheme(effective);
}

/**
 * Get the currently applied effective theme.
 */
export function getCurrentEffectiveTheme(): EffectiveTheme {
  return (document.documentElement.getAttribute('data-theme') as EffectiveTheme) ?? 'light';
}
