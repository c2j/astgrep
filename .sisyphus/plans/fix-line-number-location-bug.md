# Fix: Line Number / Location Bug in Text Output

## Problem

When running `astgrep analyze --dialect gaussdb --rules ... --format text`, all findings report incorrect line numbers — always `:1:1` or `:1:3` regardless of where the actual match occurs in the source file.

### Observed Behavior

```
1. 语句块内通过 SELECT INTO var FOR UPDATE ...  case5.sql:1:3
   1 | -- ok: plsql-read-modify-write
   1 |   ^

2. ...  case6.sql:1:1
   1 | -- @rule plsql-read-modify-write
   1 | ^

3. ...  case1.sql:1:1
   1 | -- @rule plsql-read-modify-write
   1 | ^
```

Expected: findings should report the line of the actual matching SQL code (e.g., line 5-11 depending on file), not line 1.

## Root Cause Chain

The issue spans three components:

### 1. ogsql Adapter — Missing Locations on PL Nodes

**File**: `crates/astgrep-parser/src/adapter/ogsql/pl.rs`

`convert_pl_block` creates a `UniversalNode` for the PL block **without** setting location. Its children (declarations, assignment statements, etc.) are also created via `convert_pl_declaration` and `convert_pl_statement` without `with_location`. Only `PlStatement::SqlStatement` (which recurses to DML converters) gets location via `apply_span`.

The ogsql-parser DOES provide source spans for most constructs:
- `Statement::Do(Spanned<DoStatement>)` — `s.span` IS available
- `Statement::AnonyBlock(Spanned<AnonyBlockStatement>)` — span IS available
- `PlStatement::Block(Spanned<PlBlock>)`, `If(Spanned<...>)`, `While(Spanned<...>)`, etc. — spans ARE available
- BUT: `PlStatement::Assignment { target, expression }` — **NO span**
- BUT: `PlStatement::Return { expression }` — **NO span**

The outer span from `Spanned<DoStatement>` is applied to the top-level node via `apply_span`, but child nodes created inside `convert_pl_block` don't inherit it.

### 2. TreeMatcher — `first_meaningful_child` Returns Unlocated Child

**File**: `crates/astgrep-matcher/src/tree_matcher.rs`, lines 2089-2113

```rust
fn first_meaningful_child(node: &dyn AstNode) -> &dyn AstNode {
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            // skip comments, empty text, DECLARE/BEGIN/END
            if !skip.contains(&kind) {
                // BUG: only checks for (1,1,1,1), NOT for None
                if child.location() == Some((1, 1, 1, 1)) 
                    && node.location() != Some((1, 1, 1, 1)) {
                    return node;
                }
                return child; // child has location() = None → propagated as is
            }
        }
    }
    node
}
```

When the child has `location() = None` (the common case for PL nodes), it's returned as-is, carrying no location information forward.

### 3. AdvancedRuleExecutor — Hard Fallback to (1,1,1,1)

**File**: `crates/astgrep-rules/src/executor/core/mod.rs`, lines 809-834

```rust
fn create_finding_from_match(...) -> Result<Finding> {
    let default_location = || Location {
        start_line: 1, start_column: 1,  // ← the 1:1 comes from here
        end_line: 1, end_column: 1,
    };
    let node_location = match_result.node.location()  // None for PL nodes
        .map(|(sl,sc,el,ec)| Location { ... })
        .unwrap_or_else(default_location);  // ← falls back to (1,1,1,1)
    ...
}
```

No attempt is made to walk ancestor nodes for a valid location when `node.location()` returns `None`.

**Also**: `crates/astgrep-cli/src/commands/analyze_enhanced/pattern_matcher/core.rs`, line 152 has the same pattern:
```rust
let (sl, sc, el, ec) = match match_result.node.location() {
    Some((sl, sc, el, ec)) => (sl, sc, el, ec),
    None => (1, 1, 1, 1),  // same issue
};
```

### Why case5.sql shows :1:3 instead of :1:1

The dedup/sorting heuristics in `execute_pattern_analysis` (lines 343-394 of executor/core/mod.rs) sort matches by span size and filter overlapping spans. For case5.sql, a different child node happens to get through the filter with a partial location `(1, 3, ...)`, likely because the CREATE PROCEDURE's SQLStatement sub-node has a span that was partially set by the DML converter.

## Fix Plan

### Fix 1: `first_meaningful_child` — Handle `None` location (CRITICAL — addresses the general case)

**File**: `crates/astgrep-matcher/src/tree_matcher.rs`, line 2089-2113

**Change**: When child has `location() = None`, fall back to parent (if parent has a valid non-default location).

```rust
fn first_meaningful_child(node: &dyn AstNode) -> &dyn AstNode {
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            // ... skip comments, empty text, DECLARE/BEGIN/END ...
            if !skip.contains(&kind) {
                // Keep parent's span when:
                // a) child has default location (1,1,1,1), OR
                // b) child has no location at all (None)
                let child_loc = child.location();
                let parent_loc = node.location();
                let child_is_default = child_loc.map_or(true, |(sl,sc,_,_)| (sl,sc) == (1,1));
                let parent_is_valid = parent_loc.map_or(false, |(sl,sc,_,_)| (sl,sc) != (1,1));
                if child_is_default && parent_is_valid {
                    return node;
                }
                return child;
            }
        }
    }
    node
}
```

**Rationale**: This is the most impactful single change. The parent (block node) DOES have a valid span from `apply_span` via `Spanned<DoStatement>`. Returning the parent instead of the un-located child gives the match result a valid location.

### Fix 2: `create_finding_from_match` — Walk ancestors for location (DEFENSE-IN-DEPTH)

**File**: `crates/astgrep-rules/src/executor/core/mod.rs`, lines 816-834

**Change**: Instead of immediately falling back to `(1,1,1,1)` when `node.location()` is `None`, try finding a valid location from any ancestor of the matched node. Only use `(1,1,1,1)` as the absolute last resort.

**Implementation**: Add a helper method `find_best_location(node, ast_root, source)` that walks parent chain. If the match node has no location but is a child of a node that has a location, use the parent's location (possibly narrowed to the matched text range via source scanning).

### Fix 3: ogsql Adapter — Propagate Spans to PL Child Nodes (PRECISION IMPROVEMENT)

**File**: `crates/astgrep-parser/src/adapter/ogsql/pl.rs`

**Change A**: `convert_pl_block` should accept and propagate the source span from the `Spanned` wrapper. The block node itself should have `with_location` applied.

**Change B**: `convert_pl_statement` should use `Spanned` wrappers where available (Block, If, While, For, etc.) to set locations on child nodes.

**Change C**: For `PlStatement::Assignment` and `PlStatement::Return` (which don't have `Spanned` in ogsql-parser), we may need to:
- Derive approximate location from the evaluated expression nodes (Expr has spans)
- Or use the block-level span as a fallback for these statement types

**Note**: Fix 3 requires changes to `convert_pl_block`'s signature to accept an optional `Option<SourceSpan>`, and may require checking what span information the ogsql-parser actually provides on Expr types.

### Fix 4: `core.rs` pattern_matcher — Same fix as Fix 2

**File**: `crates/astgrep-cli/src/commands/analyze_enhanced/pattern_matcher/core.rs`, line 152

```rust
// Current:
None => (1, 1, 1, 1),

// Fix: use source-aware fallback
None => {
    // If the root AST node has a location, use it as a fallback
    // rather than hard-coded (1,1,1,1)
    // This is a secondary code path; the primary fix is in tree_matcher
    // and executor/core
}
```

## Priority & Dependency

| Fix | Priority | Depends On | Scope |
|-----|----------|------------|-------|
| Fix 1 | **CRITICAL** | None | General — affects all languages |
| Fix 2 | HIGH | None | General — defense-in-depth |
| Fix 3 | MEDIUM | None | GaussDB/OpenGauss only |
| Fix 4 | LOW | Fix 2 | Secondary code path |

Fix 1 alone should resolve the `:1:1` issue for most cases. Fix 2 + Fix 3 provide completeness and improved precision for GaussDB dialect.

## Verification

After fixes, re-run:
```bash
./target/debug/astgrep analyze --dialect gaussdb \
  --rules tests/categories/sql_dialects/gaussdb/rules/select_lock-1.yaml \
  --format text tests/categories/sql_dialects/gaussdb/cases/select_lock-1/
```

Expected: Findings should show line numbers matching the actual SQL statements (lines 5-11 range depending on file), not line 1 comments.
