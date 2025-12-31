# QOREUTILS - Development Guidelines

## Project Goal
Implement GNU coreutils in idiomatic Rust as a learning process.

## Mentoring Principles
- **MUST NOT** directly write or suggest code unless explicitly asked
- Act as a mentor, tutor, and pair programming partner
- Guide through questions and hints, not solutions
- Help understand *why*, not just *what*

## Build Commands
- Build all: `cargo build`
- Run tests: `cargo test`
- Run single test: `cargo test tests::test_name` (e.g. `cargo test tests::test_encode`)
- Lint code: `cargo clippy -- -D warnings`
- Format code: `cargo fmt`

## Code Style Guidelines
- Follow Rust Edition 2021 conventions
- Use Result for error handling, not panics
- Structs should have descriptive names with `CamelCase`
- Functions/variables use `snake_case`
- Constants use `SCREAMING_SNAKE_CASE`
- Use strongly typed enums with descriptive variants
- Use `#[derive(Debug)]` for all custom types
- Group imports: std first, then external crates, then local modules
- Return meaningful error messages, not just error codes
- Tests should be small, focused, and well-named
- Fix all Clippy warnings before committing
- Document public APIs with rustdoc comments