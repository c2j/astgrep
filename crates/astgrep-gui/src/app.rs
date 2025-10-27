//! Main application structure and state management

use egui;

use crate::components::{
    RuleEditor, CodeEditor, ResultsPanel, MenuBar, StatusBar, SettingsPanel
};
use astgrep_core::{Language, Finding};
use astgrep_rules::{RuleEngine, RuleParser, RuleContext};
use astgrep_parser::LanguageParserRegistry;



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
    pub analysis_mode: AnalysisMode,
    pub inspect_rule_expanded: bool,
}

#[derive(Default, PartialEq)]
pub enum LeftTab {
    #[default]
    Simple,
    Advanced,
}

#[derive(Default, PartialEq)]
pub enum RightTab {
    #[default]
    TestCode,
    Metadata,
    Docs,
}

#[derive(Default, PartialEq, Clone, Copy, Debug)]
pub enum AnalysisMode {
    #[default]
    Normal,
    Pro,
    Turbo,
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
                left_panel_width: 500.0,  // 增加左侧面板宽度以适应规则编辑
                right_panel_width: 600.0,
                bottom_panel_height: 250.0,
                inspect_rule_expanded: true,  // 默认展开 Inspect Rule
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
            // Tab bar - 与 playground 风格一致
            ui.horizontal(|ui| {
                ui.style_mut().spacing.item_spacing.x = 0.0;

                let simple_btn = ui.selectable_label(
                    self.ui_state.selected_tab_left == LeftTab::Simple,
                    "simple"
                );
                if simple_btn.clicked() {
                    self.ui_state.selected_tab_left = LeftTab::Simple;
                }

                let advanced_btn = ui.selectable_label(
                    self.ui_state.selected_tab_left == LeftTab::Advanced,
                    "advanced"
                );
                if advanced_btn.clicked() {
                    self.ui_state.selected_tab_left = LeftTab::Advanced;
                }
            });

            ui.separator();

            // Tab content - 占据大部分空间
            let available_height = ui.available_height() - 150.0; // 为 Inspect Rule 预留空间

            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), available_height),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    match self.ui_state.selected_tab_left {
                        LeftTab::Simple => {
                            // Simple tab 显示 YAML 编辑器（简化版）
                            self.show_simple_rule_editor(ui);
                        }
                        LeftTab::Advanced => {
                            // Advanced tab 显示完整的 YAML 编辑器
                            self.rule_editor.show_advanced_view(ui);
                        }
                    }
                },
            );

            ui.separator();

            // Inspect Rule 区域
            self.show_inspect_rule(ui);
        });
    }

    fn show_simple_rule_editor(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.label("Rule YAML");
            ui.add_space(5.0);

            // 显示验证错误
            if !self.rule_editor.get_validation_errors().is_empty() {
                ui.group(|ui| {
                    ui.colored_label(egui::Color32::RED, "⚠ Validation Errors:");
                    for error in self.rule_editor.get_validation_errors() {
                        ui.colored_label(egui::Color32::RED, format!("• {}", error));
                    }
                });
                ui.add_space(5.0);
            }

            // YAML 编辑器
            egui::ScrollArea::vertical()
                .id_source("simple_rule_editor_scroll")
                .show(ui, |ui| {
                    let content = self.rule_editor.get_content().to_string();
                    let mut editable_content = content.clone();

                    let response = ui.add(
                        egui::TextEdit::multiline(&mut editable_content)
                            .font(egui::TextStyle::Monospace)
                            .code_editor()
                            .desired_rows(20)
                            .desired_width(f32::INFINITY)
                    );

                    if response.changed() {
                        self.rule_editor.set_content(&editable_content);
                    }
                });

            ui.add_space(5.0);

            // 按钮
            ui.horizontal(|ui| {
                if ui.button("📋 Load Example").clicked() {
                    self.load_default_rule();
                }

                if ui.button("✓ Validate").clicked() {
                    // 验证会在 set_content 时自动触发
                }
            });
        });
    }

    fn show_inspect_rule(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.set_width(ui.available_width());

            // 标题栏
            ui.horizontal(|ui| {
                let icon = if self.ui_state.inspect_rule_expanded { "▼" } else { "▶" };
                if ui.button(format!("{} Inspect Rule", icon)).clicked() {
                    self.ui_state.inspect_rule_expanded = !self.ui_state.inspect_rule_expanded;
                }
            });

            if self.ui_state.inspect_rule_expanded {
                ui.separator();

                // 显示解析后的规则信息
                egui::ScrollArea::vertical()
                    .max_height(100.0)
                    .show(ui, |ui| {
                        if !self.rule_editor.get_parsed_rules().is_empty() {
                            for rule in self.rule_editor.get_parsed_rules() {
                                ui.label(format!("ID: {}", rule.id));
                                ui.label(format!("Name: {}", rule.name));
                                ui.label(format!("Severity: {:?}", rule.severity));
                                ui.label(format!("Languages: {:?}", rule.languages));
                                ui.separator();
                            }
                        } else {
                            ui.colored_label(
                                egui::Color32::GRAY,
                                "No valid rules parsed. Edit the rule YAML above."
                            );
                        }
                    });
            }
        });
    }
    
    fn show_right_panel(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            // Tab bar - 与 playground 风格一致，包含 Pro/Turbo 模式切换
            ui.horizontal(|ui| {
                ui.style_mut().spacing.item_spacing.x = 0.0;

                // 左侧 tabs
                let test_code_btn = ui.selectable_label(
                    self.ui_state.selected_tab_right == RightTab::TestCode,
                    "test code"
                );
                if test_code_btn.clicked() {
                    self.ui_state.selected_tab_right = RightTab::TestCode;
                }

                let metadata_btn = ui.selectable_label(
                    self.ui_state.selected_tab_right == RightTab::Metadata,
                    "metadata"
                );
                if metadata_btn.clicked() {
                    self.ui_state.selected_tab_right = RightTab::Metadata;
                }

                let docs_btn = ui.selectable_label(
                    self.ui_state.selected_tab_right == RightTab::Docs,
                    "docs"
                );
                if docs_btn.clicked() {
                    self.ui_state.selected_tab_right = RightTab::Docs;
                }

                // 右侧空间
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Turbo 模式按钮
                    let turbo_btn = ui.selectable_label(
                        self.ui_state.analysis_mode == AnalysisMode::Turbo,
                        "Turbo"
                    );
                    if turbo_btn.clicked() {
                        self.ui_state.analysis_mode = AnalysisMode::Turbo;
                    }

                    ui.add_space(4.0);

                    // Pro 模式按钮
                    let pro_btn = ui.selectable_label(
                        self.ui_state.analysis_mode == AnalysisMode::Pro,
                        "Pro"
                    );
                    if pro_btn.clicked() {
                        self.ui_state.analysis_mode = AnalysisMode::Pro;
                    }
                });
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
                                println!("🔍 Analyze button clicked! Mode: {:?}", self.ui_state.analysis_mode);
                                self.run_analysis();
                            }
                        }
                        RightTab::Metadata => {
                            self.show_metadata_view(ui);
                        }
                        RightTab::Docs => {
                            self.show_docs_view(ui);
                        }
                    }
                },
            );

            ui.separator();

            // Bottom panel (results) - 只在 test code tab 显示
            if self.ui_state.selected_tab_right == RightTab::TestCode {
                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), self.ui_state.bottom_panel_height),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        self.results_panel.show(ui, &self.analysis_results);
                    },
                );
            }
        });
    }

    fn show_metadata_view(&self, ui: &mut egui::Ui) {
        ui.heading("Analysis Metadata");
        ui.separator();

        if self.analysis_results.is_empty() {
            ui.colored_label(egui::Color32::GRAY, "Run analysis to see metadata");
        } else {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.group(|ui| {
                    ui.label(format!("Total Findings: {}", self.analysis_results.len()));
                    ui.label(format!("Analysis Mode: {:?}", self.ui_state.analysis_mode));
                    ui.label(format!("Language: {}", self.code_editor.get_language()));

                    ui.separator();

                    // 按严重程度统计
                    let mut critical = 0;
                    let mut error = 0;
                    let mut warning = 0;
                    let mut info = 0;

                    for finding in &self.analysis_results {
                        match finding.severity {
                            astgrep_core::Severity::Critical => critical += 1,
                            astgrep_core::Severity::Error => error += 1,
                            astgrep_core::Severity::Warning => warning += 1,
                            astgrep_core::Severity::Info => info += 1,
                        }
                    }

                    ui.label("Severity Distribution:");
                    ui.label(format!("  Critical: {}", critical));
                    ui.label(format!("  Error: {}", error));
                    ui.label(format!("  Warning: {}", warning));
                    ui.label(format!("  Info: {}", info));
                });
            });
        }
    }

    fn show_docs_view(&self, ui: &mut egui::Ui) {
        ui.heading("astgrep Documentation");
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.label("Welcome to astgrep Playground!");
            ui.add_space(10.0);

            ui.heading("Quick Start");
            ui.label("1. Edit the rule YAML in the left panel (simple or advanced mode)");
            ui.label("2. Enter test code in the 'test code' tab");
            ui.label("3. Click 'Run' or press Ctrl+Enter to analyze");
            ui.label("4. View results in the 'Matches' section below");

            ui.add_space(10.0);
            ui.heading("Analysis Modes");
            ui.label("• Normal: Standard analysis");
            ui.label("• Pro: Enhanced analysis with dataflow");
            ui.label("• Turbo: Fast analysis mode");

            ui.add_space(10.0);
            ui.heading("Keyboard Shortcuts");
            ui.label("• Ctrl+Enter: Run analysis");
            ui.label("• Ctrl+S: Save rule");
            ui.label("• Ctrl+O: Open rule");

            ui.add_space(10.0);
            ui.heading("Resources");
            ui.hyperlink_to("GitHub Repository", "https://github.com/c2j/astgrep");
            ui.hyperlink_to("Rule Writing Guide", "https://github.com/c2j/astgrep/blob/main/docs/astgrep-Guide.md");
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
            "java" => astgrep_core::Language::Java,
            "javascript" => astgrep_core::Language::JavaScript,
            "python" => astgrep_core::Language::Python,
            "php" => astgrep_core::Language::Php,
            "sql" => astgrep_core::Language::Sql,
            "bash" => astgrep_core::Language::Bash,
            "c" => astgrep_core::Language::C,
            "csharp" => astgrep_core::Language::CSharp,
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
        language: astgrep_core::Language,
    ) -> anyhow::Result<Vec<astgrep_core::Finding>> {
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
            astgrep_core::Language::Java => "java",
            astgrep_core::Language::JavaScript => "js",
            astgrep_core::Language::Python => "py",
            astgrep_core::Language::Php => "php",
            astgrep_core::Language::Sql => "sql",
            astgrep_core::Language::Bash => "sh",
            astgrep_core::Language::C => "c",
            astgrep_core::Language::CSharp => "cs",
            astgrep_core::Language::Ruby => "rb",
            astgrep_core::Language::Kotlin => "kt",
            astgrep_core::Language::Swift => "swift",
            astgrep_core::Language::Xml => "xml",
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
                astgrep_core::Severity::Error => egui::Color32::from_rgb(255, 100, 100),    // Red
                astgrep_core::Severity::Warning => egui::Color32::from_rgb(255, 200, 100),  // Orange
                astgrep_core::Severity::Info => egui::Color32::from_rgb(100, 150, 255),     // Blue
                astgrep_core::Severity::Critical => egui::Color32::from_rgb(200, 50, 50),   // Dark Red
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
