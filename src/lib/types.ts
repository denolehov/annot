// =============================================================================
// Unified line model (LineOrigin + LineSemantics)
// =============================================================================

/** Where this line's content originates from. */
export type LineOrigin =
  | { type: 'source'; path: string; line: number }
  | { type: 'diff'; path: string; old_line: number | null; new_line: number | null }
  | { type: 'virtual' };

/** Content classification: what kind of line is this? */
export type LineSemantics =
  | { type: 'plain' }
  | ({ type: 'markdown' } & MarkdownSemantics)
  | ({ type: 'diff' } & DiffSemantics)
  | ({ type: 'portal' } & PortalSemantics);

/** Markdown structural semantics. */
export type MarkdownSemantics =
  | { kind: 'header'; level: number }
  | { kind: 'code_block_start'; language: string | null; color: string | null }
  | { kind: 'code_block_content' }
  | { kind: 'code_block_end' }
  | { kind: 'table_row' }
  | { kind: 'list_item'; ordered: boolean }
  | { kind: 'block_quote' }
  | { kind: 'horizontal_rule' };

/** Diff line semantics. */
export type DiffSemantics =
  | { kind: 'file_header' }
  | { kind: 'hunk_header'; context: string | null }
  | { kind: 'added' }
  | { kind: 'deleted' }
  | { kind: 'context' };

/** Portal line semantics. */
export type PortalSemantics =
  | { kind: 'header'; label: string; path: string; range: string }
  | { kind: 'content' }
  | { kind: 'footer' };

/** HTML rendering for a line - either full-line or per-cell (for tables). */
export type LineHtml =
  | { type: 'full'; value: string }
  | { type: 'cells'; value: string[] };

export interface Line {
  content: string;
  /** Rendered HTML for display:
   * - Full: for code blocks (syntax-highlighted) and markdown (inline formatting)
   * - Cells: for table rows (per-cell inline formatting)
   * - null if no rendering needed
   */
  html: LineHtml | null;
  /** Where this line originates from. */
  origin: LineOrigin;
  /** Content classification. */
  semantics: LineSemantics;
}

export interface ExitMode {
  id: string;
  name: string;
  color: string;
  instruction: string;
  order: number;
  origin: 'persisted' | 'transient';
}

// Content metadata discriminated union
export type ContentMetadata =
  | { type: 'plain' }
  | ({ type: 'diff' } & DiffMetadata)
  | ({ type: 'markdown' } & MarkdownMetadata);

export interface ContentResponse {
  label: string;
  lines: Line[];
  tags: Tag[];
  exit_modes: ExitMode[];
  selected_exit_mode_id: string | null;
  session_comment: ContentNode[] | null;
  metadata: ContentMetadata;
  /** Whether image paste is allowed (MCP content mode). */
  allows_image_paste: boolean;
  /** All bookmarks for @ autocomplete. */
  bookmarks: Bookmark[];
}

// Diff types
export interface DiffMetadata {
  files: DiffFileInfo[];
  // Note: `lines` HashMap is no longer serialized from backend.
  // Line info is now embedded in each Line's origin/semantics.
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

// Note: DiffLineInfo and DiffLineKind are no longer needed on frontend.
// Line info is now embedded in each Line's origin (LineOrigin::Diff)
// and semantics (LineSemantics::Diff).

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
  /** Last line of section content (inclusive). */
  end_line: number;
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
export type ContentNode = TextNode | TagNode | MediaNode | ExcalidrawNode | ReplaceNode | ErrorNode | PasteNode | BookmarkRefNode;

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

export interface ReplaceNode {
  type: 'replace';
  original: string; // Original line(s) content
  replacement: string; // User's proposed replacement
}

export interface ErrorNode {
  type: 'error';
  source: string; // Error source (e.g., 'mermaid')
  message: string; // Full error message
}

export interface PasteNode {
  type: 'paste';
  content: string; // Full pasted text
}

export interface BookmarkRefNode {
  type: 'bookmarkref';
  id: string; // Full resolved bookmark ID
  label: string; // Cached label for display
}

// TipTap JSON content type for internal storage
export type { JSONContent } from '@tiptap/core';

// Save content response
export interface SaveContentResponse {
  saved_path: string;
  new_label: string;
}

// =============================================================================
// Bookmarks — capture moments of attention for later reference
// =============================================================================

/** Type of session where the bookmark was created. */
export type SessionType = 'file' | 'diff' | 'content';

/** The content snapshot captured by a bookmark. */
export type BookmarkSnapshot =
  | {
      type: 'session';
      source_type: SessionType;
      source_title: string;
      context: string;
    }
  | {
      type: 'selection';
      source_type: SessionType;
      source_title: string;
      context: string;
      selected_text: string;
    };

/** A bookmark capturing a moment of attention during an annot session. */
export interface Bookmark {
  /** Unique 12-character base32 ID (prefix-matchable). */
  id: string;
  /** User-provided or auto-derived label. */
  label: string | null;
  /** When this bookmark was created (ISO 8601). */
  created_at: string;
  /** Project context (cwd at creation time). */
  project_path: string | null;
  /** The captured content snapshot. */
  snapshot: BookmarkSnapshot;
}
