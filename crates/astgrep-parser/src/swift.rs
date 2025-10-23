//! Swift language parser for astgrep

use crate::adapters::{AdapterContext, AdapterMetadata, AstAdapter};
use crate::base_adapter::BaseAdapter;
use astgrep_ast::{NodeType, UniversalNode};
use astgrep_core::{AstNode, Language, LanguageParser, Result};
use std::path::Path;

/// Swift-specific AST adapter using the base adapter
pub struct SwiftAdapter {
    base: BaseAdapter,
}

impl SwiftAdapter {
    pub fn new() -> Self {
        Self {
            base: BaseAdapter::new(Language::Swift),
        }
    }
}

impl Default for SwiftAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl AstAdapter for SwiftAdapter {
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
        // Use the base adapter's parsing with Swift-specific enhancements
        let ast = self.base.parse_to_ast(source, context)?;
        // Add Swift-specific enhancements if needed
        // For now, the base adapter provides sufficient functionality

        Ok(ast)
    }
}

// Swift-specific functionality can be added here if needed
// The base adapter provides sufficient functionality for basic parsing

/// Swift language parser
pub struct SwiftParser {
    adapter: SwiftAdapter,
}

impl SwiftParser {
    /// Create a new Swift parser
    pub fn new() -> Self {
        Self {
            adapter: SwiftAdapter::new(),
        }
    }
}

impl Default for SwiftParser {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageParser for SwiftParser {
    fn parse(&self, source: &str, _file_path: &Path) -> Result<Box<dyn AstNode>> {
        let context = AdapterContext::new(
            _file_path.to_string_lossy().to_string(),
            source.to_string(),
            Language::Swift
        );
        let ast = self.adapter.parse_to_ast(source, &context)?;
        Ok(Box::new(ast))
    }

    fn language(&self) -> Language {
        Language::Swift
    }

    fn supports_file(&self, file_path: &Path) -> bool {
        Language::Swift
            .extensions()
            .iter()
            .any(|ext| file_path.to_string_lossy().ends_with(ext))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_swift_parser_creation() {
        let parser = SwiftParser::new();
        assert_eq!(parser.language(), Language::Swift);
    }

    #[test]
    fn test_swift_parser_supports_file() {
        let parser = SwiftParser::new();
        
        let swift_path = Path::new("test.swift");
        assert!(parser.supports_file(swift_path));
        
        let java_path = Path::new("test.java");
        assert!(!parser.supports_file(java_path));
    }

    #[test]
    fn test_swift_parser_simple_function() {
        let parser = SwiftParser::new();
        let source = r#"
import Foundation

func greet(name: String) -> String {
    return "Hello, \(name)!"
}

print(greet(name: "Swift"))
"#;
        let result = parser.parse(source, Path::new("test.swift"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_swift_parser_class_definition() {
        let parser = SwiftParser::new();
        let source = r#"
class Person {
    var name: String
    var age: Int
    
    init(name: String, age: Int) {
        self.name = name
        self.age = age
    }
    
    func greet() {
        print("Hi, I'm \(name)")
    }
}

let person = Person(name: "Alice", age: 30)
person.greet()
"#;
        let result = parser.parse(source, Path::new("test.swift"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_swift_parser_struct_definition() {
        let parser = SwiftParser::new();
        let source = r#"
struct Point {
    var x: Double
    var y: Double
    
    func distance() -> Double {
        return sqrt(x * x + y * y)
    }
}

let point = Point(x: 3.0, y: 4.0)
print(point.distance())
"#;
        let result = parser.parse(source, Path::new("test.swift"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_swift_parser_optional_handling() {
        let parser = SwiftParser::new();
        let source = r#"
func findIndex(of value: Int, in array: [Int]) -> Int? {
    for (index, element) in array.enumerated() {
        if element == value {
            return index
        }
    }
    return nil
}

if let index = findIndex(of: 3, in: [1, 2, 3, 4]) {
    print("Found at index: \(index)")
}
"#;
        let result = parser.parse(source, Path::new("test.swift"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_swift_parser_closures() {
        let parser = SwiftParser::new();
        let source = r#"
let numbers = [1, 2, 3, 4, 5]
let doubled = numbers.map { $0 * 2 }
let evens = numbers.filter { $0 % 2 == 0 }
print(doubled)
print(evens)
"#;
        let result = parser.parse(source, Path::new("test.swift"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_swift_parser_protocol_definition() {
        let parser = SwiftParser::new();
        let source = r#"
protocol Drawable {
    func draw()
}

class Circle: Drawable {
    func draw() {
        print("Drawing a circle")
    }
}

let circle = Circle()
circle.draw()
"#;
        let result = parser.parse(source, Path::new("test.swift"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_swift_parser_error_handling() {
        let parser = SwiftParser::new();
        let source = r#"
enum FileError: Error {
    case fileNotFound
    case permissionDenied
}

func readFile(path: String) throws -> String {
    throw FileError.fileNotFound
}

do {
    let content = try readFile(path: "test.txt")
} catch FileError.fileNotFound {
    print("File not found")
} catch {
    print("Unknown error")
}
"#;
        let result = parser.parse(source, Path::new("test.swift"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_swift_parser_generics() {
        let parser = SwiftParser::new();
        let source = r#"
func swap<T>(_ a: inout T, _ b: inout T) {
    let temp = a
    a = b
    b = temp
}

var x = 5
var y = 10
swap(&x, &y)
print("x: \(x), y: \(y)")
"#;
        let result = parser.parse(source, Path::new("test.swift"));
        assert!(result.is_ok());
    }
}

