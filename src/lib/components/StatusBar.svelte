<script lang="ts">
  import { getAnnotContext } from '$lib/context';

  const ctx = getAnnotContext();

  // Derived for template convenience
  const selectedMode = $derived(ctx.exitModes.selectedMode);
</script>

<footer class="status-bar" style:--mode-color={selectedMode?.color ?? 'transparent'}>
  <div class="status-bar-left">
    <button
      class="exit-mode-btn"
      class:neutral={!selectedMode}
      onclick={ctx.exitModes.cycleForward}
      title={selectedMode ? `${selectedMode.name}: ${selectedMode.instruction}` : undefined}
    >
      <kbd>Tab</kbd>
      <span class="exit-mode-label">
        {#if selectedMode}
          {selectedMode.name}
          <span class="exit-mode-instruction">({selectedMode.instruction})</span>
        {:else}
          set exit mode
        {/if}
      </span>
    </button>
  </div>
  <div class="status-bar-right">
    <span class="kbd-hint"><kbd>:</kbd> command palette</span>
    <span class="kbd-hint"><kbd>c</kbd> <kbd>⇧c</kbd> annotate</span>
    <span class="kbd-hint"><kbd>b</kbd> <kbd>⇧b</kbd> bookmark</span>
    <span class="kbd-hint"><kbd>⌘w</kbd> save and close</span>
  </div>
</footer>
