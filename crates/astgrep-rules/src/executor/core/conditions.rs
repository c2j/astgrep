//! Conditions evaluation implementation
//!
//! This module contains condition evaluation methods for pattern matching

use super::*;

impl AdvancedRuleExecutor {
    pub(super) fn check_pattern_conditions(
        &self,
        pattern: &Pattern,
        match_result: &SemgrepMatchResult,
        dataflow_analysis: Option<&DataFlowAnalysis>,
        full_source: &str,
    ) -> Result<bool> {
        eprintln!(
            "DEBUG check_pattern_conditions: {} conditions to check, bindings={:?}",
            pattern.conditions.len(),
            match_result.bindings
        );
        for condition in &pattern.conditions {
            if !self.evaluate_condition(condition, match_result, dataflow_analysis, full_source)? {
                eprintln!(
                    "DEBUG check_pattern_conditions: condition {:?} FAILED",
                    condition
                );
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Evaluate a single condition
    pub(super) fn evaluate_condition(
        &self,
        condition: &Condition,
        match_result: &SemgrepMatchResult,
        _dataflow_analysis: Option<&DataFlowAnalysis>,
        full_source: &str,
    ) -> Result<bool> {
        match condition {
            Condition::MetavariableRegex(metavar_regex) => {
                let key = metavar_regex
                    .metavariable
                    .trim_start_matches('$')
                    .to_string();
                if let Some(metavar_value) = match_result.bindings.get(&key) {
                    let regex_str = &metavar_regex.regex;
                    let regex = if regex_str.starts_with("(?i)") {
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
                let key = metavar_comp
                    .metavariable
                    .trim_start_matches('$')
                    .to_string();
                eprintln!(
                    "DEBUG evaluate_condition: MetavariableComparison for '{}', bindings: {:?}",
                    key,
                    match_result.bindings.keys().collect::<Vec<_>>()
                );
                if let Some(metavar_value) = match_result.bindings.get(&key) {
                    eprintln!(
                        "DEBUG: Found value '{}' for metavariable '{}'",
                        metavar_value, key
                    );

                    let resolved_value = if let Some(ref propagator) = self.constant_propagator {
                        if let Some((start_line, start_col, _, _)) = match_result.node.location() {
                            use astgrep_dataflow::constant_propagation::SourceLocation;
                            let location = SourceLocation::new(start_line, start_col);

                            if let Some(constant) =
                                propagator.get_variable_value_at_location(metavar_value, location)
                            {
                                constant
                                    .to_string_value()
                                    .unwrap_or_else(|| metavar_value.clone())
                            } else {
                                metavar_value.clone()
                            }
                        } else {
                            metavar_value.clone()
                        }
                    } else {
                        metavar_value.clone()
                    };

                    if let ComparisonOperator::PythonExpression(expr) = &metavar_comp.operator {
                        return self.evaluate_python_expression(
                            &resolved_value,
                            expr,
                            &match_result.bindings,
                        );
                    }

                    self.evaluate_comparison(
                        &resolved_value,
                        &metavar_comp.operator,
                        &metavar_comp.value,
                    )
                } else {
                    eprintln!("DEBUG: Metavariable '{}' not found in bindings", key);
                    Ok(false)
                }
            }
            Condition::NodeType(expected_type) => {
                Ok(match_result.node.node_type() == *expected_type)
            }
            Condition::NodeAttribute(attr_name, attr_value) => {
                // Check node attribute (simplified implementation)
                // In a real implementation, this would check actual node attributes
                Ok(match_result.node.text().unwrap_or("").contains(attr_value))
            }
            Condition::MetavariableName(metavar_name) => {
                let key = metavar_name.metavariable.trim_start_matches('$');
                if let Some(metavar_value) = match_result.bindings.get(key) {
                    self.evaluate_name_constraint(
                        metavar_value,
                        &metavar_name.name_pattern,
                        full_source,
                    )
                } else {
                    Ok(false)
                }
            }
            Condition::MetavariableAnalysis(metavar_analysis) => {
                // Evaluate metavariable analysis constraint
                if let Some(metavar_value) =
                    match_result.bindings.get(&metavar_analysis.metavariable)
                {
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
                        if let Some(type_info) =
                            self.extract_type_info(match_result, var_value, full_source)
                        {
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
                self.evaluate_custom_condition(custom_condition, match_result)
            }
            Condition::MetavariablePattern(metavar_pattern) => {
                let metavar_key = metavar_pattern.metavariable.trim_start_matches('$');
                eprintln!(
                    "DEBUG MetavariablePattern: key='{}', patterns={:?}, bindings={:?}",
                    metavar_key, metavar_pattern.patterns, match_result.bindings
                );
                if let Some(bound_value) = match_result.bindings.get(metavar_key) {
                    let mut combined_bindings = match_result.bindings.clone();
                    for pattern_str in &metavar_pattern.patterns {
                        if pattern_str.starts_with("__NOT__:") {
                            let neg_pattern = &pattern_str[8..];
                            let matches = self.pattern_text_matches_value(neg_pattern, bound_value);
                            eprintln!(
                                "DEBUG MetavariablePattern NOT: neg_pattern='{}', value='{}', matches={}",
                                neg_pattern, bound_value, matches
                            );
                            if matches {
                                return Ok(false);
                            }
                        } else {
                            let (matches, new_bindings) =
                                self.pattern_text_matches_with_bindings(pattern_str, bound_value);
                            eprintln!(
                                "DEBUG MetavariablePattern: pattern='{}', value='{}', matches={}, new_bindings={:?}",
                                pattern_str, bound_value, matches, new_bindings
                            );
                            if !matches {
                                return Ok(false);
                            }
                            combined_bindings.extend(new_bindings);
                        }
                    }

                    for nested in &metavar_pattern.nested_conditions {
                        if !self.evaluate_condition_with_bindings(
                            nested,
                            &combined_bindings,
                            match_result,
                            full_source,
                        )? {
                            eprintln!("DEBUG MetavariablePattern: nested condition FAILED");
                            return Ok(false);
                        }
                    }
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
        }
    }

    fn evaluate_condition_with_bindings(
        &self,
        condition: &Condition,
        bindings: &HashMap<String, String>,
        original_match: &SemgrepMatchResult,
        full_source: &str,
    ) -> Result<bool> {
        let temp_result =
            SemgrepMatchResult::new(original_match.node.clone_node(), bindings.clone());
        self.evaluate_condition(condition, &temp_result, None, full_source)
    }

    /// Extract type information for a variable from the match context
    pub(super) fn extract_type_info(
        &self,
        match_result: &SemgrepMatchResult,
        var_name: &str,
        full_source: &str,
    ) -> Option<String> {
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
        let field_pattern = format!(
            r"(?:public|private|protected)?\s*(?:static\s+)?(?:final\s+)?(\w+)\s+{}\s*=[^;]*;",
            regex::escape(var_name)
        );
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
    pub(super) fn build_import_map(&self, full_source: &str) -> HashMap<String, String> {
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

                    eprintln!(
                        "DEBUG: Found import: {} -> {}",
                        simple_name, fully_qualified
                    );
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
    pub(super) fn resolve_type_with_imports(
        &self,
        simple_type: &str,
        import_map: &HashMap<String, String>,
    ) -> Option<String> {
        // First check if this simple type is in the import map
        if let Some(fully_qualified) = import_map.get(simple_type) {
            eprintln!(
                "DEBUG: Resolved type '{}' to '{}'",
                simple_type, fully_qualified
            );
            return Some(fully_qualified.clone());
        }

        // If not found in imports, return the simple type as-is
        // (it might be a primitive type or in the same package)
        Some(simple_type.to_string())
    }

    /// Infer the type of a value from its literal representation
    pub(super) fn infer_type_from_value(&self, value: &str) -> Option<String> {
        let trimmed = value.trim();

        // String literal: "..." or '...'
        if (trimmed.starts_with('"') && trimmed.ends_with('"'))
            || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
        {
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
    pub(super) fn evaluate_comparison(
        &self,
        metavar_value: &str,
        operator: &ComparisonOperator,
        expected_value: &str,
    ) -> Result<bool> {
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
                if let (Ok(mv), Ok(ev)) =
                    (metavar_value.parse::<f64>(), expected_value.parse::<f64>())
                {
                    Ok(mv > ev)
                } else {
                    Ok(metavar_value > expected_value)
                }
            }
            ComparisonOperator::LessThan => {
                if let (Ok(mv), Ok(ev)) =
                    (metavar_value.parse::<f64>(), expected_value.parse::<f64>())
                {
                    Ok(mv < ev)
                } else {
                    Ok(metavar_value < expected_value)
                }
            }
            _ => Ok(false),
        }
    }

    /// Evaluate name constraint with FQN resolution using imports.
    pub(super) fn evaluate_name_constraint(
        &self,
        value: &str,
        name_pattern: &str,
        full_source: &str,
    ) -> Result<bool> {
        let import_map = self.build_import_map(full_source);
        let resolved_value = self.resolve_name_to_fqn(value, &import_map);

        eprintln!(
            "DEBUG evaluate_name_constraint: value='{}', resolved='{}', pattern='{}'",
            value, resolved_value, name_pattern
        );

        if name_pattern.contains("*") {
            let regex_pattern = name_pattern.replace(".", "\\.").replace("*", ".*");
            if let Ok(regex) = regex::Regex::new(&regex_pattern) {
                Ok(regex.is_match(&resolved_value))
            } else {
                Ok(false)
            }
        } else if resolved_value == name_pattern {
            Ok(true)
        } else if name_pattern.ends_with(&format!(".{}", value)) {
            Ok(import_map
                .get(value)
                .map_or(false, |fqn| fqn == name_pattern))
        } else if resolved_value.ends_with(&format!(".{}", name_pattern)) {
            Ok(true)
        } else {
            Ok(resolved_value == name_pattern)
        }
    }

    fn resolve_name_to_fqn(&self, name: &str, import_map: &HashMap<String, String>) -> String {
        if name.contains('.') {
            return name.to_string();
        }
        import_map
            .get(name)
            .cloned()
            .unwrap_or_else(|| name.to_string())
    }

    /// Evaluate analysis constraint (entropy, type, complexity)
    pub(super) fn evaluate_analysis_constraint(
        &self,
        value: &str,
        analysis: &MetavariableAnalysis,
    ) -> Result<bool> {
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
    pub(super) fn check_entropy(
        &self,
        value: &str,
        entropy_config: &astgrep_core::EntropyAnalysis,
    ) -> Result<bool> {
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
    pub(super) fn check_type_analysis(
        &self,
        value: &str,
        type_config: &astgrep_core::TypeAnalysis,
    ) -> Result<bool> {
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
    pub(super) fn check_complexity(
        &self,
        value: &str,
        complexity_config: &astgrep_core::ComplexityAnalysis,
    ) -> Result<bool> {
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
    pub(super) fn calculate_entropy(&self, s: &str) -> f64 {
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
    pub(super) fn matches_charset(&self, value: &str, charset: &str) -> bool {
        match charset {
            "alphanumeric" => value.chars().all(|c| c.is_alphanumeric()),
            "alphabetic" => value.chars().all(|c| c.is_alphabetic()),
            "numeric" => value.chars().all(|c| c.is_numeric()),
            "ascii" => value.is_ascii(),
            _ => true, // Unknown charset, assume match
        }
    }

    /// Check if value matches a type pattern
    pub(super) fn value_matches_type(&self, value: &str, type_name: &str) -> bool {
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
    pub(super) fn evaluate_python_expression(
        &self,
        value: &str,
        expr: &str,
        bindings: &std::collections::HashMap<String, String>,
    ) -> Result<bool> {
        // This is a simplified implementation
        // In a full implementation, this would use a Python interpreter

        eprintln!(
            "DEBUG evaluate_python_expression: value='{}', expr='{}'",
            value, expr
        );

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

                        eprintln!(
                            "DEBUG: var_part='{}', mask_part='{}', expected='{}'",
                            var_part, mask_part, expected_result
                        );

                        // Check if this is the metavariable we're evaluating
                        if var_part.starts_with("$") {
                            // Parse the mask value
                            if let Ok(mask) = mask_part.parse::<i64>() {
                                // Parse the expected result
                                if let Ok(expected) = expected_result.parse::<i64>() {
                                    // Parse the actual value
                                    if let Ok(val) = value.parse::<i64>() {
                                        let result = val & mask;
                                        eprintln!(
                                            "DEBUG: val={}, mask={}, result={}, expected={}",
                                            val, mask, result, expected
                                        );
                                        return Ok(result == expected);
                                    } else {
                                        eprintln!(
                                            "DEBUG: Failed to parse value '{}' as i64",
                                            value
                                        );
                                    }
                                } else {
                                    eprintln!(
                                        "DEBUG: Failed to parse expected '{}' as i64",
                                        expected_result
                                    );
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

                        eprintln!(
                            "DEBUG bitor: var_part='{}', mask_part='{}', expected='{}'",
                            var_part, mask_part, expected_result
                        );

                        // Check if this is the metavariable we're evaluating
                        if var_part.starts_with("$") {
                            // Parse the mask value
                            if let Ok(mask) = mask_part.parse::<i64>() {
                                // Parse the expected result
                                if let Ok(expected) = expected_result.parse::<i64>() {
                                    // Parse the actual value
                                    if let Ok(val) = value.parse::<i64>() {
                                        let result = val | mask;
                                        eprintln!(
                                            "DEBUG bitor: val={}, mask={}, result={}, expected={}",
                                            val, mask, result, expected
                                        );
                                        return Ok(result == expected);
                                    } else {
                                        eprintln!(
                                            "DEBUG: Failed to parse value '{}' as i64",
                                            value
                                        );
                                    }
                                } else {
                                    eprintln!(
                                        "DEBUG: Failed to parse expected '{}' as i64",
                                        expected_result
                                    );
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

                        eprintln!(
                            "DEBUG bitxor: var_part='{}', mask_part='{}', expected='{}'",
                            var_part, mask_part, expected_result
                        );

                        // Check if this is the metavariable we're evaluating
                        if var_part.starts_with("$") {
                            // Parse the mask value
                            if let Ok(mask) = mask_part.parse::<i64>() {
                                // Parse the expected result
                                if let Ok(expected) = expected_result.parse::<i64>() {
                                    // Parse the actual value
                                    if let Ok(val) = value.parse::<i64>() {
                                        let result = val ^ mask;
                                        eprintln!(
                                            "DEBUG bitxor: val={}, mask={}, result={}, expected={}",
                                            val, mask, result, expected
                                        );
                                        return Ok(result == expected);
                                    } else {
                                        eprintln!(
                                            "DEBUG: Failed to parse value '{}' as i64",
                                            value
                                        );
                                    }
                                } else {
                                    eprintln!(
                                        "DEBUG: Failed to parse expected '{}' as i64",
                                        expected_result
                                    );
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

                eprintln!(
                    "DEBUG bitnot: var_part='{}', expected='{}'",
                    var_part, expected_result
                );

                // Check if this is the metavariable we're evaluating
                if var_part.starts_with("$") {
                    // Parse the expected result
                    if let Ok(expected) = expected_result.parse::<i64>() {
                        // Parse the actual value
                        if let Ok(val) = value.parse::<i64>() {
                            // Python's ~ operator: ~x = -(x + 1)
                            let result = -(val + 1);
                            eprintln!(
                                "DEBUG bitnot: val={}, result={}, expected={}",
                                val, result, expected
                            );
                            return Ok(result == expected);
                        } else {
                            eprintln!("DEBUG: Failed to parse value '{}' as i64", value);
                        }
                    } else {
                        eprintln!(
                            "DEBUG: Failed to parse expected '{}' as i64",
                            expected_result
                        );
                    }
                } else {
                    eprintln!("DEBUG: var_part '{}' doesn't start with $", var_part);
                }
            } else {
                eprintln!("DEBUG: parts.len() = {}, expected 2", parts.len());
            }
        }

        // Handle "in" operator: str($VAR) in "abc" or $VAR not in [150, 312]
        if expr.contains(" in ") || expr.contains(" not in ") {
            let negated = expr.contains(" not in ");
            let in_expr = if negated {
                expr.split(" not in ").collect::<Vec<_>>()
            } else {
                expr.split(" in ").collect::<Vec<_>>()
            };
            if in_expr.len() == 2 {
                let left = in_expr[0].trim();
                let right = in_expr[1].trim();

                let check_value = if left == "str($VAR)" || left == format!("str({})", value) {
                    value.to_string()
                } else if left.starts_with('$') {
                    value.to_string()
                } else {
                    value.to_string()
                };

                if right.starts_with('"') && right.ends_with('"') {
                    let target = &right[1..right.len() - 1];
                    let result = target.contains(&check_value);
                    return Ok(if negated { !result } else { result });
                } else if right.starts_with('[') && right.ends_with(']') {
                    let inner = &right[1..right.len() - 1];
                    let items: Vec<&str> = inner
                        .split(',')
                        .map(|s| s.trim())
                        .filter(|s| !s.is_empty())
                        .collect();
                    let result = items.iter().any(|item| item == &check_value.as_str());
                    return Ok(if negated { !result } else { result });
                }
            }
        }

        // Handle ** (power) operator: $X ** $Y == N
        if expr.contains("**") && expr.contains("==") {
            let parts: Vec<&str> = expr.split("==").collect();
            if parts.len() == 2 {
                let expected = parts[1].trim();
                if let Ok(expected_val) = expected.parse::<f64>() {
                    let left = parts[0].trim();
                    let power_parts: Vec<&str> = left.split("**").collect();
                    if power_parts.len() == 2 {
                        let base_str = power_parts[0].trim();
                        let exp_str = power_parts[1].trim();

                        let base_val = if base_str.starts_with('$') {
                            let key = base_str.trim_start_matches('$');
                            bindings
                                .get(key)
                                .and_then(|v| v.parse::<f64>().ok())
                                .unwrap_or_else(|| value.parse::<f64>().unwrap_or(0.0))
                        } else {
                            value.parse::<f64>().unwrap_or(0.0)
                        };

                        let exp_val = if exp_str.starts_with('$') {
                            let key = exp_str.trim_start_matches('$');
                            bindings
                                .get(key)
                                .and_then(|v| v.parse::<f64>().ok())
                                .unwrap_or(0.0)
                        } else {
                            exp_str.parse::<f64>().unwrap_or(0.0)
                        };

                        let result = base_val.powf(exp_val);
                        let matches = (result - expected_val).abs() < 1e-9;
                        return Ok(matches);
                    }
                }
            }
        }

        eprintln!("DEBUG: Expression '{}' not handled, returning false", expr);
        Ok(false)
    }

    /// Evaluate custom condition
    pub(super) fn evaluate_custom_condition(
        &self,
        condition_name: &str,
        _match_result: &SemgrepMatchResult,
    ) -> Result<bool> {
        match condition_name {
            "always_true" => Ok(true),
            "always_false" => Ok(false),
            _ => Ok(true),
        }
    }

    fn pattern_text_matches_value(&self, pattern_str: &str, value: &str) -> bool {
        let (matches, _) = self.pattern_text_matches_with_bindings(pattern_str, value);
        matches
    }

    fn pattern_text_matches_with_bindings(
        &self,
        pattern_str: &str,
        value: &str,
    ) -> (bool, HashMap<String, String>) {
        let pattern = pattern_str.trim();
        let value = value.trim();

        if pattern.contains("...") {
            let matches = self.ellipsis_pattern_matches(pattern, value);
            return (matches, HashMap::new());
        }

        let metavar_re = match regex::Regex::new(r"\$([A-Za-z_]\w*)") {
            Ok(re) => re,
            Err(_) => {
                let matches = pattern == value;
                return (matches, HashMap::new());
            }
        };

        if !metavar_re.is_match(pattern) {
            let pattern_tokens: Vec<&str> = pattern.split_whitespace().collect();
            let value_tokens: Vec<&str> = value.split_whitespace().collect();
            let matches = pattern_tokens.len() == value_tokens.len()
                && pattern_tokens
                    .iter()
                    .zip(value_tokens.iter())
                    .all(|(p, v)| *p == *v);
            return (matches, HashMap::new());
        }

        struct Segment {
            literal: String,
            metavar: Option<String>,
        }

        let mut segments: Vec<Segment> = Vec::new();
        let mut last_end = 0;

        for cap in metavar_re.find_iter(pattern) {
            if cap.start() > last_end {
                segments.push(Segment {
                    literal: pattern[last_end..cap.start()].to_string(),
                    metavar: None,
                });
            }
            segments.push(Segment {
                literal: String::new(),
                metavar: Some(cap.as_str()[1..].to_string()),
            });
            last_end = cap.end();
        }
        if last_end < pattern.len() {
            segments.push(Segment {
                literal: pattern[last_end..].to_string(),
                metavar: None,
            });
        }

        let mut bindings = HashMap::new();
        let mut val_pos: usize = 0;

        for (i, seg) in segments.iter().enumerate() {
            if let Some(ref mv) = seg.metavar {
                let next_literal_idx = segments[i + 1..]
                    .iter()
                    .position(|s| !s.literal.is_empty())
                    .map(|p| i + 1 + p);

                if let Some(next_idx) = next_literal_idx {
                    let next_lit = &segments[next_idx].literal;
                    if let Some(pos) = value[val_pos..].find(next_lit.as_str()) {
                        bindings.insert(mv.clone(), value[val_pos..val_pos + pos].to_string());
                        val_pos += pos;
                    } else {
                        return (false, HashMap::new());
                    }
                }
            } else if !seg.literal.is_empty() {
                if !value[val_pos..].starts_with(&seg.literal) {
                    return (false, HashMap::new());
                }
                val_pos += seg.literal.len();
            }
        }

        if let Some(last) = segments.last() {
            if last.metavar.is_some() && val_pos <= value.len() {
                bindings.insert(
                    last.metavar.as_ref().unwrap().clone(),
                    value[val_pos..].to_string(),
                );
                val_pos = value.len();
            }
        }

        let matches = val_pos == value.len();
        (matches, bindings)
    }

    fn ellipsis_pattern_matches(&self, pattern_str: &str, value: &str) -> bool {
        let norm_pat: String = pattern_str.split_whitespace().collect::<Vec<_>>().join(" ");
        let norm_val: String = value.split_whitespace().collect::<Vec<_>>().join(" ");

        let parts: Vec<&str> = norm_pat.split("...").filter(|s| !s.is_empty()).collect();

        if parts.is_empty() {
            return true;
        }

        let mut search_from = 0;
        for part in parts {
            let trimmed = part.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Some(pos) = norm_val[search_from..].find(trimmed) {
                search_from += pos + trimmed.len();
            } else {
                let no_space: String = trimmed.split_whitespace().collect();
                if let Some(pos) = norm_val[search_from..].find(&no_space) {
                    search_from += pos + no_space.len();
                } else {
                    return false;
                }
            }
        }
        true
    }
}
