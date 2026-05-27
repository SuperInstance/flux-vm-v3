.PHONY: build test lint fmt bench clean install

build:
	cargo build --release

test:
	cargo test --release

lint:
	cargo clippy --all-targets --all-features -- -D warnings

fmt:
	cargo fmt -- --check
	cargo fmt

bench:
	cargo bench

audit:
	cargo audit

clean:
	cargo clean

install:
	cargo install --path .
