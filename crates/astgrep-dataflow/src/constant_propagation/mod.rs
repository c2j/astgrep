//! Constant propagation analysis for data flow
//!
//! This module implements constant propagation to track constant values
//! through the program and enable more precise taint analysis.
//!
//! ## Module Structure
//!
//! This module is split into sub-modules:
//! - **state**: Data structures and state management
//!   - SourceLocation, ConstantValue, Scope, VariableDefinition
//!   - ConstantPropagator (struct definition)
//!   - VisitContext enum
//! - **analysis**: Core analysis algorithms
//!   - AST traversal and constant propagation
//! - **utils**: Helper functions
//!   - Context detection (is_static_block_context, etc.)
//!   - Name/value extraction utilities

pub mod state;
pub mod utils;
pub mod analysis;

// Re-export public items from state module for backward compatibility
pub use state::{ConstantPropagator, ConstantValue, SourceLocation, Scope, VariableDefinition, VisitContext};

// Tests remain in this file for simplicity
#[cfg(test)]
mod tests {
    use crate::constant_propagation::state::*;
    use super::*;

    #[test]
    fn test_constant_value_string() {
        let cv = ConstantValue::String("password".to_string());
        assert_eq!(cv.to_string_value(), Some("password".to_string()));
        assert!(cv.matches_pattern("pass"));
    }

    #[test]
    fn test_constant_value_integer() {
        let cv = ConstantValue::Integer(42);
        assert_eq!(cv.to_string_value(), Some("42".to_string()));
        assert!(cv.matches_pattern("42"));
    }

    #[test]
    fn test_constant_value_boolean() {
        let cv = ConstantValue::Boolean(true);
        assert_eq!(cv.to_string_value(), Some("true".to_string()));
        assert!(cv.matches_pattern("true"));
    }

    #[test]
    fn test_constant_value_null() {
        let cv = ConstantValue::Null;
        assert_eq!(cv.to_string_value(), Some("null".to_string()));
    }

    #[test]
    fn test_constant_propagator_new() {
        let propagator = ConstantPropagator::new();
        assert!(propagator.constants.is_empty());
        assert!(propagator.node_constants.is_empty());
        assert!(propagator.reassigned.is_empty());
    }

    #[test]
    fn test_constant_propagator_mark_reassigned() {
        let mut propagator = ConstantPropagator::new();
        propagator.constants.insert("x".to_string(), ConstantValue::Integer(42));

        assert!(propagator.is_constant("x"));

        propagator.mark_reassigned("x".to_string());

        assert!(!propagator.is_constant("x"));
        assert!(!propagator.constants.contains_key("x"));
    }

    #[test]
    fn test_constant_propagator_get_constant() {
        let mut propagator = ConstantPropagator::new();
        propagator.constants.insert("password".to_string(), ConstantValue::String("secret".to_string()));

        assert_eq!(
            propagator.get_constant("password"),
            Some(&ConstantValue::String("secret".to_string()))
        );
        assert_eq!(propagator.get_constant("unknown"), None);
    }

    #[test]
    fn test_constant_value_equality() {
        let cv1 = ConstantValue::String("test".to_string());
        let cv2 = ConstantValue::String("test".to_string());
        let cv3 = ConstantValue::String("other".to_string());

        assert_eq!(cv1, cv2);
        assert_ne!(cv1, cv3);
    }
}
