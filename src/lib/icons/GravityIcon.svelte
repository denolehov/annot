<script lang="ts">
  interface Props {
    class?: string;
  }
  let { class: className = '' }: Props = $props();
</script>

<svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="cp-icon {className}">
  <path stroke-linecap="round" stroke-linejoin="round" d="M12 7a3 3 0 100 6 3 3 0 000-6z" />
  <path stroke-linecap="round" stroke-linejoin="round" d="M12 13v8m0 0l-2-2m2 2l2-2M5 21h14" />
</svg>
