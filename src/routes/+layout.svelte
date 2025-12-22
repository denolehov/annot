<script lang="ts">
  import { onMount } from 'svelte';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { preloadExcalidraw } from '$lib/excalidraw-loader';
  import { preloadMermaid } from '$lib/mermaid-loader';
  import '../styles/index.css';

  onMount(() => {
    // requestIdleCallback doesn't exist in Tauri's WKWebView, use setTimeout fallback
    const scheduleIdle = window.requestIdleCallback ?? ((cb: () => void) => setTimeout(cb, 1));
    scheduleIdle(() => {
      preloadExcalidraw();
      preloadMermaid();
    });

    // Intercept link clicks and open in default browser
    const handleClick = async (e: MouseEvent) => {
      const target = (e.target as HTMLElement).closest('a');
      if (!target) return;

      const href = target.getAttribute('href');
      if (!href) return;

      // Always prevent navigation from content links
      e.preventDefault();
      e.stopPropagation();

      // Only open http/https URLs in browser (ignore relative paths, anchors, etc.)
      if (href.startsWith('http://') || href.startsWith('https://')) {
        try {
          await openUrl(href);
        } catch (err) {
          console.error('Failed to open link:', err);
        }
      }
    };

    document.addEventListener('click', handleClick, true); // capture phase
    return () => document.removeEventListener('click', handleClick, true);
  });
</script>

<slot />
