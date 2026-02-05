## Context

The tests directory currently has a flat structure with 100+ items mixed together:
- Test runner scripts (.sh, .py) scattered at root level
- Test case directories (java/, js/, patterns/, etc.)
- Configuration files (.yaml)
- Utility scripts for debugging and analysis
- Report generation scripts
- Rust test files (.rs)

This makes it difficult for users to:
1. Find the right script to run tests
2. Understand what each script does
3. Know which scripts are entry points vs utilities

## Goals / Non-Goals

**Goals:**
- Create hierarchical directory structure for better organization
- Move all executable scripts to `scripts/` subdirectory
- Separate test cases, utilities, configs, and reports
- Create comprehensive TEST_GUIDE.md at tests root
- Update script paths after moving files

**Non-Goals:**
- Modifying the actual test logic or test cases
- Changing how tests are executed (just where scripts are located)
- Removing any existing functionality
- Reorganizing the `cases/` directory (already done in previous change)

## Decisions

### Directory Structure
**Decision:** Use the following hierarchy:
```
tests/
├── TEST_GUIDE.md          # Main guide document
├── scripts/               # Executable test runners
│   ├── validation/        # Validation suite scripts
│   ├── compatibility/     # Compatibility test scripts
│   ├── performance/       # Performance test scripts
│   └── utils/             # Test utilities and helpers
├── cases/                 # Test case files (already organized)
├── config/                # Test configuration files (.yaml)
├── reports/               # Generated reports and outputs
└── lib/                   # Rust test modules (.rs files)
```

**Rationale:**
- Separates executable scripts from data/config
- Groups related scripts by purpose
- Keeps root directory clean with just the guide
- Follows common testing directory conventions

### Script Categorization
**Decision:** Categorize scripts by function:
- **validation/**: validate.sh, run_validation_suite.sh
- **compatibility/**: run_compatibility_tests.sh, demo_java_comparison.sh
- **performance/**: Performance testing scripts
- **utils/**: analyze_tests.py, debug_*.py, generate_*.py

**Rationale:** Makes it easier to find scripts by purpose

### Path Updates
**Decision:** Update relative paths in scripts after moving

**Approach:**
1. Identify all path references in each script
2. Update to reflect new location (e.g., `../cases/` instead of `./cases/`)
3. Test each script after moving

## Risks / Trade-offs

**Risk:** Scripts with hardcoded paths may break after moving → **Mitigation:** Audit and update all path references

**Risk:** External tools or documentation may reference old paths → **Mitigation:** Create migration guide in TEST_GUIDE.md

**Risk:** Some scripts may be called by other scripts → **Mitigation:** Check inter-script dependencies before moving

## Migration Plan

1. **Inventory existing files** - Catalog all scripts and their purposes
2. **Create new directory structure** - Create all subdirectories
3. **Move scripts** - Move files to appropriate directories
4. **Update paths** - Fix relative paths in moved scripts
5. **Create TEST_GUIDE.md** - Document the new structure and how to run tests
6. **Test** - Verify all scripts still work

## Open Questions

- Should we keep backward compatibility symlinks at root?
- Are there any CI/CD references to specific script paths?
- Should some scripts remain at root for easier access?
