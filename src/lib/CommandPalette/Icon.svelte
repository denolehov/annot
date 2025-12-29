<script lang="ts">
  import {
    SearchIcon,
    HashtagIcon,
    ExitIcon,
    CopyIcon,
    SaveIcon,
    ReorderIcon,
    PlusIcon,
    EditIcon,
    ObsidianIcon,
    CopyCodeIcon,
    CheckIcon,
    ViewFinderIcon,
    SunIcon,
    MoonIcon,
    ExcalidrawIcon
  } from '$lib/icons';

  interface Props {
    name: string;
    class?: string;
  }

  let { name, class: className = '' }: Props = $props();

  const icons: Record<string, typeof SearchIcon> = {
    search: SearchIcon,
    hashtag: HashtagIcon,
    exit: ExitIcon,
    copy: CopyIcon,
    save: SaveIcon,
    reorder: ReorderIcon,
    plus: PlusIcon,
    edit: EditIcon,
    obsidian: ObsidianIcon,
    'copy-code': CopyCodeIcon,
    check: CheckIcon,
    'view-finder': ViewFinderIcon,
    sun: SunIcon,
    moon: MoonIcon,
    excalidraw: ExcalidrawIcon
  };

  const IconComponent = $derived(icons[name]);
</script>

{#if IconComponent}
  <IconComponent class={className} />
{:else}
  <span class="icon-fallback">{name}</span>
{/if}

<style>
  .icon-fallback {
    font-size: 1em;
  }
</style>
