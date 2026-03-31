.PHONY: build test fmt clippy coverage coverage-html coverage-json check clean

build:
	cargo build

test:
	cargo test

fmt:
	cargo fmt

fmt-check:
	cargo fmt --all -- --check

clippy:
	cargo clippy --all-targets -- -W clippy::all

coverage:
	cargo llvm-cov --text

coverage-html:
	cargo llvm-cov --html && open target/llvm-cov/html/index.html

coverage-json:
	cargo llvm-cov --json --output-path target/llvm-cov/coverage.json

check: fmt-check clippy test

clean:
	cargo clean
