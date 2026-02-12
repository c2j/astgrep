use super::VariableDependencyGraph;

impl VariableDependencyGraph {
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
                        let parts: Vec<&str> = before_for_each
                            .split(|c: char| c == '(' || c == ',' || c == ' ')
                            .collect();
                        if let Some(collection) = parts.last() {
                            let collection = collection.trim();

                            // Extract $Y (the lambda parameter inside parentheses)
                            if let Some(open_paren) = line[for_each_pos..].find('(') {
                                let after_open = &line[for_each_pos + open_paren + 1..];
                                // Look for pattern: (param) or param
                                let param_candidates: Vec<&str> = after_open
                                    .split(|c: char| c == '(' || c == ')' || c == ',' || c == '-')
                                    .collect();
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
                && pattern_text.contains('(')
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
                                    let obj_parts: Vec<&str> = before_setter
                                        .split(|c: char| c == ' ' || c == '(' || c == '.')
                                        .collect();
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
            } else if pattern_text.contains(".append(") && pattern_text.contains('$') {
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
                        let builder_parts: Vec<&str> = before_append
                            .split(|c: char| c == ' ' || c == '(' || c == '.' || c == ',')
                            .collect();
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
                    let parts: Vec<&str> = before_prefix
                        .split(|c: char| c == '(' || c == ',' || c == ' ' || c == '.')
                        .collect();
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
        let parts: Vec<&str> = line
            .split(|c: char| c == '(' || c == ',' || c == ' ')
            .collect();
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
                        let parts: Vec<&str> = before_get
                            .split(|c: char| c == '(' || c == ',' || c == ' ')
                            .collect();
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
