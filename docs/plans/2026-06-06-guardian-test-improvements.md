# Guardian Test Score Improvement Plan (638→849+)

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Improve guardian_runner.py pass rate from 638/894 (71.4%) to ≥849/894 (95%) by fixing engine bugs in taint analysis, symbolic propagation, pattern matching, and rule execution.

**Architecture:** Six independent workstreams, each targeting a different engine subsystem. Each workstream is self-contained and can be executed independently without conflicts. Workstreams are ordered by expected test wins per effort.

**Tech Stack:** Rust, tree-sitter, regex. Build: `cargo build --release`. Test: `python3 newtest/scripts/guardian_runner.py`.

**Current State:** Branch `improve/pattern-matching-deep-expr`, 2 commits ahead of origin, score 638/894, 0 regressions.

**Score Verification Commands:**
```bash
cargo build --release 2>&1 | tail -1
python3 newtest/scripts/guardian_runner.py 2>&1 | tail -3
python3 -c "import json; r=json.load(open('newtest/guardian_report.json')); s=r['summary']; t=s['passed']+s['failed']; print(f'{s[\"passed\"]}/{t} ({100*s[\"passed\"]/t:.1f}%)')"
```

---

## Workstream A: Taint Lambda Scope Fix (~30 test wins)

**Root Cause:** `detect_taint_flows` (heuristic path) in `taint.rs:1293` pairs source and sink by method name. When source/sink are inside a lambda (anonymous class in Java), both have `Some(method_name)` but from DIFFERENT enclosing methods — the lambda body and the enclosing method. The current check `if src_method != sink_method { continue; }` (L1314) correctly skips cross-method flows, but for lambdas the source's method_name is the enclosing method while the sink's method_name is the lambda's synthetic method. They SHOULD be paired because the lambda captures the enclosing scope's tainted variable.

**Impact:** Fixes `taint_lambda1`, `taint_lambda2`, `taint_lambda4` (+11 test points from rules category, +2 from tainting_rules/java).

**Files:**
- Modify: `crates/astgrep-rules/src/executor/core/taint.rs:1304-1318`

### Task A1: Relax method scope check for lambda/anonymous class sources

**Step 1: Understand the exact failure**

For `taint_lambda1.java`, the pattern is:
- Source: `@RequestParam Map<K,V> params` in enclosing method
- Sink: `stmt.execute(query)` inside a lambda/anonymous class within that method
- Expected: flow detected (line 66, 112, 161, 213)
- Actual: flows detected at wrong lines (64, 110, 159, 257) — off by 2, suggesting the heuristic matches the method declaration line not the sink line

The heuristic path `detect_taint_flows` at L1314 does:
```rust
if let (Some(ref src_method), Some(ref sink_method)) = (&source.method_name, &sink.method_name) {
    if src_method != sink_method {
        continue; // SKIP — this is the bug for lambdas
    }
}
```

For lambdas, `source.method_name` is the enclosing method (e.g., `handleRequest`) and `sink.method_name` is the lambda's method (e.g., `run` or `apply`). They differ, so the flow is skipped.

**Step 2: Implement the fix**

In `taint.rs:1304-1318`, change the method scope check to also allow pairing when the source's method is an ancestor of the sink's method (i.e., source is in an enclosing method that contains the lambda):

```rust
// Method-level scope isolation: if both source and sink have method names,
// only pair them if they're in the same method OR if the source is in an
// enclosing method (lambda/anonymous class captures enclosing scope)
if let (Some(ref src_method), Some(ref sink_method)) =
    (&source.method_name, &sink.method_name)
{
    if src_method != sink_method {
        // Allow if source is in an enclosing method that the sink's lambda is inside.
        // Heuristic: check if the source line is before the sink line AND
        // the sink's method text contains indicators of being a lambda/anonymous class.
        let source_before_sink = source.node.location()
            .map(|(sl,_,_,_)| {
                sink.node.location().map(|(dl,_,_,_)| sl < dl).unwrap_or(false)
            })
            .unwrap_or(false);
        
        if !source_before_sink {
            eprintln!("[DEBUG] Skipping: source and sink in different methods, source after sink");
            continue;
        }
        // Source is before sink — could be enclosing scope capture.
        // Don't skip — let the dependency graph check determine if there's a real flow.
        eprintln!("[DEBUG] Allowing cross-method pair: source in '{}' (line {}), sink in '{}' — possible lambda/anonymous class capture",
            src_method,
            source.node.location().map(|(l,_,_,_)| l).unwrap_or(0),
            sink_method);
    }
}
```

**Step 3: Build and test**

```bash
cargo build --release 2>&1 | tail -1
python3 newtest/scripts/guardian_runner.py 2>&1 | tail -3
python3 -c "import json; r=json.load(open('newtest/guardian_report.json')); s=r['summary']; t=s['passed']+s['failed']; print(f'{s[\"passed\"]}/{t}')"
```

**Expected:** Score increases by ~11 (taint_lambda1/2/4 fix their line mismatches).

**Step 4: Verify no regressions**

Check that tests that were passing before still pass:
```bash
python3 -c "
import json
old = json.load(open('/tmp/wt_report_638.json'))
new = json.load(open('newtest/guardian_report.json'))
regressed = [t['test_name'] for t in new['test_results'] 
             if not t['passed'] and any(o['test_name']==t['test_name'] and o['passed'] for o in old['test_results'])]
print(f'Regressions: {len(regressed)}')
for r in regressed: print(f'  {r}')
"
```

**Step 5: Commit**

```bash
git add crates/astgrep-rules/src/executor/core/taint.rs
git commit -m "fix(taint): allow cross-method taint flows for lambda captures"
```

---

## Workstream B: Taint Param Source Extra Line Fix (~1 test win)

**Root Cause:** `taint_param_source` expects finding ONLY on line 4, but actual output includes line 9 as a false positive. The heuristic path finds a flow at line 9 because it sees the tainted variable used there, even though the env path correctly untaints it. The merge at L236-249 takes heuristic flows first, env only supplements — so the heuristic's false positive survives.

**Impact:** Fixes `taint_param_source` (+1 test, removes negative violation).

**Files:**
- Modify: `crates/astgrep-rules/src/executor/core/taint.rs:236-249`

### Task B1: Suppress heuristic flows where env explicitly untainted

**Step 1: Understand the merge logic**

Current merge (L236-249):
```rust
let mut merged: Vec<(TaintMatch, TaintMatch)> = heuristic_flows;
for flow in filtered_env {
    let sink_loc = flow.1.node.location();
    let already = merged.iter().any(|(_, s)| s.node.location() == sink_loc);
    if !already { merged.push(flow); }
}
```

The env path tracks variable state line-by-line. If env untainted a variable at line X, but heuristic still produces a flow to a sink at line Y>X, the heuristic flow is a false positive.

**Step 2: Implement the fix**

After the merge, add a post-filter that removes heuristic flows where the env path explicitly untainted the variable before the sink:

```rust
// Post-filter: remove heuristic flows where env explicitly untainted the source variable
// before reaching the sink line. This handles cases like:
//   line 4: x = tainted_source()  → env taints x
//   line 6: x = safe_value        → env untaints x
//   line 9: sink(x)               → heuristic still finds flow (false positive)
let env_untainted_lines: HashMap<String, usize> = /* collect from env */;
merged.retain(|(source, sink)| {
    if let (Some(ref var), Some((sink_line,_,_,_))) = (&source.var_name, sink.node.location()) {
        if let Some(&untaint_line) = env_untainted_lines.get(var) {
            if *sink_line > untaint_line {
                return false; // Env explicitly untainted before this sink
            }
        }
    }
    true
});
```

However, this requires access to the env's untaint events. A simpler approach: in `detect_taint_flows_with_env`, collect a set of `(var, untaint_line)` pairs. Pass this set to the merge point.

**Alternative simpler approach:** Instead of modifying the merge, modify the heuristic `detect_taint_flows` to check if there's an explicit reassignment between source and sink. In `taint.rs` around L1330, after finding a dependency path, verify the variable wasn't reassigned to a safe value:

In the dependency graph check section (around L1340-1400), add a reassignment check:

```rust
// Check if variable was reassigned to a safe value between source and sink lines
if let Some(ref source_var) = source.var_name {
    if let (Some((src_line,_,_,_)), Some((sink_line,_,_,_))) = 
        (source.node.location(), sink.node.location()) 
    {
        let lines_vec: Vec<&str> = source_text.lines().collect();
        for check_line in (src_line+1)..=*sink_line {
            if check_line > 0 && check_line <= lines_vec.len() {
                let lt = lines_vec[check_line - 1].trim();
                if let Some(eq_pos) = find_assignment_eq(lt) {
                    let lhs = lt[..eq_pos].trim();
                    let target = extract_target_var(lhs);
                    if target == *source_var {
                        let rhs = lt[eq_pos+1..].trim();
                        if is_safe_value(rhs) {
                            eprintln!("[DEBUG] Skipping: var '{}' reassigned to safe value at line {}", source_var, check_line);
                            // This sink is invalid — mark it
                            // (need to break out of the sink loop for this source-sink pair)
                        }
                    }
                }
            }
        }
    }
}
```

This is complex to integrate. **Recommended approach:** Add the `find_assignment_eq` and `is_safe_value` checks to the heuristic path's dependency graph traversal, similar to what the env path already does.

**Step 3: Build and test**

Same as Task A1.

**Step 4: Commit**

```bash
git commit -m "fix(taint): suppress heuristic flows when env untaints variable"
```

---

## Workstream C: Taint Env Field-Sensitive Propagation (~20 test wins)

**Root Cause:** `taint_field_sensitive1-8`, `taint_assign_record/record1`, `taint_object_destructure`, `taint_spread_record_*`, `taint_nested_record_pattern` — all require field-level taint tracking. The `TaintEnv` has `field_taints` and `is_field_tainted` methods, but the line-by-line walking in `detect_taint_flows_with_env` doesn't use them. Currently, taint is tracked at variable-level only: `x = tainted` → whole `x` is tainted, even if only `x.field1` should be tainted.

**Impact:** Fixes ~20 taint tests in rules category.

**Files:**
- Modify: `crates/astgrep-rules/src/executor/core/taint.rs:900-1100` (env walking)
- Reference: `crates/astgrep-rules/src/executor/core/taint_env.rs:181-272` (field APIs)

### Task C1: Add field-sensitive assignment detection in env walk

**Step 1: Understand field assignments**

Test cases have patterns like:
```python
x.field1 = tainted_source()    # Only x.field1 is tainted, not x.field2
sink(x.field1)                 # Should detect flow
sink(x.field2)                 # Should NOT detect flow
```

Current env walk (L900+) only does:
- Source match → `env.taint(var, line, idx)` (whole variable)
- Assignment → `env.propagate(target, source)` (whole variable)
- Safe reassignment → `env.untaint(var)` (whole variable)

Need to add:
- Field assignment: `x.field = tainted_val` → `env.taint_field("x", "field", line, idx)`
- Field sanitize: `x.field = safe_val` → `env.sanitize_field("x", "field")`
- Sink check: `sink(x.field)` → check `env.is_field_tainted("x", "field")`

**Step 2: Implement field-aware assignment parsing**

In `detect_taint_flows_with_env`, after the existing source match block (L934-952), add field-level source detection:

```rust
// 1b. Field-level source: "x.field = source()" pattern
if var_name.is_none() {
    if let Some(eq_pos) = find_assignment_eq(trimmed) {
        let lhs = trimmed[..eq_pos].trim();
        let (base_var, field_path) = extract_field_path(lhs);
        if let Some(field) = field_path {
            // Check if RHS references a tainted variable
            let rhs = trimmed[eq_pos+1..].trim();
            for tainted_var in env.tainted_vars() {
                if contains_var_reference(rhs, &tainted_var) {
                    env.taint_field(&base_var, &field, line_num, 0);
                    eprintln!("[DEBUG-TAINT-ENV] Line {}: tainted field '{}.{}' from var '{}'",
                        line_num, base_var, field, tainted_var);
                }
            }
        }
    }
}
```

In the assignment processing section (around L980+), add field-aware propagation:

```rust
// Field assignment: target.field = expr
if let Some(eq_pos) = find_assignment_eq(trimmed) {
    let lhs = trimmed[..eq_pos].trim();
    let (base_var, field_path) = extract_field_path(lhs);
    if let Some(field) = &field_path {
        let rhs = trimmed[eq_pos+1..].trim();
        if is_safe_value(rhs) {
            env.sanitize_field(&base_var, field);
        } else {
            for tainted_var in env.tainted_vars() {
                if contains_var_reference(rhs, &tainted_var) {
                    env.taint_field(&base_var, field, line_num, 
                        env.get_source_idx(&tainted_var).unwrap_or(0));
                }
            }
        }
    }
}
```

In the sink matching section (around L1050+), add field-sensitive sink detection:

```rust
// For each sink at this line, check both whole-var and field-level taint
for sink in sink_entries {
    if let Some(sink_text) = sink.node.text() {
        let sink_text = sink_text.trim();
        // Check if sink uses a specific field: sink(x.field)
        for tainted_var in env.tainted_vars() {
            // Whole-var taint (existing)
            if contains_var_reference(sink_text, &tainted_var) {
                // ... existing flow creation
            }
        }
        // Field-level taint
        for (var, fields) in &env.field_taints_snapshot() {
            for field in fields {
                let field_ref = format!("{}.{}", var, field);
                if contains_var_reference(sink_text, &field_ref) {
                    // Create flow for field-level taint
                }
            }
        }
    }
}
```

**Step 3: Build, test, verify no regressions**

Same verification pattern as A1.

**Step 4: Commit**

```bash
git commit -m "feat(taint): field-sensitive propagation in env-based analysis"
```

---

## Workstream D: Symbolic Propagation Variable Substitution (~10 test wins)

**Root Cause:** `find_matches_via_symbolic_propagation` (symbolic.rs:11-86) only handles patterns containing `...` (ellipsis method chains). It returns empty for simple patterns like `pandas.DataFrame(...).index.set_value(...)`. The function at L39 has:
```rust
if !pattern_str.contains("...") { return Ok(matches); }
```
This is wrong — symbolic propagation should also work for non-ellipsis patterns where a variable holds a known value.

**Impact:** Fixes `sym_prop_chain`, `sym_prop_exp`, `sym_prop_string_eq`, `sym_prop_deep`, `sym_prop_decorator`, `sym_prop_lambda`, `sym_prop_merge`, `sym_prop_no_merge2`, `sym_prop_non_constant_exp`, `sym_prop_open_redirect`, `sym_prop_python_with/1`, `sym_prop_record`, `top_level_sym_prop` (~14 tests).

**Files:**
- Modify: `crates/astgrep-rules/src/executor/core/symbolic.rs:11-86`
- Reference: `crates/astgrep-dataflow/src/symbolic_propagation.rs` (SymbolicPropagator, SymbolicState, SymbolicValue)

### Task D1: Extend symprop to handle non-ellipsis patterns via variable substitution

**Step 1: Understand the test case**

For `sym_prop_chain.py`:
```python
import pandas as pd
df = pd.DataFrame()  # df → SymbolicValue::MethodCall { base: Variable("pd"), method: "DataFrame" }
df.index.set_value(1, 2, 3)  # Should match pattern: pandas.DataFrame(...).index.set_value(...)
```

Pattern: `pandas.DataFrame(...).index.set_value(...)`

The symbolic propagator knows `df` is `pd.DataFrame()`, and `pd` resolves to `pandas` via import. So `df` → `pandas.DataFrame()` which matches `pandas.DataFrame(...)`.

**Step 2: Implement variable substitution matching**

Replace the early return at L39 with a more nuanced check:

```rust
// Remove the early return for non-ellipsis patterns
// if !pattern_str.contains("...") { return Ok(matches); }

// Instead, try variable substitution for all patterns
// 1. Try the existing ellipsis pattern matching (for patterns with ...)
if pattern_str.contains("...") {
    // ... existing ellipsis logic ...
}

// 2. Try variable substitution for ALL patterns (ellipsis or not)
self.find_matches_via_variable_substitution(
    pattern,
    ast,
    type_constraints,
    propagator,
    &mut matches,
)?;
```

Implement `find_matches_via_variable_substitution`:

```rust
fn find_matches_via_variable_substitution(
    &self,
    pattern: &SemgrepPattern,
    ast: &dyn AstNode,
    type_constraints: &[(String, String)],
    propagator: &SymbolicPropagator,
    matches: &mut Vec<SemgrepMatchResult>,
) -> Result<()> {
    let pattern_str = match &pattern.pattern_type {
        PatternType::Simple(s) => s.as_str(),
        _ => return Ok(()),
    };
    
    let state = propagator.state();
    
    // For each variable in symbolic state, check if substituting it into
    // the pattern creates a match against the AST
    for (var_name, sym_val) in state.variables() {
        let resolved = self.resolve_symbolic_to_source_text(sym_val, ast);
        if let Some(resolved_text) = resolved {
            // Try matching the pattern with this variable's resolved value
            // substituted back into the source
            // ... use existing pattern matching infrastructure
        }
    }
    Ok(())
}
```

**However**, this is complex and requires deep understanding of how pattern matching interacts with symbolic values. A simpler approach:

**Simpler approach:** Instead of full variable substitution, extend `parse_ellipsis_pattern` to also handle patterns like `pandas.DataFrame(...).index.set_value(...)` (which IS an ellipsis pattern but with nested dots). The current parser at L90-118 expects format `x(). ... .z()`, but `pandas.DataFrame(...)` has a different structure.

Actually, looking more carefully: `pandas.DataFrame(...).index.set_value(...)` DOES contain `...`. The issue is that `parse_ellipsis_pattern` returns None because it expects `something(). ... .something()` format (with `()` after start method), but `pandas.DataFrame(...)` has `(...)` which is different from `()`.

Let me trace: pattern = `pandas.DataFrame(...).index.set_value(...)`
- `pattern.replace(" ", "")` → `pandas.DataFrame(...).index.set_value(...)`
- `pattern.find("()")` → finds `()` inside `DataFrame(...)` → start_paren = 15
- `start_method = "pandas.DataFrame"` → this is actually correct
- `pattern.find("...")` → finds first `...` inside `DataFrame(...)` → ellipsis_idx = 16
- `after_ellipsis = &pattern[19..]` → `).index.set_value(...)`
- `after_ellipsis.starts_with('.')` → NO, starts with `)` → returns None

**The bug:** The parser doesn't handle `(...)` — it expects bare `()` with no content. After the `...` inside `DataFrame(...)`, the remaining text is `).index.set_value(...)` which starts with `)` not `.`.

**Step 2 (revised): Fix the ellipsis pattern parser**

Modify `parse_ellipsis_pattern` at L90-118 to handle parenthesized content:

```rust
pub(super) fn parse_ellipsis_pattern(&self, pattern_str: &str) -> Option<(String, String)> {
    let pattern = pattern_str.replace(" ", "");
    
    // Find the first "()" — but also handle "(...)" (ellipsis inside parens)
    let start_paren = pattern.find("()").or_else(|| {
        // Look for "(...)" as the first parenthesized group
        pattern.find("(...)").map(|pos| pos + 3) // Point past the "..."
    })?;
    
    // ... rest of parsing
```

Actually, a better fix: handle the pattern `pandas.DataFrame(...).index.set_value(...)` by:
1. Split on `).` to get segments: `["pandas.DataFrame(...", "index.set_value(...)"]`
2. Extract start: `pandas.DataFrame`
3. Extract end: `set_value`
4. Find the ellipsis chain: `pandas.DataFrame → .index → .set_value`

This requires rewriting `parse_ellipsis_pattern` to handle multi-segment method chains.

**Step 3: Build, test, verify**

Same verification pattern.

**Step 4: Commit**

```bash
git commit -m "fix(symprop): handle multi-segment ellipsis patterns with nested parens"
```

---

## Workstream E: Tree Matcher Fixes (~15 test wins)

### Task E1: Fix dots_inherit for class without parentheses (+1 test)

**Root Cause:** Pattern `class A(...):\n  ...` should match `class A:\n    ...` (no parenthesized base classes). In tree-sitter Python, `class A:` has no `argument_list` child, while `class A(B):` has one. The `is_optional_collection` function at L566 lists `inheritance_list` but not `argument_list` for class definitions.

**Test:** `dots_inherit` — expects match on line 2 (`class A:`) but only matches lines 7 and 12.

**Files:**
- Modify: `crates/astgrep-matcher/src/tree_matcher.rs:566-586`

**Fix:** The issue is that `class A:` in Python tree-sitter produces a `class_definition` node with children `["class", "A", ":"]`, while `class A(B):` produces `["class", "A", "(", "B", ")", ":"]`. The pattern `class A(...)` produces a pattern tree with `class_definition` containing an `argument_list` with `...`. When matching against `class A:` (no argument_list), the optional collection check should kick in and allow the match.

Check if `is_optional_collection` already handles this. Looking at L566-586, it checks for `argument_list` | `arguments` | `parameters` etc. For Python, the base class list is in an `argument_list`. The issue might be that the pattern tree's class_definition has 4 pattern children: `class`, `A`, `argument_list(...)`, `block(...)`, but the target's class_definition has 3 children: `class`, `A`, `block`.

The matching at L1593-1625 (child matching with optional collections) should handle this, but there might be a bug in how it counts required vs optional pattern children vs target children.

**Investigation needed:** Run `target/release/astgrep --pattern 'class A(...):\n  ...' tests/categories/patterns/python/dots_inherit.py` and examine debug output.

**Step 1: Add debug logging temporarily**

Add to child matching logic:
```rust
eprintln!("[DEBUG-DOTS] pattern children: {:?}, target children: {:?}",
    pattern_children.iter().map(|c| format!("{:?}", c)).collect::<Vec<_>>(),
    target_children.iter().map(|c| c.text().unwrap_or("")).collect::<Vec<_>>()
);
```

**Step 2: Fix based on investigation**

Likely fix: Ensure `is_optional_collection` is called for the Python `argument_list` child and that the child matching logic allows the target to have fewer children when optional collections are present.

### Task E2: Fix better_import3 wildcard import resolution (+1-2 tests)

**Root Cause:** Pattern `A.B.foo(...)` should match `foo()` when `import A.*;` is present. The import resolution at L1347-1392 handles `identifier` targets, but `foo()` is a `method_invocation` whose receiver is just `foo` (no qualified name). The pattern's qualified name `A.B.foo` needs to match against the bare `foo` using wildcard imports.

**Test:** `better_import3` — expects matches on lines 7, 10, 13. Currently only matches 10, 13.

**Files:**
- Modify: `crates/astgrep-matcher/src/tree_matcher.rs:1347-1392`

**Fix:** The current import resolution only runs when BOTH pattern and target have qualified name kinds (`scoped_identifier`, `member_expression`, etc.). For `foo()` call, the target is `method_invocation` → not a qualified name → import resolution is skipped.

Need to also trigger import resolution when the pattern is a qualified name AND the target is a method invocation whose name matches the last part of the qualified name.

```rust
// Case 3: Target is method invocation — check if method name resolves
if target_kind == "method_invocation" || target_kind == "call_expression" {
    // Extract the method name from the target
    for i in 0..target.child_count() {
        if let Some(child) = target.child(i) {
            let ck = child.get_attribute("ts_kind").unwrap_or(child.node_type());
            if ck == "identifier" || ck == "field_access" || ck == "member_expression" {
                if let Some(method_text) = child.text() {
                    let method_name = method_text.trim();
                    // Check if this method name matches the last part of pattern_qn
                    if let Some(last_dot) = pattern_qn.rfind('.') {
                        let last_part = &pattern_qn[last_dot+1..];
                        if method_name == last_part {
                            // Check wildcard imports for the prefix
                            let prefix = &pattern_qn[..last_dot];
                            for wc in &self.wildcard_imports {
                                if prefix.starts_with(wc) || wc.starts_with(prefix) {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
```

### Task E3: Fix dots_annotated_parameter for TypeScript decorators (+5 tests)

**Root Cause:** Pattern `function $FN(..., @Bar(...) $X, ...) {...}` should match methods with decorated parameters. The `@Bar(...)` decorator inside a parameter list is a `decorator` node that `filter_node_children` might be filtering out or the pattern parser isn't creating the right tree.

**Test:** `dots_annotated_parameter` — expects matches on lines 3, 8, 13, 18, 23. Currently 0 matches.

**Files:**
- Modify: `crates/astgrep-matcher/src/tree_matcher.rs:246-276` (filter_node_children)

**Investigation needed:** Check if `decorator` nodes in parameter lists are being filtered. The `filter_node_children` at L246 doesn't explicitly filter decorators, but decorators might have text like `@Bar(...)` that gets filtered by the punctuation check.

**Step 1:** Add `decorator` to the optional filter bypass (or check if it's already handled).

### Task E4: Fix cp_incrdecr constant propagation for ++/-- (+1 test)

**Root Cause:** Pattern `$X == $X` with constant propagation enabled should NOT match `newModelsCnt == 0` after `newModelsCnt++` because `newModelsCnt` is no longer 0. The constant propagator needs to handle `++` and `--` operators that modify constants.

**Test:** `cp_incrdecr` — expects match ONLY on line 6. Currently matches lines 3, 11, 20, 29, 38 (all `== 0` checks regardless of increment).

**Files:**
- Modify: `crates/astgrep-dataflow/src/symbolic_propagation.rs` or `crates/astgrep-rules/src/executor/core/mod.rs`

**Investigation needed:** Find where constant propagation removes `++`/`--` from expressions. The `ConstantPropagator` likely strips these operators but doesn't invalidate the constant value.

### Task E5: Fix focus-metavariable in e-rules (+3 tests)

**Root Cause:** `focus_metavariable_test` expects 6 matches but gets 0. The `focus-metavariable` directive in YAML rules is supposed to narrow the finding to only the matched metavariable's location. The rule parsing might not be recognizing `focus-metavariable` correctly.

**Test:** `focus_metavariable_test` — expects 6 positive matches, gets 0.

**Files:**
- Modify: `crates/astgrep-rules/src/parser/parsing.rs` (focus-metavariable parsing)
- Reference: `crates/astgrep-rules/src/executor/core/taint.rs:445` (focus_metavar extraction)

**Step 1:** Check if `focus-metavariable` is parsed from YAML. Look in parsing.rs for "focus" handling.

---

## Workstream F: Pattern Matching Import Resolution (~8 test wins)

### Task F1: Fix Python import resolution for patterns (+5 tests)

**Root Cause:** Patterns like `imports`, `import_negatives`, `import_negatives2`, `equivalence_naming_import`, `misc_block_import` all involve matching against imported names. The tree_matcher collects Python imports via `extract_python_import` and `extract_python_from_import` but may miss some import forms.

**Files:**
- Modify: `crates/astgrep-matcher/src/tree_matcher.rs:878-920` (Python import extraction)

**Investigation needed:** Run the failing tests and check which import forms are not being resolved.

### Task F2: Fix JS import/require aliasing (+6 tests)

**Root Cause:** `aliasing_require`, `equivalence_aliasing_import`, `equivalence_import_require`, `equivalence_import_variations/2/4` — JS has multiple import forms (`require`, `import {x as y}`, `import x from`, destructuring) that the import resolver doesn't handle.

**Files:**
- Modify: `crates/astgrep-matcher/src/tree_matcher.rs` (add JS import extraction)

---

## Execution Order

| Order | Workstream | Expected Tests | Risk | Effort |
|-------|-----------|---------------|------|--------|
| 1 | A: Taint Lambda | +30 | Low | Medium |
| 2 | C: Taint Field-Sensitive | +20 | Medium | High |
| 3 | D: Symprop Variable Sub | +14 | Medium | High |
| 4 | E1-E3: Tree Matcher Fixes | +8 | Low | Medium |
| 5 | B: Taint Param Source | +1 | Low | Low |
| 6 | F: Import Resolution | +13 | Low | Medium |
| 7 | E4-E5: CP + Focus | +4 | Medium | Medium |

**Total expected: +90 tests → 728/894 (81.4%)**

To reach 849 (95%), we need an additional +121 tests. These would come from:
- Completing all E tasks (+7 more from E4, E5)
- Taint propagator fixes (+10 from taint_propagator2, taint_propagator_lambda1/2)
- Taint labels support (+7 from taint_labels1-7)
- Additional pattern fixes (~100 remaining scattered failures)

---

## Constraints

- **No new dependencies** — use only existing crates
- **No test file modifications** — fix engine, not baselines
- **No debug eprintln in production** — remove before committing
- **Zero regressions** — verify score never decreases
- **Build must be clean** — `cargo build --release` exit code 0
- **Each commit is atomic** — one fix per commit, buildable at each step

## Anti-Regressions Checklist

After each task:
1. `cargo build --release` — must succeed
2. `python3 newtest/scripts/guardian_runner.py` — run full suite
3. Compare against `/tmp/wt_report_638.json` baseline — no previously passing tests should fail
4. `git diff --stat` — review changed files are expected
