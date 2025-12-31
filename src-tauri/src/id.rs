//! ID generation for persistent entities (bookmarks, tags, exit modes).

use rand::Rng;

/// Alphabet for base32 IDs (jj-style: no vowels, lowercase only).
/// This avoids accidental words and is case-insensitive friendly.
const BASE32_ALPHABET: &[u8] = b"0123456789kpqrstvwxyz";

/// Generates a 12-character base32 ID.
///
/// IDs are prefix-matchable (e.g., `k3u` can resolve to `k3u3daxdd2wp`).
///
/// # Example
/// ```
/// let id = annot_lib::id::generate();
/// assert_eq!(id.len(), 12);
/// assert!(id.chars().all(|c| "0123456789kpqrstvwxyz".contains(c)));
/// ```
pub fn generate() -> String {
    let mut rng = rand::thread_rng();
    (0..12)
        .map(|_| {
            let idx = rng.gen_range(0..BASE32_ALPHABET.len());
            BASE32_ALPHABET[idx] as char
        })
        .collect()
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
    fn generate_uses_valid_alphabet() {
        let id = generate();
        for c in id.chars() {
            assert!(
                "0123456789kpqrstvwxyz".contains(c),
                "Invalid character in ID: {}",
                c
            );
        }
    }

    #[test]
    fn generate_produces_unique_ids() {
        let ids: Vec<String> = (0..100).map(|_| generate()).collect();
        let unique: std::collections::HashSet<_> = ids.iter().collect();
        assert_eq!(ids.len(), unique.len(), "IDs should be unique");
    }
}
