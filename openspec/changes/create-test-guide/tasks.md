## 1. Inventory and Catalog

- [x] 1.1 List all .sh scripts in tests/ root and identify their purposes
- [x] 1.2 List all .py scripts in tests/ root and identify their purposes
- [x] 1.3 List all .rs test files in tests/ root
- [x] 1.4 List all .yaml config files in tests/ root
- [x] 1.5 Identify which files are test runners vs utilities vs configs
- [x] 1.6 Document inter-script dependencies (which scripts call others)

## 2. Create Directory Structure

- [x] 2.1 Create tests/scripts/ directory
- [x] 2.2 Create tests/scripts/validation/ subdirectory
- [x] 2.3 Create tests/scripts/compatibility/ subdirectory
- [x] 2.4 Create tests/scripts/performance/ subdirectory
- [x] 2.5 Create tests/scripts/utils/ subdirectory
- [x] 2.6 Create tests/config/ directory
- [x] 2.7 Create tests/reports/ directory
- [x] 2.8 Create tests/lib/ directory

## 3. Move Scripts to New Structure

- [x] 3.1 Move validation scripts to tests/scripts/validation/
- [x] 3.2 Move compatibility scripts to tests/scripts/compatibility/
- [x] 3.3 Move performance scripts to tests/scripts/performance/
- [x] 3.4 Move utility scripts to tests/scripts/utils/
- [x] 3.5 Move .yaml config files to tests/config/
- [x] 3.6 Move .rs test files to tests/lib/

## 4. Update Script Paths

- [x] 4.1 Update paths in validation scripts
- [x] 4.2 Update paths in compatibility scripts
- [x] 4.3 Update paths in performance scripts
- [x] 4.4 Update paths in utility scripts
- [x] 4.5 Verify all relative paths work correctly

## 5. Create TEST_GUIDE.md

- [x] 5.1 Create TEST_GUIDE.md at tests/ root
- [x] 5.2 Write Quick Start section
- [x] 5.3 Document directory structure
- [x] 5.4 Document validation scripts and how to run them
- [x] 5.5 Document compatibility scripts and how to run them
- [x] 5.6 Document performance scripts and how to run them
- [x] 5.7 Document utility scripts and their purposes
- [x] 5.8 Add examples of common test commands

## 6. Verify and Test

- [x] 6.1 Test validation scripts still work after moving
- [x] 6.2 Test compatibility scripts still work after moving
- [x] 6.3 Test utility scripts still work after moving
- [x] 6.4 Verify TEST_GUIDE.md is complete and accurate
- [x] 6.5 Check for any broken references in documentation

## 7. Final Review

- [x] 7.1 Review new directory structure
- [x] 7.2 Ensure no files were missed in migration
- [x] 7.3 Verify root tests/ directory is clean (only TEST_GUIDE.md and subdirs)
- [x] 7.4 Create summary of changes for developers
