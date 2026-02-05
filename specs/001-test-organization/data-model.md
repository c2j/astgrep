# Data Model: Test Directory Reorganization

**Date**: 2025-12-03
**Feature**: Test Directory Reorganization

## Core Entities

### TestAsset

Represents any test-related file or directory in the system.

**Attributes**:
- `id` (String): Unique identifier for the test asset
- `name` (String): Human-readable name
- `type` (Enum): `SCRIPT`, `TEST_CASE`, `FIXTURE`, `RULE_DEFINITION`, `REPORT`
- `current_path` (Path): Current file system location
- `target_path` (Path): Planned destination in new structure
- `language` (Language): Programming language association
- `category` (Category): Functional classification
- `status` (Status): Migration status
- `dependencies` (Array<String>): List of dependent asset IDs
- `metadata` (Object): Additional properties

### TestScript

Executable test runner or utility script.

**Attributes**:
- `asset_id` (String): Reference to TestAsset
- `script_type` (Enum): `VALIDATOR`, `RUNNER`, `UTILITY`, `CI_INTEGRATION`
- `platforms` (Array<Platform>): Supported execution platforms
- `execution_order` (Integer): Relative execution priority
- `arguments` (Array<Argument>): Command-line interface definition
- `exit_codes` (Object): Expected exit code mappings

### TestCase

Collection of test files for specific functionality.

**Attributes**:
- `asset_id` (String): Reference to TestAsset
- `test_type` (Enum): `PATTERN_MATCHING`, `RULE_VALIDATION`, `PARSING`, `INTEGRATION`
- `languages` (Array<Language>): Supported programming languages
- `rule_files` (Array<Path>): Associated rule definition files
- `source_files` (Array<Path>): Source code files for testing
- `expected_results` (Array<Path>): Expected output files
- `complexity` (Enum): `SIMPLE`, `MEDIUM`, `COMPLEX`

### DirectoryStructure

Defines the target organization hierarchy.

**Attributes**:
- `root_path` (Path): Base directory path
- `categories` (Array<CategoryDefinition>): Category definitions
- `naming_convention` (NamingConvention): File and directory naming rules
- `depth_limit` (Integer): Maximum directory depth
- `migration_rules` (Array<MigrationRule>): Asset placement rules

## Enums

### Language
```rust
pub enum Language {
    Python,
    JavaScript,
    TypeScript,
    Java,
    C,
    Cpp,
    Rust,
    Bash,
    SQL,
    XML,
    Json,
    Yaml,
    Go,
    Ruby,
    Php,
    Swift,
    Kotlin,
    Scala,
    Dart,
    Other(String)
}
```

### Category
```rust
pub enum Category {
    Validation,
    Performance,
    Compatibility,
    Benchmarking,
    Security,
    Quality,
    Integration,
    Parsing,
    PatternMatching,
    RuleDefinition
}
```

### Status
```rust
pub enum Status {
    Pending,
    InProgress,
    Migrated,
    Verified,
    Failed,
    Skipped
}
```

### Platform
```rust
pub enum Platform {
    Linux,
    MacOS,
    Windows,
    All
}
```

## Validation Rules

### Path Validation
- **Target paths must be absolute**: Ensure deterministic migration
- **No circular dependencies**: Prevent infinite loops during migration
- **Unique target paths**: Prevent file conflicts during migration
- **Valid naming conventions**: Follow kebab-case for directories, camelCase for files

### Dependency Validation
- **All dependencies must exist**: Referenced assets must be found
- **Forward references allowed**: Scripts can depend on fixtures that come after them
- **No cross-platform conflicts**: Dependencies must be compatible across target platforms

### Content Validation
- **File integrity verification**: SHA-256 checksums before and after migration
- **Permission preservation**: Executable bits must be maintained
- **Encoding validation**: Text files must maintain UTF-8 encoding

## State Transitions

### Asset Migration Flow

```
Pending → InProgress → Migrated → Verified
   ↓          ↓          ↓         ↓
 Failed    Failed    Failed    Complete
```

**Transition Triggers**:
- **Pending → InProgress**: Migration script starts processing asset
- **InProgress → Migrated**: File successfully copied to target location
- **Migrated → Verified**: Post-migration validation passes
- **Any → Failed**: Error occurs during processing

### Error Recovery

**Retry Strategy**:
- Automatic retry for transient file system errors (max 3 attempts)
- Manual intervention required for validation failures
- Rollback capability for failed migrations

## Data Relationships

### Composition Relationships
- `DirectoryStructure` contains `CategoryDefinition`
- `TestCase` contains multiple `TestAsset` instances
- `TestScript` has multiple `Argument` definitions

### Association Relationships
- `TestAsset` references `Language` and `Category`
- `TestCase` associated with multiple `Language` instances
- `TestScript` supports multiple `Platform` instances

### Dependency Relationships
- `TestAsset` has dependency relationships with other `TestAsset` instances
- Dependency graphs must be acyclic
- Migration order respects dependency hierarchy

## Data Format Specifications

### Asset Registry Format
```yaml
assets:
  - id: "script-validate-001"
    name: "Main validation script"
    type: "SCRIPT"
    current_path: "/tests/validate.sh"
    target_path: "/newtest/scripts/runners/validate.sh"
    language: "Bash"
    category: "VALIDATION"
    status: "PENDING"
    dependencies:
      - "config-test-001"
      - "fixture-common-001"
    metadata:
      platforms: ["Linux", "MacOS", "Windows"]
      execution_order: 1
      exit_codes:
        success: 0
        failure: 1
```

### Migration Plan Format
```yaml
migration_plan:
  version: "1.0"
  created: "2025-12-03"
  phases:
    - name: "preparation"
      description: "Create directory structure"
      assets: ["directory-structure-001"]
    - name: "script_migration"
      description: "Migrate test scripts"
      assets: ["script-validate-001", "script-runner-001"]
    - name: "content_migration"
      description: "Migrate test cases"
      assets: ["testcase-python-001", "testcase-java-001"]
```

## Performance Considerations

### Indexing Strategy
- **Path-based indexing**: Fast lookup by current or target paths
- **Category-based indexing**: Efficient filtering by functional category
- **Language-based indexing**: Quick access to language-specific assets

### Memory Management
- **Streaming migration**: Process large directories in chunks
- **Lazy loading**: Load asset metadata only when needed
- **Garbage collection**: Clean up temporary files during migration

### Scalability Limits
- **Maximum file count**: 10,000 assets per migration batch
- **Maximum file size**: 100MB per individual asset
- **Maximum path length**: 4096 characters (POSIX limit)

## Security Considerations

### File System Security
- **Permission preservation**: Maintain original file permissions
- **Ownership preservation**: Keep original file owners where possible
- **Symlink validation**: Verify symlink targets are within project bounds

### Content Security
- **Executable validation**: Verify only intended files are marked executable
- **Path traversal prevention**: Validate all target paths are within project
- **Content integrity**: Verify file contents during migration using checksums

## Audit Trail

### Migration Events
```rust
pub struct MigrationEvent {
    timestamp: DateTime<Utc>,
    asset_id: String,
    event_type: MigrationEventType,
    details: HashMap<String, String>,
    success: bool,
    error_message: Option<String>
}

pub enum MigrationEventType {
    MigrationStarted,
    FileCopied,
    ValidationStarted,
    ValidationCompleted,
    MigrationCompleted,
    MigrationFailed
}
```

### Reporting Format
- **JSON reports**: Machine-readable migration logs
- **Markdown reports**: Human-readable migration summaries
- **CSV exports**: Asset inventory for external analysis