import { invoke } from '@tauri-apps/api/core';
import type {
  FormType,
  Intensity,
  MassDirective,
  GravityDirective,
  DirectionDirective,
  TerraformRegion,
} from '$lib/types';
import { INTENSITY_LEVELS, FORM_TYPES } from '$lib/types';

/** Internal state for terraform palette. */
export interface TerraformState {
  form: FormType[];
  mass: MassDirective | null;
  gravity: GravityDirective | null;
  direction: DirectionDirective | null;
}

/** Intensity display labels for UI. */
export const INTENSITY_LABELS: Record<Intensity, string> = {
  slightly: 'slightly',
  moderately: 'moderately',
  significantly: 'significantly',
};

/** Form type display labels for UI. */
export const FORM_LABELS: Record<FormType, string> = {
  table: 'table',
  list: 'list',
  prose: 'prose',
  diagram: 'diagram',
  code: 'code',
};

/** Default intensity (gentlest level). */
const DEFAULT_INTENSITY: Intensity = 'slightly';

/** Debounce delay for phrase IPC calls (ms). */
const PHRASE_DEBOUNCE_MS = 200;

/** Create terraform state composable for palette. */
export function useTerraform(initialRegion?: TerraformRegion) {
  // Initialize state from existing region or empty
  let form: FormType[] = $state(initialRegion?.form ?? []);
  let mass: MassDirective | null = $state(initialRegion?.mass ?? null);
  let gravity: GravityDirective | null = $state(initialRegion?.gravity ?? null);
  let direction: DirectionDirective | null = $state(initialRegion?.direction ?? null);

  // Phrase from backend (updated via debounced IPC)
  let phrase = $state('');
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;

  // Check if state is empty (nothing configured)
  const isEmpty = $derived(
    form.length === 0 && mass === null && gravity === null && direction === null
  );

  // --- Override status (for visual dimming based on precedence) ---
  // Remove overrides everything
  // Pin overrides form, mass (expand/condense), direction
  // Dissolve overrides form only
  const isRemoveActive = $derived(mass?.type === 'remove');
  const isPinActive = $derived(gravity?.type === 'pin');
  const isDissolveActive = $derived(gravity?.type === 'dissolve');

  const formOverridden = $derived(isRemoveActive || isPinActive || isDissolveActive);
  const massOverridden = $derived(isRemoveActive || isPinActive);
  const gravityOverridden = $derived(isRemoveActive);
  const directionOverridden = $derived(isRemoveActive || isPinActive);

  // Human-readable override reasons for tooltips
  const formOverrideReason = $derived(
    isRemoveActive ? 'Overridden by Remove' :
    isPinActive ? 'Overridden by Pin' :
    isDissolveActive ? 'Overridden by Dissolve' : null
  );
  const massOverrideReason = $derived(
    isRemoveActive ? 'Overridden by Remove' :
    isPinActive ? 'Overridden by Pin' : null
  );
  const gravityOverrideReason = $derived(
    isRemoveActive ? 'Overridden by Remove' : null
  );
  const directionOverrideReason = $derived(
    isRemoveActive ? 'Overridden by Remove' :
    isPinActive ? 'Overridden by Pin' : null
  );

  // Update phrase from backend when state changes
  function updatePhrase() {
    if (debounceTimer) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(async () => {
      // Build a region with dummy line numbers (phrase doesn't use them)
      const region: TerraformRegion = {
        start_line: 0,
        end_line: 0,
        form,
        mass,
        gravity,
        direction,
      };
      try {
        phrase = await invoke<string>('get_terraform_phrase', { region });
      } catch (e) {
        console.error('Failed to get terraform phrase:', e);
      }
    }, PHRASE_DEBOUNCE_MS);
  }

  // Trigger phrase update on any state change
  $effect(() => {
    // Access all reactive state to create dependencies
    void form;
    void mass;
    void gravity;
    void direction;
    updatePhrase();
  });

  // --- Form mutations ---
  function toggleForm(type: FormType): void {
    const idx = form.indexOf(type);
    if (idx >= 0) {
      form = form.filter((_, i) => i !== idx);
    } else {
      form = [...form, type];
    }
  }

  // --- Mass mutations ---
  function setMassExpand(intensity: Intensity = DEFAULT_INTENSITY): void {
    mass = { type: 'expand', intensity };
  }

  function setMassCondense(intensity: Intensity = DEFAULT_INTENSITY): void {
    mass = { type: 'condense', intensity };
  }

  function setMassRemove(): void {
    mass = { type: 'remove' };
  }

  function clearMass(): void {
    mass = null;
  }

  function adjustMassIntensity(delta: number): void {
    if (!mass || mass.type === 'remove') return;
    const currentIdx = INTENSITY_LEVELS.indexOf(mass.intensity);
    const newIdx = Math.max(0, Math.min(INTENSITY_LEVELS.length - 1, currentIdx + delta));
    mass = { ...mass, intensity: INTENSITY_LEVELS[newIdx] };
  }

  // --- Gravity mutations ---
  function setGravityPin(): void {
    gravity = { type: 'pin' };
  }

  function setGravityDissolve(): void {
    gravity = { type: 'dissolve' };
  }

  function setGravityFocus(intensity: Intensity = DEFAULT_INTENSITY): void {
    gravity = { type: 'focus', intensity };
  }

  function setGravityBlur(intensity: Intensity = DEFAULT_INTENSITY): void {
    gravity = { type: 'blur', intensity };
  }

  function clearGravity(): void {
    gravity = null;
  }

  function adjustGravityIntensity(delta: number): void {
    if (!gravity || gravity.type === 'pin' || gravity.type === 'dissolve') return;
    const currentIdx = INTENSITY_LEVELS.indexOf(gravity.intensity);
    const newIdx = Math.max(0, Math.min(INTENSITY_LEVELS.length - 1, currentIdx + delta));
    gravity = { ...gravity, intensity: INTENSITY_LEVELS[newIdx] };
  }

  // --- Direction mutations ---
  function setDirectionReframe(): void {
    direction = { type: 'reframe' };
  }

  function setDirectionLeanIn(intensity: Intensity = DEFAULT_INTENSITY): void {
    direction = { type: 'leanin', intensity };
  }

  function setDirectionMoveAway(intensity: Intensity = DEFAULT_INTENSITY): void {
    direction = { type: 'moveaway', intensity };
  }

  function clearDirection(): void {
    direction = null;
  }

  function adjustDirectionIntensity(delta: number): void {
    if (!direction || direction.type === 'reframe') return;
    const currentIdx = INTENSITY_LEVELS.indexOf(direction.intensity);
    const newIdx = Math.max(0, Math.min(INTENSITY_LEVELS.length - 1, currentIdx + delta));
    direction = { ...direction, intensity: INTENSITY_LEVELS[newIdx] };
  }

  // --- Serialization ---
  function toRegion(startLine: number, endLine: number): TerraformRegion {
    return {
      start_line: startLine,
      end_line: endLine,
      form,
      mass,
      gravity,
      direction,
    };
  }

  return {
    // State (getters for reactivity)
    get form() { return form; },
    get mass() { return mass; },
    get gravity() { return gravity; },
    get direction() { return direction; },
    get phrase() { return phrase; },
    get isEmpty() { return isEmpty; },

    // Override status (for visual dimming based on precedence)
    get formOverridden() { return formOverridden; },
    get massOverridden() { return massOverridden; },
    get gravityOverridden() { return gravityOverridden; },
    get directionOverridden() { return directionOverridden; },
    get formOverrideReason() { return formOverrideReason; },
    get massOverrideReason() { return massOverrideReason; },
    get gravityOverrideReason() { return gravityOverrideReason; },
    get directionOverrideReason() { return directionOverrideReason; },

    // Form
    toggleForm,

    // Mass
    setMassExpand,
    setMassCondense,
    setMassRemove,
    clearMass,
    adjustMassIntensity,

    // Gravity
    setGravityPin,
    setGravityDissolve,
    setGravityFocus,
    setGravityBlur,
    clearGravity,
    adjustGravityIntensity,

    // Direction
    setDirectionReframe,
    setDirectionLeanIn,
    setDirectionMoveAway,
    clearDirection,
    adjustDirectionIntensity,

    // Serialization
    toRegion,
  };
}

