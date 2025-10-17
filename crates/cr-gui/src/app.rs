//! Main application structure and state management

use egui;

use crate::components::{
    RuleEditor, CodeEditor, ResultsPanel, MenuBar, StatusBar, SettingsPanel
};
use cr_core::{Language, Finding};
use cr_rules::{RuleEngine, RuleParser, RuleContext};
use cr_parser::LanguageParserRegistry;



/// Main application state
pub struct CrGuiApp {
    /// Rule editor component
    rule_editor: RuleEditor,
    
    /// Code editor component  
    code_editor: CodeEditor,
    
    /// Results panel component
    results_panel: ResultsPanel,
    
    /// Menu bar component
    menu_bar: MenuBar,
    
    /// Status bar component
    status_bar: StatusBar,
    
    /// Settings panel component
    settings_panel: SettingsPanel,
    
    /// Rule engine for analysis
    rule_engine: RuleEngine,

    /// Parser registry for different languages
    parser_registry: LanguageParserRegistry,

    /// Current analysis results
    analysis_results: Vec<Finding>,
    
    /// Application settings
    settings: AppSettings,
    
    /// UI state
    ui_state: UiState,
}

pub struct AppSettings {
    pub selected_language: Language,
    pub auto_analyze: bool,
    pub show_line_numbers: bool,
    pub theme: Theme,
    pub font_size: f32,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            selected_language: Language::Java,
            auto_analyze: true,
            show_line_numbers: true,
            theme: Theme::Dark,
            font_size: 14.0,
        }
    }
}

#[derive(Default)]
pub struct UiState {
    pub show_settings: bool,
    pub left_panel_width: f32,
    pub right_panel_width: f32,
    pub bottom_panel_height: f32,
    pub selected_tab_left: LeftTab,
    pub selected_tab_right: RightTab,
}

#[derive(Default, PartialEq)]
pub enum LeftTab {
    #[default]
    Structure,
    Advanced,
}

#[derive(Default, PartialEq)]
pub enum RightTab {
    #[default]
    TestCode,
    LiveCode,
    Metadata,
    Docs,
}

#[derive(Default, PartialEq)]
pub enum Theme {
    #[default]
    Dark,
    Light,
}

impl CrGuiApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self {
            rule_editor: RuleEditor::new(),
            code_editor: CodeEditor::new(),
            results_panel: ResultsPanel::new(),
            menu_bar: MenuBar::new(),
            status_bar: StatusBar::new(),
            settings_panel: SettingsPanel::new(),
            rule_engine: RuleEngine::new(),
            parser_registry: LanguageParserRegistry::new(),
            analysis_results: Vec::new(),
            settings: AppSettings {
                selected_language: Language::Java,
                auto_analyze: true,
                show_line_numbers: true,
                theme: Theme::Dark,
                font_size: 14.0,
            },
            ui_state: UiState {
                left_panel_width: 400.0,
                right_panel_width: 500.0,
                bottom_panel_height: 300.0, // Increased height for better visibility
                ..Default::default()
            },
        };
        
        // Load default rule
        app.load_default_rule();
        
        app
    }
    
    fn load_default_rule(&mut self) {
        let default_rule = r#"# Java Code Standards - System.out Logging Rule
rules:
  - id: cs-eh-08-system-out-logging
    name: "Avoid System.out for Logging"
    description: "Detects usage of System.out and System.err for logging"
    message: "System.out/System.err should not be used for logging. Use a proper logging framework instead."
    severity: WARNING
    languages:
      - java
    pattern-either:
      - pattern: System.out.println($MESSAGE)
      - pattern: System.out.print($MESSAGE)
      - pattern: System.err.println($MESSAGE)
      - pattern: System.err.print($MESSAGE)
"#;
        
        self.rule_editor.set_content(default_rule);
        
        let default_code = r#"public class TestClass {
    public void badLoggingExamples() {
        // These should be detected by the rule
        System.out.println("Debug: Processing user data");
        System.out.print("Status: ");
        System.err.println("Error occurred: " + getMessage());
        System.err.print("Warning: ");
        //System.err.print("Warning: ");

        String userId = "12345";
        System.out.println("User ID: " + userId);
    }

    public void goodLoggingExamples() {
        // These are proper logging practices
        Logger logger = LoggerFactory.getLogger(TestClass.class);

        logger.info("Processing user data");
        logger.debug("Debug information");
        logger.error("Error occurred", exception);
    }

    private String getMessage() {
        return "test message";
    }
}"#;
        
        self.code_editor.set_content(default_code);
    }
}

impl eframe::App for CrGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle menu bar
        self.menu_bar.show(ctx, &mut self.settings, &mut self.ui_state);
        
        // Main content area
        egui::CentralPanel::default().show(ctx, |ui| {
            self.show_main_content(ui);
        });
        
        // Status bar
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            self.status_bar.show(ui, &self.analysis_results);
        });
        
        // Settings panel (modal)
        if self.ui_state.show_settings {
            self.settings_panel.show(ctx, &mut self.settings, &mut self.ui_state.show_settings);
        }
        
        // Auto-analyze is disabled for now to avoid conflicts with manual analysis
        // if self.settings.auto_analyze {
        //     self.run_analysis();
        // }
    }
}

impl CrGuiApp {
    fn show_main_content(&mut self, ui: &mut egui::Ui) {
        // Horizontal split: left panel | right panel
        let available_rect = ui.available_rect_before_wrap();
        
        ui.allocate_ui_at_rect(available_rect, |ui| {
            ui.horizontal(|ui| {
                // Left panel (rule editor)
                ui.allocate_ui_with_layout(
                    egui::vec2(self.ui_state.left_panel_width, available_rect.height()),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        self.show_left_panel(ui);
                    },
                );
                
                // Vertical separator
                ui.separator();
                
                // Right panel (code editor and results)
                let remaining_width = available_rect.width() - self.ui_state.left_panel_width - 10.0;
                ui.allocate_ui_with_layout(
                    egui::vec2(remaining_width, available_rect.height()),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        self.show_right_panel(ui);
                    },
                );
            });
        });
    }
    
    fn show_left_panel(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            // Tab bar
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.ui_state.selected_tab_left, LeftTab::Structure, "structure");
                ui.selectable_value(&mut self.ui_state.selected_tab_left, LeftTab::Advanced, "advanced");
            });
            
            ui.separator();
            
            // Tab content
            match self.ui_state.selected_tab_left {
                LeftTab::Structure => {
                    self.rule_editor.show_structure_view(ui);
                }
                LeftTab::Advanced => {
                    self.rule_editor.show_advanced_view(ui);
                }
            }
        });
    }
    
    fn show_right_panel(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            // Tab bar
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.ui_state.selected_tab_right, RightTab::TestCode, "test code");
                ui.selectable_value(&mut self.ui_state.selected_tab_right, RightTab::LiveCode, "live code");
                ui.selectable_value(&mut self.ui_state.selected_tab_right, RightTab::Metadata, "metadata");
                ui.selectable_value(&mut self.ui_state.selected_tab_right, RightTab::Docs, "docs");
            });
            
            ui.separator();
            
            // Tab content
            let available_height = ui.available_height() - self.ui_state.bottom_panel_height - 20.0;
            
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), available_height),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    match self.ui_state.selected_tab_right {
                        RightTab::TestCode => {
                            if self.code_editor.show(ui, &self.settings) {
                                // Analyze button was clicked
                                println!("🔍 Analyze button clicked!");
                                self.run_analysis();
                            }
                        }
                        RightTab::LiveCode => {
                            ui.label("Live code editing (coming soon)");
                        }
                        RightTab::Metadata => {
                            ui.label("Rule metadata view (coming soon)");
                        }
                        RightTab::Docs => {
                            ui.label("Documentation view (coming soon)");
                        }
                    }
                },
            );
            
            ui.separator();
            
            // Bottom panel (results)
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), self.ui_state.bottom_panel_height),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    self.results_panel.show(ui, &self.analysis_results);
                },
            );
        });
    }
    
    fn run_analysis(&mut self) {
        println!("🔍 Running real analysis...");

        // Get current code from code editor
        let source_code = self.code_editor.get_content().to_string();
        let language = self.code_editor.get_language().to_string();

        // Get current rule from rule editor
        let rule_content = self.rule_editor.get_content().to_string();



        if source_code.trim().is_empty() {
            println!("⚠️ No source code to analyze");
            self.analysis_results.clear();
            self.update_code_highlights();
            return;
        }

        if rule_content.trim().is_empty() {
            println!("⚠️ No rules defined");
            self.analysis_results.clear();
            self.update_code_highlights();
            return;
        }

        // Parse language
        let lang = match language.as_str() {
            "java" => cr_core::Language::Java,
            "javascript" => cr_core::Language::JavaScript,
            "python" => cr_core::Language::Python,
            "php" => cr_core::Language::Php,
            "sql" => cr_core::Language::Sql,
            "bash" => cr_core::Language::Bash,
            "c" => cr_core::Language::C,
            "csharp" => cr_core::Language::CSharp,
            _ => {
                println!("⚠️ Unsupported language: {}", language);
                self.analysis_results.clear();
                self.update_code_highlights();
                return;
            }
        };

        // Run analysis with real rule engine
        match self.analyze_code_with_rules(&source_code, &rule_content, lang) {
            Ok(findings) => {
                println!("📊 Analysis completed with {} findings", findings.len());
                self.analysis_results = findings;
            }
            Err(e) => {
                println!("❌ Analysis failed: {}", e);
                self.analysis_results.clear();
            }
        }

        println!("📊 Analysis completed with {} findings", self.analysis_results.len());

        // Update code editor highlights based on analysis results
        self.update_code_highlights();
    }

    /// Analyze code with rules using the real rule engine
    fn analyze_code_with_rules(
        &mut self,
        source_code: &str,
        rule_content: &str,
        language: cr_core::Language,
    ) -> anyhow::Result<Vec<cr_core::Finding>> {
        use std::path::PathBuf;

        // Parse rules using the real RuleParser
        let rule_parser = RuleParser::new();
        let parsed_rules = rule_parser.parse_yaml(rule_content)
            .map_err(|e| anyhow::anyhow!("Failed to parse rules: {}", e))?;

        if parsed_rules.is_empty() {
            println!("⚠️ No applicable rules found for language {:?}", language);
            return Ok(Vec::new());
        }

        println!("📋 Loaded {} rules for {:?}", parsed_rules.len(), language);



        // Clear existing rules and add new ones
        self.rule_engine = RuleEngine::new();
        for rule in parsed_rules {
            if let Err(e) = self.rule_engine.add_rule(rule) {
                println!("⚠️ Failed to add rule: {}", e);
            }
        }

        // Parse the source code using the appropriate parser
        // Create a proper file path with the correct extension for the language
        let file_extension = match language {
            cr_core::Language::Java => "java",
            cr_core::Language::JavaScript => "js",
            cr_core::Language::Python => "py",
            cr_core::Language::Php => "php",
            cr_core::Language::Sql => "sql",
            cr_core::Language::Bash => "sh",
            cr_core::Language::C => "c",
            cr_core::Language::CSharp => "cs",
        };
        let file_path = PathBuf::from(format!("test_file.{}", file_extension));
        let ast = self.parser_registry.parse_file(&file_path, source_code)
            .map_err(|e| anyhow::anyhow!("Failed to parse source code: {}", e))?;



        // Create rule context
        let context = RuleContext::new(
            file_path.to_string_lossy().to_string(),
            language,
            source_code.to_string(),
        );

        // Execute rules using the real rule engine
        let rule_results = self.rule_engine.execute_rules(&*ast, &context)
            .map_err(|e| anyhow::anyhow!("Failed to execute rules: {}", e))?;

        // Convert rule results to findings
        let mut findings = Vec::new();
        for result in rule_results {
            findings.extend(result.findings);
        }

        Ok(findings)
    }



    fn update_code_highlights(&mut self) {
        use crate::components::code_editor::HighlightRange;

        // Clear existing highlights
        self.code_editor.clear_highlights();

        // Add highlights for each finding
        for finding in &self.analysis_results {
            let color = match finding.severity {
                cr_core::Severity::Error => egui::Color32::from_rgb(255, 100, 100),    // Red
                cr_core::Severity::Warning => egui::Color32::from_rgb(255, 200, 100),  // Orange
                cr_core::Severity::Info => egui::Color32::from_rgb(100, 150, 255),     // Blue
                cr_core::Severity::Critical => egui::Color32::from_rgb(200, 50, 50),   // Dark Red
            };

            let highlight = HighlightRange {
                start_line: finding.location.start_line.saturating_sub(1), // Convert to 0-based
                start_col: finding.location.start_column.saturating_sub(1),
                end_line: finding.location.end_line.saturating_sub(1),
                end_col: finding.location.end_column.saturating_sub(1),
                color,
                message: format!("{}: {}", finding.rule_id, finding.message),
            };

            self.code_editor.add_highlight(highlight);
        }
    }
}
