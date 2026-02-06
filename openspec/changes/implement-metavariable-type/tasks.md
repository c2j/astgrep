## 1. Add MetavariableType Types

- [ ] 1.1 Add `MetavariableType` struct to `astgrep-rules/src/types.rs` with `metavariable` and `type` fields
- [ ] 1.2 Add `MetavariableType` variant to the `Condition` enum
- [ ] 1.3 Ensure types derive necessary traits (Debug, Clone, Serialize, Deserialize)

## 2. Update Rule Parser

- [ ] 2.1 Add recognition of `metavariable-type` key in `parse_patterns_array` function
- [ ] 2.2 Create `parse_metavariable_type` function to extract metavariable and type fields
- [ ] 2.3 Attach parsed `MetavariableType` condition to the preceding pattern
- [ ] 2.4 Add error handling for missing `metavariable` or `type` fields

## 3. Implement Type Extraction

- [ ] 3.1 Create type extraction utility for Java variable declarations
- [ ] 3.2 Implement function to parse `TypeName variableName = ...` pattern
- [ ] 3.3 Build type context map from file AST (variable name -> type)
- [ ] 3.4 Add language detection and extensible type extractor trait

## 4. Integrate Type Checking into Matcher

- [ ] 4.1 Update condition evaluation logic to handle `MetavariableType` condition
- [ ] 4.2 Implement type validation that checks if metavariable value matches declared type
- [ ] 4.3 Pass type context to pattern matching functions
- [ ] 4.4 Handle cases where type cannot be determined (permissive matching)

## 5. Testing and Validation

- [ ] 5.1 Run test: `cargo test` to ensure no regressions
- [ ] 5.2 Test with the failing rule: `cargo run -- analyze --config tests/categories/rules/metavariable_type_not_java.yaml tests/categories/rules/metavariable_type_not_java.java`
- [ ] 5.3 Verify rule now matches line 8 (`pWriter.println(request.input)`) but not line 12 (`sWriter.println(request.input)`)
- [ ] 5.4 Add unit tests for type extraction logic
- [ ] 5.5 Add integration test for metavariable-type constraint

## 6. Documentation

- [ ] 6.1 Document the new `metavariable-type` constraint in rule format documentation
- [ ] 6.2 Add example usage to the examples directory
- [ ] 6.3 Document limitations (Java only initially, no type hierarchy support)
