import "@testing-library/jest-dom/vitest";
import { vi } from "vitest";

// Mock Tauri's invoke API for tests that render components using it
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue([]),
}));

// Mock document.elementFromPoint (not available in jsdom)
// Needed by TipTap's placeholder extension when calculating viewport positions
if (typeof document !== 'undefined' && !document.elementFromPoint) {
  // @ts-ignore - adding missing jsdom method
  document.elementFromPoint = vi.fn((x: number, y: number) => {
    return document.body;
  });
}
