#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TRIGREP_BIN="${TRIGREP_BIN:-$REPO_ROOT/target/release/trigrep}"

BENCH_REPO_PATH="${BENCH_REPO_PATH:-}"
BENCH_REPO_URL="${BENCH_REPO_URL:-https://github.com/git/git.git}"
BENCH_REPO_DIR="${BENCH_REPO_DIR:-/tmp/trigrep-bench/git}"
BENCH_RUNS="${BENCH_RUNS:-5}"
BENCH_WARMUP="${BENCH_WARMUP:-1}"
BENCH_OUT="${BENCH_OUT:-/tmp/trigrep-bench/benchmark.md}"

PATTERNS=(
  'TODO|FIXME'
  '^#include'
  'struct [A-Za-z_][A-Za-z0-9_]*'
  'parse'
)

require_tool() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "error: missing required tool '$1'" >&2
    exit 1
  fi
}

validate_number() {
  local name="$1"
  local value="$2"

  if ! [[ "$value" =~ ^[0-9]+$ ]]; then
    echo "error: $name must be a non-negative integer (got '$value')" >&2
    exit 1
  fi
}

measure_seconds() {
  local timing_file="$1"
  shift

  local status=0
  if /usr/bin/time -p "$@" >/dev/null 2>"$timing_file"; then
    status=0
  else
    status=$?
  fi

  if [ "$status" -ne 0 ] && [ "$status" -ne 1 ]; then
    cat "$timing_file" >&2
    echo "error: benchmark command failed with status $status: $*" >&2
    exit "$status"
  fi

  awk '$1 == "real" { print $2 }' "$timing_file"
}

compute_stats() {
  local times_file="$1"

  local samples
  local mean
  local median
  local min
  local max

  samples="$(wc -l < "$times_file" | tr -d ' ')"
  mean="$(awk '{sum += $1} END { if (NR == 0) print "n/a"; else printf "%.4f", sum / NR }' "$times_file")"
  median="$(sort -n "$times_file" | awk '{vals[NR] = $1} END { if (NR == 0) print "n/a"; else if (NR % 2 == 1) printf "%.4f", vals[(NR + 1) / 2]; else printf "%.4f", (vals[NR / 2] + vals[NR / 2 + 1]) / 2 }')"
  min="$(awk 'NR == 1 || $1 < min { min = $1 } END { if (NR == 0) print "n/a"; else printf "%.4f", min }' "$times_file")"
  max="$(awk 'NR == 1 || $1 > max { max = $1 } END { if (NR == 0) print "n/a"; else printf "%.4f", max }' "$times_file")"

  printf '%s|%s|%s|%s|%s\n' "$samples" "$mean" "$median" "$min" "$max"
}

prepare_repo() {
  if [ -n "$BENCH_REPO_PATH" ]; then
    if [ ! -d "$BENCH_REPO_PATH" ]; then
      echo "error: BENCH_REPO_PATH does not exist: $BENCH_REPO_PATH" >&2
      exit 1
    fi
    printf '%s\n' "$BENCH_REPO_PATH"
    return
  fi

  if [ -d "$BENCH_REPO_DIR/.git" ]; then
    printf '%s\n' "$BENCH_REPO_DIR"
    return
  fi

  mkdir -p "$(dirname "$BENCH_REPO_DIR")"

  if [ -e "$BENCH_REPO_DIR" ] && [ ! -d "$BENCH_REPO_DIR/.git" ]; then
    echo "error: BENCH_REPO_DIR exists but is not a git repository: $BENCH_REPO_DIR" >&2
    echo "set BENCH_REPO_PATH to an existing repo, or remove BENCH_REPO_DIR" >&2
    exit 1
  fi

  echo "==> Cloning benchmark repo into $BENCH_REPO_DIR" >&2
  git clone --depth 1 "$BENCH_REPO_URL" "$BENCH_REPO_DIR" >/dev/null
  printf '%s\n' "$BENCH_REPO_DIR"
}

run_tool_bench() {
  local tool_name="$1"
  local times_file="$2"
  local pattern="$3"
  shift 3

  local total_iterations=$((BENCH_WARMUP + BENCH_RUNS))
  local iteration
  local seconds
  local tmp_timing="$TMP_WORK_DIR/time.txt"

  for ((iteration = 1; iteration <= total_iterations; iteration++)); do
    seconds="$(measure_seconds "$tmp_timing" "$@")"
    if [ "$iteration" -gt "$BENCH_WARMUP" ]; then
      printf '%s\n' "$seconds" >> "$times_file"
    fi
  done

  echo "benchmarked $tool_name for pattern '$pattern'" >&2
}

require_tool git
require_tool grep
require_tool rg
require_tool /usr/bin/time

validate_number "BENCH_RUNS" "$BENCH_RUNS"
validate_number "BENCH_WARMUP" "$BENCH_WARMUP"

if [ "$BENCH_RUNS" -lt 1 ]; then
  echo "error: BENCH_RUNS must be >= 1" >&2
  exit 1
fi

if [ ! -x "$TRIGREP_BIN" ]; then
  echo "error: trigrep binary not found or not executable at $TRIGREP_BIN" >&2
  echo "run 'make release' first or set TRIGREP_BIN" >&2
  exit 1
fi

BENCH_ROOT="$(prepare_repo)"
BENCH_ROOT="$(cd "$BENCH_ROOT" && pwd)"

mkdir -p "$(dirname "$BENCH_OUT")"

TMP_WORK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/trigrep-bench.XXXXXX")"
trap 'rm -rf "$TMP_WORK_DIR"' EXIT

GREP_TIMES="$TMP_WORK_DIR/grep.times"
RG_TIMES="$TMP_WORK_DIR/rg.times"
TRIGREP_TIMES="$TMP_WORK_DIR/trigrep.times"

: > "$GREP_TIMES"
: > "$RG_TIMES"
: > "$TRIGREP_TIMES"

echo "==> Building trigrep index once for search-only benchmark" >&2
"$TRIGREP_BIN" index "$BENCH_ROOT" >/dev/null

for pattern in "${PATTERNS[@]}"; do
  run_tool_bench "grep" "$GREP_TIMES" "$pattern" \
    grep -r -n -E --binary-files=without-match --exclude-dir=.git --exclude-dir=.trigrep "$pattern" "$BENCH_ROOT"

  run_tool_bench "ripgrep" "$RG_TIMES" "$pattern" \
    rg -n -e "$pattern" "$BENCH_ROOT"

  run_tool_bench "trigrep" "$TRIGREP_TIMES" "$pattern" \
    "$TRIGREP_BIN" search "$pattern" "$BENCH_ROOT"
done

IFS='|' read -r grep_samples grep_mean grep_median grep_min grep_max <<< "$(compute_stats "$GREP_TIMES")"
IFS='|' read -r rg_samples rg_mean rg_median rg_min rg_max <<< "$(compute_stats "$RG_TIMES")"
IFS='|' read -r trigrep_samples trigrep_mean trigrep_median trigrep_min trigrep_max <<< "$(compute_stats "$TRIGREP_TIMES")"

bench_commit="n/a"
if git -C "$BENCH_ROOT" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  bench_commit="$(git -C "$BENCH_ROOT" rev-parse HEAD 2>/dev/null || echo "n/a")"
fi

timestamp_utc="$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
patterns_csv="$(printf '%s, ' "${PATTERNS[@]}")"
patterns_csv="${patterns_csv%, }"

cat > "$BENCH_OUT" <<EOF_MD
# trigrep benchmark

- Timestamp (UTC): $timestamp_utc
- Source repo: $BENCH_ROOT
- Source commit: $bench_commit
- Runs per pattern: $BENCH_RUNS
- Warmup runs per pattern: $BENCH_WARMUP
- Patterns: $patterns_csv
- Scope: search-only (trigrep index built once before timing)

| Tool | Mean (s) | Median (s) | Min (s) | Max (s) | Samples |
| --- | ---: | ---: | ---: | ---: | ---: |
| grep | $grep_mean | $grep_median | $grep_min | $grep_max | $grep_samples |
| ripgrep | $rg_mean | $rg_median | $rg_min | $rg_max | $rg_samples |
| trigrep | $trigrep_mean | $trigrep_median | $trigrep_min | $trigrep_max | $trigrep_samples |
EOF_MD

echo "==> Benchmark written to $BENCH_OUT" >&2
cat "$BENCH_OUT"
