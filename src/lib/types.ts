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

/** Config snapshot returned by reload_config command. */
export interface ConfigSnapshot {
  tags: Tag[];
  exit_modes: ExitMode[];
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
export type ContentNode = TextNode | TagNode | MediaNode | ExcalidrawNode | ReplaceNode | ErrorNode | PasteNode | BookmarkRefNode | RefNode | FileNode;

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
  /** Full bookmark data captured at insertion time (for detachment). */
  bookmark: Bookmark;
}

// =============================================================================
// Unified Reference System (@ mentions)
// =============================================================================

/** Snapshot for annotation references (self-contained). */
export interface AnnotationRefSnapshot {
  type: 'annotation';
  /** Line range key, e.g., "50-55" */
  source_key: string;
  /** File path (for cross-file display, null for same-file) */
  source_file: string | null;
  /** First ~50 chars of annotation content for tooltip preview */
  preview: string;
  /** Full annotation content captured at insertion time (self-contained) */
  content: ContentNode[];
}

/** Unified reference snapshot - either annotation or bookmark. */
export type RefSnapshot = AnnotationRefSnapshot | { type: 'bookmark'; bookmark: Bookmark };

/** Unified reference node - replaces BookmarkRefNode for new references. */
export interface RefNode {
  type: 'ref';
  /** Discriminator for ref type: 'annotation' or 'bookmark' */
  ref_type: 'annotation' | 'bookmark';
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

// =============================================================================
// Terraform — structured controls for guiding AI content transformation
// =============================================================================

/** Target format for restructuring. */
export type FormType = 'table' | 'list' | 'prose' | 'diagram' | 'code';

/** Intensity level for graduated controls. */
export type Intensity = 'slightly' | 'moderately' | 'significantly';

/** All intensity levels in order (for slider). */
export const INTENSITY_LEVELS: Intensity[] = ['slightly', 'moderately', 'significantly'];

/** All form types in order (for buttons). */
export const FORM_TYPES: FormType[] = ['table', 'list', 'prose', 'diagram', 'code'];

/** Mass change: expand or condense (remove is a separate intent). */
export type MassChange =
  | { type: 'expand'; intensity: Intensity }
  | { type: 'condense'; intensity: Intensity };

/** Gravity change: focus or blur (pin/dissolve are separate intents). */
export type GravityChange =
  | { type: 'focus'; intensity: Intensity }
  | { type: 'blur'; intensity: Intensity };

/** Correctness signal: lean-in, move-away, or reframe. */
export type DirectionDirective =
  | { type: 'leanin'; intensity: Intensity }
  | { type: 'moveaway'; intensity: Intensity }
  | { type: 'reframe' };

/** Type-safe terraform intent — invalid combinations are unrepresentable. */
export type TerraformIntent =
  | { kind: 'remove' }
  | { kind: 'pin' }
  | { kind: 'dissolve'; direction: DirectionDirective | null }
  | {
      kind: 'transform';
      form: FormType[];
      mass: MassChange | null;
      gravity: GravityChange | null;
      direction: DirectionDirective | null;
    };

/** A terraform region attached to a line range. */
export interface TerraformRegion {
  start_line: number;
  end_line: number;
  /** The transformation intent — type-safe combinations only. */
  intent: TerraformIntent;
}

/** Create a default empty transform intent. */
export function emptyTransformIntent(): TerraformIntent {
  return {
    kind: 'transform',
    form: [],
    mass: null,
    gravity: null,
    direction: null
  };
}

/** Check if an intent is effectively empty (no transformation requested). */
export function isIntentEmpty(intent: TerraformIntent): boolean {
  if (intent.kind !== 'transform') return false;
  return (
    intent.form.length === 0 &&
    intent.mass === null &&
    intent.gravity === null &&
    intent.direction === null
  );
}
