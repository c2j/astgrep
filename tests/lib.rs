//! Integration test suite for astgrep
//!
//! This file wires up all integration tests in `tests/lib/` so that Cargo
//! discovers them automatically. Without this entry point, Cargo only scans
//! `tests/*.rs` and ignores subdirectories.

#[path = "lib/advanced_taint_tests.rs"]
mod advanced_taint_tests;
#[path = "lib/comprehensive_analysis_tests.rs"]
mod comprehensive_analysis_tests;
#[path = "lib/concurrency_tests.rs"]
mod concurrency_tests;
#[path = "lib/constant_propagation_tests.rs"]
mod constant_propagation_tests;
#[path = "lib/integration_tests.rs"]
mod integration_tests;
#[path = "lib/interprocedural_analysis_tests.rs"]
mod interprocedural_analysis_tests;
#[path = "lib/performance_tests.rs"]
mod performance_tests;
#[path = "lib/phase4_integration_tests.rs"]
mod phase4_integration_tests;
#[path = "lib/regression_tests.rs"]
mod regression_tests;
#[path = "lib/rule_marketplace_tests.rs"]
mod rule_marketplace_tests;
#[path = "lib/semgrep_compatibility_tests.rs"]
mod semgrep_compatibility_tests;
#[path = "lib/sql_parser_integration_tests.rs"]
mod sql_parser_integration_tests;
#[path = "lib/symbol_table_tests.rs"]
mod symbol_table_tests;
#[path = "lib/taint_realworld_tests.rs"]
mod taint_realworld_tests;
#[path = "lib/vscode_integration_tests.rs"]
mod vscode_integration_tests;
