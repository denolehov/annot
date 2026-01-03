<script lang="ts">
  /**
   * ChoiceButtons - Shows annotate/bookmark choice after shift-drag-release.
   * Appears when pendingChoice is true after a shift-drag selection without modifier.
   */
  interface Props {
    onAnnotate: () => void;
    onBookmark: () => void;
  }

  let { onAnnotate, onBookmark }: Props = $props();
</script>

<div class="choice-buttons">
  <button class="choice-button" onclick={onAnnotate}>
    <kbd>c</kbd>
    <span>annotate</span>
  </button>
  <button class="choice-button" onclick={onBookmark}>
    <kbd>b</kbd>
    <span>bookmark</span>
  </button>
</div>
