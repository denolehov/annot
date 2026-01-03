<script lang="ts">
  /**
   * ChoiceButtons - Shows annotate/bookmark/terraform choice after shift-drag-release.
   * Appears when pendingChoice is true after a shift-drag selection without modifier.
   */
  interface Props {
    onAnnotate: () => void;
    onBookmark: () => void;
    onTerraform?: () => void;
  }

  let { onAnnotate, onBookmark, onTerraform }: Props = $props();
</script>

<div class="choice-buttons">
  <button class="choice-button" onclick={onAnnotate}>
    <kbd>c</kbd>
    <span>annotate</span>
  </button>
  <button class="choice-button bookmark" onclick={onBookmark}>
    <kbd>b</kbd>
    <span>bookmark</span>
  </button>
  {#if onTerraform}
    <button class="choice-button terraform" onclick={onTerraform}>
      <kbd>t</kbd>
      <span>terraform</span>
    </button>
  {/if}
</div>
