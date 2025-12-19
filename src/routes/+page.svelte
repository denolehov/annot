<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import type { ContentResponse, Line } from "$lib/types";

  let lines: Line[] = $state([]);
  let label = $state("");
  let error = $state("");

  onMount(async () => {
    try {
      const res = await invoke<ContentResponse>("get_content");
      label = res.label;
      lines = res.lines;
    } catch (e) {
      error = String(e);
    }
  });
</script>

<main class="viewer">
  <header class="header" data-tauri-drag-region>
    <div class="header-left">
      <span class="file-name">{label}</span>
    </div>
    <div class="header-right">
      <!-- Action buttons will go here -->
    </div>
  </header>

  {#if error}
    <div class="error">{error}</div>
  {:else if lines.length === 0}
    <div class="loading">Loading...</div>
  {:else}
    <div class="content">
      {#each lines as line}
        <div class="line">
          <span class="gutter">{line.number}</span>
          <span class="code">
            {#if line.html}
              {@html line.html}
            {:else}
              {line.content}
            {/if}
          </span>
        </div>
      {/each}
    </div>
  {/if}
</main>

<style>
  :global(*) {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
  }

  :global(body) {
    font-family: "JetBrains Mono", ui-monospace, "SF Mono", Menlo, Monaco, monospace;
    font-size: 12px;
    line-height: 22px;
    background-color: #fafaf9;
    color: #18181b;
    overflow: hidden;
  }

  .viewer {
    height: 100vh;
    display: flex;
    flex-direction: column;
  }

  /* Header with frosted glass effect */
  .header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    /* Left padding for traffic lights, top padding to align with them */
    padding: 10px 12px 10px 90px;
    flex-shrink: 0;
    border-bottom: 1px solid rgba(0, 0, 0, 0.06);
    background: color-mix(in srgb, #fafaf8 85%, transparent);
    backdrop-filter: blur(20px) saturate(180%);
    -webkit-backdrop-filter: blur(20px) saturate(180%);
    -webkit-app-region: drag;
    /* Soft shadow for depth */
    box-shadow:
      0 1px 3px rgba(0, 0, 0, 0.04),
      0 4px 12px rgba(0, 0, 0, 0.03);
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .file-name {
    color: #18181b;
    font-weight: 600;
    font-size: 13px;
    letter-spacing: -0.01em;
  }

  .header-right {
    display: flex;
    align-items: center;
    gap: 2px;
    -webkit-app-region: no-drag;
  }

  .content {
    flex: 1;
    overflow: auto;
    padding-bottom: 1rem;
  }

  .line {
    display: flex;
    white-space: pre;
    tab-size: 4;
  }

  .line:hover {
    background-color: #f4f4f5;
  }

  .gutter {
    width: 50px;
    flex-shrink: 0;
    padding-right: 12px;
    text-align: right;
    color: #71717a;
    -webkit-user-select: none;
    user-select: none;
    cursor: pointer;
    font-variant-numeric: tabular-nums;
    border-right: 1px solid #e4e4e7;
    /* Prevent gutter from being included in text selection */
    pointer-events: auto;
  }

  .gutter:hover {
    color: #52525b;
  }

  .code {
    flex: 1;
    padding-left: 16px;
    -webkit-user-select: text;
    user-select: text;
  }

  .error {
    padding: 2rem;
    color: #dc2626;
  }

  .loading {
    padding: 2rem;
    color: #71717a;
  }

  /* Scrollbar styling */
  .content::-webkit-scrollbar {
    width: 6px;
    height: 6px;
  }

  .content::-webkit-scrollbar-thumb {
    background-color: #d4d4d8;
    border-radius: 99px;
  }

  .content::-webkit-scrollbar-thumb:hover {
    background-color: #a1a1aa;
  }

  .content::-webkit-scrollbar-track {
    background: transparent;
  }

  /* ===========================================
     Design Tokens (GitHub Light theme)
     =========================================== */
  :global(:root) {
    --text-primary: #18181b;
    --text-secondary: #71717a;
    --code-keyword: #d73a49;
    --code-func: #6f42c1;
    --code-string: #032f62;
    --code-comment: #6a737d;
    --code-constant: #005cc5;
    --code-variable: #e36209;
    --code-tag: #22863a;
    --code-op: #d73a49;
    --code-support: #6f42c1;
  }

  /* ===========================================
     Syntax Highlighting (syntect CSS classes)

     syntect uses semantic class names like:
       <span class="storage type function rust">fn</span>
       <span class="string quoted double rust">"hello"</span>
       <span class="comment line double-slash rust">// comment</span>

     Reference: src-tauri/src/highlight.rs::documents_html_structure_and_classes
     =========================================== */

  /* Comments */
  :global(.comment) { color: var(--code-comment); font-style: italic; }

  /* Storage (keywords like fn, let, const, function, var) */
  :global(.storage) { color: var(--code-keyword); }

  /* Keywords (control flow: return, if, else, for, while) */
  :global(.keyword) { color: var(--code-keyword); }

  /* Strings */
  :global(.string) { color: var(--code-string); }

  /* Constants (numbers, booleans, null) */
  :global(.constant) { color: var(--code-constant); }

  /* Entity names (function names, class names, type names) */
  :global(.entity.name) { color: var(--code-func); }
  :global(.entity.name.function) { color: var(--code-func); }
  :global(.entity.name.type) { color: var(--code-func); }
  :global(.entity.name.class) { color: var(--code-func); }

  /* Variables */
  :global(.variable) { color: var(--text-primary); }
  :global(.variable.parameter) { color: var(--code-variable); }
  :global(.variable.other) { color: var(--text-primary); }

  /* Support (macros, built-ins) */
  :global(.support) { color: var(--code-support); }
  :global(.support.macro) { color: var(--code-support); }
  :global(.support.function) { color: var(--code-support); }

  /* Punctuation - keep neutral */
  :global(.punctuation) { color: var(--text-primary); }

  /* Meta - generally inherit, but provide fallback */
  :global(.meta) { color: inherit; }

  /* Source - base scope, inherit color */
  :global(.source) { color: var(--text-primary); }
</style>
