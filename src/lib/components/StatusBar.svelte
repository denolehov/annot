<script lang="ts">
  import type { ExitMode } from '$lib/types';

  interface Props {
    selectedMode: ExitMode | null;
    onCycleMode: () => void;
  }

  let { selectedMode, onCycleMode }: Props = $props();
</script>

<footer class="status-bar" style:--mode-color={selectedMode?.color ?? 'transparent'}>
  <div class="status-bar-left">
    <button
      class="exit-mode-btn"
      class:neutral={!selectedMode}
      onclick={onCycleMode}
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
    <span class="kbd-hint"><kbd>g</kbd> global note</span>
    <span class="kbd-hint"><kbd>⌘w</kbd> save and close</span>
  </div>
</footer>
