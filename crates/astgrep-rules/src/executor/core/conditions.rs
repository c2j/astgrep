//! Conditions evaluation implementation
//!
//! This module contains condition evaluation methods for pattern matching

use super::*;

impl AdvancedRuleExecutor {
    /// Check if pattern conditions are satisfied
    pub(super) fn check_pattern_conditions(
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
    pub(super) fn evaluate_condition(
        &self,
        condition: &Condition,
        match_result: &SemgrepMatchResult,
        _dataflow_analysis: Option<&DataFlowAnalysis>,
        full_source: &str,
    ) -> Result<bool> {
        match condition {
            Condition::MetavariableRegex(metavar_regex) => {
                // Check if metavariable exists and matches regex
                if let Some(metavar_value) = match_result.bindings.get(&metavar_regex.metavariable)
                {
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
                eprintln!(
                    "DEBUG evaluate_condition: MetavariableComparison for '{}', bindings: {:?}",
                    metavar_comp.metavariable,
                    match_result.bindings.keys().collect::<Vec<_>>()
                );
                if let Some(metavar_value) = match_result.bindings.get(&metavar_comp.metavariable) {
                    eprintln!(
                        "DEBUG: Found value '{}' for metavariable '{}'",
                        metavar_value, metavar_comp.metavariable
                    );

                    // Try to resolve the variable to its constant value using constant propagation
                    let resolved_value = if let Some(ref propagator) = self.constant_propagator {
                        // Get the location of the matched node
                        if let Some((start_line, start_col, _, _)) = match_result.node.location() {
                            use astgrep_dataflow::constant_propagation::SourceLocation;
                            let location = SourceLocation::new(start_line, start_col);

                            // Try to get the constant value at this location
                            if let Some(constant) =
                                propagator.get_variable_value_at_location(metavar_value, location)
                            {
                                let constant_str = constant
                                    .to_string_value()
                                    .unwrap_or_else(|| metavar_value.clone());
                                eprintln!(
                                    "DEBUG: Resolved variable '{}' to constant '{}' at {:?}",
                                    metavar_value, constant_str, location
                                );
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

                    self.evaluate_comparison(
                        &resolved_value,
                        &metavar_comp.operator,
                        &metavar_comp.value,
                    )
                } else {
                    eprintln!(
                        "DEBUG: Metavariable '{}' not found in bindings",
                        metavar_comp.metavariable
                    );
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
                // Custom condition evaluation
                self.evaluate_custom_condition(custom_condition, match_result)
            }
        }
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
            ComparisonOperator::PythonExpression(expr) => {
                // For now, we'll implement a simplified version
                // In a full implementation, this would use a Python interpreter
                self.evaluate_python_expression(metavar_value, expr)
            }
        }
    }

    /// Evaluate name constraint (module/namespace patterns)
    pub(super) fn evaluate_name_constraint(&self, value: &str, name_pattern: &str) -> Result<bool> {
        // Support glob-like patterns for module/namespace matching
        if name_pattern.contains("*") {
            // Convert glob pattern to regex
            let regex_pattern = name_pattern.replace(".", "\\.").replace("*", ".*");
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
    pub(super) fn evaluate_python_expression(&self, value: &str, expr: &str) -> Result<bool> {
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

        // For now, just return true for unsupported expressions
        eprintln!("DEBUG: Expression '{}' not handled, returning true", expr);
        Ok(true)
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
            _ => Ok(true), // Default to true for unknown conditions
        }
    }
}
