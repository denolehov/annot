//! ID generation for persistent entities (bookmarks, tags, exit modes).
//!
//! Follows jj's approach: random bytes encoded as "reverse hex" using k-z instead of 0-9a-f.
//! This produces IDs like `kmwvzxyqstnp` — always lowercase, always 12 chars.

use rand::Rng;

/// jj-style hex alphabet: 0-9a-f maps to z-k (reversed).
/// 0→z, 1→y, 2→x, 3→w, 4→v, 5→u, 6→t, 7→s, 8→r, 9→q, a→p, b→o, c→n, d→m, e→l, f→k
const HEX_TO_JJ: [char; 16] = [
    'z', 'y', 'x', 'w', 'v', 'u', 't', 's', 'r', 'q', 'p', 'o', 'n', 'm', 'l', 'k',
];

/// Generates a 12-character jj-style ID.
///
/// Generates 6 random bytes and encodes each nibble using jj's reverse-hex alphabet (k-z).
/// IDs are prefix-matchable (e.g., `kmw` can resolve to `kmwvzxyqstnp`).
///
/// # Example
/// ```
/// let id = annot_lib::id::generate();
/// assert_eq!(id.len(), 12);
/// assert!(id.chars().all(|c| matches!(c, 'k'..='z')));
/// ```
pub fn generate() -> String {
    let mut rng = rand::thread_rng();
    let bytes: [u8; 6] = rng.gen();

    let mut result = String::with_capacity(12);
    for byte in bytes {
        let high = (byte >> 4) as usize;
        let low = (byte & 0x0F) as usize;
        result.push(HEX_TO_JJ[high]);
        result.push(HEX_TO_JJ[low]);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_returns_12_char_string() {
        let id = generate();
        assert_eq!(id.len(), 12);
    }

    #[test]
    fn generate_uses_jj_alphabet() {
        // All characters must be in k-z range (lowercase only)
        for _ in 0..100 {
            let id = generate();
            for c in id.chars() {
                assert!(
                    matches!(c, 'k'..='z'),
                    "Invalid character in ID: '{}' (must be k-z)",
                    c
                );
            }
        }
    }

    #[test]
    fn generate_produces_unique_ids() {
        let ids: Vec<String> = (0..100).map(|_| generate()).collect();
        let unique: std::collections::HashSet<_> = ids.iter().collect();
        assert_eq!(ids.len(), unique.len(), "IDs should be unique");
    }

    #[test]
    fn hex_encoding_is_correct() {
        // Verify the mapping: each hex nibble (0-15) maps to z-k
        assert_eq!(HEX_TO_JJ[0x0], 'z');
        assert_eq!(HEX_TO_JJ[0x1], 'y');
        assert_eq!(HEX_TO_JJ[0x9], 'q');
        assert_eq!(HEX_TO_JJ[0xa], 'p');
        assert_eq!(HEX_TO_JJ[0xf], 'k');
    }
}
