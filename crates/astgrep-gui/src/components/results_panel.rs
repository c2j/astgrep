//! Results panel component for displaying analysis results

use egui;
use astgrep_core::Finding;

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
            // Header - playground 风格
            ui.strong("Matches");
            ui.separator();

            if findings.is_empty() {
                // 空状态提示 - playground 风格
                ui.group(|ui| {
                    ui.set_width(ui.available_width());
                    ui.colored_label(egui::Color32::GRAY, "No matches found.");
                    ui.add_space(5.0);
                    ui.label("Run analysis to see findings here.");
                });
                return;
            }

            // Results list - playground 风格
            egui::ScrollArea::vertical()
                .id_source("results_panel_scroll")
                .max_height(ui.available_height() - 40.0)
                .show(ui, |ui| {
                    for finding in findings.iter() {
                        self.show_finding_playground_style(ui, finding);
                    }
                });

            ui.separator();

            // 底部统计信息 - playground 风格
            ui.horizontal(|ui| {
                ui.label(format!("✓ {} match{}", findings.len(), if findings.len() == 1 { "" } else { "es" }));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label("astgrep v1.0.0");
                });
            });
        });
    }

    fn show_finding_playground_style(&mut self, ui: &mut egui::Ui, finding: &Finding) {
        // 根据严重程度选择背景色 - 与 playground 一致
        let (bg_color, border_color, text_color) = match finding.severity {
            astgrep_core::Severity::Critical | astgrep_core::Severity::Error => (
                egui::Color32::from_rgb(255, 245, 245),  // 浅红色背景
                egui::Color32::from_rgb(254, 178, 178),  // 红色边框
                egui::Color32::from_rgb(204, 0, 0),      // 深红色文字
            ),
            astgrep_core::Severity::Warning => (
                egui::Color32::from_rgb(255, 251, 240),  // 浅黄色背景
                egui::Color32::from_rgb(251, 211, 141),  // 黄色边框
                egui::Color32::from_rgb(153, 102, 0),    // 深黄色文字
            ),
            astgrep_core::Severity::Info => (
                egui::Color32::from_rgb(240, 247, 255),  // 浅蓝色背景
                egui::Color32::from_rgb(179, 217, 255),  // 蓝色边框
                egui::Color32::from_rgb(0, 102, 204),    // 深蓝色文字
            ),
        };

        // 使用 Frame 来创建带背景色和边框的卡片
        let frame = egui::Frame::none()
            .fill(bg_color)
            .stroke(egui::Stroke::new(1.0, border_color))
            .inner_margin(egui::Margin::same(12.0))
            .rounding(egui::Rounding::same(4.0));

        frame.show(ui, |ui| {
            ui.set_width(ui.available_width());

            // 行号 - 加粗显示
            ui.colored_label(text_color, format!("Line {}", finding.location.start_line));

            ui.add_space(4.0);

            // 消息
            ui.label(&finding.message);

            // 显示规则 ID
            ui.add_space(2.0);
            ui.colored_label(egui::Color32::GRAY, format!("Rule: {}", finding.rule_id));
        });

        ui.add_space(8.0);
    }
    
    fn show_finding(&mut self, ui: &mut egui::Ui, finding: &Finding, index: usize) {
        let is_selected = self.selected_finding == Some(index);
        
        // Severity color
        let severity_color = match finding.severity {
            astgrep_core::Severity::Critical => egui::Color32::DARK_RED,
            astgrep_core::Severity::Error => egui::Color32::RED,
            astgrep_core::Severity::Warning => egui::Color32::YELLOW,
            astgrep_core::Severity::Info => egui::Color32::BLUE,
        };

        // Severity icon
        let severity_icon = match finding.severity {
            astgrep_core::Severity::Critical => "🔴",
            astgrep_core::Severity::Error => "🟠",
            astgrep_core::Severity::Warning => "🟡",
            astgrep_core::Severity::Info => "🔵",
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
                                    astgrep_core::Confidence::High => "High",
                                    astgrep_core::Confidence::Medium => "Medium",
                                    astgrep_core::Confidence::Low => "Low",
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
