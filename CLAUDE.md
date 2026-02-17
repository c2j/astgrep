# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

astgrep is a high-performance, multi-language static code analysis tool implemented in Rust. It provides security vulnerability detection, code quality analysis, and pattern matching capabilities across multiple programming languages (Java, JavaScript, Python, SQL, Bash, PHP, C#, C, and more).

## Architecture

The project uses a modular workspace architecture with the following core crates:

- **astgrep-core**: Core types, traits, error handling, and analysis configuration
- **astgrep-ast**: Universal AST definitions and visitor patterns
- **astgrep-parser**: Language-specific parsers using tree-sitter with custom adapters
- **astgrep-matcher**: Pattern matching engine with metavariables and conditions
- **astgrep-rules**: Rule parsing, validation, and execution engine
- **astgrep-dataflow**: Data flow analysis, taint tracking, and call graph analysis
- **astgrep-cli**: Command-line interface and user interaction
- **astgrep-web**: Web server and API for remote analysis
- **astgrep-gui**: Desktop GUI application using egui
- **test-utils**: Common testing utilities and mock implementations

## Build and Development Commands

### Basic Commands
```bash
# Build all crates
cargo build

# Build optimized release binary
cargo build --release

# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run tests for specific crate
cargo test -p astgrep-core
cargo test -p astgrep-parser
cargo test -p astgrep-rules

# Run with logging
RUST_LOG=debug cargo run -- analyze

# Run benchmarks
cargo bench

# Install binary locally
cargo install --path .
```

### Running the Tool
```bash
# Analyze current directory with default settings
cargo run -- analyze

# Analyze specific paths with custom rules
cargo run -- analyze src/ tests/ --rules security-rules.yml

# Validate rule files
cargo run -- validate rules/*.yml

# List supported languages
cargo run -- info --extensions

# Generate SARIF output
cargo run -- analyze --format sarif --output results.sarif
```

### Specialized Testing
```bash
# Run comprehensive validation suite
./tests/run_validation_suite.sh

# Run advanced pattern tests
./tests/run_advanced_pattern_tests.sh

# Run Java comparison tests
./tests/run_java_comparison_tests.sh

# Test SQL-specific functionality
./tests/bash-sql/test_bash_script.sh
```

## Key Development Patterns

### Adding New Language Support
1. Add language variant to `astgrep_core::Language` enum
2. Create parser in `astgrep-parser/src/{language}.rs`
3. Implement tree-sitter adapter in `astgrep-parser::adapters`
4. Add file extension mappings
5. Write comprehensive tests in `tests/{language}/`

### Rule Development
- Rules are defined in YAML format with pattern matching, metavariables, and data flow specifications
- Rule validation happens in `astgrep-rules` crate
- Test rules using `cargo run -- validate rules/your-rule.yml`
- Example rule structure in `tests/rules/` directories

### SQL Processing
- SQL statement boundary detection is configurable via CLI flag `--sql-statement-boundary`
- YAML rules can override with `options.sql_statement_boundary: true/false`
- SQL parsing uses tree-sitter-sequel for enhanced coverage

### Performance Optimization
- Multi-threaded analysis enabled by default (use `--no-parallel` to disable)
- Thread count configurable with `--threads N` or `--max-threads N`
- Performance profiling available with `--profile` flag
- Use `cargo bench` for performance regression testing

### Error Handling
- All crates use `anyhow::Result` for error propagation
- Comprehensive error types in `astgrep-core::error`
- Graceful degradation for unsupported constructs
- Detailed error messages with file location context

## Testing Strategy

### Unit Tests
- Each crate has comprehensive unit tests in `src/` files
- Mock implementations in `test-utils` crate for isolated testing
- Property-based tests for core algorithms

### Integration Tests
- Language-specific test suites in `tests/{language}/`
- End-to-end validation in `tests/validation/`
- Performance benchmarks in `benches/`

### Rule Validation
- Syntax validation: `cargo run -- validate rules/`
- Semantic validation with test cases
- Performance testing with `--performance` flag

## Output Formats

The tool supports multiple output formats:
- **JSON**: Structured results for programmatic consumption
- **SARIF**: Static Analysis Results Interchange Format for CI/CD integration
- **Text**: Human-readable format with detailed explanations
- **YAML**: YAML-structured output
- **XML**: Legacy XML format compatibility

## Configuration

### CLI Configuration
- Use `--config file.toml` to specify configuration file
- Initialize with `cargo run -- init`
- Configuration supports rule directories, language filters, and output preferences

### Rule Configuration
- Rules support pattern matching, metavariables, and conditions
- Data flow analysis with sources, sinks, and sanitizers
- Metadata for severity, confidence, and categorization

## Development Environment

### Prerequisites
- Rust 1.70+ with Cargo
- tree-sitter CLI (for language grammar development)
- Git for version control

### IDE Integration
- VS Code support via `astgrep-cli::vscode_integration`
- Language Server Protocol implementation in progress
- Real-time analysis and rule editing capabilities

### Web Interface
- Web server available via `astgrep-web` crate
- REST API for remote analysis
- WebSocket support for real-time updates

## Common Development Tasks

### Adding New Rules
1. Create YAML rule file following existing patterns
2. Define patterns, metavariables, and data flow specifications
3. Add test cases demonstrating rule effectiveness
4. Validate with `cargo run -- validate your-rule.yml`
5. Run comprehensive test suite

### Extending Pattern Matching
- Core matching logic in `astgrep-matcher`
- Add new pattern types in `astgrep-matcher::patterns`
- Extend metavariable handling in `astgrep-matcher::metavar`
- Update parser for new syntax support

### Performance Improvements
- Profile with `--profile` flag and `cargo bench`
- Optimize parallel processing in `astgrep-core::optimization`
- Improve AST traversal efficiency
- Add caching for expensive operations

## Debugging

### Logging
- Enable debug logging: `RUST_LOG=debug cargo run -- analyze`
- Trace specific modules: `RUST_LOG=astgrep_parser=trace`
- Use `--verbose` flag for detailed output

### Common Issues
- Language parsing failures: Check tree-sitter grammar versions
- Rule validation errors: Verify YAML syntax and pattern structure
- Performance issues: Monitor thread usage and memory consumption
- SQL analysis: Verify statement boundary configuration

## Security Considerations

This tool is designed for defensive security analysis:
- Focuses on vulnerability detection and security code review
- Safe for analyzing untrusted code (no code execution)
- Supports security workflows and compliance checking
- Integrates with CI/CD pipelines for automated security scanning

## Active Technologies
- Rust 1.70+ + ree-sitter, clap, serde, anyhow, tokio, rayon (001-test-organization)
- File system (directories and scripts) (001-test-organization)

## Recent Changes
- 001-test-organization: Added Rust 1.70+ + ree-sitter, clap, serde, anyhow, tokio, rayon
