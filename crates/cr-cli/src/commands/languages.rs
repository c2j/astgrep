//! Languages command implementation

use anyhow::Result;
use cr_core::Language;

/// Run the languages command
pub async fn run() -> Result<()> {
    println!("Supported Languages:");
    println!("===================");
    println!();

    let languages = [
        Language::Java,
        Language::JavaScript,
        Language::Python,
        Language::Php,
        Language::Sql,
        Language::Bash,
    ];

    for language in &languages {
        println!("Language: {}", language.as_str());
        println!("  Extensions: {}", language.extensions().join(", "));
        println!("  Description: {}", get_language_description(*language));
        println!();
    }

    println!("Total: {} languages supported", languages.len());
    Ok(())
}

fn get_language_description(language: Language) -> &'static str {
    match language {
        Language::Java => "Java programming language - object-oriented, platform-independent",
        Language::JavaScript => "JavaScript/TypeScript - dynamic scripting language for web and server",
        Language::Python => "Python - high-level, interpreted programming language",
        Language::Sql => "SQL - structured query language for database operations",
        Language::Bash => "Bash/Shell - command-line scripting language for Unix-like systems",
        Language::Php => "PHP - server-side scripting language for web development",
        Language::CSharp => "C# - object-oriented programming language by Microsoft",
        Language::C => "C - low-level programming language",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_languages_command() {
        let result = run().await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_language_descriptions() {
        assert!(!get_language_description(Language::Java).is_empty());
        assert!(!get_language_description(Language::JavaScript).is_empty());
        assert!(!get_language_description(Language::Python).is_empty());
        assert!(!get_language_description(Language::Sql).is_empty());
        assert!(!get_language_description(Language::Bash).is_empty());
    }

    #[test]
    fn test_all_languages_have_descriptions() {
        let languages = [
            Language::Java,
            Language::JavaScript,
            Language::Python,
            Language::Sql,
            Language::Bash,
        ];

        for language in &languages {
            let description = get_language_description(*language);
            assert!(!description.is_empty(), "Language {:?} has no description", language);
            assert!(description.len() > 10, "Language {:?} description too short", language);
        }
    }
}
