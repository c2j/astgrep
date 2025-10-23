# astgrep Semgrep Rule Syntax Enhancement Plan

## 📋 Current Implementation Status

Based on analysis of the codebase and comparison with [Semgrep's official rule syntax](https://semgrep.dev/docs/writing-rules/rule-syntax), here's the current support status:

### ✅ Currently Supported Features

#### Required Fields
- [x] `id` - Rule identifier
- [x] `message` - Rule message
- [x] `severity` - Error levels (ERROR, WARNING, INFO, CRITICAL)
- [x] `languages` - Language specification
- [x] `pattern` - Basic pattern matching
- [x] `patterns` - Logical AND of multiple patterns
- [x] `pattern-either` - Logical OR of multiple patterns
- [x] `pattern-regex` - PCRE2 regex patterns

#### Optional Fields
- [x] `fix` - Simple autofix functionality
- [x] `metadata` - Arbitrary user-provided data
- [x] `pattern-inside` - Context-sensitive matching
- [x] `pattern-not` - Exclusion patterns
- [x] `metavariable-regex` - Metavariable regex constraints
- [x] `metavariable-pattern` - Metavariable pattern matching
- [x] `metavariable-comparison` - Basic metavariable comparisons

#### Advanced Features
- [x] Dataflow analysis (sources, sinks, sanitizers)
- [x] Confidence levels (HIGH, MEDIUM, LOW)
- [x] Rule validation and error handling
- [x] Multiple language support

### ❌ Missing Critical Features

#### Core Pattern Operators
- [ ] `pattern-not-inside` - Exclude findings inside patterns
- [ ] `pattern-not-regex` - Filter results using regex
- [ ] `focus-metavariable` - Focus on specific metavariable regions

#### Advanced Metavariable Features
- [ ] `metavariable-name` - Module/namespace constraints (requires name resolution)
- [ ] Enhanced `metavariable-comparison` with full Python expression support
- [ ] Multiple focus metavariables with union/intersection semantics

#### Rule Configuration
- [ ] `options` - Matching feature configuration
- [ ] `min-version` / `max-version` - Version compatibility
- [ ] `paths` - Include/exclude file paths
- [ ] `category` - Rule categorization

#### Language Support Gaps
- [ ] Generic pattern matching options
- [ ] Language-specific pattern optimizations
- [ ] Cross-file analysis support

## 🎯 Enhancement Roadmap

### Phase 1: Core Missing Operators (Priority: HIGH)

#### 1.1 Pattern-Not-Inside Implementation
**Files to modify:**
- `crates/cr-rules/src/types.rs` - Add `NotInside` variant to `PatternType`
- `crates/cr-rules/src/parser.rs` - Parse `pattern-not-inside` syntax
- `crates/cr-matcher/src/advanced_matcher.rs` - Implement matching logic

**Implementation:**
```rust
// Add to PatternType enum
NotInside(Box<Pattern>),

// Parser logic
else if let Some(pattern_not_inside) = self.get_optional_string_field(pattern_obj, "pattern-not-inside") {
    Pattern::not_inside(Pattern::simple(pattern_not_inside))
}
```

#### 1.2 Pattern-Not-Regex Implementation
**Files to modify:**
- `crates/cr-rules/src/types.rs` - Add `NotRegex` variant
- `crates/cr-rules/src/parser.rs` - Parse `pattern-not-regex`
- `crates/cr-matcher/src/advanced_matcher.rs` - Implement regex filtering

#### 1.3 Focus-Metavariable Enhancement
**Files to modify:**
- `crates/cr-rules/src/parser.rs` - Parse `focus-metavariable` arrays
- `crates/cr-matcher/src/advanced_matcher.rs` - Implement focus logic
- `crates/cr-core/src/patterns.rs` - Support multiple focus variables

### Phase 2: Rule Configuration (Priority: MEDIUM)

#### 2.1 Options Support
**Implementation needed:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleOptions {
    pub ac_matching: bool,
    pub attr_expr: bool,
    pub constant_propagation: bool,
    pub symmetric_eq: bool,
    pub interfile: bool,
    // ... other options
}
```

#### 2.2 Paths Configuration
**Implementation needed:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathsConfig {
    pub include: Vec<String>,
    pub exclude: Vec<String>,
}
```

#### 2.3 Version Constraints
**Implementation needed:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionConstraints {
    pub min_version: Option<String>,
    pub max_version: Option<String>,
}
```

### Phase 3: Advanced Features (Priority: LOW)

#### 3.1 Metavariable-Name Support
**Requirements:**
- Name resolution engine
- Module/namespace tracking
- Cross-file analysis capability

#### 3.2 Enhanced Metavariable-Comparison
**Features to add:**
- Full Python expression evaluation
- Date/time functions (`today()`, `strptime()`)
- Advanced string operations
- List operations with `in`/`not in`

#### 3.3 Generic Pattern Matching
**Implementation needed:**
- Comment style recognition
- Ellipsis span configuration
- Language-agnostic pattern matching

## 📝 Implementation Plan

### Week 1-2: Core Operators
1. Implement `pattern-not-inside`
2. Implement `pattern-not-regex`
3. Enhance `focus-metavariable` support
4. Add comprehensive tests

### Week 3-4: Rule Configuration
1. Add `options` parsing and validation
2. Implement `paths` include/exclude logic
3. Add version constraint checking
4. Update rule validator

### Week 5-6: Advanced Features
1. Design name resolution architecture
2. Implement enhanced metavariable comparison
3. Add generic pattern matching options
4. Performance optimization

### Week 7-8: Testing & Documentation
1. Comprehensive test suite
2. Compatibility validation with official Semgrep
3. Documentation updates
4. Performance benchmarking

## 🧪 Testing Strategy

### Compatibility Tests
- Test against official Semgrep rule repository
- Validate identical results for supported features
- Performance comparison benchmarks

### Feature Tests
- Unit tests for each new operator
- Integration tests for complex rule combinations
- Error handling and edge case validation

### Regression Tests
- Ensure existing functionality remains intact
- Backward compatibility with current rule format
- Performance regression detection

## 📊 Success Metrics

1. **Feature Completeness**: 95%+ compatibility with Semgrep rule syntax
2. **Performance**: Maintain current 10-18x speed advantage
3. **Reliability**: Zero regression in existing functionality
4. **Usability**: Seamless migration from existing rules

## 🔄 Migration Path

### For Existing Users
1. Current rules continue to work unchanged
2. Gradual adoption of new features
3. Clear migration documentation
4. Automated rule validation tools

### For New Users
1. Full Semgrep compatibility from day one
2. Enhanced features beyond standard Semgrep
3. Comprehensive documentation and examples
4. IDE integration and tooling support

## 🔧 Detailed Implementation Specifications

### Pattern-Not-Inside Implementation

**Code Changes Required:**

1. **Type Definition** (`crates/cr-rules/src/types.rs`):
```rust
// Add to PatternType enum
NotInside(Box<Pattern>),

// Add constructor method
impl Pattern {
    pub fn not_inside(inner_pattern: Pattern) -> Self {
        Self {
            pattern_type: PatternType::NotInside(Box::new(inner_pattern)),
            metavariable_pattern: None,
            conditions: Vec::new(),
            focus: None,
        }
    }
}
```

2. **Parser Logic** (`crates/cr-rules/src/parser.rs`):
```rust
// Add to parse_single_pattern method
else if let Some(pattern_not_inside) = self.get_optional_string_field(pattern_obj, "pattern-not-inside") {
    Pattern::not_inside(Pattern::simple(pattern_not_inside))
}
```

3. **Matcher Implementation** (`crates/cr-matcher/src/advanced_matcher.rs`):
```rust
PatternType::NotInside(inner_pattern) => {
    // Find all matches of the inner pattern
    let inner_matches = self.find_all_matches(inner_pattern, root)?;

    // Filter out any matches that fall within inner pattern ranges
    let filtered_matches = current_matches.into_iter()
        .filter(|m| !self.is_inside_any_range(m, &inner_matches))
        .collect();

    Ok(filtered_matches)
}
```

### Options Configuration Implementation

**Complete Options Structure:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleOptions {
    // Matching behavior
    pub ac_matching: Option<bool>,
    pub attr_expr: Option<bool>,
    pub commutative_boolop: Option<bool>,
    pub constant_propagation: Option<bool>,
    pub decorators_order_matters: Option<bool>,
    pub implicit_return: Option<bool>,
    pub symmetric_eq: Option<bool>,
    pub vardef_assign: Option<bool>,
    pub xml_attrs_implicit_ellipsis: Option<bool>,

    // Generic mode options
    pub generic_comment_style: Option<String>,
    pub generic_ellipsis_max_span: Option<u32>,

    // Taint analysis options
    pub taint_assume_safe_functions: Option<bool>,
    pub taint_assume_safe_indexes: Option<bool>,

    // Cross-file analysis
    pub interfile: Option<bool>,
}

impl Default for RuleOptions {
    fn default() -> Self {
        Self {
            ac_matching: Some(true),
            attr_expr: Some(true),
            commutative_boolop: Some(false),
            constant_propagation: Some(true),
            decorators_order_matters: Some(false),
            implicit_return: Some(true),
            symmetric_eq: Some(false),
            vardef_assign: Some(true),
            xml_attrs_implicit_ellipsis: Some(true),
            generic_comment_style: None,
            generic_ellipsis_max_span: Some(10),
            taint_assume_safe_functions: Some(false),
            taint_assume_safe_indexes: Some(false),
            interfile: Some(false),
        }
    }
}
```

### Enhanced Metavariable-Comparison

**Extended Comparison Support:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetavariableComparison {
    pub metavariable: String,
    pub comparison: String,  // Python expression
    pub base: Option<u32>,   // Integer base (legacy)
    pub strip: Option<bool>, // Strip quotes (legacy)
}

// Supported functions in comparison expressions:
// - int(), str(), len()
// - today(), strptime()
// - re.match()
// - Boolean operators: and, or, not
// - Arithmetic: +, -, *, /, %
// - Comparison: ==, !=, <, <=, >, >=
// - Membership: in, not in
```

### Paths Configuration

**File Path Filtering:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathsConfig {
    pub include: Option<Vec<String>>,
    pub exclude: Option<Vec<String>>,
}

impl PathsConfig {
    pub fn should_include_file(&self, file_path: &str) -> bool {
        // Check exclude patterns first (they take precedence)
        if let Some(ref excludes) = self.exclude {
            for pattern in excludes {
                if self.matches_glob_pattern(file_path, pattern) {
                    return false;
                }
            }
        }

        // Check include patterns
        if let Some(ref includes) = self.include {
            for pattern in includes {
                if self.matches_glob_pattern(file_path, pattern) {
                    return true;
                }
            }
            return false; // If includes specified, must match one
        }

        true // No includes specified, include by default
    }

    fn matches_glob_pattern(&self, path: &str, pattern: &str) -> bool {
        // Use wcmatch-compatible glob matching
        // Support ** for recursive directory matching
        // Support * for single-level wildcards
        // Support ? for single character matching
        todo!("Implement glob pattern matching")
    }
}
```

## 📚 Example Rules Using New Features

### Pattern-Not-Inside Example
```yaml
rules:
  - id: unsafe-eval-outside-sandbox
    message: "eval() used outside of sandbox context"
    languages: [javascript]
    severity: ERROR
    patterns:
      - pattern: eval($CODE)
      - pattern-not-inside: |
          function sandbox() {
            ...
            eval($CODE)
            ...
          }
```

### Pattern-Not-Regex Example
```yaml
rules:
  - id: detect-only-foo-package
    message: "Found foo package (not foo-bar or bar-foo)"
    languages: [regex]
    severity: ERROR
    patterns:
      - pattern-regex: foo
      - pattern-not-regex: foo-
      - pattern-not-regex: -foo
```

### Focus-Metavariable Example
```yaml
rules:
  - id: focus-on-dangerous-arg
    message: "Dangerous argument: $ARG"
    languages: [python]
    severity: ERROR
    patterns:
      - pattern: |
          def dangerous_function($PARAM1, $ARG, $PARAM2):
            ...
      - focus-metavariable: $ARG
      - metavariable-regex:
          metavariable: $ARG
          regex: ".*(password|secret|key).*"
```

### Options Configuration Example
```yaml
rules:
  - id: symmetric-comparison
    message: "Symmetric comparison detected"
    languages: [python]
    severity: INFO
    pattern: $X == $Y
    options:
      symmetric_eq: true
      constant_propagation: false
```

### Paths Configuration Example
```yaml
rules:
  - id: test-file-only-rule
    message: "This rule only applies to test files"
    languages: [python]
    severity: WARNING
    pattern: assert $CONDITION
    paths:
      include:
        - "*_test.py"
        - "tests/**/*.py"
      exclude:
        - "tests/fixtures/**"
```

### Enhanced Metavariable-Comparison Example
```yaml
rules:
  - id: large-file-permissions
    message: "File permissions too permissive"
    languages: [python]
    severity: ERROR
    patterns:
      - pattern: os.chmod($FILE, $PERMS)
      - metavariable-comparison:
          metavariable: $PERMS
          comparison: int($PERMS, 8) > 0o644
```

## 🧪 Comprehensive Test Plan

### Unit Tests for New Features

#### Pattern-Not-Inside Tests
```python
# Test file: tests/advanced_patterns/pattern_not_inside_test.py
def test_function_outside_class():
    # Should match - function not inside class
    def standalone_function():
        return "test"

class MyClass:
    def method_inside_class(self):
        # Should NOT match - function inside class
        return "test"
```

```yaml
# Test rule: tests/advanced_patterns/pattern_not_inside_test.yaml
rules:
  - id: function-outside-class
    message: "Function defined outside class"
    languages: [python]
    severity: INFO
    patterns:
      - pattern: |
          def $FUNC(...):
            ...
      - pattern-not-inside: |
          class $CLASS:
            ...
```

#### Focus-Metavariable Tests
```python
# Test file: tests/advanced_patterns/focus_metavariable_test.py
def process_data(username, password, email):
    # Should focus only on 'password' parameter
    return authenticate(username, password, email)
```

```yaml
# Test rule: tests/advanced_patterns/focus_metavariable_test.yaml
rules:
  - id: focus-sensitive-param
    message: "Sensitive parameter: $PARAM"
    languages: [python]
    severity: WARNING
    patterns:
      - pattern: |
          def $FUNC(..., $PARAM, ...):
            ...
      - focus-metavariable: $PARAM
      - metavariable-regex:
          metavariable: $PARAM
          regex: ".*(password|secret|key).*"
```

### Integration Tests

#### Complex Rule Combinations
```yaml
rules:
  - id: complex-security-check
    message: "Complex security vulnerability"
    languages: [python]
    severity: ERROR
    patterns:
      - pattern: $OBJ.$METHOD($ARG)
      - pattern-inside: |
          def $FUNC(...):
            ...
      - pattern-not-inside: |
          try:
            ...
          except:
            ...
      - metavariable-regex:
          metavariable: $METHOD
          regex: "^(execute|eval|exec)$"
      - metavariable-pattern:
          metavariable: $ARG
          patterns:
            - pattern: $VAR + $INPUT
            - pattern-not: sanitize($INPUT)
    options:
      constant_propagation: true
      interfile: false
    paths:
      include: ["src/**/*.py"]
      exclude: ["src/tests/**"]
```

### Performance Tests

#### Benchmark New Features
```rust
// File: crates/cr-matcher/benches/new_features_bench.rs
#[bench]
fn bench_pattern_not_inside(b: &mut Bencher) {
    let rule = create_pattern_not_inside_rule();
    let code = load_large_test_file();

    b.iter(|| {
        rule.analyze(&code)
    });
}

#[bench]
fn bench_focus_metavariable(b: &mut Bencher) {
    let rule = create_focus_metavariable_rule();
    let code = load_large_test_file();

    b.iter(|| {
        rule.analyze(&code)
    });
}
```

### Compatibility Tests

#### Semgrep Compatibility Validation
```bash
#!/bin/bash
# Script: scripts/validate_semgrep_compatibility.sh

# Test against official Semgrep rules
for rule_file in tests/semgrep-rules/*.yaml; do
    echo "Testing $rule_file"

    # Run with our implementation
    ./target/release/cr-cli scan --rules "$rule_file" tests/targets/

    # Run with official Semgrep (if available)
    semgrep --config "$rule_file" tests/targets/

    # Compare results
    diff_results
done
```

---

**Next Steps:**
1. Review and approve this enhancement plan
2. Set up development milestones
3. Begin Phase 1 implementation
4. Establish testing infrastructure
