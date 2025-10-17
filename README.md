# CR-SemService

A high-performance, multi-language static code analysis tool for security vulnerabilities and code quality, implemented in Rust.

## Features

- **Multi-language Support**: Java, JavaScript, Python, SQL, Bash
- **Security-focused**: Detects injection vulnerabilities, XSS, authentication issues, and more
- **High Performance**: Built in Rust for speed and memory safety
- **Flexible Rules**: YAML-based declarative rule definitions
- **Multiple Output Formats**: JSON, YAML, SARIF, Text, XML
- **Parallel Processing**: Multi-threaded analysis for large codebases
- **Extensible**: Modular architecture for easy language and rule additions

## Quick Start

### Installation

```bash
# Clone the repository
git clone https://github.com/c2j/cr-semservice.git
cd cr-semservice

# Build the project
cargo build --release

# Install the binary
cargo install --path .
```

### Basic Usage

```bash
# Analyze current directory
cr-semservice analyze

# Analyze specific files/directories
cr-semservice analyze src/ tests/

# Use specific rules
cr-semservice analyze --rules security-rules.yml

# Specify languages
cr-semservice analyze --language java --language python

# Output to file in SARIF format
cr-semservice analyze --format sarif --output results.sarif

# Validate rule files
cr-semservice validate rules/*.yml

# List supported languages
cr-semservice languages
```

## Architecture

The project is organized into several crates:

- `cr-core`: Core types, traits, and error handling
- `cr-ast`: Universal AST definitions and operations
- `cr-rules`: Rule parsing, validation, and execution engine
- `cr-parser`: Language parsers and adapters
- `cr-matcher`: Pattern matching engine
- `cr-dataflow`: Data flow and taint analysis
- `cr-cli`: Command-line interface

## Development

### Prerequisites

- Rust 1.70+ 
- Cargo

### Building

```bash
# Build all crates
cargo build

# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo run -- analyze

# Run benchmarks
cargo bench
```

### Testing

Each crate has comprehensive unit tests. Run tests with:

```bash
# Run all tests
cargo test

# Run tests for specific crate
cargo test -p cr-core

# Run tests with output
cargo test -- --nocapture
```

## Rule Format

Rules are defined in YAML format:

```yaml
rules:
  - id: java-sql-injection
    name: "SQL Injection Detection"
    description: "Detects potential SQL injection vulnerabilities"
    severity: ERROR
    confidence: HIGH
    languages: [java]
    patterns:
      - pattern: "$STMT.execute($QUERY)"
      - metavariable_pattern:
          metavariable: "$QUERY"
          patterns:
            - pattern: "$STR + $INPUT"
    dataflow:
      sources:
        - pattern: "request.getParameter($PARAM)"
      sinks:
        - pattern: "Statement.execute($QUERY)"
    fix: "Use PreparedStatement with parameterized queries"
    metadata:
      cwe: "CWE-89"
      owasp: "A03:2021 - Injection"
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes with tests
4. Ensure all tests pass: `cargo test`
5. Submit a pull request

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Roadmap

- [ ] Complete AST implementation for all languages
- [ ] Advanced pattern matching with metavariables
- [ ] Data flow and taint analysis
- [ ] IDE integrations (VS Code, IntelliJ)
- [ ] CI/CD pipeline integrations
- [ ] Web interface
- [ ] Custom rule development tools

## Support

For questions, issues, or contributions, please visit our [GitHub repository](https://github.com/c2j/cr-semservice).
