<script lang="ts">
  /**
   * CopyButton - Reusable copy button with checkmark feedback.
   *
   * Shows a copy icon, switches to checkmark on click, reverts after delay.
   */
  import Icon from '$lib/CommandPalette/Icon.svelte';

  interface Props {
    /** Async function to perform the copy action */
    onCopy: () => Promise<void>;
    /** Title/tooltip text */
    title?: string;
    /** Only show on parent hover (requires parent to set .line:hover .copy-btn) */
    hoverOnly?: boolean;
    /** Additional CSS classes */
    class?: string;
  }

  let {
    onCopy,
    title = 'Copy',
    hoverOnly = false,
    class: className = '',
  }: Props = $props();

  let copied = $state(false);

  async function handleClick() {
    try {
      await onCopy();
      copied = true;
      setTimeout(() => (copied = false), 1500);
    } catch (err) {
      console.error('Copy failed:', err);
    }
  }
</script>

<button
  class="copy-btn {className}"
  class:copied
  class:hover-only={hoverOnly}
  onclick={handleClick}
  title={copied ? 'Copied!' : title}
>
  <Icon name={copied ? 'check' : 'copy-code'} />
</button>

<style>
  .copy-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 2px;
    background: transparent;
    border: none;
    border-radius: 4px;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 16px;
    transition: color 0.15s ease, background 0.15s ease, opacity 0.15s ease;
  }

  .copy-btn.hover-only {
    opacity: 0;
  }

  .copy-btn:hover {
    color: var(--text-secondary);
    background: var(--bg-hover);
  }

  .copy-btn.copied {
    color: var(--success, #22c55e);
  }

  .copy-btn:focus-visible {
    outline: 1px solid var(--focus-ring);
    outline-offset: 2px;
    opacity: 1;
  }
</style>
