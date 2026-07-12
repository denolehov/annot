// =============================================================================
// Unified line model (LineOrigin + LineSemantics)
// =============================================================================

/** Where this line's content originates from. */
export type LineOrigin =
  | { type: 'source'; path: string; line: number }
  | { type: 'virtual' };

/** Content classification: what kind of line is this? */
export type LineSemantics =
  | { type: 'plain' }
  | ({ type: 'markdown' } & MarkdownSemantics)
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

// Content metadata discriminated union (non-diff extras; diff mode
// discriminates on `view.type`)
export type ContentMetadata =
  | { type: 'plain' }
  | ({ type: 'markdown' } & MarkdownMetadata);

/** The wire shape of the content itself. */
export type ContentView =
  | { type: 'flat'; lines: Line[] }
  | { type: 'diff'; documents: DiffDocument[] };

export interface ContentResponse {
  label: string;
  view: ContentView;
  tags: Tag[];
  exit_modes: ExitMode[];
  selected_exit_mode_id: string | null;
  session_comment: ContentNode[] | null;
  metadata: ContentMetadata;
  /** Whether image paste is allowed (MCP content mode). */
  allows_image_paste: boolean;
}

/** Config snapshot returned by reload_config command. */
export interface ConfigSnapshot {
  tags: Tag[];
  exit_modes: ExitMode[];
}

// =============================================================================
// Per-file diff documents — the diff wire shape
// =============================================================================

/**
 * Half-open, 1-indexed source line range — Rust `Range<u32>`, in git-printed
 * convention: an empty side starts at the line before the position
 * (`0..0` for a new file's old side).
 */
export interface LineRange {
  start: number;
  end: number;
}

/** VCS file status (vcs::FileStatus serialized as a plain string). */
export type FileStatus = 'modified' | 'added' | 'deleted' | 'renamed' | 'copied' | 'type_changed';

export interface Row {
  /** old-only = deleted, new-only = added, both = context — no kind field. */
  old_line: number | null;
  new_line: number | null;
  /** Raw source line — no +/-/space prefix; the sign is presentation. */
  content: string;
  html: LineHtml | null;
  /**
   * Changed-token ranges within `content`, half-open UTF-16 code-unit
   * offsets. Absent except on added/deleted rows in word-diff-gated hunks
   * (serde skips the empty vec).
   */
  word_ranges?: { start: number; end: number }[];
}

export interface HunkV2 {
  old_range: LineRange;
  new_range: LineRange;
  function_context: string | null;
  function_context_html: string | null;
  /** Hunks own their rows — S3 unfold is hunk-local splicing. */
  rows: Row[];
}

export interface DiffDocument {
  /** Display identity: new name (old for deleted files). */
  path: string;
  /** Present ⇒ rename; S1 renders "old → new". */
  old_path: string | null;
  status: FileStatus;
  /** Binary/oversize/non-UTF-8 — no hunks. */
  unavailable: boolean;
  language: string;
  /**
   * New-side total line count. Non-null ⇒ unfold available (the capability
   * signal) and sizes the trailing gap. Null ⇒ raw-patch mode or no new
   * side (deleted file) — no gap bars.
   */
  new_len: number | null;
  hunks: HunkV2[];
}

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
export type ContentNode = TextNode | TagNode | MediaNode | ExcalidrawNode | ReplaceNode | ErrorNode | PasteNode | RefNode | FileNode;

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

// =============================================================================
// Unified Reference System (@ mentions)
// =============================================================================

/** Snapshot for annotation references (self-contained). */
export interface AnnotationRefSnapshot {
  type: 'annotation';
  /** Anchor-derived line label, e.g., "50-55" (display only) */
  source_key: string;
  /** File path (for cross-file display, null for same-file) */
  source_file: string | null;
  /** First ~50 chars of annotation content for tooltip preview */
  preview: string;
  /** Full annotation content captured at insertion time (self-contained) */
  content: ContentNode[];
}

/** Snapshot for heading section references. */
export interface HeadingRefSnapshot {
  type: 'heading';
  /** Source line number of the heading. */
  line: number;
  /** Heading level (1-6). */
  level: number;
  /** Heading title text. */
  title: string;
}

/** Unified reference snapshot - annotation or heading. */
export type RefSnapshot = AnnotationRefSnapshot | HeadingRefSnapshot;

/** Unified reference node for @ mentions. */
export interface RefNode {
  type: 'ref';
  /** Discriminator for ref type: 'annotation' or 'heading' */
  ref_type: 'annotation' | 'heading';
  /** Self-contained snapshot (survives source deletion) */
  snapshot: RefSnapshot;
}

/** Reference to a project file via @ mention. */
export interface FileNode {
  type: 'file';
  /** Path relative to project root. */
  path: string;
}

// TipTap JSON content type for internal storage
export type { JSONContent } from '@tiptap/core';

// Save content response
export interface SaveContentResponse {
  saved_path: string;
  new_label: string;
}
