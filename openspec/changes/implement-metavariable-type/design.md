## Context

Astgrep parses Semgrep-style YAML rules to perform static analysis. Currently, it supports various metavariable constraints (regex, comparison, name, analysis) but lacks support for `metavariable-type`. When rules containing this field are parsed, the parser fails because `metavariable-type` entries in the patterns array don't have a `pattern` field, triggering the error "must have a pattern field".

The `metavariable-type` constraint restricts a metavariable to only match expressions that are of a specific type. For example, `$WRITER.println(...)` where `$WRITER` must be of type `PrintWriter`.

Current codebase structure:
- `astgrep-rules/src/types.rs`: Contains `Condition` enum and related types
- `astgrep-rules/src/parser.rs`: Parses YAML rules into internal representation
- `astgrep-matcher/src/conditions.rs`: Applies conditions during pattern matching

## Goals / Non-Goals

**Goals:**
- Parse `metavariable-type` constraints from YAML rules without errors
- Add `MetavariableType` condition to the rules type system
- Implement type validation logic that checks if a metavariable's matched node is of the declared type
- Integrate type checking into the pattern matching pipeline
- Pass the failing test case: `metavariable_type_not_java.yaml` should correctly identify line 8 as a match

**Non-Goals:**
- Full type inference system (we only check explicitly declared types in rules)
- Cross-file type resolution
- Generic type parameter matching
- Type hierarchy/subtype checking

## Decisions

### 1. Type Information Source
**Decision:** Extract type information from AST node text and context, not from external type database.

**Rationale:** 
- Simple implementation that works for common cases
- No external dependencies or build system integration needed
- Matches Semgrep's behavior for basic type checking

**Alternative considered:** Using tree-sitter type information - rejected because tree-sitter doesn't always provide type information in the AST.

### 2. Type Checking Strategy
**Decision:** Use string matching on variable declarations and constructor calls.

**Rationale:**
- For Java, types appear in variable declarations: `PrintWriter pWriter = ...`
- We can extract the type from the declaration and match against the constraint
- Simple and fast for the common use case

**Implementation approach:**
- When matching `$WRITER` in `$WRITER.println(...)`, check if the node's text matches the variable's declaration type
- For the example `pWriter.println(request.input)`, we check if `pWriter` was declared as `PrintWriter`

### 3. Pattern Matching Integration
**Decision:** Add type checking as a `Condition` that gets evaluated after pattern matching.

**Rationale:**
- Consistent with existing metavariable constraints (regex, comparison, etc.)
- Can reuse existing condition evaluation infrastructure
- Clean separation of concerns

### 4. Multiple Language Support
**Decision:** Implement for Java first, with extensible design for other languages.

**Rationale:**
- The test case is Java
- Java has explicit type declarations that are easy to parse
- Framework can be extended for TypeScript, Go, etc.

## Risks / Trade-offs

**[Risk] Simple string matching may miss complex type scenarios** → **Mitigation:** Document limitations. For complex cases, users can combine with other constraints.

**[Risk] No type hierarchy support (subtypes not recognized)** → **Mitigation:** Document that exact type matching is used. Future enhancement could add type hierarchy support.

**[Risk] Performance impact of type checking** → **Mitigation:** Type checking is only performed when metavariable-type is specified, so default performance is unchanged.

**[Risk] Different languages have different type declaration syntax** → **Mitigation:** Language-specific type extractors can be added incrementally. Start with Java, add others as needed.

## Migration Plan

1. **Phase 1:** Implement core types and parsing (no breaking changes)
2. **Phase 2:** Implement Java-specific type extraction
3. **Phase 3:** Add integration test with the failing rule file
4. **Phase 4:** Document the feature and limitations

No database migrations or deployment steps required - this is a pure code change.
