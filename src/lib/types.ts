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
