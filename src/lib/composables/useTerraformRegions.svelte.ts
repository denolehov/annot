import { invoke } from '@tauri-apps/api/core';
import type { TerraformRegion } from '$lib/types';
import type { Line } from '$lib/types';
import { validateRange } from '$lib/range';
import type { Range } from '$lib/range';

export interface UseTerraformRegionsOptions {
  /** Lines array for validating ranges and resolving paths */
  getLines: () => Line[];
}

/**
 * Composable for managing terraform regions with backend persistence.
 * Follows the useAnnotations pattern.
 */
export function useTerraformRegions(options: UseTerraformRegionsOptions) {
  let regions: TerraformRegion[] = $state([]);

  /**
   * Load terraform regions for a path and find one matching the given range.
   * Returns the matching region if found, undefined otherwise.
   */
  async function load(range: Range): Promise<TerraformRegion | undefined> {
    const coords = validateRange(range, options.getLines());
    if (!coords) return undefined;

    const all = await invoke<TerraformRegion[]>('get_terraform_regions', { path: coords.path });
    regions = all;
    return all.find(r => r.start_line === coords.startLine && r.end_line === coords.endLine);
  }

  /**
   * Upsert a terraform region to the backend.
   * Also updates the local regions list for visual indicators.
   */
  async function upsert(range: Range, region: TerraformRegion): Promise<void> {
    const coords = validateRange(range, options.getLines());
    if (!coords) return;
    await invoke('upsert_terraform', { path: coords.path, region });
    // Refresh regions to keep visual indicators in sync
    await loadAll(coords.path);
  }

  /**
   * Remove a terraform region from the backend.
   * Also updates the local regions list for visual indicators.
   */
  async function remove(range: Range): Promise<void> {
    const coords = validateRange(range, options.getLines());
    if (!coords) return;
    await invoke('delete_terraform', {
      path: coords.path,
      startLine: coords.startLine,
      endLine: coords.endLine
    });
    // Refresh regions to keep visual indicators in sync
    await loadAll(coords.path);
  }

  /**
   * Load all terraform regions for a file path.
   * Used on mount to populate visual indicators.
   */
  async function loadAll(path: string): Promise<void> {
    regions = await invoke<TerraformRegion[]>('get_terraform_regions', { path });
  }

  /**
   * Check if a source line number starts a terraform region.
   * Returns the region if found, undefined otherwise.
   */
  function isRegionStart(sourceLineNum: number): TerraformRegion | undefined {
    return regions.find(r => r.start_line === sourceLineNum);
  }

  /**
   * Check if a source line number is within any terraform region.
   */
  function isInRegion(sourceLineNum: number): boolean {
    return regions.some(r => sourceLineNum >= r.start_line && sourceLineNum <= r.end_line);
  }

  /**
   * Get display index range for a terraform region.
   * Returns {start, end} display indices (1-indexed).
   */
  function getDisplayRange(region: TerraformRegion): { start: number; end: number } | null {
    const lines = options.getLines();
    let start: number | null = null;
    let end: number | null = null;

    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];
      const lineNum = line.origin.type === 'source' ? line.origin.line
        : line.origin.type === 'diff' ? (line.origin.new_line ?? line.origin.old_line)
        : null;

      if (lineNum === region.start_line && start === null) {
        start = i + 1; // Convert to 1-indexed display index
      }
      if (lineNum === region.end_line) {
        end = i + 1;
      }
    }

    return start !== null && end !== null ? { start, end } : null;
  }

  /**
   * Get the natural language phrase for a terraform region.
   */
  async function getPhrase(region: TerraformRegion): Promise<string> {
    return invoke<string>('get_terraform_phrase', { region });
  }

  return {
    /** All loaded regions (reactive) */
    get regions() { return regions; },
    load,
    upsert,
    remove,
    loadAll,
    isRegionStart,
    isInRegion,
    getDisplayRange,
    getPhrase
  };
}
