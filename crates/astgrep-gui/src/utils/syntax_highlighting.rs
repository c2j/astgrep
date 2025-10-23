//! Syntax highlighting utilities

use egui;

/// Syntax highlighter for different languages
pub struct SyntaxHighlighter {
    /// Language-specific keywords
    keywords: std::collections::HashMap<String, Vec<String>>,
}

impl SyntaxHighlighter {
    pub fn new() -> Self {
        let mut keywords = std::collections::HashMap::new();
        
        // Java keywords
        keywords.insert("java".to_string(), vec![
            "abstract", "assert", "boolean", "break", "byte", "case", "catch", "char",
            "class", "const", "continue", "default", "do", "double", "else", "enum",
            "extends", "final", "finally", "float", "for", "goto", "if", "implements",
            "import", "instanceof", "int", "interface", "long", "native", "new", "package",
            "private", "protected", "public", "return", "short", "static", "strictfp",
            "super", "switch", "synchronized", "this", "throw", "throws", "transient",
            "try", "void", "volatile", "while"
        ].into_iter().map(|s| s.to_string()).collect());
        
        // JavaScript keywords
        keywords.insert("javascript".to_string(), vec![
            "async", "await", "break", "case", "catch", "class", "const", "continue",
            "debugger", "default", "delete", "do", "else", "export", "extends", "finally",
            "for", "function", "if", "import", "in", "instanceof", "let", "new", "return",
            "super", "switch", "this", "throw", "try", "typeof", "var", "void", "while",
            "with", "yield"
        ].into_iter().map(|s| s.to_string()).collect());
        
        // Python keywords
        keywords.insert("python".to_string(), vec![
            "and", "as", "assert", "break", "class", "continue", "def", "del", "elif",
            "else", "except", "exec", "finally", "for", "from", "global", "if", "import",
            "in", "is", "lambda", "not", "or", "pass", "print", "raise", "return", "try",
            "while", "with", "yield", "async", "await"
        ].into_iter().map(|s| s.to_string()).collect());
        
        Self { keywords }
    }
    
    /// Apply syntax highlighting to text
    pub fn highlight_text(&self, ui: &mut egui::Ui, text: &str, language: &str) {
        let keywords = self.keywords.get(language).cloned().unwrap_or_default();
        
        // Simple token-based highlighting
        let tokens = self.tokenize(text);
        
        ui.horizontal_wrapped(|ui| {
            for token in tokens {
                match token.token_type {
                    TokenType::Keyword => {
                        if keywords.contains(&token.text) {
                            ui.colored_label(egui::Color32::LIGHT_BLUE, &token.text);
                        } else {
                            ui.label(&token.text);
                        }
                    }
                    TokenType::String => {
                        ui.colored_label(egui::Color32::LIGHT_GREEN, &token.text);
                    }
                    TokenType::Comment => {
                        ui.colored_label(egui::Color32::GRAY, &token.text);
                    }
                    TokenType::Number => {
                        ui.colored_label(egui::Color32::YELLOW, &token.text);
                    }
                    TokenType::Operator => {
                        ui.colored_label(egui::Color32::WHITE, &token.text);
                    }
                    TokenType::Whitespace => {
                        ui.label(&token.text);
                    }
                    TokenType::Other => {
                        ui.label(&token.text);
                    }
                }
            }
        });
    }
    
    /// Simple tokenizer
    fn tokenize(&self, text: &str) -> Vec<Token> {
        let mut tokens = Vec::new();
        let mut current_token = String::new();
        let mut in_string = false;
        let mut in_comment = false;
        let mut string_char = '"';
        
        for ch in text.chars() {
            if in_comment {
                current_token.push(ch);
                if ch == '\n' {
                    tokens.push(Token {
                        text: current_token.clone(),
                        token_type: TokenType::Comment,
                    });
                    current_token.clear();
                    in_comment = false;
                }
                continue;
            }
            
            if in_string {
                current_token.push(ch);
                if ch == string_char {
                    tokens.push(Token {
                        text: current_token.clone(),
                        token_type: TokenType::String,
                    });
                    current_token.clear();
                    in_string = false;
                }
                continue;
            }
            
            match ch {
                '"' | '\'' => {
                    if !current_token.is_empty() {
                        tokens.push(Token {
                            text: current_token.clone(),
                            token_type: self.classify_token(&current_token),
                        });
                        current_token.clear();
                    }
                    current_token.push(ch);
                    string_char = ch;
                    in_string = true;
                }
                '/' if text.chars().nth(1) == Some('/') => {
                    if !current_token.is_empty() {
                        tokens.push(Token {
                            text: current_token.clone(),
                            token_type: self.classify_token(&current_token),
                        });
                        current_token.clear();
                    }
                    current_token.push(ch);
                    in_comment = true;
                }
                ' ' | '\t' | '\n' | '\r' => {
                    if !current_token.is_empty() {
                        tokens.push(Token {
                            text: current_token.clone(),
                            token_type: self.classify_token(&current_token),
                        });
                        current_token.clear();
                    }
                    tokens.push(Token {
                        text: ch.to_string(),
                        token_type: TokenType::Whitespace,
                    });
                }
                '(' | ')' | '{' | '}' | '[' | ']' | ';' | ',' | '.' => {
                    if !current_token.is_empty() {
                        tokens.push(Token {
                            text: current_token.clone(),
                            token_type: self.classify_token(&current_token),
                        });
                        current_token.clear();
                    }
                    tokens.push(Token {
                        text: ch.to_string(),
                        token_type: TokenType::Operator,
                    });
                }
                _ => {
                    current_token.push(ch);
                }
            }
        }
        
        if !current_token.is_empty() {
            tokens.push(Token {
                text: current_token,
                token_type: if in_comment { TokenType::Comment } else { TokenType::Other },
            });
        }
        
        tokens
    }
    
    fn classify_token(&self, token: &str) -> TokenType {
        if token.chars().all(|c| c.is_ascii_digit()) {
            TokenType::Number
        } else if token.chars().all(|c| c.is_alphabetic() || c == '_') {
            TokenType::Keyword
        } else {
            TokenType::Other
        }
    }
}

#[derive(Debug, Clone)]
struct Token {
    text: String,
    token_type: TokenType,
}

#[derive(Debug, Clone, PartialEq)]
enum TokenType {
    Keyword,
    String,
    Comment,
    Number,
    Operator,
    Whitespace,
    Other,
}
