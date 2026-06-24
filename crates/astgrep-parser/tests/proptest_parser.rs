//! Property-based tests for astgrep-parser using proptest.
//!
//! These tests verify that the parser handles arbitrary input gracefully
//! (never panics, always returns Ok or Err).

use astgrep_core::Language;
use astgrep_parser::LanguageParserRegistry;
use proptest::prelude::*;
use std::path::Path;

/// Extension → Language mapping for proptest
fn extension_for_lang(lang: Language) -> &'static str {
    match lang {
        Language::Java => "java",
        Language::JavaScript => "js",
        Language::Python => "py",
        Language::Sql => "sql",
        Language::Bash => "sh",
        Language::Xml => "xml",
        Language::Text => "txt",
    }
}

proptest! {
    /// 1. Any string can be parsed without panicking for each supported language.
    ///    The parser must return Ok or Err but never panic.
    #[test]
    fn prop_parse_arbitrary_string_never_panics(
        lang in prop::sample::select(vec![
            Language::Java,
            Language::JavaScript,
            Language::Python,
            Language::Sql,
            Language::Bash,
            Language::Text,
        ]),
        input in ".*"
    ) {
        let registry = LanguageParserRegistry::new();
        let ext = extension_for_lang(lang);
        let path = format!("test.{}", ext);

        let _result = registry.parse_file(Path::new(&path), &input);
    }

    /// 2. Empty string parsing never panics for any supported language.
    #[test]
    fn prop_empty_string_parse_never_panics(
        lang in prop::sample::select(vec![
            Language::Java,
            Language::JavaScript,
            Language::Python,
            Language::Sql,
            Language::Bash,
        ])
    ) {
        let registry = LanguageParserRegistry::new();
        let ext = extension_for_lang(lang);
        let path = format!("test.{}", ext);

        let _result = registry.parse_file(Path::new(&path), "");
    }

    /// 3. Unicode strings parse without panic.
    #[test]
    fn prop_unicode_parse_never_panics(
        lang in prop::sample::select(vec![
            Language::Java,
            Language::JavaScript,
            Language::Python,
        ]),
        input in "\\p{Any}*"
    ) {
        let registry = LanguageParserRegistry::new();
        let ext = extension_for_lang(lang);
        let path = format!("test.{}", ext);

        let _result = registry.parse_file(Path::new(&path), &input);
    }

    /// 4. Long strings parse without panic.
    #[test]
    fn prop_long_string_parse(
        lang in prop::sample::select(vec![
            Language::Java,
            Language::JavaScript,
            Language::Python,
        ]),
        s in "[a-z ]{0,5000}"
    ) {
        let registry = LanguageParserRegistry::new();
        let ext = extension_for_lang(lang);
        let path = format!("test.{}", ext);

        let _result = registry.parse_file(Path::new(&path), &s);
    }

    /// 5. Random bytes (as lossy UTF-8) parse without panic.
    #[test]
    fn prop_random_bytes_parse(
        lang in prop::sample::select(vec![
            Language::Java,
            Language::JavaScript,
            Language::Python,
            Language::Sql,
            Language::Bash,
        ]),
        bytes in prop::collection::vec(any::<u8>(), 0..1000)
    ) {
        let registry = LanguageParserRegistry::new();
        let ext = extension_for_lang(lang);
        let path = format!("test.{}", ext);

        let s = String::from_utf8_lossy(&bytes);
        let _result = registry.parse_file(Path::new(&path), &s);
    }
}
