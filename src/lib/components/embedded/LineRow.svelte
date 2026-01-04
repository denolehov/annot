<script lang="ts">
  /**
   * LineRow - Shared line-rendering component for embedded content.
   *
   * Handles common concerns across Portal, CodeBlock, and RegularLines:
   * - Selection, annotation, and preview state
   * - Mouse/pointer event handlers
   * - Bookmark indicator
   * - data-display-idx attribute
   *
   * ⚠️ SYNC WARNING: Table.svelte uses <tr>/<td> structure instead of <div>/<span>,
   * so it cannot use this component. When modifying LineRow, check if Table.svelte
   * needs equivalent changes (especially for: selection state, bookmark support,
   * event handlers, new CSS classes).
   */
  import type { Snippet } from 'svelte';
  import type { Line } from '$lib/types';
  import { getAnnotContext } from '$lib/context';
  import { BookmarkIcon, TerraformIcon } from '$lib/icons';
  import { getLineNumber, getFilePath } from '$lib/line-utils';
  import { tooltip } from '$lib/actions/tooltip';
  import ChoiceButtons from '$lib/components/ChoiceButtons.svelte';
  import TerraformPalette from '$lib/components/TerraformPalette.svelte';
  import { isIntentEmpty, type TerraformRegion } from '$lib/types';

  interface Props {
    line: Line;
    displayIndex: number;
    isBookmarked?: boolean;
    showBookmarkIcon?: boolean;
    onDeleteBookmark?: () => void;
    additionalClasses?: Record<string, boolean>;
    gutterClass?: string;
    gutter: Snippet<[]>;
    code: Snippet<[]>;
    trailing?: Snippet<[]>;
    /** Optional wrapper for the code span. When provided, consumer controls the element and can attach actions. */
    codeWrapper?: Snippet<[Snippet]>;
  }

  let {
    line,
    displayIndex,
    isBookmarked = false,
    showBookmarkIcon = false,
    onDeleteBookmark,
    additionalClasses = {},
    gutterClass = '',
    gutter,
    code,
    trailing,
    codeWrapper,
  }: Props = $props();

  const ctx = getAnnotContext();

  // Unified state derivation from context
  const selected = $derived(ctx.interaction.isLineHighlighted(displayIndex));
  const annotated = $derived(ctx.annotations.hasAnnotation(displayIndex));
  const preview = $derived(ctx.interaction.isLinePreview(displayIndex));
  const markdownMetadata = $derived(ctx.markdownMetadata);

  // Terraform region indicator (shows on first line of region)
  const terraformRegionStart = $derived.by(() => {
    const lineNum = getLineNumber(line);
    if (lineNum === null) return undefined;
    return ctx.terraform.isRegionStart(lineNum);
  });

  // Terraform border (shows on all lines in region)
  const inTerraformRegion = $derived.by(() => {
    const lineNum = getLineNumber(line);
    if (lineNum === null) return false;
    return ctx.terraform.isInRegion(lineNum);
  });

  // Terraform phrase for tooltip (fetched lazily)
  let terraformPhrase = $state('');
  $effect(() => {
    if (terraformRegionStart) {
      ctx.terraform.getPhrase(terraformRegionStart).then(p => terraformPhrase = p);
    } else {
      terraformPhrase = '';
    }
  });

  // Show choice buttons on the last line of selection when pending choice
  const showChoiceButtons = $derived(
    ctx.interaction.pendingChoice &&
    ctx.interaction.range !== null &&
    displayIndex === Math.max(ctx.interaction.range.start, ctx.interaction.range.end)
  );

  // Show terraform palette on last line of selection when terraforming
  const showTerraformPalette = $derived(
    ctx.interaction.phase === 'terraforming' &&
    ctx.interaction.range !== null &&
    displayIndex === Math.max(ctx.interaction.range.start, ctx.interaction.range.end)
  );

  // Convert additionalClasses object to class string
  const extraClasses = $derived(
    Object.entries(additionalClasses)
      .filter(([_, v]) => v)
      .map(([k]) => k)
      .join(' ')
  );

  function handleChooseAnnotate() {
    ctx.interaction.confirmChoice('annotate');
  }

  function handleChooseBookmark() {
    ctx.interaction.confirmChoice('bookmark');
  }

  function handleChooseTerraform() {
    ctx.interaction.confirmChoice('terraform');
  }

  async function handleTerraformConfirm(region: TerraformRegion) {
    const range = ctx.interaction.range;
    if (!range) return;

    if (isIntentEmpty(region.intent)) {
      await ctx.terraform.remove(range);
    } else {
      await ctx.terraform.upsert(range, region);
      // Show toast with line range and phrase
      const phrase = await ctx.terraform.getPhrase(region);
      const startLine = Math.min(range.start, range.end);
      const endLine = Math.max(range.start, range.end);
      const lineLabel = startLine === endLine ? `Line ${startLine}` : `Lines ${startLine}-${endLine}`;
      ctx.showToast(`${lineLabel}: ${phrase}`);
    }

    ctx.interaction.closeTerraform();
  }

  function handleTerraformCancel() {
    ctx.interaction.closeTerraform();
  }

  // Auto-save terraform changes as user edits
  async function handleTerraformChange(region: TerraformRegion) {
    const range = ctx.interaction.range;
    if (!range) return;

    if (isIntentEmpty(region.intent)) {
      await ctx.terraform.remove(range);
    } else {
      await ctx.terraform.upsert(range, region);
    }
  }

  // Load existing terraform region when palette opens
  function loadTerraformRegion() {
    const range = ctx.interaction.range;
    if (!range) return Promise.resolve(undefined);
    return ctx.terraform.load(range);
  }
</script>

<div
  class="line {extraClasses}"
  class:selected
  class:annotated
  class:preview
  class:bookmarked={isBookmarked}
  class:has-terraform={inTerraformRegion}
  data-display-idx={displayIndex}
  onmouseenter={() => ctx.interaction.handleLineEnter(displayIndex)}
  onmouseleave={() => ctx.interaction.handleLineLeave()}
  role="presentation"
>
  <button
    class="add-btn"
    onpointerdown={(e) => ctx.interaction.handlePointerDown(displayIndex, e)}
    aria-label="Add annotation"
  >+</button>
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <span
    class="gutter {gutterClass}"
    class:selected
    onpointerdown={(e) => ctx.interaction.handlePointerDown(displayIndex, e)}
    onclick={() => ctx.interaction.handleGutterClick(displayIndex)}
    role="button"
    tabindex="-1"
  >
    {@render gutter()}
  </span>
  {#if codeWrapper}
    {@render codeWrapper(code)}
  {:else}
    <span class="code" class:md={markdownMetadata}>
      {@render code()}
    </span>
  {/if}
  {#if trailing || showBookmarkIcon || terraformRegionStart}
    <span class="line-actions">
      {#if trailing}
        {@render trailing()}
      {/if}
      {#if showBookmarkIcon}
        <button class="line-action bookmark-indicator" onclick={onDeleteBookmark} title="Remove bookmark">
          <BookmarkIcon filled />
        </button>
      {/if}
      {#if terraformRegionStart}
        <button
          class="line-action terraform-indicator"
          use:tooltip={{ content: terraformPhrase, placement: 'top' }}
          onclick={() => {
            const range = ctx.terraform.getDisplayRange(terraformRegionStart);
            if (range) {
              ctx.interaction.setSelection(range);
              ctx.interaction.openTerraform();
            }
          }}
        >
          <TerraformIcon />
        </button>
      {/if}
    </span>
  {/if}
</div>
{#if showChoiceButtons}
  <ChoiceButtons
    onAnnotate={handleChooseAnnotate}
    onBookmark={handleChooseBookmark}
    onTerraform={handleChooseTerraform}
  />
{/if}
{#if showTerraformPalette && ctx.interaction.range}
  {#await loadTerraformRegion() then existingRegion}
    <TerraformPalette
      startLine={ctx.interaction.range.start}
      endLine={ctx.interaction.range.end}
      initialRegion={existingRegion}
      onConfirm={handleTerraformConfirm}
      onCancel={handleTerraformCancel}
      onChange={handleTerraformChange}
    />
  {/await}
{/if}
