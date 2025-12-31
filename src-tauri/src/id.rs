//! ID generation for persistent entities (bookmarks, tags, exit modes).

use rand::Rng;

/// Alphabet for IDs (jj-style: no vowels, no numbers).
/// This avoids accidental words.
const ALPHABET: &[u8] = b"kpqrstvwxyzKPQRSTVWXYZ";

/// Generates a 12-character ID.
///
/// IDs are prefix-matchable (e.g., `kXp` can resolve to `kXpQrStVwXyZ`).
///
/// # Example
/// ```
/// let id = annot_lib::id::generate();
/// assert_eq!(id.len(), 12);
/// assert!(id.chars().all(|c| "kpqrstvwxyzKPQRSTVWXYZ".contains(c)));
/// ```
pub fn generate() -> String {
    let mut rng = rand::thread_rng();
    (0..12)
        .map(|_| {
            let idx = rng.gen_range(0..ALPHABET.len());
            ALPHABET[idx] as char
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
                "kpqrstvwxyzKPQRSTVWXYZ".contains(c),
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
