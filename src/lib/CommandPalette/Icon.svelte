<script lang="ts">
  import {
    SearchIcon,
    HashtagIcon,
    BookmarkIcon,
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
    ExcalidrawIcon,
    WarningIcon,
    ImageIcon,
    ClipboardIcon,
    PaperclipIcon,
    ChatBubbleIcon,
    HeadingH1Icon,
    HeadingH2Icon,
    HeadingH3Icon
  } from '$lib/icons';

  interface Props {
    name: string;
    class?: string;
  }

  let { name, class: className = '' }: Props = $props();

  const icons: Record<string, typeof SearchIcon> = {
    search: SearchIcon,
    hashtag: HashtagIcon,
    bookmark: BookmarkIcon,
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
    excalidraw: ExcalidrawIcon,
    warning: WarningIcon,
    image: ImageIcon,
    clipboard: ClipboardIcon,
    paperclip: PaperclipIcon,
    'chat-bubble': ChatBubbleIcon,
    'heading-h1': HeadingH1Icon,
    'heading-h2': HeadingH2Icon,
    'heading-h3': HeadingH3Icon
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
