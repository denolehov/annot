<script lang="ts">
  /**
   * SearchBar - Floating search bar for in-file search.
   *
   * Opens with Cmd+F, shows match count, navigates with Enter/Shift+Enter.
   */
  import type { SearchContext } from '$lib/composables/useSearch.svelte';
  import { SearchIcon, ChevronUpIcon, ChevronDownIcon, XMarkIcon } from '$lib/icons';

  interface Props {
    search: SearchContext;
  }

  let { search }: Props = $props();

  let inputEl: HTMLInputElement | null = $state(null);

  // Auto-focus input when search opens
  $effect(() => {
    if (search.isOpen && inputEl) {
      inputEl.focus();
      inputEl.select();
    }
  });

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      search.close();
      return;
    }

    if (e.key === 'Enter') {
      e.preventDefault();
      if (e.shiftKey) {
        search.prevMatch();
      } else {
        search.nextMatch();
      }
      return;
    }
  }

  function handleInput(e: Event) {
    const target = e.target as HTMLInputElement;
    search.setQuery(target.value);
  }
</script>

{#if search.isOpen}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div class="search-bar" role="search" onkeydown={handleKeyDown}>
    <SearchIcon class="search-icon" />

    <input
      bind:this={inputEl}
      type="text"
      class="search-input"
      placeholder="Search..."
      value={search.query}
      oninput={handleInput}
      aria-label="Search in file"
    />

    <span class="match-count">
      {#if search.query}
        {search.totalMatches > 0 ? search.currentMatchIndex + 1 : 0}/{search.totalMatches}
      {/if}
    </span>

    <div class="nav-buttons">
      <button
        class="nav-btn"
        onclick={() => search.prevMatch()}
        disabled={search.totalMatches === 0}
        title="Previous match (Shift+Enter)"
        aria-label="Previous match"
      >
        <ChevronUpIcon />
      </button>
      <button
        class="nav-btn"
        onclick={() => search.nextMatch()}
        disabled={search.totalMatches === 0}
        title="Next match (Enter)"
        aria-label="Next match"
      >
        <ChevronDownIcon />
      </button>
    </div>

    <button
      class="close-btn"
      onclick={() => search.close()}
      title="Close (Escape)"
      aria-label="Close search"
    >
      <XMarkIcon />
    </button>
  </div>
{/if}

<style>
  @import '../../styles/components/search-bar.css';
</style>
