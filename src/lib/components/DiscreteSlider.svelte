<script lang="ts">
  /**
   * DiscreteSlider - 5-position slider for intensity controls.
   * Displays left/right labels with a clickable track of steps.
   */
  import type { Snippet } from 'svelte';

  interface Props {
    /** Current value index (0-4). */
    value: number;
    /** Called when value changes. */
    onchange: (value: number) => void;
    /** Left label snippet (low end). */
    leftLabel?: Snippet;
    /** Right label snippet (high end). */
    rightLabel?: Snippet;
    /** Number of steps (default 5). */
    steps?: number;
    /** Whether the slider is disabled. */
    disabled?: boolean;
    /** Bipolar mode: neutral center, fill extends from center toward value. */
    bipolar?: boolean;
  }

  let {
    value,
    onchange,
    leftLabel,
    rightLabel,
    steps = 5,
    disabled = false,
    bipolar = false,
  }: Props = $props();

  const center = Math.floor(steps / 2);

  function isFilled(idx: number): boolean {
    if (bipolar) {
      // Fill between center and value (exclusive of center)
      if (value < center) return idx >= value && idx < center;
      if (value > center) return idx > center && idx <= value;
      return false; // neutral: nothing filled
    }
    return idx < value;
  }

  // Distance from center determines width class: edge (3) > middle (2) > adjacent (1) > center (0)
  function getStepSize(idx: number): 'edge' | 'middle' | 'adjacent' | 'center' {
    const dist = Math.abs(idx - center);
    if (dist === 0) return 'center';
    if (dist === 1) return 'adjacent';
    if (dist === 2) return 'middle';
    return 'edge';
  }

  // Glow intensity based on distance from center (only for active step)
  function getGlowLevel(idx: number): 'glow-1' | 'glow-2' | 'glow-3' | null {
    if (!bipolar || idx !== value || value === center) return null;
    const dist = Math.abs(value - center);
    if (dist === 1) return 'glow-1'; // slightly
    if (dist === 2) return 'glow-2'; // moderately
    return 'glow-3'; // significantly
  }

  function handleClick(idx: number) {
    if (!disabled) {
      onchange(idx);
    }
  }
</script>

<div class="terraform-slider" class:disabled>
  {#if leftLabel}
    <span class="terraform-slider-label left">{@render leftLabel()}</span>
  {/if}
  <div class="terraform-slider-track">
    {#each Array(steps) as _, idx}
      <button
        class="terraform-slider-step size-{getStepSize(idx)} {getGlowLevel(idx) ?? ''}"
        class:active={idx === value}
        class:filled={isFilled(idx)}
        onclick={() => handleClick(idx)}
        {disabled}
        aria-label="Intensity level {idx + 1}"
      ></button>
    {/each}
  </div>
  {#if rightLabel}
    <span class="terraform-slider-label right">{@render rightLabel()}</span>
  {/if}
</div>

<style>
  .terraform-slider.disabled {
    opacity: 0.4;
    pointer-events: none;
  }
</style>
