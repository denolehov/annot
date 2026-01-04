import { invoke } from '@tauri-apps/api/core';
import type {
  FormType,
  Intensity,
  MassChange,
  GravityChange,
  DirectionDirective,
  TerraformRegion,
  TerraformIntent,
} from '$lib/types';
import { INTENSITY_LEVELS, FORM_TYPES, emptyTransformIntent, isIntentEmpty } from '$lib/types';

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
const PHRASE_DEBOUNCE_MS = 20;

/** Create terraform state composable for palette. */
export function useTerraform(initialRegion?: TerraformRegion) {
  // Single intent state - type-safe combinations only
  let intent: TerraformIntent = $state(initialRegion?.intent ?? emptyTransformIntent());

  // Store previous transform state for recovery when exiting terminal modes
  let previousTransform: (TerraformIntent & { kind: 'transform' }) | null = $state(null);

  // Phrase from backend (updated via debounced IPC)
  let phrase = $state('');
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;

  // Check if state is empty (nothing configured)
  const isEmpty = $derived(isIntentEmpty(intent));

  // --- Derived state for UI convenience ---
  // These expose the inner fields when in transform mode
  const form = $derived(intent.kind === 'transform' ? intent.form : []);
  const mass = $derived(intent.kind === 'transform' ? intent.mass : null);
  const gravity = $derived(intent.kind === 'transform' ? intent.gravity : null);
  const direction = $derived(
    intent.kind === 'transform' ? intent.direction :
    intent.kind === 'dissolve' ? intent.direction :
    null
  );

  // --- Override status for visual dimming ---
  const isRemoveActive = $derived(intent.kind === 'remove');
  const isPinActive = $derived(intent.kind === 'pin');
  const isDissolveActive = $derived(intent.kind === 'dissolve');

  const formOverridden = $derived(isRemoveActive || isPinActive || isDissolveActive);
  const massOverridden = $derived(isRemoveActive || isPinActive || isDissolveActive);
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
    isPinActive ? 'Overridden by Pin' :
    isDissolveActive ? 'Overridden by Dissolve' : null
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
        intent,
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
    void intent;
    updatePhrase();
  });

  // --- Helper: ensure we're in transform mode ---
  function ensureTransform(): TerraformIntent & { kind: 'transform' } {
    if (intent.kind === 'transform') {
      return intent;
    }
    // Transition to transform, preserving direction if coming from dissolve
    const newIntent: TerraformIntent = {
      kind: 'transform',
      form: [],
      mass: null,
      gravity: null,
      direction: intent.kind === 'dissolve' ? intent.direction : null,
    };
    intent = newIntent;
    return newIntent;
  }

  // --- Helper: save current transform state before entering terminal mode ---
  function saveTransformState(): void {
    if (intent.kind === 'transform') {
      previousTransform = intent;
    }
  }

  // --- Terminal state setters ---
  function setRemove(): void {
    saveTransformState();
    intent = { kind: 'remove' };
  }

  function clearRemove(): void {
    if (intent.kind === 'remove') {
      intent = previousTransform ?? emptyTransformIntent();
      previousTransform = null;
    }
  }

  function setPin(): void {
    saveTransformState();
    intent = { kind: 'pin' };
  }

  function clearPin(): void {
    if (intent.kind === 'pin') {
      intent = previousTransform ?? emptyTransformIntent();
      previousTransform = null;
    }
  }

  function setDissolve(): void {
    saveTransformState();
    // Preserve direction when transitioning to dissolve
    const currentDirection = intent.kind === 'transform' ? intent.direction :
                             intent.kind === 'dissolve' ? intent.direction : null;
    intent = { kind: 'dissolve', direction: currentDirection };
  }

  function clearDissolve(): void {
    if (intent.kind === 'dissolve') {
      // Restore previous transform, merging in any direction changes made in dissolve mode
      const prev = previousTransform;
      intent = {
        kind: 'transform',
        form: prev?.form ?? [],
        mass: prev?.mass ?? null,
        gravity: prev?.gravity ?? null,
        direction: intent.direction, // Keep direction from dissolve mode
      };
      previousTransform = null;
    }
  }

  // --- Form mutations (auto-switch to transform) ---
  function toggleForm(type: FormType): void {
    const t = ensureTransform();
    const idx = t.form.indexOf(type);
    if (idx >= 0) {
      intent = { ...t, form: t.form.filter((_, i) => i !== idx) };
    } else {
      intent = { ...t, form: [...t.form, type] };
    }
  }

  // --- Mass mutations (auto-switch to transform) ---
  function setMassExpand(intensity: Intensity = DEFAULT_INTENSITY): void {
    const t = ensureTransform();
    intent = { ...t, mass: { type: 'expand', intensity } };
  }

  function setMassCondense(intensity: Intensity = DEFAULT_INTENSITY): void {
    const t = ensureTransform();
    intent = { ...t, mass: { type: 'condense', intensity } };
  }

  function clearMass(): void {
    if (intent.kind === 'transform') {
      intent = { ...intent, mass: null };
    }
  }

  function adjustMassIntensity(delta: number): void {
    if (intent.kind !== 'transform' || !intent.mass) return;
    const currentIdx = INTENSITY_LEVELS.indexOf(intent.mass.intensity);
    const newIdx = Math.max(0, Math.min(INTENSITY_LEVELS.length - 1, currentIdx + delta));
    intent = { ...intent, mass: { ...intent.mass, intensity: INTENSITY_LEVELS[newIdx] } };
  }

  // --- Gravity mutations (auto-switch to transform) ---
  function setGravityFocus(intensity: Intensity = DEFAULT_INTENSITY): void {
    const t = ensureTransform();
    intent = { ...t, gravity: { type: 'focus', intensity } };
  }

  function setGravityBlur(intensity: Intensity = DEFAULT_INTENSITY): void {
    const t = ensureTransform();
    intent = { ...t, gravity: { type: 'blur', intensity } };
  }

  function clearGravity(): void {
    if (intent.kind === 'transform') {
      intent = { ...intent, gravity: null };
    }
  }

  function adjustGravityIntensity(delta: number): void {
    if (intent.kind !== 'transform' || !intent.gravity) return;
    const currentIdx = INTENSITY_LEVELS.indexOf(intent.gravity.intensity);
    const newIdx = Math.max(0, Math.min(INTENSITY_LEVELS.length - 1, currentIdx + delta));
    intent = { ...intent, gravity: { ...intent.gravity, intensity: INTENSITY_LEVELS[newIdx] } };
  }

  // --- Direction mutations (works in transform or dissolve) ---
  function setDirectionReframe(): void {
    if (intent.kind === 'transform') {
      intent = { ...intent, direction: { type: 'reframe' } };
    } else if (intent.kind === 'dissolve') {
      intent = { ...intent, direction: { type: 'reframe' } };
    } else {
      // Auto-switch to transform if in terminal state
      intent = { kind: 'transform', form: [], mass: null, gravity: null, direction: { type: 'reframe' } };
    }
  }

  function setDirectionLeanIn(intensity: Intensity = DEFAULT_INTENSITY): void {
    if (intent.kind === 'transform') {
      intent = { ...intent, direction: { type: 'leanin', intensity } };
    } else if (intent.kind === 'dissolve') {
      intent = { ...intent, direction: { type: 'leanin', intensity } };
    } else {
      intent = { kind: 'transform', form: [], mass: null, gravity: null, direction: { type: 'leanin', intensity } };
    }
  }

  function setDirectionMoveAway(intensity: Intensity = DEFAULT_INTENSITY): void {
    if (intent.kind === 'transform') {
      intent = { ...intent, direction: { type: 'moveaway', intensity } };
    } else if (intent.kind === 'dissolve') {
      intent = { ...intent, direction: { type: 'moveaway', intensity } };
    } else {
      intent = { kind: 'transform', form: [], mass: null, gravity: null, direction: { type: 'moveaway', intensity } };
    }
  }

  function clearDirection(): void {
    if (intent.kind === 'transform') {
      intent = { ...intent, direction: null };
    } else if (intent.kind === 'dissolve') {
      intent = { ...intent, direction: null };
    }
  }

  function adjustDirectionIntensity(delta: number): void {
    const dir = intent.kind === 'transform' ? intent.direction :
                intent.kind === 'dissolve' ? intent.direction : null;
    if (!dir || dir.type === 'reframe') return;

    const currentIdx = INTENSITY_LEVELS.indexOf(dir.intensity);
    const newIdx = Math.max(0, Math.min(INTENSITY_LEVELS.length - 1, currentIdx + delta));
    const newDirection = { ...dir, intensity: INTENSITY_LEVELS[newIdx] };

    if (intent.kind === 'transform') {
      intent = { ...intent, direction: newDirection };
    } else if (intent.kind === 'dissolve') {
      intent = { ...intent, direction: newDirection };
    }
  }

  // --- Serialization ---
  function toRegion(startLine: number, endLine: number): TerraformRegion {
    return {
      start_line: startLine,
      end_line: endLine,
      intent,
    };
  }

  return {
    // Intent (full access)
    get intent() { return intent; },

    // Derived fields for UI convenience
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

    // Terminal state setters
    setRemove,
    clearRemove,
    setPin,
    clearPin,
    setDissolve,
    clearDissolve,

    // Form
    toggleForm,

    // Mass
    setMassExpand,
    setMassCondense,
    clearMass,
    adjustMassIntensity,

    // Gravity
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
