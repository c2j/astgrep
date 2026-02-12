//! Variable dependency tracker for intra-procedural dataflow analysis
//!
//! Tracks which variables depend on (are derived from) other variables

use std::collections::HashMap;

/// Variable dependency tracker for intra-procedural dataflow analysis
/// Tracks which variables depend on (are derived from) other variables
pub struct VariableDependencyGraph {
    /// Maps a variable to the set of variables it depends on
    pub dependencies: HashMap<String, Vec<String>>,
    /// Maps a variable to its assigned expression text
    pub assignments: HashMap<String, String>,
    /// Maps object fields to their tainted status (object_name.field_name -> tainted_by)
    pub field_taints: HashMap<String, Vec<String>>,
    /// Maps getter calls to their source fields (e.g., "e.getX()" -> "e.x")
    pub getter_to_field: HashMap<String, String>,
    /// Custom propagator rules
    pub propagators: Vec<crate::types::PropagatorPattern>,
}

impl VariableDependencyGraph {
    pub fn new() -> Self {
        Self {
            dependencies: HashMap::new(),
            assignments: HashMap::new(),
            field_taints: HashMap::new(),
            getter_to_field: HashMap::new(),
            propagators: Vec::new(),
        }
    }

    pub fn with_propagators(mut self, propagators: Vec<crate::types::PropagatorPattern>) -> Self {
        self.propagators = propagators;
        self
    }

    /// Record that `target` variable is assigned from `source_vars`
    pub fn record_assignment(&mut self, target: String, source_vars: Vec<String>, expr: String) {
        eprintln!(
            "[DEBUG] Recording assignment: {} depends on {:?} (expr: {})",
            target, source_vars, expr
        );
        self.dependencies
            .insert(target.clone(), source_vars.clone());
        self.assignments.insert(target.clone(), expr);
        eprintln!(
            "[DEBUG] Assignment recorded. Dependencies for '{}': {:?}",
            target,
            self.dependencies.get(&target)
        );
    }

    /// Record that an object's field is tainted by a source
    pub fn record_field_taint(&mut self, object: &str, field: &str, source: &str) {
        let key = format!("{}.{}", object, field);
        let sources = self.field_taints.entry(key).or_insert_with(Vec::new);
        if !sources.contains(&source.to_string()) {
            sources.push(source.to_string());
        }
    }

    /// Map a getter call to its corresponding field
    pub fn record_getter_mapping(&mut self, getter_call: &str, object: &str, field: &str) {
        let field_key = format!("{}.{}", object, field);
        self.getter_to_field
            .insert(getter_call.to_string(), field_key);
    }

    /// Check if a getter call returns a tainted field
    pub fn is_getter_tainted(&self, getter_call: &str, source_vars: &[String]) -> bool {
        eprintln!(
            "[DEBUG] is_getter_tainted: checking '{}', source_vars={:?}",
            getter_call, source_vars
        );

        // Check if this getter maps to a field
        if let Some(field_key) = self.getter_to_field.get(getter_call) {
            eprintln!(
                "[DEBUG] Found getter mapping: {} -> {}",
                getter_call, field_key
            );
            // Check if the field itself is in source_vars (field-level source)
            if source_vars.contains(field_key) {
                eprintln!("[DEBUG] Match found! Field {} is in source_vars", field_key);
                return true;
            }
            // Also check if the field is tainted by any source
            if let Some(taint_sources) = self.field_taints.get(field_key) {
                eprintln!(
                    "[DEBUG] Field {} has taint sources: {:?}",
                    field_key, taint_sources
                );
                for taint_source in taint_sources {
                    if source_vars.contains(taint_source) {
                        eprintln!(
                            "[DEBUG] Match found! {} is tainted by {}",
                            field_key, taint_source
                        );
                        return true;
                    }
                }
            } else {
                eprintln!("[DEBUG] Field {} has no taint sources", field_key);
            }
        } else {
            eprintln!("[DEBUG] No getter mapping found for {}", getter_call);
        }
        false
    }

    /// Check if `var` depends on (transitively) any of the `source_vars`
    /// If `check_safe_context` is true, returns false if the dependency path goes through a safe numeric context
    pub fn depends_on(&self, var: &str, source_vars: &[String], check_safe_context: bool) -> bool {
        let mut visited = std::collections::HashSet::new();
        self.check_dependency_recursive(var, source_vars, &mut visited, check_safe_context)
    }

    fn check_dependency_recursive(
        &self,
        var: &str,
        source_vars: &[String],
        visited: &mut std::collections::HashSet<String>,
        check_safe_context: bool,
    ) -> bool {
        if !visited.insert(var.to_string()) {
            return false; // Already visited, avoid cycles
        }

        // Direct match
        if source_vars.iter().any(|s| s == var) {
            return true;
        }

        // Check field-level taint for getter calls (e.g., e.getX())
        if var.contains(".get") && var.ends_with("()") {
            if self.is_getter_tainted(var, source_vars) {
                return true;
            }
        }

        // Check method calls on variables (e.g., sqlBuilder.toString())
        // If var is "obj.method()", also check if "obj" depends on source_vars
        if var.contains(".") && var.ends_with(")") {
            // Extract the receiver object (e.g., "sqlBuilder" from "sqlBuilder.toString()")
            if let Some(dot_pos) = var.find('.') {
                let receiver = &var[..dot_pos];
                eprintln!("[DEBUG] Method call detected: '{}' has receiver '{}', checking if receiver depends on source", var, receiver);
                if self.check_dependency_recursive(
                    receiver,
                    source_vars,
                    visited,
                    check_safe_context,
                ) {
                    eprintln!(
                        "[DEBUG] Receiver '{}' depends on source_vars, returning true",
                        receiver
                    );
                    return true;
                }
            }
        }

        // Check transitive dependencies
        if let Some(deps) = self.dependencies.get(var) {
            for dep in deps {
                // Check if this dependency is through a safe numeric context
                if check_safe_context {
                    if let Some(expr) = self.assignments.get(var) {
                        if self.is_safe_numeric_expression_advanced(expr, &self.assignments) {
                            // This variable is assigned from a safe numeric expression,
                            // so don't consider it as tainted even if it depends on source
                            eprintln!(
                                "[DEBUG] Variable '{}' assigned from safe numeric expression: {}",
                                var, expr
                            );
                            continue;
                        }
                    }
                }

                if self.check_dependency_recursive(dep, source_vars, visited, check_safe_context) {
                    return true;
                }
            }
        }

        false
    }

    /// Check if an expression is in a safe numeric context
    /// Also checks if any variables in the expression are assigned string values
    fn is_safe_numeric_expression_advanced(
        &self,
        expr: &str,
        var_assignments: &HashMap<String, String>,
    ) -> bool {
        let expr = expr.trim();

        // IMPORTANT: If the expression contains string literals, it's likely string concatenation
        // which should NOT be considered safe numeric context
        if expr.contains('"') || expr.contains('\'') {
            return false;
        }

        // Check if any variable in the expression is assigned a string value
        let vars_in_expr = self.extract_variables_from_expression(expr);
        for var in &vars_in_expr {
            if let Some(assign_expr) = var_assignments.get(var) {
                let assign_expr = assign_expr.trim();
                // If the assignment expression contains string literals, it's a string variable
                if assign_expr.contains('"') || assign_expr.contains('\'') {
                    eprintln!(
                        "[DEBUG] Variable '{}' is assigned a string value: {}",
                        var, assign_expr
                    );
                    return false;
                }
            }
        }

        // Now check for numeric patterns
        self.is_safe_numeric_expression(expr)
    }

    /// Check if an expression is in a safe numeric context (basic check)
    fn is_safe_numeric_expression(&self, expr: &str) -> bool {
        let expr = expr.trim();

        // Check for numeric method calls: getSomething(), x.length, etc.
        // These typically return numeric values
        let numeric_method_patterns = [
            ".getSomething()",
            ".length",
            ".size()",
            ".count()",
            ".indexOf(",
            ".lastIndexOf(",
            ".compareTo(",
        ];

        for pattern in &numeric_method_patterns {
            if expr.contains(pattern) {
                return true;
            }
        }

        // Check for type casts to numeric types
        if regex::Regex::new(r"\(int\)|\(long\)|\(short\)|\(byte\)|\(float\)|\(double\)|\(Integer\)|\(Long\)|\(Short\)|\(Byte\)|\(Float\)|\(Double\)").ok()
            .map(|re| re.is_match(expr))
            .unwrap_or(false) {
            return true;
        }

        // Check for numeric wrapper conversions
        let numeric_conversions = [
            "Integer.valueOf(",
            "Integer.parseInt(",
            "Long.valueOf(",
            "Long.parseLong(",
            "Short.valueOf(",
            "Short.parseShort(",
        ];

        for pattern in &numeric_conversions {
            if expr.contains(pattern) {
                return true;
            }
        }

        // Check for arithmetic operations (indicates numeric context)
        // But only if there are no string variables involved
        let arithmetic_ops = ['+', '-', '*', '/', '%'];
        if expr.chars().any(|c| arithmetic_ops.contains(&c)) {
            return true;
        }

        false
    }

    /// Check if a variable is assigned a non-empty string literal
    pub fn is_assigned_string_literal(&self, var: &str) -> bool {
        if let Some(expr) = self.assignments.get(var) {
            let expr = expr.trim();
            // Check if the expression is a non-empty string literal
            // Pattern: "..." where ... is not empty
            if expr.starts_with('"') && expr.ends_with('"') && expr.len() > 2 {
                // Make sure it's not just an empty string ""
                let content = &expr[1..expr.len() - 1];
                if !content.is_empty() {
                    return true;
                }
            }
        }
        false
    }

    /// Check if a variable depends on (transitively) any variable that is assigned a non-empty string literal
    pub fn has_string_literal_in_dependency_chain(&self, var: &str) -> bool {
        let mut visited = std::collections::HashSet::new();
        self.check_string_literal_dependency_recursive(var, &mut visited)
    }

    fn check_string_literal_dependency_recursive(
        &self,
        var: &str,
        visited: &mut std::collections::HashSet<String>,
    ) -> bool {
        if !visited.insert(var.to_string()) {
            return false;
        }

        // Check if this variable is assigned a string literal
        if self.is_assigned_string_literal(var) {
            return true;
        }

        // Check transitive dependencies
        if let Some(deps) = self.dependencies.get(var) {
            for dep in deps {
                if self.check_string_literal_dependency_recursive(dep, visited) {
                    return true;
                }
            }
        }

        false
    }

}

mod builder;
