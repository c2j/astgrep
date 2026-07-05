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

impl Default for VariableDependencyGraph {
    fn default() -> Self {
        Self::new()
    }
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
        let sources = self.field_taints.entry(key).or_default();
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
        if var.contains(".get") && var.ends_with("()") && self.is_getter_tainted(var, source_vars) {
            return true;
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
            if expr.starts_with('"') && expr.ends_with('"') && expr.len() > 2 {
                let content = &expr[1..expr.len() - 1];
                if !content.is_empty() {
                    return true;
                }
            }
        }
        false
    }

    pub fn is_assigned_specific_string(&self, var: &str, target: &str) -> bool {
        if target.is_empty() {
            return false;
        }
        if let Some(expr) = self.assignments.get(var) {
            let expr = expr.trim();
            if expr.starts_with('"') && expr.ends_with('"') && expr.len() > 2 {
                let content = &expr[1..expr.len() - 1];
                return content == target;
            }
            if expr == target {
                return true;
            }
        }
        false
    }

    pub fn has_specific_string_in_dependency_chain(&self, var: &str, target: &str) -> bool {
        if target.is_empty() {
            return false;
        }
        let mut visited = std::collections::HashSet::new();
        self.check_specific_string_dependency_recursive(var, target, &mut visited)
    }

    fn check_specific_string_dependency_recursive(
        &self,
        var: &str,
        target: &str,
        visited: &mut std::collections::HashSet<String>,
    ) -> bool {
        if !visited.insert(var.to_string()) {
            return false;
        }
        if self.is_assigned_specific_string(var, target) {
            return true;
        }
        if let Some(deps) = self.dependencies.get(var) {
            for dep in deps {
                if self.check_specific_string_dependency_recursive(dep, target, visited) {
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

    /// Build dependency graph from method body text
    pub fn build_from_method(&mut self, method_text: &str) {
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
                        self.record_assignment(
                            var_name.clone(),
                            source_vars.clone(),
                            after_eq.to_string(),
                        );

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
                    eprintln!(
                        "[DEBUG] Propagator forEach pattern matched in line: {}",
                        line
                    );

                    // Extract $X (the collection/object before .forEach)
                    if let Some(for_each_pos) = line.find(".forEach") {
                        let before_for_each = &line[..for_each_pos];
                        let parts: Vec<&str> = before_for_each.split(['(', ',', ' ']).collect();
                        if let Some(collection) = parts.last() {
                            let collection = collection.trim();

                            // Extract $Y (the lambda parameter inside parentheses)
                            if let Some(open_paren) = line[for_each_pos..].find('(') {
                                let after_open = &line[for_each_pos + open_paren + 1..];
                                // Look for pattern: (param) or param
                                let param_candidates: Vec<&str> =
                                    after_open.split(['(', ')', ',', '-']).collect();
                                for candidate in param_candidates {
                                    let candidate = candidate.trim();
                                    // Valid parameter: non-empty, starts with letter, not a keyword
                                    if !candidate.is_empty()
                                        && candidate
                                            .chars()
                                            .next()
                                            .map(|c| c.is_alphabetic())
                                            .unwrap_or(false)
                                        && candidate != "null"
                                        && candidate != "true"
                                        && candidate != "false"
                                    {
                                        eprintln!(
                                            "[DEBUG] Propagator forEach: {} -> {}",
                                            collection, candidate
                                        );
                                        propagations
                                            .push((collection.to_string(), candidate.to_string()));
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            } else if (pattern_text.contains(".set") || pattern_text.contains("$SETTER"))
                && pattern_text.contains("(")
            {
                // Handle setter pattern: obj.setX(data) - propagate from data to obj
                // Pattern: (Type $OBJ).$SETTER($DATA) where $SETTER matches set.*
                // Also handle patterns like $PAGE.$SETTER($DATA) where SETTER is a metavariable
                eprintln!(
                    "[DEBUG] Checking setter pattern: '{}' on line: '{}'",
                    pattern_text, line
                );

                // Extract the setter method name pattern (e.g., $SETTER or setOrderBy)
                // Handle both literal patterns (obj.setX) and metavariable patterns (obj.$SETTER)
                let setter_pattern_opt = if let Some(set_pos) = pattern_text.find(".set") {
                    let after_set = &pattern_text[set_pos + 1..]; // Skip the dot, keep "set..."
                    after_set.find('(').map(|paren_pos| &after_set[..paren_pos])
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
                                    let obj_parts: Vec<&str> =
                                        before_setter.split([' ', '(', '.']).collect();
                                    if let Some(obj_name) = obj_parts.last() {
                                        let obj_name = obj_name.trim();

                                        // Extract argument (inside parentheses)
                                        let after_paren = &line_after_set[line_paren_pos + 1..];
                                        if let Some(close_paren) = after_paren.find(')') {
                                            let arg = &after_paren[..close_paren].trim();

                                            eprintln!(
                                                "[DEBUG] Setter propagator: {} -> {} (setter: {})",
                                                arg, obj_name, actual_setter
                                            );
                                            propagations
                                                .push((arg.to_string(), obj_name.to_string()));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            } else if pattern_text.contains(".append(") && pattern_text.contains("$") {
                // Handle append pattern: $BUILDER.append($STR) - propagate from STR to BUILDER
                eprintln!(
                    "[DEBUG] Checking append pattern: '{}' on line: '{}'",
                    pattern_text, line
                );

                // Check if line contains .append(
                if line.contains(".append(") {
                    // Find the append call
                    if let Some(append_pos) = line.find(".append(") {
                        let before_append = &line[..append_pos];
                        // Extract builder name
                        let builder_parts: Vec<&str> =
                            before_append.split([' ', '(', '.', ',']).collect();
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
                eprintln!(
                    "[DEBUG] Propagator pattern matched (substring): {}",
                    pattern_text
                );

                // Extract from and to metavariables
                let from_var = self.extract_metavariable(&propagator.from, line, pattern_text);
                let to_var = self.extract_metavariable(&propagator.to, line, pattern_text);

                if let (Some(from), Some(to)) = (from_var, to_var) {
                    eprintln!("[DEBUG] Propagator: {} -> {}", from, to);
                    propagations.push((from, to));
                }
            } else if pattern_text.contains('$') {
                eprintln!(
                    "[DEBUG] Trying regex matching for propagator pattern: '{}' on line: '{}'",
                    pattern_text, line
                );
                let mut var_order: Vec<String> = Vec::new();
                let mut remaining = pattern_text;
                let mut regex_pat = String::new();
                while let Some(dollar_pos) = remaining.find('$') {
                    regex_pat.push_str(&regex::escape(&remaining[..dollar_pos]));
                    remaining = &remaining[dollar_pos + 1..];
                    let var_end = remaining
                        .find(|c: char| !c.is_alphanumeric() && c != '_')
                        .unwrap_or(remaining.len());
                    if var_end > 0 {
                        var_order.push(remaining[..var_end].to_string());
                        regex_pat.push_str(r"(\w+)");
                        remaining = &remaining[var_end..];
                    }
                }
                regex_pat.push_str(&regex::escape(remaining));
                if let Ok(re) = regex::Regex::new(&format!("^{}$", regex_pat)) {
                    if let Some(captures) = re.captures(line.trim()) {
                        eprintln!("[DEBUG] Regex matched propagator pattern");
                        let captured_values: Vec<String> = (1..=var_order.len())
                            .filter_map(|i| captures.get(i).map(|m| m.as_str().to_string()))
                            .collect();

                        let from_var = Self::extract_metavar_value(
                            &var_order,
                            &captured_values,
                            &propagator.from,
                        );
                        let to_var = Self::extract_metavar_value(
                            &var_order,
                            &captured_values,
                            &propagator.to,
                        );

                        if let (Some(from), Some(to)) = (from_var, to_var) {
                            eprintln!("[DEBUG] Propagator (regex): {} -> {}", from, to);
                            propagations.push((from, to));
                        }
                    } else {
                        eprintln!("[DEBUG] Regex did not match line: '{}'", line);
                    }
                }
            }
        }

        // Apply collected propagations
        for (from, to) in propagations {
            self.record_assignment(
                to.clone(),
                vec![from.clone()],
                format!("propagated from {} to {}", from, to),
            );
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
        let _var_name = &metavar[1..]; // Remove $

        // Find pattern prefix before metavar
        if let Some(var_pos) = pattern.find(metavar) {
            let prefix = &pattern[..var_pos];
            if line.contains(prefix) {
                // Extract the identifier before the prefix in line
                if let Some(prefix_pos) = line.find(prefix) {
                    let before_prefix = &line[..prefix_pos].trim();
                    let parts: Vec<&str> = before_prefix.split(['(', ',', ' ', '.']).collect();
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
        let parts: Vec<&str> = line.split(['(', ',', ' ']).collect();
        for part in parts {
            let part = part.trim().trim_end_matches('.').trim_end_matches(')');
            if !part.is_empty()
                && !part.starts_with('$')
                && part
                    .chars()
                    .next()
                    .map(|c| c.is_alphabetic())
                    .unwrap_or(false)
            {
                return Some(part.to_string());
            }
        }

        None
    }

    fn extract_metavar_value(
        var_order: &[String],
        captured: &[String],
        metavar: &str,
    ) -> Option<String> {
        if !metavar.starts_with('$') {
            return Some(metavar.to_string());
        }
        let name = &metavar[1..];
        var_order
            .iter()
            .position(|v| v == name)
            .and_then(|idx| captured.get(idx).cloned())
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
                    let args =
                        &line[actual_pos + paren_pos + 1..actual_pos + paren_pos + close_paren];
                    // Getter calls typically have no arguments
                    if args.trim().is_empty() || !args.contains(',') {
                        // Extract object name
                        let before_get = &line[..actual_pos];
                        let parts: Vec<&str> = before_get.split(['(', ',', ' ']).collect();
                        if let Some(obj_name) = parts.last() {
                            let obj_name = obj_name.trim();
                            if !obj_name.is_empty() && !obj_name.contains('"') {
                                let field_name = after_get.to_lowercase();
                                let getter_call = format!("{}.get{}()", obj_name, after_get);

                                // Record getter mapping
                                self.record_getter_mapping(&getter_call, obj_name, &field_name);
                                eprintln!(
                                    "[DEBUG] Recorded argument getter mapping: {} -> {}.{}",
                                    getter_call, obj_name, field_name
                                );
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
                                    eprintln!(
                                        "[DEBUG] Recorded field taint: {}.{} tainted by {}",
                                        obj_name, field_name, source
                                    );
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
    fn process_getter_call(&mut self, _target_var: &str, expr: &str) {
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
                        eprintln!(
                            "[DEBUG] Recorded getter mapping: {} -> {}.{}",
                            getter_call, obj_name, field_name
                        );
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
                    eprintln!(
                        "[DEBUG] Recorded direct field taint: {}.{} tainted by {}",
                        obj_name, field_name, source
                    );
                }
            }
        }
    }

    /// Extract variable names from an expression
    pub fn extract_variables_from_expression(&self, expr: &str) -> Vec<String> {
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
