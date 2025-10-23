//! XML language parser for astgrep

use crate::adapters::{AdapterContext, AdapterMetadata, AstAdapter};
use crate::base_adapter::BaseAdapter;
use astgrep_ast::{NodeType, UniversalNode};
use astgrep_core::{AstNode, Language, LanguageParser, Result};
use std::path::Path;

/// XML-specific AST adapter using the base adapter
pub struct XmlAdapter {
    base: BaseAdapter,
}

impl XmlAdapter {
    pub fn new() -> Self {
        Self {
            base: BaseAdapter::new(Language::Xml),
        }
    }
}

impl Default for XmlAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl AstAdapter for XmlAdapter {
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
        // Use the base adapter's parsing with XML-specific enhancements
        let ast = self.base.parse_to_ast(source, context)?;
        // Add XML-specific enhancements if needed
        // For now, the base adapter provides sufficient functionality

        Ok(ast)
    }
}

// XML-specific functionality can be added here if needed
// The base adapter provides sufficient functionality for basic parsing

/// XML language parser
pub struct XmlParser {
    adapter: XmlAdapter,
}

impl XmlParser {
    /// Create a new XML parser
    pub fn new() -> Self {
        Self {
            adapter: XmlAdapter::new(),
        }
    }
}

impl Default for XmlParser {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageParser for XmlParser {
    fn parse(&self, source: &str, _file_path: &Path) -> Result<Box<dyn AstNode>> {
        let context = AdapterContext::new(
            _file_path.to_string_lossy().to_string(),
            source.to_string(),
            Language::Xml
        );
        let ast = self.adapter.parse_to_ast(source, &context)?;
        Ok(Box::new(ast))
    }

    fn language(&self) -> Language {
        Language::Xml
    }

    fn supports_file(&self, file_path: &Path) -> bool {
        Language::Xml
            .extensions()
            .iter()
            .any(|ext| file_path.to_string_lossy().ends_with(ext))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xml_parser_creation() {
        let parser = XmlParser::new();
        assert_eq!(parser.language(), Language::Xml);
    }

    #[test]
    fn test_xml_parser_supports_file() {
        let parser = XmlParser::new();
        
        let xml_path = Path::new("test.xml");
        assert!(parser.supports_file(xml_path));
        
        let xsd_path = Path::new("schema.xsd");
        assert!(parser.supports_file(xsd_path));
        
        let svg_path = Path::new("image.svg");
        assert!(parser.supports_file(svg_path));
        
        let pom_path = Path::new("pom.xml");
        assert!(parser.supports_file(pom_path));
        
        let java_path = Path::new("test.java");
        assert!(!parser.supports_file(java_path));
    }

    #[test]
    fn test_xml_parser_simple_document() {
        let parser = XmlParser::new();
        let source = r#"<?xml version="1.0" encoding="UTF-8"?>
<root>
    <element>value</element>
</root>"#;
        let result = parser.parse(source, Path::new("test.xml"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_xml_parser_with_attributes() {
        let parser = XmlParser::new();
        let source = r#"<?xml version="1.0"?>
<book id="123" category="fiction">
    <title lang="en">Example Book</title>
    <author>John Doe</author>
    <year>2024</year>
</book>"#;
        let result = parser.parse(source, Path::new("test.xml"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_xml_parser_nested_elements() {
        let parser = XmlParser::new();
        let source = r#"<?xml version="1.0"?>
<catalog>
    <book>
        <title>XML Developer's Guide</title>
        <author>
            <name>John Smith</name>
            <email>john@example.com</email>
        </author>
    </book>
</catalog>"#;
        let result = parser.parse(source, Path::new("test.xml"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_xml_parser_cdata() {
        let parser = XmlParser::new();
        let source = r#"<?xml version="1.0"?>
<script>
    <![CDATA[
        function test() {
            return x < y && y > z;
        }
    ]]>
</script>"#;
        let result = parser.parse(source, Path::new("test.xml"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_xml_parser_namespaces() {
        let parser = XmlParser::new();
        let source = r#"<?xml version="1.0"?>
<root xmlns:h="http://www.w3.org/TR/html4/"
      xmlns:f="https://www.example.com/furniture">
    <h:table>
        <h:tr>
            <h:td>Cell</h:td>
        </h:tr>
    </h:table>
    <f:table>
        <f:name>Coffee Table</f:name>
    </f:table>
</root>"#;
        let result = parser.parse(source, Path::new("test.xml"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_xml_parser_svg() {
        let parser = XmlParser::new();
        let source = r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
    <circle cx="50" cy="50" r="40" stroke="black" stroke-width="2" fill="red"/>
    <rect x="10" y="10" width="30" height="30" fill="blue"/>
</svg>"#;
        let result = parser.parse(source, Path::new("image.svg"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_xml_parser_maven_pom() {
        let parser = XmlParser::new();
        let source = r#"<?xml version="1.0" encoding="UTF-8"?>
<project xmlns="http://maven.apache.org/POM/4.0.0">
    <modelVersion>4.0.0</modelVersion>
    <groupId>com.example</groupId>
    <artifactId>my-app</artifactId>
    <version>1.0-SNAPSHOT</version>
    <dependencies>
        <dependency>
            <groupId>junit</groupId>
            <artifactId>junit</artifactId>
            <version>4.12</version>
            <scope>test</scope>
        </dependency>
    </dependencies>
</project>"#;
        let result = parser.parse(source, Path::new("pom.xml"));
        assert!(result.is_ok());
    }
}

