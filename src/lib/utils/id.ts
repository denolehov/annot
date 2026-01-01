/**
 * jj-style ID generation.
 *
 * Follows jj's approach: random bytes encoded as "reverse hex" using k-z instead of 0-9a-f.
 * Produces IDs like `kmwvzxyqstnp` — always lowercase, always 12 chars.
 *
 * Mapping: 0→z, 1→y, 2→x, 3→w, 4→v, 5→u, 6→t, 7→s, 8→r, 9→q, a→p, b→o, c→n, d→m, e→l, f→k
 */

const HEX_TO_JJ = 'zyxwvutsrqponmlk';

/**
 * Generates a 12-character jj-style ID.
 *
 * Generates 6 random bytes and encodes each nibble using jj's reverse-hex alphabet (k-z).
 * IDs are prefix-matchable (e.g., `kmw` can resolve to `kmwvzxyqstnp`).
 */
export function generateId(): string {
  const bytes = new Uint8Array(6);
  crypto.getRandomValues(bytes);

  let result = '';
  for (const byte of bytes) {
    const high = byte >> 4;
    const low = byte & 0x0f;
    result += HEX_TO_JJ[high];
    result += HEX_TO_JJ[low];
  }
  return result;
}
