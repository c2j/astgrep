//! Taint environment for forward dataflow analysis
//!
//! Tracks taint state through variable assignments using a simple
//! variable-centric state machine rather than heuristic text matching.

use std::collections::{HashMap, HashSet};

/// Taint state for a single variable
#[derive(Debug, Clone)]
pub struct TaintState {
    /// Whether this variable is currently tainted
    pub tainted: bool,
    /// Lines where taint was introduced (source pattern match lines)
    pub source_lines: HashSet<usize>,
    /// Index into the source TaintMatch array that contributed taint
    pub source_idx: Option<usize>,
}

/// Environment mapping variable names to their taint state.
///
/// Used for forward dataflow analysis within a single scope (method/function).
/// Walks source text top-to-bottom, tracking which variables are tainted
/// and propagating taint through assignments.
#[derive(Debug, Clone)]
pub struct TaintEnv {
    /// Variable name -> TaintState
    state: HashMap<String, TaintState>,
}

impl TaintEnv {
    /// Create a new empty taint environment
    pub fn new() -> Self {
        Self {
            state: HashMap::new(),
        }
    }

    /// Mark a variable as tainted from a source at a given line
    pub fn taint(&mut self, var: &str, source_line: usize, source_idx: usize) {
        let entry = self.state.entry(var.to_string()).or_insert(TaintState {
            tainted: false,
            source_lines: HashSet::new(),
            source_idx: None,
        });
        entry.tainted = true;
        entry.source_lines.insert(source_line);
        entry.source_idx = Some(source_idx);
    }

    /// Check if a variable is currently tainted
    pub fn is_tainted(&self, var: &str) -> bool {
        self.state.get(var).map(|s| s.tainted).unwrap_or(false)
    }

    /// Get the source index that tainted a variable
    pub fn get_source_idx(&self, var: &str) -> Option<usize> {
        self.state.get(var)?.source_idx
    }

    /// Propagate taint from source var to target var (for assignments)
    pub fn propagate(&mut self, target: &str, source: &str) {
        if let Some(source_state) = self.state.get(source).cloned() {
            if source_state.tainted {
                self.state.insert(target.to_string(), source_state);
            }
        }
    }

    /// Sanitize a variable (remove taint)
    pub fn sanitize(&mut self, var: &str) {
        if let Some(state) = self.state.get_mut(var) {
            state.tainted = false;
        }
    }

    /// Untaint a variable (reassignment to known-safe value)
    pub fn untaint(&mut self, var: &str) {
        if let Some(state) = self.state.get_mut(var) {
            state.tainted = false;
            state.source_lines.clear();
            state.source_idx = None;
        }
    }

    /// Get all currently tainted variable names
    pub fn tainted_vars(&self) -> Vec<String> {
        self.state
            .iter()
            .filter(|(_, s)| s.tainted)
            .map(|(k, _)| k.clone())
            .collect()
    }

    /// Clear all state
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.state.clear();
    }

    /// Fork environment for branch analysis (returns a clone)
    #[allow(dead_code)]
    pub fn fork(&self) -> TaintEnv {
        self.clone()
    }

    /// Merge branch environments (union semantics: tainted if tainted in either branch)
    #[allow(dead_code)]
    pub fn merge(&mut self, other: &TaintEnv) {
        for (var, other_state) in &other.state {
            if other_state.tainted {
                let entry = self
                    .state
                    .entry(var.clone())
                    .or_insert(TaintState {
                        tainted: false,
                        source_lines: HashSet::new(),
                        source_idx: None,
                    });
                entry.tainted = true;
                for line in &other_state.source_lines {
                    entry.source_lines.insert(*line);
                }
                if entry.source_idx.is_none() {
                    entry.source_idx = other_state.source_idx;
                }
            }
        }
    }
}

impl Default for TaintEnv {
    fn default() -> Self {
        Self::new()
    }
}

// ── Helper functions for line-by-line taint processing ──

/// Find the position of an assignment `=` sign, ignoring `==`, `!=`, `<=`, `>=`
pub fn find_assignment_eq(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'=' {
            let prev = if i > 0 { bytes[i - 1] } else { b' ' };
            let next = if i + 1 < bytes.len() { bytes[i + 1] } else { b' ' };
            if prev != b'=' && prev != b'!' && prev != b'<' && prev != b'>' && next != b'=' {
                return Some(i);
            }
        }
    }
    None
}

/// Extract the target variable from an assignment LHS.
/// Handles: `x`, `Type x`, `this.x`, `final Type x`
pub fn extract_target_var(lhs: &str) -> String {
    let lhs = lhs.trim();
    // Handle "this.field" → strip prefix for consistent lookup
    let normalized = lhs.strip_prefix("this.").unwrap_or(lhs);
    // Handle "Type var" → take last word
    let parts: Vec<&str> = normalized.split_whitespace().collect();
    if let Some(last) = parts.last() {
        last.trim().to_string()
    } else {
        normalized.to_string()
    }
}

/// Check if a value expression references a variable using word-boundary matching.
pub fn contains_var_reference(expr: &str, var: &str) -> bool {
    if var.is_empty() {
        return false;
    }
    for (pos, _) in expr.match_indices(var) {
        let before_ok = pos == 0
            || {
                let c = expr.as_bytes()[pos - 1];
                !c.is_ascii_alphanumeric() && c != b'_'
            };
        let after_ok = pos + var.len() >= expr.len()
            || {
                let c = expr.as_bytes()[pos + var.len()];
                !c.is_ascii_alphanumeric() && c != b'_'
            };
        if before_ok && after_ok {
            return true;
        }
    }
    false
}

/// Check if a value is a known-safe literal (numeric, boolean, null, empty string)
pub fn is_safe_value(value: &str) -> bool {
    let v = value.trim().trim_end_matches(';').trim();
    if v.is_empty() {
        return true;
    }
    if v.parse::<i64>().is_ok() || v.parse::<f64>().is_ok() {
        return true;
    }
    if v == "true" || v == "false" {
        return true;
    }
    if v == "None" || v == "null" || v == "nil" {
        return true;
    }
    if v == "\"\"" || v == "''" {
        return true;
    }
    if (v.starts_with('"') && v.ends_with('"') && v.len() <= 2)
        || (v.starts_with('\'') && v.ends_with('\'') && v.len() <= 2)
    {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_taint_env_basic() {
        let mut env = TaintEnv::new();
        assert!(!env.is_tainted("x"));

        env.taint("x", 1, 0);
        assert!(env.is_tainted("x"));
        assert_eq!(env.get_source_idx("x"), Some(0));

        let tainted = env.tainted_vars();
        assert_eq!(tainted, vec!["x"]);
    }

    #[test]
    fn test_taint_env_propagate() {
        let mut env = TaintEnv::new();
        env.taint("source", 1, 0);

        env.propagate("a", "source");
        assert!(env.is_tainted("a"));
        assert_eq!(env.get_source_idx("a"), Some(0));

        env.propagate("b", "a");
        assert!(env.is_tainted("b"));
        assert_eq!(env.get_source_idx("b"), Some(0));
    }

    #[test]
    fn test_taint_env_sanitize() {
        let mut env = TaintEnv::new();
        env.taint("x", 1, 0);
        assert!(env.is_tainted("x"));

        env.sanitize("x");
        assert!(!env.is_tainted("x"));
        // source_idx still present (for tracking)
        assert_eq!(env.get_source_idx("x"), Some(0));
    }

    #[test]
    fn test_taint_env_untaint() {
        let mut env = TaintEnv::new();
        env.taint("x", 1, 0);
        env.untaint("x");
        assert!(!env.is_tainted("x"));
        assert_eq!(env.get_source_idx("x"), None);
    }

    #[test]
    fn test_taint_env_fork_merge() {
        let mut env = TaintEnv::new();
        env.taint("x", 1, 0);

        let mut branch = env.fork();
        branch.taint("y", 2, 1);

        env.merge(&branch);
        assert!(env.is_tainted("x"));
        assert!(env.is_tainted("y"));
    }

    #[test]
    fn test_find_assignment_eq() {
        assert_eq!(find_assignment_eq("x = 1"), Some(2));
        assert_eq!(find_assignment_eq("x == 1"), None);
        assert_eq!(find_assignment_eq("x != 1"), None);
        assert_eq!(find_assignment_eq("x <= 1"), None);
        assert_eq!(find_assignment_eq("x >= 1"), None);
        assert_eq!(find_assignment_eq("if (x == 1)"), None);
        assert_eq!(find_assignment_eq("String x = foo()"), Some(9));
    }

    #[test]
    fn test_extract_target_var() {
        assert_eq!(extract_target_var("x"), "x");
        assert_eq!(extract_target_var("String x"), "x");
        assert_eq!(extract_target_var("final String x"), "x");
        assert_eq!(extract_target_var("this.x"), "x");
    }

    #[test]
    fn test_contains_var_reference() {
        assert!(contains_var_reference("sink(x)", "x"));
        assert!(contains_var_reference("x + y", "x"));
        assert!(contains_var_reference("f(x, y)", "x"));
        assert!(!contains_var_reference("sax", "x"));
        assert!(!contains_var_reference("ex", "x"));
        assert!(contains_var_reference("this.x", "x"));
    }

    #[test]
    fn test_is_safe_value() {
        assert!(is_safe_value("42"));
        assert!(is_safe_value("3.14"));
        assert!(is_safe_value("true"));
        assert!(is_safe_value("false"));
        assert!(is_safe_value("null"));
        assert!(is_safe_value("None"));
        assert!(is_safe_value("\"\""));
        assert!(is_safe_value("''"));
        assert!(!is_safe_value("\"hello\""));
        assert!(!is_safe_value("getUserInput()"));
    }
}
