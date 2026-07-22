//! Text pattern classification for multi-pattern merge optimization.

use aho_corasick::{AhoCorasick, AhoCorasickBuilder, AhoCorasickKind, MatchKind};

/// A classified text pattern ready for matching.
#[derive(Debug, Clone)]
pub(crate) enum TextPattern {
    /// Exact literal string (no metavariables, no regex syntax).
    /// Eligible for Aho-Corasick batch matching.
    Literal {
        text: String,
        rule_id: String,
        pattern_index: usize,
    },
    /// Pattern containing semgrep metavariables ($VAR, ...) or regex syntax.
    /// Must use individual regex matching.
    Regex {
        regex_str: String,
        rule_id: String,
        pattern_index: usize,
    },
}

impl TextPattern {
    /// Classify a simple pattern string. Returns None if the pattern
    /// requires AST matching and should not use text-based matching.
    pub(crate) fn classify(
        pattern_str: &str,
        rule_id: String,
        pattern_index: usize,
    ) -> Option<Self> {
        if pattern_str.contains('@')
            || pattern_str.contains('{')
            || pattern_str.contains('=')
        {
            return None;
        }

        if pattern_str.contains('$') || pattern_str.contains("...") {
            let regex_str =
                crate::engine::traversal::matching::semgrep_pattern_to_regex(pattern_str);
            return Some(TextPattern::Regex {
                regex_str,
                rule_id,
                pattern_index,
            });
        }

        Some(TextPattern::Literal {
            text: pattern_str.to_string(),
            rule_id,
            pattern_index,
        })
    }
}

/// Batch matcher for literal text patterns.
/// Uses Aho-Corasick to match ALL literal patterns in a single pass.
pub(crate) struct LiteralPatternMatcher {
    ac: AhoCorasick,
    pattern_info: Vec<(String, usize)>,
}

impl LiteralPatternMatcher {
    pub(crate) fn build(literals: &[TextPattern]) -> Option<Self> {
        let literal_patterns: Vec<&TextPattern> = literals
            .iter()
            .filter(|p| matches!(p, TextPattern::Literal { .. }))
            .collect();

        if literal_patterns.is_empty() {
            return None;
        }

        let texts: Vec<&str> = literal_patterns
            .iter()
            .map(|p| match p {
                TextPattern::Literal { text, .. } => text.as_str(),
                _ => unreachable!(),
            })
            .collect();

        let pattern_info: Vec<(String, usize)> = literal_patterns
            .iter()
            .map(|p| match p {
                TextPattern::Literal {
                    rule_id,
                    pattern_index,
                    ..
                } => (rule_id.clone(), *pattern_index),
                _ => unreachable!(),
            })
            .collect();

        let ac = AhoCorasickBuilder::new()
            .kind(Some(AhoCorasickKind::DFA))
            .match_kind(MatchKind::Standard)
            .build(&texts)
            .expect("Aho-Corasick build should fail only on empty patterns (guarded above)");

        Some(Self { ac, pattern_info })
    }

    pub(crate) fn scan(&self, source: &str) -> Vec<(&str, usize, usize, usize)> {
        self.ac
            .find_overlapping_iter(source)
            .map(|mat| {
                let (ref rule_id, pattern_index) = self.pattern_info[mat.pattern().as_usize()];
                (rule_id.as_str(), pattern_index, mat.start(), mat.end())
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_literal() {
        let p = TextPattern::classify("SELECT", "r1".into(), 0);
        assert!(matches!(p, Some(TextPattern::Literal { .. })));
    }

    #[test]
    fn test_classify_metavar_becomes_regex() {
        let p = TextPattern::classify("SELECT $COL", "r1".into(), 0);
        assert!(matches!(p, Some(TextPattern::Regex { .. })));
    }

    #[test]
    fn test_classify_metadata_binding_skipped() {
        let p = TextPattern::classify("$VAR@attr", "r1".into(), 0);
        assert!(p.is_none());
    }

    #[test]
    fn test_classify_binary_op_skipped() {
        let p = TextPattern::classify("a = b", "r1".into(), 0);
        assert!(p.is_none());
    }

    #[test]
    fn test_literal_matcher_basic() {
        let patterns = vec![
            TextPattern::Literal { text: "SELECT".into(), rule_id: "r1".into(), pattern_index: 0 },
            TextPattern::Literal { text: "DELETE".into(), rule_id: "r2".into(), pattern_index: 0 },
            TextPattern::Literal { text: "UPDATE".into(), rule_id: "r3".into(), pattern_index: 0 },
        ];
        let matcher = LiteralPatternMatcher::build(&patterns).unwrap();
        let source = "SELECT * FROM t; DELETE FROM t;";
        let results = matcher.scan(source);
        assert_eq!(results.len(), 2);
        let rids: Vec<&str> = results.iter().map(|(rid, _, _, _)| *rid).collect();
        assert!(rids.contains(&"r1"));
        assert!(rids.contains(&"r2"));
    }

    #[test]
    fn test_literal_matcher_empty_returns_none() {
        let patterns: Vec<TextPattern> = vec![];
        assert!(LiteralPatternMatcher::build(&patterns).is_none());
        let regex_only = vec![
            TextPattern::Regex { regex_str: "\\d+".into(), rule_id: "r1".into(), pattern_index: 0 },
        ];
        assert!(LiteralPatternMatcher::build(&regex_only).is_none());
    }

    #[test]
    fn test_literal_matcher_sql_boundary() {
        let patterns = vec![
            TextPattern::Literal { text: "SELECT".into(), rule_id: "r1".into(), pattern_index: 0 },
            TextPattern::Literal { text: "INSERT".into(), rule_id: "r2".into(), pattern_index: 0 },
        ];
        let matcher = LiteralPatternMatcher::build(&patterns).unwrap();
        let source = "SELECT 1; INSERT INTO t VALUES (1);";
        let results = matcher.scan(source);
        assert_eq!(results.len(), 2);
    }
}
