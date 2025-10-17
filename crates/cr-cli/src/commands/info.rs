//! Info command for showing system information

use anyhow::Result;
use cr_core::Language;
use tracing::info;

/// Show information about supported languages and features
pub async fn run(
    language: Option<String>,
    show_extensions: bool,
    show_categories: bool,
) -> Result<()> {
    info!("Displaying system information");

    if let Some(lang) = language {
        show_language_info(&lang)?;
    } else if show_extensions {
        show_file_extensions();
    } else if show_categories {
        show_rule_categories();
    } else {
        show_general_info();
    }

    Ok(())
}

fn show_general_info() {
    println!("🔍 CR-SemService - Static Code Analysis Tool");
    println!("Version: {}", env!("CARGO_PKG_VERSION"));
    println!("Build: {} ({})",
        option_env!("VERGEN_BUILD_DATE").unwrap_or("unknown"),
        option_env!("VERGEN_GIT_SHA").unwrap_or("unknown")
    );
    println!();

    println!("📋 Supported Languages:");
    for lang in &[
        Language::Java,
        Language::JavaScript,
        Language::Python,
        Language::Sql,
        Language::Bash,
    ] {
        let (extensions, description) = get_language_details(lang);
        println!("  • {:?}: {} ({})", lang, description, extensions.join(", "));
    }
    println!();

    println!("🎯 Analysis Features:");
    println!("  • Pattern Matching: Advanced AST-based pattern detection");
    println!("  • Data Flow Analysis: Track data flow across function boundaries");
    println!("  • Security Scanning: Detect common security vulnerabilities");
    println!("  • Code Quality: Identify maintainability and performance issues");
    println!("  • Custom Rules: Support for user-defined analysis rules");
    println!("  • Multiple Formats: JSON, SARIF, HTML, Markdown output");
    println!();

    println!("⚡ Performance Features:");
    println!("  • Parallel Processing: Multi-threaded analysis");
    println!("  • Incremental Analysis: Only analyze changed files");
    println!("  • Caching: Cache analysis results for faster subsequent runs");
    println!("  • Memory Optimization: Efficient memory usage for large codebases");
    println!();

    println!("🔧 Integration Support:");
    println!("  • CI/CD: GitHub Actions, Jenkins, GitLab CI");
    println!("  • IDEs: VS Code, IntelliJ IDEA, Eclipse");
    println!("  • Issue Tracking: JIRA, GitHub Issues");
    println!("  • Notifications: Slack, Email, Webhooks");
    println!();

    println!("📚 Documentation:");
    println!("  • User Guide: https://github.com/your-org/cr-semservice/docs");
    println!("  • API Reference: https://docs.rs/cr-semservice");
    println!("  • Rule Writing: https://github.com/your-org/cr-semservice/wiki/rules");
    println!();

    println!("💡 Quick Start:");
    println!("  1. Initialize config: cr-semservice init");
    println!("  2. Analyze code: cr-semservice analyze src/");
    println!("  3. View rules: cr-semservice list --detailed");
    println!("  4. Get help: cr-semservice --help");
}

fn show_language_info(lang_str: &str) -> Result<()> {
    let language = match lang_str.to_lowercase().as_str() {
        "java" => Language::Java,
        "javascript" | "js" => Language::JavaScript,
        "python" | "py" => Language::Python,
        "sql" => Language::Sql,
        "bash" | "sh" => Language::Bash,
        _ => {
            return Err(anyhow::anyhow!("Unsupported language: {}", lang_str));
        }
    };

    let (extensions, description) = get_language_details(&language);

    println!("📋 Language Information: {:?}", language);
    println!();
    println!("Description: {}", description);
    println!("File Extensions: {}", extensions.join(", "));
    println!();

    match language {
        Language::Java => show_java_info(),
        Language::JavaScript => show_javascript_info(),
        Language::Python => show_python_info(),
        Language::Sql => show_sql_info(),
        Language::Bash => show_bash_info(),
        Language::Php => println!("PHP support is basic"),
        Language::CSharp => println!("C# support is basic"),
        Language::C => println!("C support is basic"),
    }

    Ok(())
}

fn get_language_details(language: &Language) -> (Vec<&'static str>, &'static str) {
    match language {
        Language::Java => (
            vec![".java"],
            "Object-oriented programming language"
        ),
        Language::JavaScript => (
            vec![".js", ".jsx", ".ts", ".tsx"],
            "Dynamic programming language for web development"
        ),
        Language::Python => (
            vec![".py"],
            "High-level programming language"
        ),
        Language::Sql => (
            vec![".sql"],
            "Structured Query Language for databases"
        ),
        Language::Bash => (
            vec![".sh", ".bash"],
            "Unix shell scripting language"
        ),
        Language::Php => (
            vec![".php"],
            "Server-side scripting language"
        ),
        Language::CSharp => (
            vec![".cs"],
            "Object-oriented programming language by Microsoft"
        ),
        Language::C => (
            vec![".c", ".h"],
            "Low-level programming language"
        ),
    }
}

fn show_java_info() {
    println!("🔍 Java Analysis Capabilities:");
    println!("  • Security Vulnerabilities:");
    println!("    - SQL Injection detection");
    println!("    - XSS vulnerability scanning");
    println!("    - Insecure deserialization");
    println!("    - Weak cryptography usage");
    println!("    - Path traversal vulnerabilities");
    println!();
    println!("  • Code Quality Issues:");
    println!("    - Null pointer dereference");
    println!("    - Resource leak detection");
    println!("    - Exception handling problems");
    println!("    - Thread safety issues");
    println!("    - Performance anti-patterns");
    println!();
    println!("  • Best Practices:");
    println!("    - Proper logging usage");
    println!("    - Design pattern violations");
    println!("    - Code complexity analysis");
    println!("    - Naming convention checks");
    println!();
    println!("  • Framework Support:");
    println!("    - Spring Framework");
    println!("    - Hibernate/JPA");
    println!("    - Apache Struts");
    println!("    - Android development");
}

fn show_javascript_info() {
    println!("🔍 JavaScript/TypeScript Analysis Capabilities:");
    println!("  • Security Vulnerabilities:");
    println!("    - XSS vulnerability detection");
    println!("    - Prototype pollution");
    println!("    - Insecure randomness");
    println!("    - Eval usage detection");
    println!("    - CSRF vulnerabilities");
    println!();
    println!("  • Code Quality Issues:");
    println!("    - Undefined variable usage");
    println!("    - Type coercion problems");
    println!("    - Async/await misuse");
    println!("    - Memory leak detection");
    println!("    - Performance bottlenecks");
    println!();
    println!("  • Best Practices:");
    println!("    - Modern ES6+ usage");
    println!("    - Proper error handling");
    println!("    - Code organization");
    println!("    - Testing patterns");
    println!();
    println!("  • Framework Support:");
    println!("    - React/JSX");
    println!("    - Vue.js");
    println!("    - Angular");
    println!("    - Node.js");
    println!("    - Express.js");
}

fn show_python_info() {
    println!("🔍 Python Analysis Capabilities:");
    println!("  • Security Vulnerabilities:");
    println!("    - SQL injection detection");
    println!("    - Command injection");
    println!("    - Pickle deserialization");
    println!("    - Path traversal");
    println!("    - Weak cryptography");
    println!();
    println!("  • Code Quality Issues:");
    println!("    - Import statement problems");
    println!("    - Exception handling");
    println!("    - Resource management");
    println!("    - Type hint violations");
    println!("    - Performance issues");
    println!();
    println!("  • Best Practices:");
    println!("    - PEP 8 compliance");
    println!("    - Pythonic code patterns");
    println!("    - Documentation standards");
    println!("    - Testing practices");
    println!();
    println!("  • Framework Support:");
    println!("    - Django");
    println!("    - Flask");
    println!("    - FastAPI");
    println!("    - SQLAlchemy");
    println!("    - Pandas/NumPy");
}

fn show_sql_info() {
    println!("🔍 SQL Analysis Capabilities:");
    println!("  • Security Vulnerabilities:");
    println!("    - SQL injection patterns");
    println!("    - Privilege escalation");
    println!("    - Data exposure risks");
    println!("    - Weak authentication");
    println!();
    println!("  • Performance Issues:");
    println!("    - Missing indexes");
    println!("    - Inefficient queries");
    println!("    - Cartesian products");
    println!("    - Subquery optimization");
    println!();
    println!("  • Best Practices:");
    println!("    - Query optimization");
    println!("    - Schema design");
    println!("    - Transaction management");
    println!("    - Data integrity");
    println!();
    println!("  • Database Support:");
    println!("    - MySQL");
    println!("    - PostgreSQL");
    println!("    - Oracle");
    println!("    - SQL Server");
    println!("    - SQLite");
}

fn show_bash_info() {
    println!("🔍 Bash/Shell Analysis Capabilities:");
    println!("  • Security Vulnerabilities:");
    println!("    - Command injection");
    println!("    - Path traversal");
    println!("    - Privilege escalation");
    println!("    - Insecure file operations");
    println!();
    println!("  • Code Quality Issues:");
    println!("    - Unquoted variables");
    println!("    - Error handling");
    println!("    - Exit code management");
    println!("    - Resource cleanup");
    println!();
    println!("  • Best Practices:");
    println!("    - ShellCheck compliance");
    println!("    - Portable scripting");
    println!("    - Documentation standards");
    println!("    - Testing practices");
    println!();
    println!("  • Shell Support:");
    println!("    - Bash");
    println!("    - Zsh");
    println!("    - Dash");
    println!("    - POSIX shell");
}

fn show_file_extensions() {
    println!("📁 Supported File Extensions:");
    println!();

    let languages = [
        (Language::Java, "Java"),
        (Language::JavaScript, "JavaScript/TypeScript"),
        (Language::Python, "Python"),
        (Language::Sql, "SQL"),
        (Language::Bash, "Bash/Shell"),
    ];

    for (lang, name) in &languages {
        let (extensions, _) = get_language_details(lang);
        println!("  {} ({})", name, extensions.join(", "));
    }

    println!();
    println!("💡 Note: Files are automatically detected based on their extension.");
    println!("   You can also specify languages explicitly using the --language flag.");
}

fn show_rule_categories() {
    println!("📂 Available Rule Categories:");
    println!();

    let categories = [
        ("security", "Security vulnerability detection", "🔒"),
        ("best-practice", "Code quality and best practices", "✨"),
        ("performance", "Performance optimization", "⚡"),
        ("maintainability", "Code maintainability", "🔧"),
        ("reliability", "Code reliability and correctness", "🛡️"),
        ("style", "Code style and formatting", "🎨"),
        ("complexity", "Code complexity analysis", "📊"),
        ("documentation", "Documentation quality", "📚"),
        ("testing", "Testing practices", "🧪"),
        ("experimental", "Experimental rules", "🔬"),
    ];

    for (category, description, emoji) in &categories {
        println!("  {} {}: {}", emoji, category, description);
    }

    println!();
    println!("💡 Use 'cr-semservice list --category <name>' to see rules in a specific category.");
    println!("   Configure enabled categories in your configuration file.");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_show_general_info() {
        // This test just ensures the function doesn't panic
        let result = run(None, false, false).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_show_language_info_valid() {
        let result = run(Some("java".to_string()), false, false).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_show_language_info_invalid() {
        let result = run(Some("invalid".to_string()), false, false).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_show_extensions() {
        let result = run(None, true, false).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_show_categories() {
        let result = run(None, false, true).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_language_details() {
        let (extensions, description) = get_language_details(&Language::Java);
        assert_eq!(extensions, vec![".java"]);
        assert!(description.contains("Object-oriented"));

        let (extensions, description) = get_language_details(&Language::JavaScript);
        assert!(extensions.contains(&".js"));
        assert!(extensions.contains(&".ts"));
        assert!(description.contains("Dynamic"));
    }
}
