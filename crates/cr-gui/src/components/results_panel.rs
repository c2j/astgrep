//! Results panel component for displaying analysis results

use egui;
use cr_core::Finding;

/// Results panel component
pub struct ResultsPanel {
    /// Selected finding index
    selected_finding: Option<usize>,
    
    /// Show detailed view
    show_details: bool,
    
    /// Filter settings
    filter_severity: Option<String>,
    filter_rule: Option<String>,
}

impl ResultsPanel {
    pub fn new() -> Self {
        Self {
            selected_finding: None,
            show_details: true,
            filter_severity: None,
            filter_rule: None,
        }
    }
    
    pub fn show(&mut self, ui: &mut egui::Ui, findings: &[Finding]) {
        ui.vertical(|ui| {
            // Header with controls
            ui.horizontal(|ui| {
                ui.heading(format!("Analysis Results ({})", findings.len()));
                
                ui.separator();
                
                // Filters
                ui.label("Filter:");
                
                egui::ComboBox::from_id_source("severity_filter")
                    .selected_text(self.filter_severity.as_deref().unwrap_or("All Severities"))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.filter_severity, None, "All Severities");
                        ui.selectable_value(&mut self.filter_severity, Some("ERROR".to_string()), "ERROR");
                        ui.selectable_value(&mut self.filter_severity, Some("WARNING".to_string()), "WARNING");
                        ui.selectable_value(&mut self.filter_severity, Some("INFO".to_string()), "INFO");
                    });
                
                ui.separator();
                
                // Statistics
                let error_count = findings.iter().filter(|f| matches!(f.severity, cr_core::Severity::Error)).count();
                let warning_count = findings.iter().filter(|f| matches!(f.severity, cr_core::Severity::Warning)).count();
                let info_count = findings.iter().filter(|f| matches!(f.severity, cr_core::Severity::Info)).count();
                
                ui.colored_label(egui::Color32::RED, format!("🔴 {} errors", error_count));
                ui.colored_label(egui::Color32::YELLOW, format!("🟡 {} warnings", warning_count));
                ui.colored_label(egui::Color32::BLUE, format!("🔵 {} info", info_count));
                
                ui.separator();
                
                ui.checkbox(&mut self.show_details, "Show details");
            });
            
            ui.separator();
            
            if findings.is_empty() {
                ui.centered_and_justified(|ui| {
                    ui.label("No analysis results. Run analysis to see findings here.");
                });
                return;
            }
            
            // Results list
            egui::ScrollArea::vertical()
                .id_source("results_panel_scroll")
                .show(ui, |ui| {
                    for (index, finding) in findings.iter().enumerate() {
                        // Apply filters
                        if let Some(ref severity_filter) = self.filter_severity {
                            let finding_severity = match finding.severity {
                                cr_core::Severity::Critical => "CRITICAL",
                                cr_core::Severity::Error => "ERROR",
                                cr_core::Severity::Warning => "WARNING",
                                cr_core::Severity::Info => "INFO",
                            };
                            if finding_severity != severity_filter {
                                continue;
                            }
                        }

                        self.show_finding(ui, finding, index);
                    }
                });
        });
    }
    
    fn show_finding(&mut self, ui: &mut egui::Ui, finding: &Finding, index: usize) {
        let is_selected = self.selected_finding == Some(index);
        
        // Severity color
        let severity_color = match finding.severity {
            cr_core::Severity::Critical => egui::Color32::DARK_RED,
            cr_core::Severity::Error => egui::Color32::RED,
            cr_core::Severity::Warning => egui::Color32::YELLOW,
            cr_core::Severity::Info => egui::Color32::BLUE,
        };

        // Severity icon
        let severity_icon = match finding.severity {
            cr_core::Severity::Critical => "🔴",
            cr_core::Severity::Error => "🟠",
            cr_core::Severity::Warning => "🟡",
            cr_core::Severity::Info => "🔵",
        };
        
        let response = ui.group(|ui| {
            ui.horizontal(|ui| {
                // Severity indicator
                ui.colored_label(severity_color, severity_icon);
                
                // Main content
                ui.vertical(|ui| {
                    // Title line
                    ui.horizontal(|ui| {
                        ui.strong(&finding.rule_id);
                        ui.separator();
                        ui.label(format!("{}:{}", finding.location.start_line, finding.location.start_column));
                        ui.separator();
                        ui.label(finding.location.file.to_string_lossy().as_ref());
                    });
                    
                    // Message
                    ui.label(&finding.message);
                    
                    // Details (if enabled and selected)
                    if self.show_details && is_selected {
                        ui.separator();
                        
                        ui.group(|ui| {
                            ui.label("Details:");
                            
                            ui.horizontal(|ui| {
                                ui.label("Location:");
                                ui.monospace(format!(
                                    "{}:{}-{}:{}",
                                    finding.location.start_line,
                                    finding.location.start_column,
                                    finding.location.end_line,
                                    finding.location.end_column
                                ));
                            });
                            
                            ui.horizontal(|ui| {
                                ui.label("Confidence:");
                                let confidence_text = match finding.confidence {
                                    cr_core::Confidence::High => "High",
                                    cr_core::Confidence::Medium => "Medium",
                                    cr_core::Confidence::Low => "Low",
                                };
                                ui.label(confidence_text);
                            });
                            
                            if let Some(ref fix) = finding.fix_suggestion {
                                if !fix.is_empty() {
                                    ui.separator();
                                    ui.label("Suggested fix:");
                                    ui.monospace(fix);
                                }
                            }
                            
                            if !finding.metadata.is_empty() {
                                ui.separator();
                                ui.label("Metadata:");
                                for (key, value) in &finding.metadata {
                                    ui.horizontal(|ui| {
                                        ui.label(format!("{}:", key));
                                        ui.label(value);
                                    });
                                }
                            }
                        });
                    }
                });
                
                // Action buttons
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("📋").on_hover_text("Copy to clipboard").clicked() {
                        self.copy_finding_to_clipboard(finding);
                    }
                    
                    if ui.button("🔍").on_hover_text("Go to location").clicked() {
                        // This would jump to the location in the code editor
                        // Implementation depends on integration with code editor
                    }
                });
            });
        });
        
        // Handle selection
        if response.response.clicked() {
            self.selected_finding = if is_selected { None } else { Some(index) };
        }
        
        ui.add_space(2.0);
    }
    
    fn copy_finding_to_clipboard(&self, finding: &Finding) {
        let text = format!(
            "Rule: {}\nMessage: {}\nLocation: {}:{}:{}\nSeverity: {:?}\nFile: {}",
            finding.rule_id,
            finding.message,
            finding.location.file.to_string_lossy(),
            finding.location.start_line,
            finding.location.start_column,
            finding.severity,
            finding.location.file.to_string_lossy()
        );
        
        // Copy to clipboard (this would need a clipboard crate)
        // For now, just print to console
        println!("Copied to clipboard: {}", text);
    }
    
    /// Get the currently selected finding
    pub fn get_selected_finding(&self) -> Option<usize> {
        self.selected_finding
    }
    
    /// Set the selected finding
    pub fn set_selected_finding(&mut self, index: Option<usize>) {
        self.selected_finding = index;
    }
    
    /// Clear all filters
    pub fn clear_filters(&mut self) {
        self.filter_severity = None;
        self.filter_rule = None;
    }
}
