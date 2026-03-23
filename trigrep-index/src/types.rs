/// A trigram hash: 3 bytes packed into the low 24 bits of a u32.
/// Collision-free for trigrams since 3 bytes fit exactly.
pub type TrigramHash = u32;

/// Pack 3 bytes into a trigram hash. No hashing needed — direct bit packing.
#[inline]
pub fn trigram_hash(a: u8, b: u8, c: u8) -> TrigramHash {
    ((a as u32) << 16) | ((b as u32) << 8) | (c as u32)
}

/// Map a byte to one of 8 Bloom bits for next_mask.
/// Uses a multiplicative hash to spread ASCII characters evenly.
#[inline]
pub fn bloom_hash(b: u8) -> u8 {
    (b.wrapping_mul(0x9E) >> 5) & 0x07
}

/// A single posting entry: one trigram occurrence in one file.
/// 6 bytes on disk: file_id(4) + loc_mask(1) + next_mask(1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PostingEntry {
    pub file_id: u32,
    /// Bit i is set if the trigram occurs at some byte offset where offset % 8 == i.
    pub loc_mask: u8,
    /// 8-bit Bloom filter of characters immediately following this trigram.
    pub next_mask: u8,
}

/// A lookup table entry pointing into the postings file.
/// 16 bytes on disk: ngram_hash(4) + offset(8) + length(4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LookupEntry {
    pub ngram_hash: TrigramHash,
    /// Byte offset into postings.bin.
    pub offset: u64,
    /// Number of PostingEntry items in this posting list.
    pub length: u32,
}

/// A single trigram query with optional adjacency constraints.
#[derive(Debug, Clone)]
pub struct TrigramQuery {
    pub hash: TrigramHash,
    /// Expected next character for next_mask Bloom check.
    pub expected_next: Option<u8>,
}

/// A query plan built from regex decomposition.
#[derive(Debug, Clone)]
pub enum QueryPlan {
    /// All trigram queries must match (literal sequence).
    And(Vec<TrigramQuery>),
    /// At least one branch must match (alternation).
    Or(Vec<QueryPlan>),
    /// No trigrams could be extracted — must scan all files.
    MatchAll,
}
