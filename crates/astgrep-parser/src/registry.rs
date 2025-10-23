//! Parser registry for managing language parsers
//!
//! This module provides the registry system for managing different language parsers.

use astgrep_core::{Language, LanguageParser, Result, constants::defaults::parser};
use std::collections::HashMap;

/// Parser registry configuration
#[derive(Debug, Clone)]
pub struct ParserConfig {
    pub timeout_ms: Option<u64>,
    pub max_file_size: Option<usize>,
    pub enable_recovery: bool,
    pub strict_mode: bool,
}

impl Default for ParserConfig {
    fn default() -> Self {
        Self {
            timeout_ms: Some(parser::DEFAULT_TIMEOUT_MS),
            max_file_size: Some(parser::DEFAULT_MAX_FILE_SIZE),
            enable_recovery: true,
            strict_mode: false,
        }
    }
}

/// Parser registry with configuration support
pub struct ConfigurableParserRegistry {
    parsers: HashMap<Language, Box<dyn LanguageParser>>,
    configs: HashMap<Language, ParserConfig>,
    global_config: ParserConfig,
}

impl ConfigurableParserRegistry {
    /// Create a new configurable parser registry
    pub fn new() -> Self {
        Self {
            parsers: HashMap::new(),
            configs: HashMap::new(),
            global_config: ParserConfig::default(),
        }
    }

    /// Set global parser configuration
    pub fn set_global_config(&mut self, config: ParserConfig) {
        self.global_config = config;
    }

    /// Set configuration for a specific language
    pub fn set_language_config(&mut self, language: Language, config: ParserConfig) {
        self.configs.insert(language, config);
    }

    /// Get configuration for a language (language-specific or global)
    pub fn get_config(&self, language: Language) -> &ParserConfig {
        self.configs.get(&language).unwrap_or(&self.global_config)
    }

    /// Register a parser with configuration
    pub fn register_parser_with_config(
        &mut self,
        language: Language,
        parser: Box<dyn LanguageParser>,
        config: Option<ParserConfig>,
    ) {
        self.parsers.insert(language, parser);
        if let Some(cfg) = config {
            self.configs.insert(language, cfg);
        }
    }

    /// Get parser statistics
    pub fn get_stats(&self) -> ParserStats {
        ParserStats {
            total_parsers: self.parsers.len(),
            configured_parsers: self.configs.len(),
            supported_languages: self.parsers.keys().cloned().collect(),
        }
    }

    /// Validate parser configuration
    pub fn validate_config(&self, language: Language) -> Result<()> {
        let config = self.get_config(language);
        
        if let Some(timeout) = config.timeout_ms {
            if timeout == 0 {
                return Err(astgrep_core::AnalysisError::parse_error(
                    "Parser timeout cannot be zero".to_string()
                ));
            }
        }

        if let Some(max_size) = config.max_file_size {
            if max_size == 0 {
                return Err(astgrep_core::AnalysisError::parse_error(
                    "Max file size cannot be zero".to_string()
                ));
            }
        }

        Ok(())
    }

    /// Check if source meets size requirements
    pub fn check_source_size(&self, language: Language, source: &str) -> Result<()> {
        let config = self.get_config(language);
        
        if let Some(max_size) = config.max_file_size {
            if source.len() > max_size {
                return Err(astgrep_core::AnalysisError::parse_error(format!(
                    "Source file too large: {} bytes (max: {} bytes)",
                    source.len(),
                    max_size
                )));
            }
        }

        Ok(())
    }
}

impl Default for ConfigurableParserRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Parser statistics
#[derive(Debug, Clone)]
pub struct ParserStats {
    pub total_parsers: usize,
    pub configured_parsers: usize,
    pub supported_languages: Vec<Language>,
}

/// Parser factory for creating language-specific parsers
pub struct ParserFactory;

impl ParserFactory {
    /// Create a parser for the specified language
    pub fn create_parser(language: Language) -> Result<Box<dyn LanguageParser>> {
        match language {
            Language::Java => Ok(Box::new(crate::java::JavaParser::new())),
            Language::JavaScript => Ok(Box::new(crate::javascript::JavaScriptParser::new())),
            Language::Python => Ok(Box::new(crate::python::PythonParser::new())),
            Language::Sql => Ok(Box::new(crate::sql::SqlParser::new())),
            Language::Bash => Ok(Box::new(crate::bash::BashParser::new())),
            Language::Php => Ok(Box::new(crate::php::PhpParser::new())),
            Language::CSharp => Ok(Box::new(crate::csharp::CSharpParser::new())),
            Language::C => Ok(Box::new(crate::c::CParser::new())),
            Language::Ruby => Ok(Box::new(crate::ruby::RubyParser::new())),
            Language::Kotlin => Ok(Box::new(crate::kotlin::KotlinParser::new())),
            Language::Swift => Ok(Box::new(crate::swift::SwiftParser::new())),
            Language::Xml => Ok(Box::new(crate::xml::XmlParser::new())),
        }
    }

    /// Create a parser with custom configuration
    pub fn create_parser_with_config(
        language: Language,
        _config: &ParserConfig,
    ) -> Result<Box<dyn LanguageParser>> {
        // For now, configuration doesn't affect parser creation
        // In a real implementation, this would customize the parser
        Self::create_parser(language)
    }

    /// Get default configuration for a language
    pub fn get_default_config(language: Language) -> ParserConfig {
        match language {
            Language::Java => ParserConfig {
                timeout_ms: Some(parser::JAVA_TIMEOUT_MS),
                max_file_size: Some(parser::JAVA_MAX_FILE_SIZE),
                enable_recovery: true,
                strict_mode: false,
            },
            Language::JavaScript => ParserConfig {
                timeout_ms: Some(parser::DEFAULT_TIMEOUT_MS),
                max_file_size: Some(parser::DEFAULT_MAX_FILE_SIZE),
                enable_recovery: true,
                strict_mode: false,
            },
            Language::Python => ParserConfig {
                timeout_ms: Some(parser::DEFAULT_TIMEOUT_MS),
                max_file_size: Some(parser::PYTHON_MAX_FILE_SIZE),
                enable_recovery: true,
                strict_mode: false,
            },
            Language::Sql => ParserConfig {
                timeout_ms: Some(parser::SQL_TIMEOUT_MS),
                max_file_size: Some(parser::SQL_MAX_FILE_SIZE),
                enable_recovery: false, // SQL should be strict
                strict_mode: true,
            },
            Language::Bash => ParserConfig {
                timeout_ms: Some(parser::BASH_TIMEOUT_MS),
                max_file_size: Some(parser::BASH_MAX_FILE_SIZE),
                enable_recovery: true,
                strict_mode: false,
            },
            Language::Php => ParserConfig {
                timeout_ms: Some(parser::DEFAULT_TIMEOUT_MS),
                max_file_size: Some(parser::DEFAULT_MAX_FILE_SIZE),
                enable_recovery: true,
                strict_mode: false,
            },
            Language::CSharp => ParserConfig {
                timeout_ms: Some(parser::DEFAULT_TIMEOUT_MS),
                max_file_size: Some(parser::DEFAULT_MAX_FILE_SIZE),
                enable_recovery: true,
                strict_mode: false,
            },
            Language::C => ParserConfig {
                timeout_ms: Some(parser::DEFAULT_TIMEOUT_MS),
                max_file_size: Some(parser::DEFAULT_MAX_FILE_SIZE),
                enable_recovery: true,
                strict_mode: false,
            },
            Language::Ruby => ParserConfig {
                timeout_ms: Some(parser::DEFAULT_TIMEOUT_MS),
                max_file_size: Some(parser::DEFAULT_MAX_FILE_SIZE),
                enable_recovery: true,
                strict_mode: false,
            },
            Language::Kotlin => ParserConfig {
                timeout_ms: Some(parser::DEFAULT_TIMEOUT_MS),
                max_file_size: Some(parser::DEFAULT_MAX_FILE_SIZE),
                enable_recovery: true,
                strict_mode: false,
            },
            Language::Swift => ParserConfig {
                timeout_ms: Some(parser::DEFAULT_TIMEOUT_MS),
                max_file_size: Some(parser::DEFAULT_MAX_FILE_SIZE),
                enable_recovery: true,
                strict_mode: false,
            },
            Language::Xml => ParserConfig {
                timeout_ms: Some(parser::DEFAULT_TIMEOUT_MS),
                max_file_size: Some(parser::DEFAULT_MAX_FILE_SIZE),
                enable_recovery: true,
                strict_mode: false,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_config_default() {
        let config = ParserConfig::default();
        assert_eq!(config.timeout_ms, Some(parser::DEFAULT_TIMEOUT_MS));
        assert_eq!(config.max_file_size, Some(parser::DEFAULT_MAX_FILE_SIZE));
        assert!(config.enable_recovery);
        assert!(!config.strict_mode);
    }

    #[test]
    fn test_configurable_registry() {
        let mut registry = ConfigurableParserRegistry::new();
        
        let config = ParserConfig {
            timeout_ms: Some(60000),
            max_file_size: Some(50 * 1024 * 1024),
            enable_recovery: false,
            strict_mode: true,
        };
        
        registry.set_language_config(Language::Java, config.clone());
        
        let retrieved_config = registry.get_config(Language::Java);
        assert_eq!(retrieved_config.timeout_ms, Some(60000));
        assert_eq!(retrieved_config.max_file_size, Some(50 * 1024 * 1024));
        assert!(!retrieved_config.enable_recovery);
        assert!(retrieved_config.strict_mode);
    }

    #[test]
    fn test_parser_factory() {
        let parser = ParserFactory::create_parser(Language::Java);
        assert!(parser.is_ok());
        
        let parser = parser.unwrap();
        assert_eq!(parser.language(), Language::Java);
    }

    #[test]
    fn test_parser_factory_default_configs() {
        let java_config = ParserFactory::get_default_config(Language::Java);
        assert_eq!(java_config.timeout_ms, Some(parser::JAVA_TIMEOUT_MS));
        assert_eq!(java_config.max_file_size, Some(parser::JAVA_MAX_FILE_SIZE));

        let sql_config = ParserFactory::get_default_config(Language::Sql);
        assert!(sql_config.strict_mode);
        assert!(!sql_config.enable_recovery);
    }

    #[test]
    fn test_validate_config() {
        let mut registry = ConfigurableParserRegistry::new();
        
        // Valid config
        let valid_config = ParserConfig::default();
        registry.set_language_config(Language::Java, valid_config);
        assert!(registry.validate_config(Language::Java).is_ok());
        
        // Invalid config - zero timeout
        let invalid_config = ParserConfig {
            timeout_ms: Some(0),
            ..Default::default()
        };
        registry.set_language_config(Language::Python, invalid_config);
        assert!(registry.validate_config(Language::Python).is_err());
    }

    #[test]
    fn test_check_source_size() {
        let mut registry = ConfigurableParserRegistry::new();
        
        let config = ParserConfig {
            max_file_size: Some(100), // 100 bytes
            ..Default::default()
        };
        registry.set_language_config(Language::Java, config);
        
        // Small source should pass
        let small_source = "public class Test {}";
        assert!(registry.check_source_size(Language::Java, small_source).is_ok());
        
        // Large source should fail
        let large_source = "a".repeat(200);
        assert!(registry.check_source_size(Language::Java, &large_source).is_err());
    }

    #[test]
    fn test_parser_stats() {
        let mut registry = ConfigurableParserRegistry::new();
        
        // Register some parsers
        let java_parser = ParserFactory::create_parser(Language::Java).unwrap();
        let js_parser = ParserFactory::create_parser(Language::JavaScript).unwrap();
        
        registry.register_parser_with_config(Language::Java, java_parser, None);
        registry.register_parser_with_config(Language::JavaScript, js_parser, Some(ParserConfig::default()));
        
        let stats = registry.get_stats();
        assert_eq!(stats.total_parsers, 2);
        assert_eq!(stats.configured_parsers, 1); // Only JS has explicit config
        assert!(stats.supported_languages.contains(&Language::Java));
        assert!(stats.supported_languages.contains(&Language::JavaScript));
    }
}
