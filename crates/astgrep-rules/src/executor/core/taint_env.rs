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
    /// Variable name -> set of labels associated with its taint
    labels: HashMap<String, HashSet<String>>,
    /// Variable name -> set of tainted field paths (e.g., {"a", "a.b.c"})
    field_taints: HashMap<String, HashSet<String>>,
    /// Variable name -> set of explicitly clean field paths (sanitized fields)
    clean_fields: HashMap<String, HashSet<String>>,
}

impl TaintEnv {
    /// Create a new empty taint environment
    pub fn new() -> Self {
        Self {
            state: HashMap::new(),
            labels: HashMap::new(),
            field_taints: HashMap::new(),
            clean_fields: HashMap::new(),
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

    /// Mark a variable as tainted and associate a label with it
    pub fn taint_with_label(
        &mut self,
        var: &str,
        source_line: usize,
        source_idx: usize,
        label: Option<&str>,
    ) {
        self.taint(var, source_line, source_idx);
        if let Some(lbl) = label {
            self.labels
                .entry(var.to_string())
                .or_default()
                .insert(lbl.to_string());
        }
    }

    /// Get all labels associated with a tainted variable
    pub fn get_labels(&self, var: &str) -> HashSet<String> {
        self.labels.get(var).cloned().unwrap_or_default()
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
                if let Some(source_labels) = self.labels.get(source).cloned() {
                    self.labels
                        .entry(target.to_string())
                        .or_default()
                        .extend(source_labels);
                }
            }
        }
    }

    /// Sanitize a variable (remove taint)
    pub fn sanitize(&mut self, var: &str) {
        if let Some(state) = self.state.get_mut(var) {
            state.tainted = false;
        }
        self.labels.remove(var);
    }

    /// Untaint a variable (reassignment to known-safe value)
    pub fn untaint(&mut self, var: &str) {
        if let Some(state) = self.state.get_mut(var) {
            state.tainted = false;
            state.source_lines.clear();
            state.source_idx = None;
        }
        self.labels.remove(var);
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
        self.labels.clear();
        self.field_taints.clear();
        self.clean_fields.clear();
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
                let entry = self.state.entry(var.clone()).or_insert(TaintState {
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
        for (var, other_labels) in &other.labels {
            let entry = self.labels.entry(var.clone()).or_default();
            for label in other_labels {
                entry.insert(label.clone());
            }
        }
        for (var, other_fields) in &other.field_taints {
            let entry = self.field_taints.entry(var.clone()).or_default();
            for field in other_fields {
                entry.insert(field.clone());
            }
        }
    }

    pub fn taint_field(&mut self, var: &str, field: &str, source_line: usize, source_idx: usize) {
        self.field_taints
            .entry(var.to_string())
            .or_default()
            .insert(field.to_string());
        if !self.is_tainted(var) {
            self.taint(var, source_line, source_idx);
        }
    }

    pub fn is_field_tainted(&self, var: &str, field: &str) -> bool {
        if let Some(fields) = self.field_taints.get(var) {
            for tainted_field in fields {
                if field.starts_with(tainted_field) {
                    let rest = &field[tainted_field.len()..];
                    if rest.is_empty() || rest.starts_with('.') || rest.starts_with('[') {
                        return true;
                    }
                }
                if let Some(rest) = tainted_field.strip_prefix(field) {
                    if rest.is_empty() || rest.starts_with('.') || rest.starts_with('[') {
                        return true;
                    }
                }
            }
        }
        if self.is_clean_field(var, field) {
            return false;
        }
        if self.is_tainted(var) && !self.has_field_taints(var) {
            return true;
        }
        false
    }

    fn is_clean_field(&self, var: &str, field: &str) -> bool {
        if let Some(clean) = self.clean_fields.get(var) {
            for cf in clean {
                if field == *cf {
                    return true;
                }
                if field.starts_with(cf) {
                    let rest = &field[cf.len()..];
                    if rest.starts_with('.') || rest.starts_with('[') {
                        return true;
                    }
                }
            }
        }
        false
    }

    pub fn has_field_taints(&self, var: &str) -> bool {
        self.field_taints.get(var).is_some_and(|f| !f.is_empty())
    }

    pub fn add_label(&mut self, var: &str, label: String) {
        self.labels
            .entry(var.to_string())
            .or_default()
            .insert(label);
    }

    pub fn add_global_label(&mut self, label: String) {
        self.labels
            .entry("__global__".to_string())
            .or_default()
            .insert(label);
    }

    pub fn get_all_labels(&self) -> HashSet<String> {
        let mut all = HashSet::new();
        for labels in self.labels.values() {
            all.extend(labels.clone());
        }
        all
    }

    pub fn get_field_taints(&self, var: &str) -> HashSet<String> {
        self.field_taints.get(var).cloned().unwrap_or_default()
    }

    pub fn sanitize_field(&mut self, var: &str, field: &str) {
        if let Some(fields) = self.field_taints.get_mut(var) {
            fields.retain(|f| !f.starts_with(field) && !field.starts_with(f));
        }
        self.clean_fields
            .entry(var.to_string())
            .or_default()
            .insert(field.to_string());
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
            let next = if i + 1 < bytes.len() {
                bytes[i + 1]
            } else {
                b' '
            };
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
        let before_ok = pos == 0 || {
            let c = expr.as_bytes()[pos - 1];
            !c.is_ascii_alphanumeric() && c != b'_'
        };
        let after_ok = pos + var.len() >= expr.len() || {
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

/// Check if a tainted variable appears as an array index in an expression.
/// When `assume_safe_indexes` is enabled, taint should NOT propagate
/// through array index access (e.g., `x = a[i]` where `i` is tainted).
pub fn is_tainted_as_array_index(expr: &str, var: &str) -> bool {
    if var.is_empty() || expr.len() < 3 {
        return false;
    }
    // Look for the variable name followed by ']' (end of index)
    // or preceded by '[' (start of index)
    // Patterns: [var], [var +, [var -, [var *, etc.
    let bracket_patterns = [
        format!("[{}]", var),
        format!("[{} ", var),
        format!("[{}=", var),
        format!("[{}*", var),
        format!("[{}/", var),
        format!("[{}+", var),
        format!("[{}-", var),
        format!("[{}%", var),
    ];
    for pattern in &bracket_patterns {
        if expr.contains(pattern.as_str()) {
            return true;
        }
    }
    false
}

/// Evaluate a `requires` expression against a set of labels.
///
/// Supports:
/// - Single label: `"TAINTED"` → true if TAINTED present
/// - Conjunction: `"P and Q"` → true if both present
/// - Negation: `"TAINTED and not CLEANED"` → true if TAINTED present AND CLEANED absent
/// - Built-in: `"__SOURCE__"` → true if any taint present
pub fn evaluate_requires(expr: &str, labels: &HashSet<String>, has_taint: bool) -> bool {
    let expr = expr.trim();
    if expr.is_empty() {
        return true;
    }
    let parts: Vec<&str> = expr.split(" and ").collect();
    for part in parts {
        let part = part.trim();
        if part.starts_with("not ") {
            let negated_label = part[4..].trim();
            if labels.contains(negated_label) {
                return false;
            }
        } else if part == "__SOURCE__" {
            if !has_taint {
                return false;
            }
        } else {
            if !labels.contains(part) {
                return false;
            }
        }
    }
    true
}

/// Extract (base_var, optional_field_path) from a target expression.
/// "x" → ("x", None)
/// "x.a" → ("x", Some("a"))
/// "x.a.b" → ("x", Some("a.b"))
/// "x[i]" → ("x", Some("["))
/// "x.a[i]" → ("x", Some("a["))
/// "this.x.a" → ("x", Some("a"))
pub fn extract_field_path(target: &str) -> (String, Option<String>) {
    let target = target.trim();
    let normalized = target.strip_prefix("this.").unwrap_or(target);
    let parts: Vec<&str> = normalized.split_whitespace().collect();
    let base = parts.last().unwrap_or(&normalized);
    if let Some(dot_pos) = base.find('.') {
        let var = base[..dot_pos].to_string();
        let field_raw = &base[dot_pos + 1..];
        let mut normalized_parts: Vec<&str> = Vec::new();
        for segment in field_raw.split('.') {
            let before_bracket = segment.split('[').next().unwrap_or(segment);
            if !before_bracket.is_empty() {
                normalized_parts.push(before_bracket);
            }
        }
        let field = normalized_parts.join(".");
        (var, Some(field))
    } else if base.contains('[') {
        let bracket_pos = base.find('[').unwrap();
        let var = base[..bracket_pos].to_string();
        (var, None)
    } else {
        (base.to_string(), None)
    }
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

    #[test]
    fn test_extract_field_path() {
        assert_eq!(extract_field_path("x"), ("x".to_string(), None));
        assert_eq!(
            extract_field_path("x.a"),
            ("x".to_string(), Some("a".to_string()))
        );
        assert_eq!(
            extract_field_path("x.a.b"),
            ("x".to_string(), Some("a.b".to_string()))
        );
        assert_eq!(
            extract_field_path("x.c[i]"),
            ("x".to_string(), Some("c".to_string()))
        );
        assert_eq!(
            extract_field_path("x.c[i].d"),
            ("x".to_string(), Some("c.d".to_string()))
        );
        assert_eq!(
            extract_field_path("this.x.a"),
            ("x".to_string(), Some("a".to_string()))
        );
    }

    #[test]
    fn test_field_sensitive_taint() {
        let mut env = TaintEnv::new();
        env.taint("x", 1, 0);

        assert!(env.is_field_tainted("x", "a"));
        assert!(env.is_field_tainted("x", "b"));
        assert!(env.is_field_tainted("x", "a.b"));

        env.sanitize_field("x", "a");
        assert!(!env.is_field_tainted("x", "a"));
        assert!(!env.is_field_tainted("x", "a.b"));
        assert!(env.is_field_tainted("x", "b"));
    }

    #[test]
    fn test_field_taint_propagation() {
        let mut env = TaintEnv::new();
        env.taint_field("x", "a", 1, 0);

        assert!(env.is_tainted("x"));
        assert!(env.is_field_tainted("x", "a"));
        assert!(env.is_field_tainted("x", "a.b"));
        assert!(!env.is_field_tainted("x", "b"));
    }

    #[test]
    fn test_clean_fields_vs_whole_var_taint() {
        let mut env = TaintEnv::new();
        env.taint("x", 1, 0);
        env.sanitize_field("x", "a");

        assert!(env.is_tainted("x"));
        assert!(!env.is_field_tainted("x", "a"));
        assert!(env.is_field_tainted("x", "b"));
    }

    #[test]
    fn test_extract_field_path_bracket_normalization() {
        assert_eq!(
            extract_field_path("x.data[idx]"),
            ("x".to_string(), Some("data".to_string()))
        );
        assert_eq!(
            extract_field_path("x.data[0].name"),
            ("x".to_string(), Some("data.name".to_string()))
        );
    }
}
