import { describe, expect, it } from "vitest";
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative } from "node:path";

/**
 * Content zoom scales lengths through --content-zoom (see tokens.css), so a
 * bare `font-size: Npx` silently ignores zoom — it looks correct at 100% and
 * wrong at every other level. This test makes that failure loud.
 *
 * Reads files with node:fs (not import.meta.glob: eager ?raw globs are
 * inlined at transform time and served from Vite's cache, so edits to the
 * scanned files don't reliably re-run through the test).
 *
 * Rules:
 * - Content-plane text uses `var(--fs-N)` (tokens.css) or an explicit
 *   `calc(Npx * var(--content-zoom, 1))`.
 * - Fixed chrome (titlebar, SaveModal, toasts) may use bare px, but the
 *   declaration must carry an `unscaled` comment on the same line.
 * - The excalidraw/mermaid routes are separate windows that never set
 *   --content-zoom; they are exempt wholesale.
 */

const SRC_ROOT = join(process.cwd(), "src");

const EXEMPT_DIRS = ["routes/excalidraw", "routes/mermaid"];

const BARE_PX_FONT_SIZE = /font-size:\s*[0-9]+(?:\.[0-9]+)?px/;

function walk(dir: string, files: string[] = []): string[] {
  for (const entry of readdirSync(dir)) {
    const path = join(dir, entry);
    if (statSync(path).isDirectory()) {
      walk(path, files);
    } else if (/\.(css|svelte)$/.test(entry)) {
      files.push(path);
    }
  }
  return files;
}

describe("font-size scaling", () => {
  it("every bare px font-size is either scaled or marked unscaled", () => {
    const violations: string[] = [];

    for (const file of walk(SRC_ROOT)) {
      const rel = relative(SRC_ROOT, file).replaceAll("\\", "/");
      if (EXEMPT_DIRS.some((dir) => rel.startsWith(dir))) continue;

      const lines = readFileSync(file, "utf8").split("\n");
      lines.forEach((line, i) => {
        if (BARE_PX_FONT_SIZE.test(line) && !line.includes("unscaled")) {
          violations.push(`src/${rel}:${i + 1}: ${line.trim()}`);
        }
      });
    }

    expect(
      violations,
      `Bare px font-size ignores --content-zoom. Use var(--fs-N) from ` +
        `tokens.css (or calc(Npx * var(--content-zoom, 1))); if this is ` +
        `fixed chrome, add an /* unscaled */ comment on the line.\n` +
        violations.join("\n"),
    ).toEqual([]);
  });
});
