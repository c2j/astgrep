//! Taint analysis implementation
//!
//! This module contains taint analysis methods for tracking data flow from sources to sinks

use super::*;

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
        let source_text = ast.text().unwrap_or_default();

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

        let taint_flows = self.detect_taint_flows(
            &source_matches,
            &sink_matches,
            ast,
            dataflow_analysis,
            assume_safe_booleans,
            assume_safe_numbers,
            only_propagate_through_assignments,
            &source_text,
            &dataflow_spec.propagators,
            &dataflow_spec,
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
                let mut var_name = None;

                // If focus-metavariables are specified, extract the binding for the first focus variable
                if !source_pattern.focus_metavariables.is_empty() {
                    let focus_var = &source_pattern.focus_metavariables[0];
                    // Remove the "$" prefix to match the binding key
                    let focus_key = focus_var.trim_start_matches('$');
                    if let Some(value) = m.bindings.get(focus_key) {
                        if !value.is_empty() {
                            var_name = Some(value.clone());
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
                            var_name = Some(value.clone());
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
                    if let Some(text) = m.node.text() {
                        let text = text.trim();
                        // Check if this is a string literal pattern match (starts and ends with ")
                        if text.starts_with('"') && text.ends_with('"') && text.len() > 2 {
                            // This is a string literal match, find the variable it's assigned to
                            if let Some((start_line, _, _, _)) = m.node.location() {
                                var_name = self.find_variable_for_string_literal(
                                    source_text,
                                    start_line,
                                    text,
                                );
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
                        var_name = Some(value.clone());
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

            sinks.push(TaintMatch {
                node: m.node,
                bindings: m.bindings,
                var_name,
                method_name,
            });
        }

        Ok(sinks)
    }

    /// Detect taint flows from sources to sinks
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
