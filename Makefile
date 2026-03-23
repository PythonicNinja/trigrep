.PHONY: build release install test check clean fmt lint

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
