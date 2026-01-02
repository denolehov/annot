<script lang="ts">
  import type { Bookmark } from '$lib/types';

  interface Props {
    bookmark: Bookmark;
    labelValue: string;
    focusedField: number;
    pendingDelete?: boolean;
  }

  let { bookmark, labelValue, focusedField, pendingDelete = false }: Props = $props();

  // Format date for display
  function formatDate(isoString: string): string {
    const date = new Date(isoString);
    return date.toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric',
      year: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  }

  // Format source type for display
  function formatSourceType(type: string): string {
    return type.charAt(0).toUpperCase() + type.slice(1);
  }

  // Get all context lines
  let contextLines = $derived(bookmark.snapshot.context.split('\n'));

  // Get selection lines (if selection bookmark)
  let selectionLines = $derived.by(() => {
    if (bookmark.snapshot.type === 'selection') {
      return bookmark.snapshot.selected_text.split('\n');
    }
    return [];
  });
</script>

<div class="form-view" class:pending-delete={pendingDelete}>
  <form>
    <!-- Label field -->
    <div class="field" class:focused={focusedField === 0}>
      <label for="label">Label</label>
      <input
        type="text"
        id="label"
        name="label"
        value={labelValue}
        placeholder="Enter a label..."
      />
    </div>

    <!-- Metadata section -->
    <div class="metadata-section">
    <div class="metadata-row">
      <span class="metadata-label">Source</span>
      <span class="metadata-value">{bookmark.snapshot.source_title} ({formatSourceType(bookmark.snapshot.source_type)})</span>
    </div>
    <div class="metadata-row">
      <span class="metadata-label">Created</span>
      <span class="metadata-value">{formatDate(bookmark.created_at)}</span>
    </div>
    {#if bookmark.project_path}
      <div class="metadata-row">
        <span class="metadata-label">Project</span>
        <span class="metadata-value project-path">{bookmark.project_path}</span>
      </div>
    {/if}
  </div>

  <!-- Selection box (only for selection bookmarks) -->
  {#if bookmark.snapshot.type === 'selection'}
    <div class="snapshot-box">
      <div class="snapshot-header">Selection</div>
      <div class="snapshot-content">
        {#each selectionLines as line}
          <div class="snapshot-line">{line || ' '}</div>
        {/each}
      </div>
    </div>
  {/if}

  <!-- Context box -->
  <div class="snapshot-box">
    <div class="snapshot-header">Context</div>
    <div class="snapshot-content">
      {#each contextLines as line}
        <div class="snapshot-line">{line || ' '}</div>
      {/each}
    </div>
  </div>
  </form>
</div>

<style>
  /* Styles are in src/styles/components/command-palette.css */
</style>
