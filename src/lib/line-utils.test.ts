import { describe, it, expect } from 'vitest';
import { extractCodeBlockContent } from './line-utils';
import type { Line } from './types';

// Helper to create a source line
function makeSourceLine(path: string, lineNum: number, content: string): Line {
  return {
    content,
    html: null,
    origin: { type: 'source', path, line: lineNum },
    semantics: { type: 'plain' },
  };
}

// Helper to create a portal content line
function makePortalLine(path: string, lineNum: number, content: string): Line {
  return {
    content,
    html: null,
    origin: { type: 'source', path, line: lineNum },
    semantics: { type: 'portal', kind: 'content' },
  };
}

describe('extractCodeBlockContent', () => {
  it('extracts content between fence lines', () => {
    const lines: Line[] = [
      makeSourceLine('doc.md', 1, '# Title'),
      makeSourceLine('doc.md', 2, '```mermaid'),
      makeSourceLine('doc.md', 3, 'graph TD'),
      makeSourceLine('doc.md', 4, '    A --> B'),
      makeSourceLine('doc.md', 5, '```'),
      makeSourceLine('doc.md', 6, 'Done'),
    ];

    const result = extractCodeBlockContent(lines, 2, 5, 'doc.md');
    expect(result).toBe('graph TD\n    A --> B');
  });

  it('excludes portal content with overlapping line numbers', () => {
    // Scenario: markdown file has a portal at line 2, then mermaid at lines 5-8
    // Portal expands to show lines 1-10 from main.rs
    // The portal lines have source line numbers that overlap with mermaid range
    const lines: Line[] = [
      makeSourceLine('doc.md', 1, '# Title'),
      makeSourceLine('doc.md', 2, '[portal](main.rs#L1-L10)'),
      // Portal content - these have line numbers 1-10 from main.rs
      makePortalLine('main.rs', 1, 'fn main() {'),
      makePortalLine('main.rs', 2, '    println!("hello");'),
      makePortalLine('main.rs', 3, '}'),
      makePortalLine('main.rs', 4, ''),
      makePortalLine('main.rs', 5, 'fn helper() {'),  // Line 5 overlaps!
      makePortalLine('main.rs', 6, '    // code'),    // Line 6 overlaps!
      makePortalLine('main.rs', 7, '}'),              // Line 7 overlaps!
      // Back to markdown
      makeSourceLine('doc.md', 3, ''),
      makeSourceLine('doc.md', 4, '## Diagram'),
      makeSourceLine('doc.md', 5, '```mermaid'),       // startLine
      makeSourceLine('doc.md', 6, 'graph TD'),         // content
      makeSourceLine('doc.md', 7, '    A --> B'),      // content
      makeSourceLine('doc.md', 8, '```'),              // endLine
      makeSourceLine('doc.md', 9, 'Done'),
    ];

    const result = extractCodeBlockContent(lines, 5, 8, 'doc.md');

    // Should only contain mermaid content, NOT the Rust code from portal
    expect(result).toBe('graph TD\n    A --> B');
    expect(result).not.toContain('fn helper');
    expect(result).not.toContain('// code');
  });

  it('returns empty string when no matching lines', () => {
    const lines: Line[] = [
      makeSourceLine('doc.md', 1, '```'),
      makeSourceLine('doc.md', 2, '```'),
    ];

    const result = extractCodeBlockContent(lines, 1, 2, 'doc.md');
    expect(result).toBe('');
  });
});
