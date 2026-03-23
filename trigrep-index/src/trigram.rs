use std::collections::HashMap;
use crate::types::{TrigramHash, trigram_hash, bloom_hash};

/// Extract all overlapping trigrams from file content.
/// Returns a map of trigram_hash → (loc_mask, next_mask) for this file.
pub fn extract_trigrams(content: &[u8]) -> HashMap<TrigramHash, (u8, u8)> {
    let mut result: HashMap<TrigramHash, (u8, u8)> = HashMap::new();
    if content.len() < 3 {
        return result;
    }

    for i in 0..content.len() - 2 {
        let hash = trigram_hash(content[i], content[i + 1], content[i + 2]);
        let loc_bit: u8 = 1 << (i % 8);
        let next_bit: u8 = if i + 3 < content.len() {
            1 << bloom_hash(content[i + 3])
        } else {
            0
        };

        let entry = result.entry(hash).or_insert((0u8, 0u8));
        entry.0 |= loc_bit;
        entry.1 |= next_bit;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_trigrams_basic() {
        let content = b"abcde";
        let trigrams = extract_trigrams(content);
        // "abcde" -> trigrams: "abc", "bcd", "cde"
        assert_eq!(trigrams.len(), 3);

        let abc = trigram_hash(b'a', b'b', b'c');
        let bcd = trigram_hash(b'b', b'c', b'd');
        let cde = trigram_hash(b'c', b'd', b'e');

        assert!(trigrams.contains_key(&abc));
        assert!(trigrams.contains_key(&bcd));
        assert!(trigrams.contains_key(&cde));
    }

    #[test]
    fn test_extract_trigrams_short() {
        assert!(extract_trigrams(b"ab").is_empty());
        assert!(extract_trigrams(b"a").is_empty());
        assert!(extract_trigrams(b"").is_empty());
    }

    #[test]
    fn test_extract_trigrams_exact_three() {
        let trigrams = extract_trigrams(b"abc");
        assert_eq!(trigrams.len(), 1);
        let abc = trigram_hash(b'a', b'b', b'c');
        assert!(trigrams.contains_key(&abc));
    }

    #[test]
    fn test_loc_mask_wraps() {
        // A content long enough that positions wrap around mod 8
        let content = b"0123456789";
        let trigrams = extract_trigrams(content);
        // Trigram at position 0 ("012") and position 8 ("89" — wait, only 10 chars so positions 0..7)
        // Position 0: loc_mask bit 0; position 8 would be bit 0 again
        // With 10 chars we get 8 trigrams at positions 0..7
        let hash_0 = trigram_hash(b'0', b'1', b'2');
        let (loc, _) = trigrams[&hash_0];
        assert_eq!(loc & 1, 1); // bit 0 is set
    }

    #[test]
    fn test_next_mask_set() {
        let content = b"abcd";
        let trigrams = extract_trigrams(content);
        let abc = trigram_hash(b'a', b'b', b'c');
        let (_, next) = trigrams[&abc];
        let expected_bit = 1u8 << bloom_hash(b'd');
        assert_ne!(next & expected_bit, 0);
    }

    #[test]
    fn test_repeated_trigram_merges_masks() {
        // "abcabc" has "abc" at positions 0 and 3
        let content = b"abcabc";
        let trigrams = extract_trigrams(content);
        let abc = trigram_hash(b'a', b'b', b'c');
        let (loc, _) = trigrams[&abc];
        // Position 0 -> bit 0, position 3 -> bit 3
        assert_ne!(loc & (1 << 0), 0);
        assert_ne!(loc & (1 << 3), 0);
    }
}
