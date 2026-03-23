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

Based on the approach described in Cursor's
[Fast regex search: indexing text for agent tools](https://cursor.com/blog/fast-regex-search).

## Benchmark

| Tool | Mean (s) | Median (s) | Min (s) | Max (s) | Samples |
| --- | ---: | ---: | ---: | ---: | ---: |
| grep | 0.5990 | 0.5550 | 0.3600 | 0.9600 | 20 |
| ripgrep | 0.0640 | 0.0500 | 0.0500 | 0.1700 | 20 |
| trigrep | 0.0405 | 0.0500 | 0.0100 | 0.0700 | 20 |

In a real search-only benchmark on `git.git`, trigrep had the lowest mean query
time: `0.0405s` vs `0.0640s` for `ripgrep` and `0.5990s` for `grep`
(`~1.58x` faster than ripgrep and `~14.79x` faster than grep for this
workload).

```markdown
- Timestamp (UTC): 2026-03-23T18:01:22Z
- Source repo: /tmp/trigrep-bench/git
- Source commit: 6e8d538aab8fe4dd07ba9fb87b5c7edcfa5706ad
- Runs per pattern: 5
- Warmup runs per pattern: 1
- Patterns: TODO|FIXME, ^#include, struct [A-Za-z_][A-Za-z0-9_]*, parse
- Scope: search-only (trigrep index built once before timing)
```

## Installation

```bash
# From source
git clone <repo-url> && cd trigrep
make install

# Or build without installing
make build
```

The binary is installed to `~/.cargo/bin/trigrep`.

## GitHub Releases

Pushing a version tag (for example `v0.1.0`) triggers the release workflow in
`.github/workflows/release.yml`. It builds `trigrep` for Linux, macOS, and
Windows, packages binaries, and publishes a GitHub Release with those assets.

```bash
git tag v0.1.0
git push origin v0.1.0
```

## Quick Start

```bash
# Build the index for your repo
trigrep index .

# Search with a regex
trigrep search "fn main" .

# Check index status
trigrep status .
```

## Run Benchmark Yourself

Use `make benchmark` to compare `grep`, `ripgrep`, and `trigrep` and get a
Markdown table with timing stats.

```bash
# Default: clones git.git into /tmp/trigrep-bench/git and benchmarks search time
make benchmark

# Benchmark any local repository
make benchmark BENCH_REPO_PATH=/path/to/repo
```

The benchmark writes Markdown to `/tmp/trigrep-bench/benchmark.md` by default
and also prints it to stdout.

### Benchmark Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `BENCH_REPO_PATH` | _(unset)_ | Local repo to benchmark. If set, no cloning is done. |
| `BENCH_REPO_URL` | `https://github.com/git/git.git` | Repo URL used when `BENCH_REPO_PATH` is unset. |
| `BENCH_REPO_DIR` | `/tmp/trigrep-bench/git` | Clone destination for default benchmark corpus. |
| `BENCH_RUNS` | `5` | Timed runs per pattern per tool. |
| `BENCH_WARMUP` | `1` | Warmup runs per pattern per tool. |
| `BENCH_OUT` | `/tmp/trigrep-bench/benchmark.md` | Output Markdown report path. |

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
