# Contributing to langfuse-rs

Thank you for your interest in contributing to langfuse-rs! This document outlines the process and guidelines for contributing.

## Prerequisites

Before you start, ensure you have:

- **Rust 1.93.1 or later** (MSRV — Minimum Supported Rust Version)
- **Cargo** (comes with Rust)

Check your Rust version:

```bash
rustc --version
cargo --version
```

Update Rust if needed:

```bash
rustup update
```

## Building

Build all crates in the workspace:

```bash
cargo build --workspace
```

For faster day-to-day development builds, the workspace uses Cargo's recommended lighter `dev`
debug-info settings. This keeps useful panic backtraces while reducing codegen and link time.

If you need full debugger-quality symbols locally, use the custom debugging profile:

```bash
cargo build --workspace --profile debugging
```

## Testing

Run all tests:

```bash
cargo test --workspace
```

## Code Style & Linting

This project enforces strict code quality standards. All code must pass formatting and linting checks.

### Format Check

```bash
cargo fmt --all -- --check
```

To automatically fix formatting:

```bash
cargo fmt --all
```

### Lint Check

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

All warnings are treated as errors. Clippy warnings must be resolved before submitting a PR.

## Code Style Notes

- **Edition**: Rust 2024
- **Error Handling**: Use `thiserror` for custom errors. Never use `anyhow`.
- **Warnings as Errors**: All compiler warnings must be resolved. Use `#[allow]` only on auto-generated code.
- **Builder Pattern**: Implement manually (not derived). Use `Option<T>` fields and `impl Into<String>` for arguments.
- **Serialization**: Use `serde` derives. API types use `rename_all = "camelCase"`. Enums use `SCREAMING_SNAKE_CASE`.
- **Tests**: Write integration tests in `crates/*/tests/`. Do not use inline `#[cfg(test)] mod tests`.
- **Concurrency**: Use `DashMap` for caches, `Arc<Mutex<Vec>>` for batch buffers, `tokio::sync::Semaphore` for bounded parallelism.

## Pull Request Process

1. **Open an issue first** — Discuss your proposed change before implementing. This prevents wasted effort and ensures alignment with project goals.

2. **One concern per PR** — Keep PRs focused. A PR should address a single feature, bug fix, or refactoring. Multiple unrelated changes should be separate PRs.

3. **Ensure all checks pass**:
   - `cargo fmt --all -- --check` (formatting)
   - `cargo clippy --workspace --all-targets -- -D warnings` (linting)
   - `cargo build --workspace` (build)
   - `cargo test --workspace` (tests)

   If you are investigating a tricky bug under a debugger, prefer `--profile debugging` for local
   builds instead of changing the shared workspace defaults.

4. **Write clear commit messages** — Explain the "why" behind your changes, not just the "what".

5. **Update documentation** — If your change affects public APIs or behavior, update relevant docs and examples.

6. **Link to the issue** — Reference the issue number in your PR description.

## License

By contributing to langfuse-rs, you agree that your contributions will be licensed under the same terms as the project: **MIT OR Apache-2.0**.

All contributions are made under these dual licenses at your option.

## Code of Conduct

This project adheres to the [Rust Community Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct). By participating, you are expected to uphold this code. Please report unacceptable behavior to the project maintainers.

## Questions?

If you have questions about contributing, feel free to open an issue or reach out to the maintainers.

Happy coding! 🦀
