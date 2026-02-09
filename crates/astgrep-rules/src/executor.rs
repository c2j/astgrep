//! Advanced rule executor with pattern matching and data flow integration
//!
//! This module provides a high-level rule executor that integrates with the pattern
//! matching engine and data flow analyzer for comprehensive static analysis.

use crate::types::*;
use astgrep_core::{AstNode, Finding, Language, Location, Result, Severity, MetavariableAnalysis, ComparisonOperator, SemgrepPattern, SemgrepMatchResult};
use astgrep_matcher::{PatternMatcher, AdvancedSemgrepMatcher};
use astgrep_dataflow::{DataFlowAnalyzer, DataFlowAnalysis};
use serde_yaml::Value;
use std::collections::HashMap;
use std::path::Path;

/// Check if a node type/text is an operator
fn is_operator_node(node_type: &str, node_text: Option<&str>) -> bool {
    if matches!(node_type,
        "=" | "+=" | "-=" | "*=" | "/=" | "%=" | "++" | "--" |
        "+" | "-" | "*" | "/" | "%" |
        "==" | "!=" | "<" | ">" | "<=" | ">=" |
        "&&" | "||" | "!" | "&" | "|" | "^" | "~" |
        "<<" | ">>" | ">>>" |
        "assignment_operator" | "operator"
    ) {
        return true;
    }

    if let Some(text) = node_text {
        if text.len() <= 3 && matches!(text,
            "=" | "+=" | "-=" | "*=" | "/=" | "%=" |
            "+" | "-" | "*" | "/" | "%" |
            "==" | "!=" | "<" | ">" | "<=" | ">=" |
            "&&" | "||" | "!" | "&" | "|" | "^" | "~" |
            "<<" | ">>" | ">>>"
        ) {
            return true;
        }
    }

    false
}

/// Represents a taint match (source or sink)
struct TaintMatch {
    node: Box<dyn AstNode>,
    bindings: HashMap<String, String>,
    var_name: Option<String>,
    /// Method name containing this match (for scope isolation)
    method_name: Option<String>,
}

/// Variable dependency tracker for intra-procedural dataflow analysis
/// Tracks which variables depend on (are derived from) other variables
struct VariableDependencyGraph {
    /// Maps a variable to the set of variables it depends on
    dependencies: HashMap<String, Vec<String>>,
    /// Maps a variable to its assigned expression text
    assignments: HashMap<String, String>,
    /// Maps object fields to their tainted status (object_name.field_name -> tainted_by)
    field_taints: HashMap<String, Vec<String>>,
    /// Maps getter calls to their source fields (e.g., "e.getX()" -> "e.x")
    getter_to_field: HashMap<String, String>,
    /// Custom propagator rules
    propagators: Vec<crate::types::PropagatorPattern>,
}

impl VariableDependencyGraph {
    fn new() -> Self {
        Self {
            dependencies: HashMap::new(),
            assignments: HashMap::new(),
            field_taints: HashMap::new(),
            getter_to_field: HashMap::new(),
            propagators: Vec::new(),
        }
    }

    fn with_propagators(mut self, propagators: Vec<crate::types::PropagatorPattern>) -> Self {
        self.propagators = propagators;
        self
    }

    /// Record that `target` variable is assigned from `source_vars`
    fn record_assignment(&mut self, target: String, source_vars: Vec<String>, expr: String) {
        eprintln!("[DEBUG] Recording assignment: {} depends on {:?} (expr: {})", target, source_vars, expr);
        self.dependencies.insert(target.clone(), source_vars.clone());
        self.assignments.insert(target.clone(), expr);
        eprintln!("[DEBUG] Assignment recorded. Dependencies for '{}': {:?}", target, self.dependencies.get(&target));
    }

    /// Record that an object's field is tainted by a source
    fn record_field_taint(&mut self, object: &str, field: &str, source: &str) {
        let key = format!("{}.{}", object, field);
        let sources = self.field_taints.entry(key).or_insert_with(Vec::new);
        if !sources.contains(&source.to_string()) {
            sources.push(source.to_string());
        }
    }

    /// Map a getter call to its corresponding field
    fn record_getter_mapping(&mut self, getter_call: &str, object: &str, field: &str) {
        let field_key = format!("{}.{}", object, field);
        self.getter_to_field.insert(getter_call.to_string(), field_key);
    }

    /// Check if a getter call returns a tainted field
    fn is_getter_tainted(&self, getter_call: &str, source_vars: &[String]) -> bool {
        eprintln!("[DEBUG] is_getter_tainted: checking '{}', source_vars={:?}", getter_call, source_vars);
        
        // Check if this getter maps to a field
        if let Some(field_key) = self.getter_to_field.get(getter_call) {
            eprintln!("[DEBUG] Found getter mapping: {} -> {}", getter_call, field_key);
            // Check if the field itself is in source_vars (field-level source)
            if source_vars.contains(field_key) {
                eprintln!("[DEBUG] Match found! Field {} is in source_vars", field_key);
                return true;
            }
            // Also check if the field is tainted by any source
            if let Some(taint_sources) = self.field_taints.get(field_key) {
                eprintln!("[DEBUG] Field {} has taint sources: {:?}", field_key, taint_sources);
                for taint_source in taint_sources {
                    if source_vars.contains(taint_source) {
                        eprintln!("[DEBUG] Match found! {} is tainted by {}", field_key, taint_source);
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
    fn depends_on(&self, var: &str, source_vars: &[String], check_safe_context: bool) -> bool {
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
                if self.check_dependency_recursive(receiver, source_vars, visited, check_safe_context) {
                    eprintln!("[DEBUG] Receiver '{}' depends on source_vars, returning true", receiver);
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
                            eprintln!("[DEBUG] Variable '{}' assigned from safe numeric expression: {}", var, expr);
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
    fn is_safe_numeric_expression_advanced(&self, expr: &str, var_assignments: &HashMap<String, String>) -> bool {
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
                    eprintln!("[DEBUG] Variable '{}' is assigned a string value: {}", var, assign_expr);
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
    fn is_assigned_string_literal(
        &self, var: &str
    ) -> bool {
        if let Some(expr) = self.assignments.get(var) {
            let expr = expr.trim();
            // Check if the expression is a non-empty string literal
            // Pattern: "..." where ... is not empty
            if expr.starts_with('"') && expr.ends_with('"') && expr.len() > 2 {
                // Make sure it's not just an empty string ""
                let content = &expr[1..expr.len()-1];
                if !content.is_empty() {
                    return true;
                }
            }
        }
        false
    }

    /// Check if a variable depends on (transitively) any variable that is assigned a non-empty string literal
    fn has_string_literal_in_dependency_chain(&self, var: &str) -> bool {
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

    /// Build dependency graph from method body text
    fn build_from_method(&mut self, method_text: &str) {
        // Parse local variable declarations and assignments
        // Pattern: Type var = expression;
        // Pattern: var = expression;
        // Pattern: var = source();
        // Pattern: var = other + something;
        // Pattern: obj.setX(source);  // setter call
        // Pattern: var = obj.getX();  // getter call
        // Pattern: obj.x = source;    // direct field access

        for line in method_text.lines() {
            let line = line.trim();

            // Match local variable declaration: Type var = expr;
            // or assignment: var = expr;
            if let Some(eq_pos) = line.find('=') {
                let before_eq = line[..eq_pos].trim();
                let after_eq = line[eq_pos + 1..].trim().trim_end_matches(';').trim();

                // Extract variable name (last identifier before =)
                let parts: Vec<&str> = before_eq.split_whitespace().collect();
                if let Some(var_name_ref) = parts.last() {
                    let var_name = var_name_ref.trim().to_string();
                    if !var_name.is_empty() {
                        // Find all variables used in the right-hand side
                        let source_vars = self.extract_variables_from_expression(after_eq);
                        self.record_assignment(var_name.clone(), source_vars.clone(), after_eq.to_string());
                        
                        // Check if this is a getter call: var = obj.getX()
                        self.process_getter_call(&var_name, after_eq);
                    }
                }
                
                // Check for direct field access: obj.x = source
                self.process_field_assignment(before_eq, after_eq);
            }
            
            // Check for setter call: obj.setX(source)
            self.process_setter_call(line);
            
            // Also check for getter calls used as method arguments (not in assignment)
            // Pattern: sink(obj.getX()) or method(obj.getY())
            self.process_getter_in_arguments(line);
            
            // Apply custom propagator rules
            self.apply_propagators(line);
        }
    }
    
    /// Apply custom propagator rules to a line
    fn apply_propagators(&mut self, line: &str) {
        eprintln!("[DEBUG] apply_propagators called with line: '{}'", line);
        eprintln!("[DEBUG] Number of propagators: {}", self.propagators.len());
        
        // Collect propagations first to avoid borrow issues
        let mut propagations: Vec<(String, String)> = Vec::new();
        
        for (i, propagator) in self.propagators.iter().enumerate() {
            eprintln!("[DEBUG] Processing propagator {}", i);
            // Get pattern text
            let pattern_text = match &propagator.pattern.pattern_type {
                crate::types::PatternType::Simple(s) => s.as_str(),
                _ => continue,
            };
            
            // Check if line matches propagator pattern
            // Handle forEach pattern: $X.forEach(($Y) -> ...)
            if pattern_text.contains(".forEach") && pattern_text.contains("->") {
                if line.contains(".forEach") && line.contains("->") {
                    eprintln!("[DEBUG] Propagator forEach pattern matched in line: {}", line);
                    
                    // Extract $X (the collection/object before .forEach)
                    if let Some(for_each_pos) = line.find(".forEach") {
                        let before_for_each = &line[..for_each_pos];
                        let parts: Vec<&str> = before_for_each.split(|c: char| c == '(' || c == ',' || c == ' ').collect();
                        if let Some(collection) = parts.last() {
                            let collection = collection.trim();
                            
                            // Extract $Y (the lambda parameter inside parentheses)
                            if let Some(open_paren) = line[for_each_pos..].find('(') {
                                let after_open = &line[for_each_pos + open_paren + 1..];
                                // Look for pattern: (param) or param
                                let param_candidates: Vec<&str> = after_open.split(|c: char| c == '(' || c == ')' || c == ',' || c == '-').collect();
                                for candidate in param_candidates {
                                    let candidate = candidate.trim();
                                    // Valid parameter: non-empty, starts with letter, not a keyword
                                    if !candidate.is_empty() 
                                        && candidate.chars().next().map(|c| c.is_alphabetic()).unwrap_or(false)
                                        && candidate != "null" && candidate != "true" && candidate != "false" {
                                        eprintln!("[DEBUG] Propagator forEach: {} -> {}", collection, candidate);
                                        propagations.push((collection.to_string(), candidate.to_string()));
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            } else if (pattern_text.contains(".set") || pattern_text.contains("$SETTER")) && pattern_text.contains("(") {
                // Handle setter pattern: obj.setX(data) - propagate from data to obj
                // Pattern: (Type $OBJ).$SETTER($DATA) where $SETTER matches set.*
                // Also handle patterns like $PAGE.$SETTER($DATA) where SETTER is a metavariable
                eprintln!("[DEBUG] Checking setter pattern: '{}' on line: '{}'", pattern_text, line);
                
                // Extract the setter method name pattern (e.g., $SETTER or setOrderBy)
                // Handle both literal patterns (obj.setX) and metavariable patterns (obj.$SETTER)
                let setter_pattern_opt = if let Some(set_pos) = pattern_text.find(".set") {
                    let after_set = &pattern_text[set_pos + 1..]; // Skip the dot, keep "set..."
                    if let Some(paren_pos) = after_set.find('(') {
                        Some(&after_set[..paren_pos]) // e.g., "setX"
                    } else {
                        None
                    }
                } else if pattern_text.contains("$SETTER") {
                    // Pattern uses $SETTER metavariable, treat it as a wildcard setter pattern
                    Some("$SETTER")
                } else {
                    None
                };
                
                if let Some(setter_pattern) = setter_pattern_opt {
                    eprintln!("[DEBUG] Setter pattern extracted: {}", setter_pattern);
                    
                    // Check if line contains a setter call
                    if line.contains(".set") && line.contains('(') {
                        // Find the setter call in the line
                        if let Some(line_set_pos) = line.find(".set") {
                            let line_after_set = &line[line_set_pos + 1..];
                            if let Some(line_paren_pos) = line_after_set.find('(') {
                                let actual_setter = &line_after_set[..line_paren_pos];
                                eprintln!("[DEBUG] Found setter in line: {}", actual_setter);
                                
                                // Check if setter matches pattern (starts with "set")
                                if actual_setter.starts_with("set") {
                                    // Extract object name (before .set)
                                    let before_setter = &line[..line_set_pos];
                                    let obj_parts: Vec<&str> = before_setter.split(|c: char| c == ' ' || c == '(' || c == '.').collect();
                                    if let Some(obj_name) = obj_parts.last() {
                                        let obj_name = obj_name.trim();
                                        
                                        // Extract argument (inside parentheses)
                                        let after_paren = &line_after_set[line_paren_pos + 1..];
                                        if let Some(close_paren) = after_paren.find(')') {
                                            let arg = &after_paren[..close_paren].trim();
                                            
                                            eprintln!("[DEBUG] Setter propagator: {} -> {} (setter: {})", arg, obj_name, actual_setter);
                                            propagations.push((arg.to_string(), obj_name.to_string()));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            } else if pattern_text.contains(".append(") && pattern_text.contains("$") {
                // Handle append pattern: $BUILDER.append($STR) - propagate from STR to BUILDER
                eprintln!("[DEBUG] Checking append pattern: '{}' on line: '{}'", pattern_text, line);
                
                // Check if line contains .append(
                if line.contains(".append(") {
                    // Find the append call
                    if let Some(append_pos) = line.find(".append(") {
                        let before_append = &line[..append_pos];
                        // Extract builder name
                        let builder_parts: Vec<&str> = before_append.split(|c: char| c == ' ' || c == '(' || c == '.' || c == ',').collect();
                        if let Some(builder_name) = builder_parts.last() {
                            let builder_name = builder_name.trim();
                            
                            // Extract argument inside append(...)
                            // Handle nested parentheses: find the matching closing paren
                            let after_append = &line[append_pos + 8..]; // Skip ".append("
                            let mut paren_depth = 1;
                            let mut close_pos = 0;
                            for (i, c) in after_append.char_indices() {
                                match c {
                                    '(' => paren_depth += 1,
                                    ')' => {
                                        paren_depth -= 1;
                                        if paren_depth == 0 {
                                            close_pos = i;
                                            break;
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            
                            if paren_depth == 0 {
                                let arg = after_append[..close_pos].trim();
                                
                                eprintln!("[DEBUG] Append propagator: {} -> {}", arg, builder_name);
                                propagations.push((arg.to_string(), builder_name.to_string()));
                            }
                        }
                    }
                }
            } else if line.contains(pattern_text) {
                // For other patterns, use simple substring matching
                eprintln!("[DEBUG] Propagator pattern matched: {}", pattern_text);
                
                // Extract from and to metavariables
                let from_var = self.extract_metavariable(&propagator.from, line, pattern_text);
                let to_var = self.extract_metavariable(&propagator.to, line, pattern_text);
                
                if let (Some(from), Some(to)) = (from_var, to_var) {
                    eprintln!("[DEBUG] Propagator: {} -> {}", from, to);
                    propagations.push((from, to));
                }
            }
        }
        
        // Apply collected propagations
        for (from, to) in propagations {
            self.record_assignment(to.clone(), vec![from.clone()], format!("propagated from {} to {}", from, to));
        }
    }
    
    /// Extract metavariable value from a line based on pattern
    fn extract_metavariable(&self, metavar: &str, line: &str, pattern: &str) -> Option<String> {
        // Simple heuristic: if metavar starts with $, extract the identifier at that position
        if !metavar.starts_with('$') {
            return Some(metavar.to_string());
        }
        
        // For pattern like "$X.forEach" and line like "students.forEach"
        // Extract X = students
        let var_name = &metavar[1..]; // Remove $
        
        // Find pattern prefix before metavar
        if let Some(var_pos) = pattern.find(metavar) {
            let prefix = &pattern[..var_pos];
            if line.contains(prefix) {
                // Extract the identifier before the prefix in line
                if let Some(prefix_pos) = line.find(prefix) {
                    let before_prefix = &line[..prefix_pos].trim();
                    let parts: Vec<&str> = before_prefix.split(|c: char| c == '(' || c == ',' || c == ' ' || c == '.').collect();
                    if let Some(identifier) = parts.last() {
                        let identifier = identifier.trim();
                        if !identifier.is_empty() {
                            return Some(identifier.to_string());
                        }
                    }
                }
            }
        }
        
        // Try to extract any identifier that could match
        let parts: Vec<&str> = line.split(|c: char| c == '(' || c == ',' || c == ' ').collect();
        for part in parts {
            let part = part.trim().trim_end_matches('.').trim_end_matches(')');
            if !part.is_empty() && !part.starts_with('$') && part.chars().next().map(|c| c.is_alphabetic()).unwrap_or(false) {
                return Some(part.to_string());
            }
        }
        
        None
    }
    
    /// Process getter calls that are used as method arguments (not in assignments)
    fn process_getter_in_arguments(&mut self, line: &str) {
        // Pattern: methodName(..., obj.getX(), ...)
        // We need to find all getter calls and record their mappings
        let line = line.trim();
        
        // Find all occurrences of .getX() patterns
        let mut search_start = 0;
        while let Some(get_pos) = line[search_start..].find(".get") {
            let actual_pos = search_start + get_pos;
            
            // Check if this looks like a getter call
            if let Some(paren_pos) = line[actual_pos..].find('(') {
                let after_get = &line[actual_pos + 4..actual_pos + paren_pos];
                // Check if the next char is ')' (getter has no args) or followed by ')'
                if let Some(close_paren) = line[actual_pos + paren_pos..].find(')') {
                    let args = &line[actual_pos + paren_pos + 1..actual_pos + paren_pos + close_paren];
                    // Getter calls typically have no arguments
                    if args.trim().is_empty() || !args.contains(',') {
                        // Extract object name
                        let before_get = &line[..actual_pos];
                        let parts: Vec<&str> = before_get.split(|c: char| c == '(' || c == ',' || c == ' ').collect();
                        if let Some(obj_name) = parts.last() {
                            let obj_name = obj_name.trim();
                            if !obj_name.is_empty() && !obj_name.contains('"') {
                                let field_name = after_get.to_lowercase();
                                let getter_call = format!("{}.get{}()", obj_name, after_get);
                                
                                // Record getter mapping
                                self.record_getter_mapping(&getter_call, obj_name, &field_name);
                                eprintln!("[DEBUG] Recorded argument getter mapping: {} -> {}.{}", getter_call, obj_name, field_name);
                            }
                        }
                    }
                }
            }
            
            search_start = actual_pos + 1;
        }
    }

    /// Process setter call pattern: obj.setX(source)
    /// Records that obj's X field is tainted by source
    fn process_setter_call(&mut self, line: &str) {
        // Pattern: obj.setX(source) or obj.setX(source);
        if let Some(set_pos) = line.find(".set") {
            if let Some(paren_pos) = line[set_pos..].find('(') {
                let after_set = &line[set_pos + 4..set_pos + paren_pos];
                let method_name = format!("set{}", after_set);
                
                // Extract object name
                let before_set = &line[..set_pos];
                let parts: Vec<&str> = before_set.split_whitespace().collect();
                if let Some(obj_name) = parts.last() {
                    let obj_name = obj_name.trim();
                    
                    // Extract field name from setter (setX -> x)
                    if let Some(field_name) = method_name.strip_prefix("set") {
                        let field_name = field_name.to_lowercase();
                        
                        // Extract arguments inside parentheses
                        if let Some(open_paren) = line[set_pos..].find('(') {
                            let args_start = set_pos + open_paren + 1;
                            if let Some(close_paren) = line[args_start..].find(')') {
                                let args = &line[args_start..args_start + close_paren];
                                let source_vars = self.extract_variables_from_expression(args);
                                
                                // Record field taint for each source
                                for source in &source_vars {
                                    self.record_field_taint(obj_name, &field_name, source);
                                    eprintln!("[DEBUG] Recorded field taint: {}.{} tainted by {}", obj_name, field_name, source);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Process getter call pattern: var = obj.getX()
    /// Records mapping from getter call to field
    fn process_getter_call(&mut self, target_var: &str, expr: &str) {
        // Pattern: obj.getX()
        if let Some(get_pos) = expr.find(".get") {
            if let Some(paren_pos) = expr[get_pos..].find('(') {
                let after_get = &expr[get_pos + 4..get_pos + paren_pos];
                let method_name = format!("get{}", after_get);
                
                // Extract object name
                let before_get = &expr[..get_pos];
                let parts: Vec<&str> = before_get.split_whitespace().collect();
                if let Some(obj_name) = parts.last() {
                    let obj_name = obj_name.trim();
                    
                    // Extract field name from getter (getX -> x)
                    if let Some(field_name) = method_name.strip_prefix("get") {
                        let field_name = field_name.to_lowercase();
                        let getter_call = format!("{}.{}()", obj_name, method_name.to_lowercase());
                        
                        // Record getter mapping
                        self.record_getter_mapping(&getter_call, obj_name, &field_name);
                        eprintln!("[DEBUG] Recorded getter mapping: {} -> {}.{}", getter_call, obj_name, field_name);
                    }
                }
            }
        }
    }

    /// Process direct field assignment pattern: obj.x = source
    fn process_field_assignment(&mut self, before_eq: &str, after_eq: &str) {
        // Pattern: obj.field = value
        if let Some(dot_pos) = before_eq.rfind('.') {
            let obj_part = &before_eq[..dot_pos].trim();
            let field_part = &before_eq[dot_pos + 1..].trim();
            
            // Check if obj_part is a variable and field_part is a field name
            let obj_parts: Vec<&str> = obj_part.split_whitespace().collect();
            if let Some(obj_name) = obj_parts.last() {
                let obj_name = obj_name.trim();
                let field_name = field_part.trim();
                
                // Extract source variables from right-hand side
                let source_vars = self.extract_variables_from_expression(after_eq);
                
                // Record field taint
                for source in &source_vars {
                    self.record_field_taint(obj_name, field_name, source);
                    eprintln!("[DEBUG] Recorded direct field taint: {}.{} tainted by {}", obj_name, field_name, source);
                }
            }
        }
    }

    /// Extract variable names from an expression
    fn extract_variables_from_expression(&self, expr: &str) -> Vec<String> {
        let mut vars = Vec::new();
        let expr = expr.trim();

        // Skip string literals
        // Simple heuristic: split by operators and extract identifiers
        let operators = ['+', '-', '*', '/', '%', '(', ')', '.', ',', ';'];
        let tokens: Vec<&str> = expr.split(|c: char| operators.contains(&c)).collect();

        for token in tokens {
            let token = token.trim();
            // Skip empty, numeric literals, string literals, and keywords
            if token.is_empty()
                || token.parse::<f64>().is_ok()
                || token.starts_with('"')
                || token.starts_with('\'')
                || token == "source"
                || token == "sink"
                || token == "sink1"
                || token == "sink2"
                || token == "new"
                || token == "null"
                || token == "true"
                || token == "false"
            {
                continue;
            }

            // Check if it looks like an identifier (starts with letter or underscore)
            if let Some(first_char) = token.chars().next() {
                if first_char.is_alphabetic() || first_char == '_' {
                    vars.push(token.to_string());
                }
            }
        }

        vars
    }
}

impl Clone for TaintMatch {
    fn clone(&self) -> Self {
        Self {
            node: self.node.clone_node(),
            bindings: self.bindings.clone(),
            var_name: self.var_name.clone(),
            method_name: self.method_name.clone(),
        }
    }
}

/// Advanced rule executor with full integration
pub struct AdvancedRuleExecutor {
    pattern_matcher: AdvancedSemgrepMatcher,
    dataflow_analyzer: DataFlowAnalyzer,
    execution_stats: ExecutionStatistics,
    /// Constant propagator for variable value tracking
    constant_propagator: Option<astgrep_dataflow::ConstantPropagator>,
    /// Symbolic propagator for alias tracking
    symbolic_propagator: Option<astgrep_dataflow::SymbolicPropagator>,
}

impl AdvancedRuleExecutor {
    /// Create a new advanced rule executor
    pub fn new() -> Self {
        Self {
            pattern_matcher: AdvancedSemgrepMatcher::new(),
            dataflow_analyzer: DataFlowAnalyzer::new(),
            execution_stats: ExecutionStatistics::new(),
            constant_propagator: None,
            symbolic_propagator: None,
        }
    }

    /// Execute rules with full analysis
    pub fn execute_comprehensive_analysis(
        &mut self,
        rules: &[Rule],
        ast: &dyn AstNode,
        language: Language,
        file_path: Option<&Path>,
        enable_constant_propagation: bool,
    ) -> Result<ComprehensiveAnalysisResult> {
        let start_time = std::time::Instant::now();
        
        // Filter applicable rules
        let applicable_rules: Vec<&Rule> = rules.iter()
            .filter(|rule| rule.applies_to(language))
            .collect();

        if applicable_rules.is_empty() {
            return Ok(ComprehensiveAnalysisResult::empty(start_time.elapsed()));
        }

        // Perform constant propagation analysis if enabled
        self.constant_propagator = if enable_constant_propagation {
            use astgrep_dataflow::ConstantPropagator;
            let mut propagator = ConstantPropagator::new();
            match propagator.analyze_ast(ast) {
                Ok(values) => {
                    if !values.is_empty() {
                        tracing::info!("Constant propagation found {} constants", values.len());
                    }
                    // Set constant values in the pattern matcher
                    eprintln!("DEBUG: Setting {} constant values in pattern matcher", values.len());
                    for (k, v) in &values {
                        eprintln!("DEBUG: Constant {} = {:?}", k, v);
                    }
                    self.pattern_matcher.set_constant_values(values);
                    Some(propagator)
                }
                Err(e) => {
                    tracing::warn!("Constant propagation analysis failed: {}", e);
                    None
                }
            }
        } else {
            None
        };

        // Perform symbolic propagation analysis if needed
        let enable_symbolic_propagation = applicable_rules.iter().any(|r| r.requires_symbolic_propagation());
        if enable_symbolic_propagation {
            use astgrep_dataflow::SymbolicPropagator;
            let mut propagator = SymbolicPropagator::new().with_deep_propagation(true);
            match propagator.analyze(ast) {
                Ok(()) => {
                    eprintln!("DEBUG: Symbolic propagation analysis completed");
                    // Set the symbolic propagator in the pattern matcher
                    self.pattern_matcher.set_symbolic_propagator(propagator.clone());
                    self.symbolic_propagator = Some(propagator);
                }
                Err(e) => {
                    eprintln!("DEBUG: Symbolic propagation analysis failed: {}", e);
                    self.symbolic_propagator = None;
                }
            }
        } else {
            self.symbolic_propagator = None;
        }

        // Perform data flow analysis if needed
        let dataflow_analysis = if applicable_rules.iter().any(|r| r.requires_dataflow()) {
            Some(self.dataflow_analyzer.analyze(ast)?)
        } else {
            None
        };

        let mut all_findings = Vec::new();
        let mut rule_results = Vec::new();

        // Execute each rule
        for rule in applicable_rules {
            let rule_start = std::time::Instant::now();
            
            match self.execute_single_rule(rule, ast, dataflow_analysis.as_ref(), file_path) {
                Ok(findings) => {
                    let execution_time = rule_start.elapsed();
                    self.execution_stats.record_rule_execution(&rule.id, execution_time, findings.len());
                    
                    all_findings.extend(findings.clone());
                    rule_results.push(RuleExecutionResult {
                        rule_id: rule.id.clone(),
                        findings,
                        execution_time,
                        success: true,
                        error: None,
                    });
                }
                Err(e) => {
                    let execution_time = rule_start.elapsed();
                    self.execution_stats.record_rule_error(&rule.id, execution_time);
                    
                    rule_results.push(RuleExecutionResult {
                        rule_id: rule.id.clone(),
                        findings: Vec::new(),
                        execution_time,
                        success: false,
                        error: Some(e.to_string()),
                    });
                }
            }
        }

        Ok(ComprehensiveAnalysisResult {
            findings: all_findings,
            rule_results,
            dataflow_analysis,
            execution_time: start_time.elapsed(),
            statistics: self.execution_stats.clone(),
        })
    }

    /// Execute a single rule with full context
    fn execute_single_rule(
        &mut self,
        rule: &Rule,
        ast: &dyn AstNode,
        dataflow_analysis: Option<&DataFlowAnalysis>,
        file_path: Option<&Path>,
    ) -> Result<Vec<Finding>> {
        let mut findings = Vec::new();

        // For taint mode, use special handling
        if rule.mode == crate::types::RuleMode::Taint {
            if let Some(ref dataflow_spec) = rule.dataflow {
                let taint_findings = self.execute_taint_analysis(rule, dataflow_spec, ast, dataflow_analysis, file_path)?;
                findings.extend(taint_findings);
            }
            return Ok(findings);
        }

        // Execute pattern-based analysis
        for pattern in &rule.patterns {
            let pattern_findings = self.execute_pattern_analysis(rule, pattern, ast, dataflow_analysis, file_path)?;
            findings.extend(pattern_findings);
        }

        // Execute data flow analysis if specified
        if let Some(ref dataflow_spec) = rule.dataflow {
            if let Some(analysis) = dataflow_analysis {
                let dataflow_findings = self.execute_dataflow_analysis(rule, dataflow_spec, analysis, file_path)?;
                findings.extend(dataflow_findings);
            }
        }

        Ok(findings)
    }

    /// Execute pattern-based analysis
    fn execute_pattern_analysis(
        &mut self,
        rule: &Rule,
        pattern: &Pattern,
        ast: &dyn AstNode,
        dataflow_analysis: Option<&DataFlowAnalysis>,
        file_path: Option<&Path>,
    ) -> Result<Vec<Finding>> {
        let mut findings = Vec::new();

        // Preprocess pattern to handle typed metavariable syntax like "($TYPE $VAR).method()"
        let (processed_pattern, type_constraints) = self.preprocess_typed_metavariables(pattern);

        // Convert astgrep_rules::Pattern to astgrep_core::SemgrepPattern
        let semgrep_pattern = self.convert_pattern_to_semgrep_pattern(&processed_pattern)?;

        // Find pattern matches using the advanced matcher
        let matches = self.pattern_matcher.find_matches(&semgrep_pattern, ast)?;

        // Check if pattern contains ellipsis (indicating potential cross-statement matches)
        let pattern_str = match &processed_pattern.pattern_type {
            PatternType::Simple(s) => s.as_str(),
            _ => "",
        };
        let has_ellipsis = pattern_str.contains("...");

        // If no matches found and we have either:
        // 1. Type constraints with symbolic propagation, or
        // 2. Pattern contains ellipsis and symbolic propagation is enabled
        // try to find matches using symbolic propagation
        let matches = if matches.is_empty() && self.symbolic_propagator.is_some()
            && (!type_constraints.is_empty() || has_ellipsis) {
            eprintln!("DEBUG: No direct matches found, attempting symbolic propagation matching");
            self.find_matches_via_symbolic_propagation(&semgrep_pattern, ast, &type_constraints)?
        } else {
            matches
        };

        // Heuristic de-dup: keep only smallest, non-overlapping spans to avoid repeated matches
        let mut mm: Vec<((usize, usize), usize, usize, usize, usize, SemgrepMatchResult)> = matches
            .into_iter()
            .map(|m| {
                if let Some((sl, sc, el, ec)) = m.node.location() {
                    let dl = el.saturating_sub(sl);
                    let dc = ec.saturating_sub(sc);
                    ((dl, dc), sl, sc, el, ec, m)
                } else {
                    ((usize::MAX, usize::MAX), 0, 0, usize::MAX, usize::MAX, m)
                }
            })
            .collect();
        mm.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| (a.1, a.2, a.3, a.4).cmp(&(b.1, b.2, b.3, b.4))));

        let overlaps = |a: (usize, usize, usize, usize), b: (usize, usize, usize, usize)| -> bool {
            let (a_sl, a_sc, a_el, a_ec) = a;
            let (b_sl, b_sc, b_el, b_ec) = b;
            // Simple line-based overlap, with basic column checks when on same line
            if a_el < b_sl || b_el < a_sl { return false; }
            if a_sl == b_el && a_sc >= b_ec { return false; }
            if b_sl == a_el && b_sc >= a_ec { return false; }
            true
        };

        let mut selected_spans: Vec<(usize, usize, usize, usize)> = Vec::new();
        let mut filtered: Vec<SemgrepMatchResult> = Vec::new();
        'outer: for (_, sl, sc, el, ec, m) in mm {
            for s in &selected_spans {
                if overlaps((sl, sc, el, ec), *s) {
                    continue 'outer;
                }
            }
            selected_spans.push((sl, sc, el, ec));
            filtered.push(m);
        }

        // Get full source code from the root AST node
        let full_source = ast.text().unwrap_or("").to_string();

        for match_result in filtered {
            // Check pattern conditions with full source code
            // Also check type constraints from typed metavariable syntax
            let conditions_passed = self.check_pattern_conditions(&processed_pattern, &match_result, dataflow_analysis, &full_source)?;
            
            // Check additional type constraints from typed metavariable preprocessing
            let mut final_conditions_passed = conditions_passed;
            if conditions_passed {
                for (var_name, expected_type) in &type_constraints {
                    // Check if the variable's type matches the expected type
                    if let Some(var_value) = match_result.bindings.get(var_name) {
                        let type_check_passed = self.check_variable_type(var_value, expected_type, &full_source);
                        if !type_check_passed {
                            final_conditions_passed = false;
                            break;
                        }
                    }
                }
            }
            
            if final_conditions_passed {
                let finding = self.create_finding_from_match(rule, pattern, &match_result, file_path)?;
                findings.push(finding);
            }
        }

        Ok(findings)
    }

    /// Execute data flow analysis
    fn execute_dataflow_analysis(
        &self,
        rule: &Rule,
        dataflow_spec: &DataFlowSpec,
        analysis: &DataFlowAnalysis,
        file_path: Option<&Path>,
    ) -> Result<Vec<Finding>> {
        let mut findings = Vec::new();

        // Check for vulnerable taint flows
        for flow in &analysis.taint_flows {
            if flow.is_vulnerable() {
                // Check if flow matches the specification
                if self.matches_dataflow_spec(flow, dataflow_spec) {
                    let finding = self.create_dataflow_finding(rule, flow, file_path)?;
                    findings.push(finding);
                }
            }
        }

        Ok(findings)
    }

    /// Check if pattern conditions are satisfied
    fn check_pattern_conditions(
        &self,
        pattern: &Pattern,
        match_result: &SemgrepMatchResult,
        dataflow_analysis: Option<&DataFlowAnalysis>,
        full_source: &str,
    ) -> Result<bool> {
        for condition in &pattern.conditions {
            if !self.evaluate_condition(condition, match_result, dataflow_analysis, full_source)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Evaluate a single condition
    fn evaluate_condition(
        &self,
        condition: &Condition,
        match_result: &SemgrepMatchResult,
        _dataflow_analysis: Option<&DataFlowAnalysis>,
        full_source: &str,
    ) -> Result<bool> {
        match condition {
            Condition::MetavariableRegex(metavar_regex) => {
                // Check if metavariable exists and matches regex
                if let Some(metavar_value) = match_result.bindings.get(&metavar_regex.metavariable) {
                    // Support (?i) case-insensitive flag and other inline regex flags
                    let regex_str = &metavar_regex.regex;
                    let regex = if regex_str.starts_with("(?i)") {
                        // Case-insensitive regex
                        regex::Regex::new(&format!("(?i){}", &regex_str[4..]))
                    } else {
                        regex::Regex::new(regex_str)
                    };
                    
                    if let Ok(re) = regex {
                        Ok(re.is_match(metavar_value))
                    } else {
                        Ok(false)
                    }
                } else {
                    Ok(false)
                }
            }
            Condition::MetavariableComparison(metavar_comp) => {
                // Check if metavariable exists and satisfies comparison
                eprintln!("DEBUG evaluate_condition: MetavariableComparison for '{}', bindings: {:?}", 
                         metavar_comp.metavariable, match_result.bindings.keys().collect::<Vec<_>>());
                if let Some(metavar_value) = match_result.bindings.get(&metavar_comp.metavariable) {
                    eprintln!("DEBUG: Found value '{}' for metavariable '{}'", metavar_value, metavar_comp.metavariable);
                    
                    // Try to resolve the variable to its constant value using constant propagation
                    let resolved_value = if let Some(ref propagator) = self.constant_propagator {
                        // Get the location of the matched node
                        if let Some((start_line, start_col, _, _)) = match_result.node.location() {
                            use astgrep_dataflow::constant_propagation::SourceLocation;
                            let location = SourceLocation::new(start_line, start_col);
                            
                            // Try to get the constant value at this location
                            if let Some(constant) = propagator.get_variable_value_at_location(metavar_value, location) {
                                let constant_str = constant.to_string_value().unwrap_or_else(|| metavar_value.clone());
                                eprintln!("DEBUG: Resolved variable '{}' to constant '{}' at {:?}", metavar_value, constant_str, location);
                                constant_str
                            } else {
                                metavar_value.clone()
                            }
                        } else {
                            metavar_value.clone()
                        }
                    } else {
                        metavar_value.clone()
                    };
                    
                    self.evaluate_comparison(&resolved_value, &metavar_comp.operator, &metavar_comp.value)
                } else {
                    eprintln!("DEBUG: Metavariable '{}' not found in bindings", metavar_comp.metavariable);
                    Ok(false)
                }
            }
            Condition::NodeType(expected_type) => {
                // Check if the matched node has the expected type
                Ok(match_result.node.node_type() == *expected_type)
            }
            Condition::NodeAttribute(attr_name, attr_value) => {
                // Check node attribute (simplified implementation)
                // In a real implementation, this would check actual node attributes
                Ok(match_result.node.text().unwrap_or("").contains(attr_value))
            }
            Condition::MetavariableName(metavar_name) => {
                // Evaluate metavariable name constraint
                if let Some(metavar_value) = match_result.bindings.get(&metavar_name.metavariable) {
                    self.evaluate_name_constraint(metavar_value, &metavar_name.name_pattern)
                } else {
                    Ok(false)
                }
            }
            Condition::MetavariableAnalysis(metavar_analysis) => {
                // Evaluate metavariable analysis constraint
                if let Some(metavar_value) = match_result.bindings.get(&metavar_analysis.metavariable) {
                    self.evaluate_analysis_constraint(metavar_value, &metavar_analysis.analysis)
                } else {
                    Ok(false)
                }
            }
            Condition::MetavariableType(metavar_type) => {
                // Check if metavariable exists and matches the expected type
                if let Some(metavar_value) = match_result.bindings.get(&metavar_type.metavariable) {
                    // Extract the actual value from the metavariable binding
                    let var_value = metavar_value.trim();
                    
                    // First, try to infer type from the value itself (for literals)
                    let inferred_type = self.infer_type_from_value(var_value);
                    
                    if let Some(type_info) = inferred_type {
                        // Value is a literal, compare its inferred type
                        Ok(type_info == metavar_type.var_type)
                    } else {
                        // Value is a variable, extract type from source code
                        if let Some(type_info) = self.extract_type_info(match_result, var_value, full_source) {
                            Ok(type_info == metavar_type.var_type)
                        } else {
                            // If we can't determine the type, reject the match
                            // This prevents false positives when type info is unavailable
                            Ok(false)
                        }
                    }
                } else {
                    Ok(false)
                }
            }
            Condition::Custom(custom_condition) => {
                // Custom condition evaluation
                self.evaluate_custom_condition(custom_condition, match_result)
            }
        }
    }

    /// Extract type information for a variable from the match context
    fn extract_type_info(&self, match_result: &SemgrepMatchResult, var_name: &str, full_source: &str) -> Option<String> {
        // Try to extract type information from the full source code
        // This looks for variable declarations like "TypeName varName" in method signatures or declarations
        
        // Build import map for name resolution
        let import_map = self.build_import_map(full_source);
        
        // Pattern 1: Method parameter declarations like "String varName" or "Type varName"
        // Matches: "Type varName" followed by comma, closing paren, or space
        let param_pattern = format!(r"(\w+)\s+{}\s*[,)]", regex::escape(var_name));
        if let Ok(regex) = regex::Regex::new(&param_pattern) {
            if let Some(captures) = regex.captures(full_source) {
                if let Some(type_match) = captures.get(1) {
                    let simple_type = type_match.as_str().to_string();
                    return self.resolve_type_with_imports(&simple_type, &import_map);
                }
            }
        }
        
        // Pattern 2: Variable declarations like "Type varName = ...;" or "Type varName;"
        // Handles cases like: PrintWriter pWriter = response.getWriter();
        let var_pattern = format!(r"(\w+)\s+{}\s*=[^;]*;", regex::escape(var_name));
        if let Ok(regex) = regex::Regex::new(&var_pattern) {
            if let Some(captures) = regex.captures(full_source) {
                if let Some(type_match) = captures.get(1) {
                    let simple_type = type_match.as_str().to_string();
                    return self.resolve_type_with_imports(&simple_type, &import_map);
                }
            }
        }
        
        // Pattern 3: Field declarations like "private Type varName = ...;" or "private Type varName;"
        // Handles cases like: private PrintWriter pWriter = response.getWriter();
        let field_pattern = format!(r"(?:public|private|protected)?\s*(?:static\s+)?(?:final\s+)?(\w+)\s+{}\s*=[^;]*;", regex::escape(var_name));
        if let Ok(regex) = regex::Regex::new(&field_pattern) {
            if let Some(captures) = regex.captures(full_source) {
                if let Some(type_match) = captures.get(1) {
                    let simple_type = type_match.as_str().to_string();
                    return self.resolve_type_with_imports(&simple_type, &import_map);
                }
            }
        }
        
        None
    }
    
    /// Build a map of imported simple names to their fully qualified names
    fn build_import_map(&self, full_source: &str) -> HashMap<String, String> {
        let mut import_map = HashMap::new();
        
        // Parse import statements like "import org.foo.Foo;" or "import org.foo.*;"
        let import_pattern = regex::Regex::new(r"import\s+([\w.]+)(?:\.\*)?;").unwrap();
        
        for captures in import_pattern.captures_iter(full_source) {
            if let Some(import_match) = captures.get(1) {
                let import_path = import_match.as_str();
                
                // Extract the simple name (last part after the last dot)
                if let Some(last_dot) = import_path.rfind('.') {
                    let simple_name = &import_path[last_dot + 1..];
                    let fully_qualified = import_path.to_string();
                    
                    eprintln!("DEBUG: Found import: {} -> {}", simple_name, fully_qualified);
                    import_map.insert(simple_name.to_string(), fully_qualified);
                } else {
                    // No dot, the whole import is the simple name
                    import_map.insert(import_path.to_string(), import_path.to_string());
                }
            }
        }
        
        import_map
    }
    
    /// Resolve a simple type name to its fully qualified name using import map
    fn resolve_type_with_imports(&self, simple_type: &str, import_map: &HashMap<String, String>) -> Option<String> {
        // First check if this simple type is in the import map
        if let Some(fully_qualified) = import_map.get(simple_type) {
            eprintln!("DEBUG: Resolved type '{}' to '{}'", simple_type, fully_qualified);
            return Some(fully_qualified.clone());
        }
        
        // If not found in imports, return the simple type as-is
        // (it might be a primitive type or in the same package)
        Some(simple_type.to_string())
    }

    /// Infer the type of a value from its literal representation
    fn infer_type_from_value(&self, value: &str) -> Option<String> {
        let trimmed = value.trim();
        
        // String literal: "..." or '...'
        if (trimmed.starts_with('"') && trimmed.ends_with('"')) ||
           (trimmed.starts_with('\'') && trimmed.ends_with('\'')) {
            return Some("String".to_string());
        }
        
        // Integer literal: digits only (possibly with negative sign)
        if trimmed.parse::<i64>().is_ok() {
            return Some("int".to_string());
        }
        
        // Floating point literal: contains dot and parses as float
        if trimmed.contains('.') && trimmed.parse::<f64>().is_ok() {
            return Some("float".to_string());
        }
        
        // Boolean literal: true or false
        if trimmed == "true" || trimmed == "false" {
            return Some("boolean".to_string());
        }
        
        // Null literal
        if trimmed == "null" {
            return Some("null".to_string());
        }
        
        // Not a recognized literal, probably a variable name
        None
    }

    /// Evaluate comparison between metavariable value and expected value
    fn evaluate_comparison(&self, metavar_value: &str, operator: &ComparisonOperator, expected_value: &str) -> Result<bool> {
        match operator {
            ComparisonOperator::Equals => Ok(metavar_value == expected_value),
            ComparisonOperator::NotEquals => Ok(metavar_value != expected_value),
            ComparisonOperator::Contains => Ok(metavar_value.contains(expected_value)),
            ComparisonOperator::StartsWith => Ok(metavar_value.starts_with(expected_value)),
            ComparisonOperator::EndsWith => Ok(metavar_value.ends_with(expected_value)),
            ComparisonOperator::Matches => {
                if let Ok(regex) = regex::Regex::new(expected_value) {
                    Ok(regex.is_match(metavar_value))
                } else {
                    Ok(false)
                }
            }
            ComparisonOperator::GreaterThan => {
                if let (Ok(mv), Ok(ev)) = (metavar_value.parse::<f64>(), expected_value.parse::<f64>()) {
                    Ok(mv > ev)
                } else {
                    Ok(metavar_value > expected_value)
                }
            }
            ComparisonOperator::LessThan => {
                if let (Ok(mv), Ok(ev)) = (metavar_value.parse::<f64>(), expected_value.parse::<f64>()) {
                    Ok(mv < ev)
                } else {
                    Ok(metavar_value < expected_value)
                }
            }
            ComparisonOperator::PythonExpression(expr) => {
                // For now, we'll implement a simplified version
                // In a full implementation, this would use a Python interpreter
                self.evaluate_python_expression(metavar_value, expr)
            }
        }
    }

    /// Evaluate name constraint (module/namespace patterns)
    fn evaluate_name_constraint(&self, value: &str, name_pattern: &str) -> Result<bool> {
        // Support glob-like patterns for module/namespace matching
        if name_pattern.contains("*") {
            // Convert glob pattern to regex
            let regex_pattern = name_pattern
                .replace(".", "\\.")
                .replace("*", ".*");
            if let Ok(regex) = regex::Regex::new(&regex_pattern) {
                Ok(regex.is_match(value))
            } else {
                Ok(false)
            }
        } else {
            // Exact match
            Ok(value == name_pattern)
        }
    }

    /// Evaluate analysis constraint (entropy, type, complexity)
    fn evaluate_analysis_constraint(&self, value: &str, analysis: &MetavariableAnalysis) -> Result<bool> {
        // Check entropy if specified
        if let Some(entropy_config) = &analysis.entropy {
            if !self.check_entropy(value, entropy_config)? {
                return Ok(false);
            }
        }

        // Check type analysis if specified
        if let Some(type_config) = &analysis.type_analysis {
            if !self.check_type_analysis(value, type_config)? {
                return Ok(false);
            }
        }

        // Check complexity if specified
        if let Some(complexity_config) = &analysis.complexity {
            if !self.check_complexity(value, complexity_config)? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Check entropy constraints
    fn check_entropy(&self, value: &str, entropy_config: &astgrep_core::EntropyAnalysis) -> Result<bool> {
        let entropy = self.calculate_entropy(value);

        if entropy < entropy_config.min_entropy {
            return Ok(false);
        }

        if let Some(max_entropy) = entropy_config.max_entropy {
            if entropy > max_entropy {
                return Ok(false);
            }
        }

        // Check charset if specified
        if let Some(charset) = &entropy_config.charset {
            if !self.matches_charset(value, charset) {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Check type analysis constraints
    fn check_type_analysis(&self, value: &str, type_config: &astgrep_core::TypeAnalysis) -> Result<bool> {
        // This is a simplified implementation
        // In a real implementation, you would analyze the AST to determine types

        // For now, we'll do basic pattern matching
        if !type_config.expected_types.is_empty() {
            let mut matches_expected = false;
            for expected_type in &type_config.expected_types {
                if self.value_matches_type(value, expected_type) {
                    matches_expected = true;
                    break;
                }
            }
            if !matches_expected {
                return Ok(false);
            }
        }

        // Check forbidden types
        for forbidden_type in &type_config.forbidden_types {
            if self.value_matches_type(value, forbidden_type) {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Check complexity constraints
    fn check_complexity(&self, value: &str, complexity_config: &astgrep_core::ComplexityAnalysis) -> Result<bool> {
        // This is a simplified implementation
        // In a real implementation, you would analyze the AST for complexity metrics

        if let Some(max_lines) = complexity_config.max_lines {
            let line_count = value.lines().count() as u32;
            if line_count > max_lines {
                return Ok(false);
            }
        }

        // For cyclomatic complexity and nesting depth, we'd need proper AST analysis
        // For now, we'll just return true
        Ok(true)
    }

    /// Calculate Shannon entropy of a string
    fn calculate_entropy(&self, s: &str) -> f64 {
        use std::collections::HashMap;

        if s.is_empty() {
            return 0.0;
        }

        let mut char_counts = HashMap::new();
        for c in s.chars() {
            *char_counts.entry(c).or_insert(0) += 1;
        }

        let len = s.len() as f64;
        let mut entropy = 0.0;

        for count in char_counts.values() {
            let p = *count as f64 / len;
            entropy -= p * p.log2();
        }

        entropy
    }

    /// Check if value matches charset
    fn matches_charset(&self, value: &str, charset: &str) -> bool {
        match charset {
            "alphanumeric" => value.chars().all(|c| c.is_alphanumeric()),
            "alphabetic" => value.chars().all(|c| c.is_alphabetic()),
            "numeric" => value.chars().all(|c| c.is_numeric()),
            "ascii" => value.is_ascii(),
            _ => true, // Unknown charset, assume match
        }
    }

    /// Check if value matches a type pattern
    fn value_matches_type(&self, value: &str, type_name: &str) -> bool {
        match type_name {
            "string" => true, // All values are strings at this level
            "number" => value.parse::<f64>().is_ok(),
            "integer" => value.parse::<i64>().is_ok(),
            "boolean" => value == "true" || value == "false",
            "null" => value == "null" || value == "None" || value == "nil",
            _ => false, // Unknown type
        }
    }

    /// Simplified Python expression evaluation
    fn evaluate_python_expression(&self, value: &str, expr: &str) -> Result<bool> {
        // This is a simplified implementation
        // In a full implementation, you would use a Python interpreter
        
        eprintln!("DEBUG evaluate_python_expression: value='{}', expr='{}'", value, expr);

        // Handle some common patterns
        if expr.contains("len(") {
            if let Some(len_expr) = expr.strip_prefix("len(").and_then(|s| s.strip_suffix(")")) {
                if len_expr.trim() == "$VAR" {
                    // Extract the comparison from the full expression
                    // This is very simplified - a real implementation would parse the full expression
                    return Ok(value.len() > 0);
                }
            }
        }

        // Handle bit operations like "$X & 1 == 1"
        // Parse expressions like "$VAR & 1 == 1" or "$VAR & 1 == 0"
        if expr.contains('&') && expr.contains("==") {
            // Try to parse the expression
            // Format: $VAR & N == M
            let parts: Vec<&str> = expr.split("==").collect();
            if parts.len() == 2 {
                let left_side = parts[0].trim();
                let expected_result = parts[1].trim();
                
                // Parse the bit operation: $VAR & N
                if left_side.contains('&') {
                    let bit_parts: Vec<&str> = left_side.split('&').collect();
                    if bit_parts.len() == 2 {
                        let var_part = bit_parts[0].trim();
                        let mask_part = bit_parts[1].trim();
                        
                        eprintln!("DEBUG: var_part='{}', mask_part='{}', expected='{}'", var_part, mask_part, expected_result);
                        
                        // Check if this is the metavariable we're evaluating
                        if var_part.starts_with("$") {
                            // Parse the mask value
                            if let Ok(mask) = mask_part.parse::<i64>() {
                                // Parse the expected result
                                if let Ok(expected) = expected_result.parse::<i64>() {
                                    // Parse the actual value
                                    if let Ok(val) = value.parse::<i64>() {
                                        let result = val & mask;
                                        eprintln!("DEBUG: val={}, mask={}, result={}, expected={}", val, mask, result, expected);
                                        return Ok(result == expected);
                                    } else {
                                        eprintln!("DEBUG: Failed to parse value '{}' as i64", value);
                                    }
                                } else {
                                    eprintln!("DEBUG: Failed to parse expected '{}' as i64", expected_result);
                                }
                            } else {
                                eprintln!("DEBUG: Failed to parse mask '{}' as i64", mask_part);
                            }
                        } else {
                            eprintln!("DEBUG: var_part '{}' doesn't start with $", var_part);
                        }
                    } else {
                        eprintln!("DEBUG: bit_parts.len() = {}, expected 2", bit_parts.len());
                    }
                } else {
                    eprintln!("DEBUG: left_side '{}' doesn't contain &", left_side);
                }
            } else {
                eprintln!("DEBUG: parts.len() = {}, expected 2", parts.len());
            }
        }

        // Handle bit OR operations like "$X | 1 == 1"
        // Parse expressions like "$VAR | 1 == 1" or "$VAR | 1 == 3"
        if expr.contains('|') && expr.contains("==") && !expr.contains("||") {
            // Try to parse the expression
            // Format: $VAR | N == M
            let parts: Vec<&str> = expr.split("==").collect();
            if parts.len() == 2 {
                let left_side = parts[0].trim();
                let expected_result = parts[1].trim();

                // Parse the bit operation: $VAR | N
                if left_side.contains('|') {
                    let bit_parts: Vec<&str> = left_side.split('|').collect();
                    if bit_parts.len() == 2 {
                        let var_part = bit_parts[0].trim();
                        let mask_part = bit_parts[1].trim();

                        eprintln!("DEBUG bitor: var_part='{}', mask_part='{}', expected='{}'", var_part, mask_part, expected_result);

                        // Check if this is the metavariable we're evaluating
                        if var_part.starts_with("$") {
                            // Parse the mask value
                            if let Ok(mask) = mask_part.parse::<i64>() {
                                // Parse the expected result
                                if let Ok(expected) = expected_result.parse::<i64>() {
                                    // Parse the actual value
                                    if let Ok(val) = value.parse::<i64>() {
                                        let result = val | mask;
                                        eprintln!("DEBUG bitor: val={}, mask={}, result={}, expected={}", val, mask, result, expected);
                                        return Ok(result == expected);
                                    } else {
                                        eprintln!("DEBUG: Failed to parse value '{}' as i64", value);
                                    }
                                } else {
                                    eprintln!("DEBUG: Failed to parse expected '{}' as i64", expected_result);
                                }
                            } else {
                                eprintln!("DEBUG: Failed to parse mask '{}' as i64", mask_part);
                            }
                        } else {
                            eprintln!("DEBUG: var_part '{}' doesn't start with $", var_part);
                        }
                    } else {
                        eprintln!("DEBUG: bit_parts.len() = {}, expected 2", bit_parts.len());
                    }
                } else {
                    eprintln!("DEBUG: left_side '{}' doesn't contain |", left_side);
                }
            } else {
                eprintln!("DEBUG: parts.len() = {}, expected 2", parts.len());
            }
        }

        // Handle bit XOR operations like "$X ^ 1 == 3"
        // Parse expressions like "$VAR ^ 1 == 3" or "$VAR ^ 1 == 2"
        if expr.contains('^') && expr.contains("==") {
            // Try to parse the expression
            // Format: $VAR ^ N == M
            let parts: Vec<&str> = expr.split("==").collect();
            if parts.len() == 2 {
                let left_side = parts[0].trim();
                let expected_result = parts[1].trim();

                // Parse the bit operation: $VAR ^ N
                if left_side.contains('^') {
                    let bit_parts: Vec<&str> = left_side.split('^').collect();
                    if bit_parts.len() == 2 {
                        let var_part = bit_parts[0].trim();
                        let mask_part = bit_parts[1].trim();

                        eprintln!("DEBUG bitxor: var_part='{}', mask_part='{}', expected='{}'", var_part, mask_part, expected_result);

                        // Check if this is the metavariable we're evaluating
                        if var_part.starts_with("$") {
                            // Parse the mask value
                            if let Ok(mask) = mask_part.parse::<i64>() {
                                // Parse the expected result
                                if let Ok(expected) = expected_result.parse::<i64>() {
                                    // Parse the actual value
                                    if let Ok(val) = value.parse::<i64>() {
                                        let result = val ^ mask;
                                        eprintln!("DEBUG bitxor: val={}, mask={}, result={}, expected={}", val, mask, result, expected);
                                        return Ok(result == expected);
                                    } else {
                                        eprintln!("DEBUG: Failed to parse value '{}' as i64", value);
                                    }
                                } else {
                                    eprintln!("DEBUG: Failed to parse expected '{}' as i64", expected_result);
                                }
                            } else {
                                eprintln!("DEBUG: Failed to parse mask '{}' as i64", mask_part);
                            }
                        } else {
                            eprintln!("DEBUG: var_part '{}' doesn't start with $", var_part);
                        }
                    } else {
                        eprintln!("DEBUG: bit_parts.len() = {}, expected 2", bit_parts.len());
                    }
                } else {
                    eprintln!("DEBUG: left_side '{}' doesn't contain ^", left_side);
                }
            } else {
                eprintln!("DEBUG: parts.len() = {}, expected 2", parts.len());
            }
        }

        // Handle bit NOT operations like "~ $X == -1"
        // Python: ~x = -(x + 1)
        if expr.contains('~') && expr.contains("==") {
            // Format: ~$VAR == N or ~ $VAR == N
            let parts: Vec<&str> = expr.split("==").collect();
            if parts.len() == 2 {
                let left_side = parts[0].trim();
                let expected_result = parts[1].trim();
                
                // Remove the ~ operator and get the variable part
                // Handle both "~$VAR" and "~ $VAR"
                let var_part = if left_side.starts_with("~") {
                    left_side[1..].trim()
                } else {
                    left_side
                };
                
                eprintln!("DEBUG bitnot: var_part='{}', expected='{}'", var_part, expected_result);
                
                // Check if this is the metavariable we're evaluating
                if var_part.starts_with("$") {
                    // Parse the expected result
                    if let Ok(expected) = expected_result.parse::<i64>() {
                        // Parse the actual value
                        if let Ok(val) = value.parse::<i64>() {
                            // Python's ~ operator: ~x = -(x + 1)
                            let result = -(val + 1);
                            eprintln!("DEBUG bitnot: val={}, result={}, expected={}", val, result, expected);
                            return Ok(result == expected);
                        } else {
                            eprintln!("DEBUG: Failed to parse value '{}' as i64", value);
                        }
                    } else {
                        eprintln!("DEBUG: Failed to parse expected '{}' as i64", expected_result);
                    }
                } else {
                    eprintln!("DEBUG: var_part '{}' doesn't start with $", var_part);
                }
            } else {
                eprintln!("DEBUG: parts.len() = {}, expected 2", parts.len());
            }
        }

        // For now, just return true for unsupported expressions
        eprintln!("DEBUG: Expression '{}' not handled, returning true", expr);
        Ok(true)
    }

    /// Evaluate custom condition
    fn evaluate_custom_condition(&self, condition_name: &str, _match_result: &SemgrepMatchResult) -> Result<bool> {
        match condition_name {
            "always_true" => Ok(true),
            "always_false" => Ok(false),
            _ => Ok(true), // Default to true for unknown conditions
        }
    }

    /// Check if a taint flow matches the data flow specification
    fn matches_dataflow_spec(&self, flow: &astgrep_dataflow::TaintFlow, spec: &DataFlowSpec) -> bool {
        // Simple pattern matching for sources and sinks
        let source_matches = spec.sources.iter().any(|pattern| {
            match pattern.pattern_type() {
                PatternType::Simple(ref s) => {
                    let text = &flow.source.description;
                    text.contains(s)
                }
                _ => false,
            }
        });

        let sink_matches = spec.sinks.iter().any(|pattern| {
            match pattern.pattern_type() {
                PatternType::Simple(ref s) => {
                    let text = &flow.sink.description;
                    text.contains(s)
                }
                _ => false,
            }
        });

        source_matches && sink_matches
    }

    /// Create a finding from a pattern match
    fn create_finding_from_match(
        &self,
        rule: &Rule,
        pattern: &Pattern,
        match_result: &SemgrepMatchResult,
        file_path: Option<&Path>,
    ) -> Result<Finding> {
        let location = match_result.node.location().map(|(start_line, start_col, end_line, end_col)| {
            Location {
                file: file_path.map(|p| p.to_path_buf()).unwrap_or_default(),
                start_line,
                start_column: start_col,
                end_line,
                end_column: end_col,
            }
        }).unwrap_or_else(|| {
            Location {
                file: file_path.map(|p| p.to_path_buf()).unwrap_or_default(),
                start_line: 1,
                start_column: 1,
                end_line: 1,
                end_column: 1,
            }
        });

        let mut message = rule.description.clone();
        
        // Replace metavariables in message
        for (name, value) in &match_result.bindings {
            let placeholder = format!("${}", name);
            message = message.replace(&placeholder, value);
        }

        let mut metadata = HashMap::new();
        metadata.insert("rule_name".to_string(), Value::String(rule.name.clone()));
        let pattern_str = pattern.get_pattern_string().unwrap_or(&"<complex pattern>".to_string()).clone();
        metadata.insert("pattern".to_string(), Value::String(pattern_str));
        
        if let Some(category) = rule.get_metadata_string("category") {
            metadata.insert("category".to_string(), Value::String(category));
        }

        Ok(Finding {
            rule_id: rule.id.clone(),
            message,
            location,
            severity: rule.severity,
            confidence: rule.confidence,
            metadata,
            fix_suggestion: None,
        })
    }

    /// Create a finding from a data flow analysis
    fn create_dataflow_finding(
        &self,
        rule: &Rule,
        flow: &astgrep_dataflow::TaintFlow,
        file_path: Option<&Path>,
    ) -> Result<Finding> {
        let location = Location {
            file: file_path.map(|p| p.to_path_buf()).unwrap_or_default(),
            start_line: 1, // Would need to extract from flow
            start_column: 1,
            end_line: 1,
            end_column: 1,
        };

        let message = format!(
            "{}: Potential {} vulnerability - data flows from {} to {}",
            rule.name,
            &flow.vulnerability_type,
            flow.source.description,
            flow.sink.description
        );

        let mut metadata = HashMap::new();
        metadata.insert("rule_name".to_string(), Value::String(rule.name.clone()));
        metadata.insert("analysis_type".to_string(), Value::String("dataflow".to_string()));
        metadata.insert("vulnerability_type".to_string(), Value::String(flow.vulnerability_type.clone()));
        metadata.insert("confidence".to_string(), Value::String(format!("{:.2}", flow.confidence)));

        Ok(Finding {
            rule_id: rule.id.clone(),
            message,
            location,
            severity: rule.severity,
            confidence: rule.confidence,
            metadata,
            fix_suggestion: None,
        })
    }

    /// Execute taint analysis for taint mode rules
    pub fn execute_taint_analysis(
        &mut self,
        rule: &Rule,
        dataflow_spec: &DataFlowSpec,
        ast: &dyn AstNode,
        dataflow_analysis: Option<&DataFlowAnalysis>,
        file_path: Option<&Path>,
    ) -> Result<Vec<Finding>> {
        let mut findings = Vec::new();
        let source_text = ast.text().unwrap_or_default();
        
        // Debug: Print dataflow spec details
        eprintln!("[DEBUG] Dataflow spec - sources: {}, sinks: {}, propagators: {}", 
                  dataflow_spec.sources.len(), 
                  dataflow_spec.sinks.len(),
                  dataflow_spec.propagators.len());
        for (i, prop) in dataflow_spec.propagators.iter().enumerate() {
            eprintln!("[DEBUG] Propagator {}: pattern='{:?}', from='{}', to='{}'", 
                      i, prop.pattern.pattern_type, prop.from, prop.to);
        }
        
        // Step 1: Find all source matches using pattern matching
        let source_matches = self.find_taint_sources(ast, dataflow_spec, &source_text)?;
        eprintln!("[DEBUG] Source matches found: {}", source_matches.len());
        if source_matches.is_empty() {
            eprintln!("[DEBUG] No source matches, returning early");
            return Ok(findings);
        }

        // Step 2: Find all sink matches using pattern matching
        let sink_matches = self.find_taint_sinks(ast, dataflow_spec, &source_text)?;
        if sink_matches.is_empty() {
            return Ok(findings);
        }
        
        // Step 3: Check for taint flow from sources to sinks
        // Get taint options from rule metadata or dataflow spec
        let assume_safe_booleans = if let Some(val) = rule.metadata.get("taint_assume_safe_booleans") {
            if let serde_yaml::Value::String(ref s) = val {
                s == "true"
            } else if let serde_yaml::Value::Bool(ref b) = val {
                *b
            } else {
                false
            }
        } else {
            dataflow_spec.taint_assume_safe_booleans.unwrap_or(false)
        };
        
        let assume_safe_numbers = if let Some(val) = rule.metadata.get("taint_assume_safe_numbers") {
            if let serde_yaml::Value::String(ref s) = val {
                s == "true"
            } else if let serde_yaml::Value::Bool(ref b) = val {
                *b
            } else {
                false
            }
        } else {
            dataflow_spec.taint_assume_safe_numbers.unwrap_or(false)
        };
        
        let only_propagate_through_assignments = if let Some(val) = rule.metadata.get("taint_only_propagate_through_assignments") {
            if let serde_yaml::Value::String(ref s) = val {
                s == "true"
            } else if let serde_yaml::Value::Bool(ref b) = val {
                *b
            } else {
                false
            }
        } else {
            dataflow_spec.taint_only_propagate_through_assignments.unwrap_or(false)
        };
        
        let taint_flows = self.detect_taint_flows(
            &source_matches,
            &sink_matches,
            ast,
            dataflow_analysis,
            assume_safe_booleans,
            assume_safe_numbers,
            only_propagate_through_assignments,
            &source_text,
            &dataflow_spec.propagators
        )?;

        // Step 4: Create findings for each unique sink with taint flow
        // Filter out nested/contained findings (keep only outermost ones)
        let mut filtered_flows: Vec<(TaintMatch, TaintMatch)> = Vec::new();
        
        // Sort flows by start position (line, col) in ascending order
        let mut sorted_flows = taint_flows.clone();
        sorted_flows.sort_by(|(_, sink_a), (_, sink_b)| {
            let loc_a = sink_a.node.location().unwrap_or((0, 0, 0, 0));
            let loc_b = sink_b.node.location().unwrap_or((0, 0, 0, 0));
            loc_a.0.cmp(&loc_b.0).then(loc_a.1.cmp(&loc_b.1))
        });
        
        // Keep only flows that are not contained within another flow
        for (source_match, sink_match) in sorted_flows {
            if let Some(location) = sink_match.node.location() {
                let (start_line, start_col, end_line, end_col) = location;
                
                // Check if this flow is contained within any already kept flow
                let is_contained = filtered_flows.iter().any(|(_, existing_sink)| {
                    if let Some(existing_loc) = existing_sink.node.location() {
                        let (e_start_line, e_start_col, e_end_line, e_end_col) = existing_loc;
                        // This flow is contained if it's inside the existing flow
                        (start_line > e_start_line || (start_line == e_start_line && start_col >= e_start_col))
                            && (end_line < e_end_line || (end_line == e_end_line && end_col <= e_end_col))
                    } else {
                        false
                    }
                });
                
                if !is_contained {
                    filtered_flows.push((source_match, sink_match));
                }
            }
        }
        
        // Create findings for filtered flows
        for (_source_match, sink_match) in filtered_flows {
            if let Some(location) = sink_match.node.location() {
                let finding = Finding::new(
                    rule.id.clone(),
                    format!("{}: {}", rule.name, rule.description),
                    rule.severity,
                    rule.confidence,
                    Location::new(
                        file_path.map(|p| p.to_path_buf()).unwrap_or_default(),
                        location.0, location.1, location.2, location.3
                    ),
                );
                findings.push(finding);
            }
        }

        Ok(findings)
    }
    
    /// Find all taint sources matching the source patterns
    fn find_taint_sources(
        &mut self,
        ast: &dyn AstNode,
        dataflow_spec: &DataFlowSpec,
        source_text: &str
    ) -> Result<Vec<TaintMatch>> {
        eprintln!("[DEBUG] ENTER find_taint_sources with {} source patterns", dataflow_spec.sources.len());
        let mut sources = Vec::new();

        for source_pattern in &dataflow_spec.sources {
            // Always try to find annotated method parameters (e.g., @RequestParam, @PathVariable)
            // These are common taint sources in web applications
            let annotation_sources = self.find_annotated_method_params(ast, source_text);
            if !annotation_sources.is_empty() {
                eprintln!("[DEBUG] Found {} annotated method parameter sources", annotation_sources.len());
                sources.extend(annotation_sources);
            }

            // Normalize pattern: remove trailing semicolons and whitespace for more flexible matching
            let original_pattern = source_pattern.pattern_text();
            let normalized_pattern = original_pattern.trim_end_matches(';').trim_end_matches('\n').trim();
            eprintln!("[DEBUG] Normalizing source pattern: '{:?}' -> '{}'", original_pattern, normalized_pattern);

            // Convert source pattern to SemgrepPattern
            let semgrep_pattern = astgrep_core::SemgrepPattern {
                pattern_type: astgrep_core::PatternType::Simple(normalized_pattern.to_string()),
                metavariable_pattern: None,
                conditions: Vec::new(),
                focus: if source_pattern.focus_metavariables.is_empty() {
                    None
                } else {
                    Some(source_pattern.focus_metavariables.clone())
                },
            };

            // Find matches
            let mut matches = self.pattern_matcher.find_matches(&semgrep_pattern, ast)?;
            
            // If no matches and pattern looks like a fully qualified name, try matching just the class and method
            if matches.is_empty() && normalized_pattern.contains('.') {
                if let Some(simplified) = Self::simplify_fully_qualified_pattern(normalized_pattern) {
                    eprintln!("[DEBUG] No matches with full pattern, trying simplified: '{}'", simplified);
                    let simplified_semgrep_pattern = astgrep_core::SemgrepPattern {
                        pattern_type: astgrep_core::PatternType::Simple(simplified),
                        metavariable_pattern: None,
                        conditions: Vec::new(),
                        focus: if source_pattern.focus_metavariables.is_empty() {
                            None
                        } else {
                            Some(source_pattern.focus_metavariables.clone())
                        },
                    };
                    matches = self.pattern_matcher.find_matches(&simplified_semgrep_pattern, ast)?;
                }
            }
            
            eprintln!("[DEBUG] Source matches found: {}", matches.len());
            for m in matches {
                eprintln!("[DEBUG] Source match: bindings={:?}, text={:?}", m.bindings, m.node.text());
                // Extract the variable name from bindings if available
                let mut var_name = None;
                
                // If focus-metavariables are specified, extract the binding for the first focus variable
                if !source_pattern.focus_metavariables.is_empty() {
                    let focus_var = &source_pattern.focus_metavariables[0];
                    // Remove the "$" prefix to match the binding key
                    let focus_key = focus_var.trim_start_matches('$');
                    if let Some(value) = m.bindings.get(focus_key) {
                        if !value.is_empty() {
                            var_name = Some(value.clone());
                            eprintln!("[DEBUG] Extracted var_name from focus-metavariable '{}': {}", focus_var, value);
                        }
                    }
                }
                
                // If no var_name from focus-metavariable, try any binding that starts with "$"
                if var_name.is_none() {
                    for (key, value) in &m.bindings {
                        if key.starts_with("$") && !value.is_empty() {
                            var_name = Some(value.clone());
                            break;
                        }
                    }
                }

                // If no var_name from bindings, try to extract from parent assignment
                if var_name.is_none() {
                    var_name = self.extract_variable_name_from_assignment(m.node.as_ref(), source_text);
                }
                
                // If still no var_name, check if source is in a for-each loop and extract the iteration variable
                if var_name.is_none() {
                    var_name = self.extract_foreach_iteration_variable(m.node.as_ref(), source_text);
                }
                
                // If still no var_name and the match looks like a string literal,
                // try to find the variable that is assigned this string literal
                if var_name.is_none() {
                    if let Some(text) = m.node.text() {
                        let text = text.trim();
                        // Check if this is a string literal pattern match (starts and ends with ")
                        if text.starts_with('"') && text.ends_with('"') && text.len() > 2 {
                            // This is a string literal match, find the variable it's assigned to
                            if let Some((start_line, _, _, _)) = m.node.location() {
                                var_name = self.find_variable_for_string_literal(source_text, start_line, text);
                            }
                        }
                    }
                }
                
                // If still no var_name and focus-metavariables are specified, 
                // try to extract from method parameters for method declaration patterns
                if var_name.is_none() && !source_pattern.focus_metavariables.is_empty() {
                    var_name = self.extract_focused_parameter_name(m.node.as_ref());
                }
                
                // If still no var_name and pattern is a simple variable pattern like $SOURCE,
                // try to extract from method parameter declarations
                if var_name.is_none() {
                    if let Some(text) = m.node.text() {
                        let text = text.trim();
                        // Check if this looks like a simple identifier that could be a method parameter
                        if !text.contains("(") && !text.contains(".") && !text.contains("=") {
                            var_name = self.extract_method_parameter_name(m.node.as_ref(), source_text, text);
                        }
                    }
                }
                
                // If still no var_name, check if source is assigned to a field/variable
                // Pattern: Type var = source() or var = source()
                if var_name.is_none() {
                    if let Some(text) = m.node.text() {
                        // Check for field/variable assignment: Type var = DocumentBuilderFactory.newInstance()
                        // The text might be the full assignment statement
                        var_name = self.extract_field_assignment_target(m.node.as_ref(), source_text, text);
                        
                        // If still not found, try to extract from the text itself if it looks like an assignment
                        if var_name.is_none() && (text.contains("=") || text.contains("static")) {
                            var_name = self.extract_var_from_assignment_text(text);
                        }
                    }
                }
                
                // If still no var_name, check if this is a tainted value being assigned to a field/variable
                // Pattern: x = tainted  or  this.x = tainted
                if var_name.is_none() {
                    if let Some(text) = m.node.text() {
                        if text == "tainted" || text.contains("tainted") {
                            var_name = self.extract_assignment_target(m.node.as_ref(), source_text);
                            
                            // Also check if this is in a setter call: obj.setX(tainted)
                            if var_name.is_none() {
                                if let Some((line_num, _, _, _)) = m.node.location() {
                                    var_name = self.extract_setter_argument(line_num, source_text);
                                }
                            }
                        }
                    }
                }

                eprintln!("[DEBUG] Extracted var_name: {:?}", var_name);
                
                // Check if the source variable is sanitized and skip if so
                if let Some(ref vname) = var_name {
                    if let Some((start_line, _, _, _)) = m.node.location() {
                        let lines: Vec<&str> = source_text.lines().collect();
                        if start_line > 0 && start_line <= lines.len() {
                            let line_text = lines[start_line - 1];
                            // Find the assignment and check if right-hand side is sanitized
                            if let Some(eq_pos) = line_text.find('=') {
                                let after_eq = &line_text[eq_pos + 1..].trim();
                                if self.is_sanitized_expression(after_eq) {
                                    eprintln!("[DEBUG] Source variable '{}' is sanitized, skipping", vname);
                                    continue;
                                }
                            }
                        }
                    }
                }

                // When taint_assume_safe_numbers is true, filter out numeric type sources
                if dataflow_spec.taint_assume_safe_numbers.unwrap_or(false) {
                    if let Some(ref vname) = var_name {
                        if self.is_numeric_parameter(m.node.as_ref(), vname) {
                            continue;
                        }
                    }
                }

                // Extract method name for scope isolation using source location
                let node_ref = m.node.as_ref();
                
                // First check if we have method name in bindings (e.g., from pattern like "public void $F(...)")
                let method_name_from_bindings = m.bindings.get("F").cloned();
                
                let method_name = if let Some(name) = method_name_from_bindings {
                    Some(name)
                } else if node_ref.node_type() == "method_declaration" {
                    self.extract_method_name_from_declaration(node_ref)
                } else if let Some((start_line, _, _, _)) = node_ref.location() {
                    self.find_method_name_by_line(source_text, start_line)
                } else {
                    None
                };

                sources.push(TaintMatch {
                    node: m.node,
                    bindings: m.bindings,
                    var_name,
                    method_name,
                });
            }
        }
        
        Ok(sources)
    }

    /// Find annotated method parameters (e.g., @RequestParam String orderBy)
    /// This handles complex source patterns with annotation matching
    fn find_annotated_method_params(
        &self,
        ast: &dyn AstNode,
        source_text: &str,
    ) -> Vec<TaintMatch> {
        let mut results = Vec::new();

        eprintln!("[DEBUG] Looking for annotated method parameters");

        // List of taint-related annotations
        let taint_annotations = [
            "RequestParam",
            "PathVariable",
            "RequestBody",
            "RequestHeader",
            "CookieValue",
        ];

        // Iterate through all lines to find method declarations with annotated parameters
        for (line_num, line) in source_text.lines().enumerate() {
            eprintln!("[DEBUG] Checking line {}: {}", line_num, line);
            // Check if this line contains any taint annotation
            for annotation in &taint_annotations {
                let anno_str = format!("@{}", annotation);
                if line.contains(&anno_str) {
                    eprintln!("[DEBUG] Found annotation {} in line {}", anno_str, line_num);
                    // Found an annotation, now try to extract the parameter name
                    if let Some(param_name) = self.extract_annotated_param(line, annotation) {
                        eprintln!(
                            "[DEBUG] Found annotated parameter: {} with @{}",
                            param_name, annotation
                        );

                        // Find the method name for this parameter
                        let method_name =
                            self.find_method_name_by_line(source_text, line_num + 1);

                        // Create a TaintMatch for this parameter
                        // We need to find or create a node for this parameter
                        if let Some(param_node) =
                            self.find_param_node_by_name(ast, &param_name)
                        {
                            let mut bindings = std::collections::HashMap::new();
                            bindings.insert(
                                "SOURCE".to_string(),
                                param_name.clone(),
                            );

                            results.push(TaintMatch {
                                node: param_node,
                                bindings,
                                var_name: Some(param_name),
                                method_name,
                            });
                        }
                    }
                }
            }
        }

        results
    }

    /// Extract parameter name from a line with annotation
    /// e.g., "@RequestParam(value = \"smth\") String orderBy" -> "orderBy"
    fn extract_annotated_param(&self, line: &str, annotation: &str) -> Option<String> {
        eprintln!("[DEBUG] extract_annotated_param called with line: '{}'", line);
        // Find the annotation position
        if let Some(anno_pos) = line.find(&format!("@{}", annotation)) {
            eprintln!("[DEBUG] Found annotation at position {}", anno_pos);
            let after_anno = &line[anno_pos..];

            // Skip the annotation and its parentheses if present
            // Need to find the matching closing paren for the annotation
            let after_anno = if after_anno.contains('(') {
                // Find the matching closing paren by counting
                let mut paren_count = 0;
                let mut close_pos = None;
                for (i, c) in after_anno.char_indices() {
                    match c {
                        '(' => paren_count += 1,
                        ')' => {
                            paren_count -= 1;
                            if paren_count == 0 {
                                close_pos = Some(i);
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                if let Some(pos) = close_pos {
                    eprintln!("[DEBUG] Found matching closing paren at position {}", pos);
                    &after_anno[pos + 1..]
                } else {
                    eprintln!("[DEBUG] No matching closing paren found");
                    // Fallback: just skip the annotation name
                    &after_anno[annotation.len() + 1..]
                }
            } else {
                // No parentheses, skip just the annotation name
                eprintln!("[DEBUG] No parentheses in annotation");
                &after_anno[annotation.len() + 1..]
            };

            eprintln!("[DEBUG] After annotation: '{}'", after_anno);

            // Now we should have something like: " String orderBy" or "String orderBy, HttpServletResponse response) {"
            // Extract the parameter name (the word after the type, right after the annotation)
            let parts: Vec<&str> = after_anno.split_whitespace().collect();
            eprintln!("[DEBUG] Parts after split: {:?}", parts);

            // We expect at least 2 parts: type and variable name
            // e.g., ["String", "orderBy,"] or ["String", "orderBy,", "HttpServletResponse", "response)"]
            // The parameter name is the second element (index 1), not the last one
            if parts.len() >= 2 {
                let param_name = parts[1].trim();
                // Remove trailing comma, closing paren, or other punctuation if present
                let param_name = param_name.trim_end_matches(|c: char| c == ',' || c == ')' || c == '{').to_string();

                eprintln!("[DEBUG] Extracted param name: '{}'", param_name);

                // Validate it looks like a variable name
                if !param_name.is_empty()
                    && param_name.chars().next().map(|c| c.is_alphabetic()).unwrap_or(false)
                    && !param_name.contains("(")
                    && !param_name.contains(")")
                    && !param_name.contains(",")
                {
                    eprintln!("[DEBUG] Param name validated: '{}'", param_name);
                    return Some(param_name);
                } else {
                    eprintln!("[DEBUG] Param name validation failed for: '{}'", param_name);
                }
            } else {
                eprintln!("[DEBUG] Not enough parts: {}", parts.len());
            }
        } else {
            eprintln!("[DEBUG] Annotation not found in line");
        }

        None
    }

    /// Find a parameter node by name in the AST
    fn find_param_node_by_name(
        &self,
        ast: &dyn AstNode,
        param_name: &str,
    ) -> Option<Box<dyn AstNode>> {
        // Try to find a formal_parameter or identifier node with the given name
        self.find_node_by_type_and_text(ast, "identifier", param_name)
    }

    /// Find a node by type and text content
    fn find_node_by_type_and_text(
        &self,
        node: &dyn AstNode,
        node_type: &str,
        text: &str,
    ) -> Option<Box<dyn AstNode>> {
        if node.node_type() == node_type {
            if let Some(node_text) = node.text() {
                if node_text.trim() == text {
                    return Some(node.clone_node());
                }
            }
        }

        // Recursively search children
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                if let Some(found) = self.find_node_by_type_and_text(&*child, node_type, text) {
                    return Some(found);
                }
            }
        }

        None
    }

    /// Find the variable name that is assigned a specific string literal
    /// This is used when a string literal pattern matches to find the variable it's assigned to
    fn find_variable_for_string_literal(
        &self,
        source_text: &str,
        line_num: usize,
        literal: &str,
    ) -> Option<String> {
        let lines: Vec<&str> = source_text.lines().collect();
        if line_num == 0 || line_num > lines.len() {
            return None;
        }

        // Look at the line containing the string literal
        let line = lines[line_num - 1];
        
        // Pattern: Type var = "literal" or var = "literal"
        // Find the assignment that contains this string literal
        if let Some(eq_pos) = line.find('=') {
            let before_eq = line[..eq_pos].trim();
            let after_eq = line[eq_pos + 1..].trim().trim_end_matches(';').trim();
            
            // Check if the string literal appears in the right-hand side
            if after_eq.contains(literal) {
                // Extract variable name from left-hand side
                let parts: Vec<&str> = before_eq.split_whitespace().collect();
                if let Some(var_name) = parts.last() {
                    let var_name = var_name.trim().to_string();
                    if !var_name.is_empty() {
                        eprintln!("[DEBUG] Found variable '{}' assigned string literal: {}", var_name, literal);
                        return Some(var_name);
                    }
                }
            }
        }

        None
    }
    
    /// Find method name by line number in source text
    fn find_method_name_by_line(&self, source_text: &str, line_num: usize) -> Option<String> {
        let lines: Vec<&str> = source_text.lines().collect();
        if line_num == 0 || line_num > lines.len() {
            return None;
        }
        
        // Search backwards from the given line to find the method declaration
        // Method declarations typically look like: "public void methodName(...)" or "public static void methodName(...)"
        for i in (0..line_num).rev() {
            let line = lines[i];
            
            // Skip class declarations (contain "class" keyword)
            if line.contains("class") && line.contains("public") {
                continue;
            }
            
            // Look for method declaration patterns
            // Pattern: public [static] [final] [returnType] methodName(
            // Return type can be: void, primitive types, or ClassName (with possible generics)
            if let Some(captures) = regex::Regex::new(r"public\s+(?:static\s+)?(?:final\s+)?(?:void|int|long|short|byte|float|double|boolean|char|\w+(?:<[^>]+>)?)\s+(\w+)\s*\(").ok()?.captures(line) {
                if let Some(method_name) = captures.get(1) {
                    return Some(method_name.as_str().to_string());
                }
            }
            
            // Also check for ResponseEntity<String> pattern (generic return types)
            if let Some(captures) = regex::Regex::new(r"public\s+(?:static\s+)?(?:\w+(?:<[^>]+>)?)\s+(\w+)\s*\(").ok()?.captures(line) {
                if let Some(method_name) = captures.get(1) {
                    let name = method_name.as_str();
                    // Make sure it's not a class name (class names typically start with uppercase)
                    // Method names typically start with lowercase
                    if name.chars().next().map(|c| c.is_lowercase()).unwrap_or(false) {
                        return Some(name.to_string());
                    }
                }
            }
        }
        
        None
    }

    /// Find the method name that contains a given node
    fn find_containing_method_name(&self, node: &dyn AstNode) -> Option<String> {
        // Try to find the parent method by looking at the node's text context
        // For Java, method declarations look like: "public void methodName(...) { ... }"
        let node_text = node.text().unwrap_or_default();
        
        // Simple heuristic: look for method signature patterns in the text
        // This is a simplified approach - in a real implementation we'd traverse the AST
        if let Some(method_match) = regex::Regex::new(r"public\s+(?:static\s+)?(?:void|\w+)\s+(\w+)\s*\(").ok() {
            for cap in method_match.captures_iter(&node_text) {
                if let Some(method_name) = cap.get(1) {
                    return Some(method_name.as_str().to_string());
                }
            }
        }
        
        // If the node itself is a method declaration, extract its name
        if node.node_type() == "method_declaration" {
            return self.extract_method_name_from_declaration(node);
        }
        
        None
    }
    
    /// Extract method name from a method declaration node
    fn extract_method_name_from_declaration(&self, node: &dyn AstNode) -> Option<String> {
        if node.node_type() != "method_declaration" {
            return None;
        }
        
        // Look for the identifier child which is the method name
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                if child.node_type() == "identifier" {
                    return child.text().map(|s| s.to_string());
                }
            }
        }
        
        // Fallback: try to extract from text
        let text = node.text().unwrap_or_default();
        if let Some(paren_idx) = text.find('(') {
            let before = &text[..paren_idx];
            if let Some(space_idx) = before.rfind(' ') {
                return Some(before[space_idx+1..].trim().to_string());
            }
        }
        
        None
    }

    /// Extract variable name from an assignment expression
    fn extract_variable_name_from_assignment(
        &self,
        node: &dyn AstNode,
        source_text: &str
    ) -> Option<String> {
        // Try to get parent node by searching through the tree
        let node_text = node.text().unwrap_or_default();
        eprintln!("[DEBUG] extract_variable_name_from_assignment: node_text='{}', node_type='{}'", node_text, node.node_type());

        // Get the node's location to find it in the full source
        if let Some((start_line, start_col, _end_line, _end_col)) = node.location() {
            eprintln!("[DEBUG] Node location: line={}, col={}", start_line, start_col);
            
            // Find the line containing this node
            let lines: Vec<&str> = source_text.lines().collect();
            if start_line > 0 && start_line <= lines.len() {
                let line_text = lines[start_line - 1];
                eprintln!("[DEBUG] Line containing node: '{}'", line_text);
                
                // Look for pattern: "var = <node_text>" or "Type var = <node_text>"
                // Find where the node text appears in the line
                if let Some(node_pos) = line_text.find(&node_text) {
                    let before_node = &line_text[..node_pos];
                    let after_node = &line_text[node_pos + node_text.len()..];
                    eprintln!("[DEBUG] Text before node: '{}'", before_node);
                    eprintln!("[DEBUG] Text after node: '{}'", after_node);
                    
                    // Check if the variable is sanitized (e.g., .replaceAll("'", ""))
                    if self.is_sanitized_expression(after_node) {
                        eprintln!("[DEBUG] Variable is sanitized, not treating as tainted source");
                        return None;
                    }
                    
                    // Look for the last '=' before the node
                    if let Some(eq_pos) = before_node.rfind('=') {
                        let before_eq = &before_node[..eq_pos].trim();
                        eprintln!("[DEBUG] Text before '=': '{}'", before_eq);
                        
                        // Extract the variable name (last word before '=')
                        let parts: Vec<&str> = before_eq.split_whitespace().collect();
                        if let Some(last_part) = parts.last() {
                            let var_name = last_part.trim().to_string();
                            // Clean up any trailing characters like semicolons or spaces
                            let var_name = var_name.trim_end_matches(';').trim().to_string();
                            if !var_name.is_empty() && !var_name.contains("(") && !var_name.contains("=") {
                                eprintln!("[DEBUG] Extracted var_name '{}' from assignment", var_name);
                                return Some(var_name);
                            }
                        }
                    }
                }
            }
        }

        None
    }

    /// Extract the focused parameter name from a method declaration
    /// This is used when focus-metavariable is set to a parameter in a method pattern
    fn extract_focused_parameter_name(&self, node: &dyn AstNode) -> Option<String> {
        // Only handle method declarations
        if node.node_type() != "method_declaration" {
            return None;
        }
        
        // Look for formal_parameters child
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                if child.node_type() == "formal_parameters" {
                    // Find the first parameter
                    for j in 0..child.child_count() {
                        if let Some(param) = child.child(j) {
                            if param.node_type() == "formal_parameter" {
                                // Find the identifier in the parameter
                                for k in 0..param.child_count() {
                                    if let Some(param_child) = param.child(k) {
                                        if param_child.node_type() == "identifier" {
                                            return param_child.text().map(|s| s.to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        None
    }

    /// Extract the iteration variable from a for-each loop when source is in the iterable expression
    /// For example: "for (StackTraceElement ste : Thread.currentThread().getStackTrace())"
    /// When source matches "getStackTrace()", this should return "ste"
    fn extract_foreach_iteration_variable(&self, node: &dyn AstNode, source_text: &str) -> Option<String> {
        let node_text = node.text().unwrap_or_default();
        
        // Get the node's location
        if let Some((start_line, _start_col, _end_line, _end_col)) = node.location() {
            let lines: Vec<&str> = source_text.lines().collect();
            if start_line > 0 && start_line <= lines.len() {
                let line_text = lines[start_line - 1];
                
                // Check if this line contains a for-each loop pattern
                // Pattern: "for (Type var : iterable_expression)"
                if line_text.contains("for (") && line_text.contains(":") {
                    // Find the for loop pattern
                    if let Some(for_start) = line_text.find("for (") {
                        let after_for = &line_text[for_start + 5..]; // Skip "for ("
                        
                        // Look for the colon separator
                        if let Some(colon_pos) = after_for.find(':') {
                            let before_colon = &after_for[..colon_pos].trim();
                            
                            // Extract the variable name from "Type var" pattern
                            // The variable name is the last word before the colon
                            let parts: Vec<&str> = before_colon.split_whitespace().collect();
                            if parts.len() >= 2 {
                                // Last part should be the variable name
                                let var_name = parts.last().unwrap().trim().to_string();
                                eprintln!("[DEBUG] Extracted for-each iteration variable: '{}'", var_name);
                                return Some(var_name);
                            }
                        }
                    }
                }
            }
        }
        
        None
    }

    /// Check if an expression is sanitized (e.g., contains .replaceAll("'", ""))
    fn is_sanitized_expression(&self, expr: &str) -> bool {
        let expr = expr.trim();
        
        // Check for common sanitization patterns
        // Pattern 1: .replaceAll("'", "") - removes quotes (SQL injection prevention)
        if expr.contains(".replaceAll") && expr.contains("'") {
            return true;
        }
        
        // Pattern 2: .replace("'", "") - simple replace
        if expr.contains(".replace") && expr.contains("'") {
            return true;
        }
        
        // Pattern 3: PreparedStatement.setString() - parameterized query
        if expr.contains(".setString") {
            return true;
        }
        
        // Pattern 4: encode(), escape() methods
        if expr.contains(".encode") || expr.contains(".escape") {
            return true;
        }
        
        // Pattern 5: validation patterns like matches(), contains validation
        if expr.contains(".matches") || expr.contains(".validate") {
            return true;
        }
        
        false
    }

    /// Extract method parameter name when a simple identifier pattern matches
    /// This handles cases like @RequestParam String path where pattern is $SOURCE
    fn extract_method_parameter_name(&self, node: &dyn AstNode, source_text: &str, matched_text: &str) -> Option<String> {
        eprintln!("[DEBUG] extract_method_parameter_name: matched_text='{}'", matched_text);
        
        // Get the node's location
        if let Some((start_line, _start_col, _end_line, _end_col)) = node.location() {
            let lines: Vec<&str> = source_text.lines().collect();
            if start_line > 0 && start_line <= lines.len() {
                let line_text = lines[start_line - 1];
                eprintln!("[DEBUG] Checking line {}: '{}'", start_line, line_text);
                
                // Check if this line contains a method declaration with parameters
                // Pattern: "methodName(..., Type paramName, ...)" or "methodName(@Annotation Type paramName, ...)"
                if line_text.contains("public ") && line_text.contains('(') && line_text.contains(')') {
                    eprintln!("[DEBUG] Found method declaration line");
                    
                    // Look for the method parameter section
                    if let Some(paren_start) = line_text.find('(') {
                        if let Some(paren_end) = line_text.rfind(')') {
                            let params_section = &line_text[paren_start..=paren_end];
                            eprintln!("[DEBUG] Params section: '{}'", params_section);
                            
                            // Check if the matched identifier appears in the parameter list
                            // First try to find it as a standalone word
                            let search_pattern = format!("\\b{}\\b", regex::escape(matched_text));
                            if let Ok(re) = regex::Regex::new(&search_pattern) {
                                if re.is_match(params_section) {
                                    eprintln!("[DEBUG] Found '{}' in params section", matched_text);
                                    // Verify this is actually a parameter by checking context
                                    // Split params by comma to get individual parameters
                                    let params: Vec<&str> = params_section[1..params_section.len()-1].split(',').collect();
                                    for param in params {
                                        let param = param.trim();
                                        eprintln!("[DEBUG] Checking param: '{}'", param);
                                        // Check if param ends with our matched text (the variable name)
                                        // Pattern: "Type varName" or "@Annotation Type varName"
                                        let param_words: Vec<&str> = param.split_whitespace().collect();
                                        if let Some(last_word) = param_words.last() {
                                            if *last_word == matched_text {
                                                eprintln!("[DEBUG] Extracted method parameter: '{}'", matched_text);
                                                return Some(matched_text.to_string());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        None
    }

    /// Check if a method body contains sanitization operations for a specific variable
    /// This detects patterns like PreparedStatement.setString() which parameterizes queries
    fn contains_sanitization_in_scope(
        &self,
        method_body: &str,
        var_name: Option<&str>,
    ) -> bool {
        eprintln!("[DEBUG] Checking sanitization in method body of length {}", method_body.len());
        
        // Check for PreparedStatement.setString() or similar parameterization patterns
        // This indicates the variable is being safely parameterized
        if method_body.contains(".setString(") {
            eprintln!("[DEBUG] Found .setString() sanitization in method body");
            return true;
        }
        
        // Check for other common sanitization patterns
        if method_body.contains("PreparedStatement") && method_body.contains("setString") {
            eprintln!("[DEBUG] Found PreparedStatement with setString sanitization");
            return true;
        }
        
        // Check for .replaceAll() patterns that remove dangerous characters
        if method_body.contains(".replaceAll") {
            // If we know the variable name, check if it's specifically sanitized
            if let Some(vname) = var_name {
                // Check if the variable appears in a replaceAll call
                if method_body.contains(&format!("{}.", vname)) && method_body.contains("replaceAll") {
                    eprintln!("[DEBUG] Found replaceAll sanitization for variable: {}", vname);
                    return true;
                }
            }
        }
        
        false
    }

    /// Extract field/variable assignment target when source pattern matches an expression
    /// For example: "private static DocumentBuilderFactory dbf = DocumentBuilderFactory.newInstance();"
    /// When source matches "DocumentBuilderFactory.newInstance()", extracts "dbf"
    fn extract_field_assignment_target(&self, node: &dyn AstNode, source_text: &str, matched_text: &str) -> Option<String> {
        // Get the node's location
        if let Some((start_line, _start_col, _end_line, _end_col)) = node.location() {
            let lines: Vec<&str> = source_text.lines().collect();
            if start_line > 0 && start_line <= lines.len() {
                let line_text = lines[start_line - 1];
                
                // Check if this line contains an assignment with our matched expression
                if let Some(expr_pos) = line_text.find(matched_text) {
                    let before_expr = &line_text[..expr_pos];
                    
                    // Look for assignment operator before the expression
                    if let Some(eq_pos) = before_expr.rfind('=') {
                        let target = &before_expr[..eq_pos].trim();
                        
                        // Extract the variable/field name
                        // Handle: "Type var", "static Type var", "private static Type var", etc.
                        let parts: Vec<&str> = target.split_whitespace().collect();
                        if let Some(last_part) = parts.last() {
                            let var_name = last_part.trim().to_string();
                            // Remove any trailing characters
                            let var_name = var_name.trim_end_matches(|c: char| c == ';' || c == ' ').to_string();
                            
                            if !var_name.is_empty() && !var_name.contains("(") && !var_name.contains("=") {
                                eprintln!("[DEBUG] Extracted field assignment target: '{}'", var_name);
                                return Some(var_name);
                            }
                        }
                    }
                }
                
                // Also check for static initialization block pattern
                // static { dbf = DocumentBuilderFactory.newInstance(); }
                if line_text.contains("static") && line_text.contains('{') {
                    // This might be a static block, the assignment could be in this or next lines
                    // For simplicity, check if the matched expression is in a line with assignment
                    for (i, line) in source_text.lines().enumerate() {
                        if i >= start_line.saturating_sub(3) && i <= start_line + 1 {
                            if line.contains(matched_text) && line.contains('=') {
                                if let Some(eq_pos) = line.find('=') {
                                    let before_eq = &line[..eq_pos].trim();
                                    let parts: Vec<&str> = before_eq.split_whitespace().collect();
                                    if let Some(last_part) = parts.last() {
                                        let var_name = last_part.trim().to_string();
                                        if !var_name.is_empty() && !var_name.contains("(") {
                                            eprintln!("[DEBUG] Extracted static block assignment target: '{}'", var_name);
                                            return Some(var_name);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        None
    }

    /// Extract variable name from assignment text directly
    /// For text like "private static DocumentBuilderFactory dbf = DocumentBuilderFactory.newInstance();"
    fn extract_var_from_assignment_text(&self, text: &str) -> Option<String> {
        // Remove trailing semicolon
        let text = text.trim_end_matches(';').trim();
        
        // Pattern: [modifiers] Type var = expr
        // Find the '=' sign
        if let Some(eq_pos) = text.find('=') {
            let before_eq = &text[..eq_pos].trim();
            
            // Split by whitespace and get the last part (the variable name)
            let parts: Vec<&str> = before_eq.split_whitespace().collect();
            if let Some(last_part) = parts.last() {
                let var_name = last_part.trim().to_string();
                if !var_name.is_empty() 
                    && !var_name.contains("(") 
                    && !var_name.contains("=")
                    && !var_name.contains("<")  // Avoid generics like Map<String>
                    && var_name.chars().next().map(|c| c.is_alphabetic() || c == '_').unwrap_or(false) {
                    eprintln!("[DEBUG] Extracted var from assignment text: '{}'", var_name);
                    return Some(var_name);
                }
            }
        }
        
        None
    }

    /// Extract the target variable/field from an assignment statement
    /// When source matches "tainted" in "x = tainted" or "this.x = tainted", extract "x"
    fn extract_assignment_target(&self, node: &dyn AstNode, source_text: &str) -> Option<String> {
        let node_text = node.text().unwrap_or_default();
        
        // Get the node's location
        if let Some((start_line, _start_col, _end_line, _end_col)) = node.location() {
            let lines: Vec<&str> = source_text.lines().collect();
            if start_line > 0 && start_line <= lines.len() {
                let line_text = lines[start_line - 1];
                
                // Find the position of "tainted" in the line
                if let Some(tainted_pos) = line_text.find("tainted") {
                    let before_tainted = &line_text[..tainted_pos];
                    
                    // Look for assignment operator before tainted
                    if let Some(eq_pos) = before_tainted.rfind('=') {
                        let target = &before_tainted[..eq_pos].trim();
                        
                        // Extract the variable/field name
                        // Handle: "x", "this.x", "Type x", etc.
                        let parts: Vec<&str> = target.split_whitespace().collect();
                        if let Some(last_part) = parts.last() {
                            let var_name = last_part.trim().to_string();
                            // Remove "this." prefix if present
                            let var_name = if var_name.starts_with("this.") {
                                var_name[5..].to_string()
                            } else {
                                var_name
                            };
                            
                            if !var_name.is_empty() && !var_name.contains("(") && !var_name.contains("=") {
                                eprintln!("[DEBUG] Extracted assignment target: '{}'", var_name);
                                return Some(var_name);
                            }
                        }
                    }
                }
            }
        }
        
        None
    }

    /// Extract the setter argument when source is inside a setter call
    /// For example: obj.setX(tainted) -> returns "tainted" with field info
    fn extract_setter_argument(&self, line_num: usize, source_text: &str) -> Option<String> {
        let lines: Vec<&str> = source_text.lines().collect();
        if line_num == 0 || line_num > lines.len() {
            return None;
        }
        
        let line_text = lines[line_num - 1];
        
        // Check if this line contains a setter call pattern: obj.setX(tainted)
        if let Some(set_pos) = line_text.find(".set") {
            if let Some(paren_pos) = line_text[set_pos..].find('(') {
                // Extract object name
                let before_set = &line_text[..set_pos];
                let parts: Vec<&str> = before_set.split_whitespace().collect();
                if let Some(obj_name) = parts.last() {
                    let obj_name = obj_name.trim();
                    
                    // Extract field name from setter (setX -> x)
                    let after_set = &line_text[set_pos + 4..set_pos + paren_pos];
                    let field_name = after_set.to_lowercase();
                    
                    // Check if tainted is inside the parentheses
                    let args_start = set_pos + paren_pos + 1;
                    if let Some(close_paren) = line_text[args_start..].find(')') {
                        let args = &line_text[args_start..args_start + close_paren];
                        if args.contains("tainted") {
                            // Return field reference like "e.x"
                            let field_ref = format!("{}.{}", obj_name, field_name);
                            eprintln!("[DEBUG] Extracted setter argument as field ref: '{}'", field_ref);
                            return Some(field_ref);
                        }
                    }
                }
            }
        }
        
        None
    }

    /// Check if a parameter is of a numeric type
    fn is_numeric_parameter(&self, node: &dyn AstNode, param_name: &str) -> bool {
        // Check node text for method signature pattern
        let node_text = node.text().unwrap_or_default();
        
        // Look for method parameter declarations in the text
        // Pattern: "type paramName" inside parentheses of a method declaration
        // Examples: "int x", "long y", "String s", "Object o"
        if let Some(paren_start) = node_text.find('(') {
            if let Some(paren_end) = node_text.find(')') {
                let params_section = &node_text[paren_start..=paren_end];
                
                // Check if the parameter name appears in the parameter list
                if params_section.contains(param_name) {
                    // Extract the type for this parameter
                    // Pattern: "type paramName" or "type paramName," or ", type paramName"
                    let param_patterns = [
                        format!("int {}", param_name),
                        format!("long {}", param_name),
                        format!("short {}", param_name),
                        format!("byte {}", param_name),
                        format!("float {}", param_name),
                        format!("double {}", param_name),
                        format!("Integer {}", param_name),
                        format!("Long {}", param_name),
                        format!("Short {}", param_name),
                        format!("Byte {}", param_name),
                        format!("Float {}", param_name),
                        format!("Double {}", param_name),
                    ];
                    
                    for pattern in &param_patterns {
                        if params_section.contains(pattern) {
                            eprintln!("[DEBUG] Found numeric parameter: {}", pattern);
                            return true;
                        }
                    }
                }
            }
        }

        false
    }

    /// Find all taint sinks matching the sink patterns
    fn find_taint_sinks(
        &mut self,
        ast: &dyn AstNode,
        dataflow_spec: &DataFlowSpec,
        source_text: &str
    ) -> Result<Vec<TaintMatch>> {
        let mut sinks = Vec::new();
        
        for sink_pattern in &dataflow_spec.sinks {
            // Normalize pattern: remove trailing semicolons and whitespace for more flexible matching
            let original_pattern = sink_pattern.pattern_text();
            let normalized_pattern = original_pattern.trim_end_matches(';').trim_end_matches('\n').trim();
            eprintln!("[DEBUG] Normalizing sink pattern: '{:?}' -> '{}'", original_pattern, normalized_pattern);

            // Convert sink pattern to SemgrepPattern
            let semgrep_pattern = astgrep_core::SemgrepPattern {
                pattern_type: astgrep_core::PatternType::Simple(normalized_pattern.to_string()),
                metavariable_pattern: None,
                conditions: Vec::new(),
                focus: None,
            };
            
            // Find matches
            let mut matches = self.pattern_matcher.find_matches(&semgrep_pattern, ast)?;
            
            // If no matches and pattern looks like a fully qualified name, try matching just the class and method
            if matches.is_empty() && normalized_pattern.contains('.') {
                if let Some(simplified) = Self::simplify_fully_qualified_pattern(normalized_pattern) {
                    eprintln!("[DEBUG] No matches with full sink pattern, trying simplified: '{}'", simplified);
                    let simplified_semgrep_pattern = astgrep_core::SemgrepPattern {
                        pattern_type: astgrep_core::PatternType::Simple(simplified),
                        metavariable_pattern: None,
                        conditions: Vec::new(),
                        focus: None,
                    };
                    matches = self.pattern_matcher.find_matches(&simplified_semgrep_pattern, ast)?;
                }
            }
            
            eprintln!("[DEBUG] Sink matches found: {}", matches.len());
            for m in matches {
                eprintln!("[DEBUG] Sink match: bindings={:?}, text={:?}", m.bindings, m.node.text());
                
                // Extract method name for scope isolation using source location
                let node = m.node.as_ref();
                let method_name = if let Some((start_line, _, _, _)) = node.location() {
                    self.find_method_name_by_line(source_text, start_line)
                } else {
                    None
                };
                
                // Extract variable name from focus-metavariable if specified
                let mut var_name = None;
                
                // First, check if the sink pattern has focus_metavariables defined
                // and try to extract the value from the match bindings
                if !sink_pattern.focus_metavariables.is_empty() {
                    for focus_var in &sink_pattern.focus_metavariables {
                        // Try with the $ prefix first (e.g., "$SQL")
                        if let Some(value) = m.bindings.get(focus_var) {
                            eprintln!("[DEBUG] Extracted var_name from focus-metavariable '{}': {}", focus_var, value);
                            var_name = Some(value.clone());
                            break;
                        }
                        // Also try without the $ prefix (e.g., "SQL")
                        let focus_var_no_dollar = focus_var.trim_start_matches('$');
                        if let Some(value) = m.bindings.get(focus_var_no_dollar) {
                            eprintln!("[DEBUG] Extracted var_name from focus-metavariable '{}': {}", focus_var, value);
                            var_name = Some(value.clone());
                            break;
                        }
                    }
                }
                
                // If no var_name from focus-metavariables, check bindings for any metavariable
                if var_name.is_none() {
                    for (key, value) in &m.bindings {
                        if key.starts_with("$") || !key.chars().next().map(|c| c.is_ascii_lowercase()).unwrap_or(false) {
                            // This is likely a metavariable binding
                            var_name = Some(value.clone());
                            break;
                        }
                    }
                }
                
                // If we have a focus-metavariable but the extracted value looks incomplete
                // (e.g., "toString" instead of "sqlBuilder.toString()"), try to extract
                // the full first argument from the method call
                if var_name.is_some() && sink_pattern.focus_metavariables.is_empty() == false {
                    if let Some(ref v) = var_name {
                        // If the extracted value is just a simple identifier that could be a method name
                        // and doesn't contain a dot (receiver), try to get the full argument
                        if !v.contains('.') && !v.contains('(') {
                            if let Some(text) = m.node.text() {
                                if let Some(args) = Self::extract_last_call_args(text.trim()) {
                                    let arg_parts: Vec<&str> = args.split(',').collect();
                                    if !arg_parts.is_empty() {
                                        let first_arg = arg_parts[0].trim();
                                        if first_arg.contains(v) && first_arg != *v {
                                            eprintln!("[DEBUG] Replacing incomplete var_name '{}' with full argument '{}'", v, first_arg);
                                            var_name = Some(first_arg.to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                
                // If no var_name from bindings, try to extract from sink call argument
                // e.g., sink(w) -> extract "w"
                if var_name.is_none() {
                    if let Some(text) = m.node.text() {
                        let text = text.trim();
                        // Pattern: sink(arg), Runtime.getRuntime().exec(arg), etc.
                        // Find the matching opening paren for the last closing paren
                        // This handles nested calls like sink(obj.getX())
                        if let Some(args) = Self::extract_last_call_args(text) {
                            // For simple case sink(w), extract w
                            // For sink1("Abc", w), extract w (second argument)
                            let arg_parts: Vec<&str> = args.split(',').collect();
                            if arg_parts.len() == 1 {
                                // Single argument: sink(w)
                                let arg = arg_parts[0].trim();
                                if !arg.is_empty() && !arg.contains('"') && !arg.contains('\'') {
                                    var_name = Some(arg.to_string());
                                }
                            } else if arg_parts.len() >= 2 {
                                // Multiple arguments: sink1("...", w), take the last one
                                let last_arg = arg_parts.last().unwrap().trim();
                                if !last_arg.is_empty() && !last_arg.contains('"') && !last_arg.contains('\'') {
                                    var_name = Some(last_arg.to_string());
                                }
                            }
                        }
                    }
                }
                
                eprintln!("[DEBUG] Creating sink TaintMatch: var_name={:?}, method_name={:?}", var_name, method_name);
                
                // Check if this sink is in a method that contains sanitization
                // For example: applicationJdbcTemplate.query() followed by setString()
                if let Some(ref mname) = method_name {
                    if let Some(method_body) = self.extract_method_body(source_text, mname) {
                        // Check if method contains .setString() or other sanitization patterns
                        if self.contains_sanitization_in_scope(&method_body, var_name.as_deref()) {
                            eprintln!("[DEBUG] Skipping sink in sanitized method: {}", mname);
                            continue;
                        }
                    }
                }
                
                sinks.push(TaintMatch {
                    node: m.node,
                    bindings: m.bindings,
                    var_name,
                    method_name,
                });
            }
        }
        
        Ok(sinks)
    }
    
    /// Detect taint flows from sources to sinks
    fn detect_taint_flows(
        &self,
        sources: &[TaintMatch],
        sinks: &[TaintMatch],
        ast: &dyn AstNode,
        dataflow_analysis: Option<&DataFlowAnalysis>,
        taint_assume_safe_booleans: bool,
        taint_assume_safe_numbers: bool,
        taint_only_propagate_through_assignments: bool,
        source_text: &str,
        propagators: &[crate::types::PropagatorPattern],
    ) -> Result<Vec<(TaintMatch, TaintMatch)>> {
        eprintln!("[DEBUG] detect_taint_flows: {} sources, {} sinks, assume_safe_booleans={}, assume_safe_numbers={}, only_assignments={}",
                  sources.len(), sinks.len(), taint_assume_safe_booleans, taint_assume_safe_numbers, taint_only_propagate_through_assignments);
        let mut flows = Vec::new();

        // Build method cache for dependency graphs
        let mut method_cache: HashMap<Option<String>, VariableDependencyGraph> = HashMap::new();

        // Use simple heuristics to detect taint flows
        for (i, source) in sources.iter().enumerate() {
            eprintln!("[DEBUG] Checking source {}: var_name={:?}, method={:?}", i, source.var_name, source.method_name);
            if let Some(ref source_var) = source.var_name {
                for (j, sink) in sinks.iter().enumerate() {
                    eprintln!("[DEBUG] Checking source {} with sink {}: var='{}' vs sink text='{}', source_method={:?}, sink_method={:?}",
                              i, j, source_var, sink.node.text().unwrap_or_default(), source.method_name, sink.method_name);
                    
                    // Method-level scope isolation: if both source and sink have method names,
                    // only pair them if they're in the same method
                    if let (Some(ref src_method), Some(ref sink_method)) = (&source.method_name, &sink.method_name) {
                        if src_method != sink_method {
                            eprintln!("[DEBUG] Skipping: source and sink are in different methods ({} vs {})", src_method, sink_method);
                            continue;
                        }
                    }
                    
                    // Check if source variable appears in sink context
                    if self.is_variable_flowing_to_sink(source_var, sink.node.as_ref(), ast, taint_assume_safe_booleans, taint_assume_safe_numbers, taint_only_propagate_through_assignments, source_text) {
                        eprintln!("[DEBUG] FLOW FOUND: source {} -> sink {}", i, j);
                        flows.push((source.clone(), sink.clone()));
                        continue;
                    }

                    // Check dataflow: if sink variable depends on source variable
                    eprintln!("[DEBUG] Checking dataflow analysis: sink.var_name={:?}, sink.method_name={:?}", sink.var_name, sink.method_name);
                    if let (Some(ref sink_var), Some(ref method_name)) = (&sink.var_name, &sink.method_name) {
                        eprintln!("[DEBUG] Entering dataflow analysis: sink_var={}, method_name={}", sink_var, method_name);
                        // Build or get dependency graph for this method
                        let dep_graph = method_cache.entry(Some(method_name.clone())).or_insert_with(|| {
                            let graph = VariableDependencyGraph::new()
                                .with_propagators(propagators.to_vec());
                            // Extract method body and build dependency graph
                            eprintln!("[DEBUG] Extracting method body for: {}", method_name);
                            if let Some(method_body) = self.extract_method_body(source_text, method_name) {
                                eprintln!("[DEBUG] Building dependency graph for method: {} (body length: {})", method_name, method_body.len());
                                let mut graph = graph;
                                graph.build_from_method(&method_body);
                                eprintln!("[DEBUG] Dependency graph built. Assignments: {:?}", graph.assignments.keys().collect::<Vec<_>>());
                                graph
                            } else {
                                eprintln!("[DEBUG] Failed to extract method body for: {}", method_name);
                                graph
                            }
                        });

                        // Check if sink variable depends on source variable
                        // When taint_assume_safe_numbers is true, check for safe numeric context
                        let check_safe_context = taint_assume_safe_numbers;
                        
                        // Also check if the sink is accessing a numeric field
                        if taint_assume_safe_numbers {
                            if let Some(sink_field) = self.extract_field_from_sink(&sink.node.text().unwrap_or_default()) {
                                if self.is_numeric_field(&sink_field, source_text) {
                                    eprintln!("[DEBUG] Skipping dataflow: sink field '{}' is numeric (taint_assume_safe_numbers)", sink_field);
                                    continue;
                                }
                            }
                        }
                        
                        eprintln!("[DEBUG] Checking dependency: {} depends on {} (check_safe={})", sink_var, source_var, check_safe_context);
                        if dep_graph.depends_on(sink_var, &[source_var.clone()], check_safe_context) {
                            eprintln!("[DEBUG] FLOW FOUND (dataflow): source {} -> sink {} ({} depends on {})", i, j, sink_var, source_var);
                            flows.push((source.clone(), sink.clone()));
                        } else {
                            eprintln!("[DEBUG] No dependency found: {} does not depend on {}", sink_var, source_var);
                        }
                    } else {
                        eprintln!("[DEBUG] Skipping dataflow analysis: sink.var_name or sink.method_name is None");
                    }
                }
            } else {
                // Source has no var_name - might be a string literal pattern or method parameter with annotation
                eprintln!("[DEBUG] Source {} has no var_name, checking for string literal assignments and method parameters", i);
                
                for (j, sink) in sinks.iter().enumerate() {
                    if let (Some(ref sink_var), Some(ref method_name)) = (&sink.var_name, &sink.method_name) {
                        // Check 1: String literal assignments
                        let dep_graph = method_cache.entry(Some(method_name.clone())).or_insert_with(|| {
                            let graph = VariableDependencyGraph::new()
                                .with_propagators(propagators.to_vec());
                            if let Some(method_body) = self.extract_method_body(source_text, method_name) {
                                let mut graph = graph;
                                graph.build_from_method(&method_body);
                                graph
                            } else {
                                graph
                            }
                        });
                        
                        if dep_graph.is_assigned_string_literal(sink_var) ||
                           dep_graph.has_string_literal_in_dependency_chain(sink_var) {
                            eprintln!("[DEBUG] FLOW FOUND (string literal): source {} -> sink {} ({} is assigned string literal)", i, j, sink_var);
                            flows.push((source.clone(), sink.clone()));
                            continue;
                        }
                        
                        // Check 2: Method parameter with taint annotation
                        // If sink variable is a method parameter with @RequestParam, @PathVariable, etc., it's tainted
                        if self.is_tainted_method_parameter(source_text, method_name, sink_var) {
                            eprintln!("[DEBUG] FLOW FOUND (tainted parameter): source {} -> sink {} ({} is a tainted method parameter)", i, j, sink_var);
                            flows.push((source.clone(), sink.clone()));
                        }
                    }
                }
            }
        }

        // Also check dataflow analysis results if available
        if let Some(analysis) = dataflow_analysis {
            for flow in &analysis.taint_flows {
                if flow.is_vulnerable() {
                    // Find corresponding source and sink matches
                    for source in sources {
                        for sink in sinks {
                            if self.is_flow_matching(&flow, source, sink) {
                                flows.push((source.clone(), sink.clone()));
                            }
                        }
                    }
                }
            }
        }
        
        Ok(flows)
    }

    /// Check if a variable is a method parameter with taint-related annotations
    /// This handles cases like @RequestParam, @PathVariable, @RequestBody, etc.
    fn is_tainted_method_parameter(&self, source_text: &str, method_name: &str, var_name: &str) -> bool {
        // Find the method declaration line
        let lines: Vec<&str> = source_text.lines().collect();
        
        for (i, line) in lines.iter().enumerate() {
            // Look for method declaration with the given method name
            if line.contains(&format!("{}", method_name)) && line.contains('(') && line.contains(')') {
                // Check if this line or previous lines have taint-related annotations
                let method_line = line;
                
                // Check the method signature for the parameter
                if let Some(paren_start) = method_line.find('(') {
                    if let Some(paren_end) = method_line.rfind(')') {
                        let params_section = &method_line[paren_start..=paren_end];
                        
                        // Split parameters and check each one
                        let params: Vec<&str> = params_section[1..params_section.len()-1].split(',').collect();
                        for param in params {
                            let param = param.trim();
                            // Check if this parameter matches our variable name
                            let param_words: Vec<&str> = param.split_whitespace().collect();
                            if let Some(last_word) = param_words.last() {
                                if *last_word == var_name {
                                    // Found the parameter, now check for taint annotations
                                    // Look at current line and previous lines for annotations
                                    let start_check = if i > 3 { i - 3 } else { 0 };
                                    for j in start_check..=i {
                                        let check_line = lines[j];
                                        // Check for taint-related annotations
                                        if check_line.contains("@RequestParam") 
                                            || check_line.contains("@PathVariable")
                                            || check_line.contains("@RequestBody")
                                            || check_line.contains("@RequestHeader")
                                            || check_line.contains("@CookieValue") {
                                            // Check if this annotation is associated with our parameter
                                            // The annotation should be close to the parameter
                                            if j == i || (j < i && lines[j+1..=i].join(" ").contains(var_name)) {
                                                eprintln!("[DEBUG] Found tainted parameter: {} with annotation in method {}", var_name, method_name);
                                                return true;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        false
    }

    /// Extract method body by method name from source text
    fn extract_method_body(&self, source_text: &str, method_name: &str) -> Option<String> {
        // Find the method declaration
        let method_pattern = format!(r"public\s+(?:static\s+)?(?:\w+)\s+{}\s*\([^{{]*\{{", regex::escape(method_name));
        if let Ok(re) = regex::Regex::new(&method_pattern) {
            if let Some(mat) = re.find(source_text) {
                let start = mat.start();
                // Find matching closing brace
                let mut brace_count = 0;
                let mut in_method = false;
                let mut end = start;
                
                for (i, c) in source_text[start..].chars().enumerate() {
                    if c == '{' {
                        brace_count += 1;
                        in_method = true;
                    } else if c == '}' {
                        brace_count -= 1;
                        if in_method && brace_count == 0 {
                            end = start + i + 1;
                            break;
                        }
                    }
                }
                
                return Some(source_text[start..end].to_string());
            }
        }
        
        None
    }
    
    /// Check if a node uses any of the given variables
    fn node_uses_variables(&self, node: &dyn AstNode, variables: &[String]) -> bool {
        let node_text = node.text().unwrap_or_default();
        for var in variables {
            if node_text.contains(var) {
                return true;
            }
        }
        false
    }
    
    /// Check if a variable flows to a sink, using symbolic propagation for alias tracking
    fn is_variable_flowing_to_sink(
        &self,
        var_name: &str,
        sink_node: &dyn AstNode,
        _ast: &dyn AstNode,
        taint_assume_safe_booleans: bool,
        taint_assume_safe_numbers: bool,
        taint_only_propagate_through_assignments: bool,
        source_text: &str,
    ) -> bool {
        let sink_text = sink_node.text().unwrap_or_default();

        // When taint_assume_safe_booleans is true, check if the variable is used in safe boolean contexts
        if taint_assume_safe_booleans && self.is_variable_in_safe_boolean_context(var_name, &sink_text) {
            eprintln!("[DEBUG] Variable '{}' in safe boolean context, not flowing", var_name);
            return false;
        }

        // When taint_assume_safe_numbers is true, check if the variable is used in safe numeric contexts
        if taint_assume_safe_numbers && self.is_variable_in_safe_number_context(var_name, &sink_text) {
            eprintln!("[DEBUG] Variable '{}' in safe number context, not flowing", var_name);
            return false;
        }

        // When taint_assume_safe_numbers is true, also check if the sink is accessing a numeric field
        // This handles cases like "sink(this.y)" where "y" is an "int" field
        if taint_assume_safe_numbers {
            if let Some(sink_field) = self.extract_field_from_sink(&sink_text) {
                if self.is_numeric_field(&sink_field, source_text) {
                    eprintln!("[DEBUG] Sink field '{}' is numeric, not flowing (taint_assume_safe_numbers)", sink_field);
                    return false;
                }
            }
        }

        // When taint_only_propagate_through_assignments is true, check if there's a direct assignment chain
        if taint_only_propagate_through_assignments {
            if !self.is_direct_assignment_chain(var_name, &sink_text) {
                eprintln!("[DEBUG] Variable '{}' not flowing through direct assignment chain", var_name);
                return false;
            }
        }

        // Check if source variable directly appears in sink node
        if sink_text.contains(var_name) {
            return true;
        }

        // Handle field access normalization: "this.x" and "x" should be treated as the same field
        // Case 1: var_name is "x", sink contains "this.x"
        let field_access_pattern = format!("this.{}", var_name);
        if sink_text.contains(&field_access_pattern) {
            return true;
        }
        
        // Case 2: var_name is "this.x", sink contains "x"
        if var_name.starts_with("this.") {
            let field_name = &var_name[5..]; // Remove "this." prefix
            // Check if the field name appears as a standalone variable in the sink
            // We need to be careful to match whole words only
            if self.contains_whole_word(&sink_text, field_name) {
                return true;
            }
        }

        // Use symbolic propagator to check for aliases
        if let Some(ref propagator) = self.symbolic_propagator {
            // Get all aliases of the source variable
            let aliases = propagator.state().get_all_aliases(var_name);
            
            // Check if any alias appears in the sink
            for alias in &aliases {
                if sink_text.contains(alias) {
                    return true;
                }
            }
            
            // Also check if the source variable is derived from any variable in the sink
            // This handles cases like: dbf.newDocumentBuilder() where dbf is a field
            if let Some(sink_var) = self.extract_receiver_from_sink(sink_node) {
                // Check if sink_var is an alias of var_name
                if propagator.state().is_alias(var_name, &sink_var) {
                    return true;
                }
                // Check if sink_var equals var_name
                if sink_var == var_name {
                    return true;
                }
            }
        }
        
        false
    }

    /// Check if a variable is used in a safe boolean context
    /// Returns true if the variable is used in a way that doesn't propagate taint
    /// (e.g., Boolean.valueOf(var), var != "safe", etc.)
    fn is_variable_in_safe_boolean_context(&self, var_name: &str, sink_text: &str) -> bool {
        // Pattern 1: Boolean conversion functions
        if sink_text.contains(&format!("Boolean.valueOf({})", var_name)) {
            return true;
        }
        if sink_text.contains(&format!("Boolean.parseBoolean({})", var_name)) {
            return true;
        }

        // Pattern 2: Boolean comparison operators
        // Check for patterns like "var != something" or "var == something"
        // We need to make sure var is actually the variable being compared, not part of a string
        let comparison_patterns = [
            format!("{} != ", var_name),
            format!("{} == ", var_name),
            format!("{} > ", var_name),
            format!("{} < ", var_name),
            format!("{} >= ", var_name),
            format!("{} <= ", var_name),
            format!(" {}!= ", var_name),
            format!(" {}== ", var_name),
            format!(" {}> ", var_name),
            format!(" {}< ", var_name),
            format!(" {}>= ", var_name),
            format!(" {}<= ", var_name),
        ];

        for pattern in &comparison_patterns {
            if sink_text.contains(pattern) {
                return true;
            }
        }

        // Pattern 3: More complex boolean expressions using parentheses
        // Like "(x != "safe")"
        let paren_patterns = [
            format!("({} != ", var_name),
            format!("({} == ", var_name),
            format!("({} > ", var_name),
            format!("({} < ", var_name),
        ];

        for pattern in &paren_patterns {
            if sink_text.contains(pattern) {
                return true;
            }
        }

        false
    }

    /// Returns true if the variable is used in a numeric context that doesn't propagate taint
    /// (e.g., Integer.valueOf(var), var.length, comparison operations, etc.)
    fn is_variable_in_safe_number_context(&self, var_name: &str, sink_text: &str) -> bool {
        // Pattern 1: Numeric wrapper class conversion functions
        let numeric_conversions = [
            format!("Integer.valueOf({})", var_name),
            format!("Integer.parseInt({})", var_name),
            format!("Long.valueOf({})", var_name),
            format!("Long.parseLong({})", var_name),
            format!("Short.valueOf({})", var_name),
            format!("Short.parseShort({})", var_name),
            format!("Double.valueOf({})", var_name),
            format!("Double.parseDouble({})", var_name),
            format!("Float.valueOf({})", var_name),
            format!("Float.parseFloat({})", var_name),
        ];
        for pattern in &numeric_conversions {
            if sink_text.contains(pattern) {
                return true;
            }
        }

        // Pattern 2: String comparison operations that return integers
        // e.g., "var.compareTo()", "var.indexOf()", "var.lastIndexOf()"
        let string_methods_returning_int = [
            format!("{}.compareTo(", var_name),
            format!("{}.indexOf(", var_name),
            format!("{}.lastIndexOf(", var_name),
            format!("{}.length()", var_name),
        ];
        for pattern in &string_methods_returning_int {
            if sink_text.contains(pattern) {
                return true;
            }
        }

        // Pattern 3: Array length access
        if sink_text.contains(&format!("{}.length", var_name)) {
            return true;
        }

        // Pattern 4: Numeric comparison operators (these return booleans, safe for numeric taint)
        let comparison_patterns = [
            format!("{} != ", var_name),
            format!("{} == ", var_name),
            format!("{} > ", var_name),
            format!("{} < ", var_name),
            format!("{} >= ", var_name),
            format!("{} <= ", var_name),
            format!(" {}!= ", var_name),
            format!(" {}== ", var_name),
            format!(" {}> ", var_name),
            format!(" {}< ", var_name),
            format!(" {}>= ", var_name),
            format!(" {}<= ", var_name),
        ];
        for pattern in &comparison_patterns {
            if sink_text.contains(pattern) {
                return true;
            }
        }

        false
    }

    /// Returns true if there's a direct assignment chain from source to sink
    /// When taint_only_propagate_through_assignments is true, we only consider
    /// taint that flows through direct assignments, not through function calls
    fn is_direct_assignment_chain(&self, var_name: &str, sink_text: &str) -> bool {
        // For now, we consider it a direct assignment if:
        // 1. The variable appears directly as an argument (simple case)
        // 2. The sink contains the variable in a non-complex expression
        
        // Check if the variable is used in a String.format or similar complex expression
        // These are NOT direct assignments
        if sink_text.contains(&format!("String.format(",)) && sink_text.contains(var_name) {
            // If the variable is inside String.format, it's not a direct assignment
            return false;
        }
        
        // If the variable appears directly, it's considered a direct assignment
        if sink_text.contains(var_name) {
            return true;
        }
        
        false
    }

    fn extract_receiver_from_sink(&self, sink_node: &dyn AstNode) -> Option<String> {
        let sink_text = sink_node.text().unwrap_or_default();
        
        // Pattern: receiver.methodName()
        // Extract the receiver part before the first dot
        if let Some(dot_pos) = sink_text.find('.') {
            let receiver = sink_text[..dot_pos].trim();
            if !receiver.is_empty() {
                return Some(receiver.to_string());
            }
        }
        
        // Try AST-based extraction for method_invocation or call_expression
        if sink_node.node_type() == "method_invocation" || sink_node.node_type() == "call_expression" {
            for i in 0..sink_node.child_count() {
                if let Some(child) = sink_node.child(i) {
                    if child.node_type() == "identifier" || child.node_type() == "field_access" {
                        if let Some(text) = child.text() {
                            return Some(text.to_string());
                        }
                    }
                }
            }
        }
        
        None
    }

    /// Extract field name from sink text if it's a field access pattern
    /// e.g., "sink(this.y)" -> Some("y"), "sink(obj.x)" -> Some("x")
    fn extract_field_from_sink(&self, sink_text: &str) -> Option<String> {
        // Look for patterns like "this.field" or "obj.field" inside the sink call
        // Pattern: sink(... this.field ...) or sink(... obj.field ...)
        if let Some(open_paren) = sink_text.find('(') {
            let args = &sink_text[open_paren + 1..];
            // Find field access patterns
            if let Some(dot_pos) = args.find('.') {
                let after_dot = &args[dot_pos + 1..];
                // Extract field name (until next non-identifier character)
                let field_name: String = after_dot
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                if !field_name.is_empty() {
                    return Some(field_name);
                }
            }
        }
        None
    }

    /// Check if a field is of numeric type by looking at class field declarations
    fn is_numeric_field(&self, field_name: &str, source_text: &str) -> bool {
        // Look for field declarations like: "int y;", "Integer x;", "long count;", etc.
        let numeric_types = [
            "int", "long", "short", "byte", "float", "double",
            "Integer", "Long", "Short", "Byte", "Float", "Double"
        ];

        for line in source_text.lines() {
            let line = line.trim();
            // Check for field declaration pattern: "Type fieldName;" or "Type fieldName = ..."
            for type_name in &numeric_types {
                let patterns = [
                    format!("{} {};", type_name, field_name),
                    format!("{} {} =", type_name, field_name),
                    format!("{} {}=", type_name, field_name),
                ];
                for pattern in &patterns {
                    if line.contains(pattern) {
                        eprintln!("[DEBUG] Found numeric field declaration: {}", line);
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Check if a string contains a whole word (not part of another word)
    fn contains_whole_word(&self, text: &str, word: &str) -> bool {
        // Simple heuristic: check for common delimiters around the word
        let delimiters = ['(', ')', ' ', ',', ';', '+', '-', '*', '/', '=', '<', '>', '!'];
        
        // Check if word appears at the start
        if text.starts_with(word) {
            if text.len() == word.len() || delimiters.contains(&text.chars().nth(word.len()).unwrap_or(' ')) {
                return true;
            }
        }
        
        // Check if word appears in the middle or end
        for (i, _) in text.match_indices(word) {
            let before = if i == 0 { ' ' } else { text.chars().nth(i - 1).unwrap_or(' ') };
            let after_pos = i + word.len();
            let after = if after_pos >= text.len() { ' ' } else { text.chars().nth(after_pos).unwrap_or(' ') };
            
            // Check if word is surrounded by delimiters or string boundaries
            let before_is_delimiter = i == 0 || delimiters.contains(&before);
            let after_is_delimiter = after_pos >= text.len() || delimiters.contains(&after);
            
            if before_is_delimiter && after_is_delimiter {
                return true;
            }
        }
        
        false
    }

    /// Check if a dataflow matches source and sink
    fn is_flow_matching(
        &self,
        flow: &astgrep_dataflow::TaintFlow,
        source: &TaintMatch,
        sink: &TaintMatch
    ) -> bool {
        // Check if flow source matches our source
        let source_matches = if let (Some(src_loc), Some(flow_loc)) = (source.node.location(), &flow.source.location) {
            src_loc.0 == flow_loc.start_line && src_loc.1 == flow_loc.start_column
        } else {
            // Fallback: compare by description text
            if let Some(source_text) = source.node.text() {
                flow.source.description.contains(&source_text) || source_text.contains(&flow.source.description)
            } else {
                false
            }
        };
        
        // Check if flow sink matches our sink
        let sink_matches = if let (Some(sink_loc), Some(flow_loc)) = (sink.node.location(), &flow.sink.location) {
            sink_loc.0 == flow_loc.start_line && sink_loc.1 == flow_loc.start_column
        } else {
            // Fallback: compare by description text
            if let Some(sink_text) = sink.node.text() {
                flow.sink.description.contains(&sink_text) || sink_text.contains(&flow.sink.description)
            } else {
                false
            }
        };
        
        source_matches && sink_matches
    }
    
    /// Extract arguments from the last function call in a chain
    /// e.g., "sink(e.getX())" -> Some("e.getX()")
    /// e.g., "Runtime.getRuntime().exec(nodeSucc)" -> Some("nodeSucc")
    fn extract_last_call_args(text: &str) -> Option<&str> {
        // Find the last closing paren
        let close_pos = text.rfind(')')?;
        
        // Find the matching opening paren by counting
        let mut paren_count = 1;
        let mut open_pos = None;
        
        for (i, c) in text[..close_pos].chars().rev().enumerate() {
            match c {
                ')' => paren_count += 1,
                '(' => {
                    paren_count -= 1;
                    if paren_count == 0 {
                        open_pos = Some(close_pos - i - 1);
                        break;
                    }
                }
                _ => {}
            }
        }
        
        open_pos.map(|pos| &text[pos + 1..close_pos])
    }

    /// Get execution statistics
    pub fn statistics(&self) -> &ExecutionStatistics {
        &self.execution_stats
    }

    /// Reset the executor
    pub fn reset(&mut self) {
        self.dataflow_analyzer.reset();
        self.execution_stats = ExecutionStatistics::new();
    }

    /// Convert astgrep_rules::Pattern to astgrep_core::SemgrepPattern
    fn convert_pattern_to_semgrep_pattern(&self, pattern: &Pattern) -> Result<astgrep_core::SemgrepPattern> {
        use astgrep_core::{SemgrepPattern, PatternType as CorePatternType};

        let core_pattern_type = match &pattern.pattern_type {
            crate::PatternType::Simple(pattern_str) => CorePatternType::Simple(pattern_str.clone()),
            crate::PatternType::Either(patterns) => {
                let converted: Result<Vec<_>> = patterns.iter()
                    .map(|p| self.convert_pattern_to_semgrep_pattern(p))
                    .collect();
                CorePatternType::Either(converted?)
            }
            crate::PatternType::Inside(inner_pattern) => {
                let converted = self.convert_pattern_to_semgrep_pattern(inner_pattern)?;
                CorePatternType::Inside(Box::new(converted))
            }
            crate::PatternType::NotInside(inner_pattern) => {
                let converted = self.convert_pattern_to_semgrep_pattern(inner_pattern)?;
                CorePatternType::NotInside(Box::new(converted))
            }
            crate::PatternType::Not(inner_pattern) => {
                let converted = self.convert_pattern_to_semgrep_pattern(inner_pattern)?;
                CorePatternType::Not(Box::new(converted))
            }
            crate::PatternType::Regex(regex) => CorePatternType::Regex(regex.clone()),
            crate::PatternType::NotRegex(regex) => CorePatternType::NotRegex(regex.clone()),
            crate::PatternType::All(patterns) => {
                let converted: Result<Vec<_>> = patterns.iter()
                    .map(|p| self.convert_pattern_to_semgrep_pattern(p))
                    .collect();
                CorePatternType::All(converted?)
            }
            crate::PatternType::Any(patterns) => {
                let converted: Result<Vec<_>> = patterns.iter()
                    .map(|p| self.convert_pattern_to_semgrep_pattern(p))
                    .collect();
                CorePatternType::Any(converted?)
            }
        };

        // Convert conditions
        let conditions: Vec<astgrep_core::Condition> = pattern.conditions.iter()
            .map(|cond| self.convert_condition_to_core(cond))
            .collect::<Result<Vec<_>>>()?;

        Ok(SemgrepPattern {
            pattern_type: core_pattern_type,
            metavariable_pattern: None, // TODO: Convert metavariable patterns
            conditions,
            focus: pattern.focus.clone(),
        })
    }

    /// Convert astgrep_rules::Condition to astgrep_core::Condition
    fn convert_condition_to_core(&self, condition: &Condition) -> Result<astgrep_core::Condition> {
        use astgrep_core::{Condition as CoreCondition, MetavariableComparison as CoreMetavariableComparison, ComparisonOperator as CoreComparisonOperator};
        use astgrep_core::{MetavariableRegex as CoreMetavariableRegex, MetavariableName as CoreMetavariableName, MetavariableAnalysisCondition as CoreMetavariableAnalysisCondition, MetavariableType as CoreMetavariableType};

        match condition {
            Condition::MetavariableRegex(metavar_regex) => {
                // Convert MetavariableRegex to core Condition
                let core_regex = CoreMetavariableRegex {
                    metavariable: metavar_regex.metavariable.clone(),
                    regex: metavar_regex.regex.clone(),
                };
                Ok(CoreCondition::MetavariableRegex(core_regex))
            }
            Condition::MetavariableComparison(metavar_comp) => {
                // Convert MetavariableComparison to core Condition
                let core_comp = CoreMetavariableComparison {
                    metavariable: metavar_comp.metavariable.clone(),
                    operator: match &metavar_comp.operator {
                        ComparisonOperator::Equals => CoreComparisonOperator::Equals,
                        ComparisonOperator::NotEquals => CoreComparisonOperator::NotEquals,
                        ComparisonOperator::Contains => CoreComparisonOperator::Contains,
                        ComparisonOperator::StartsWith => CoreComparisonOperator::StartsWith,
                        ComparisonOperator::EndsWith => CoreComparisonOperator::EndsWith,
                        ComparisonOperator::Matches => CoreComparisonOperator::Matches,
                        ComparisonOperator::GreaterThan => CoreComparisonOperator::GreaterThan,
                        ComparisonOperator::LessThan => CoreComparisonOperator::LessThan,
                        ComparisonOperator::PythonExpression(expr) => CoreComparisonOperator::PythonExpression(expr.clone()),
                    },
                    value: metavar_comp.value.clone(),
                };
                Ok(CoreCondition::MetavariableComparison(core_comp))
            }
            Condition::MetavariableName(metavar_name) => {
                // Convert MetavariableName to core Condition
                let core_name = CoreMetavariableName {
                    metavariable: metavar_name.metavariable.clone(),
                    name_pattern: metavar_name.name_pattern.clone(),
                };
                Ok(CoreCondition::MetavariableName(core_name))
            }
            Condition::MetavariableAnalysis(metavar_analysis) => {
                // Convert MetavariableAnalysis to core Condition
                let core_analysis = CoreMetavariableAnalysisCondition {
                    metavariable: metavar_analysis.metavariable.clone(),
                    analysis: metavar_analysis.analysis.clone(),
                };
                Ok(CoreCondition::MetavariableAnalysis(core_analysis))
            }
            Condition::NodeType(node_type) => {
                Ok(CoreCondition::NodeType(node_type.clone()))
            }
            Condition::NodeAttribute(attr_name, attr_value) => {
                Ok(CoreCondition::NodeAttribute(attr_name.clone(), attr_value.clone()))
            }
            Condition::MetavariableType(metavar_type) => {
                // Convert MetavariableType to core Condition
                let core_type = CoreMetavariableType {
                    metavariable: metavar_type.metavariable.clone(),
                    var_type: metavar_type.var_type.clone(),
                };
                Ok(CoreCondition::MetavariableType(core_type))
            }
            Condition::Custom(custom) => {
                Ok(CoreCondition::Custom(custom.clone()))
            }
        }
    }
}

impl Default for AdvancedRuleExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// Comprehensive analysis result
#[derive(Debug, Clone)]
pub struct ComprehensiveAnalysisResult {
    pub findings: Vec<Finding>,
    pub rule_results: Vec<RuleExecutionResult>,
    pub dataflow_analysis: Option<DataFlowAnalysis>,
    pub execution_time: std::time::Duration,
    pub statistics: ExecutionStatistics,
}

impl ComprehensiveAnalysisResult {
    fn empty(execution_time: std::time::Duration) -> Self {
        Self {
            findings: Vec::new(),
            rule_results: Vec::new(),
            dataflow_analysis: None,
            execution_time,
            statistics: ExecutionStatistics::new(),
        }
    }

    /// Get findings by severity
    pub fn findings_by_severity(&self, severity: Severity) -> Vec<&Finding> {
        self.findings.iter().filter(|f| f.severity == severity).collect()
    }

    /// Check if analysis found any critical issues
    pub fn has_critical_findings(&self) -> bool {
        self.findings.iter().any(|f| f.severity == Severity::Error)
    }

    /// Get summary statistics
    pub fn summary(&self) -> AnalysisSummary {
        let mut summary = AnalysisSummary::default();
        
        for finding in &self.findings {
            match finding.severity {
                Severity::Error => summary.error_count += 1,
                Severity::Warning => summary.warning_count += 1,
                Severity::Info => summary.info_count += 1,
                Severity::Critical => summary.error_count += 1, // Treat critical as error
            }
        }
        
        summary.total_findings = self.findings.len();
        summary.rules_executed = self.rule_results.len();
        summary.execution_time = self.execution_time;

        summary
    }
}

/// Individual rule execution result
#[derive(Debug, Clone)]
pub struct RuleExecutionResult {
    pub rule_id: String,
    pub findings: Vec<Finding>,
    pub execution_time: std::time::Duration,
    pub success: bool,
    pub error: Option<String>,
}

/// Execution statistics
#[derive(Debug, Clone)]
pub struct ExecutionStatistics {
    pub rules_executed: usize,
    pub total_findings: usize,
    pub total_execution_time: std::time::Duration,
    pub rule_timings: HashMap<String, std::time::Duration>,
    pub rule_finding_counts: HashMap<String, usize>,
}

impl ExecutionStatistics {
    fn new() -> Self {
        Self {
            rules_executed: 0,
            total_findings: 0,
            total_execution_time: std::time::Duration::new(0, 0),
            rule_timings: HashMap::new(),
            rule_finding_counts: HashMap::new(),
        }
    }

    fn record_rule_execution(&mut self, rule_id: &str, execution_time: std::time::Duration, finding_count: usize) {
        self.rules_executed += 1;
        self.total_findings += finding_count;
        self.total_execution_time += execution_time;
        self.rule_timings.insert(rule_id.to_string(), execution_time);
        self.rule_finding_counts.insert(rule_id.to_string(), finding_count);
    }

    fn record_rule_error(&mut self, rule_id: &str, execution_time: std::time::Duration) {
        self.rules_executed += 1;
        self.total_execution_time += execution_time;
        self.rule_timings.insert(rule_id.to_string(), execution_time);
    }
}

/// Analysis summary
#[derive(Debug, Clone, Default)]
pub struct AnalysisSummary {
    pub total_findings: usize,
    pub error_count: usize,
    pub warning_count: usize,
    pub info_count: usize,
    pub rules_executed: usize,
    pub execution_time: std::time::Duration,
}

// Add trait to Rule for checking if dataflow is required
impl Rule {
    /// Check if this rule requires data flow analysis
    pub fn requires_dataflow(&self) -> bool {
        self.dataflow.is_some()
    }
    
    /// Check if this rule requires symbolic propagation analysis
    pub fn requires_symbolic_propagation(&self) -> bool {
        // Check metadata for symbolic_propagation option
        if let Some(Value::String(val)) = self.metadata.get("symbolic_propagation") {
            return val == "true" || val == "on" || val == "yes" || val == "1";
        }
        if let Some(Value::Bool(val)) = self.metadata.get("symbolic_propagation") {
            return *val;
        }
        // For taint mode rules, enable symbolic propagation by default
        self.mode == crate::types::RuleMode::Taint
    }
}

// Helper functions for typed metavariable support
impl AdvancedRuleExecutor {
    /// Preprocess pattern to handle typed metavariable syntax like "($TYPE $VAR).method()"
    /// Returns the processed pattern and a map of variable names to their expected types
    fn preprocess_typed_metavariables(
        &self,
        pattern: &Pattern,
    ) -> (Pattern, Vec<(String, String)>) {
        let mut type_constraints = Vec::new();

        // Process based on pattern type
        match &pattern.pattern_type {
            PatternType::Simple(_) => {
                // For Simple patterns, check if it contains typed metavariable syntax
                // with access to any metavariable_pattern on the pattern itself
                let metavar_patterns: Vec<&MetavariablePattern> = pattern
                    .metavariable_pattern
                    .as_ref()
                    .map(|mp| vec![mp])
                    .unwrap_or_default();
                let (new_pattern, constraints) =
                    self.process_subpattern_with_metavars(pattern, &metavar_patterns);
                if !constraints.is_empty() {
                    return (new_pattern, constraints);
                }
            }
            PatternType::All(patterns) => {
                // Collect metavariable patterns from sub-patterns first
                let mut metavar_patterns: Vec<&MetavariablePattern> = Vec::new();
                for sub_pattern in patterns {
                    if let Some(ref mp) = sub_pattern.metavariable_pattern {
                        metavar_patterns.push(mp);
                    }
                }

                // Process each sub-pattern in the All composite
                let mut processed_patterns = Vec::new();
                let mut all_constraints = Vec::new();

                for sub_pattern in patterns {
                    // Skip placeholder patterns (Simple("...") with metavariable_pattern)
                    if let PatternType::Simple(ref s) = sub_pattern.pattern_type {
                        if s == "..." && sub_pattern.metavariable_pattern.is_some() {
                            // Extract type constraints from this metavariable-pattern
                            if let Some(ref mp) = sub_pattern.metavariable_pattern {
                                for type_pattern in &mp.patterns {
                                    // The metavariable_pattern applies to the main pattern
                                    // We'll handle this when processing the main pattern
                                    eprintln!(
                                        "DEBUG: Found metavariable-pattern for {}: {}",
                                        mp.metavariable, type_pattern
                                    );
                                }
                            }
                            continue; // Skip adding this placeholder pattern
                        }
                    }

                    // Process the sub-pattern with access to metavariable patterns
                    let (processed, constraints) =
                        self.process_subpattern_with_metavars(sub_pattern, &metavar_patterns);
                    processed_patterns.push(processed);
                    all_constraints.extend(constraints);
                }

                if !all_constraints.is_empty() || !processed_patterns.is_empty() {
                    let mut new_pattern = if processed_patterns.len() == 1 {
                        processed_patterns.into_iter().next().unwrap()
                    } else {
                        Pattern::all(processed_patterns)
                    };
                    new_pattern.conditions = pattern.conditions.clone();
                    new_pattern.focus = pattern.focus.clone();
                    return (new_pattern, all_constraints);
                }
            }
            PatternType::Either(patterns) => {
                // Process each sub-pattern in the Either composite
                let mut processed_patterns = Vec::new();
                let mut all_constraints = Vec::new();

                for sub_pattern in patterns {
                    let (processed, constraints) = self.preprocess_typed_metavariables(sub_pattern);
                    processed_patterns.push(processed);
                    all_constraints.extend(constraints);
                }

                if !all_constraints.is_empty() {
                    let mut new_pattern = Pattern::either(processed_patterns);
                    new_pattern.conditions = pattern.conditions.clone();
                    new_pattern.focus = pattern.focus.clone();
                    new_pattern.metavariable_pattern = pattern.metavariable_pattern.clone();
                    return (new_pattern, all_constraints);
                }
            }
            _ => {
                // Other pattern types not yet supported for typed metavariables
            }
        }

        // No transformation needed, return original
        (pattern.clone(), type_constraints)
    }

    /// Process a sub-pattern with access to metavariable patterns from other sub-patterns
    fn process_subpattern_with_metavars(
        &self,
        pattern: &Pattern,
        metavar_patterns: &[&MetavariablePattern],
    ) -> (Pattern, Vec<(String, String)>) {
        let mut type_constraints = Vec::new();

        if let PatternType::Simple(pattern_str) = &pattern.pattern_type {
            // Check if pattern contains typed metavariable syntax: "($TYPE $VAR)"
            // where both TYPE and VAR are metavariables (start with $)
            let typed_metavar_regex = regex::Regex::new(r"\(\$(\w+)\s+\$(\w+)\)").unwrap();

            if let Some(captures) = typed_metavar_regex.captures(pattern_str) {
                let type_var = captures.get(1).unwrap().as_str();
                let value_var = captures.get(2).unwrap().as_str();
                let full_match = captures.get(0).unwrap();

                eprintln!(
                    "DEBUG: Found typed metavariable syntax: type_var=${}, value_var=${}",
                    type_var, value_var
                );

                // Extract the expected type from metavariable patterns
                for mp in metavar_patterns {
                    if mp.metavariable == format!("${}", type_var)
                        || mp.metavariable == type_var
                    {
                        // Get the expected type from the patterns
                        for type_pattern in &mp.patterns {
                            eprintln!(
                                "DEBUG: Type pattern for ${}: {}",
                                type_var, type_pattern
                            );
                            type_constraints.push((value_var.to_string(), type_pattern.clone()));
                        }
                    }
                }

                // Replace "($TYPE $VAR)" with "$VAR" in the pattern
                let new_pattern_str =
                    pattern_str.replacen(full_match.as_str(), &format!("${}", value_var), 1);
                eprintln!(
                    "DEBUG: Transformed pattern from '{}' to '{}'",
                    pattern_str, new_pattern_str
                );

                let mut new_pattern = Pattern::simple(new_pattern_str);
                new_pattern.conditions = pattern.conditions.clone();
                new_pattern.focus = pattern.focus.clone();
                new_pattern.metavariable_pattern = pattern.metavariable_pattern.clone();

                return (new_pattern, type_constraints);
            }

            // Check for inline typed metavariable syntax: "(Type $VAR)"
            // where Type is a literal type name and VAR is a metavariable
            let inline_typed_regex = regex::Regex::new(r"\((\w+)\s+\$(\w+)\)").unwrap();

            if let Some(captures) = inline_typed_regex.captures(pattern_str) {
                let type_name = captures.get(1).unwrap().as_str();
                let value_var = captures.get(2).unwrap().as_str();
                let full_match = captures.get(0).unwrap();

                eprintln!(
                    "DEBUG: Found inline typed metavariable syntax: type={}, value_var=${}",
                    type_name, value_var
                );

                // Add type constraint directly from the pattern
                type_constraints.push((value_var.to_string(), type_name.to_string()));

                // Replace "(Type $VAR)" with "$VAR" in the pattern
                let new_pattern_str =
                    pattern_str.replacen(full_match.as_str(), &format!("${}", value_var), 1);
                eprintln!(
                    "DEBUG: Transformed pattern from '{}' to '{}'",
                    pattern_str, new_pattern_str
                );

                let mut new_pattern = Pattern::simple(new_pattern_str);
                new_pattern.conditions = pattern.conditions.clone();
                new_pattern.focus = pattern.focus.clone();
                new_pattern.metavariable_pattern = pattern.metavariable_pattern.clone();

                return (new_pattern, type_constraints);
            }
        }

        // No transformation needed, return original
        (pattern.clone(), type_constraints)
    }

    /// Check if a variable's type matches the expected type
    /// Also uses symbolic propagation to track if the variable comes from a method call
    /// on an object of the expected type
    fn check_variable_type(&self, var_value: &str, expected_type: &str, full_source: &str) -> bool {
        // Build import map for name resolution
        let import_map = self.build_import_map(full_source);
        
        // Look for variable declaration in the source code
        // Try different patterns to find the variable's type
        
        // Pattern 1: "Type varName =" or "Type varName;"
        let var_pattern = format!(r"(\w+)\s+{}\s*[=;]", regex::escape(var_value));
        if let Ok(regex) = regex::Regex::new(&var_pattern) {
            if let Some(captures) = regex.captures(full_source) {
                if let Some(type_match) = captures.get(1) {
                    let simple_type = type_match.as_str();
                    let resolved_type = self.resolve_type_with_imports(simple_type, &import_map);
                    
                    eprintln!("DEBUG: Variable {} has type '{}', resolved to '{:?}', expected '{}'", 
                             var_value, simple_type, resolved_type, expected_type);
                    
                    // Check if the resolved type matches the expected type
                    if let Some(resolved) = resolved_type {
                        // Check exact match or simple name match
                        if resolved == expected_type || simple_type == expected_type {
                            return true;
                        }
                        
                        // Check if expected type ends with the simple type (e.g., "org.foo.Foo" ends with "Foo")
                        if expected_type.ends_with(&format!(".{}", simple_type)) {
                            return true;
                        }
                    }
                }
            }
        }
        
        // Pattern 2: Method parameter "Type varName," or "Type varName)"
        let param_pattern = format!(r"(\w+)\s+{}\s*[,)]", regex::escape(var_value));
        if let Ok(regex) = regex::Regex::new(&param_pattern) {
            if let Some(captures) = regex.captures(full_source) {
                if let Some(type_match) = captures.get(1) {
                    let simple_type = type_match.as_str();
                    let resolved_type = self.resolve_type_with_imports(simple_type, &import_map);
                    
                    eprintln!("DEBUG: Parameter {} has type '{}', resolved to '{:?}', expected '{}'", 
                             var_value, simple_type, resolved_type, expected_type);
                    
                    if let Some(resolved) = resolved_type {
                        if resolved == expected_type || simple_type == expected_type {
                            return true;
                        }
                        
                        if expected_type.ends_with(&format!(".{}", simple_type)) {
                            return true;
                        }
                    }
                }
            }
        }
        
        // Pattern 3: Use symbolic propagation to check if variable comes from expected type
        // This handles cases like: ZipEntry c = ...; name = c.getName(); (ZipEntry $X).getName()
        eprintln!("DEBUG check_variable_type: Checking {} against expected type {}, symbolic_propagator is {}", 
                 var_value, expected_type, 
                 if self.symbolic_propagator.is_some() { "Some" } else { "None" });
        if let Some(ref propagator) = self.symbolic_propagator {
            eprintln!("DEBUG: Attempting symbolic propagation check for {} -> {}", var_value, expected_type);
            eprintln!("DEBUG: Symbolic state variables: {:?}", propagator.state().variables.keys().collect::<Vec<_>>());
            if self.check_type_via_symbolic_propagation(var_value, expected_type, propagator, full_source) {
                return true;
            }
        }
        
        // If we can't determine the type, be permissive
        eprintln!("DEBUG: Could not determine type for {}, allowing match", var_value);
        true
    }
    
    /// Check if a variable's origin traces back to an object of the expected type
    /// using symbolic propagation
    fn check_type_via_symbolic_propagation(
        &self,
        var_value: &str,
        expected_type: &str,
        propagator: &astgrep_dataflow::SymbolicPropagator,
        full_source: &str,
    ) -> bool {
        use astgrep_dataflow::SymbolicValue;
        
        eprintln!("DEBUG: Checking symbolic propagation for {} with expected type {}", 
                 var_value, expected_type);
        
        // Get the symbolic value for this variable
        let state = propagator.state();
        if let Some(symbolic_value) = state.get(var_value) {
            eprintln!("DEBUG: Found symbolic value for {}: {:?}", var_value, symbolic_value);
            
            // Get the root variable of this symbolic value
            if let Some(root_var) = symbolic_value.root_variable() {
                eprintln!("DEBUG: Root variable for {} is {}", var_value, root_var);
                
                // Check if the root variable is of the expected type
                // Look for variable declaration: "ExpectedType root_var = ..."
                let var_pattern = format!(r"{}\s+{}\s*[=;]", regex::escape(expected_type), regex::escape(root_var));
                if let Ok(regex) = regex::Regex::new(&var_pattern) {
                    if regex.is_match(full_source) {
                        eprintln!("DEBUG: Found {} declared as {} via symbolic propagation", 
                                 root_var, expected_type);
                        return true;
                    }
                }
                
                // Also check with import resolution
                let import_map = self.build_import_map(full_source);
                
                // Try to find declaration of the root variable
                let decl_pattern = format!(r"(\w+)\s+{}\s*[=;]", regex::escape(root_var));
                if let Ok(regex) = regex::Regex::new(&decl_pattern) {
                    if let Some(captures) = regex.captures(full_source) {
                        if let Some(type_match) = captures.get(1) {
                            let actual_type = type_match.as_str();
                            eprintln!("DEBUG: Root variable {} has type {}", root_var, actual_type);
                            
                            // Check if it matches expected type
                            if actual_type == expected_type {
                                return true;
                            }
                            
                            // Check with import resolution
                            if let Some(resolved) = self.resolve_type_with_imports(actual_type, &import_map) {
                                if resolved == expected_type || resolved.ends_with(&format!(".{}", expected_type)) {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Also check aliases of the variable
        let aliases = propagator.state().get_all_aliases(var_value);
        eprintln!("DEBUG: Aliases for {}: {:?}", var_value, aliases);
        
        for alias in aliases {
            // Check if any alias is of the expected type
            let alias_pattern = format!(r"{}\s+{}\s*[=;]", regex::escape(expected_type), regex::escape(&alias));
            if let Ok(regex) = regex::Regex::new(&alias_pattern) {
                if regex.is_match(full_source) {
                    eprintln!("DEBUG: Found alias {} of type {} via symbolic propagation", 
                             alias, expected_type);
                    return true;
                }
            }
            
            // Check the alias's symbolic value
            if let Some(alias_symbolic) = state.get(&alias) {
                if let Some(root_var) = alias_symbolic.root_variable() {
                    let decl_pattern = format!(r"{}\s+{}\s*[=;]", regex::escape(expected_type), regex::escape(root_var));
                    if let Ok(regex) = regex::Regex::new(&decl_pattern) {
                        if regex.is_match(full_source) {
                            eprintln!("DEBUG: Found root variable {} of type {} via alias symbolic propagation", 
                                     root_var, expected_type);
                            return true;
                        }
                    }
                }
            }
        }
        
        eprintln!("DEBUG: Symbolic propagation did not confirm type {} for {}", 
                 expected_type, var_value);
        false
    }
    
    /// Find pattern matches using symbolic propagation
    /// This is used when direct pattern matching fails but symbolic propagation
    /// might reveal matches through variable tracking
    fn find_matches_via_symbolic_propagation(
        &self,
        pattern: &astgrep_core::SemgrepPattern,
        ast: &dyn AstNode,
        type_constraints: &[(String, String)],
    ) -> Result<Vec<astgrep_core::SemgrepMatchResult>> {
        use astgrep_core::SemgrepMatchResult;

        eprintln!("DEBUG: Searching for symbolic propagation matches with {} type constraints",
                 type_constraints.len());

        let mut matches = Vec::new();

        // Get the symbolic propagator
        let propagator = match self.symbolic_propagator {
            Some(ref p) => p,
            None => return Ok(matches),
        };

        // Extract pattern info
        let pattern_str = match &pattern.pattern_type {
            astgrep_core::PatternType::Simple(s) => s.as_str(),
            _ => return Ok(matches),
        };

        eprintln!("DEBUG: Pattern string: '{}'", pattern_str);

        // Check if pattern contains ellipsis for method chaining
        if !pattern_str.contains("...") {
            eprintln!("DEBUG: Pattern does not contain ellipsis, skipping symbolic propagation");
            return Ok(matches);
        }

        // Try to parse the pattern to extract start and end methods
        // Patterns like "x(). ... .z()" or "$X(). ... .z()"
        if let Some((start_method, end_method)) = self.parse_ellipsis_pattern(pattern_str) {
            eprintln!("DEBUG: Parsed ellipsis pattern: start='{}', end='{}'", start_method, end_method);
            // Get full source code from AST
            let full_source = ast.text().unwrap_or("").to_string();
            self.find_ellipsis_matches_via_symbolic_propagation(
                ast, &start_method, &end_method, propagator, &full_source, &mut matches
            )?;
        } else {
            // Fall back to original if statement logic for getName().contains() patterns
            if pattern_str.contains("if") && pattern_str.contains("getName") && pattern_str.contains("contains") {
                let full_source = ast.text().unwrap_or("").to_string();
                self.find_if_statements_with_symbolic_match(ast, pattern_str, type_constraints, propagator, &full_source, &mut matches)?;
            } else {
                eprintln!("DEBUG: Could not parse ellipsis pattern, skipping symbolic propagation");
            }
        }

        eprintln!("DEBUG: Symbolic propagation found {} matches", matches.len());
        Ok(matches)
    }

    /// Parse an ellipsis pattern like "x(). ... .z()" or "$X(). ... .z()"
    /// Returns (start_method, end_method) if successful
    fn parse_ellipsis_pattern(&self, pattern_str: &str) -> Option<(String, String)> {
        // Remove whitespace for easier parsing
        let pattern = pattern_str.replace(" ", "");

        // Pattern format: something(). ... .something()
        // Find "()" at the start
        let start_paren = pattern.find("()")?;
        let start_method = if start_paren > 0 {
            pattern[..start_paren].to_string()
        } else {
            return None;
        };

        // Find "...()" sequence
        let ellipsis_idx = pattern.find("...")?;
        let after_ellipsis = &pattern[ellipsis_idx + 3..];

        // Skip one dot, then find the final "()" for end method
        if !after_ellipsis.starts_with('.') {
            return None;
        }

        let remaining = &after_ellipsis[1..];
        let end_paren = remaining.find("()")?;
        // Remove any leading dots from end_method
        let end_method = remaining[..end_paren].trim_start_matches('.').to_string();

        Some((start_method, end_method))
    }

    /// Find matches for ellipsis patterns using symbolic propagation
    /// Matches patterns like "x(). ... .z()" by tracking variable assignments
    fn find_ellipsis_matches_via_symbolic_propagation(
        &self,
        node: &dyn AstNode,
        start_method: &str,
        end_method: &str,
        propagator: &astgrep_dataflow::SymbolicPropagator,
        full_source: &str,
        matches: &mut Vec<astgrep_core::SemgrepMatchResult>,
    ) -> Result<()> {
        use astgrep_core::SemgrepMatchResult;
        use astgrep_dataflow::SymbolicValue;

        eprintln!("DEBUG: Searching for matches: {}(). ... .{}()", start_method, end_method);

        // Collect all variable declarations and their locations
        let mut var_declarations: Vec<(String, usize, usize)> = Vec::new();

        // Collect all method invocation nodes
        let mut method_calls: Vec<(String, String, usize, usize, Box<dyn AstNode>)> = Vec::new();

        // First pass: collect all variable declarations
        self.collect_variable_declarations(node, &mut var_declarations)?;

        // Second pass: collect all method calls
        self.collect_method_calls(node, &mut method_calls)?;

        eprintln!("DEBUG: Found {} variable declarations", var_declarations.len());
        eprintln!("DEBUG: Found {} method calls", method_calls.len());

        // For each variable, check if it's derived from start_method
        let mut derived_vars: Vec<String> = Vec::new();

        for (var_name, _var_line, _var_col) in &var_declarations {
            // Get symbolic value for this variable
            if let Some(sym_val) = propagator.get_symbolic_value(var_name) {
                eprintln!("DEBUG: Variable '{}' has symbolic value: {:?}", var_name, sym_val);

                // Check if this value is derived from start_method
                if self.is_symbolic_value_derived_from_method(sym_val, start_method) {
                    eprintln!("DEBUG: Variable '{}' is derived from {}()", var_name, start_method);
                    derived_vars.push(var_name.clone());
                } else {
                    // Check if it's derived indirectly through other variables
                    if self.check_indirect_derivation(var_name, start_method, propagator) {
                        eprintln!("DEBUG: Variable '{}' is indirectly derived from {}()", var_name, start_method);
                        derived_vars.push(var_name.clone());
                    }
                }
            }
        }

        // Now look for method calls that use these derived variables and call end_method
        for (receiver, method_name, line, col, node) in &method_calls {
            if method_name == end_method {
                eprintln!("DEBUG: Found {}() call with receiver '{}' at {}:{}", method_name, receiver, line, col);

                // Check if the receiver is derived from start_method
                if derived_vars.contains(receiver) {
                    eprintln!("DEBUG: Match found! Receiver '{}' is derived from {}()", receiver, start_method);

                    // Create a match result
                    let bindings = std::collections::HashMap::new();
                    let match_result = SemgrepMatchResult::new(node.clone_node(), bindings);
                    matches.push(match_result);
                }
            }
        }

        Ok(())
    }

    /// Check if a symbolic value is derived from a specific method call
    fn is_symbolic_value_derived_from_method(
        &self,
        sym_val: &astgrep_dataflow::SymbolicValue,
        method_name: &str,
    ) -> bool {
        use astgrep_dataflow::SymbolicValue;

        match sym_val {
            SymbolicValue::MethodCall { base, method } => {
                // Check if this method matches or if base is derived from it
                eprintln!("DEBUG: Checking MethodCall: method='{}', base={:?}", method, base);
                if method == method_name {
                    eprintln!("DEBUG: Method matches target '{}'", method_name);
                    return true;
                }
                let result = self.is_symbolic_value_derived_from_method(base, method_name);
                eprintln!("DEBUG: Base derived from '{}': {}", method_name, result);
                result
            }
            SymbolicValue::Variable(name) => {
                // For variables, check the name directly
                eprintln!("DEBUG: Checking Variable: name='{}' vs method_name='{}'", name, method_name);
                let result = name == method_name;
                eprintln!("DEBUG: Variable matches: {}", result);
                result
            }
            SymbolicValue::FieldAccess { base, .. } => {
                return self.is_symbolic_value_derived_from_method(base, method_name);
            }
            _ => {
                eprintln!("DEBUG: Unknown symbolic value, returning false");
                false
            }
        }
    }

    /// Check if a variable is indirectly derived from a method through other variables
    fn check_indirect_derivation(
        &self,
        var_name: &str,
        target_method: &str,
        propagator: &astgrep_dataflow::SymbolicPropagator,
    ) -> bool {
        use astgrep_dataflow::SymbolicValue;

        let mut visited = std::collections::HashSet::new();
        let mut to_check = vec![var_name.to_string()];

        while let Some(current_var) = to_check.pop() {
            if visited.contains(&current_var) {
                continue;
            }
            visited.insert(current_var.clone());

            if let Some(sym_val) = propagator.get_symbolic_value(&current_var) {
                // Check if this value is derived from target_method
                if self.is_symbolic_value_derived_from_method(sym_val, target_method) {
                    return true;
                }

                // If this is a variable reference, add it to the check queue
                if let SymbolicValue::Variable(ref_name) = sym_val {
                    to_check.push(ref_name.clone());
                }
                
                // If this is a method call or field access, add the base variable to check queue
                if let SymbolicValue::MethodCall { base, .. } = sym_val {
                    if let SymbolicValue::Variable(ref_name) = base.as_ref() {
                        to_check.push(ref_name.clone());
                    }
                }
                if let SymbolicValue::FieldAccess { base, .. } = sym_val {
                    if let SymbolicValue::Variable(ref_name) = base.as_ref() {
                        to_check.push(ref_name.clone());
                    }
                }
            }
        }

        false
    }

    /// Collect all variable declarations in the AST
    fn collect_variable_declarations(
        &self,
        node: &dyn AstNode,
        declarations: &mut Vec<(String, usize, usize)>,
    ) -> Result<()> {
        let node_type = node.node_type();

        match node_type {
            "local_variable_declaration" | "variable_declaration" | "field_declaration" => {
                // Extract variable name
                for i in 0..node.child_count() {
                    if let Some(child) = node.child(i) {
                        if child.node_type() == "identifier" {
                            if let Some(name) = child.text() {
                                if let Some((line, col, _, _)) = child.location() {
                                    declarations.push((name.to_string(), line, col));
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        // Recursively process children
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.collect_variable_declarations(child, declarations)?;
            }
        }

        Ok(())
    }

    /// Collect all method invocations in the AST
    fn collect_method_calls(
        &self,
        node: &dyn AstNode,
        method_calls: &mut Vec<(String, String, usize, usize, Box<dyn AstNode>)>,
    ) -> Result<()> {
        let node_type = node.node_type();

        if node_type == "method_invocation" || node_type == "call_expression" {
            // Extract receiver and method name
            let mut receiver = None;
            let mut method_name = None;

            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    let child_type = child.node_type();
                    let child_text = child.text();

                    match child_type {
                        "identifier" => {
                            // The first identifier might be the method name (for no-receiver calls)
                            // or the receiver (for chained calls like a.b())
                            if receiver.is_none() {
                                receiver = child_text.map(|s| s.to_string());
                            } else if method_name.is_none() {
                                method_name = child_text.map(|s| s.to_string());
                            }
                        }
                        "field_access" | "member_expression" => {
                            // This is likely the receiver part
                            if let Some(text) = child_text {
                                // Extract the receiver name from field access
                                let parts: Vec<&str> = text.split('.').collect();
                                if parts.len() >= 2 {
                                    receiver = Some(parts[0].to_string());
                                    method_name = Some(parts[1].to_string());
                                }
                            }
                        }
                        _ => {
                            // Check for method names in arguments or other positions
                            if method_name.is_none() && !is_operator_node(child_type, child_text) {
                                if let Some(text) = child_text {
                                    if text.contains('(') && text.contains(')') {
                                        // Extract method name from "method()"
                                        let name_part = text.trim_end_matches('(').trim_end_matches(')');
                                        method_name = Some(name_part.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if let (Some(recv), Some(meth), Some((line, col, _, _))) = (receiver, method_name, node.location()) {
                method_calls.push((recv, meth, line, col, node.clone_node()));
            }
        }

        // Recursively process children
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.collect_method_calls(child, method_calls)?;
            }
        }

        Ok(())
    }
    
    /// Find if statements that match via symbolic propagation
    fn find_if_statements_with_symbolic_match(
        &self,
        node: &dyn AstNode,
        pattern_str: &str,
        type_constraints: &[(String, String)],
        propagator: &astgrep_dataflow::SymbolicPropagator,
        full_source: &str,
        matches: &mut Vec<astgrep_core::SemgrepMatchResult>,
    ) -> Result<()> {
        // Check if this is an if_statement
        if node.node_type() == "if_statement" {
            eprintln!("DEBUG: Checking if_statement for symbolic match: {}", 
                     node.text().unwrap_or("").lines().next().unwrap_or(""));
            
            // Check if this if statement matches via symbolic propagation
            if let Some(match_result) = self.check_if_statement_symbolic_match(
                node, pattern_str, type_constraints, propagator, full_source
            )? {
                eprintln!("DEBUG: Found symbolic match for if_statement");
                matches.push(match_result);
            }
        }
        
        // Recursively check children
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.find_if_statements_with_symbolic_match(
                    child, pattern_str, type_constraints, propagator, full_source, matches
                )?;
            }
        }
        
        Ok(())
    }
    
    /// Check if an if statement matches the pattern via symbolic propagation
    fn check_if_statement_symbolic_match(
        &self,
        if_node: &dyn AstNode,
        pattern_str: &str,
        type_constraints: &[(String, String)],
        propagator: &astgrep_dataflow::SymbolicPropagator,
        full_source: &str,
    ) -> Result<Option<astgrep_core::SemgrepMatchResult>> {
        use astgrep_core::SemgrepMatchResult;
        use std::collections::HashMap;

        // Extract condition from if statement
        // Look for condition like "(b1 && b2)" or "(name == null)"
        let condition_text = self.extract_if_condition(if_node);
        eprintln!("DEBUG: If condition: {:?}", condition_text);

        if condition_text.is_none() {
            return Ok(None);
        }
        let condition = condition_text.unwrap();

        // For each type constraint, check if condition variables involve that type
        let mut bindings = HashMap::new();

        for (var_name, expected_type) in type_constraints {
            eprintln!("DEBUG: Checking if condition '{}' involves variable '${}' of type '{}'",
                     condition, var_name, expected_type);

            // Extract variables used in condition (e.g., "b1 && b2" -> ["b1", "b2"])
            let condition_vars = self.extract_variables_from_condition(&condition);
            eprintln!("DEBUG: Variables in condition: {:?}", condition_vars);

            for cond_var in condition_vars {
                // Check if this variable traces back to expected type via symbolic propagation
                // AND ensure it involves a contains() call
                if self.check_variable_type_via_symbolic_propagation(
                    &cond_var, expected_type, propagator, &full_source
                ) && self.variable_involves_contains(
                    &cond_var, &full_source
                ) {
                    eprintln!("DEBUG: Variable '{}' matches type '{}' via symbolic propagation and involves contains()",
                             cond_var, expected_type);
                    // Bind the pattern variable to this condition variable
                    bindings.insert(var_name.clone(), cond_var);
                }
            }
        }

        // If we found bindings for all type constraints, create a match result
        if bindings.len() == type_constraints.len() {
            eprintln!("DEBUG: Creating symbolic match result with bindings: {:?}", bindings);
            Ok(Some(SemgrepMatchResult::new(if_node.clone_node(), bindings)))
        } else {
            eprintln!("DEBUG: Not all type constraints satisfied. Found {} of {} bindings",
                     bindings.len(), type_constraints.len());
            Ok(None)
        }
    }

    /// Extract the condition text from an if statement node
    fn extract_if_condition(&self, if_node: &dyn AstNode) -> Option<String> {
        // The condition is typically child 1 (child 0 is "if", child 1 is condition, child 2 is body)
        if if_node.child_count() >= 2 {
            if let Some(condition_node) = if_node.child(1) {
                return condition_node.text().map(|s| s.to_string());
            }
        }
        None
    }
    
    /// Extract variable names from a condition string
    fn extract_variables_from_condition(&self, condition: &str) -> Vec<String> {
        // Simple regex to find identifier-like tokens
        let re = regex::Regex::new(r"\b([a-zA-Z_]\w*)\b").unwrap();
        re.captures_iter(condition)
            .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
            .filter(|name| {
                // Filter out keywords
                !matches!(name.as_str(), "if" | "else" | "while" | "for" | "return" | "true" | "false" | "null")
            })
            .collect()
    }
    
    /// Check if a variable's type matches expected type using symbolic propagation
    fn check_variable_type_via_symbolic_propagation(
        &self,
        var_value: &str,
        expected_type: &str,
        propagator: &astgrep_dataflow::SymbolicPropagator,
        full_source: &str,
    ) -> bool {
        use astgrep_dataflow::SymbolicValue;
        
        eprintln!("DEBUG check_var_type_sym: Checking if '{}' traces to type '{}'", var_value, expected_type);
        
        // Get the symbolic value for this variable
        let state = propagator.state();
        
        // Direct check: is this variable of the expected type?
        if let Some(symbolic_value) = state.get(var_value) {
            eprintln!("DEBUG: Found symbolic value for {}: {:?}", var_value, symbolic_value);
            
            // Get the root variable
            if let Some(root_var) = symbolic_value.root_variable() {
                eprintln!("DEBUG: Root variable is {}", root_var);
                
                // Check if root variable is of expected type
                let var_pattern = format!(r"{}\s+{}\s*[=;]", regex::escape(expected_type), regex::escape(root_var));
                if let Ok(regex) = regex::Regex::new(&var_pattern) {
                    if regex.is_match(full_source) {
                        eprintln!("DEBUG: Found direct type match for {}", root_var);
                        return true;
                    }
                }
            }
        }
        
        // Check if this variable is defined using the expected type
        // Look for patterns like "Type var = ..." or "boolean var = Type.method()"
        let decl_pattern = format!(r"\w+\s+{}\s*=\s*[^;]*", regex::escape(var_value));
        if let Ok(regex) = regex::Regex::new(&decl_pattern) {
            if let Some(cap) = regex.captures(full_source) {
                let decl = cap.get(0).map(|m| m.as_str()).unwrap_or("");
                eprintln!("DEBUG: Found declaration for {}: {}", var_value, decl);
                
                // Check if declaration involves the expected type
                // For example: "boolean b1 = !name.contains(...)" where name comes from ZipEntry
                if decl.contains("contains") || decl.contains("getName") {
                    // Extract the source variable (e.g., "name" from "name.contains")
                    if let Some(source_var) = self.extract_source_variable_from_declaration(decl) {
                        eprintln!("DEBUG: Source variable in declaration: {}", source_var);
                        
                        // Check if source variable traces to expected type
                        if let Some(symbolic_value) = state.get(&source_var) {
                            if let Some(root_var) = symbolic_value.root_variable() {
                                let type_pattern = format!(r"{}\s+{}\s*[=;]", 
                                    regex::escape(expected_type), regex::escape(root_var));
                                if let Ok(regex) = regex::Regex::new(&type_pattern) {
                                    if regex.is_match(full_source) {
                                        eprintln!("DEBUG: Source variable {} traces to type {}", 
                                                 root_var, expected_type);
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Check aliases
        let aliases = propagator.state().get_all_aliases(var_value);
        for alias in aliases {
            if let Some(alias_symbolic) = state.get(&alias) {
                if let Some(root_var) = alias_symbolic.root_variable() {
                    let type_pattern = format!(r"{}\s+{}\s*[=;]", 
                        regex::escape(expected_type), regex::escape(root_var));
                    if let Ok(regex) = regex::Regex::new(&type_pattern) {
                        if regex.is_match(full_source) {
                            return true;
                        }
                    }
                }
            }
        }
        
        false
    }
    
    /// Check if a variable's declaration involves a contains() call
    fn variable_involves_contains(&self, var_name: &str, full_source: &str) -> bool {
        // Look for the variable declaration in the source
        let decl_pattern = format!(r"\w+\s+{}\s*=\s*[^;]*", regex::escape(var_name));
        if let Ok(regex) = regex::Regex::new(&decl_pattern) {
            if let Some(cap) = regex.captures(full_source) {
                let decl = cap.get(0).map(|m| m.as_str()).unwrap_or("");
                eprintln!("DEBUG: Checking if declaration for '{}' involves contains(): {}", var_name, decl);
                // Check if the declaration contains a contains() call
                return decl.contains(".contains(");
            }
        }
        false
    }

    /// Extract the source variable from a declaration like "boolean b1 = !name.contains(...)"
    fn extract_source_variable_from_declaration(
        &self,
        decl: &str,
    ) -> Option<String> {
        // Look for patterns like "name.contains" or "obj.method"
        let re = regex::Regex::new(r"(\w+)\.(?:getName|contains)").unwrap();
        let result: Option<String> = re.captures_iter(decl)
            .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
            .next();
        result
    }

    /// Simplify a fully qualified class name pattern to just class.method pattern
    /// For example: "javax.xml.parsers.DocumentBuilderFactory.newInstance()" -> "DocumentBuilderFactory.newInstance()"
    fn simplify_fully_qualified_pattern(pattern: &str) -> Option<String> {
        // Check if this looks like a fully qualified name (contains multiple dots indicating package)
        let dot_count = pattern.matches('.').count();
        if dot_count < 2 {
            // Not a fully qualified name, no need to simplify
            return None;
        }

        // Split by dots and get the last two parts (class name and method/field)
        let parts: Vec<&str> = pattern.split('.').collect();
        if parts.len() >= 2 {
            // Get the last two parts: class name and method/field
            let class_name = parts[parts.len() - 2];
            let method_or_field = parts[parts.len() - 1];
            
            // Reconstruct as "ClassName.method()"
            let simplified = format!("{}.{}", class_name, method_or_field);
            eprintln!("[DEBUG] Simplified FQN pattern: '{}' -> '{}'", pattern, simplified);
            return Some(simplified);
        }

        None
    }
}
