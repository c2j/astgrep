//! Ruby language parser for astgrep

use crate::adapters::{AdapterContext, AdapterMetadata, AstAdapter};
use crate::base_adapter::BaseAdapter;
use astgrep_ast::{NodeType, UniversalNode};
use astgrep_core::{AstNode, Language, LanguageParser, Result};
use std::path::Path;

/// Ruby-specific AST adapter using the base adapter
pub struct RubyAdapter {
    base: BaseAdapter,
}

impl RubyAdapter {
    pub fn new() -> Self {
        Self {
            base: BaseAdapter::new(Language::Ruby),
        }
    }
}

impl Default for RubyAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl AstAdapter for RubyAdapter {
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
        // Use the base adapter's parsing with Ruby-specific enhancements
        let ast = self.base.parse_to_ast(source, context)?;
        // Add Ruby-specific enhancements if needed
        // For now, the base adapter provides sufficient functionality

        Ok(ast)
    }
}

// Ruby-specific functionality can be added here if needed
// The base adapter provides sufficient functionality for basic parsing

/// Ruby language parser
pub struct RubyParser {
    adapter: RubyAdapter,
}

impl RubyParser {
    /// Create a new Ruby parser
    pub fn new() -> Self {
        Self {
            adapter: RubyAdapter::new(),
        }
    }
}

impl Default for RubyParser {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageParser for RubyParser {
    fn parse(&self, source: &str, _file_path: &Path) -> Result<Box<dyn AstNode>> {
        let context = AdapterContext::new(
            _file_path.to_string_lossy().to_string(),
            source.to_string(),
            Language::Ruby
        );
        let ast = self.adapter.parse_to_ast(source, &context)?;
        Ok(Box::new(ast))
    }

    fn language(&self) -> Language {
        Language::Ruby
    }

    fn supports_file(&self, file_path: &Path) -> bool {
        Language::Ruby
            .extensions()
            .iter()
            .any(|ext| file_path.to_string_lossy().ends_with(ext))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ruby_parser_creation() {
        let parser = RubyParser::new();
        assert_eq!(parser.language(), Language::Ruby);
    }

    #[test]
    fn test_ruby_parser_supports_file() {
        let parser = RubyParser::new();
        
        let rb_path = Path::new("test.rb");
        assert!(parser.supports_file(rb_path));
        
        let rbw_path = Path::new("test.rbw");
        assert!(parser.supports_file(rbw_path));
        
        let rake_path = Path::new("Rakefile");
        assert!(parser.supports_file(rake_path));
        
        let java_path = Path::new("test.java");
        assert!(!parser.supports_file(java_path));
    }

    #[test]
    fn test_ruby_parser_simple_code() {
        let parser = RubyParser::new();
        let source = r#"
def hello(name)
  puts "Hello, #{name}!"
end

hello("World")
"#;
        let result = parser.parse(source, Path::new("test.rb"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_ruby_parser_class_definition() {
        let parser = RubyParser::new();
        let source = r#"
class Person
  def initialize(name)
    @name = name
  end
  
  def greet
    puts "Hi, I'm #{@name}"
  end
end

person = Person.new("Alice")
person.greet
"#;
        let result = parser.parse(source, Path::new("test.rb"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_ruby_parser_blocks_and_iterators() {
        let parser = RubyParser::new();
        let source = r#"
[1, 2, 3].each do |num|
  puts num * 2
end

result = [1, 2, 3].map { |x| x * 2 }
"#;
        let result = parser.parse(source, Path::new("test.rb"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_ruby_parser_symbols_and_hashes() {
        let parser = RubyParser::new();
        let source = r#"
person = {
  name: "Bob",
  age: 30,
  city: "New York"
}

puts person[:name]
"#;
        let result = parser.parse(source, Path::new("test.rb"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_ruby_parser_string_interpolation() {
        let parser = RubyParser::new();
        let source = r#"
name = "Ruby"
version = 3.0
puts "Welcome to #{name} #{version}"
"#;
        let result = parser.parse(source, Path::new("test.rb"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_ruby_parser_regex() {
        let parser = RubyParser::new();
        let source = r#"
pattern = /\d+/
text = "The answer is 42"
if text =~ pattern
  puts "Found a number"
end
"#;
        let result = parser.parse(source, Path::new("test.rb"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_ruby_parser_exception_handling() {
        let parser = RubyParser::new();
        let source = r#"
begin
  result = 10 / 0
rescue ZeroDivisionError => e
  puts "Error: #{e.message}"
ensure
  puts "Cleanup"
end
"#;
        let result = parser.parse(source, Path::new("test.rb"));
        assert!(result.is_ok());
    }
}

