import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor, fireEvent } from "@testing-library/svelte";
import Page from "./+page.svelte";

// Mock @tauri-apps/api/core
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

// Mock @tauri-apps/api/window
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: vi.fn(() => ({
    onCloseRequested: vi.fn(() => Promise.resolve()),
    show: vi.fn(() => Promise.resolve()),
  })),
}));

import { invoke } from "@tauri-apps/api/core";

// Helper to create a line with the new model
function makeLine(num: number, content: string, html: string | null = null) {
  return {
    content,
    html,
    origin: { type: 'source' as const, path: 'test.rs', line: num },
    semantics: { type: 'plain' as const },
  };
}

// Helper to create a valid mock response
function createMockResponse(overrides: Partial<{
  label: string;
  lines: ReturnType<typeof makeLine>[];
  exit_modes: [];
  selected_exit_mode_id: string | null;
  tags: [];
}> = {}) {
  return {
    label: "test.rs",
    lines: [makeLine(1, "// comment")],
    exit_modes: [],
    selected_exit_mode_id: null,
    tags: [],
    session_comment: null,
    metadata: { type: 'plain' as const },
    allows_image_paste: false,
    ...overrides,
  };
}

describe("+page.svelte", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders file content with line numbers", async () => {
    vi.mocked(invoke).mockResolvedValue(createMockResponse({
      label: "test.rs",
      lines: [
        makeLine(1, "fn main() {"),
        makeLine(2, '    println!("hello");'),
        makeLine(3, "}"),
      ],
    }));

    render(Page);

    await waitFor(() => {
      expect(screen.getByText("test.rs")).toBeInTheDocument();
      expect(screen.getByText("1")).toBeInTheDocument();
      expect(screen.getByText("2")).toBeInTheDocument();
      expect(screen.getByText("3")).toBeInTheDocument();
      expect(screen.getByText("fn main() {")).toBeInTheDocument();
    });
  });

  it("displays filename in header", async () => {
    vi.mocked(invoke).mockResolvedValue(createMockResponse({
      label: "my_module.rs",
      lines: [makeLine(1, "// comment")],
    }));

    render(Page);

    await waitFor(() => {
      expect(screen.getByText("my_module.rs")).toBeInTheDocument();
    });
  });

  it("shows loading state initially", () => {
    vi.mocked(invoke).mockImplementation(() => new Promise(() => {})); // never resolves

    render(Page);

    expect(screen.getByText("Loading...")).toBeInTheDocument();
  });

  it("shows error when IPC fails", async () => {
    vi.mocked(invoke).mockRejectedValue(new Error("IPC failed"));

    render(Page);

    await waitFor(() => {
      expect(screen.getByText("Error: IPC failed")).toBeInTheDocument();
    });
  });

  it("does not open editor when Cmd+C is pressed (allows copy)", async () => {
    vi.mocked(invoke).mockResolvedValue(createMockResponse({
      label: "test.rs",
      lines: [
        makeLine(1, "fn main() {"),
        makeLine(2, '    println!("hello");'),
      ],
    }));

    render(Page);

    await waitFor(() => {
      expect(screen.getByText("fn main() {")).toBeInTheDocument();
    });

    // Simulate hovering over a line (set hoveredLine)
    const line1 = screen.getByText("fn main() {").closest('.line');
    if (line1) {
      await fireEvent.mouseEnter(line1);
    }

    // Press Cmd+C (should NOT open editor - should let browser handle copy)
    const event = new KeyboardEvent('keydown', {
      key: 'c',
      metaKey: true,
      bubbles: true,
    });
    const prevented = !window.dispatchEvent(event);

    // Cmd+C should NOT be prevented (browser handles copy)
    expect(prevented).toBe(false);

    // Verify no annotation editor appeared
    expect(screen.queryByText('Type annotation…')).not.toBeInTheDocument();
  });

  it("opens editor when 'c' is pressed alone on hovered line", async () => {
    vi.mocked(invoke).mockResolvedValue(createMockResponse({
      label: "test.rs",
      lines: [
        makeLine(1, "fn main() {"),
        makeLine(2, '    println!("hello");'),
      ],
    }));

    render(Page);

    await waitFor(() => {
      expect(screen.getByText("fn main() {")).toBeInTheDocument();
    });

    // Simulate hovering over a line
    const line1 = screen.getByText("fn main() {").closest('.line');
    if (line1) {
      await fireEvent.mouseEnter(line1);
    }

    // Press 'c' alone (should open editor)
    await fireEvent.keyDown(window, { key: 'c' });

    // Verify annotation editor appeared (look for toolbar hints)
    await waitFor(() => {
      // The toolbar shows "⌘↵ done" when editor is open and not sealed
      expect(screen.getByText('done')).toBeInTheDocument();
    });
  });
});
