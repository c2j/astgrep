//! Pattern parser
//! 
//! This module provides functionality to parse pattern strings into structured representations.

use cr_core::{AnalysisError, Result};
use std::fmt;

/// Parsed pattern representation
#[derive(Debug, Clone, PartialEq)]
pub enum ParsedPattern {
    /// Literal text to match
    Literal(String),
    /// Metavariable (e.g., $VAR)
    Metavariable(String),
    /// Ellipsis metavariable (e.g., $...ARGS)
    EllipsisMetavariable(String),
    /// Node type constraint
    NodeType(String),
    /// Sequence of patterns
    Sequence(Vec<ParsedPattern>),
    /// Alternative patterns (OR)
    Alternative(Vec<ParsedPattern>),
    /// Wildcard (matches anything)
    Wildcard,
}

impl fmt::Display for ParsedPattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParsedPattern::Literal(s) => write!(f, "\"{}\"", s),
            ParsedPattern::Metavariable(s) => write!(f, "${}", s),
            ParsedPattern::EllipsisMetavariable(s) => write!(f, "$...{}", s),
            ParsedPattern::NodeType(s) => write!(f, "@{}", s),
            ParsedPattern::Sequence(patterns) => {
                write!(f, "(")?;
                for (i, pattern) in patterns.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", pattern)?;
                }
                write!(f, ")")
            }
            ParsedPattern::Alternative(patterns) => {
                write!(f, "(")?;
                for (i, pattern) in patterns.iter().enumerate() {
                    if i > 0 {
                        write!(f, " | ")?;
                    }
                    write!(f, "{}", pattern)?;
                }
                write!(f, ")")
            }
            ParsedPattern::Wildcard => write!(f, "..."),
        }
    }
}

/// Pattern parser
pub struct PatternParser {
    strict_mode: bool,
}

impl PatternParser {
    /// Create a new pattern parser
    pub fn new() -> Self {
        Self {
            strict_mode: false,
        }
    }

    /// Create a parser in strict mode
    pub fn strict() -> Self {
        Self {
            strict_mode: true,
        }
    }

    /// Parse a pattern string
    pub fn parse(&self, pattern: &str) -> Result<ParsedPattern> {
        let tokens = self.tokenize(pattern)?;
        self.parse_tokens(&tokens)
    }

    /// Tokenize the pattern string
    fn tokenize(&self, pattern: &str) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();
        let mut chars = pattern.chars().peekable();
        let mut current_pos = 0;

        while let Some(ch) = chars.next() {
            current_pos += 1;
            
            match ch {
                // Skip whitespace
                ' ' | '\t' | '\n' | '\r' => continue,
                
                // Metavariable
                '$' => {
                    let mut name = String::new();

                    // Check for ellipsis metavariable ($...VAR)
                    if chars.peek() == Some(&'.') {
                        chars.next(); // consume first dot
                        current_pos += 1;
                        if chars.peek() == Some(&'.') {
                            chars.next(); // consume second dot
                            current_pos += 1;
                            if chars.peek() == Some(&'.') {
                                chars.next(); // consume third dot
                                current_pos += 1;

                                // Now collect the variable name
                                while let Some(&next_ch) = chars.peek() {
                                    if next_ch.is_alphanumeric() || next_ch == '_' {
                                        name.push(chars.next().unwrap());
                                        current_pos += 1;
                                    } else {
                                        break;
                                    }
                                }

                                if name.is_empty() {
                                    return Err(AnalysisError::pattern_match_error(
                                        format!("Invalid ellipsis metavariable at position {}", current_pos)
                                    ));
                                }

                                tokens.push(Token::EllipsisMetavariable(name));
                            } else {
                                return Err(AnalysisError::pattern_match_error(
                                    format!("Invalid ellipsis pattern at position {}", current_pos)
                                ));
                            }
                        } else {
                            return Err(AnalysisError::pattern_match_error(
                                format!("Invalid ellipsis pattern at position {}", current_pos)
                            ));
                        }
                    } else {
                        // Regular metavariable
                        while let Some(&next_ch) = chars.peek() {
                            if next_ch.is_alphanumeric() || next_ch == '_' {
                                name.push(chars.next().unwrap());
                                current_pos += 1;
                            } else {
                                break;
                            }
                        }

                        if name.is_empty() {
                            return Err(AnalysisError::pattern_match_error(
                                format!("Invalid metavariable at position {}", current_pos)
                            ));
                        }

                        tokens.push(Token::Metavariable(name));
                    }
                }
                
                // Node type constraint
                '@' => {
                    let mut name = String::new();
                    while let Some(&next_ch) = chars.peek() {
                        if next_ch.is_alphanumeric() || next_ch == '_' {
                            name.push(chars.next().unwrap());
                            current_pos += 1;
                        } else {
                            break;
                        }
                    }
                    
                    if name.is_empty() {
                        return Err(AnalysisError::pattern_match_error(
                            format!("Invalid node type at position {}", current_pos)
                        ));
                    }
                    
                    tokens.push(Token::NodeType(name));
                }
                
                // Parentheses
                '(' => tokens.push(Token::LeftParen),
                ')' => tokens.push(Token::RightParen),
                
                // Alternative operator
                '|' => tokens.push(Token::Pipe),
                
                // Wildcard
                '.' => {
                    if chars.peek() == Some(&'.') {
                        chars.next(); // consume second dot
                        current_pos += 1;
                        if chars.peek() == Some(&'.') {
                            chars.next(); // consume third dot
                            current_pos += 1;
                            tokens.push(Token::Wildcard);
                        } else {
                            return Err(AnalysisError::pattern_match_error(
                                format!("Invalid wildcard at position {}", current_pos)
                            ));
                        }
                    } else {
                        // Single dot is treated as literal
                        tokens.push(Token::Literal(".".to_string()));
                    }
                }
                
                // String literals
                '"' => {
                    let mut literal = String::new();
                    let mut escaped = false;
                    
                    while let Some(next_ch) = chars.next() {
                        current_pos += 1;
                        
                        if escaped {
                            match next_ch {
                                'n' => literal.push('\n'),
                                't' => literal.push('\t'),
                                'r' => literal.push('\r'),
                                '\\' => literal.push('\\'),
                                '"' => literal.push('"'),
                                _ => {
                                    literal.push('\\');
                                    literal.push(next_ch);
                                }
                            }
                            escaped = false;
                        } else if next_ch == '\\' {
                            escaped = true;
                        } else if next_ch == '"' {
                            break;
                        } else {
                            literal.push(next_ch);
                        }
                    }
                    
                    tokens.push(Token::Literal(literal));
                }
                
                // Regular characters (treated as literal)
                _ => {
                    let mut literal = String::new();
                    literal.push(ch);
                    
                    // Continue collecting literal characters
                    while let Some(&next_ch) = chars.peek() {
                        if next_ch.is_alphanumeric() || "_-+*=<>!&^%#".contains(next_ch) {
                            literal.push(chars.next().unwrap());
                            current_pos += 1;
                        } else {
                            break;
                        }
                    }
                    
                    tokens.push(Token::Literal(literal));
                }
            }
        }

        Ok(tokens)
    }

    /// Parse tokens into a pattern
    fn parse_tokens(&self, tokens: &[Token]) -> Result<ParsedPattern> {
        if tokens.is_empty() {
            return Ok(ParsedPattern::Wildcard);
        }

        self.parse_alternative(tokens, 0).map(|(pattern, _)| pattern)
    }

    /// Parse alternative patterns (lowest precedence)
    fn parse_alternative(&self, tokens: &[Token], start: usize) -> Result<(ParsedPattern, usize)> {
        let (pattern, mut pos) = self.parse_sequence(tokens, start)?;
        let mut alternatives = vec![pattern];

        while pos < tokens.len() {
            if let Token::Pipe = tokens[pos] {
                pos += 1; // consume pipe
                let (alt_pattern, new_pos) = self.parse_sequence(tokens, pos)?;
                alternatives.push(alt_pattern);
                pos = new_pos;
            } else {
                break;
            }
        }

        if alternatives.len() == 1 {
            Ok((alternatives.into_iter().next().unwrap(), pos))
        } else {
            Ok((ParsedPattern::Alternative(alternatives), pos))
        }
    }

    /// Parse sequence patterns
    fn parse_sequence(&self, tokens: &[Token], start: usize) -> Result<(ParsedPattern, usize)> {
        let mut patterns = Vec::new();
        let mut pos = start;

        while pos < tokens.len() {
            match &tokens[pos] {
                Token::RightParen | Token::Pipe => break,
                _ => {
                    let (pattern, new_pos) = self.parse_primary(tokens, pos)?;
                    patterns.push(pattern);
                    pos = new_pos;
                }
            }
        }

        if patterns.is_empty() {
            Ok((ParsedPattern::Wildcard, pos))
        } else if patterns.len() == 1 {
            Ok((patterns.into_iter().next().unwrap(), pos))
        } else {
            Ok((ParsedPattern::Sequence(patterns), pos))
        }
    }

    /// Parse primary patterns (highest precedence)
    fn parse_primary(&self, tokens: &[Token], start: usize) -> Result<(ParsedPattern, usize)> {
        if start >= tokens.len() {
            return Err(AnalysisError::pattern_match_error("Unexpected end of pattern"));
        }

        match &tokens[start] {
            Token::Literal(s) => Ok((ParsedPattern::Literal(s.clone()), start + 1)),
            Token::Metavariable(s) => Ok((ParsedPattern::Metavariable(s.clone()), start + 1)),
            Token::EllipsisMetavariable(s) => Ok((ParsedPattern::EllipsisMetavariable(s.clone()), start + 1)),
            Token::NodeType(s) => Ok((ParsedPattern::NodeType(s.clone()), start + 1)),
            Token::Wildcard => Ok((ParsedPattern::Wildcard, start + 1)),
            Token::LeftParen => {
                let (pattern, pos) = self.parse_alternative(tokens, start + 1)?;
                if pos >= tokens.len() || !matches!(tokens[pos], Token::RightParen) {
                    return Err(AnalysisError::pattern_match_error("Missing closing parenthesis"));
                }
                Ok((pattern, pos + 1))
            }
            Token::RightParen => Err(AnalysisError::pattern_match_error("Unexpected closing parenthesis")),
            Token::Pipe => Err(AnalysisError::pattern_match_error("Unexpected pipe operator")),
        }
    }
}

impl Default for PatternParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Token types for pattern parsing
#[derive(Debug, Clone, PartialEq)]
enum Token {
    Literal(String),
    Metavariable(String),
    EllipsisMetavariable(String),
    NodeType(String),
    LeftParen,
    RightParen,
    Pipe,
    Wildcard,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_literal() {
        let parser = PatternParser::new();
        let pattern = parser.parse("hello").unwrap();
        assert_eq!(pattern, ParsedPattern::Literal("hello".to_string()));
    }

    #[test]
    fn test_parse_metavariable() {
        let parser = PatternParser::new();
        let pattern = parser.parse("$VAR").unwrap();
        assert_eq!(pattern, ParsedPattern::Metavariable("VAR".to_string()));
    }

    #[test]
    fn test_parse_node_type() {
        let parser = PatternParser::new();
        let pattern = parser.parse("@identifier").unwrap();
        assert_eq!(pattern, ParsedPattern::NodeType("identifier".to_string()));
    }

    #[test]
    fn test_parse_wildcard() {
        let parser = PatternParser::new();
        let pattern = parser.parse("...").unwrap();
        assert_eq!(pattern, ParsedPattern::Wildcard);
    }

    #[test]
    fn test_parse_sequence() {
        let parser = PatternParser::new();
        let pattern = parser.parse("hello $VAR world").unwrap();
        assert_eq!(
            pattern,
            ParsedPattern::Sequence(vec![
                ParsedPattern::Literal("hello".to_string()),
                ParsedPattern::Metavariable("VAR".to_string()),
                ParsedPattern::Literal("world".to_string()),
            ])
        );
    }

    #[test]
    fn test_parse_alternative() {
        let parser = PatternParser::new();
        let pattern = parser.parse("hello | world").unwrap();
        assert_eq!(
            pattern,
            ParsedPattern::Alternative(vec![
                ParsedPattern::Literal("hello".to_string()),
                ParsedPattern::Literal("world".to_string()),
            ])
        );
    }

    #[test]
    fn test_parse_parentheses() {
        let parser = PatternParser::new();
        let pattern = parser.parse("(hello | world) $VAR").unwrap();
        assert_eq!(
            pattern,
            ParsedPattern::Sequence(vec![
                ParsedPattern::Alternative(vec![
                    ParsedPattern::Literal("hello".to_string()),
                    ParsedPattern::Literal("world".to_string()),
                ]),
                ParsedPattern::Metavariable("VAR".to_string()),
            ])
        );
    }

    #[test]
    fn test_parse_string_literal() {
        let parser = PatternParser::new();
        let pattern = parser.parse("\"hello world\"").unwrap();
        assert_eq!(pattern, ParsedPattern::Literal("hello world".to_string()));
    }

    #[test]
    fn test_parse_escaped_string() {
        let parser = PatternParser::new();
        let pattern = parser.parse("\"hello\\nworld\"").unwrap();
        assert_eq!(pattern, ParsedPattern::Literal("hello\nworld".to_string()));
    }

    #[test]
    fn test_parse_invalid_metavariable() {
        let parser = PatternParser::new();
        let result = parser.parse("$");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_unmatched_parentheses() {
        let parser = PatternParser::new();
        let result = parser.parse("(hello");
        assert!(result.is_err());
    }

    #[test]
    fn test_pattern_display() {
        let pattern = ParsedPattern::Sequence(vec![
            ParsedPattern::Literal("hello".to_string()),
            ParsedPattern::Metavariable("VAR".to_string()),
        ]);
        assert_eq!(pattern.to_string(), "(\"hello\" $VAR)");
    }
}
