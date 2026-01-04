<script lang="ts">
  interface Props {
    class?: string;
  }
  let { class: className = '' }: Props = $props();
</script>

<svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="cp-icon {className}">
  <path stroke-linecap="round" stroke-linejoin="round" d="M8 6a4 4 0 018 0v1H8V6z" />
  <path stroke-linecap="round" stroke-linejoin="round" d="M6 7h12l2 11a2 2 0 01-2 2H6a2 2 0 01-2-2L6 7z" />
  <circle cx="12" cy="14" r="2" stroke-linecap="round" stroke-linejoin="round"/>
</svg>
