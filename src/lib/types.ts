export interface Line {
  number: number;
  content: string;
}

export interface ContentResponse {
  label: string;
  lines: Line[];
}
