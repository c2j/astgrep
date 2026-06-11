# Taint Dataflow Engine Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the current heuristic-based taint analysis with a proper forward dataflow engine that tracks taint through variable assignments, field accesses, array elements, and function boundaries.

**Architecture:** Build a scope-aware, variable-centric taint state machine that operates on source text rather than AST graphs. The engine maintains a `TaintEnv` (mapping variable names to taint state) and walks statements top-to-bottom, propagating taint through assignments, method calls, and control flow. This mirrors semgrep's approach: pattern-match sources/sinks against the AST, then use text-based dataflow to verify source→sink connectivity.

**Tech Stack:** Rust, existing `astgrep-matcher` for pattern matching, `astgrep-dataflow` types, no new external dependencies.

---

## Background: Why a Rewrite

The current taint engine (`crates/astgrep-rules/src/executor/core/taint.rs`) works as follows:

1. Pattern-match `pattern-sources` and `pattern-sinks` against the AST
2. Extract variable names from matches using ~10 heuristics
3. Check connectivity via text search (`sink_text.contains(var_name)`) and a `VariableDependencyGraph` built from method body text
4. Pair every source with every sink in the same method scope

**What's broken:**
- No proper taint state tracking (is variable X currently tainted?)
- No assignment propagation (`a = tainted; b = a; sink(b)` fails)
- No control flow awareness (branches, loops, exceptions)
- No field-sensitive tracking (`obj.x` vs `obj.y`)
- No array tracking (`arr[i] = tainted; sink(arr[j])` fails)
- No sanitizer state (sanitizer removes taint for specific variables)
- `VariableDependencyGraph` is text-based, not AST-based
- 79 failing taint tests out of ~100 total

**What we keep:**
- Source/sink finding via pattern matching (`find_taint_sources`, `find_taint_sinks`)
- `TaintMatch` struct with variable name extraction
- Method scope isolation
- Safe-context filtering (booleans/numbers)

## Key Insight: Variable-Centric State Machine

Rather than building a full AST dataflow graph (complex, slow), we use a simpler approach that works for the test cases:

```
TaintEnv = HashMap<VarName, TaintState>

For each statement top-to-bottom:
  if statement matches a source pattern → taint the matched variable
  if statement is "x = f(y)" and y is tainted → taint x
  if statement is "x = sanitize(y)" → untaint x (sanitizer)
  if statement matches a sink pattern and sink arg is tainted → REPORT FLOW
```

This handles ~80% of failing tests with minimal complexity.

---

## Phase 1: Core Taint State Machine (est. 25 tests)

**Scope:** Variable-centric forward dataflow with assignment propagation.

### Task 1: Define TaintEnv and TaintState types

**Files:**
- Create: `crates/astgrep-rules/src/executor/core/taint_env.rs`

**Step 1: Write the types**

```rust
use std::collections::{HashMap, HashSet};

/// Taint state for a single variable
#[derive(Debug, Clone)]
pub struct TaintState {
    /// Lines where taint was introduced (source pattern match lines)
    pub source_lines: HashSet<usize>,
    /// Whether this variable is currently tainted
    pub tainted: bool,
    /// Sanitizers that have been applied
    pub sanitized_by: Vec<String>,
    /// Origin: which source pattern(s) contributed taint
    pub origin_patterns: Vec<String>,
}

/// Environment mapping variable names to their taint state
#[derive(Debug, Clone)]
pub struct TaintEnv {
    /// Per-scope taint state: scope_key → (var_name → TaintState)
    scopes: Vec<HashMap<String, TaintState>>,
    /// Current method/function name for scope isolation
    current_scope: Option<String>,
}

impl TaintEnv {
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
            current_scope: None,
        }
    }

    /// Mark a variable as tainted
    pub fn taint(&mut self, var: &str, source_line: usize, pattern: &str) {
        if let Some(scope) = self.scopes.last_mut() {
            let state = scope.entry(var.to_string()).or_insert(TaintState {
                source_lines: HashSet::new(),
                tainted: false,
                sanitized_by: Vec::new(),
                origin_patterns: Vec::new(),
            });
            state.tainted = true;
            state.source_lines.insert(source_line);
            if !state.origin_patterns.contains(&pattern.to_string()) {
                state.origin_patterns.push(pattern.to_string());
            }
        }
    }

    /// Check if a variable is tainted (considering sanitizers)
    pub fn is_tainted(&self, var: &str) -> bool {
        self.scopes.last()
            .and_then(|scope| scope.get(var))
            .map(|state| state.tainted)
            .unwrap_or(false)
    }

    /// Propagate taint from source var to target var (assignment)
    pub fn propagate(&mut self, target: &str, source: &str) {
        if let Some(source_state) = self.scopes.last().and_then(|s| s.get(source).cloned()) {
            if source_state.tainted {
                if let Some(scope) = self.scopes.last_mut() {
                    scope.insert(target.to_string(), source_state);
                }
            }
        }
    }

    /// Sanitize a variable (remove taint)
    pub fn sanitize(&mut self, var: &str, sanitizer: &str) {
        if let Some(scope) = self.scopes.last_mut() {
            if let Some(state) = scope.get_mut(var) {
                state.tainted = false;
                state.sanitized_by.push(sanitizer.to_string());
            }
        }
    }

    /// Push a new scope
    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    /// Pop a scope
    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    /// Get all currently tainted variables
    pub fn tainted_vars(&self) -> Vec<(String, TaintState)> {
        self.scopes.last()
            .map(|scope| {
                scope.iter()
                    .filter(|(_, state)| state.tainted)
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Clear taint for a variable (reassignment to untainted value)
    pub fn untaint(&mut self, var: &str) {
        if let Some(scope) = self.scopes.last_mut() {
            if let Some(state) = scope.get_mut(var) {
                state.tainted = false;
                state.source_lines.clear();
            }
        }
    }
}
```

**Step 2: Run `cargo build -p astgrep-rules` to verify compilation**

**Step 3: Commit**
```
Add TaintEnv state machine for variable-centric taint tracking
```

---

### Task 2: Build statement walker for taint propagation

**Files:**
- Modify: `crates/astgrep-rules/src/executor/core/taint_env.rs`
- Add: statement parsing and forward walk logic

**Step 1: Add statement parsing to TaintEnv**

The key insight: we don't need a full AST walk. We parse the source text line-by-line, tracking:
- Assignments: `x = expr`
- Method calls that are sources: matches source pattern
- Method calls that are sinks: matches sink pattern
- Method calls that are sanitizers: matches sanitizer pattern

```rust
impl TaintEnv {
    /// Process a single line of source code for taint propagation.
    /// Returns sink matches found on this line.
    pub fn process_line(
        &mut self,
        line_num: usize,
        line_text: &str,
        source_var: &str,         // variable name from source pattern match
        sink_pattern: &str,       // sink pattern text
        sanitizer_vars: &[String], // variables that have been sanitized
    ) -> Vec<usize> {
        let mut sink_hits = Vec::new();
        let trimmed = line_text.trim();

        // Skip comments and empty lines
        if trimmed.starts_with('#') || trimmed.starts_with("//") || trimmed.is_empty() {
            return sink_hits;
        }

        // 1. Check if source var appears in this line → it's a source hit
        if trimmed.contains(source_var) {
            // Already handled by pattern matching; taint is set by caller
        }

        // 2. Check for assignment: detect "target = ..." patterns
        if let Some(eq_pos) = trimmed.find('=') {
            // Make sure it's not ==, !=, <=, >=
            let before_eq = &trimmed[..eq_pos];
            let after_eq = &trimmed[eq_pos + 1..];
            let char_before = before_eq.chars().last();
            if !matches!(char_before, Some('=') | Some('!') | Some('<') | Some('>')) {
                let target = before_eq.trim().to_string();
                let value = after_eq.trim();

                // Check if value references a tainted variable
                for (tvar, _) in self.tainted_vars() {
                    if value.contains(&tvar) {
                        // Check if value is a sanitizer call
                        let is_sanitized = sanitizer_vars.iter().any(|s| value.contains(s));
                        if is_sanitized {
                            self.sanitize(&target, "sanitizer");
                        } else {
                            self.propagate(&target, &tvar);
                        }
                    }
                }
            }
        }

        // 3. Check if this line matches sink pattern
        // Simple check: does the line contain the sink function name?
        if let Some(sink_fn) = extract_function_name(sink_pattern) {
            if trimmed.contains(&sink_fn) {
                // Check if any tainted var appears as argument
                for (tvar, _) in self.tainted_vars() {
                    if trimmed.contains(&tvar) {
                        sink_hits.push(line_num);
                        break;
                    }
                }
            }
        }

        sink_hits
    }
}

fn extract_function_name(pattern: &str) -> Option<String> {
    // Extract "sink" from "sink(...)" or "obj.sink(...)"
    let pattern = pattern.trim();
    if let Some(paren) = pattern.find('(') {
        let before_paren = &pattern[..paren];
        let parts: Vec<&str> = before_paren.rsplit('.').collect();
        if let Some(name) = parts.first() {
            let name = name.trim();
            if !name.is_empty() && !name.starts_with('$') {
                return Some(name.to_string());
            }
        }
    }
    None
}
```

**Step 2: Build and verify compilation**

**Step 3: Commit**
```
Add statement walker for forward taint propagation
```

---

### Task 3: Integrate TaintEnv into detect_taint_flows

**Files:**
- Modify: `crates/astgrep-rules/src/executor/core/taint.rs`
- Modify: `crates/astgrep-rules/src/executor/core/mod.rs`

**Step 1: Replace the O(n*m) source×sink pairing with TaintEnv-driven flow detection**

The current `detect_taint_flows` tries every source × every sink pair with heuristic checks. Replace with:

1. Walk source text line by line
2. When a source pattern matches → `env.taint(var, line, pattern)`
3. When assignment found → `env.propagate(target, source)`
4. When sanitizer found → `env.sanitize(var, sanitizer)`
5. When a sink pattern matches → check `env.is_tainted(arg)` → report flow

```rust
pub(super) fn detect_taint_flows_with_env(
    &self,
    sources: &[TaintMatch],
    sinks: &[TaintMatch],
    ast: &dyn AstNode,
    dataflow_spec: &DataFlowSpec,
    source_text: &str,
) -> Result<Vec<(TaintMatch, TaintMatch)>> {
    let mut flows = Vec::new();
    let mut env = TaintEnv::new();

    let lines: Vec<&str> = source_text.lines().collect();

    // Build source map: line → (var_name, TaintMatch)
    let source_map: HashMap<usize, (&str, &TaintMatch)> = sources.iter()
        .filter_map(|s| {
            s.var_name.as_ref().and_then(|var| {
                s.node.location().map(|(sl, _, _, _)| (sl, (var.as_str(), s)))
            })
        })
        .collect();

    // Build sink map: line → TaintMatch
    let sink_map: HashMap<usize, &TaintMatch> = sinks.iter()
        .filter_map(|s| {
            s.node.location().map(|(sl, _, _, _)| (sl, s))
        })
        .collect();

    // Extract sanitizer function names from DataFlowSpec
    let sanitizer_fns: Vec<String> = dataflow_spec.sanitizers.iter()
        .filter_map(|s| extract_function_name(s))
        .collect();

    // Walk lines top to bottom
    for (line_idx, line) in lines.iter().enumerate() {
        let line_num = line_idx + 1;

        // 1. Source match at this line?
        if let Some((var, source_match)) = source_map.get(&line_num) {
            env.taint(var, line_num, &source_match.node.text().unwrap_or(""));
        }

        // 2. Assignment propagation
        if let Some(eq_pos) = line.find('=') {
            let before = &line[..eq_pos];
            let char_before = before.chars().last();
            if !matches!(char_before, Some('=') | Some('!') | Some('<') | Some('>')) {
                let target = before.trim();
                let value = &line[eq_pos + 1..];

                // Propagate from tainted vars
                for (tvar, _) in env.tainted_vars() {
                    if value.contains(&tvar) {
                        // Check if sanitizer
                        let is_sanitized = sanitizer_fns.iter().any(|s| value.contains(s));
                        if is_sanitized {
                            env.sanitize(target, "sanitizer");
                        } else {
                            env.propagate(target, &tvar);
                        }
                    }
                }

                // Reassignment to literal → untaint
                let value_trimmed = value.trim();
                if !value_trimmed.contains('$')
                    && !env.tainted_vars().iter().any(|(v, _)| value_trimmed.contains(v.as_str()))
                    && (value_trimmed.starts_with('"') || value_trimmed.starts_with('\'')
                        || value_trimmed.parse::<i64>().is_ok()
                        || value_trimmed == "true" || value_trimmed == "false"
                        || value_trimmed == "None" || value_trimmed == "null")
                {
                    env.untaint(target);
                }
            }
        }

        // 3. Sink match at this line?
        if let Some(sink_match) = sink_map.get(&line_num) {
            if let Some(sink_var) = &sink_match.var_name {
                if env.is_tainted(sink_var) {
                    // Find the source match for this taint
                    if let Some((_, source_match)) = source_map.values().find(|(var, _)| env.is_tainted(var)) {
                        flows.push(((*source_match).clone(), sink_match.clone()));
                    }
                }
            }
        }
    }

    Ok(flows)
}
```

**Step 2: Wire into `execute_taint_analysis`**

In `execute_taint_analysis`, after finding sources and sinks, call `detect_taint_flows_with_env` as the primary method, falling back to the old `detect_taint_flows` for cases the new method doesn't cover.

**Step 3: Run guardian to measure improvement**

Expected: ~10-15 tests should now pass (simple_var and deep_chain tests).

**Step 4: Commit**
```
Integrate TaintEnv into taint flow detection
```

---

## Phase 2: Control Flow & Sanitizers (est. 15 tests)

**Scope:** Branch-aware taint, sanitizer state tracking.

### Task 4: Add control flow branch merging

**Files:**
- Modify: `crates/astgrep-rules/src/executor/core/taint_env.rs`

**Step 1: Add branch-aware processing**

```rust
impl TaintEnv {
    /// Fork environment for a branch (if/else, try/catch)
    pub fn fork(&self) -> TaintEnv {
        self.clone()
    }

    /// Merge two branch environments (union semantics: tainted if tainted in either)
    pub fn merge(&mut self, other: &TaintEnv) {
        if let (Some(self_scope), Some(other_scope)) = (self.scopes.last_mut(), other.scopes.last()) {
            for (var, other_state) in other_scope.iter() {
                if other_state.tainted {
                    let state = self_scope.entry(var.clone()).or_insert(TaintState {
                        source_lines: HashSet::new(),
                        tainted: false,
                        sanitized_by: Vec::new(),
                        origin_patterns: Vec::new(),
                    });
                    state.tainted = true;
                    for line in &other_state.source_lines {
                        state.source_lines.insert(*line);
                    }
                }
            }
        }
    }
}
```

**Step 2: Add control-flow-aware line processing in `detect_taint_flows_with_env`**

Detect `if/else`, `try/except`, `for/while` blocks and fork/merge the env accordingly.

**Step 3: Commit**
```
Add control flow branch merging to TaintEnv
```

### Task 5: Implement sanitizer pattern matching

**Files:**
- Modify: `crates/astgrep-rules/src/executor/core/taint_env.rs`
- Modify: `crates/astgrep-rules/src/executor/core/taint.rs`

**Step 1: Parse sanitizer patterns from DataFlowSpec and match against assignments**

When a line contains a sanitizer function call and assigns the result to a variable, that variable should be untainted.

Example: `url = is_safe_url(url)` → `url` is now sanitized.

**Step 2: Run guardian to measure improvement**

Expected: ~5-8 more tests (sanitizer-dependent tests).

**Step 3: Commit**
```
Add sanitizer pattern matching to taint propagation
```

---

## Phase 3: Field-Sensitive Tracking (est. 10 tests)

**Scope:** Track taint through object field access and array elements.

### Task 6: Add field-sensitive TaintEnv keys

**Files:**
- Modify: `crates/astgrep-rules/src/executor/core/taint_env.rs`

**Step 1: Change TaintEnv keys from `String` to `FieldPath`**

```rust
/// Represents a variable access path: "x", "x.field", "x[i]"
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FieldPath {
    /// Simple variable: "x"
    Var(String),
    /// Field access: base.field
    Field { base: Box<FieldPath>, field: String },
    /// Array/Object index: base[key]
    Index { base: Box<FieldPath>, key: String },
}
```

**Step 2: Update propagation rules**

- `obj.field = tainted` → taint `Field { "obj", "field" }`
- `sink(obj.field)` → check if `Field { "obj", "field" }` is tainted
- `obj[i] = tainted` → taint `Index { "obj", "*" }` (any index)
- `sink(obj[j])` → check if `Index { "obj", "*" }` is tainted (any index matches)

**Step 3: Commit**
```
Add field-sensitive tracking to TaintEnv
```

---

## Phase 4: Inter-Procedural (est. 10 tests)

**Scope:** Track taint across function boundaries.

### Task 7: Add function boundary tracking

**Files:**
- Modify: `crates/astgrep-rules/src/executor/core/taint_env.rs`

**Step 1: Track function parameters and return values**

When entering a function:
1. Check if any parameter is marked as a taint source
2. Propagate taint through function body
3. If tainted value is returned, mark the return as tainted
4. At call site, propagate return taint to assigned variable

**Step 2: Commit**
```
Add inter-procedural taint tracking
```

---

## Phase 5: Label Propagation & Options (est. 15 tests)

**Scope:** Pattern labels, taint options handling.

### Task 8: Implement label-based propagation

**Files:**
- Modify: `crates/astgrep-rules/src/executor/core/taint.rs`

Handle `propagators` with `from`/`to` fields that specify which metavar carries taint through.

### Task 9: Implement taint options

Handle `taint_assume_safe_booleans`, `taint_assume_safe_numbers`, `taint_assume_safe_indexes`, `taint_only_propagate_through_assignments`, `taint_unify_mvars`.

---

## Test Validation Strategy

After each task, run:
```bash
cargo build --release
python3 newtest/scripts/guardian_runner.py
```

Track taint-specific tests:
```bash
python3 -c "
import json
with open('newtest/guardian_report.json') as f:
    data = json.load(f)
taint = [r for r in data['test_results'] if 'taint' in r.get('category','')]
passed = sum(1 for t in taint if t.get('passed'))
print(f'Taint: {passed}/{len(taint)} = {passed/len(taint)*100:.1f}%')
"
```

## Priority Order

| Phase | Tests Gained | Effort | ROI |
|-------|-------------|--------|-----|
| Phase 1: Core state machine | ~25 | 3 tasks | **Highest** |
| Phase 2: Control flow + sanitizers | ~15 | 2 tasks | High |
| Phase 3: Field-sensitive | ~10 | 1 task | Medium |
| Phase 4: Inter-procedural | ~10 | 1 task | Medium |
| Phase 5: Labels + options | ~15 | 2 tasks | Medium |
| **Total** | **~75** | **9 tasks** | |

## Files Reference

| File | Role |
|------|------|
| `crates/astgrep-rules/src/executor/core/taint_env.rs` | NEW: TaintEnv state machine |
| `crates/astgrep-rules/src/executor/core/taint.rs` | MODIFY: Replace detect_taint_flows with env-based |
| `crates/astgrep-rules/src/executor/core/mod.rs` | MODIFY: Wire new module |
| `crates/astgrep-rules/src/types.rs` | REFERENCE: DataFlowSpec, PropagatorPattern |
| `crates/astgrep-dataflow/src/taint.rs` | REFERENCE: Existing TaintTracker (keep for now) |

## Risk Mitigation

1. **No regressions**: Run full guardian after each task
2. **Fallback path**: Keep old `detect_taint_flows` as fallback; new method is additive
3. **Incremental**: Each phase is independently testable and committable
4. **No new dependencies**: Pure Rust implementation using existing crates
