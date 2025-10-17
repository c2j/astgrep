//! Core types and traits for CR-SemService static code analysis tool
//! 
//! This crate provides the fundamental types, traits, and error handling
//! used throughout the CR-SemService ecosystem.

pub mod error;
pub mod error_handling;
pub mod types;
pub mod traits;
pub mod optimization;
pub mod patterns;
pub mod constants;

// Re-export commonly used types
pub use error::{AnalysisError, Result};
pub use error_handling::*;
pub use types::*;
pub use optimization::*;
pub use traits::*;
pub use patterns::*;
pub use constants::*;

#[cfg(test)]
mod tests {
    #[test]
    fn test_core_module_loads() {
        // Basic smoke test to ensure the module loads correctly
        assert!(true);
    }
}
