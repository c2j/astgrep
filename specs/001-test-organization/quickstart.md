# Quickstart Guide: Test Directory Reorganization

**Version**: 1.0.0
**Updated**: 2025-12-03

## Overview

This guide helps you get started with ASTGreP's test directory reorganization feature. The feature provides a systematic way to migrate test assets from the current scattered structure to an organized `newtest/` directory while preserving all functionality.

## Prerequisites

### System Requirements
- **Rust**: 1.70 or higher
- **Python**: 3.8 or higher
- **Disk Space**: At least 2GB free space for migration operations
- **Permissions**: Write access to the ASTGreP repository root

### Required Tools
```bash
# Verify Rust installation
rustc --version

# Verify Python installation
python3 --version

# Verify required system tools
find --version
rsync --version
```

## Installation

### 1. Build ASTGreP with Migration Support
```bash
cd astgrep/
cargo build --release --features test-migration
```

### 2. Verify Migration CLI
```bash
./target/release/astgrep migrate --help
```

## Basic Usage

### 1. Analyze Current Test Structure

Get an overview of current test assets:
```bash
astgrep migrate analyze --output analysis_report.md
```

This creates a comprehensive report showing:
- Total number of test assets
- Distribution by category and language
- Potential migration issues
- Estimated completion time

### 2. Validate Migration Plan

Before migrating, validate your plan:
```bash
# Basic validation
astgrep migrate validate --category validation --language python

# Comprehensive validation (recommended)
astgrep migrate validate --all --level comprehensive

# Validation with dependency checking
astgrep migrate validate --all --check-dependencies --check-disk-space
```

### 3. Dry Run Migration

Test the migration without making changes:
```bash
astgrep migrate migrate --dry-run --all --report dry_run_report.json
```

### 4. Execute Migration

#### Option A: Full Migration (Recommended for small projects)
```bash
astgrep migrate migrate --all --create-backups
```

#### Option B: Phased Migration (Recommended for large projects)
```bash
# Phase 1: Scripts only
astgrep migrate migrate --type script --preserve-timestamps

# Phase 2: Validation tests
astgrep migrate migrate --category validation --validate-after

# Phase 3: Remaining assets
astgrep migrate migrate --remaining --continue-on-error
```

### 5. Verify Migration Results

```bash
# Run verification suite
astgrep migrate verify --detailed

# Test execution with new structure
astgrep migrate test --new-structure --all

# Compare results with original
astgrep migrate compare --before-reports/ --after-reports/
```

## Common Workflows

### Workflow 1: Development Setup

Set up the new test structure for development:
```bash
# Create new structure while keeping original
astgrep migrate create-structure --no-migrate

# Update test discovery configuration
astgrep migrate update-config --scripts newtest/scripts/

# Test new setup
astgrep test --config new_test_config.yaml
```

### Workflow 2: CI/CD Integration

Update CI/CD pipelines to use new structure:
```bash
# Generate CI configuration
astgrep migrate generate-ci-config --template github-actions

# Validate CI compatibility
astgrep migrate validate-ci --config .github/workflows/test.yml

# Test CI configuration locally
astgrep migrate test-ci --local
```

### Workflow 3: Gradual Rollout

Roll out changes incrementally:
```bash
# Enable compatibility mode
astgrep migrate enable-compatibility --create-symlinks

# Test specific categories
astgrep migrate test --new-structure --category validation

# Gradually disable old paths
astgrep migrate disable-compatibility --category validation
```

## Directory Structure

### Target Organization

After migration, your test structure will look like:

```
newtest/
├── scripts/                      # Test execution scripts
│   ├── runners/                 # Main test runners
│   │   ├── validate.sh          # Primary validation interface
│   │   └── run_validation_suite.sh
│   ├── language_runners/        # Language-specific runners
│   └── utils/                   # Test utilities and helpers
├── functional/                   # Functional test suites
│   ├── core_patterns/           # Basic pattern matching tests
│   ├── advanced_patterns/       # Complex pattern features
│   └── language_specific/       # Language-specific tests
│       ├── python/              # Python test cases
│       ├── javascript/          # JavaScript/TypeScript tests
│       ├── java/                # Java test cases
│       ├── bash/                # Shell script tests
│       └── sql/                 # SQL test cases
├── rules/                       # Rule definition tests
│   ├── security/               # Security vulnerability rules
│   ├── quality/                # Code quality rules
│   └── compatibility/          # Semgrep compatibility rules
├── integration/                # End-to-end integration tests
│   ├── cli/                    # CLI interface tests
│   └── api/                    # API integration tests
├── performance/                # Performance benchmarks
└── fixtures/                   # Shared test data and resources
```

## Configuration

### Migration Configuration

Create `migration_config.yaml`:
```yaml
migration:
  target_directory: "newtest"
  preserve_timestamps: true
  create_backups: true
  validate_after_migration: true

  categories:
    scripts:
      target: "scripts/{type}/"
      naming_convention: "kebab-case"
    test_cases:
      target: "functional/language_specific/{language}/"
      naming_convention: "kebab-case"
    rules:
      target: "rules/{category}/"
      naming_convention: "kebab-case"

  languages:
    python: "python/"
    javascript: "javascript/"
    java: "java/"
    bash: "bash/"
    sql: "sql/"

  validation:
    check_dependencies: true
    check_disk_space: true
    validate_permissions: true
    create_checksums: true

  compatibility:
    create_symlinks: true
    preserve_original: true
    update_discovery: true
```

### Test Discovery Configuration

Update test discovery in `config.yaml`:
```yaml
test_discovery:
  patterns:
    - "functional/core_patterns/**/*.yaml"
    - "functional/advanced_patterns/**/*.yaml"
    - "functional/language_specific/**/*.{yaml,sgrep}"
    - "rules/**/*.{yaml,sgrep}"
    - "integration/**/*.yaml"

  languages:
    mapping:
      python: [".py"]
      javascript: [".js", ".mjs", ".cjs"]
      typescript: [".ts", ".tsx"]
      java: [".java"]
      bash: [".sh", ".bash"]

  reporting:
    formats: ["json", "markdown"]
    include_metrics: true
    include_performance: true
```

## Troubleshooting

### Common Issues

#### Issue: Permission Denied
```bash
# Solution: Check and fix permissions
sudo chown -R $USER:$USER newtest/
find newtest/ -type f -name "*.sh" -exec chmod +x {} \;
```

#### Issue: Disk Space
```bash
# Solution: Check available space and clean up
df -h .
astgrep migrate cleanup --temp-files --old-backups
```

#### Issue: Dependency Conflicts
```bash
# Solution: Validate and fix dependencies
astgrep migrate validate-dependencies --fix-automatically
astgrep migrate repair-broken-links --target newtest/
```

#### Issue: Test Discovery Fails
```bash
# Solution: Update test configuration
astgrep migrate update-discovery --config config.yaml
astgrep migrate test-discovery --dry-run
```

### Debug Mode

Enable debug logging for troubleshooting:
```bash
RUST_LOG=debug astgrep migrate migrate --all --debug
```

### Recovery

#### Rollback Migration
```bash
# Rollback to original structure
astgrep migrate rollback --backup-dir backups/pre-migration/

# Verify rollback
astgrep migrate verify-rollback --compare-original
```

#### Partial Recovery
```bash
# Restore specific category
astgrep migrate restore --category validation --from-backup

# Fix broken symlinks
astgrep migrate fix-symlinks --target newtest/
```

## Advanced Features

### Custom Migration Rules

Define custom asset placement rules:
```yaml
custom_rules:
  - pattern: "tests/performance/**/*.rs"
    target: "performance/benchmarks/rust/"
    metadata:
      category: "benchmarking"
      language: "rust"
  - pattern: "tests/**/test_*.py"
    target: "functional/language_specific/python/unit/"
    metadata:
      test_type: "unit"
```

### Parallel Processing

Speed up large migrations:
```bash
# Use parallel processing (default: number of CPU cores)
astgrep migrate migrate --all --parallel --threads 8

# Batch processing for very large projects
astgrep migrate migrate --all --batch-size 100 --parallel
```

### Progress Monitoring

Monitor migration progress:
```bash
# Real-time progress
astgrep migrate migrate --all --progress-bar

# Detailed logging
astgrep migrate migrate --all --verbose --log-file migration.log

# Remote monitoring
astgrep migrate migrate --all --web-dashboard --port 8080
```

## Best Practices

### Before Migration
1. **Backup Everything**: Create a complete repository backup
2. **Run Validation**: Use comprehensive validation before migrating
3. **Test on Copy**: Practice migration on a copy first
4. **Document Current State**: Record current test structure and dependencies

### During Migration
1. **Use Dry Run**: Always start with dry-run migration
2. **Monitor Progress**: Watch for errors or warnings
3. **Preserve Timestamps**: Maintain original file timestamps
4. **Create Checkpoints**: Use migration checkpoints for large projects

### After Migration
1. **Verify Thoroughly**: Run comprehensive test suites
2. **Update Documentation**: Update project documentation
3. **Train Team**: Educate team on new structure
4. **Monitor Performance**: Watch for performance regressions

## Getting Help

### Documentation
- **Full Documentation**: [ASTGreP Migration Guide](docs/migration.md)
- **API Reference**: [Migration API Documentation](api/migration.html)
- **Configuration Guide**: [Configuration Options](config/migration.md)

### Support
- **Issues**: Report bugs on [GitHub Issues](https://github.com/astgrep/astgrep/issues)
- **Discussions**: Ask questions on [GitHub Discussions](https://github.com/astgrep/astgrep/discussions)
- **Community**: Join our [Discord Community](https://discord.gg/astgrep)

### Examples
- **Sample Configurations**: [examples/migration/](examples/migration/)
- **Migration Scripts**: [scripts/migration/](scripts/migration/)
- **Test Templates**: [templates/test/](templates/test/)