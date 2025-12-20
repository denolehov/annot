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
  markdown_metadata: MarkdownMetadata | null;
  /** Whether this is an ephemeral session (enables image paste). */
  ephemeral: boolean;
}

// Diff types
export interface DiffMetadata {
  files: DiffFileInfo[];
  lines: Record<number, DiffLineInfo>;
}

export interface HunkInfo {
  display_line: number;
  old_start: number;
  old_count: number;
  new_start: number;
  new_count: number;
  function_context: string | null;
  function_context_html: string | null;
}

export interface DiffFileInfo {
  old_name: string | null;
  new_name: string | null;
  language: string;
  start_line: number;
  end_line: number;
  hunks: HunkInfo[];
}

export interface DiffLineInfo {
  kind: DiffLineKind;
  old_line_num: number | null;
  new_line_num: number | null;
  file_index: number;
}

export type DiffLineKind = 'context' | 'added' | 'deleted' | 'header';

// Markdown types
export interface MarkdownMetadata {
  sections: SectionInfo[];
  code_blocks: CodeBlockInfo[];
  tables: TableInfo[];
}

export interface SectionInfo {
  source_line: number;
  level: number;
  title: string;
  parent_index: number | null;
}

export interface CodeBlockInfo {
  start_line: number;
  end_line: number;
  language: string | null;
}

export interface TableInfo {
  start_line: number;
  end_line: number;
  formatted_lines: string[];
}

// Tag definition (composable mini-prompts)
export interface Tag {
  id: string;
  name: string;
  instruction: string;
}

// Content node types for structured annotation content (output format)
export type ContentNode = TextNode | TagNode | MediaNode | ExcalidrawNode;

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

export interface MediaNode {
  type: 'media';
  image: string; // data URL: "data:image/png;base64,..."
  mime_type: string; // e.g., "image/png"
}

export interface ExcalidrawNode {
  type: 'excalidraw';
  elements: string; // JSON string of Excalidraw elements
  image?: string; // base64 PNG data URL for MCP export
}

// TipTap JSON content type for internal storage
export type { JSONContent } from '@tiptap/core';
