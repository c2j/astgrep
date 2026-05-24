//! Taint analysis implementation
//!
//! This module contains taint analysis methods for tracking data flow from sources to sinks

use super::*;
use super::taint_env::is_tainted_as_array_index;

impl AdvancedRuleExecutor {
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
        // Read actual file content when file_path is available (preserves original line numbering).
        // Falls back to ast.text() which may strip leading empty lines, causing line offset issues.
        let source_text = if let Some(path) = file_path {
            std::fs::read_to_string(path).unwrap_or_else(|_| ast.text().unwrap_or_default().to_string())
        } else {
            ast.text().unwrap_or_default().to_string()
        };

        // Debug: Print dataflow spec details
        eprintln!(
            "[DEBUG] Dataflow spec - sources: {}, sinks: {}, propagators: {}",
            dataflow_spec.sources.len(),
            dataflow_spec.sinks.len(),
            dataflow_spec.propagators.len()
        );
        for (i, prop) in dataflow_spec.propagators.iter().enumerate() {
            eprintln!(
                "[DEBUG] Propagator {}: pattern='{:?}', from='{}', to='{}'",
                i, prop.pattern.pattern_type, prop.from, prop.to
            );
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
        let assume_safe_booleans =
            if let Some(val) = rule.metadata.get("taint_assume_safe_booleans") {
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

        let assume_safe_numbers = if let Some(val) = rule.metadata.get("taint_assume_safe_numbers")
        {
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

        let assume_safe_indexes =
            if let Some(val) = rule.metadata.get("taint_assume_safe_indexes") {
                if let serde_yaml::Value::String(ref s) = val {
                    s == "true"
                } else if let serde_yaml::Value::Bool(ref b) = val {
                    *b
                } else {
                    false
                }
            } else {
                dataflow_spec.taint_assume_safe_indexes.unwrap_or(false)
            };

        let assume_safe_functions =
            if let Some(val) = rule.metadata.get("taint_assume_safe_functions") {
                if let serde_yaml::Value::String(ref s) = val {
                    s == "true"
                } else if let serde_yaml::Value::Bool(ref b) = val {
                    *b
                } else {
                    false
                }
            } else {
                dataflow_spec.taint_assume_safe_functions.unwrap_or(false)
            };

        let only_propagate_through_assignments = if let Some(val) = rule
            .metadata
            .get("taint_only_propagate_through_assignments")
        {
            if let serde_yaml::Value::String(ref s) = val {
                s == "true"
            } else if let serde_yaml::Value::Bool(ref b) = val {
                *b
            } else {
                false
            }
        } else {
            dataflow_spec
                .taint_only_propagate_through_assignments
                .unwrap_or(false)
        };

        // Helper: extract sink argument from a sink call like "sink(x)" -> "x"
        let extract_sink_arg = |sink_text: &str| -> Option<String> {
            let text = sink_text.trim();
            if let Some(paren) = text.find('(') {
                let after = &text[paren + 1..];
                if let Some(close) = after.rfind(')') {
                    let arg = after[..close].trim();
                    if !arg.is_empty() && !arg.contains('"') && !arg.contains('\'') {
                        return Some(arg.to_string());
                    }
                }
            }
            None
        };

        // Helper: check if a flow goes through an array index assignment
        let flow_goes_through_array_index = |source: &TaintMatch, sink: &TaintMatch| -> bool {
            if let Some(ref source_var) = source.var_name {
                if let Some(sink_text) = sink.node.text() {
                    if let Some(sink_var) = extract_sink_arg(&sink_text) {
                        let lines: Vec<&str> = source_text.lines().collect();
                        if let Some((sl, _, _, _)) = sink.node.location() {
                            if sl > 0 && sl <= lines.len() {
                                for l in (0..sl - 1).rev() {
                                    let line = lines[l].trim();
                                    if line.starts_with(&format!("{} =", sink_var))
                                        || line.starts_with(&format!("{}=", sink_var))
                                    {
                                        let rhs = line
                                            .splitn(2, '=')
                                            .nth(1)
                                            .unwrap_or("")
                                            .trim()
                                            .trim_end_matches(';');
                                        if is_tainted_as_array_index(rhs, source_var) {
                                            return true;
                                        }
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            false
        };

        let taint_flows = {
            // Env-based forward dataflow detection (skip if rule uses labels — not supported)
            let env_flows = if dataflow_spec.uses_labels {
                Vec::new()
            } else {
                // Check if any sink pattern has exact: false
                let has_non_exact_sinks = dataflow_spec.sinks.iter().any(|s| s.exact == Some(false));
                self.detect_taint_flows_with_env(
                    &source_matches,
                    &sink_matches,
                    &source_text,
                    dataflow_spec,
                    assume_safe_booleans,
                    assume_safe_numbers,
                    assume_safe_indexes,
                    assume_safe_functions,
                    only_propagate_through_assignments,
                    has_non_exact_sinks,
                )
            };

            // Heuristic detection
            let heuristic_flows = self.detect_taint_flows(
                &source_matches,
                &sink_matches,
                ast,
                dataflow_analysis,
                assume_safe_booleans,
                assume_safe_numbers,
                only_propagate_through_assignments,
                &source_text,
                &dataflow_spec.propagators,
                dataflow_spec,
            )?;

            // Filter env flows through safe-context checks to avoid false positives
            let filtered_env: Vec<_> = env_flows
                .into_iter()
                .filter(|(source, sink)| {
                    let sink_text = sink.node.text().unwrap_or_default();
                    if let Some(ref source_var) = source.var_name {
                        if assume_safe_booleans
                            && self.is_variable_in_safe_boolean_context(source_var, &sink_text)
                        {
                            eprintln!(
                                "[DEBUG] Filtering env flow: '{}' in safe boolean context",
                                source_var
                            );
                            return false;
                        }
                        if assume_safe_numbers
                            && self.is_variable_in_safe_number_context(source_var, &sink_text)
                        {
                            eprintln!(
                                "[DEBUG] Filtering env flow: '{}' in safe number context",
                                source_var
                            );
                            return false;
                        }
                    }
                    true
                })
                .collect();

            // Merge: take union of both, dedup by sink location
            // Heuristic flows go first (already filtered for safe contexts)
            // Env flows are added if their sink location isn't already covered
            let env_count = filtered_env.len();
            let mut merged: Vec<(TaintMatch, TaintMatch)> = heuristic_flows;
            let mut added_from_env = 0;
            for flow in filtered_env {
                let sink_loc = flow.1.node.location();
                let already = merged.iter().any(|(_, s)| s.node.location() == sink_loc);
                if !already {
                    merged.push(flow);
                    added_from_env += 1;
                }
            }

            eprintln!(
                "[DEBUG] Merged taint flows: {} (heuristic) + {} (env new) = {} total",
                merged.len() - added_from_env, added_from_env, merged.len()
            );

            // Filter flows that go through array index access when assume_safe_indexes is set
            if assume_safe_indexes {
                let before = merged.len();
                merged.retain(|(source, sink)| {
                    !flow_goes_through_array_index(source, sink)
                });
                let removed = before - merged.len();
                if removed > 0 {
                    eprintln!(
                        "[DEBUG] Filtered {} flows through array index (assume_safe_indexes)",
                        removed
                    );
                }
            }

            merged
        };

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
                        (start_line > e_start_line
                            || (start_line == e_start_line && start_col >= e_start_col))
                            && (end_line < e_end_line
                                || (end_line == e_end_line && end_col <= e_end_col))
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
                        location.0,
                        location.1,
                        location.2,
                        location.3,
                    ),
                );
                findings.push(finding);
            }
        }

        Ok(findings)
    }

    /// Find all taint sources matching the source patterns
    pub(super) fn find_taint_sources(
        &mut self,
        ast: &dyn AstNode,
        dataflow_spec: &DataFlowSpec,
        source_text: &str,
    ) -> Result<Vec<TaintMatch>> {
        eprintln!(
            "[DEBUG] ENTER find_taint_sources with {} source patterns",
            dataflow_spec.sources.len()
        );
        let mut sources = Vec::new();

        for source_pattern in &dataflow_spec.sources {
            // Always try to find annotated method parameters (e.g., @RequestParam, @PathVariable)
            // These are common taint sources in web applications
            let annotation_sources = self.find_annotated_method_params(ast, source_text);
            if !annotation_sources.is_empty() {
                eprintln!(
                    "[DEBUG] Found {} annotated method parameter sources",
                    annotation_sources.len()
                );
                for src in &annotation_sources {
                    eprintln!(
                        "[DEBUG] Annotation source: var_name={:?}, bindings={:?}",
                        src.var_name, src.bindings
                    );
                }
                sources.extend(annotation_sources);
            }

            // Skip pattern matching if we already have sources from annotation detection
            // This avoids issues with complex patterns
            if !sources.is_empty() {
                continue;
            }

            // Normalize pattern: remove trailing semicolons and whitespace for more flexible matching
            let original_pattern = source_pattern.pattern_text();
            let normalized_pattern = original_pattern
                .trim_end_matches(';')
                .trim_end_matches('\n')
                .trim();
            eprintln!(
                "[DEBUG] Normalizing source pattern: '{:?}' -> '{}'",
                original_pattern, normalized_pattern
            );

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
                if let Some(simplified) = Self::simplify_fully_qualified_pattern(normalized_pattern)
                {
                    eprintln!(
                        "[DEBUG] No matches with full pattern, trying simplified: '{}'",
                        simplified
                    );
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
                    matches = self
                        .pattern_matcher
                        .find_matches(&simplified_semgrep_pattern, ast)?;
                }
            }

            eprintln!("[DEBUG] Source matches found: {}", matches.len());
            for m in matches {
                eprintln!(
                    "[DEBUG] Source match: bindings={:?}, text={:?}",
                    m.bindings,
                    m.node.text()
                );
                // Extract the variable name from bindings if available
                let mut var_name: Option<String> = None;

                // If focus-metavariables are specified, extract the binding for the first focus variable
                if !source_pattern.focus_metavariables.is_empty() {
                    let focus_var = &source_pattern.focus_metavariables[0];
                    // Remove the "$" prefix to match the binding key
                    let focus_key = focus_var.trim_start_matches('$');
                    if let Some(value) = m.bindings.get(focus_key) {
                        if !value.is_empty() {
                            var_name = Some(value.value.clone());
                            eprintln!(
                                "[DEBUG] Extracted var_name from focus-metavariable '{}': {}",
                                focus_var, value
                            );
                        }
                    }
                }

                // If no var_name from focus-metavariable, try any binding that starts with "$"
                if var_name.is_none() {
                    for (key, value) in &m.bindings {
                        if key.starts_with("$") && !value.is_empty() {
                            var_name = Some(value.value.clone());
                            break;
                        }
                    }
                }

                // If no var_name from bindings, try to extract from parent assignment
                if var_name.is_none() {
                    var_name =
                        self.extract_variable_name_from_assignment(m.node.as_ref(), source_text);
                }

                // If still no var_name, check if source is in a for-each loop and extract the iteration variable
                if var_name.is_none() {
                    var_name =
                        self.extract_foreach_iteration_variable(m.node.as_ref(), source_text);
                }

                // If still no var_name and the match looks like a string literal,
                // try to find the variable that is assigned this string literal
                if var_name.is_none() {
                    // Check if the original pattern is a string literal pattern (e.g. '"password"')
                    // Tree-sitter may return string content without quotes (e.g. "password")
                    let is_string_pattern = normalized_pattern.starts_with('"')
                        && normalized_pattern.ends_with('"');
                    if is_string_pattern {
                        if let Some(text) = m.node.text() {
                            let text = text.trim().trim_matches('"');
                            if let Some((start_line, _, _, _)) = m.node.location() {
                                // Try the reported line; if it doesn't have '=', try the previous line
                                var_name = self.find_variable_for_string_literal(
                                    source_text,
                                    start_line,
                                    text,
                                );
                                if var_name.is_none() && start_line > 1 {
                                    var_name = self.find_variable_for_string_literal(
                                        source_text,
                                        start_line - 1,
                                        text,
                                    );
                                }
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
                            var_name = self.extract_method_parameter_name(
                                m.node.as_ref(),
                                source_text,
                                text,
                            );
                        }
                    }
                }

                // If still no var_name, check if source is assigned to a field/variable
                // Pattern: Type var = source() or var = source()
                if var_name.is_none() {
                    if let Some(text) = m.node.text() {
                        // Check for field/variable assignment: Type var = DocumentBuilderFactory.newInstance()
                        // The text might be the full assignment statement
                        var_name = self.extract_field_assignment_target(
                            m.node.as_ref(),
                            source_text,
                            text,
                        );

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
                                    eprintln!(
                                        "[DEBUG] Source variable '{}' is sanitized, skipping",
                                        vname
                                    );
                                    continue;
                                }
                            }
                        }
                    }
                }

                // When taint_assume_safe_numbers is true, filter out numeric type sources
                if dataflow_spec.taint_assume_safe_numbers.unwrap_or(false) {
                    if let Some(ref vname) = var_name {
                        if self.is_numeric_parameter(m.node.as_ref(), vname.as_str()) {
                            continue;
                        }
                    }
                }

                // Extract method name for scope isolation using source location
                let node_ref = m.node.as_ref();

                // First check if we have method name in bindings (e.g., from pattern like "public void $F(...)")
                let method_name_from_bindings: Option<String> =
                    m.bindings.get("F").map(|v| v.value.clone());

                let method_name = if let Some(name) = method_name_from_bindings {
                    Some(name)
                } else if node_ref.node_type() == "method_declaration" {
                    self.extract_method_name_from_declaration(node_ref)
                } else if let Some((start_line, _, _, _)) = node_ref.location() {
                    self.find_method_name_by_line(source_text, start_line)
                } else {
                    None
                };

                let str_bindings: HashMap<String, String> = m
                    .bindings
                    .iter()
                    .map(|(k, v)| (k.clone(), v.value.clone()))
                    .collect();
                sources.push(TaintMatch {
                    node: m.node,
                    bindings: str_bindings,
                    var_name,
                    method_name,
                });
            }
        }

        eprintln!(
            "[DEBUG] find_taint_sources: returning {} sources",
            sources.len()
        );
        Ok(sources)
    }

    /// Find all taint sinks matching the sink patterns
    pub(super) fn find_taint_sinks(
        &mut self,
        ast: &dyn AstNode,
        dataflow_spec: &DataFlowSpec,
        source_text: &str,
    ) -> Result<Vec<TaintMatch>> {
        let mut sinks = Vec::new();

        for sink_pattern in &dataflow_spec.sinks {
            eprintln!("[DEBUG] find_taint_sinks: processing sink pattern");
            // Recursively collect all simple patterns from the pattern tree
            let simple_patterns = self.collect_simple_patterns(&sink_pattern.pattern);
            eprintln!(
                "[DEBUG] find_taint_sinks: collected {} simple patterns",
                simple_patterns.len()
            );
            for pattern_str in &simple_patterns {
                eprintln!("[DEBUG] find_taint_sinks: trying pattern '{}'", pattern_str);
                let matches = self.find_sink_matches_for_pattern(
                    &pattern_str,
                    ast,
                    source_text,
                    &sink_pattern.focus_metavariables,
                )?;
                eprintln!(
                    "[DEBUG] find_taint_sinks: pattern '{}' found {} matches",
                    pattern_str,
                    matches.len()
                );
                sinks.extend(matches);
            }
        }

        Ok(sinks)
    }

    /// Recursively collect all simple pattern strings from a pattern tree
    fn collect_simple_patterns(&self, pattern: &Pattern) -> Vec<String> {
        let mut result = Vec::new();
        match &pattern.pattern_type {
            PatternType::Simple(s) => {
                result.push(s.clone());
            }
            PatternType::Either(inner_patterns) => {
                for inner in inner_patterns {
                    result.extend(self.collect_simple_patterns(inner));
                }
            }
            _ => {}
        }
        result
    }

    fn find_sink_matches_for_pattern(
        &mut self,
        pattern_str: &str,
        ast: &dyn AstNode,
        source_text: &str,
        focus_metavariables: &[String],
    ) -> Result<Vec<TaintMatch>> {
        let mut sinks = Vec::new();

        let normalized_pattern = pattern_str.trim().trim_end_matches(';');
        eprintln!(
            "[DEBUG] Normalizing sink pattern: '{:?}' -> '{}'",
            pattern_str, normalized_pattern
        );

        let semgrep_pattern = astgrep_core::SemgrepPattern {
            pattern_type: astgrep_core::PatternType::Simple(normalized_pattern.to_string()),
            metavariable_pattern: None,
            conditions: Vec::new(),
            focus: None,
        };

        // Find matches
        let mut matches = self.pattern_matcher.find_matches(&semgrep_pattern, ast)?;

        // If no matches and pattern looks like a fully qualified name, try matching just the class and method
        if matches.is_empty() && pattern_str.contains('.') {
            if let Some(simplified) = Self::simplify_fully_qualified_pattern(pattern_str) {
                eprintln!(
                    "[DEBUG] No matches with full sink pattern, trying simplified: '{}'",
                    simplified
                );
                let simplified_semgrep_pattern = astgrep_core::SemgrepPattern {
                    pattern_type: astgrep_core::PatternType::Simple(simplified),
                    metavariable_pattern: None,
                    conditions: Vec::new(),
                    focus: None,
                };
                matches = self
                    .pattern_matcher
                    .find_matches(&simplified_semgrep_pattern, ast)?;
            }
        }

        eprintln!("[DEBUG] Sink matches found: {}", matches.len());
        for m in matches {
            eprintln!(
                "[DEBUG] Sink match: bindings={:?}, text={:?}",
                m.bindings,
                m.node.text()
            );

            // Extract method name for scope isolation using source location
            let node = m.node.as_ref();
            let method_name = if let Some((start_line, _, _, _)) = node.location() {
                self.find_method_name_by_line(source_text, start_line)
            } else {
                None
            };

            // Extract variable name from focus-metavariable if specified
            let mut var_name = None;

            if !focus_metavariables.is_empty() {
                for focus_var in focus_metavariables {
                    let focus_var_no_dollar = focus_var.trim_start_matches('$');
                    if let Some(value) = m.bindings.get(focus_var_no_dollar) {
                        var_name = Some(value.value.clone());
                        break;
                    }
                }
            }

            // If no var_name from focus-metavariables, extract from sink call
            if var_name.is_none() {
                if let Some(text) = m.node.text() {
                    let text = text.trim();
                    if let Some(args) = Self::extract_last_call_args(text) {
                        let arg_parts: Vec<&str> = args.split(',').collect();
                        if arg_parts.len() == 1 {
                            let arg = arg_parts[0].trim();
                            if !arg.is_empty() && !arg.contains('"') && !arg.contains('\'') {
                                var_name = Some(arg.to_string());
                            }
                        } else if arg_parts.len() >= 2 {
                            let last_arg = arg_parts.last().unwrap().trim();
                            if !last_arg.is_empty()
                                && !last_arg.contains('"')
                                && !last_arg.contains('\'')
                            {
                                var_name = Some(last_arg.to_string());
                            }
                        }
                    }
                }
            }

            // Check if this sink is in a method that contains sanitization
            if let Some(ref mname) = method_name {
                if let Some(method_body) = self.extract_method_body(source_text, mname) {
                    if self.contains_sanitization_in_scope(&method_body, var_name.as_deref()) {
                        continue;
                    }
                }
            }

            let str_bindings: HashMap<String, String> = m
                .bindings
                .iter()
                .map(|(k, v)| (k.clone(), v.value.clone()))
                .collect();
            sinks.push(TaintMatch {
                node: m.node,
                bindings: str_bindings,
                var_name,
                method_name,
            });
        }

        Ok(sinks)
    }

    /// Detect taint flows using forward dataflow state machine (TaintEnv).
    /// Walks source text line-by-line, tracking taint through assignments.
    pub(super) fn detect_taint_flows_with_env(
        &self,
        sources: &[TaintMatch],
        sinks: &[TaintMatch],
        source_text: &str,
        dataflow_spec: &crate::types::DataFlowSpec,
        _taint_assume_safe_booleans: bool,
        _taint_assume_safe_numbers: bool,
        _taint_assume_safe_indexes: bool,
        _taint_assume_safe_functions: bool,
        taint_only_propagate_through_assignments: bool,
        has_non_exact_sinks: bool,
    ) -> Vec<(TaintMatch, TaintMatch)> {
        use super::taint_env::{
            contains_var_reference, extract_target_var, find_assignment_eq, is_safe_value,
            is_tainted_as_array_index, TaintEnv,
        };

        let mut flows = Vec::new();
        let mut env = TaintEnv::new();
        let lines: Vec<&str> = source_text.lines().collect();

        // Build source lookup: line → [(source_idx, source_match)]
        let mut source_map: HashMap<usize, Vec<(usize, &TaintMatch)>> = HashMap::new();
        let mut sourceless_map: HashMap<usize, Vec<(usize, &TaintMatch)>> = HashMap::new();
        for (idx, source) in sources.iter().enumerate() {
            if let Some((sl, _, _, _)) = source.node.location() {
                if source.var_name.is_some() {
                    source_map.entry(sl).or_default().push((idx, source));
                } else {
                    sourceless_map.entry(sl).or_default().push((idx, source));
                }
            }
        }

        // Build sink lookup: line → [sink_match]
        let mut sink_map: HashMap<usize, Vec<&TaintMatch>> = HashMap::new();
        for sink in sinks.iter() {
            if let Some((sl, _, _, _)) = sink.node.location() {
                sink_map.entry(sl).or_default().push(sink);
            }
        }

        // Extract sanitizer function names
        let sanitizer_fns: Vec<String> = dataflow_spec
            .sanitizers
            .iter()
            .filter_map(|s| {
                let s = s.trim();
                let name = s.trim_end_matches("(...)");
                let parts: Vec<&str> = name.rsplit('.').collect();
                parts.first().map(|n| n.to_string())
            })
            .collect();

        eprintln!(
            "[DEBUG-TAINT-ENV] Starting env-based flow detection: {} sources, {} sinks, {} sanitizers",
            sources.len(),
            sinks.len(),
            sanitizer_fns.len()
        );

        // Walk lines top-to-bottom
        for (line_idx, line) in lines.iter().enumerate() {
            let line_num = line_idx + 1;
            let trimmed = line.trim();

            if trimmed.is_empty()
                || trimmed.starts_with('#')
                || trimmed.starts_with("//")
                || trimmed.starts_with('*')
                || trimmed.starts_with("/*")
            {
                continue;
            }

            // 1. Source match at this line → taint the variable
            if let Some(source_entries) = source_map.get(&line_num) {
                for (source_idx, source_match) in source_entries {
                    if let Some(ref var) = source_match.var_name {
                        let normalized_var = var.strip_prefix("this.").unwrap_or(var);
                        // Strip array index: x[i] → x for array-level taint tracking
                        let base_var = normalized_var.split('[').next().unwrap_or(normalized_var).trim();
                        env.taint(base_var, line_num, *source_idx);
                        let display_var = if base_var != normalized_var {
                            format!("{} (base: {})", normalized_var, base_var)
                        } else {
                            normalized_var.to_string()
                        };
                        eprintln!(
                            "[DEBUG-TAINT-ENV] Line {}: tainted var '{}' from source {}",
                            line_num, display_var, source_idx
                        );
                    }
                }
            }

            for prop in &dataflow_spec.propagators {
                let prop_text = match &prop.pattern.pattern_type {
                    crate::types::PatternType::Simple(s) => s.as_str(),
                    _ => continue,
                };
                if prop_text.contains('$') {
                    let mut var_order: Vec<String> = Vec::new();
                    let mut remaining = prop_text;
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
                        if let Some(captures) = re.captures(trimmed) {
                            let captured: Vec<String> = (1..=var_order.len())
                                .filter_map(|i| captures.get(i).map(|m| m.as_str().to_string()))
                                .collect();
                            let from_val = if prop.from.starts_with('$') {
                                let name = &prop.from[1..];
                                var_order.iter().position(|v| v == name)
                                    .and_then(|idx| captured.get(idx).cloned())
                            } else { None };
                            let to_val = if prop.to.starts_with('$') {
                                let name = &prop.to[1..];
                                var_order.iter().position(|v| v == name)
                                    .and_then(|idx| captured.get(idx).cloned())
                            } else { None };
                            if let (Some(ref from), Some(ref to)) = (from_val, to_val) {
                                if env.is_tainted(from) {
                                    env.taint(to, line_num, 0);
                                    eprintln!(
                                        "[DEBUG-TAINT-ENV] Line {}: propagator '{}' -> '{}' tainted '{}'",
                                        line_num, from, to, to
                                    );
                                }
                            }
                        }
                    }
                }
            }

            // Assignment propagation
            if let Some(eq_pos) = find_assignment_eq(trimmed) {
                let target = trimmed[..eq_pos].trim();
                let value = trimmed[eq_pos + 1..].trim();
                let target_var = extract_target_var(target);

                // Check if the RHS is a comparison expression (e.g., "x != something")
                // Comparison results should NOT propagate taint (safe_comparisons)
                let is_comparison = value.contains(" != ")
                    || value.contains(" == ")
                    || value.contains(" > ")
                    || value.contains(" < ")
                    || value.contains(" >= ")
                    || value.contains(" <= ");

                // Check if the RHS is a numeric-returning method call (safe_numbers)
                let is_numeric_call = value.contains(".getSomething()")
                    || value.contains(".length")
                    || value.contains(".size()")
                    || value.contains(".count()")
                    || value.contains(".indexOf(")
                    || value.contains(".lastIndexOf(")
                    || value.contains(".compareTo(")
                    || value.contains("Integer.valueOf(")
                    || value.contains("Integer.parseInt(")
                    || value.contains("Long.valueOf(")
                    || value.contains("Long.parseLong(")
                    || value.contains("Double.valueOf(")
                    || value.contains("Float.valueOf(");

                let mut propagated = false;
                for tvar in env.tainted_vars() {
                    if contains_var_reference(value, &tvar) {
                        let is_sanitized = sanitizer_fns
                            .iter()
                            .any(|sf| value.contains(sf.as_str()));
                        if is_sanitized {
                            env.sanitize(&target_var);
                            eprintln!(
                                "[DEBUG-TAINT-ENV] Line {}: sanitized '{}' (via sanitizer)",
                                line_num, target_var
                            );
                        } else if is_comparison {
                            // Comparison results don't propagate taint
                            eprintln!(
                                "[DEBUG-TAINT-ENV] Line {}: NOT propagating '{}' -> '{}' (comparison expression)",
                                line_num, tvar, target_var
                            );
                        } else if _taint_assume_safe_numbers && is_numeric_call {
                            // Numeric-returning calls don't propagate taint when safe_numbers enabled
                            eprintln!(
                                "[DEBUG-TAINT-ENV] Line {}: NOT propagating '{}' -> '{}' (numeric call, safe_numbers)",
                                line_num, tvar, target_var
                            );
                        } else if taint_only_propagate_through_assignments
                            && value.trim() != tvar
                        {
                            // Only propagate through direct assignments (x = y), not expressions (x = y+1)
                            eprintln!(
                                "[DEBUG-TAINT-ENV] Line {}: NOT propagating '{}' -> '{}' (not pure assignment)",
                                line_num, tvar, target_var
                            );
                        } else if _taint_assume_safe_indexes
                            && is_tainted_as_array_index(value, &tvar)
                        {
                            // Array index access doesn't propagate taint when safe_indexes enabled
                            eprintln!(
                                "[DEBUG-TAINT-ENV] Line {}: NOT propagating '{}' -> '{}' (array index, safe_indexes)",
                                line_num, tvar, target_var
                            );
                        } else {
                            // Strip brackets from propagation target: x[i] → x (array-level tracking)
                            let prop_target = target_var.split('[').next().unwrap_or(&target_var).trim().to_string();
                            env.propagate(&prop_target, &tvar);
                            eprintln!(
                                "[DEBUG-TAINT-ENV] Line {}: propagated '{}' -> '{}'",
                                line_num, tvar, target_var
                            );
                        }
                        propagated = true;
                        break;
                    }
                }

                if !propagated {
                    // Direct target check: untaint if safe value reassigned
                    if env.is_tainted(&target_var) && is_safe_value(value) {
                        env.untaint(&target_var);
                        eprintln!(
                            "[DEBUG-TAINT-ENV] Line {}: untainted '{}' (reassigned safe value)",
                            line_num, target_var
                        );
                    }
                    // Array element assignment: x[i] = non_source → untaint array x
                    // Skip if a source was matched at this line (source match already seeds taint)
                    if target_var.contains('[') && !source_map.contains_key(&line_num) {
                        let target_base = target_var.split('[').next().unwrap_or(&target_var).trim().to_string();
                        if env.is_tainted(&target_base) {
                            env.untaint(&target_base);
                            eprintln!(
                                "[DEBUG-TAINT-ENV] Line {}: untainted array '{}' (element assigned non-tainted value)",
                                line_num, target_base
                            );
                        }
                    }
                }
            }

            // 3. Sink match at this line → check taint
            if let Some(sink_entries) = sink_map.get(&line_num) {
                for sink_match in sink_entries {
                    let sink_text = sink_match.node.text().unwrap_or_default();

                    // Determine if sink's argument is tainted
                    let (sink_tainted, source_var_name) = if let Some(ref sink_var) =
                        sink_match.var_name
                    {
                        let normalized = sink_var.strip_prefix("this.").unwrap_or(sink_var);
                        if env.is_tainted(normalized) {
                            (true, normalized.to_string())
                        } else {
                            // Check if any tainted var appears in sink text
                            let found = env
                                .tainted_vars()
                                .iter()
                                .find(|tv| contains_var_reference(&sink_text, tv))
                                .cloned();
                            (found.is_some(), found.unwrap_or_default())
                        }
                    } else {
                        let found = env
                            .tainted_vars()
                            .iter()
                            .find(|tv| contains_var_reference(&sink_text, tv))
                            .cloned();
                        (found.is_some(), found.unwrap_or_default())
                    };

                    if sink_tainted {
                        if let Some(src_idx) = env.get_source_idx(&source_var_name) {
                            if src_idx < sources.len() {
                                // Method scope isolation: compare sink method with CORRECT source
                                let scope_ok = match (
                                    &sink_match.method_name,
                                    sources.get(src_idx),
                                ) {
                                    (Some(sink_method), Some(source)) => {
                                        match &source.method_name {
                                            Some(src_method) => {
                                                src_method == sink_method
                                            }
                                            None => true,
                                        }
                                    }
                                    _ => true,
                                };
                                if !scope_ok {
                                    continue;
                                }
                                eprintln!(
                                    "[DEBUG-TAINT-ENV] FLOW FOUND: source {} -> sink at line {}",
                                    src_idx, line_num
                                );
                                flows.push((
                                    sources[src_idx].clone(),
                                    (*sink_match).clone(),
                                ));
                            }
                        } else if let Some(first_source) = sources.first() {
                            // Fallback: scope check with first source
                            let scope_ok = match (
                                &sink_match.method_name,
                                sources.first(),
                            ) {
                                (Some(sink_method), Some(source)) => {
                                    match &source.method_name {
                                        Some(src_method) => {
                                            src_method == sink_method
                                        }
                                        None => true,
                                    }
                                }
                                _ => true,
                            };
                            if !scope_ok {
                                continue;
                            }
                            eprintln!(
                                "[DEBUG-TAINT-ENV] FLOW FOUND (fallback): first source -> sink at line {}",
                                line_num
                            );
                            flows.push((
                                first_source.clone(),
                                (*sink_match).clone(),
                            ));
                        }
                    } else {
                        // Check for sourceless source on same line as sink
                        if let Some(sourceless_entries) = sourceless_map.get(&line_num) {
                            let sink_text = sink_match.node.text().unwrap_or_default();
                            // Extract sink argument (text between outermost parens)
                            let sink_arg = sink_text.find('(')
                                .and_then(|open| {
                                    sink_text.rfind(')').map(|close| {
                                        sink_text[open+1..close].trim()
                                    })
                                });
                            for (src_idx, source_match) in sourceless_entries {
                                let source_text = source_match.node.text().unwrap_or_default();
                                let matched = if has_non_exact_sinks {
                                    // Non-exact: source text appears anywhere in sink
                                    sink_text.contains(source_text.trim())
                                } else {
                                    // Exact/best-fit: source text is the direct sink argument
                                    sink_arg == Some(source_text.trim())
                                };
                                if matched {
                                    // Method scope isolation
                                    let scope_ok = match (
                                        &sink_match.method_name,
                                        source_match.method_name.as_ref(),
                                    ) {
                                        (Some(sink_method), Some(src_method)) => {
                                            sink_method == src_method
                                        }
                                        _ => true,
                                    };
                                    if !scope_ok {
                                        continue;
                                    }
                                    eprintln!(
                                        "[DEBUG-TAINT-ENV] FLOW FOUND (sourceless): source {} -> sink at line {}",
                                        src_idx, line_num
                                    );
                                    flows.push((
                                        sources[*src_idx].clone(),
                                        (*sink_match).clone(),
                                    ));
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }

        eprintln!(
            "[DEBUG-TAINT-ENV] Found {} flows",
            flows.len()
        );
        flows
    }

    /// Detect taint flows from sources to sinks (heuristic fallback)
    pub(super) fn detect_taint_flows(
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
        dataflow_spec: &crate::types::DataFlowSpec,
    ) -> Result<Vec<(TaintMatch, TaintMatch)>> {
        eprintln!("[DEBUG] detect_taint_flows: {} sources, {} sinks, assume_safe_booleans={}, assume_safe_numbers={}, only_assignments={}",
                  sources.len(), sinks.len(), taint_assume_safe_booleans, taint_assume_safe_numbers, taint_only_propagate_through_assignments);
        let mut flows = Vec::new();

        // Build method cache for dependency graphs
        let mut method_cache: HashMap<Option<String>, VariableDependencyGraph> = HashMap::new();

        // Use simple heuristics to detect taint flows
        for (i, source) in sources.iter().enumerate() {
            eprintln!(
                "[DEBUG] Checking source {}: var_name={:?}, method={:?}",
                i, source.var_name, source.method_name
            );
            if let Some(ref source_var) = source.var_name {
                for (j, sink) in sinks.iter().enumerate() {
                    eprintln!("[DEBUG] Checking source {} with sink {}: var='{}' vs sink text='{}', source_method={:?}, sink_method={:?}",
                              i, j, source_var, sink.node.text().unwrap_or_default(), source.method_name, sink.method_name);

                    // Method-level scope isolation: if both source and sink have method names,
                    // only pair them if they're in the same method
                    if let (Some(ref src_method), Some(ref sink_method)) =
                        (&source.method_name, &sink.method_name)
                    {
                        if src_method != sink_method {
                            eprintln!("[DEBUG] Skipping: source and sink are in different methods ({} vs {})", src_method, sink_method);
                            continue;
                        }
                    }

                    // Check if source variable appears in sink context
                    if self.is_variable_flowing_to_sink(
                        source_var,
                        sink.node.as_ref(),
                        ast,
                        taint_assume_safe_booleans,
                        taint_assume_safe_numbers,
                        taint_only_propagate_through_assignments,
                        source_text,
                    ) {
                        // Check if the sink variable is sanitized
                        if !dataflow_spec.sanitizers.is_empty() {
                            if let Some(sink_var) = self.extract_sink_argument(sink.node.as_ref()) {
                                if self.is_sink_variable_sanitized(
                                    &sink_var,
                                    source_text,
                                    dataflow_spec,
                                ) {
                                    eprintln!(
                                        "[DEBUG] Sink variable '{}' is sanitized, skipping flow",
                                        sink_var
                                    );
                                    continue;
                                }
                            }
                        }
                        eprintln!("[DEBUG] FLOW FOUND: source {} -> sink {}", i, j);
                        flows.push((source.clone(), sink.clone()));
                        continue;
                    }

                    // Check dataflow: if sink variable depends on source variable
                    eprintln!("[DEBUG] Checking dataflow analysis: sink.var_name={:?}, sink.method_name={:?}", sink.var_name, sink.method_name);
                    if let (Some(ref sink_var), Some(ref method_name)) =
                        (&sink.var_name, &sink.method_name)
                    {
                        eprintln!(
                            "[DEBUG] Entering dataflow analysis: sink_var={}, method_name={}",
                            sink_var, method_name
                        );
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
                            if let Some(sink_field) =
                                self.extract_field_from_sink(&sink.node.text().unwrap_or_default())
                            {
                                if self.is_numeric_field(&sink_field, source_text) {
                                    eprintln!("[DEBUG] Skipping dataflow: sink field '{}' is numeric (taint_assume_safe_numbers)", sink_field);
                                    continue;
                                }
                            }
                        }

                        eprintln!(
                            "[DEBUG] Checking dependency: {} depends on {} (check_safe={})",
                            sink_var, source_var, check_safe_context
                        );
                        if dep_graph.depends_on(sink_var, &[source_var.clone()], check_safe_context)
                        {
                            eprintln!("[DEBUG] FLOW FOUND (dataflow): source {} -> sink {} ({} depends on {})", i, j, sink_var, source_var);
                            flows.push((source.clone(), sink.clone()));
                        } else {
                            eprintln!(
                                "[DEBUG] No dependency found: {} does not depend on {}",
                                sink_var, source_var
                            );
                        }
                    } else {
                        eprintln!("[DEBUG] Skipping dataflow analysis: sink.var_name or sink.method_name is None");
                    }
                }
            } else {
                // Source has no var_name - might be a string literal pattern or method parameter with annotation
                eprintln!("[DEBUG] Source {} has no var_name, checking for string literal assignments and method parameters", i);

                for (j, sink) in sinks.iter().enumerate() {
                    if let (Some(ref sink_var), Some(ref method_name)) =
                        (&sink.var_name, &sink.method_name)
                    {
                        // Check 1: String literal assignments
                        let dep_graph = method_cache
                            .entry(Some(method_name.clone()))
                            .or_insert_with(|| {
                                let graph = VariableDependencyGraph::new()
                                    .with_propagators(propagators.to_vec());
                                if let Some(method_body) =
                                    self.extract_method_body(source_text, method_name)
                                {
                                    let mut graph = graph;
                                    graph.build_from_method(&method_body);
                                    graph
                                } else {
                                    graph
                                }
                            });

                        if dep_graph.is_assigned_string_literal(sink_var)
                            || dep_graph.has_string_literal_in_dependency_chain(sink_var)
                        {
                            eprintln!("[DEBUG] FLOW FOUND (string literal): source {} -> sink {} ({} is assigned string literal)",
                                      i, j, sink_var);
                            flows.push((source.clone(), sink.clone()));
                            continue;
                        }

                        // Check 2: Method parameter with taint annotation
                        // If sink variable is a method parameter with @RequestParam, @PathVariable, etc., it's tainted
                        if self.is_tainted_method_parameter(source_text, method_name, sink_var) {
                            eprintln!("[DEBUG] FLOW FOUND (tainted parameter): source {} -> sink {} ({} is a tainted method parameter)",
                                      i, j, sink_var);
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

    /// Check if a variable is flowing to a sink, using symbolic propagation for alias tracking
    pub(super) fn is_variable_flowing_to_sink(
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
        if taint_assume_safe_booleans
            && self.is_variable_in_safe_boolean_context(var_name, &sink_text)
        {
            eprintln!(
                "[DEBUG] Variable '{}' in safe boolean context, not flowing",
                var_name
            );
            return false;
        }

        // When taint_assume_safe_numbers is true, check if the variable is used in safe numeric contexts
        if taint_assume_safe_numbers
            && self.is_variable_in_safe_number_context(var_name, &sink_text)
        {
            eprintln!(
                "[DEBUG] Variable '{}' in safe number context, not flowing",
                var_name
            );
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
                eprintln!(
                    "[DEBUG] Variable '{}' not flowing through direct assignment chain",
                    var_name
                );
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
    pub(super) fn is_variable_in_safe_boolean_context(
        &self,
        var_name: &str,
        sink_text: &str,
    ) -> bool {
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
    pub(super) fn is_variable_in_safe_number_context(
        &self,
        var_name: &str,
        sink_text: &str,
    ) -> bool {
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
    pub(super) fn is_direct_assignment_chain(&self, var_name: &str, sink_text: &str) -> bool {
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

    /// Extract field name from sink text if it's a field access pattern
    /// e.g., "sink(this.y)" -> Some("y"), "sink(obj.x)" -> Some("x")
    pub(super) fn extract_field_from_sink(&self, sink_text: &str) -> Option<String> {
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
    pub(super) fn is_numeric_field(&self, field_name: &str, source_text: &str) -> bool {
        // Look for field declarations like: "int y;", "Integer x;", "long count;", etc.
        let numeric_types = [
            "int", "long", "short", "byte", "float", "double", "Integer", "Long", "Short", "Byte",
            "Float", "Double",
        ];

        for line in source_text.lines() {
            let line = line.trim();
            // Check for field declaration pattern: "Type fieldName;" or "Type fieldName = ..."
            for type_name in &numeric_types {
                let patterns = [
                    format!("{} {};", type_name, field_name),
                    format!("{} {} =", type_name, field_name),
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

    /// Check if a dataflow matches source and sink
    pub(super) fn is_flow_matching(
        &self,
        flow: &astgrep_dataflow::TaintFlow,
        source: &TaintMatch,
        sink: &TaintMatch,
    ) -> bool {
        // Check if flow source matches our source
        let source_matches = if let (Some(src_loc), Some(flow_loc)) =
            (source.node.location(), &flow.source.location)
        {
            src_loc.0 == flow_loc.start_line && src_loc.1 == flow_loc.start_column
        } else {
            // Fallback: compare by description text
            if let Some(source_text) = source.node.text() {
                flow.source.description.contains(&source_text)
                    || source_text.contains(&flow.source.description)
            } else {
                false
            }
        };

        // Check if flow sink matches our sink
        let sink_matches =
            if let (Some(sink_loc), Some(flow_loc)) = (sink.node.location(), &flow.sink.location) {
                sink_loc.0 == flow_loc.start_line && sink_loc.1 == flow_loc.start_column
            } else {
                // Fallback: compare by description text
                if let Some(sink_text) = sink.node.text() {
                    flow.sink.description.contains(&sink_text)
                        || sink_text.contains(&flow.sink.description)
                } else {
                    false
                }
            };

        source_matches && sink_matches
    }

    /// Extract arguments from the last function call in a chain
    /// e.g., "sink(e.getX())" -> Some("e.getX()")
    /// e.g., "Runtime.getRuntime().exec(nodeSucc)" -> Some("nodeSucc")
    pub(super) fn extract_last_call_args(text: &str) -> Option<&str> {
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

    /// Simplify a fully qualified class name pattern to just class.method pattern
    /// For example: "javax.xml.parsers.DocumentBuilderFactory.newInstance()" -> "DocumentBuilderFactory.newInstance()"
    pub(super) fn simplify_fully_qualified_pattern(pattern: &str) -> Option<String> {
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
            eprintln!(
                "[DEBUG] Simplified FQN pattern: '{}' -> '{}'",
                pattern, simplified
            );
            return Some(simplified);
        }

        None
    }

    /// Find variable name for a string literal in source
    pub(super) fn find_variable_for_string_literal(
        &self,
        source_text: &str,
        line_num: usize,
        literal: &str,
    ) -> Option<String> {
        let lines: Vec<&str> = source_text.lines().collect();
        if line_num == 0 || line_num > lines.len() {
            return None;
        }

        let line = lines[line_num - 1];
        if let Some(eq_pos) = line.find('=') {
            let before_eq = line[..eq_pos].trim();
            let after_eq = line[eq_pos + 1..].trim().trim_end_matches(';').trim();

            if after_eq.contains(literal) {
                let parts: Vec<&str> = before_eq.split_whitespace().collect();
                if let Some(var_name) = parts.last() {
                    let var_name = var_name.trim().to_string();
                    if !var_name.is_empty() {
                        return Some(var_name);
                    }
                }
            }
        }
        None
    }

    /// Check if a method parameter is tainted (has taint-related annotations)
    pub(super) fn is_tainted_method_parameter(
        &self,
        source_text: &str,
        method_name: &str,
        var_name: &str,
    ) -> bool {
        let lines: Vec<&str> = source_text.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            if line.contains(method_name) && line.contains('(') && line.contains(')') {
                if let Some(paren_start) = line.find('(') {
                    if let Some(paren_end) = line.rfind(')') {
                        let params_section = &line[paren_start..=paren_end];
                        let params: Vec<&str> = params_section[1..params_section.len() - 1]
                            .split(',')
                            .collect();
                        for param in params {
                            let param = param.trim();
                            let param_words: Vec<&str> = param.split_whitespace().collect();
                            if let Some(last_word) = param_words.last() {
                                if *last_word == var_name {
                                    let start_check = if i > 3 { i - 3 } else { 0 };
                                    for j in start_check..=i {
                                        let check_line = lines[j];
                                        if check_line.contains("@RequestParam")
                                            || check_line.contains("@PathVariable")
                                            || check_line.contains("@RequestBody")
                                            || check_line.contains("@RequestHeader")
                                            || check_line.contains("@CookieValue")
                                        {
                                            if j == i
                                                || (j < i
                                                    && lines[j + 1..=i]
                                                        .join(" ")
                                                        .contains(var_name))
                                            {
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

    /// Extract receiver from sink node
    pub(super) fn extract_receiver_from_sink(&self, sink_node: &dyn AstNode) -> Option<String> {
        let sink_text = sink_node.text().unwrap_or_default();

        if let Some(dot_pos) = sink_text.find('.') {
            let receiver = sink_text[..dot_pos].trim();
            if !receiver.is_empty() {
                return Some(receiver.to_string());
            }
        }

        if sink_node.node_type() == "method_invocation"
            || sink_node.node_type() == "call_expression"
        {
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

    fn extract_sink_argument(&self, sink_node: &dyn AstNode) -> Option<String> {
        let sink_text = sink_node.text().unwrap_or_default().trim();
        if let Some(open_paren) = sink_text.find('(') {
            let args = sink_text[open_paren + 1..].trim_end_matches(')');
            let args = args.trim();
            if !args.is_empty() {
                return Some(args.to_string());
            }
        }
        if sink_node.node_type() == "call_expression" {
            for i in (0..sink_node.child_count()).rev() {
                if let Some(child) = sink_node.child(i) {
                    if child.node_type() == "identifier" || child.node_type() == "argument_list" {
                        if let Some(text) = child.text() {
                            let trimmed = text.trim().trim_end_matches(')');
                            if !trimmed.is_empty() && !trimmed.starts_with('(') {
                                return Some(trimmed.to_string());
                            }
                        }
                    }
                }
            }
        }
        None
    }

    fn is_sink_variable_sanitized(
        &self,
        sink_var: &str,
        source_text: &str,
        dataflow_spec: &crate::types::DataFlowSpec,
    ) -> bool {
        for sanitizer_pattern in &dataflow_spec.sanitizers {
            let sanitizer_func = sanitizer_pattern
                .trim_end_matches("(...)")
                .split("::")
                .last()
                .unwrap_or(sanitizer_pattern);

            let pattern = format!("{} = {}({})", sink_var, sanitizer_func, sink_var);
            if source_text.contains(&pattern) {
                return true;
            }
            let alt_pattern = format!("{} = {}()", sink_var, sanitizer_func);
            if source_text.contains(&alt_pattern) {
                return true;
            }
        }
        false
    }
}
