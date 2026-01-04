<script lang="ts">
  interface Props {
    class?: string;
  }
  let { class: className = '' }: Props = $props();
</script>

<svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="cp-icon {className}">
  <path stroke-linecap="round" stroke-linejoin="round" d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
  <path stroke-linecap="round" stroke-linejoin="round" d="M16.2 7.8L14.1 14.1L7.8 16.2L9.9 9.9z" />
  <path stroke-linecap="round" stroke-linejoin="round" d="M14.1 14.1L9.9 9.9" />
</svg>
