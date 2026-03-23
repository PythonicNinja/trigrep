.PHONY: build release install test check clean fmt lint benchmark

BENCH_REPO_PATH ?=
BENCH_REPO_URL ?= https://github.com/git/git.git
BENCH_REPO_DIR ?= /tmp/trigrep-bench/git
BENCH_RUNS ?= 5
BENCH_WARMUP ?= 1
BENCH_OUT ?= /tmp/trigrep-bench/benchmark.md

build:
	cargo build --workspace

release:
	cargo build --workspace --release

install: release
	cp target/release/trigrep ~/.cargo/bin/trigrep

test:
	cargo test --workspace

check:
	cargo check --workspace

fmt:
	cargo fmt --all

lint:
	cargo clippy --workspace -- -D warnings

clean:
	cargo clean
	rm -rf .trigrep/

benchmark: release
	BENCH_REPO_PATH="$(BENCH_REPO_PATH)" \
	BENCH_REPO_URL="$(BENCH_REPO_URL)" \
	BENCH_REPO_DIR="$(BENCH_REPO_DIR)" \
	BENCH_RUNS="$(BENCH_RUNS)" \
	BENCH_WARMUP="$(BENCH_WARMUP)" \
	BENCH_OUT="$(BENCH_OUT)" \
	./scripts/benchmark.sh
