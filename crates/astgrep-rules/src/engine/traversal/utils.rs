//! Traversal module - Token utilities
//!
//! This module provides tokenization and utility functions for pattern matching.

use crate::engine::traversal::RuleExecutionEngine;

impl RuleExecutionEngine {
    /// Tokenize a string, preserving operators and punctuation as separate tokens.
    /// Note: recognizes "..." as a single Ellipsis token in patterns and text.
    pub(crate) fn tokenize(&self, s: &str) -> Vec<String> {
        self.tokenize_spanned(s)
            .into_iter()
            .map(|(t, _, _)| t)
            .collect()
    }

    /// Tokenize a pattern string with Semgrep-compatible post-processing.
    /// Specifically, coalesce `$ ...` into a single ellipsis token `...` to support `$...` syntax.
    pub(crate) fn tokenize_pattern(&self, s: &str) -> Vec<String> {
        let mut tokens = self.tokenize(s);
        if tokens.is_empty() {
            return tokens;
        }
        let mut coalesced: Vec<String> = Vec::with_capacity(tokens.len());
        let mut idx = 0usize;
        while idx < tokens.len() {
            if tokens[idx] == "$" && idx + 1 < tokens.len() && tokens[idx + 1] == "..." {
                coalesced.push("...".to_string());
                idx += 2;
            } else {
                coalesced.push(std::mem::take(&mut tokens[idx]));
                idx += 1;
            }
        }
        coalesced
    }

    /// Tokenize a string and return tokens with their byte spans (start, end)
    /// Note: recognizes "..." as a single Ellipsis token.
    pub(crate) fn tokenize_spanned(&self, s: &str) -> Vec<(String, usize, usize)> {
        use std::iter::Peekable;
        let mut tokens: Vec<(String, usize, usize)> = Vec::new();
        let mut current = String::new();
        let mut current_start: Option<usize> = None;
        let mut last_end: usize = 0;
        let mut it: Peekable<std::str::CharIndices<'_>> = s.char_indices().peekable();
        while let Some((i, ch)) = it.next() {
            let ch_end = i + ch.len_utf8();
            match ch {
                '+' | '-' | '*' | '/' | '%' | '=' | '<' | '>' | '!' | '&' | '|' | '^' | '~'
                | '?' | ':' | ';' | ',' | '(' | ')' | '[' | ']' | '{' | '}' | '.' => {
                    // flush current ident
                    if !current.is_empty() {
                        tokens.push((std::mem::take(&mut current), current_start.unwrap_or(i), i));
                        current_start = None;
                    }
                    // special case: ellipsis
                    if ch == '.' {
                        // check next two chars form "..."
                        let mut consumed_two = false;
                        if let Some(&(_i2, ch2)) = it.peek() {
                            if ch2 == '.' {
                                // consume second '.'
                                let _ = it.next();
                                if let Some(&(_i3, ch3)) = it.peek() {
                                    if ch3 == '.' {
                                        // consume third '.' and push ellipsis token
                                        let _ = it.next();
                                        tokens.push(("...".to_string(), i, i + 3));
                                        last_end = i + 3;
                                        consumed_two = true;
                                    }
                                }
                            }
                        }
                        if consumed_two {
                            continue;
                        }
                    }
                    // push as single-char token
                    tokens.push((ch.to_string(), i, ch_end));
                    last_end = ch_end;
                }
                '"' | '\'' | '\'' => {
                    // flush current ident
                    if !current.is_empty() {
                        tokens.push((std::mem::take(&mut current), current_start.unwrap_or(i), i));
                        current_start = None;
                    }
                    // read string literal
                    let quote = ch;
                    let start = i;
                    let mut val = String::new();
                    val.push(quote);
                    while let Some((_j, c)) = it.next() {
                        val.push(c);
                        if c == '\\' {
                            if let Some((_, c2)) = it.next() {
                                val.push(c2);
                            }
                        } else if c == quote {
                            break;
                        }
                    }
                    tokens.push((val.clone(), start, start + val.len()));
                    last_end = start + val.len();
                }
                _ if ch.is_ascii_alphanumeric() || ch == '_' || ch == '$' => {
                    if current.is_empty() {
                        current_start = Some(i);
                    }
                    current.push(ch);
                }
                _ => {
                    // flush current ident on any other char (including whitespace)
                    if !current.is_empty() {
                        tokens.push((std::mem::take(&mut current), current_start.unwrap_or(i), i));
                        current_start = None;
                    }
                }
            }
        }
        if !current.is_empty() {
            tokens.push((
                std::mem::take(&mut current),
                current_start.unwrap_or(last_end),
                s.len(),
            ));
        }
        tokens
    }

    /// Convert a byte index in `s` to 1-based (line, column)
    pub(crate) fn byte_index_to_line_col(s: &str, byte_idx: usize) -> (usize, usize) {
        let mut line: usize = 1;
        let mut col: usize = 1;
        for (ci, ch) in s.char_indices() {
            if ci >= byte_idx {
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }
        (line, col)
    }

    /// Convert 1-based (line, column) to byte index in `s`
    pub(crate) fn line_col_to_byte_index(
        &self,
        s: &str,
        target_line: usize,
        target_col: usize,
    ) -> usize {
        let mut line: usize = 1;
        let mut col: usize = 1;
        for (ci, ch) in s.char_indices() {
            if line == target_line && col == target_col {
                return ci;
            }
            if ch == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }
        s.len()
    }
}
