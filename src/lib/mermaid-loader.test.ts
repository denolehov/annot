import { describe, it, expect } from 'vitest';
import { isMermaidExcalidrawSupported } from './mermaid-loader';

describe('isMermaidExcalidrawSupported', () => {
  describe('supported diagram types', () => {
    it('returns true for flowchart LR', () => {
      expect(isMermaidExcalidrawSupported('flowchart LR\n    A --> B')).toBe(true);
    });

    it('returns true for flowchart TD', () => {
      expect(isMermaidExcalidrawSupported('flowchart TD\n    A --> B')).toBe(true);
    });

    it('returns true for graph TD', () => {
      expect(isMermaidExcalidrawSupported('graph TD\n    A --> B')).toBe(true);
    });

    it('returns true for graph LR', () => {
      expect(isMermaidExcalidrawSupported('graph LR\n    A --> B')).toBe(true);
    });

    it('returns true for sequenceDiagram', () => {
      expect(isMermaidExcalidrawSupported('sequenceDiagram\n    participant A')).toBe(true);
    });

    it('returns true for classDiagram', () => {
      expect(isMermaidExcalidrawSupported('classDiagram\n    class Animal')).toBe(true);
    });
  });

  describe('unsupported diagram types', () => {
    it('returns false for stateDiagram-v2', () => {
      expect(isMermaidExcalidrawSupported('stateDiagram-v2\n    [*] --> Idle')).toBe(false);
    });

    it('returns false for stateDiagram', () => {
      expect(isMermaidExcalidrawSupported('stateDiagram\n    [*] --> Idle')).toBe(false);
    });

    it('returns false for gantt', () => {
      expect(isMermaidExcalidrawSupported('gantt\n    title Project')).toBe(false);
    });

    it('returns false for erDiagram', () => {
      expect(isMermaidExcalidrawSupported('erDiagram\n    USER ||--o{ ORDER')).toBe(false);
    });

    it('returns false for pie', () => {
      expect(isMermaidExcalidrawSupported('pie\n    "Cats" : 50')).toBe(false);
    });

    it('returns false for journey', () => {
      expect(isMermaidExcalidrawSupported('journey\n    title My journey')).toBe(false);
    });

    it('returns false for gitGraph', () => {
      expect(isMermaidExcalidrawSupported('gitGraph\n    commit')).toBe(false);
    });

    it('returns false for mindmap', () => {
      expect(isMermaidExcalidrawSupported('mindmap\n    Root')).toBe(false);
    });
  });

  describe('edge cases', () => {
    it('handles leading whitespace', () => {
      expect(isMermaidExcalidrawSupported('  flowchart LR\n    A --> B')).toBe(true);
    });

    it('handles leading newlines', () => {
      expect(isMermaidExcalidrawSupported('\n\nflowchart LR\n    A --> B')).toBe(true);
    });

    it('returns false for empty string', () => {
      expect(isMermaidExcalidrawSupported('')).toBe(false);
    });

    it('returns false for whitespace only', () => {
      expect(isMermaidExcalidrawSupported('   \n  ')).toBe(false);
    });

    it('does not match partial keywords', () => {
      // flowchartX should not match "flowchart"
      expect(isMermaidExcalidrawSupported('flowchartX TD')).toBe(false);
      expect(isMermaidExcalidrawSupported('classDiagramX')).toBe(false);
    });
  });

  describe('leading comments and directives', () => {
    it('handles leading comment', () => {
      const source = `%% This is a flowchart
flowchart LR
    A --> B`;
      expect(isMermaidExcalidrawSupported(source)).toBe(true);
    });

    it('handles leading directive', () => {
      const source = `%%{init: {"theme": "dark"}}%%
flowchart LR
    A --> B`;
      expect(isMermaidExcalidrawSupported(source)).toBe(true);
    });

    it('handles multiple leading comments', () => {
      const source = `%% Comment 1
%% Comment 2
sequenceDiagram
    participant A`;
      expect(isMermaidExcalidrawSupported(source)).toBe(true);
    });

    it('handles directive followed by comment', () => {
      const source = `%%{init: {"theme": "forest"}}%%
%% Description of the diagram
classDiagram
    class Animal`;
      expect(isMermaidExcalidrawSupported(source)).toBe(true);
    });

    it('handles comments with unsupported diagram (still unsupported)', () => {
      const source = `%% This is a state diagram
stateDiagram-v2
    [*] --> Idle`;
      expect(isMermaidExcalidrawSupported(source)).toBe(false);
    });

    it('handles whitespace between comments and diagram type', () => {
      const source = `%% Comment

graph TD
    A --> B`;
      expect(isMermaidExcalidrawSupported(source)).toBe(true);
    });
  });
});
