# Handoff: Type Annotation Stripping Fix

## Problem
Python pattern `def $F(filename):\n  ...` failed to match `def download_file(filename: str):` because tree-sitter wraps typed parameters in a `typed_parameter` node (containing `identifier "filename" + : + type`), but the pattern only has a bare `identifier "filename"`. The text-based Phase 1 matching (`match_node` in `tree_matcher.rs`) failed because `"filename" ≠ "filename: str"`, and since `identifier` is a leaf node (no children), it returned `false` at line 1324 without reaching structural Phase 2 matching.

## Fix
**File:** `crates/astgrep-matcher/src/tree_matcher.rs` (+64/-1 lines)

Three changes:

1. **Phase 1 type annotation extraction** (lines 1341-1367): When pattern text fails to match target text, check if the target is a `typed_parameter`/`optional_parameter`/`default_parameter`. If so, iterate its children to find an inner `identifier`, then compare the pattern text against that child's text (stripped + whitespace-normalized).

2. **Kind-match equivalence** (lines 1390-1391): Added `identifier ↔ typed_parameter/optional_parameter/default_parameter` equivalence in Phase 2 as defense-in-depth (for non-leaf cases where Phase 1 doesn't apply).

3. **Chain matching guard** (lines 1573-1574 + new helper `has_ellipsis_in_skip_kind_child` at lines 144-163): Prevents chain matching from incorrectly consuming ellipsis nodes inside skip-kind children.

## Verification

| Check | Result |
|-------|--------|
| `cargo build --release` | ✅ Clean build |
| `cargo test` (206 tests) | ✅ 205/206 pass (1 pre-existing failure in `vscode_integration_tests`) |
| LSP diagnostics | ✅ No errors |
| Guardian runner score | **646→647/894** (72.3%→72.4%) ✅ No regressions |
| Manual binary test | ✅ Pattern matches BOTH untyped and typed parameter functions |

## Manual test command
```bash
cat > /tmp/test_rule.yml << 'EOF'
rules:
  - id: less_typehint
    pattern: |
      def $F(filename):
        ...
    message: Pattern match
    languages: [python]
    severity: INFO
EOF
target/release/astgrep analyze tests/categories/patterns/python/less_typehint.py \
  -r /tmp/test_rule.yml --format json
```
Expected: 2 findings (lines 6 and 11).

## Residual issue
The `less_typehint` guardian test still reports `MISSED: lines [10] | EXTRA: lines [11]`. The typed function IS now matched, but the baseline expects match at the decorator line (10) while astgrep reports it at the `def` line (11). This decorator-offset is a pre-existing line-numbering issue, NOT caused by this fix. The test was already failing before (`MISSED: lines [10]`).

## All modified files
- `crates/astgrep-matcher/src/tree_matcher.rs` — only file with uncommitted changes
