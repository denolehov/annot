export interface Line {
  number: number;
  content: string;
  /** Syntax-highlighted HTML with CSS classes, or null if unavailable */
  html: string | null;
}

export interface ContentResponse {
  label: string;
  lines: Line[];
}

// Content node types for structured annotation content (output format)
export type ContentNode = TextNode;

export interface TextNode {
  type: 'text';
  text: string;
}

// TipTap JSON content type for internal storage
export type { JSONContent } from '@tiptap/core';
