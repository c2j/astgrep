# AGENTS.md

This file provides guidance for agentic coding agents working in this repository.

## Project Overview

astgrep is a high-performance, multi-language static code analysis tool implemented in Rust. It provides security vulnerability detection, code quality analysis, and pattern matching capabilities across multiple programming languages.

## Build/Test/Lint Commands

```bash
# Build all crates
cargo build

# Build optimized release binary
cargo build --release

# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run a single test by name
cargo test test_name_here

# Run tests for specific crate
cargo test -p astgrep-core
cargo test -p astgrep-parser
cargo test -p astgrep-rules

# Run benchmarks
cargo bench

# Run the CLI
cargo run -- analyze
cargo run -- validate rules/*.yml
cargo run -- info --extensions

# Run with logging
RUST_LOG=debug cargo run -- analyze
```

## Code Style Guidelines

### Imports
- Group imports: std lib first, then external crates, then internal crates
- Use `use anyhow::Result` for error handling
- Re-export commonly used types at crate root
- Example:
```rust
use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use astgrep_core::{AnalysisConfig, Language};
```

### Formatting
- Use standard rustfmt (no custom rustfmt.toml)
- Max line length: follow Rust conventions (100 chars recommended)
- 4 spaces for indentation
- Trailing commas in multi-line structs/enums

### Types & Naming
- Use `PascalCase` for types, traits, enums, structs
- Use `snake_case` for functions, variables, modules
- Use `SCREAMING_SNAKE_CASE` for constants
- Use `Result<T>` alias from `anyhow` for function returns
- Prefer `&str` over `&String`, `Path` over `&PathBuf` for params

### Error Handling
- Use `anyhow::Result` for error propagation
- Use `thiserror` for custom error types in `astgrep-core::error`
- Provide context with `.context()` when propagating errors
- Use `?` operator for error propagation
- Example:
```rust
pub type Result<T> = std::result::Result<T, AnalysisError>;

#[derive(Error, Debug)]
pub enum AnalysisError {
    #[error("Parse error: {message}")]
    ParseError { message: String },
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}
```

### Documentation
- Use `//!` for module-level documentation
- Use `///` for item-level documentation
- Document all public APIs
- Include examples in doc comments for complex functions

### Testing
- Unit tests inline in source files under `#[cfg(test)]` modules
- Integration tests in `tests/` directory
- Use `test-utils` crate for shared test utilities
- Name tests descriptively: `test_<function>_<scenario>`

### Architecture Patterns
- Workspace structure with crates in `crates/`
- Core crate: `astgrep-core` for types and error handling
- Parser crate: `astgrep-parser` for language-specific parsing
- CLI crate: `astgrep-cli` for command-line interface
- Use traits for extensibility (e.g., `LanguageParser`)

### Language Support
- Add new languages to `astgrep_core::Language` enum
- Create parser in `astgrep-parser/src/{language}.rs`
- Register parser in `LanguageParserRegistry`
- Add file extension mappings in `Language::extensions()`

### Dependencies
- Define workspace dependencies in root `Cargo.toml`
- Use `workspace = true` for shared dependencies
- Key crates: anyhow, thiserror, serde, tokio, rayon, tree-sitter, clap

### Safety & Security
- Tool is for defensive security analysis only
- No code execution when analyzing untrusted code
- Focus on vulnerability detection and code review
- Validate all inputs, use safe Rust patterns

## Running the Tool

```bash
# Analyze current directory
cargo run -- analyze

# Analyze with custom rules
cargo run -- analyze src/ tests/ --rules security-rules.yml

# Generate SARIF output
cargo run -- analyze --format sarif --output results.sarif

# Validate rule files
cargo run -- validate rules/*.yml
```
