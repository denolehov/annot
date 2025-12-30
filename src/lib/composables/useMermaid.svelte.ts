/**
 * Mermaid diagram handling composable.
 *
 * Manages:
 * - Validation of mermaid blocks with error tracking
 * - Content extraction from code blocks
 * - Opening mermaid preview windows
 */

import { invoke } from '@tauri-apps/api/core';
import type { Line, MarkdownMetadata, CodeBlockInfo } from '$lib/types';
import { extractCodeBlockContent } from '$lib/line-utils';
import { validateMermaid } from '$lib/mermaid-loader';

export interface UseMermaidDeps {
  getLines: () => Line[];
  getLabel: () => string;
  getMarkdownMetadata: () => MarkdownMetadata | null;
}

export function useMermaid(deps: UseMermaidDeps) {
  const { getLines, getLabel, getMarkdownMetadata } = deps;

  // Mermaid validation errors keyed by "startLine-endLine"
  let mermaidErrors = $state<Map<string, string>>(new Map());

  // Validate mermaid blocks when content changes
  $effect(() => {
    const markdownMetadata = getMarkdownMetadata();
    if (!markdownMetadata?.code_blocks) {
      mermaidErrors = new Map();
      return;
    }

    const mermaidBlocks = markdownMetadata.code_blocks.filter(b => b.language === 'mermaid');
    if (mermaidBlocks.length === 0) {
      mermaidErrors = new Map();
      return;
    }

    // Validate each mermaid block
    const newErrors = new Map<string, string>();
    const validations = mermaidBlocks.map(async (block) => {
      const source = getMermaidContent(block.start_line, block.end_line);
      const error = await validateMermaid(source);
      if (error) {
        newErrors.set(`${block.start_line}-${block.end_line}`, error);
      }
    });

    Promise.all(validations).then(() => {
      mermaidErrors = newErrors;
    });
  });

  /**
   * Check if a line starts a mermaid code block.
   * Returns the CodeBlockInfo if found, null otherwise.
   */
  function getMermaidBlockAt(lineNum: number): CodeBlockInfo | null {
    const markdownMetadata = getMarkdownMetadata();
    if (!markdownMetadata?.code_blocks) return null;
    return markdownMetadata.code_blocks.find(
      b => b.start_line === lineNum && b.language === 'mermaid'
    ) ?? null;
  }

  /**
   * Extract mermaid content from a code block (excluding fence lines).
   */
  function getMermaidContent(startLine: number, endLine: number): string {
    return extractCodeBlockContent(getLines(), startLine, endLine, getLabel());
  }

  /**
   * Get the validation error for a mermaid block, if any.
   */
  function getMermaidError(startLine: number, endLine: number): string | null {
    return mermaidErrors.get(`${startLine}-${endLine}`) ?? null;
  }

  /**
   * Open the mermaid preview window for a code block.
   */
  async function openMermaidWindow(block: { start_line: number; end_line: number }): Promise<void> {
    const source = getMermaidContent(block.start_line, block.end_line);
    try {
      await invoke('open_mermaid_window', {
        source,
        filePath: getLabel(),
        startLine: block.start_line,
        endLine: block.end_line,
      });
    } catch (e) {
      console.error('Failed to open mermaid window:', e);
    }
  }

  return {
    /** Reactive map of validation errors keyed by "startLine-endLine" */
    get mermaidErrors() { return mermaidErrors; },
    getMermaidBlockAt,
    getMermaidContent,
    getMermaidError,
    openMermaidWindow,
  };
}
