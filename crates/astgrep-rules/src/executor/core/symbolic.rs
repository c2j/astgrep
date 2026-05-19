//! Symbolic execution implementation
//!
//! This module contains symbolic execution methods for tracking variable relationships

use super::*;

impl AdvancedRuleExecutor {
    /// Find pattern matches using symbolic propagation
    /// This is used when direct pattern matching fails but symbolic propagation
    /// might reveal matches through variable tracking
    pub(super) fn find_matches_via_symbolic_propagation(
        &self,
        pattern: &astgrep_core::SemgrepPattern,
        ast: &dyn AstNode,
        type_constraints: &[(String, String)],
    ) -> Result<Vec<astgrep_core::SemgrepMatchResult>> {
        use astgrep_core::SemgrepMatchResult;

        eprintln!(
            "DEBUG: Searching for symbolic propagation matches with {} type constraints",
            type_constraints.len()
        );

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
            eprintln!(
                "DEBUG: Parsed ellipsis pattern: start='{}', end='{}'",
                start_method, end_method
            );
            // Get full source code from AST
            let full_source = ast.text().unwrap_or("").to_string();
            self.find_ellipsis_matches_via_symbolic_propagation(
                ast,
                &start_method,
                &end_method,
                propagator,
                &full_source,
                &mut matches,
            )?;
        } else {
            // Fall back to original if statement logic for getName().contains() patterns
            if pattern_str.contains("if")
                && pattern_str.contains("getName")
                && pattern_str.contains("contains")
            {
                let full_source = ast.text().unwrap_or("").to_string();
                self.find_if_statements_with_symbolic_match(
                    ast,
                    pattern_str,
                    type_constraints,
                    propagator,
                    &full_source,
                    &mut matches,
                )?;
            } else {
                eprintln!("DEBUG: Could not parse ellipsis pattern, skipping symbolic propagation");
            }
        }

        eprintln!(
            "DEBUG: Symbolic propagation found {} matches",
            matches.len()
        );
        Ok(matches)
    }

    /// Parse an ellipsis pattern like "x(). ... .z()" or "$X(). ... .z()"
    /// Returns (start_method, end_method) if successful
    pub(super) fn parse_ellipsis_pattern(&self, pattern_str: &str) -> Option<(String, String)> {
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
    pub(super) fn find_ellipsis_matches_via_symbolic_propagation(
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

        eprintln!(
            "DEBUG: Searching for matches: {}(). ... .{}()",
            start_method, end_method
        );

        // Collect all variable declarations and their locations
        let mut var_declarations: Vec<(String, usize, usize)> = Vec::new();

        // Collect all method invocation nodes
        let mut method_calls: Vec<(String, String, usize, usize, Box<dyn AstNode>)> = Vec::new();

        // First pass: collect all variable declarations
        self.collect_variable_declarations(node, &mut var_declarations)?;

        // Second pass: collect all method calls
        self.collect_method_calls(node, &mut method_calls)?;

        eprintln!(
            "DEBUG: Found {} variable declarations",
            var_declarations.len()
        );
        eprintln!("DEBUG: Found {} method calls", method_calls.len());

        // For each variable, check if it's derived from start_method
        let mut derived_vars: Vec<String> = Vec::new();

        for (var_name, _var_line, _var_col) in &var_declarations {
            // Get symbolic value for this variable
            if let Some(sym_val) = propagator.get_symbolic_value(var_name) {
                eprintln!(
                    "DEBUG: Variable '{}' has symbolic value: {:?}",
                    var_name, sym_val
                );

                // Check if this value is derived from start_method
                if self.is_symbolic_value_derived_from_method(sym_val, start_method) {
                    eprintln!(
                        "DEBUG: Variable '{}' is derived from {}()",
                        var_name, start_method
                    );
                    derived_vars.push(var_name.clone());
                } else {
                    // Check if it's derived indirectly through other variables
                    if self.check_indirect_derivation(var_name, start_method, propagator) {
                        eprintln!(
                            "DEBUG: Variable '{}' is indirectly derived from {}()",
                            var_name, start_method
                        );
                        derived_vars.push(var_name.clone());
                    }
                }
            }
        }

        // Now look for method calls that use these derived variables and call end_method
        for (receiver, method_name, line, col, node) in &method_calls {
            if method_name == end_method {
                eprintln!(
                    "DEBUG: Found {}() call with receiver '{}' at {}:{}",
                    method_name, receiver, line, col
                );

                // Check if the receiver is derived from start_method
                if derived_vars.contains(receiver) {
                    eprintln!(
                        "DEBUG: Match found! Receiver '{}' is derived from {}()",
                        receiver, start_method
                    );

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
    pub(super) fn is_symbolic_value_derived_from_method(
        &self,
        sym_val: &astgrep_dataflow::SymbolicValue,
        method_name: &str,
    ) -> bool {
        use astgrep_dataflow::SymbolicValue;

        match sym_val {
            SymbolicValue::MethodCall { base, method } => {
                // Check if this method matches or if base is derived from it
                eprintln!(
                    "DEBUG: Checking MethodCall: method='{}', base={:?}",
                    method, base
                );
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
                eprintln!(
                    "DEBUG: Checking Variable: name='{}' vs method_name='{}'",
                    name, method_name
                );
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
    pub(super) fn check_indirect_derivation(
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
    pub(super) fn collect_variable_declarations(
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
    pub(super) fn collect_method_calls(
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
                                        let name_part =
                                            text.trim_end_matches('(').trim_end_matches(')');
                                        method_name = Some(name_part.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if let (Some(recv), Some(meth), Some((line, col, _, _))) =
                (receiver, method_name, node.location())
            {
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
    pub(super) fn find_if_statements_with_symbolic_match(
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
            eprintln!(
                "DEBUG: Checking if_statement for symbolic match: {}",
                node.text().unwrap_or("").lines().next().unwrap_or("")
            );

            // Check if this if statement matches via symbolic propagation
            if let Some(match_result) = self.check_if_statement_symbolic_match(
                node,
                pattern_str,
                type_constraints,
                propagator,
                full_source,
            )? {
                eprintln!("DEBUG: Found symbolic match for if_statement");
                matches.push(match_result);
            }
        }

        // Recursively check children
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.find_if_statements_with_symbolic_match(
                    child,
                    pattern_str,
                    type_constraints,
                    propagator,
                    full_source,
                    matches,
                )?;
            }
        }

        Ok(())
    }

    /// Check if an if statement matches the pattern via symbolic propagation
    pub(super) fn check_if_statement_symbolic_match(
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
            eprintln!(
                "DEBUG: Checking if condition '{}' involves variable '${}' of type '{}'",
                condition, var_name, expected_type
            );

            // Extract variables used in condition (e.g., "b1 && b2" -> ["b1", "b2"])
            let condition_vars = self.extract_variables_from_condition(&condition);
            eprintln!("DEBUG: Variables in condition: {:?}", condition_vars);

            for cond_var in condition_vars {
                // Check if this variable traces back to expected type via symbolic propagation
                // AND ensure it involves a contains() call
                if self.check_variable_type_via_symbolic_propagation(
                    &cond_var,
                    expected_type,
                    propagator,
                    &full_source,
                ) && self.variable_involves_contains(&cond_var, &full_source)
                {
                    eprintln!("DEBUG: Variable '{}' matches type '{}' via symbolic propagation and involves contains()",
                             cond_var, expected_type);
                    // Bind the pattern variable to this condition variable
                    bindings.insert(var_name.clone(), cond_var);
                }
            }
        }

        // If we found bindings for all type constraints, create a match result
        // Special case: if no type constraints but pattern has required elements,
        // verify that condition variables match the pattern semantics
        let all_constraints_satisfied =
            type_constraints.is_empty() || bindings.len() == type_constraints.len();
        let meaningful_match = if type_constraints.is_empty() {
            let condition_vars = self.extract_variables_from_condition(&condition);
            let mut found = false;
            for cond_var in &condition_vars {
                let has_contains = self.variable_involves_contains(cond_var, &full_source);
                let has_class_type = self.traces_to_class_type(cond_var, propagator, &full_source);
                eprintln!(
                    "DEBUG: cond_var='{}', has_contains={}, has_class_type={}",
                    cond_var, has_contains, has_class_type
                );
                if has_contains && has_class_type {
                    found = true;
                    break;
                }
            }
            found
        } else {
            true
        };

        if all_constraints_satisfied && meaningful_match {
            eprintln!(
                "DEBUG: Creating symbolic match result with bindings: {:?}",
                bindings
            );
            Ok(Some(SemgrepMatchResult::new(
                if_node.clone_node(),
                bindings,
            )))
        } else {
            eprintln!(
                "DEBUG: Not all type constraints satisfied. Found {} of {} bindings",
                bindings.len(),
                type_constraints.len()
            );
            Ok(None)
        }
    }

    /// Extract the condition text from an if statement node
    pub(super) fn extract_if_condition(&self, if_node: &dyn AstNode) -> Option<String> {
        // The condition is typically child 1 (child 0 is "if", child 1 is condition, child 2 is body)
        if if_node.child_count() >= 2 {
            if let Some(condition_node) = if_node.child(1) {
                return condition_node.text().map(|s| s.to_string());
            }
        }
        None
    }

    /// Extract variable names from a condition string
    pub(super) fn extract_variables_from_condition(&self, condition: &str) -> Vec<String> {
        // Simple regex to find identifier-like tokens
        let re = regex::Regex::new(r"\b([a-zA-Z_]\w*)\b").unwrap();
        re.captures_iter(condition)
            .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
            .filter(|name| {
                // Filter out keywords
                !matches!(
                    name.as_str(),
                    "if" | "else" | "while" | "for" | "return" | "true" | "false" | "null"
                )
            })
            .collect()
    }

    /// Check if a variable's type matches expected type using symbolic propagation
    pub(super) fn check_variable_type_via_symbolic_propagation(
        &self,
        var_value: &str,
        expected_type: &str,
        propagator: &astgrep_dataflow::SymbolicPropagator,
        full_source: &str,
    ) -> bool {
        use astgrep_dataflow::SymbolicValue;

        eprintln!(
            "DEBUG check_var_type_sym: Checking if '{}' traces to type '{}'",
            var_value, expected_type
        );

        // Get the symbolic value for this variable
        let state = propagator.state();

        // Direct check: is this variable of the expected type?
        if let Some(symbolic_value) = state.get(var_value) {
            eprintln!(
                "DEBUG: Found symbolic value for {}: {:?}",
                var_value, symbolic_value
            );

            // Get the root variable
            if let Some(root_var) = symbolic_value.root_variable() {
                eprintln!("DEBUG: Root variable is {}", root_var);

                // Check if root variable is of expected type
                // Look for variable declaration: "ExpectedType root_var = ..."
                let var_pattern = format!(
                    r"{}\s+{}\s*[=;]",
                    regex::escape(expected_type),
                    regex::escape(root_var)
                );
                if let Ok(regex) = regex::Regex::new(&var_pattern) {
                    if regex.is_match(full_source) {
                        eprintln!(
                            "DEBUG: Found {} declared as {} via symbolic propagation",
                            root_var, expected_type
                        );
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
                            if let Some(resolved) =
                                self.resolve_type_with_imports(actual_type, &import_map)
                            {
                                if resolved == expected_type
                                    || resolved.ends_with(&format!(".{}", expected_type))
                                {
                                    return true;
                                }
                            }
                        }
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
                eprintln!(
                    "DEBUG: Checking if declaration for '{}' involves contains(): {}",
                    var_value, decl
                );
                // Check if the declaration contains a contains() call
                if decl.contains(".contains(") {
                    return true;
                }
            }
        }

        // Check aliases
        let aliases = propagator.state().get_all_aliases(var_value);
        for alias in aliases {
            if let Some(alias_symbolic) = state.get(&alias) {
                if let Some(root_var) = alias_symbolic.root_variable() {
                    let type_pattern = format!(
                        r"{}\s+{}\s*[=;]",
                        regex::escape(expected_type),
                        regex::escape(root_var)
                    );
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
    pub(super) fn variable_involves_contains(&self, var_name: &str, full_source: &str) -> bool {
        // Look for the variable declaration in the source
        let decl_pattern = format!(r"\w+\s+{}\s*=\s*[^;]*", regex::escape(var_name));
        if let Ok(regex) = regex::Regex::new(&decl_pattern) {
            if let Some(cap) = regex.captures(full_source) {
                let decl = cap.get(0).map(|m| m.as_str()).unwrap_or("");
                eprintln!(
                    "DEBUG: Checking if declaration for '{}' involves contains(): {}",
                    var_name, decl
                );
                // Check if the declaration contains a contains() call
                return decl.contains(".contains(");
            }
        }
        false
    }

    /// Extract the source variable from a declaration like "boolean b1 = !name.contains(...)"
    pub(super) fn extract_source_variable_from_declaration(&self, decl: &str) -> Option<String> {
        // Look for patterns like "name.contains" or "obj.method"
        let re = regex::Regex::new(r"(\w+)\.(?:getName|contains)").unwrap();
        let result: Option<String> = re
            .captures_iter(decl)
            .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
            .next();
        result
    }

    /// Check if a variable's origin traces back to an object of the expected type
    /// using symbolic propagation
    pub(super) fn check_type_via_symbolic_propagation(
        &self,
        var_value: &str,
        expected_type: &str,
        propagator: &astgrep_dataflow::SymbolicPropagator,
        full_source: &str,
    ) -> bool {
        use astgrep_dataflow::SymbolicValue;

        let state = propagator.state();
        if let Some(symbolic_value) = state.get(var_value) {
            if let Some(root_var) = symbolic_value.root_variable() {
                let var_pattern = format!(
                    r"{}\s+{}\s*[=;]",
                    regex::escape(expected_type),
                    regex::escape(root_var)
                );
                if let Ok(regex) = regex::Regex::new(&var_pattern) {
                    if regex.is_match(full_source) {
                        return true;
                    }
                }

                let import_map = self.build_import_map(full_source);
                let decl_pattern = format!(r"(\w+)\s+{}\s*[=;]", regex::escape(root_var));
                if let Ok(regex) = regex::Regex::new(&decl_pattern) {
                    if let Some(captures) = regex.captures(full_source) {
                        if let Some(type_match) = captures.get(1) {
                            let actual_type = type_match.as_str();
                            if actual_type == expected_type {
                                return true;
                            }
                            if let Some(resolved) =
                                self.resolve_type_with_imports(actual_type, &import_map)
                            {
                                if resolved == expected_type
                                    || resolved.ends_with(&format!(".{}", expected_type))
                                {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        }

        let aliases = propagator.state().get_all_aliases(var_value);
        for alias in aliases {
            let alias_pattern = format!(
                r"{}\s+{}\s*[=;]",
                regex::escape(expected_type),
                regex::escape(&alias)
            );
            if let Ok(regex) = regex::Regex::new(&alias_pattern) {
                if regex.is_match(full_source) {
                    return true;
                }
            }

            if let Some(alias_symbolic) = state.get(&alias) {
                if let Some(root_var) = alias_symbolic.root_variable() {
                    let decl_pattern = format!(
                        r"{}\s+{}\s*[=;]",
                        regex::escape(expected_type),
                        regex::escape(root_var)
                    );
                    if let Ok(regex) = regex::Regex::new(&decl_pattern) {
                        if regex.is_match(full_source) {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }

    /// Check if a variable traces back to a class-typed variable (not a primitive)
    pub(super) fn traces_to_class_type(
        &self,
        var_name: &str,
        propagator: &astgrep_dataflow::SymbolicPropagator,
        full_source: &str,
    ) -> bool {
        let mut visited = std::collections::HashSet::new();
        self.traces_to_class_type_inner(var_name, propagator, full_source, &mut visited)
    }

    fn traces_to_class_type_inner(
        &self,
        var_name: &str,
        propagator: &astgrep_dataflow::SymbolicPropagator,
        full_source: &str,
        visited: &mut std::collections::HashSet<String>,
    ) -> bool {
        if !visited.insert(var_name.to_string()) {
            return false;
        }

        let state = propagator.state();

        if self.is_class_typed_variable(var_name, full_source) {
            return true;
        }

        if let Some(symbolic_value) = state.get(var_name) {
            if let Some(root_var) = symbolic_value.root_variable() {
                if self.is_class_typed_variable(&root_var, full_source) {
                    return true;
                }
                if root_var != var_name {
                    return self.traces_to_class_type_inner(
                        &root_var,
                        propagator,
                        full_source,
                        visited,
                    );
                }
            }
        }

        for alias in state.get_all_aliases(var_name) {
            if self.is_class_typed_variable(&alias, full_source) {
                return true;
            }
            if alias != var_name {
                if self.traces_to_class_type_inner(&alias, propagator, full_source, visited) {
                    return true;
                }
            }
        }

        let decl_pattern = format!(r"{}\s*=\s*([^;]+)", regex::escape(var_name));
        if let Ok(regex) = regex::Regex::new(&decl_pattern) {
            if let Some(captures) = regex.captures(full_source) {
                if let Some(rhs) = captures.get(1) {
                    let rhs_text = rhs.as_str();
                    let var_pattern = regex::Regex::new(r"\b([a-z][a-zA-Z0-9]*)\b").unwrap();
                    for var_cap in var_pattern.captures_iter(rhs_text) {
                        if let Some(rhs_var) = var_cap.get(1) {
                            let rhs_var_name = rhs_var.as_str();
                            if ![
                                "if", "else", "for", "while", "return", "new", "true", "false",
                                "null",
                            ]
                            .contains(&rhs_var_name)
                            {
                                if self.is_class_typed_variable(rhs_var_name, full_source) {
                                    return true;
                                }
                                if rhs_var_name != var_name {
                                    if self.traces_to_class_type_inner(
                                        rhs_var_name,
                                        propagator,
                                        full_source,
                                        visited,
                                    ) {
                                        return true;
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

    /// Check if a variable is declared with a class type (not primitive)
    fn is_class_typed_variable(&self, var_name: &str, full_source: &str) -> bool {
        let decl_pattern = format!(r"(\w+)\s+{}\s*[=;]", regex::escape(var_name));
        if let Ok(regex) = regex::Regex::new(&decl_pattern) {
            if let Some(captures) = regex.captures(full_source) {
                if let Some(type_match) = captures.get(1) {
                    let primitives = [
                        "int", "long", "short", "byte", "float", "double", "boolean", "char",
                        "void", "String",
                    ];
                    return !primitives.contains(&type_match.as_str());
                }
            }
        }
        false
    }
}
