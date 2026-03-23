# trigrep Algorithm

## Why it's faster than grep/ripgrep

Traditional tools (`grep`, `ripgrep`) scan every file on every search. Their
search time scales linearly with repository size. trigrep pre-builds an inverted
index of trigrams (3-byte sequences) so that at query time it only reads the
small set of files that could possibly match — skipping the vast majority of
the repo entirely.

## Core Concepts

### Trigram

A **trigram** is every overlapping 3-byte window in a file's content.

```
"the cat" → "the", "he ", "e c", " ca", "cat"
```

We pack 3 bytes into a `u32`: `(a << 16) | (b << 8) | c`. This is
collision-free (max value 0x00FFFFFF = ~16.7M unique trigrams).

### Inverted Index

A mapping from each trigram to the list of files containing it (a "posting
list"). At query time we look up only the trigrams present in the search
pattern and intersect their posting lists.

## On-Disk Format

All files stored under `.trigrep/` in the repo root.

### `lookup.bin` — sorted trigram → postings pointer

A flat array of 16-byte entries sorted by `ngram_hash`, enabling binary search
via mmap:

```
┌──────────────┬──────────────┬──────────────┐
│ ngram_hash   │ offset       │ length       │
│ u32 (4B LE)  │ u64 (8B LE)  │ u32 (4B LE)  │
└──────────────┴──────────────┴──────────────┘
```

- `offset`: byte position in `postings.bin`
- `length`: number of `PostingEntry` items

### `postings.bin` — concatenated posting lists

Each posting entry is 6 bytes:

```
┌──────────────┬──────────┬──────────┐
│ file_id      │ loc_mask │ next_mask│
│ u32 (4B LE)  │ u8 (1B)  │ u8 (1B)  │
└──────────────┴──────────┴──────────┘
```

- `loc_mask`: 8-bit field. Bit `i` is set if the trigram occurs at some byte
  offset where `offset % 8 == i`. Used for adjacency checking.
- `next_mask`: 8-bit Bloom filter of the byte immediately following the
  trigram. Used to approximate "3.5-gram" matching without storing full
  quadgrams.

### `files.bin` — file ID to path mapping

Variable-length records: `file_id(u32) + path_len(u16) + path_bytes`.

### `meta.json` — index metadata

Version, timestamps, file/trigram counts, git HEAD for staleness detection.

## Indexing Pipeline

1. **Walk** the repo using `.gitignore`-aware traversal
2. **Skip** binary files (extension check + NUL-byte detection in first 8KB)
3. **Extract** all overlapping trigrams from each file's bytes
4. For each trigram occurrence, set the appropriate `loc_mask` bit and
   `next_mask` Bloom bit
5. **Merge** per-file trigram maps into a global `HashMap<trigram → Vec<PostingEntry>>`
6. **Sort** by trigram hash (for binary-searchable lookup table)
7. **Write** `postings.bin` (sequential), `lookup.bin` (sorted), `files.bin`, `meta.json`

File reading is parallelized with rayon.

## Query Pipeline

1. **Parse** the regex using `regex-syntax` into a High-level IR (HIR)
2. **Decompose** the HIR to extract literal segments:
   - `Literal` → overlapping trigrams (AND)
   - `Concat` → collect literals, extract trigrams from each run (AND)
   - `Alternation` → each branch independently (OR)
   - `Repetition(min≥1)` → recurse into sub; `min=0` → MatchAll
   - Character classes, anchors, `.` → no trigrams (MatchAll)
3. **Build** a `QueryPlan`: tree of AND/OR/MatchAll nodes
4. **Execute** against the index:
   - Binary search `lookup.bin` for each trigram hash
   - Read posting lists from `postings.bin`
   - AND → intersect file ID sets (smallest-first)
   - OR → union file ID sets
5. **Verify** by running the real `regex` engine on candidate files only
6. **Output** matches in grep-compatible or JSON format

## Mask Filtering (Adjacency Checks)

For a literal like `"abcde"` producing trigrams `"abc"`, `"bcd"`, `"cde"`:

**loc_mask adjacency**: Consecutive trigrams in a literal are offset by 1 byte.
To check if `"abc"` at some position could be immediately followed by `"bcd"`,
rotate `abc`'s `loc_mask` left by 1 bit and AND with `bcd`'s `loc_mask`. A
non-zero result means adjacency is possible.

**next_mask check**: For trigram `"abc"` where the next character is `"d"`,
check `abc.next_mask & (1 << bloom_hash('d')) != 0`. This is probabilistic
(Bloom false positives possible) but never has false negatives.

These filters reduce false positive candidate files before the expensive
regex verification step.

## Performance Characteristics

- **Index once, search many**: Index build is O(total bytes) but amortized
  across all subsequent searches.
- **Query time**: O(k · log(N) + |posting lists|) where k = number of
  trigrams in the query and N = unique trigrams in the index. For typical
  searches with literals, only a tiny fraction of files are candidates.
- **Memory**: Only `lookup.bin` is mmap'd (~16 bytes per unique trigram).
  Posting lists are read from disk on demand. The OS page cache handles
  repeated reads efficiently.

## Future: Sparse N-grams (Phase 2)

Instead of all consecutive trigrams, assign a weight to each character pair
(e.g., via character-pair frequency from a large corpus). Extract n-grams only
at boundaries where the boundary weight exceeds all internal weights. This
produces fewer, longer, more selective n-grams — reducing both index size and
query-time posting list lookups.
