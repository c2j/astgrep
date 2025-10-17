//! Rule editor component for YAML rule editing and validation

use egui;
use cr_rules::RuleParser;

/// Rule editor component
pub struct RuleEditor {
    /// Raw YAML content
    content: String,

    /// Parsed rules using the real rule parser
    parsed_rules: Vec<cr_rules::Rule>,

    /// Validation errors
    validation_errors: Vec<String>,

    /// Current cursor position
    cursor_pos: usize,

    /// Rule parser instance
    rule_parser: RuleParser,
}

impl RuleEditor {
    pub fn new() -> Self {
        Self {
            content: String::new(),
            parsed_rules: Vec::new(),
            validation_errors: Vec::new(),
            cursor_pos: 0,
            rule_parser: RuleParser::new(),
        }
    }
    
    pub fn set_content(&mut self, content: &str) {
        self.content = content.to_string();
        self.parse_rule();
    }
    
    pub fn get_content(&self) -> &str {
        &self.content
    }
    
    /// Show structure view (form-based editing)
    pub fn show_structure_view(&mut self, ui: &mut egui::Ui) {
        ui.heading("Rule Structure");

        egui::ScrollArea::vertical()
            .id_source("rule_editor_structure_scroll")
            .show(ui, |ui| {
                if let Some(rule) = self.parsed_rules.first() {
                ui.group(|ui| {
                    ui.label("Basic Information");

                    ui.horizontal(|ui| {
                        ui.label("ID:");
                        ui.label(&rule.id);
                    });

                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        ui.label(&rule.name);
                    });

                    ui.horizontal(|ui| {
                        ui.label("Severity:");
                        ui.label(format!("{:?}", rule.severity));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Confidence:");
                        ui.label(format!("{:?}", rule.confidence));
                    });
                });

                ui.add_space(10.0);

                ui.group(|ui| {
                    ui.label("Description");
                    ui.label(&rule.description);
                });

                ui.add_space(10.0);

                ui.group(|ui| {
                    ui.label("Languages");
                    ui.horizontal_wrapped(|ui| {
                        for lang in &rule.languages {
                            ui.label(format!("{:?}", lang));
                        }
                    });
                });

                ui.add_space(10.0);

                ui.group(|ui| {
                    ui.label("Patterns");
                    for (i, pattern) in rule.patterns.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(format!("{}:", i + 1));
                            match &pattern.pattern_type {
                                cr_rules::PatternType::Simple(pattern_str) => {
                                    ui.label(pattern_str);
                                }
                                _ => {
                                    ui.label(format!("{:?}", pattern.pattern_type));
                                }
                            }
                        });
                    }
                });

                ui.add_space(10.0);

                if let Some(fix) = &rule.fix {
                    ui.group(|ui| {
                        ui.label("Fix Suggestion");
                        ui.label(fix);
                    });
                    ui.add_space(10.0);
                }

                if !rule.metadata.is_empty() {
                    ui.group(|ui| {
                        ui.label("Metadata");
                        for (key, value) in &rule.metadata {
                            ui.horizontal(|ui| {
                                ui.label(format!("{}:", key));
                                ui.label(value);
                            });
                        }
                    });
                }
            } else {
                ui.label("No valid rule parsed. Please check the YAML syntax in the Advanced tab.");
            }
        });
    }
    
    /// Show advanced view (raw YAML editing)
    pub fn show_advanced_view(&mut self, ui: &mut egui::Ui) {
        ui.heading("Rule YAML");
        
        ui.vertical(|ui| {
            // Validation errors
            if !self.validation_errors.is_empty() {
                ui.group(|ui| {
                    ui.colored_label(egui::Color32::RED, "Validation Errors:");
                    for error in &self.validation_errors {
                        ui.colored_label(egui::Color32::RED, format!("• {}", error));
                    }
                });
                ui.add_space(5.0);
            }
            
            // YAML editor
            egui::ScrollArea::vertical()
                .id_source("rule_editor_yaml_scroll")
                .show(ui, |ui| {
                    let response = ui.add(
                        egui::TextEdit::multiline(&mut self.content)
                            .font(egui::TextStyle::Monospace)
                            .code_editor()
                            .desired_rows(25)
                            .desired_width(f32::INFINITY)
                    );

                    if response.changed() {
                        self.parse_rule();
                    }
                });
            
            ui.horizontal(|ui| {
                if ui.button("Validate").clicked() {
                    self.parse_rule(); // Re-parse to validate
                }

                if ui.button("Format").clicked() {
                    self.format_yaml();
                }
                
                if ui.button("Load Example").clicked() {
                    self.load_example_rule();
                }
            });
        });
    }
    
    fn parse_rule(&mut self) {
        // Use the real RuleParser from cr-rules
        self.validation_errors.clear();
        self.parsed_rules.clear();

        if self.content.trim().is_empty() {
            return;
        }

        // Parse rules using the real rule parser
        match self.rule_parser.parse_yaml(&self.content) {
            Ok(rules) => {
                self.parsed_rules = rules;
                if self.parsed_rules.is_empty() {
                    self.validation_errors.push("No valid rules found in YAML".to_string());
                }
            }
            Err(e) => {
                self.validation_errors.push(format!("Rule parsing error: {}", e));
            }
        }
    }

    
    fn format_yaml(&mut self) {
        // Simple formatting - in real implementation, use proper YAML formatter
        // For now, just ensure consistent indentation
    }
    
    fn load_example_rule(&mut self) {
        let example = r#"rules:
  - id: example-rule
    name: "Example Rule"
    description: "An example rule for demonstration"
    message: "This is an example finding"
    severity: WARNING
    languages:
      - java
      - javascript
    patterns:
      - pattern: console.log($MSG)
      - pattern: System.out.println($MSG)
"#;
        self.set_content(example);
    }

}
