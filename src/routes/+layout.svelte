<script lang="ts">
  import { onMount } from 'svelte';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { preloadExcalidraw } from '$lib/excalidraw-loader';
  import { preloadMermaid } from '$lib/mermaid-loader';
  import '../styles/index.css';

  onMount(() => {
    requestIdleCallback(() => {
      preloadExcalidraw();
      preloadMermaid();
    });

    // Intercept link clicks and open in default browser
    const handleClick = (e: MouseEvent) => {
      const target = (e.target as HTMLElement).closest('a');
      if (!target) return;

      const href = target.getAttribute('href');
      if (href && (href.startsWith('http://') || href.startsWith('https://'))) {
        e.preventDefault();
        openUrl(href);
      }
    };

    document.addEventListener('click', handleClick);
    return () => document.removeEventListener('click', handleClick);
  });
</script>

<slot />
