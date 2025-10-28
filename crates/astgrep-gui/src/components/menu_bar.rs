//! Menu bar component

use egui;
use crate::app::{AppSettings, UiState};

/// Menu bar component
pub struct MenuBar {
    /// Show about dialog
    show_about: bool,
}

impl MenuBar {
    pub fn new() -> Self {
        Self {
            show_about: false,
        }
    }
    
    pub fn show(&mut self, ctx: &egui::Context, settings: &mut AppSettings, ui_state: &mut UiState) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                // File menu
                ui.menu_button("File", |ui| {
                    if ui.button("📁 Open Rule...").clicked() {
                        ui_state.request_open_rule = true;
                        ui.close_menu();
                    }

                    if ui.button("💾 Save Rule...").clicked() {
                        ui_state.request_save_rule = true;
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui.button("📂 Open Code...").clicked() {
                        ui_state.request_open_code = true;
                        ui.close_menu();
                    }

                    if ui.button("💾 Save Code...").clicked() {
                        ui_state.request_save_code = true;
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui.button("📤 Export Results...").clicked() {
                        ui_state.request_export_results = true;
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui.button("🚪 Exit").clicked() {
                        std::process::exit(0);
                    }
                });
                
                // Edit menu
                ui.menu_button("Edit", |ui| {
                    if ui.button("↶ Undo").clicked() {
                        ui_state.request_undo = true;
                        ui.close_menu();
                    }

                    if ui.button("↷ Redo").clicked() {
                        ui_state.request_redo = true;
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui.button("✂️ Cut").clicked() {
                        ui_state.request_cut = true;
                        ui.close_menu();
                    }

                    if ui.button("📋 Copy").clicked() {
                        ui_state.request_copy = true;
                        ui.close_menu();
                    }

                    if ui.button("📄 Paste").clicked() {
                        ui_state.request_paste = true;
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui.button("🔍 Find...").clicked() {
                        ui_state.request_find = true;
                        ui.close_menu();
                    }

                    if ui.button("🔄 Replace...").clicked() {
                        ui_state.request_replace = true;
                        ui.close_menu();
                    }
                });
                
                // Analysis menu
                ui.menu_button("Analysis", |ui| {
                    if ui.button("▶️ Run Analysis").clicked() {
                        ui_state.request_run_analysis = true;
                        ui.close_menu();
                    }

                    if ui.button("⏹️ Stop Analysis").clicked() {
                        ui_state.request_stop_analysis = true;
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui.button("✅ Validate Rule").clicked() {
                        ui_state.request_validate_rule = true;
                        ui.close_menu();
                    }

                    if ui.button("🔧 Format Rule").clicked() {
                        ui_state.request_format_rule = true;
                        ui.close_menu();
                    }

                    ui.separator();

                    ui.checkbox(&mut settings.auto_analyze, "Auto-analyze");
                });
                
                // View menu
                ui.menu_button("View", |ui| {
                    ui.checkbox(&mut settings.show_line_numbers, "Show line numbers");
                    
                    ui.separator();
                    
                    ui.menu_button("Theme", |ui| {
                        ui.radio_value(&mut settings.theme, crate::app::Theme::Dark, "Dark");
                        ui.radio_value(&mut settings.theme, crate::app::Theme::Light, "Light");
                    });
                    
                    ui.separator();
                    
                    ui.menu_button("Font Size", |ui| {
                        ui.radio_value(&mut settings.font_size, 12.0, "Small (12px)");
                        ui.radio_value(&mut settings.font_size, 14.0, "Medium (14px)");
                        ui.radio_value(&mut settings.font_size, 16.0, "Large (16px)");
                        ui.radio_value(&mut settings.font_size, 18.0, "Extra Large (18px)");
                    });
                    
                    ui.separator();
                    
                    if ui.button("⚙️ Settings...").clicked() {
                        ui_state.show_settings = true;
                        ui.close_menu();
                    }
                });
                
                // Help menu
                ui.menu_button("Help", |ui| {
                    if ui.button("📖 Documentation").clicked() {
                        self.open_documentation();
                        ui.close_menu();
                    }

                    if ui.button("🎯 Examples").clicked() {
                        ui_state.request_load_examples = true;
                        ui.close_menu();
                    }

                    if ui.button("🐛 Report Issue").clicked() {
                        self.report_issue();
                        ui.close_menu();
                    }
                    
                    ui.separator();
                    
                    if ui.button("ℹ️ About").clicked() {
                        self.show_about = true;
                        ui.close_menu();
                    }
                });
                

            });
        });
        
        // Show about dialog
        if self.show_about {
            self.show_about_dialog(ctx);
        }
    }
    
    fn show_about_dialog(&mut self, ctx: &egui::Context) {
        egui::Window::new("About astgrep Playground")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("astgrep Playground");
                    ui.add_space(10.0);
                    
                    ui.label("A graphical user interface for the astgrep");
                    ui.label("static code analysis tool.");
                    ui.add_space(10.0);
                    
                    ui.label("Version: 0.1.0");
                    ui.label("Built with Rust and egui");
                    ui.add_space(10.0);
                    
                    ui.hyperlink_to("GitHub Repository", "https://github.com/c2j/astgrep");
                    ui.add_space(10.0);
                    
                    if ui.button("Close").clicked() {
                        self.show_about = false;
                    }
                });
            });
    }
    
    fn open_rule_file(&self) {
        // TODO: Implement file opening
        println!("Opening rule file...");
    }
    
    fn save_rule_file(&self) {
        // TODO: Implement file saving
        println!("Saving rule file...");
    }
    
    fn open_code_file(&self) {
        // TODO: Implement code file opening
        println!("Opening code file...");
    }
    
    fn save_code_file(&self) {
        // TODO: Implement code file saving
        println!("Saving code file...");
    }
    
    fn export_results(&self) {
        // TODO: Implement results export
        println!("Exporting results...");
    }
    
    fn run_analysis(&self) {
        // TODO: Trigger analysis
        println!("Running analysis...");
    }
    
    fn stop_analysis(&self) {
        // TODO: Stop analysis
        println!("Stopping analysis...");
    }
    
    fn validate_rule(&self) {
        // TODO: Validate current rule
        println!("Validating rule...");
    }
    
    fn format_rule(&self) {
        // TODO: Format current rule
        println!("Formatting rule...");
    }
    
    fn open_documentation(&self) {
        // TODO: Open documentation
        if let Err(e) = webbrowser::open("https://github.com/c2j/astgrep/blob/main/README.md") {
            eprintln!("Failed to open documentation: {}", e);
        }
    }
    
    fn load_examples(&self) {
        // TODO: Load example rules and code
        println!("Loading examples...");
    }
    
    fn report_issue(&self) {
        // TODO: Open issue reporting
        if let Err(e) = webbrowser::open("https://github.com/c2j/astgrep/issues/new") {
            eprintln!("Failed to open issue reporting: {}", e);
        }
    }
}
