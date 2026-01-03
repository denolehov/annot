import "@testing-library/jest-dom/vitest";
import { vi } from "vitest";

// Mock Tauri's invoke API for tests that render components using it
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue([]),
}));
