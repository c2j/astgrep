## Why

Astgrep needs to support `metavariable-type` constraint from Semgrep rules. This feature restricts metavariables to only match expressions of a specific type (e.g., `$WRITER` only matches `PrintWriter` type). Without this support, valid Semgrep rules fail to parse with "must have a pattern field" errors, preventing users from running existing Semgrep rule sets.

## What Changes

- Add support for parsing `metavariable-type` constraints from YAML rules
- Add new `MetavariableType` condition type to the rules engine
- Implement type matching logic that validates metavariable values against declared types
- Integrate type checking into the pattern matching pipeline

## Capabilities

### New Capabilities
- `metavariable-type`: Parse and apply type constraints to metavariables in Semgrep-style rules, allowing rules to restrict matches based on variable/ expression types

### Modified Capabilities
- `yaml-rule-parsing`: Extend rule parser to recognize `metavariable-type` as a valid pattern constraint alongside existing metavariable conditions

## Impact

- `astgrep-rules` crate: New condition type and parser logic
- Rule YAML parsing: New field recognition
- Pattern matching engine: Type validation during matching
- No breaking changes to existing functionality
