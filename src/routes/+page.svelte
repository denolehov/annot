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
          <span class="code">{line.content}</span>
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
    border-bottom: 1px solid #e4e4e7;
    background: color-mix(in srgb, #fafaf8 70%, transparent);
    backdrop-filter: blur(16px) saturate(150%);
    -webkit-backdrop-filter: blur(16px) saturate(150%);
    -webkit-app-region: drag;
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
    user-select: none;
    cursor: pointer;
    font-variant-numeric: tabular-nums;
    border-right: 1px solid #e4e4e7;
  }

  .gutter:hover {
    color: #52525b;
  }

  .code {
    flex: 1;
    padding-left: 16px;
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
</style>
