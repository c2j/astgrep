//! Status bar component

use egui;
use astgrep_core::Finding;

/// Status bar component
pub struct StatusBar {
    /// Last analysis time
    last_analysis_time: Option<std::time::Instant>,
    
    /// Analysis duration
    analysis_duration: Option<std::time::Duration>,
    
    /// Current status message
    status_message: String,
}

impl StatusBar {
    pub fn new() -> Self {
        Self {
            last_analysis_time: None,
            analysis_duration: None,
            status_message: "Ready".to_string(),
        }
    }
    
    pub fn show(&mut self, ui: &mut egui::Ui, findings: &[Finding]) {
        ui.horizontal(|ui| {
            // Status message
            ui.label(&self.status_message);
            
            ui.separator();
            
            // Analysis statistics
            if !findings.is_empty() {
                let error_count = findings.iter().filter(|f| matches!(f.severity, astgrep_core::Severity::Error)).count();
                let warning_count = findings.iter().filter(|f| matches!(f.severity, astgrep_core::Severity::Warning)).count();
                let info_count = findings.iter().filter(|f| matches!(f.severity, astgrep_core::Severity::Info)).count();
                
                ui.colored_label(egui::Color32::RED, format!("{} errors", error_count));
                ui.colored_label(egui::Color32::YELLOW, format!("{} warnings", warning_count));
                ui.colored_label(egui::Color32::BLUE, format!("{} info", info_count));
                
                ui.separator();
            }
            
            // Analysis timing
            if let Some(duration) = self.analysis_duration {
                ui.label(format!("Analysis completed in {:.2}s", duration.as_secs_f64()));
                ui.separator();
            }
            
            // Right-aligned items
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Current time
                let now = chrono::Local::now();
                ui.label(now.format("%H:%M:%S").to_string());
                
                ui.separator();
                
                // Memory usage (approximate)
                if let Ok(usage) = sys_info::mem_info() {
                    let used_mb = (usage.total - usage.free) / 1024;
                    ui.label(format!("Memory: {} MB", used_mb));
                }
            });
        });
    }
    
    /// Set the status message
    pub fn set_status(&mut self, message: &str) {
        self.status_message = message.to_string();
    }
    
    /// Mark analysis as started
    pub fn analysis_started(&mut self) {
        self.last_analysis_time = Some(std::time::Instant::now());
        self.analysis_duration = None;
        self.status_message = "Running analysis...".to_string();
    }
    
    /// Mark analysis as completed
    pub fn analysis_completed(&mut self, findings_count: usize) {
        if let Some(start_time) = self.last_analysis_time {
            self.analysis_duration = Some(start_time.elapsed());
        }
        
        self.status_message = if findings_count > 0 {
            format!("Analysis completed - {} findings", findings_count)
        } else {
            "Analysis completed - no issues found".to_string()
        };
    }
    
    /// Mark analysis as failed
    pub fn analysis_failed(&mut self, error: &str) {
        self.analysis_duration = None;
        self.status_message = format!("Analysis failed: {}", error);
    }
}
