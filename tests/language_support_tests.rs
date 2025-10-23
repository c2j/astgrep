//! Tests for new language support (Ruby, Kotlin, Swift)
//!
//! Tests for verifying that Ruby, Kotlin, and Swift parsers are properly integrated.

use astgrep_core::Language;
use astgrep_parser::ParserFactory;
use std::path::Path;

#[test]
fn test_ruby_language_enum() {
    assert_eq!(Language::Ruby.as_str(), "ruby");
    assert!(Language::Ruby.extensions().contains(&".rb"));
    assert!(Language::Ruby.extensions().contains(&".rbw"));
}

#[test]
fn test_kotlin_language_enum() {
    assert_eq!(Language::Kotlin.as_str(), "kotlin");
    assert!(Language::Kotlin.extensions().contains(&".kt"));
    assert!(Language::Kotlin.extensions().contains(&".kts"));
}

#[test]
fn test_swift_language_enum() {
    assert_eq!(Language::Swift.as_str(), "swift");
    assert!(Language::Swift.extensions().contains(&".swift"));
}

#[test]
fn test_language_from_str_ruby() {
    assert_eq!(Language::from_str("ruby"), Some(Language::Ruby));
    assert_eq!(Language::from_str("rb"), Some(Language::Ruby));
    assert_eq!(Language::from_str("RUBY"), Some(Language::Ruby));
}

#[test]
fn test_language_from_str_kotlin() {
    assert_eq!(Language::from_str("kotlin"), Some(Language::Kotlin));
    assert_eq!(Language::from_str("kt"), Some(Language::Kotlin));
    assert_eq!(Language::from_str("KOTLIN"), Some(Language::Kotlin));
}

#[test]
fn test_language_from_str_swift() {
    assert_eq!(Language::from_str("swift"), Some(Language::Swift));
    assert_eq!(Language::from_str("SWIFT"), Some(Language::Swift));
}

#[test]
fn test_language_from_extension_ruby() {
    assert_eq!(Language::from_extension("rb"), Some(Language::Ruby));
    assert_eq!(Language::from_extension(".rb"), Some(Language::Ruby));
    assert_eq!(Language::from_extension("rbw"), Some(Language::Ruby));
    assert_eq!(Language::from_extension(".rbw"), Some(Language::Ruby));
}

#[test]
fn test_language_from_extension_kotlin() {
    assert_eq!(Language::from_extension("kt"), Some(Language::Kotlin));
    assert_eq!(Language::from_extension(".kt"), Some(Language::Kotlin));
    assert_eq!(Language::from_extension("kts"), Some(Language::Kotlin));
    assert_eq!(Language::from_extension(".kts"), Some(Language::Kotlin));
}

#[test]
fn test_language_from_extension_swift() {
    assert_eq!(Language::from_extension("swift"), Some(Language::Swift));
    assert_eq!(Language::from_extension(".swift"), Some(Language::Swift));
}

#[test]
fn test_ruby_parser_creation() {
    let parser = ParserFactory::create_parser(Language::Ruby);
    assert!(parser.is_ok());
    let parser = parser.unwrap();
    assert_eq!(parser.language(), Language::Ruby);
}

#[test]
fn test_kotlin_parser_creation() {
    let parser = ParserFactory::create_parser(Language::Kotlin);
    assert!(parser.is_ok());
    let parser = parser.unwrap();
    assert_eq!(parser.language(), Language::Kotlin);
}

#[test]
fn test_swift_parser_creation() {
    let parser = ParserFactory::create_parser(Language::Swift);
    assert!(parser.is_ok());
    let parser = parser.unwrap();
    assert_eq!(parser.language(), Language::Swift);
}

#[test]
fn test_ruby_parser_supports_file() {
    let parser = ParserFactory::create_parser(Language::Ruby).unwrap();

    assert!(parser.supports_file(Path::new("test.rb")));
    assert!(parser.supports_file(Path::new("test.rbw")));
    assert!(parser.supports_file(Path::new("test.rake")));
    assert!(parser.supports_file(Path::new("test.gemspec")));

    assert!(!parser.supports_file(Path::new("test.java")));
    assert!(!parser.supports_file(Path::new("test.py")));
}

#[test]
fn test_kotlin_parser_supports_file() {
    let parser = ParserFactory::create_parser(Language::Kotlin).unwrap();
    
    assert!(parser.supports_file(Path::new("test.kt")));
    assert!(parser.supports_file(Path::new("test.kts")));
    
    assert!(!parser.supports_file(Path::new("test.java")));
    assert!(!parser.supports_file(Path::new("test.swift")));
}

#[test]
fn test_swift_parser_supports_file() {
    let parser = ParserFactory::create_parser(Language::Swift).unwrap();
    
    assert!(parser.supports_file(Path::new("test.swift")));
    
    assert!(!parser.supports_file(Path::new("test.java")));
    assert!(!parser.supports_file(Path::new("test.kt")));
}

#[test]
fn test_ruby_parser_simple_code() {
    let parser = ParserFactory::create_parser(Language::Ruby).unwrap();
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
fn test_kotlin_parser_simple_code() {
    let parser = ParserFactory::create_parser(Language::Kotlin).unwrap();
    let source = r#"
fun main() {
    println("Hello, Kotlin!")
}
"#;
    let result = parser.parse(source, Path::new("test.kt"));
    assert!(result.is_ok());
}

#[test]
fn test_swift_parser_simple_code() {
    let parser = ParserFactory::create_parser(Language::Swift).unwrap();
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
fn test_ruby_parser_class_definition() {
    let parser = ParserFactory::create_parser(Language::Ruby).unwrap();
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
fn test_kotlin_parser_class_definition() {
    let parser = ParserFactory::create_parser(Language::Kotlin).unwrap();
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
fn test_swift_parser_class_definition() {
    let parser = ParserFactory::create_parser(Language::Swift).unwrap();
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
fn test_ruby_parser_blocks() {
    let parser = ParserFactory::create_parser(Language::Ruby).unwrap();
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
fn test_kotlin_parser_lambdas() {
    let parser = ParserFactory::create_parser(Language::Kotlin).unwrap();
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
fn test_swift_parser_closures() {
    let parser = ParserFactory::create_parser(Language::Swift).unwrap();
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
fn test_ruby_parser_error_handling() {
    let parser = ParserFactory::create_parser(Language::Ruby).unwrap();
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

#[test]
fn test_kotlin_parser_null_safety() {
    let parser = ParserFactory::create_parser(Language::Kotlin).unwrap();
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
fn test_swift_parser_optionals() {
    let parser = ParserFactory::create_parser(Language::Swift).unwrap();
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

