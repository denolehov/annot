import "@testing-library/jest-dom/vitest";
import { vi } from "vitest";

// Mock Tauri's invoke API for tests that render components using it
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue([]),
}));

// Tripwire: Node.normalize() deletes the empty text nodes Svelte uses as
// {@html}/branch effect anchors, so the next update renders into a detached
// anchor and the line goes blank (this shipped once — unfold turned every
// swatch-effect normalize() into blank rows). Fail any test that reaches for
// it; re-join specific splits with mergeTextSplit() from $lib/dom-text.
if (typeof Node !== 'undefined') {
  Node.prototype.normalize = function () {
    throw new Error(
      'Node.normalize() is banned on Svelte-managed DOM: it deletes empty text nodes, ' +
        'which Svelte uses as effect anchors. Use mergeTextSplit() from $lib/dom-text instead.',
    );
  };
}

// Mock document.elementFromPoint (not available in jsdom)
// Needed by TipTap's placeholder extension when calculating viewport positions
if (typeof document !== 'undefined' && !document.elementFromPoint) {
  // @ts-ignore - adding missing jsdom method
  document.elementFromPoint = vi.fn((x: number, y: number) => {
    return document.body;
  });
}
