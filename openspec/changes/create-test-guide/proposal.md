## Why

The tests directory currently contains 100+ files and directories mixed together without clear organization. Users cannot easily find test scripts or understand how to run them to validate test cases. Additionally, there's no comprehensive guide explaining the available test scripts, their purposes, and how to use them.

## What Changes

- Create a comprehensive TEST_GUIDE.md documenting all test scripts and how to run them
- Reorganize the tests directory into a hierarchical structure:
  - `scripts/` - All test runner scripts (.sh, .py)
  - `cases/` - Test case files organized by language (from previous change)
  - `utils/` - Test utility scripts and helpers
  - `reports/` - Test output and report files
  - `config/` - Test configuration files (.yaml)
- Move existing test scripts from root to appropriate subdirectories
- Update any relative paths in scripts after moving
- Document the purpose of each test category and script

## Capabilities

### New Capabilities
- `test-guide-creation`: Create comprehensive guide for running test scripts
- `tests-directory-reorganization`: Reorganize tests directory into hierarchical structure
- `test-scripts-inventory`: Catalog and document all test scripts and their purposes

### Modified Capabilities
- (none - this is documentation and reorganization)

## Impact

- Tests directory structure
- Test script locations (will be moved to subdirectories)
- Documentation for developers and users
- Any hardcoded paths in scripts (need updating)
