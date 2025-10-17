//! Kotlin language parser for CR-SemService

use crate::adapters::{AdapterContext, AdapterMetadata, AstAdapter};
use crate::base_adapter::BaseAdapter;
use cr_ast::{NodeType, UniversalNode};
use cr_core::{AstNode, Language, LanguageParser, Result};
use std::path::Path;

/// Kotlin-specific AST adapter using the base adapter
pub struct KotlinAdapter {
    base: BaseAdapter,
}

impl KotlinAdapter {
    pub fn new() -> Self {
        Self {
            base: BaseAdapter::new(Language::Kotlin),
        }
    }
}

impl Default for KotlinAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl AstAdapter for KotlinAdapter {
    fn language(&self) -> Language {
        self.base.language()
    }

    fn metadata(&self) -> AdapterMetadata {
        self.base.metadata()
    }

    fn adapt_node(&self, node: &dyn std::any::Any, context: &AdapterContext) -> Result<UniversalNode> {
        self.base.adapt_node(node, context)
    }

    fn parse_to_ast(&self, source: &str, context: &AdapterContext) -> Result<UniversalNode> {
        // Use the base adapter's parsing with Kotlin-specific enhancements
        let ast = self.base.parse_to_ast(source, context)?;
        // Add Kotlin-specific enhancements if needed
        // For now, the base adapter provides sufficient functionality

        Ok(ast)
    }
}

// Kotlin-specific functionality can be added here if needed
// The base adapter provides sufficient functionality for basic parsing

/// Kotlin language parser
pub struct KotlinParser {
    adapter: KotlinAdapter,
}

impl KotlinParser {
    /// Create a new Kotlin parser
    pub fn new() -> Self {
        Self {
            adapter: KotlinAdapter::new(),
        }
    }
}

impl Default for KotlinParser {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageParser for KotlinParser {
    fn parse(&self, source: &str, _file_path: &Path) -> Result<Box<dyn AstNode>> {
        let context = AdapterContext::new(
            _file_path.to_string_lossy().to_string(),
            source.to_string(),
            Language::Kotlin
        );
        let ast = self.adapter.parse_to_ast(source, &context)?;
        Ok(Box::new(ast))
    }

    fn language(&self) -> Language {
        Language::Kotlin
    }

    fn supports_file(&self, file_path: &Path) -> bool {
        Language::Kotlin
            .extensions()
            .iter()
            .any(|ext| file_path.to_string_lossy().ends_with(ext))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kotlin_parser_creation() {
        let parser = KotlinParser::new();
        assert_eq!(parser.language(), Language::Kotlin);
    }

    #[test]
    fn test_kotlin_parser_supports_file() {
        let parser = KotlinParser::new();
        
        let kt_path = Path::new("test.kt");
        assert!(parser.supports_file(kt_path));
        
        let kts_path = Path::new("test.kts");
        assert!(parser.supports_file(kts_path));
        
        let java_path = Path::new("test.java");
        assert!(!parser.supports_file(java_path));
    }

    #[test]
    fn test_kotlin_parser_simple_function() {
        let parser = KotlinParser::new();
        let source = r#"
fun main() {
    println("Hello, Kotlin!")
}
"#;
        let result = parser.parse(source, Path::new("test.kt"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_kotlin_parser_class_definition() {
        let parser = KotlinParser::new();
        let source = r#"
class Person(val name: String, val age: Int) {
    fun greet() {
        println("Hi, I'm $name")
    }
}

fun main() {
    val person = Person("Alice", 30)
    person.greet()
}
"#;
        let result = parser.parse(source, Path::new("test.kt"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_kotlin_parser_data_class() {
        let parser = KotlinParser::new();
        let source = r#"
data class User(val id: Int, val name: String, val email: String)

fun main() {
    val user = User(1, "Bob", "bob@example.com")
    println(user)
}
"#;
        let result = parser.parse(source, Path::new("test.kt"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_kotlin_parser_extension_functions() {
        let parser = KotlinParser::new();
        let source = r#"
fun String.isValidEmail(): Boolean {
    return this.contains("@")
}

fun main() {
    val email = "test@example.com"
    println(email.isValidEmail())
}
"#;
        let result = parser.parse(source, Path::new("test.kt"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_kotlin_parser_lambda_and_higher_order() {
        let parser = KotlinParser::new();
        let source = r#"
fun main() {
    val numbers = listOf(1, 2, 3, 4, 5)
    val doubled = numbers.map { it * 2 }
    println(doubled)
}
"#;
        let result = parser.parse(source, Path::new("test.kt"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_kotlin_parser_null_safety() {
        let parser = KotlinParser::new();
        let source = r#"
fun main() {
    val name: String? = "Kotlin"
    val length = name?.length ?: 0
    println(length)
}
"#;
        let result = parser.parse(source, Path::new("test.kt"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_kotlin_parser_when_expression() {
        let parser = KotlinParser::new();
        let source = r#"
fun main() {
    val x = 5
    when (x) {
        1 -> println("One")
        2 -> println("Two")
        else -> println("Other")
    }
}
"#;
        let result = parser.parse(source, Path::new("test.kt"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_kotlin_parser_coroutines() {
        let parser = KotlinParser::new();
        let source = r#"
suspend fun fetchData(): String {
    return "Data"
}

fun main() {
    println("Kotlin coroutines")
}
"#;
        let result = parser.parse(source, Path::new("test.kt"));
        assert!(result.is_ok());
    }
}

