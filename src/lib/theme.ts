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

interface ApplyThemeOptions {
  /** Temporarily disable CSS transitions to prevent visual flash */
  suppressTransitions?: boolean;
}

/**
 * Apply theme to the document by setting the data-theme attribute.
 */
export function applyTheme(theme: EffectiveTheme, options: ApplyThemeOptions = {}): void {
  const { suppressTransitions } = options;

  if (suppressTransitions) {
    document.documentElement.classList.add('no-transitions');
  }

  document.documentElement.setAttribute('data-theme', theme);

  if (suppressTransitions) {
    // Force reflow to ensure styles are applied before removing the class
    document.documentElement.offsetHeight;
    document.documentElement.classList.remove('no-transitions');
  }
}

/**
 * Load theme preference from backend and apply it.
 * Returns the effective theme that was applied.
 * Suppresses transitions to prevent flash on initial load.
 */
export async function initTheme(): Promise<EffectiveTheme> {
  const preference = await invoke<ThemePreference>('get_theme');
  const effective = resolveTheme(preference);
  applyTheme(effective, { suppressTransitions: true });
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
