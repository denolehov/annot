<script lang="ts">
  import { keys } from '$lib/keys';

  interface Props {
    onClose: () => void;
  }

  let { onClose }: Props = $props();

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) {
      onClose();
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape' || e.key === '?') {
      e.preventDefault();
      onClose();
    }
  }

  const shortcuts = [
    {
      category: 'Annotation',
      items: [
        { keys: ['c'], description: 'Comment hovered line' },
        { keys: [keys.shift, 'C'], description: 'Open session editor (global comment)' },
      ]
    },
    {
      category: 'Editor',
      items: [
        { keys: ['#'], description: 'Insert tag' },
        { keys: ['@'], description: 'Reference (annotations, sections, files)' },
        { keys: ['/'], description: 'Slash commands (/replace, /excalidraw)' },
      ]
    },
    {
      category: 'Navigation',
      items: [
        { keys: [keys.cmd, 'F'], description: 'Search content' },
        { keys: [':'], description: 'Open command palette' },
        { keys: ['Tab'], description: 'Cycle exit mode forward' },
        { keys: [keys.shift, 'Tab'], description: 'Cycle exit mode backward' },
        { keys: [keys.alt, 'Tab'], description: 'Open exit mode picker' },
      ]
    },
    {
      category: 'Selection',
      items: [
        { keys: [keys.shift, 'drag'], description: 'Select line range' },
      ]
    },
    {
      category: 'View',
      items: [
        { keys: [keys.cmd, 'B'], description: 'Toggle file tree (diffs)' },
        { keys: [keys.cmd, '+'], description: 'Zoom in' },
        { keys: [keys.cmd, '-'], description: 'Zoom out' },
        { keys: [keys.cmd, '0'], description: 'Reset zoom' },
      ]
    },
    {
      category: 'File',
      items: [
        { keys: [keys.cmd, 'S'], description: 'Save to file' },
        { keys: [keys.cmd, 'W'], description: 'Save and close' },
      ]
    },
  ];
</script>

<svelte:window onkeydown={handleKeyDown} />

<!-- svelte-ignore a11y_click_events_have_key_events -->
<div class="help-backdrop" onclick={handleBackdropClick} role="presentation">
  <div class="help-modal" role="dialog" aria-modal="true">
    <button class="help-close" onclick={onClose} title="Close (Escape)">
      <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <line x1="18" y1="6" x2="6" y2="18"></line>
        <line x1="6" y1="6" x2="18" y2="18"></line>
      </svg>
    </button>

    <h3 class="help-title">Keyboard Shortcuts</h3>

    <div class="help-content">
      {#each shortcuts as { category, items }}
        <section class="shortcut-section">
          <h4 class="section-title">{category}</h4>
          <ul class="shortcut-list">
            {#each items as { keys, description }}
              <li class="shortcut-item">
                <span class="shortcut-desc">{description}</span>
                <span class="shortcut-keys">
                  {#each keys as key, i}
                    <kbd>{key}</kbd>{#if i < keys.length - 1}<span class="key-separator">+</span>{/if}
                  {/each}
                </span>
              </li>
            {/each}
          </ul>
        </section>
      {/each}
    </div>

    <div class="help-footer">
      Press <kbd>?</kbd> or <kbd>Esc</kbd> to close
    </div>
  </div>
</div>

<style>
  .help-backdrop {
    position: fixed;
    inset: 0;
    background: var(--backdrop-dark);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .help-modal {
    width: calc(780px * var(--content-zoom, 1));
    max-width: 90vw;
    max-height: 80vh;
    background: var(--bg-panel);
    border-radius: calc(var(--radius-xl) * var(--content-zoom, 1));
    padding: calc(20px * var(--content-zoom, 1));
    position: relative;
    box-shadow: var(--shadow-lg);
    display: flex;
    flex-direction: column;
  }

  .help-close {
    position: absolute;
    top: calc(12px * var(--content-zoom, 1));
    right: calc(12px * var(--content-zoom, 1));
    background: transparent;
    border: none;
    padding: calc(4px * var(--content-zoom, 1));
    cursor: pointer;
    color: var(--text-secondary);
    border-radius: calc(var(--radius-sm) * var(--content-zoom, 1));
    display: flex;
    align-items: center;
    justify-content: center;
  }

  /* Overrides the inline width/height="16" attrs on the SVG (CSS beats
     presentational HTML attributes) so the icon tracks --content-zoom. */
  .help-close svg {
    width: calc(16px * var(--content-zoom, 1));
    height: calc(16px * var(--content-zoom, 1));
  }

  .help-close:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .help-title {
    font-family: var(--font-ui);
    font-size: var(--fs-16);
    font-weight: 600;
    color: var(--text-primary);
    margin: 0 0 calc(16px * var(--content-zoom, 1)) 0;
  }

  .help-content {
    column-count: 2;
    column-gap: calc(24px * var(--content-zoom, 1));
    overflow-y: auto;
    flex: 1;
    padding-right: calc(8px * var(--content-zoom, 1));
  }

  .shortcut-section {
    break-inside: avoid;
    margin-bottom: calc(16px * var(--content-zoom, 1));
  }

  .section-title {
    font-family: var(--font-ui);
    font-size: var(--fs-11);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-tertiary);
    margin: 0 0 calc(8px * var(--content-zoom, 1)) 0;
  }

  .shortcut-list {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .shortcut-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: calc(12px * var(--content-zoom, 1));
    padding: calc(4px * var(--content-zoom, 1)) 0;
    font-size: var(--fs-13);
  }

  .shortcut-desc {
    color: var(--text-secondary);
    font-family: var(--font-ui);
    text-align: left;
    flex: 1;
    min-width: 0;
  }

  .shortcut-keys {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: calc(2px * var(--content-zoom, 1));
    flex-shrink: 0;
  }

  .shortcut-keys kbd {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: calc(20px * var(--content-zoom, 1));
    height: calc(20px * var(--content-zoom, 1));
    padding: 0 calc(5px * var(--content-zoom, 1));
    font-family: var(--font-ui);
    font-size: var(--fs-11);
    font-weight: 500;
    background: var(--bg-main);
    border: 1px solid var(--border-subtle);
    border-radius: calc(var(--radius-sm) * var(--content-zoom, 1));
    color: var(--text-secondary);
  }

  .key-separator {
    font-size: var(--fs-10);
    color: var(--text-tertiary);
    margin: 0 1px;
  }

  .help-footer {
    margin-top: calc(16px * var(--content-zoom, 1));
    padding-top: calc(12px * var(--content-zoom, 1));
    border-top: 1px solid var(--border-subtle);
    text-align: center;
    font-family: var(--font-ui);
    font-size: var(--fs-12);
    color: var(--text-tertiary);
  }

  .help-footer kbd {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: calc(18px * var(--content-zoom, 1));
    height: calc(18px * var(--content-zoom, 1));
    padding: 0 calc(4px * var(--content-zoom, 1));
    font-family: var(--font-ui);
    font-size: var(--fs-10);
    font-weight: 500;
    background: var(--bg-main);
    border: 1px solid var(--border-subtle);
    border-radius: calc(var(--radius-sm) * var(--content-zoom, 1));
    color: var(--text-secondary);
    margin: 0 2px;
  }
</style>
