//! Core types and traits for astgrep static code analysis tool
//!
//! This crate provides the fundamental types, traits, and error handling
//! used throughout the astgrep ecosystem.

pub mod config;
pub mod constants;
pub mod error;
pub mod error_handling;
pub mod execution;
pub mod models;
pub mod optimization;
pub mod patterns;
pub mod traits;
pub mod types;

// Re-export commonly used types
pub use constants::*;
pub use error::{AnalysisError, Result};
pub use error_handling::*;
pub use models::*;
pub use optimization::*;
pub use patterns::*;
pub use traits::*;
pub use types::*;

#[cfg(test)]
mod tests {
    #[test]
    fn test_core_module_loads() {
        // Basic smoke test to ensure the module loads correctly
        assert!(true);
    }
}
