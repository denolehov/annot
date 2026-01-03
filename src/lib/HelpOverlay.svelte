<script lang="ts">
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
        { keys: ['⇧', 'C'], description: 'Open session editor (global comment)' },
        { keys: ['/'], description: 'Insert tag (in editor)' },
      ]
    },
    {
      category: 'Bookmarks',
      items: [
        { keys: ['b'], description: 'Bookmark hovered line or selection' },
        { keys: ['⇧', 'B'], description: 'Bookmark entire session' },
        { keys: ['e'], description: 'Edit last created bookmark' },
      ]
    },
    {
      category: 'Navigation',
      items: [
        { keys: ['⌘', 'F'], description: 'Search content' },
        { keys: [':'], description: 'Open command palette' },
        { keys: ['Tab'], description: 'Cycle exit mode forward' },
        { keys: ['⇧', 'Tab'], description: 'Cycle exit mode backward' },
        { keys: ['⌥', 'Tab'], description: 'Open exit mode picker' },
      ]
    },
    {
      category: 'Selection',
      items: [
        { keys: ['⇧', 'drag'], description: 'Select line range' },
        { keys: ['Esc'], description: 'Cancel pending choice' },
      ]
    },
    {
      category: 'View',
      items: [
        { keys: ['⌘', '+'], description: 'Zoom in' },
        { keys: ['⌘', '-'], description: 'Zoom out' },
        { keys: ['⌘', '0'], description: 'Reset zoom' },
      ]
    },
    {
      category: 'File',
      items: [
        { keys: ['⌘', 'S'], description: 'Save to file' },
        { keys: ['⌘', 'W'], description: 'Save and close' },
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
    width: 780px;
    max-width: 90vw;
    max-height: 80vh;
    background: var(--bg-panel);
    border-radius: var(--radius-xl);
    padding: 20px;
    position: relative;
    box-shadow: var(--shadow-lg);
    display: flex;
    flex-direction: column;
  }

  .help-close {
    position: absolute;
    top: 12px;
    right: 12px;
    background: transparent;
    border: none;
    padding: 4px;
    cursor: pointer;
    color: var(--text-secondary);
    border-radius: var(--radius-sm);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .help-close:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .help-title {
    font-family: var(--font-ui);
    font-size: 16px;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0 0 16px 0;
  }

  .help-content {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 16px;
    overflow-y: auto;
    flex: 1;
  }

  .shortcut-section {
    min-width: 0;
  }

  .section-title {
    font-family: var(--font-ui);
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-tertiary);
    margin: 0 0 8px 0;
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
    gap: 12px;
    padding: 4px 0;
    font-size: 13px;
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
    gap: 2px;
    flex-shrink: 0;
  }

  .shortcut-keys kbd {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 20px;
    height: 20px;
    padding: 0 5px;
    font-family: var(--font-ui);
    font-size: 11px;
    font-weight: 500;
    background: var(--bg-main);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    color: var(--text-secondary);
  }

  .key-separator {
    font-size: 10px;
    color: var(--text-tertiary);
    margin: 0 1px;
  }

  .help-footer {
    margin-top: 16px;
    padding-top: 12px;
    border-top: 1px solid var(--border-subtle);
    text-align: center;
    font-family: var(--font-ui);
    font-size: 12px;
    color: var(--text-tertiary);
  }

  .help-footer kbd {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 18px;
    height: 18px;
    padding: 0 4px;
    font-family: var(--font-ui);
    font-size: 10px;
    font-weight: 500;
    background: var(--bg-main);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    color: var(--text-secondary);
    margin: 0 2px;
  }
</style>
