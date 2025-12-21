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

describe("+page.svelte", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders file content with line numbers", async () => {
    vi.mocked(invoke).mockResolvedValue({
      label: "test.rs",
      lines: [
        { number: 1, content: "fn main() {" },
        { number: 2, content: '    println!("hello");' },
        { number: 3, content: "}" },
      ],
      exit_modes: [],
      selected_exit_mode_id: null,
    });

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
    vi.mocked(invoke).mockResolvedValue({
      label: "my_module.rs",
      lines: [{ number: 1, content: "// comment" }],
      exit_modes: [],
      selected_exit_mode_id: null,
    });

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
    vi.mocked(invoke).mockResolvedValue({
      label: "test.rs",
      lines: [
        { number: 1, content: "fn main() {" },
        { number: 2, content: '    println!("hello");' },
      ],
      exit_modes: [],
      selected_exit_mode_id: null,
      tags: [],
    });

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
    vi.mocked(invoke).mockResolvedValue({
      label: "test.rs",
      lines: [
        { number: 1, content: "fn main() {" },
        { number: 2, content: '    println!("hello");' },
      ],
      exit_modes: [],
      selected_exit_mode_id: null,
      tags: [],
    });

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
