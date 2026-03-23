# trigrep

trigrep is an **indexed regex search tool for large codebases**, written in Rust.

Instead of scanning every file on each search like `grep` or `ripgrep`, trigrep
builds a **local, disk-backed trigram index** of your repository. At query time
it:

- decomposes your regex into literal fragments,
- turns them into trigrams (and later sparse n-grams),
- uses an inverted index to find only the **small set of candidate files**,
- then runs a real regex engine just on those candidates.

This keeps search latency almost flat even as your monorepo grows, and makes
trigrep especially useful for **AI coding agents** and humans who search a lot
in very large trees.

## Installation

```bash
# From source
git clone <repo-url> && cd trigrep
make install

# Or build without installing
make build
```

The binary is installed to `~/.cargo/bin/trigrep`.

## Quick Start

```bash
# Build the index for your repo
trigrep index .

# Search with a regex
trigrep search "fn main" .

# Check index status
trigrep status .
```

## Usage

### `trigrep index [path]`

Build or rebuild the trigram index. Creates a `.trigrep/` directory at the
target path.

```bash
trigrep index .              # index current directory
trigrep index /path/to/repo  # index a specific repo
trigrep index . --force      # force rebuild
```

### `trigrep search <pattern> [path]`

Search for a regex pattern using the index. If no index exists, one is built
automatically.

```bash
trigrep search "pattern" .
trigrep search "fn\s+\w+" .          # regex supported
trigrep search "TODO|FIXME" .        # alternations
trigrep search "error" . -i          # case-insensitive
trigrep search "MyStruct" . -l       # files only
trigrep search "fn main" . -c        # count matches per file
trigrep search "pattern" . -w        # whole word
trigrep search "pattern" . --json    # JSON output
trigrep search "pattern" . --stats   # show index hit stats
trigrep search "pattern" . --no-index  # skip index, brute-force scan
```

**Flags:**

| Flag | Description |
|------|-------------|
| `-i, --ignore-case` | Case-insensitive matching |
| `-n, --line-number` | Show line numbers (default: on) |
| `-c, --count` | Print match count per file |
| `-l, --files-with-matches` | Print only filenames |
| `-w, --word-regexp` | Match whole words only |
| `-A <N>, --after-context` | Show N lines after match |
| `-B <N>, --before-context` | Show N lines before match |
| `-C <N>, --context` | Show N lines before and after |
| `--json` | Output as JSON (one object per line) |
| `--no-index` | Skip index, grep all files |
| `--stats` | Print query plan and candidate stats |

### `trigrep status [path]`

Show index metadata and staleness check against current git HEAD.

```bash
trigrep status .
```

## How It Works

1. **Indexing**: trigrep walks your repo (respecting `.gitignore`), extracts
   every overlapping 3-byte trigram from each text file, and writes an inverted
   index to `.trigrep/` on disk.

2. **Querying**: Your regex is parsed and decomposed into literal fragments.
   These fragments are converted to trigrams and looked up via binary search in
   the mmap'd index. Posting lists are intersected (AND) or unioned (OR) to
   find candidate files. Only those candidates are scanned with the real regex
   engine.

See [algorithm.md](algorithm.md) for full technical details.

## Output Format

Default output is grep-compatible:

```
src/main.rs:42:fn trigram_hash(a: u8, b: u8, c: u8) -> u32 {
```

JSON mode (`--json`):

```json
{"file":"src/main.rs","line":42,"content":"fn trigram_hash(a: u8, b: u8, c: u8) -> u32 {"}
```

## Project Structure

```
trigrep/
├── trigrep-index/   # Library: index building, reading, querying
└── trigrep-cli/     # Binary: CLI, regex decomposition, output formatting
```
