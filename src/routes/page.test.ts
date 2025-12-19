import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/svelte";
import Page from "./+page.svelte";

// Mock @tauri-apps/api/core
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
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
});
