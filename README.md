# astgrep

A high-performance, multi-language static code analysis tool for security vulnerabilities and code quality, implemented in Rust.

## Features

- **Multi-language Support**: Java, JavaScript, Python, SQL, Bash, and XML are fully supported
- **Multi-dialect SQL**: GaussDB, OpenGauss, PolarDB-MySQL, and Standard SQL — each powered by a specialized parser
  - GaussDB / OpenGauss: `ogsql-parser`
  - PolarDB-MySQL: `sqlparser-rs`
  - Standard SQL: `tree-sitter-sequel`
- **Embedded SQL Preprocessor**: Extract and analyze SQL embedded in Java source code (annotations and strings) and MyBatis XML, then match it with SQL semantic rules without writing complex host-language patterns
- **Security-focused**: Detects injection vulnerabilities, XSS, authentication issues, and more
- **High Performance**: Built in Rust for speed and memory safety
- **Flexible Rules**: YAML-based declarative rule definitions with metavariables, conditions, and dataflow tracking
- **Multiple Output Formats**: JSON, SARIF, Text, HTML, Markdown, and Semgrep-compatible output
- **Parallel Processing**: Multi-threaded analysis for large codebases
- **Web Playground**: Browser-based interactive rule testing through `astgrep-web` (REST API + Playground UI)
- **Desktop GUI**: Cross-platform desktop application built with egui for interactive analysis
- **Extensible**: Modular architecture for easy language and rule additions

## Quick Start

### Installation

```bash
# Clone the repository
git clone https://github.com/c2j/astgrep.git
cd astgrep

# Build the project
cargo build --release

# Install the binary
cargo install --path .
```

### Basic Usage

```bash
# Analyze current directory
astgrep analyze

# Analyze specific files/directories
astgrep analyze src/ tests/

# Use specific rules
astgrep analyze --rules security-rules.yml

# Specify languages
astgrep analyze --language java --language python

# Output to file in SARIF format
astgrep analyze --format sarif --output results.sarif

# Validate rule files
astgrep validate rules/*.yml

# List supported languages
astgrep languages

# SQL dialect analysis (GaussDB compatibility scan)
astgrep analyze --dialect gaussdb --rules tests/categories/rules/sql_dialects/gaussdb/ *.sql

# OpenGauss analysis
astgrep analyze --dialect opengauss --rules tests/categories/rules/sql_dialects/gaussdb/ *.sql

# PolarDB-MySQL analysis
astgrep analyze --dialect polardb-mysql --rules tests/categories/rules/sql_dialects/polardb_mysql/ *.sql

# List available rules
astgrep list --language java --detailed

# Initialize configuration
astgrep init --template security --output astgrep.toml

# Show supported languages and extensions
astgrep info --extensions
```

## Architecture

The project is organized as a Cargo workspace with ten crates:

- `astgrep-core`: Core types, `Language` enum, error handling, and configuration
- `astgrep-ast`: `UniversalNode` AST definitions, visitor, and builder
- `astgrep-parser`: Tree-sitter adapters per language plus the SQL dialect dispatcher
- `astgrep-matcher`: Pattern matching engine (literal, metavariable, and structural matching)
- `astgrep-dataflow`: Taint analysis, data flow, call graph, and constant propagation
- `astgrep-rules`: YAML rule parsing, validation, and execution engine
- `astgrep-cli`: Command-line interface with 14 commands
- `astgrep-web`: Axum REST API server and Web Playground
- `astgrep-gui`: egui desktop playground
- `test-utils`: `MockAstNode`, `MockParser`, and other testing utilities

## Components

### Available Interfaces

- **CLI (`astgrep`)**: The primary interface. Runs full analysis, validates rules, lists languages, and supports all output formats.
- **Web Service (`astgrep-web-server`)**: Axum-based REST API (default port `8080`) plus a browser Playground at `/playground` for interactive rule testing.
- **Desktop GUI (`astgrep-gui`)**: Cross-platform egui application with an interactive rule editor and embedded documentation.

## Supported Languages

| Language | Extensions | AST | Taint |
|----------|------------|-----|-------|
| Java | `.java` | Full | Full |
| JavaScript | `.js`, `.jsx` | Full | Full |
| Python | `.py` | Full | Full |
| SQL | `.sql` | Full | Full |
| Bash | `.sh`, `.bash` | Full | Full |
| XML | `.xml`, `.xsd`, `.xsl` | Full | — |

Parser adapters also exist for PHP, C, C#, Ruby, Kotlin, and Swift, but these languages are not yet fully integrated into the `Language` enum.

### SQL Dialect Support

astgrep understands four SQL dialects, selected with the `--dialect` flag:

- `gaussdb`
- `opengauss`
- `polardb-mysql`
- `standard`

See [docs/sql-dialects.md](docs/sql-dialects.md) for details on dialect-specific parsing and rule writing.

## Development

### Prerequisites

- Rust 1.70+
- Cargo

After cloning, install the pre-commit hooks:

```bash
lefthook install
```

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
cargo test -p astgrep-core

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

## Documentation

- [User Guide](docs/User-Guide.md) — End-user manual for the CLI, Web Playground, and Desktop GUI
- [Developer Guide](docs/DeveloperGuide.md) — REST API, integration, and architecture details
- [Rule Writing Guide](docs/astgrep-Guide.md) — How to write YAML rules
- [SQL Dialect Support](docs/sql-dialects.md) — Multi-dialect SQL analysis
- [Contributing Guide](docs/CONTRIBUTING.md) — Development setup and conventions
- [Roadmap](docs/ROADMAP.md) — Project roadmap and milestones

## Contributing

See [docs/CONTRIBUTING.md](docs/CONTRIBUTING.md) for detailed guidelines.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Roadmap

See [docs/ROADMAP.md](docs/ROADMAP.md) for the detailed, prioritized roadmap.

### Current Priorities

1. **Codebase Health** — Fix compilation errors, reduce warnings, add CI/CD
2. **Architecture Refactoring** — Break up oversized files
3. **Semgrep Compatibility** — Complete remaining compatibility fixes
4. **Test Infrastructure** — Complete test directory reorganization

## Support

For questions, issues, or contributions, please visit our [GitHub repository](https://github.com/c2j/astgrep).
