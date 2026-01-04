<script lang="ts">
  import { useTerraform, FORM_LABELS } from '$lib/composables/useTerraform.svelte';
  import type { TerraformRegion, FormType } from '$lib/types';
  import { FORM_TYPES, INTENSITY_LEVELS, emptyTransformIntent } from '$lib/types';
  import DiscreteSlider from './DiscreteSlider.svelte';
  import { GlobeIcon, FormIcon, MassIcon, GravityIcon, DirectionIcon, WarningIcon } from '$lib/icons';

  /** Explanatory titles for form buttons */
  const formTitles: Record<FormType, string> = {
    table: 'Restructure into a Markdown table',
    list: 'Restructure into a bulleted list',
    prose: 'Rewrite as flowing prose',
    diagram: 'Express as a Mermaid diagram',
    code: 'Convert into code or pseudocode',
  };

  interface Props {
    /** Existing region to edit (null for new). */
    initialRegion?: TerraformRegion;
    /** Called when user confirms (Enter). */
    onConfirm: (region: TerraformRegion) => void;
    /** Called when user cancels (Esc). */
    onCancel: () => void;
    /** Called when terraform state changes (for auto-save). */
    onChange?: (region: TerraformRegion) => void;
    /** Line range for the region. */
    startLine: number;
    endLine: number;
  }

  let { initialRegion, onConfirm, onCancel, onChange, startLine, endLine }: Props = $props();

  // Capture initial region at mount time (intentional - we don't want to reset on prop changes)
  // svelte-ignore state_referenced_locally
  const terraform = useTerraform(initialRegion);

  let paletteEl: HTMLDivElement;

  // Focus palette on mount for keyboard capture
  $effect(() => {
    if (paletteEl) {
      paletteEl.focus();
    }
  });

  // Click outside to dismiss/submit
  $effect(() => {
    function handleClickOutside(e: MouseEvent) {
      if (paletteEl && !paletteEl.contains(e.target as Node)) {
        if (terraform.isEmpty) {
          onCancel();
        } else {
          onConfirm(terraform.toRegion(startLine, endLine));
        }
      }
    }

    // Delay to avoid catching the click that opened the palette
    const timeout = setTimeout(() => {
      document.addEventListener('click', handleClickOutside);
    }, 0);

    return () => {
      clearTimeout(timeout);
      document.removeEventListener('click', handleClickOutside);
    };
  });

  // Auto-save: notify parent when state changes
  let isFirstRun = true;
  $effect(() => {
    // Access reactive state to create dependencies
    const region = terraform.toRegion(startLine, endLine);

    // Skip initial run to avoid saving unchanged state
    if (isFirstRun) {
      isFirstRun = false;
      return;
    }

    onChange?.(region);
  });

  function handleKeydown(event: KeyboardEvent) {
    // Cmd+Enter to confirm
    if (event.key === 'Enter' && (event.metaKey || event.ctrlKey)) {
      event.preventDefault();
      if (!terraform.isEmpty) {
        onConfirm(terraform.toRegion(startLine, endLine));
      } else if (initialRegion) {
        // Empty state while editing = delete
        onCancel();
      }
    }
    // Cmd+D to delete existing region
    else if (event.key === 'd' && (event.metaKey || event.ctrlKey) && initialRegion) {
      event.preventDefault();
      // Explicitly notify parent with empty region to trigger deletion
      const emptyRegion: TerraformRegion = {
        start_line: startLine,
        end_line: endLine,
        intent: emptyTransformIntent(),
      };
      onChange?.(emptyRegion);
      onCancel();
    }
    else if (event.key === 'Escape') {
      event.preventDefault();
      onCancel();
    }
    // Form toggles: 1-5 keys
    else if (event.key >= '1' && event.key <= '5') {
      event.preventDefault();
      const idx = parseInt(event.key) - 1;
      if (idx < FORM_TYPES.length) {
        terraform.toggleForm(FORM_TYPES[idx]);
      }
    }
    // Mass: x toggles remove
    else if (event.key === 'x') {
      event.preventDefault();
      toggleRemove();
    }
    // Mass: + moves toward expand, - moves toward condense
    // Steps through: slightly → moderately → significantly
    else if (event.key === '+' || event.key === '=') {
      event.preventDefault();
      const pos = getMassSliderValue();
      if (pos < 6) handleMassSliderChange(pos + 1);
    }
    else if (event.key === '-') {
      event.preventDefault();
      const pos = getMassSliderValue();
      if (pos > 0) handleMassSliderChange(pos - 1);
    }
    // Gravity: p/d shortcuts to extremes
    else if (event.key === 'p') {
      event.preventDefault();
      toggleGravityPin();
    }
    else if (event.key === 'd') {
      event.preventDefault();
      toggleGravityDissolve();
    }
    // Gravity: f moves toward focus/pin, b moves toward blur/dissolve
    else if (event.key === 'f') {
      event.preventDefault();
      if (terraform.intent.kind === 'pin') {
        // Already pinned, do nothing
      } else if (terraform.intent.kind === 'dissolve') {
        terraform.setGravityBlur('significantly'); // dissolve → max blur
      } else {
        const pos = getGravitySliderValue();
        if (pos === 0) {
          terraform.setPin(); // max focus → pin
        } else {
          handleGravitySliderChange(pos - 1);
        }
      }
    }
    else if (event.key === 'b') {
      event.preventDefault();
      if (terraform.intent.kind === 'dissolve') {
        // Already dissolved, do nothing
      } else if (terraform.intent.kind === 'pin') {
        terraform.setGravityFocus('significantly'); // pin → max focus
      } else {
        const pos = getGravitySliderValue();
        if (pos === 6) {
          terraform.setDissolve(); // max blur → dissolve
        } else {
          handleGravitySliderChange(pos + 1);
        }
      }
    }
    // Direction: r toggles reframe
    else if (event.key === 'r') {
      event.preventDefault();
      toggleDirectionReframe();
    }
    // Direction: < moves toward lean in (left), > moves toward move away (right)
    // Steps through: slightly → moderately → significantly
    else if (event.key === '<') {
      event.preventDefault();
      const pos = getDirectionSliderValue();
      if (pos > 0) handleDirectionSliderChange(pos - 1);
    }
    else if (event.key === '>') {
      event.preventDefault();
      const pos = getDirectionSliderValue();
      if (pos < 6) handleDirectionSliderChange(pos + 1);
    }
  }

  function isFormActive(type: FormType): boolean {
    return terraform.form.includes(type);
  }

  // --- Mass slider helpers ---
  // Slider positions (7 total): 0=condense significantly, 1=condense moderately, 2=condense slightly,
  // 3=neutral, 4=expand slightly, 5=expand moderately, 6=expand significantly
  function getMassSliderValue(): number {
    const mass = terraform.mass;
    if (!mass) return 3; // neutral (includes when in remove/pin/dissolve mode)
    if (mass.type === 'condense') {
      switch (mass.intensity) {
        case 'significantly': return 0;
        case 'moderately': return 1;
        case 'slightly': return 2;
      }
    } else {
      switch (mass.intensity) {
        case 'slightly': return 4;
        case 'moderately': return 5;
        case 'significantly': return 6;
      }
    }
  }

  function handleMassSliderChange(idx: number) {
    switch (idx) {
      case 0: terraform.setMassCondense('significantly'); break;
      case 1: terraform.setMassCondense('moderately'); break;
      case 2: terraform.setMassCondense('slightly'); break;
      case 3: terraform.clearMass(); break;
      case 4: terraform.setMassExpand('slightly'); break;
      case 5: terraform.setMassExpand('moderately'); break;
      case 6: terraform.setMassExpand('significantly'); break;
    }
  }

  function toggleRemove() {
    if (terraform.intent.kind === 'remove') {
      terraform.clearRemove();
    } else {
      terraform.setRemove();
    }
  }

  // --- Gravity slider helpers ---
  // Slider positions (7 total): 0=focus significantly, 1=focus moderately, 2=focus slightly,
  // 3=neutral, 4=blur slightly, 5=blur moderately, 6=blur significantly
  // Pin and dissolve are beyond the slider (toggle buttons) but fill the corresponding side
  function getGravitySliderValue(): number {
    // For pin/dissolve intents, show slider at the extreme
    if (terraform.intent.kind === 'pin') return 0;
    if (terraform.intent.kind === 'dissolve') return 6;

    const gravity = terraform.gravity;
    if (!gravity) return 3; // neutral
    if (gravity.type === 'focus') {
      switch (gravity.intensity) {
        case 'significantly': return 0;
        case 'moderately': return 1;
        case 'slightly': return 2;
      }
    } else {
      switch (gravity.intensity) {
        case 'slightly': return 4;
        case 'moderately': return 5;
        case 'significantly': return 6;
      }
    }
  }

  function handleGravitySliderChange(idx: number) {
    switch (idx) {
      case 0: terraform.setGravityFocus('significantly'); break;
      case 1: terraform.setGravityFocus('moderately'); break;
      case 2: terraform.setGravityFocus('slightly'); break;
      case 3: terraform.clearGravity(); break;
      case 4: terraform.setGravityBlur('slightly'); break;
      case 5: terraform.setGravityBlur('moderately'); break;
      case 6: terraform.setGravityBlur('significantly'); break;
    }
  }

  function toggleGravityPin() {
    if (terraform.intent.kind === 'pin') {
      terraform.clearPin();
    } else {
      terraform.setPin();
    }
  }

  function toggleGravityDissolve() {
    if (terraform.intent.kind === 'dissolve') {
      terraform.clearDissolve();
    } else {
      terraform.setDissolve();
    }
  }

  // --- Direction slider helpers ---
  // Slider positions (7 total): 0=lean in significantly, 1=lean in moderately, 2=lean in slightly,
  // 3=neutral, 4=move away slightly, 5=move away moderately, 6=move away significantly
  // Reframe is a toggle button (like remove in Mass)
  function getDirectionSliderValue(): number {
    const direction = terraform.direction;
    if (!direction || direction.type === 'reframe') return 3; // neutral
    if (direction.type === 'leanin') {
      switch (direction.intensity) {
        case 'significantly': return 0;
        case 'moderately': return 1;
        case 'slightly': return 2;
      }
    } else {
      switch (direction.intensity) {
        case 'slightly': return 4;
        case 'moderately': return 5;
        case 'significantly': return 6;
      }
    }
  }

  function handleDirectionSliderChange(idx: number) {
    switch (idx) {
      case 0: terraform.setDirectionLeanIn('significantly'); break;
      case 1: terraform.setDirectionLeanIn('moderately'); break;
      case 2: terraform.setDirectionLeanIn('slightly'); break;
      case 3: terraform.clearDirection(); break;
      case 4: terraform.setDirectionMoveAway('slightly'); break;
      case 5: terraform.setDirectionMoveAway('moderately'); break;
      case 6: terraform.setDirectionMoveAway('significantly'); break;
    }
  }

  function toggleDirectionReframe() {
    if (terraform.direction?.type === 'reframe') {
      terraform.clearDirection();
    } else {
      terraform.setDirectionReframe();
    }
  }

  // --- Click handlers for kbd buttons ---
  function handleMassDecrement() {
    const pos = getMassSliderValue();
    if (pos > 0) handleMassSliderChange(pos - 1);
  }

  function handleMassIncrement() {
    const pos = getMassSliderValue();
    if (pos < 6) handleMassSliderChange(pos + 1);
  }

  function handleGravityFocusClick() {
    if (terraform.intent.kind === 'pin') return;
    if (terraform.intent.kind === 'dissolve') {
      terraform.setGravityBlur('significantly');
    } else {
      const pos = getGravitySliderValue();
      if (pos === 0) {
        terraform.setPin();
      } else {
        handleGravitySliderChange(pos - 1);
      }
    }
  }

  function handleGravityBlurClick() {
    if (terraform.intent.kind === 'dissolve') return;
    if (terraform.intent.kind === 'pin') {
      terraform.setGravityFocus('significantly');
    } else {
      const pos = getGravitySliderValue();
      if (pos === 6) {
        terraform.setDissolve();
      } else {
        handleGravitySliderChange(pos + 1);
      }
    }
  }

  function handleDirectionLeanInClick() {
    const pos = getDirectionSliderValue();
    if (pos > 0) handleDirectionSliderChange(pos - 1);
  }

  function handleDirectionMoveAwayClick() {
    const pos = getDirectionSliderValue();
    if (pos < 6) handleDirectionSliderChange(pos + 1);
  }
</script>

<div
  class="terraform-palette"
  role="dialog"
  aria-label="Terraform region"
  tabindex="-1"
  bind:this={paletteEl}
  onkeydown={handleKeydown}
>
  <div class="terraform-header">
    <GlobeIcon class="terraform-icon" />
    <span>Terraform</span>
  </div>

  <!-- Form section -->
  <div class="terraform-row" class:overridden={terraform.formOverridden}>
    <span class="terraform-row-label">
      {#if terraform.formOverridden}
        <WarningIcon class="terraform-axis-icon terraform-warning-icon" title={terraform.formOverrideReason} />
      {:else}
        <FormIcon class="terraform-axis-icon" />
      {/if}
      Form
    </span>
    {#each FORM_TYPES as type, idx}
      <button
        class="terraform-toggle"
        class:active={isFormActive(type)}
        onclick={() => terraform.toggleForm(type)}
        title={formTitles[type]}
      >
        <kbd>{idx + 1}</kbd>
        <span>{FORM_LABELS[type]}</span>
      </button>
    {/each}
  </div>

  <!-- Mass section -->
  <div class="terraform-row">
    <span class="terraform-row-label">
      {#if terraform.massOverridden}
        <WarningIcon class="terraform-axis-icon terraform-warning-icon" title={terraform.massOverrideReason} />
      {:else}
        <MassIcon class="terraform-axis-icon" />
      {/if}
      Mass
    </span>
    <button
      class="terraform-toggle"
      class:active={terraform.intent.kind === 'remove'}
      onclick={toggleRemove}
      title="Remove this content entirely"
    >
      <kbd>x</kbd>
      <span>remove</span>
    </button>
    <div class="terraform-slider-wrap" class:overridden={terraform.massOverridden} title={terraform.massOverrideReason}>
      <DiscreteSlider
        value={getMassSliderValue()}
        onchange={handleMassSliderChange}
        disabled={terraform.intent.kind === 'remove' || terraform.massOverridden}
        bipolar
        steps={7}
      >
        {#snippet leftLabel()}<button class="kbd-btn" onclick={handleMassDecrement} title="Condense to essentials"><kbd>-</kbd></button> condense{/snippet}
        {#snippet rightLabel()}expand <button class="kbd-btn" onclick={handleMassIncrement} title="Expand with more depth and examples"><kbd>+</kbd></button>{/snippet}
      </DiscreteSlider>
    </div>
  </div>

  <!-- Gravity section: [pin] ← focus ←→ blur → [dissolve] -->
  <div class="terraform-row" class:overridden={terraform.gravityOverridden}>
    <span class="terraform-row-label">
      {#if terraform.gravityOverridden}
        <WarningIcon class="terraform-axis-icon terraform-warning-icon" title={terraform.gravityOverrideReason} />
      {:else}
        <GravityIcon class="terraform-axis-icon" />
      {/if}
      Gravity
    </span>
    <button
      class="terraform-toggle"
      class:active={terraform.intent.kind === 'pin'}
      onclick={toggleGravityPin}
      title="Preserve exactly as written"
    >
      <kbd>p</kbd>
      <span>pin</span>
    </button>
    <DiscreteSlider
      value={getGravitySliderValue()}
      onchange={handleGravitySliderChange}
      disabled={terraform.intent.kind === 'pin' || terraform.intent.kind === 'dissolve' || terraform.gravityOverridden}
      bipolar
      steps={7}
    >
      {#snippet leftLabel()}<button class="kbd-btn" onclick={handleGravityFocusClick} title="Make more central/prominent"><kbd>f</kbd></button> focus{/snippet}
      {#snippet rightLabel()}blur <button class="kbd-btn" onclick={handleGravityBlurClick} title="Reduce prominence; treat as supporting context"><kbd>b</kbd></button>{/snippet}
    </DiscreteSlider>
    <button
      class="terraform-toggle"
      class:active={terraform.intent.kind === 'dissolve'}
      onclick={toggleGravityDissolve}
      title="Remove as unit; integrate into surroundings"
    >
      <kbd>d</kbd>
      <span>dissolve</span>
    </button>
  </div>

  <!-- Direction section: [reframe] ← lean in ←→ move away -->
  <div class="terraform-row" class:overridden={terraform.directionOverridden}>
    <span class="terraform-row-label">
      {#if terraform.directionOverridden}
        <WarningIcon class="terraform-axis-icon terraform-warning-icon" title={terraform.directionOverrideReason} />
      {:else}
        <DirectionIcon class="terraform-axis-icon" />
      {/if}
      Direction
    </span>
    <button
      class="terraform-toggle"
      class:active={terraform.direction?.type === 'reframe'}
      onclick={toggleDirectionReframe}
      title="Keep the facts; change the angle or framing"
    >
      <kbd>r</kbd>
      <span>reframe</span>
    </button>
    <DiscreteSlider
      value={getDirectionSliderValue()}
      onchange={handleDirectionSliderChange}
      disabled={terraform.direction?.type === 'reframe' || terraform.directionOverridden}
      bipolar
      steps={7}
    >
      {#snippet leftLabel()}<button class="kbd-btn" onclick={handleDirectionLeanInClick} title="You're on the right track — amplify this thinking"><kbd>&lt;</kbd></button> lean in{/snippet}
      {#snippet rightLabel()}move away <button class="kbd-btn" onclick={handleDirectionMoveAwayClick} title="This approach is off-target; replace with alternative"><kbd>&gt;</kbd></button>{/snippet}
    </DiscreteSlider>
  </div>

  {#if !terraform.isEmpty && terraform.phrase}
    <div class="terraform-phrase">"{terraform.phrase}"</div>
  {/if}

  <div class="terraform-hints">
    <span class="terraform-hint"><kbd>⌘↵</kbd> save</span>
    {#if initialRegion}
      <span class="terraform-hint"><kbd>⌘D</kbd> delete</span>
    {/if}
    <span class="terraform-hint"><kbd>esc</kbd> cancel</span>
  </div>
</div>
