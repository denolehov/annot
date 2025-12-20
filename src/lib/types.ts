export interface Line {
  number: number;
  content: string;
  /** Syntax-highlighted HTML with CSS classes, or null if unavailable */
  html: string | null;
}

export interface ExitMode {
  id: string;
  name: string;
  color: string;
  instruction: string;
  order: number;
  is_ephemeral: boolean;
}

export interface ContentResponse {
  label: string;
  lines: Line[];
  tags: Tag[];
  exit_modes: ExitMode[];
  selected_exit_mode_id: string | null;
  session_comment: ContentNode[] | null;
  diff_metadata: DiffMetadata | null;
}

// Diff types
export interface DiffMetadata {
  files: DiffFileInfo[];
  lines: Record<number, DiffLineInfo>;
}

export interface DiffFileInfo {
  old_name: string | null;
  new_name: string | null;
  language: string;
  start_line: number;
  end_line: number;
}

export interface DiffLineInfo {
  kind: DiffLineKind;
  old_line_num: number | null;
  new_line_num: number | null;
  file_index: number;
}

export type DiffLineKind = 'context' | 'added' | 'deleted' | 'header';

// Tag definition (composable mini-prompts)
export interface Tag {
  id: string;
  name: string;
  instruction: string;
}

// Content node types for structured annotation content (output format)
export type ContentNode = TextNode | TagNode;

export interface TextNode {
  type: 'text';
  text: string;
}

export interface TagNode {
  type: 'tag';
  id: string;
  name: string;
  instruction: string;
}

// TipTap JSON content type for internal storage
export type { JSONContent } from '@tiptap/core';
