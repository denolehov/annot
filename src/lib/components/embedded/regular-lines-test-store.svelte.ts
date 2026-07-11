// Test helper: reactive diff-document store so RegularLines tests can
// replace a document mid-flight (the unfold flow) and observe the re-render.
import { deriveDisplay, type DiffDisplay } from '$lib/display-rows';
import type { DiffDocument } from '$lib/types';

export function makeStore(initial: DiffDocument[]) {
  let docs = $state(initial);
  const display = $derived(deriveDisplay(docs));
  return {
    get display(): DiffDisplay {
      return display;
    },
    replace(idx: number, doc: DiffDocument) {
      docs[idx] = doc;
    },
  };
}
