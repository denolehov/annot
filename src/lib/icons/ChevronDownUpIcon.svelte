<script lang="ts">
  interface Props {
    class?: string;
  }
  let { class: className = '' }: Props = $props();
</script>

<!-- Inward-pointing counterpart of ChevronUpDownIcon: fold/collapse all -->
<svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="cp-icon {className}">
  <path stroke-linecap="round" stroke-linejoin="round" d="M8.25 5.25 12 9l3.75-3.75M8.25 18.75 12 15l3.75 3.75" />
</svg>
