<script lang="ts">
  import { onMount } from 'svelte';
  import { preloadExcalidraw } from '$lib/excalidraw-loader';
  import { preloadMermaid } from '$lib/mermaid-loader';
  import '../styles/index.css';

  onMount(() => {
    requestIdleCallback(() => {
      preloadExcalidraw();
      preloadMermaid();
    });
  });
</script>

<slot />
