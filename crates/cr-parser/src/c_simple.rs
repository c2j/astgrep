//! Simplified C language parser using the basic adapter macro

use crate::adapters::AstAdapter;
use cr_core::Language;

// Use the macro to generate the basic adapter and parser
crate::impl_basic_adapter!(
    CSimpleAdapter,
    CSimpleParser,
    Language::C,
    "C Simple Adapter",
    "Simplified C language adapter for CR-SemService",
    &["c", "h"]
);

#[cfg(test)]
mod tests {
    use super::*;
    use cr_core::LanguageParser;
    use cr_ast::NodeType;
    use std::path::Path;

    #[test]
    fn test_c_simple_parser_creation() {
        let parser = CSimpleParser::new();
        assert_eq!(parser.language(), Language::C);
        assert!(parser.extensions().contains(&"c"));
    }

    #[test]
    fn test_c_simple_basic_parsing() {
        let parser = CSimpleParser::new();
        let source = r#"#include <stdio.h>
int main() {
    printf("Hello, World!");
    return 0;
}"#;
        
        let result = parser.parse(source, Path::new("test.c"));
        assert!(result.is_ok());
        
        let ast = result.unwrap();
        assert_eq!(ast.node_type(), "program");
    }

    #[test]
    fn test_adapter_metadata() {
        let adapter = CSimpleAdapter;
        let metadata = adapter.metadata();
        
        assert_eq!(metadata.name, "C Simple Adapter");
        assert_eq!(metadata.version, "1.0.0");
        assert!(metadata.supported_features.contains(&"basic_parsing".to_string()));
        assert!(metadata.supported_features.contains(&"taint_analysis".to_string()));
    }
}
