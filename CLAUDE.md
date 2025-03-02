# The Brain - Development Guidelines

## Commands
- Build: `cargo build`
- Run: `cargo run -p brain-rs`
- Check: `cargo check --all`
- Format: `cargo fmt --all`
- Lint: `cargo clippy --all -- -D warnings`
- Test all: `cargo test --all`
- Test specific: `cargo test -p <crate> -- <test_name>`

## Code Style
- **Formatting**: Follow Rust standard style with `cargo fmt`
- **Imports**: Group imports by source (std, external, internal)
- **Types**: Use clear type annotations for function signatures
- **Error Handling**: Use `anyhow::Result` for external APIs, `thiserror` for defining errors
- **Naming**: Use snake_case for functions/variables, CamelCase for types/traits, SCREAMING_CASE for constants
- **Documentation**: Document all public APIs with doc comments (`///`)
- **Organization**: Keep files focused on a single responsibility
- **Concurrency**: Prefer `Arc<Mutex<T>>` for shared state, use async/await consistently

The Brain follows a thought pipeline architecture with memory systems and tool integrations. The codebase is organized as a Rust workspace with crates for core logic, memory systems, tool integrations, and TUI.