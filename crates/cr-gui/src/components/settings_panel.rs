//! Settings panel component

use egui;
use crate::app::{AppSettings, Theme};

/// Settings panel component
pub struct SettingsPanel;

impl SettingsPanel {
    pub fn new() -> Self {
        Self
    }
    
    pub fn show(&mut self, ctx: &egui::Context, settings: &mut AppSettings, show: &mut bool) {
        let mut close_requested = false;

        egui::Window::new("Settings")
            .open(show)
            .resizable(true)
            .default_width(400.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .id_source("settings_panel_scroll")
                    .show(ui, |ui| {
                        self.show_general_settings(ui, settings);
                        ui.separator();
                        self.show_editor_settings(ui, settings);
                        ui.separator();
                        self.show_analysis_settings(ui, settings);
                        ui.separator();
                        self.show_appearance_settings(ui, settings);
                    });

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("Reset to Defaults").clicked() {
                        *settings = AppSettings::default();
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Close").clicked() {
                            close_requested = true;
                        }

                        if ui.button("Apply").clicked() {
                            // Settings are applied immediately, but this could trigger
                            // additional actions like saving to file
                        }
                    });
                });
            });

        if close_requested {
            *show = false;
        }
    }
    
    fn show_general_settings(&self, ui: &mut egui::Ui, settings: &mut AppSettings) {
        ui.heading("General");
        
        ui.horizontal(|ui| {
            ui.label("Default Language:");
            egui::ComboBox::from_id_source("default_language")
                .selected_text(format!("{:?}", settings.selected_language))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut settings.selected_language, cr_core::Language::Java, "Java");
                    ui.selectable_value(&mut settings.selected_language, cr_core::Language::JavaScript, "JavaScript");
                    ui.selectable_value(&mut settings.selected_language, cr_core::Language::Python, "Python");
                    ui.selectable_value(&mut settings.selected_language, cr_core::Language::C, "C");
                    ui.selectable_value(&mut settings.selected_language, cr_core::Language::CSharp, "C#");
                    ui.selectable_value(&mut settings.selected_language, cr_core::Language::Php, "PHP");
                });
        });
        
        ui.checkbox(&mut settings.auto_analyze, "Auto-analyze on code changes");
    }
    
    fn show_editor_settings(&self, ui: &mut egui::Ui, settings: &mut AppSettings) {
        ui.heading("Editor");
        
        ui.checkbox(&mut settings.show_line_numbers, "Show line numbers");
        
        ui.horizontal(|ui| {
            ui.label("Font Size:");
            ui.add(egui::Slider::new(&mut settings.font_size, 8.0..=24.0).suffix("px"));
        });
        
        // Additional editor settings could go here
        ui.label("Tab Size: 4 spaces (fixed)");
        ui.label("Word Wrap: Enabled (fixed)");
    }
    
    fn show_analysis_settings(&self, ui: &mut egui::Ui, _settings: &mut AppSettings) {
        ui.heading("Analysis");
        
        ui.label("Analysis Engine: CR-SemService (built-in)");
        ui.label("Timeout: 30 seconds (fixed)");
        ui.label("Max Memory: 1GB (fixed)");
        
        // Future analysis settings could include:
        // - Custom rule directories
        // - Analysis timeout
        // - Memory limits
        // - Parallel processing settings
    }
    
    fn show_appearance_settings(&self, ui: &mut egui::Ui, settings: &mut AppSettings) {
        ui.heading("Appearance");
        
        ui.horizontal(|ui| {
            ui.label("Theme:");
            ui.radio_value(&mut settings.theme, Theme::Dark, "Dark");
            ui.radio_value(&mut settings.theme, Theme::Light, "Light");
        });
        
        // Color scheme preview
        ui.group(|ui| {
            ui.label("Color Preview:");
            
            match settings.theme {
                Theme::Dark => {
                    ui.horizontal(|ui| {
                        ui.colored_label(egui::Color32::WHITE, "Text");
                        ui.colored_label(egui::Color32::GRAY, "Comments");
                        ui.colored_label(egui::Color32::LIGHT_BLUE, "Keywords");
                        ui.colored_label(egui::Color32::LIGHT_GREEN, "Strings");
                    });
                }
                Theme::Light => {
                    ui.horizontal(|ui| {
                        ui.colored_label(egui::Color32::BLACK, "Text");
                        ui.colored_label(egui::Color32::DARK_GRAY, "Comments");
                        ui.colored_label(egui::Color32::BLUE, "Keywords");
                        ui.colored_label(egui::Color32::DARK_GREEN, "Strings");
                    });
                }
            }
        });
    }
}
