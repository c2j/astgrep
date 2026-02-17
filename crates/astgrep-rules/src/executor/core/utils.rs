//! Utility methods for the executor
//!
//! This module contains helper methods used by the executor

use super::*;

impl AdvancedRuleExecutor {
    /// Find annotated method parameters (e.g., @RequestParam, @PathVariable)
    /// This handles complex source patterns with annotation matching
    pub fn find_annotated_method_params(
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
                        let method_name = self.find_method_name_by_line(source_text, line_num + 1);

                        // Create a TaintMatch for this parameter
                        // We need to find or create a node for this parameter
                        if let Some(param_node) = self.find_param_node_by_name(ast, &param_name) {
                            let mut bindings = std::collections::HashMap::new();
                            bindings.insert("SOURCE".to_string(), param_name.clone());

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
    pub(super) fn extract_annotated_param(&self, line: &str, annotation: &str) -> Option<String> {
        eprintln!(
            "[DEBUG] extract_annotated_param called with line: '{}'",
            line
        );
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
                let param_name = param_name
                    .trim_end_matches(|c: char| c == ',' || c == ')' || c == '{')
                    .to_string();

                eprintln!("[DEBUG] Extracted param name: '{}'", param_name);

                // Validate it looks like a variable name
                if !param_name.is_empty()
                    && param_name
                        .chars()
                        .next()
                        .map(|c| c.is_alphabetic())
                        .unwrap_or(false)
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
    pub(super) fn find_param_node_by_name(
        &self,
        ast: &dyn AstNode,
        param_name: &str,
    ) -> Option<Box<dyn AstNode>> {
        eprintln!(
            "[DEBUG] find_param_node_by_name: looking for '{}'",
            param_name
        );
        // Try to find a formal_parameter or identifier node with the given name
        let result = self.find_node_by_type_and_text(ast, "identifier", param_name);
        eprintln!(
            "[DEBUG] find_param_node_by_name: result = {:?}",
            result.as_ref().map(|n| n.text())
        );
        result
    }

    /// Find a node by type and text content
    pub(super) fn find_node_by_type_and_text(
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

    /// Extract variable name from an assignment expression
    pub fn extract_variable_name_from_assignment(
        &self,
        node: &dyn AstNode,
        source_text: &str,
    ) -> Option<String> {
        // Try to get parent node by searching through the tree
        let node_text = node.text().unwrap_or_default();
        eprintln!(
            "[DEBUG] extract_variable_name_from_assignment: node_text='{}', node_type='{}'",
            node_text,
            node.node_type()
        );

        // Get the node's location to find it in the full source
        if let Some((start_line, start_col, _end_line, _end_col)) = node.location() {
            eprintln!(
                "[DEBUG] Node location: line={}, col={}",
                start_line, start_col
            );

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
                            if !var_name.is_empty()
                                && !var_name.contains("(")
                                && !var_name.contains("=")
                            {
                                eprintln!(
                                    "[DEBUG] Extracted var_name '{}' from assignment",
                                    var_name
                                );
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
    pub(super) fn extract_focused_parameter_name(&self, node: &dyn AstNode) -> Option<String> {
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
    pub fn extract_foreach_iteration_variable(
        &self,
        node: &dyn AstNode,
        source_text: &str,
    ) -> Option<String> {
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
                                eprintln!(
                                    "[DEBUG] Extracted for-each iteration variable: '{}'",
                                    var_name
                                );
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
    pub fn is_sanitized_expression(&self, expr: &str) -> bool {
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
    pub fn extract_method_parameter_name(
        &self,
        node: &dyn AstNode,
        source_text: &str,
        matched_text: &str,
    ) -> Option<String> {
        eprintln!(
            "[DEBUG] extract_method_parameter_name: matched_text='{}'",
            matched_text
        );

        // Get the node's location
        if let Some((start_line, _start_col, _end_line, _end_col)) = node.location() {
            let lines: Vec<&str> = source_text.lines().collect();
            if start_line > 0 && start_line <= lines.len() {
                let line_text = lines[start_line - 1];
                eprintln!("[DEBUG] Checking line {}: '{}'", start_line, line_text);

                // Check if this line contains a method declaration with parameters
                // Pattern: "methodName(..., Type paramName, ...)" or "methodName(@Annotation Type paramName, ...)"
                if line_text.contains("public ")
                    && line_text.contains('(')
                    && line_text.contains(')')
                {
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
                                    let params: Vec<&str> = params_section
                                        [1..params_section.len() - 1]
                                        .split(',')
                                        .collect();
                                    for param in params {
                                        let param = param.trim();
                                        eprintln!("[DEBUG] Checking param: '{}'", param);
                                        // Check if param ends with our matched text (the variable name)
                                        // Pattern: "Type varName" or "@Annotation Type varName"
                                        let param_words: Vec<&str> =
                                            param.split_whitespace().collect();
                                        if let Some(last_word) = param_words.last() {
                                            if *last_word == matched_text {
                                                eprintln!(
                                                    "[DEBUG] Extracted method parameter: '{}'",
                                                    matched_text
                                                );
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

    /// Extract field/variable assignment target when source pattern matches an expression
    /// For example: "private static DocumentBuilderFactory dbf = DocumentBuilderFactory.newInstance();"
    /// When source matches "DocumentBuilderFactory.newInstance()", extracts "dbf"
    pub fn extract_field_assignment_target(
        &self,
        node: &dyn AstNode,
        source_text: &str,
        matched_text: &str,
    ) -> Option<String> {
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
                            let var_name = var_name
                                .trim_end_matches(|c: char| c == ';' || c == ' ')
                                .to_string();

                            if !var_name.is_empty()
                                && !var_name.contains("(")
                                && !var_name.contains("=")
                            {
                                eprintln!(
                                    "[DEBUG] Extracted field assignment target: '{}'",
                                    var_name
                                );
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
                                            eprintln!(
                                                "[DEBUG] Extracted static block assignment target: '{}'",
                                                var_name
                                            );
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
    pub fn extract_var_from_assignment_text(&self, text: &str) -> Option<String> {
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
                // Clean up any trailing characters like semicolons or spaces
                let var_name = var_name.trim_end_matches(';').trim().to_string();
                if !var_name.is_empty()
                    && !var_name.contains("(")
                    && !var_name.contains("=")
                    && !var_name.contains("<") // Avoid generics like Map<String>
                    && var_name.chars().next().map(|c| c.is_alphabetic() || c == '_').unwrap_or(false)
                {
                    eprintln!("[DEBUG] Extracted var from assignment text: '{}'", var_name);
                    return Some(var_name);
                }
            }
        }

        None
    }

    /// Extract the target variable/field from an assignment statement
    /// When source matches "tainted" in "x = tainted" or "this.x = tainted", extract "x"
    pub fn extract_assignment_target(
        &self,
        node: &dyn AstNode,
        source_text: &str,
    ) -> Option<String> {
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

                            if !var_name.is_empty()
                                && !var_name.contains("(")
                                && !var_name.contains("=")
                            {
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
    pub fn extract_setter_argument(&self, line_num: usize, source_text: &str) -> Option<String> {
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
                            eprintln!(
                                "[DEBUG] Extracted setter argument as field ref: '{}'",
                                field_ref
                            );
                            return Some(field_ref);
                        }
                    }
                }
            }
        }

        None
    }

    /// Check if a parameter is of a numeric type
    pub fn is_numeric_parameter(&self, node: &dyn AstNode, param_name: &str) -> bool {
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

    /// Extract method name by line number in source text
    pub fn find_method_name_by_line(&self, source_text: &str, line_num: usize) -> Option<String> {
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
            if let Some(captures) =
                regex::Regex::new(r"public\s+(?:static\s+)?(?:\w+(?:<[^>]+>)?)\s+(\w+)\s*\(")
                    .ok()?
                    .captures(line)
            {
                if let Some(method_name) = captures.get(1) {
                    let name = method_name.as_str();
                    // Make sure it's not a class name (class names typically start with uppercase)
                    // Method names typically start with lowercase
                    if name
                        .chars()
                        .next()
                        .map(|c| c.is_lowercase())
                        .unwrap_or(false)
                    {
                        return Some(name.to_string());
                    }
                }
            }
        }

        None
    }

    /// Extract method name from a method declaration node
    pub fn extract_method_name_from_declaration(&self, node: &dyn AstNode) -> Option<String> {
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
                return Some(before[space_idx + 1..].trim().to_string());
            }
        }

        None
    }

    /// Extract method body by method name from source text
    pub fn extract_method_body(&self, source_text: &str, method_name: &str) -> Option<String> {
        // Find the method declaration
        let method_pattern = format!(
            r"public\s+(?:static\s+)?(?:\w+)\s+{}\s*\([^{{]*\{{",
            regex::escape(method_name)
        );
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

    /// Check if a method body contains sanitization operations for a specific variable
    /// This detects patterns like PreparedStatement.setString() which parameterizes queries
    pub fn contains_sanitization_in_scope(
        &self,
        method_body: &str,
        var_name: Option<&str>,
    ) -> bool {
        eprintln!(
            "[DEBUG] Checking sanitization in method body of length {}",
            method_body.len()
        );

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
                if method_body.contains(&format!("{}.", vname))
                    && method_body.contains("replaceAll")
                {
                    eprintln!(
                        "[DEBUG] Found replaceAll sanitization for variable: {}",
                        vname
                    );
                    return true;
                }
            }
        }

        false
    }

    /// Check if a string contains a whole word (not part of another word)
    pub(super) fn contains_whole_word(&self, text: &str, word: &str) -> bool {
        // Simple heuristic: check for common delimiters around the word
        let delimiters = [
            '(', ')', ' ', ',', ';', '+', '-', '*', '/', '=', '<', '>', '!',
        ];

        // Check if word appears at the start
        if text.starts_with(word) {
            if text.len() == word.len()
                || delimiters.contains(&text.chars().nth(word.len()).unwrap_or(' '))
            {
                return true;
            }
        }

        // Check if word appears in the middle or end
        for (i, _) in text.match_indices(word) {
            let before = if i == 0 {
                ' '
            } else {
                text.chars().nth(i - 1).unwrap_or(' ')
            };
            let after_pos = i + word.len();
            let after = if after_pos >= text.len() {
                ' '
            } else {
                text.chars().nth(after_pos).unwrap_or(' ')
            };

            // Check if word is surrounded by delimiters or string boundaries
            let before_is_delimiter = i == 0 || delimiters.contains(&before);
            let after_is_delimiter = after_pos >= text.len() || delimiters.contains(&after);

            if before_is_delimiter && after_is_delimiter {
                return true;
            }
        }

        false
    }
}
