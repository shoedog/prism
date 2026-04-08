# Contributing to Prism

Thank you for your interest in contributing to Prism.

## Getting Started

1. Fork the repository
2. Create a feature branch (`git checkout -b my-feature`)
3. Make your changes
4. Run the test suite: `cargo test`
5. Submit a pull request

## Development

```bash
# Build
cargo build

# Run tests
cargo test

# Run a specific test
cargo test test_taint_forward

# Check formatting
cargo fmt -- --check

# Run clippy
cargo clippy -- -D warnings
```

## Adding a New Algorithm

1. Create `src/algorithms/my_algorithm.rs`
1. Implement the `SlicingAlgorithm` trait
1. Register in `src/algorithms/mod.rs`
1. Add tests covering at least Python, JavaScript, and C
1. Document in the algorithm table in README.md

## Adding a New Language

1. Add the variant to `Language` enum in `src/languages/mod.rs`
1. Add tree-sitter grammar dependency to `Cargo.toml`
1. Implement language-specific queries in `src/languages/mod.rs`
1. Add a type provider in `src/type_providers/` (optional but recommended)
1. Add test fixtures in `tests/fixtures/<language>/`

## Code Style

- Follow standard Rust conventions (`cargo fmt`)
- All public functions need doc comments
- Tests go in `#[cfg(test)]` modules within the source file
- Integration tests go in `tests/`

## Reporting Issues

Please include:

- Rust version (`rustc --version`)
- OS and architecture
- Minimal reproduction case (diff + source file)
- Expected vs actual output
