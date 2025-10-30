//! Main application structure and state management

use egui;

use crate::components::{
    RuleEditor, CodeEditor, ResultsPanel, MenuBar, StatusBar, SettingsPanel
};
use crate::utils::file_operations::FileOperations;
use astgrep_core::{Language, Finding, OutputFormat};
use astgrep_rules::{RuleEngine, RuleParser, RuleContext};
use astgrep_parser::LanguageParserRegistry;
use std::sync::{mpsc, Arc};
use std::sync::atomic::{AtomicBool, Ordering};



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

    /// Async analysis: receiver for results
    analysis_rx: Option<mpsc::Receiver<AnalysisMessage>>,
    /// Async analysis: cancellation token
    analysis_cancel: Option<Arc<AtomicBool>>,
    /// Async analysis: generation id to ignore stale results
    analysis_gen: u64,
    /// Whether analysis is running
    is_analysis_running: bool,

    /// Internal clipboard buffer for lightweight copy/paste
    internal_clipboard: String,
    /// If set, will be written to system clipboard this frame
    pending_clipboard_copy: Option<String>,

    /// Find/Replace UI state
    show_find_replace: bool,
    find_text: String,
    replace_text: String,
    find_case_sensitive: bool,
    last_find_pos: usize,
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
    // Menu actions requested by MenuBar (processed by App in update loop)
    pub request_open_rule: bool,
    pub request_save_rule: bool,
    pub request_open_code: bool,
    pub request_save_code: bool,
    pub request_export_results: bool,
    pub request_run_analysis: bool,
    pub request_stop_analysis: bool,
    pub request_validate_rule: bool,
    pub request_format_rule: bool,
    pub request_load_examples: bool,
    // Edit actions (lightweight)
    pub request_undo: bool,
    pub request_redo: bool,
    pub request_cut: bool,
    pub request_copy: bool,
    pub request_paste: bool,
    pub request_find: bool,
    pub request_replace: bool,
}


/// Messages sent from analysis background task back to UI
enum AnalysisMessage {
    Finished(u64, Vec<Finding>),
    Error(u64, String),
    Cancelled(u64),
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
            analysis_rx: None,
            analysis_cancel: None,
            analysis_gen: 0,
            is_analysis_running: false,
            internal_clipboard: String::new(),
            pending_clipboard_copy: None,
            show_find_replace: false,
            find_text: String::new(),
            replace_text: String::new(),
            find_case_sensitive: true,
            last_find_pos: 0,
        };

        // Load default rule
        app.load_default_rule();

        app
    }

    fn load_default_rule(&mut self) {
        let lang = self.code_editor.get_language();
        match lang.as_str() {
            "java" => {
                let rule = r#"# Java Code Standards - System.out Logging Rule
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
                let code = r#"public class TestClass {
    public void badLoggingExamples() {
        System.out.println("Debug: Processing user data");
        System.out.print("Status: ");
        System.err.println("Error occurred: " + getMessage());
        System.err.print("Warning: ");

        String userId = "12345";
        System.out.println("User ID: " + userId);
    }

    public void goodLoggingExamples() {
        Logger logger = LoggerFactory.getLogger(TestClass.class);
        logger.info("Processing user data");
        logger.debug("Debug information");
        logger.error("Error occurred", exception);
    }

    private String getMessage() { return "test message"; }
}
"#;
                self.rule_editor.set_content(rule);
                self.code_editor.set_content(code);
            }
            "javascript" => {
                let rule = r#"rules:
  - id: js-avoid-console-log
    name: "Avoid console.log in production"
    description: "Detects usage of console.log"
    message: "Avoid console.log in production"
    severity: WARNING
    languages: [javascript]
    pattern: console.log($MSG)
"#;
                let code = r#"function test() {
  // should match:
  console.log("debug message");
  console.log("user:", getUser());

  // good:
  logger.info("info");
}
"#;
                self.rule_editor.set_content(rule);
                self.code_editor.set_content(code);
            }
            "python" => {
                let rule = r#"rules:
  - id: py-avoid-print
    name: "Avoid print in production"
    description: "Detects usage of print()"
    message: "Avoid print() in production"
    severity: WARNING
    languages: [python]
    pattern: print($MSG)
"#;
                let code = r#"def foo():
    # should match
    print("debug")
    print("user:", user_id)

    # good
    logger.info("ok")
"#;
                self.rule_editor.set_content(rule);
                self.code_editor.set_content(code);
            }
            "go" => {
                let rule = r#"rules:
  - id: go-avoid-fmt-println
    name: "Avoid fmt.Println"
    description: "Detects usage of fmt.Println"
    message: "Avoid fmt.Println in production"
    severity: WARNING
    languages: [go]
    pattern: fmt.Println($MSG)
"#;
                let code = r#"package main
import "fmt"
func main() {
  // should match
  fmt.Println("debug", 123)

  // good
  log.Println("ok")
}
"#;
                self.rule_editor.set_content(rule);
                self.code_editor.set_content(code);
            }
            "rust" => {
                let rule = r#"rules:
  - id: rs-avoid-println
    name: "Avoid println!"
    description: "Detects usage of println! macro"
    message: "Avoid println! in production"
    severity: WARNING
    languages: [rust]
    pattern: println!($MSG)
"#;
                let code = r#"fn main() {
    // should match
    println!("debug: {}", 42);
    // good
    log::info!("ok");
}
"#;
                self.rule_editor.set_content(rule);
                self.code_editor.set_content(code);
            }
            "php" => {
                let rule = r#"rules:
  - id: php-avoid-echo
    name: "Avoid echo"
    description: "Detects usage of echo for logging"
    message: "Avoid echo for logging"
    severity: WARNING
    languages: [php]
    pattern: echo $MSG
"#;
                let code = r#"<?php
function test() {
  // should match
  echo "debug";
  // good
  error_log("ok");
}
"#;
                self.rule_editor.set_content(rule);
                self.code_editor.set_content(code);
            }
            "sql" => {
                let rule = r#"rules:
  - id: sql-avoid-select-star
    name: "Avoid SELECT *"
    description: "Detects usage of SELECT *"
    message: "Avoid SELECT *; specify columns explicitly"
    severity: WARNING
    languages: [sql]
    pattern-either:
      - pattern: SELECT * FROM $TABLE
      - pattern: select * from $TABLE
"#;
                let code = r#"-- should match
SELECT * FROM users;

-- good
SELECT id, name FROM users;

-- should match (lowercase)
select * from orders;
"#;
                self.rule_editor.set_content(rule);
                self.code_editor.set_content(code);
            }
            "xml" => {
                let rule = r#"rules:
  - id: xml-avoid-inline-script
    name: "Avoid inline script tags"
    description: "Detects inline <script> tags"
    message: "Avoid inline <script> in XML/HTML"
    severity: WARNING
    languages: [xml]
    pattern: "<script> $CONTENT </script>"
"#;
                let code = r#"<root>
  <data>value</data>
  <!-- should match -->
  <script>console.log('debug');</script>
  <!-- should also match multi-line -->
  <script>
    alert('x');
  </script>
  <!-- good -->
  <div>ok</div>
</root>
"#;
                self.rule_editor.set_content(rule);
                self.code_editor.set_content(code);
            }
            _ => {
                // Fallback to Java
                self.code_editor.set_language("java");
                self.load_default_rule();
                return;
            }
        }
        self.status_bar.set_status(&format!("Loaded example for {}", lang));
    }
}

impl eframe::App for CrGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Apply theme from settings
        self.apply_theme(ctx);

        // Handle menu bar (collect action requests into ui_state)
        self.menu_bar.show(ctx, &mut self.settings, &mut self.ui_state);
        // Process any actions requested by the menu bar
        self.process_menu_actions();

        // Poll background analysis messages without holding an immutable borrow of self
        {
            use std::sync::mpsc::TryRecvError;
            let mut messages: Vec<AnalysisMessage> = Vec::new();
            let mut disconnected = false;
            if let Some(rx) = self.analysis_rx.as_ref() {
                loop {
                    match rx.try_recv() {
                        Ok(msg) => messages.push(msg),
                        Err(TryRecvError::Empty) => break,
                        Err(TryRecvError::Disconnected) => { disconnected = true; break; }
                    }
                }
            }
            if disconnected { self.analysis_rx = None; }
            for msg in messages {
                match msg {
                    AnalysisMessage::Finished(gen, findings) => {
                        if gen == self.analysis_gen {
                            let mut findings = findings;
                            findings.sort_by(|a, b| {
                                a.location.start_line
                                    .cmp(&b.location.start_line)
                                    .then(a.location.start_column.cmp(&b.location.start_column))
                                    .then(a.location.end_line.cmp(&b.location.end_line))
                                    .then(a.location.end_column.cmp(&b.location.end_column))
                            });
                            self.analysis_results = findings;
                            self.status_bar.analysis_completed(self.analysis_results.len());
                            self.update_code_highlights();
                            self.is_analysis_running = false;
                        }
                    }
                    AnalysisMessage::Error(gen, e) => {
                        if gen == self.analysis_gen {
                            self.analysis_results.clear();
                            self.status_bar.analysis_failed(&e);
                            self.update_code_highlights();
                            self.is_analysis_running = false;
                        }
                    }
                    AnalysisMessage::Cancelled(gen) => {
                        if gen == self.analysis_gen {
                            self.status_bar.set_status("Analysis cancelled");
                            self.is_analysis_running = false;
                        }
                    }
                }
            }
        }

        // Apply any pending system clipboard copy
        if let Some(text) = self.pending_clipboard_copy.take() {
            ctx.output_mut(|o| o.copied_text = text);
        }

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

        // Find/Replace modal
        if self.show_find_replace {
            self.show_find_replace_window(ctx);
        }
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
                    .id_source("inspect_rule_scroll")
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

                // 右侧空间（移除 Pro/Turbo 无用按钮）
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |_ui| {});
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
                            if self.code_editor.show(ui, &mut self.settings) {
                                // Analyze button was clicked
                                println!("🔍 Analyze button clicked! Mode: {:?}", self.ui_state.analysis_mode);
                                self.start_analysis_async();
                            }
                            // Load Example requested from Code Editor toolbar
                            if self.code_editor.take_request_load_examples() {
                                self.load_default_rule();
                                // Clear matches after loading example
                                self.analysis_results.clear();
                                self.results_panel.set_selected_finding(None);
                                self.results_panel.clear_filters();
                                self.update_code_highlights();
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
                // After drawing results, handle any jump request
                if let Some(loc) = self.results_panel.take_pending_jump() {
                    self.jump_to_location(loc);
                }
            }
        });
    }

    fn show_metadata_view(&self, ui: &mut egui::Ui) {
        ui.heading("Analysis Metadata");
        ui.separator();

        if self.analysis_results.is_empty() {
            ui.colored_label(egui::Color32::GRAY, "Run analysis to see metadata");
        } else {
            egui::ScrollArea::vertical().id_source("metadata_scroll").show(ui, |ui| {
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

                    // 完整 JSON 展示
                    ui.add_space(8.0);
                    ui.separator();
                    ui.strong("Full JSON");

                    let metadata_value = serde_json::json!({
                        "analysis_mode": format!("{:?}", self.ui_state.analysis_mode),
                        "language": self.code_editor.get_language(),
                        "total_findings": self.analysis_results.len(),
                        "severity_counts": {
                            "CRITICAL": critical,
                            "ERROR": error,
                            "WARNING": warning,
                            "INFO": info
                        },
                        "findings": self.analysis_results,
                    });

                    let mut json_str = match serde_json::to_string_pretty(&metadata_value) {
                        Ok(s) => s,
                        Err(e) => format!("<error serializing json: {}>", e),
                    };

                    egui::ScrollArea::vertical()
                        .id_source("metadata_json_scroll")
                        .max_height(300.0)
                        .show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut json_str)
                                    .code_editor()
                                    .desired_width(ui.available_width())
                                    .desired_rows(12)
                                    .interactive(false),
                            );
                        });
                });
            });
        }
    }

    fn show_docs_view(&self, ui: &mut egui::Ui) {
        ui.heading("astgrep Documentation");
        ui.separator();

        egui::ScrollArea::vertical().id_source("docs_scroll").show(ui, |ui| {
            ui.label("Welcome to astgrep Playground!");
            ui.add_space(10.0);

            ui.heading("Quick Start");
            ui.label("1. Edit the rule YAML in the left panel (simple or advanced mode)");
            ui.label("2. Enter test code in the 'test code' tab");
            ui.label("3. Click 'Run' or press Ctrl+Enter to analyze");
            ui.label("4. View results in the 'Matches' section below");



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
        self.status_bar.analysis_started();

        // Get current code from code editor
        let source_code = self.code_editor.get_content().to_string();
        let language = self.code_editor.get_language().to_string();

        // Get current rule from rule editor
        let rule_content = self.rule_editor.get_content().to_string();

        if source_code.trim().is_empty() {
            println!("⚠️ No source code to analyze");
            self.analysis_results.clear();
            self.update_code_highlights();
            self.status_bar.analysis_completed(0);
            return;
        }

        if rule_content.trim().is_empty() {
            println!("⚠️ No rules defined");
            self.analysis_results.clear();
            self.update_code_highlights();
            self.status_bar.analysis_completed(0);
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
            "ruby" => astgrep_core::Language::Ruby,
            "kotlin" => astgrep_core::Language::Kotlin,
            "swift" => astgrep_core::Language::Swift,
            "xml" => astgrep_core::Language::Xml,
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
                let mut findings = findings;
                findings.sort_by(|a, b| {
                    a.location.start_line
                        .cmp(&b.location.start_line)
                        .then(a.location.start_column.cmp(&b.location.start_column))
                        .then(a.location.end_line.cmp(&b.location.end_line))
                        .then(a.location.end_column.cmp(&b.location.end_column))
                });
                self.analysis_results = findings;
            }
            Err(e) => {
                println!("❌ Analysis failed: {}", e);
                self.analysis_results.clear();
                self.status_bar.analysis_failed(&format!("{}", e));
            }
        }

        println!("📊 Analysis completed with {} findings", self.analysis_results.len());
        self.status_bar.analysis_completed(self.analysis_results.len());

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
        let mut context = RuleContext::new(
            file_path.to_string_lossy().to_string(),
            language,
            source_code.to_string(),
        );
        // GUI: default to ON; YAML can override per-rule in engine
        context = context.add_data("sql_statement_boundary".to_string(), "true".to_string());

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
    fn cancel_analysis(&mut self) {
        if let Some(cancel) = &self.analysis_cancel {
            cancel.store(true, Ordering::Relaxed);
            self.status_bar.set_status("Cancelling analysis...");
        } else {
            self.status_bar.set_status("No analysis in progress");
        }
    }

    fn start_analysis_async(&mut self) {
        // Prepare inputs
        let source_code = self.code_editor.get_content().to_string();
        let language = self.code_editor.get_language().to_string();
        let rule_content = self.rule_editor.get_content().to_string();

        if source_code.trim().is_empty() {
            self.analysis_results.clear();
            self.update_code_highlights();
            self.status_bar.analysis_completed(0);
            return;
        }
        if rule_content.trim().is_empty() {
            self.analysis_results.clear();
            self.update_code_highlights();
            self.status_bar.analysis_completed(0);
            return;
        }

        let lang = match Self::map_language(&language) {
            Some(l) => l,
            None => {
                self.status_bar.analysis_failed(&format!("Unsupported language: {}", language));
                return;
            }
        };

        // If a previous analysis is running, request cancel
        if self.is_analysis_running {
            if let Some(c) = &self.analysis_cancel { c.store(true, Ordering::Relaxed); }
        }

        self.status_bar.analysis_started();
        self.is_analysis_running = true;
        self.analysis_gen = self.analysis_gen.wrapping_add(1);
        let gen = self.analysis_gen;

        let (tx, rx) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        self.analysis_rx = Some(rx);
        self.analysis_cancel = Some(cancel.clone());

        std::thread::spawn(move || {
            if cancel.load(Ordering::Relaxed) {
                let _ = tx.send(AnalysisMessage::Cancelled(gen));
                return;
            }
            match CrGuiApp::analyze_code_with_rules_stateless(&source_code, &rule_content, lang, cancel.clone()) {
                Ok(Some(findings)) => { let _ = tx.send(AnalysisMessage::Finished(gen, findings)); }
                Ok(None) => { let _ = tx.send(AnalysisMessage::Cancelled(gen)); }
                Err(e) => { let _ = tx.send(AnalysisMessage::Error(gen, format!("{}", e))); }
            }
        });
    }

    fn map_language(language: &str) -> Option<astgrep_core::Language> {
        Some(match language {
            "java" => astgrep_core::Language::Java,
            "javascript" => astgrep_core::Language::JavaScript,
            "python" => astgrep_core::Language::Python,
            "php" => astgrep_core::Language::Php,
            "sql" => astgrep_core::Language::Sql,
            "bash" => astgrep_core::Language::Bash,
            "c" => astgrep_core::Language::C,
            "csharp" | "c#" => astgrep_core::Language::CSharp,
            "ruby" => astgrep_core::Language::Ruby,
            "kotlin" => astgrep_core::Language::Kotlin,
            "swift" => astgrep_core::Language::Swift,
            "xml" => astgrep_core::Language::Xml,
            _ => return None,
        })
    }

    fn analyze_code_with_rules_stateless(
        source_code: &str,
        rule_content: &str,
        language: astgrep_core::Language,
        cancel: Arc<AtomicBool>,
    ) -> anyhow::Result<Option<Vec<astgrep_core::Finding>>> {
        use std::path::PathBuf;

        if cancel.load(Ordering::Relaxed) { return Ok(None); }

        // Parse rules
        let rule_parser = RuleParser::new();
        let parsed_rules = rule_parser.parse_yaml(rule_content)
            .map_err(|e| anyhow::anyhow!("Failed to parse rules: {}", e))?;
        if cancel.load(Ordering::Relaxed) { return Ok(None); }

        let mut rule_engine = RuleEngine::new();
        for rule in parsed_rules {
            if let Err(e) = rule_engine.add_rule(rule) {
                eprintln!("Failed to add rule: {}", e);
            }
        }
        if cancel.load(Ordering::Relaxed) { return Ok(None); }

        // Parse source
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
        let parser_registry = LanguageParserRegistry::new();
        let ast = parser_registry.parse_file(&file_path, source_code)
            .map_err(|e| anyhow::anyhow!("Failed to parse source code: {}", e))?;
        if cancel.load(Ordering::Relaxed) { return Ok(None); }

        let mut context = RuleContext::new(
            file_path.to_string_lossy().to_string(),
            language,
            source_code.to_string(),
        );
        // GUI: default to ON; YAML can override per-rule in engine
        context = context.add_data("sql_statement_boundary".to_string(), "true".to_string());
        let rule_results = rule_engine.execute_rules(&*ast, &context)
            .map_err(|e| anyhow::anyhow!("Failed to execute rules: {}", e))?;
        if cancel.load(Ordering::Relaxed) { return Ok(None); }

        let mut findings = Vec::new();
        for result in rule_results { findings.extend(result.findings); }
        Ok(Some(findings))
    }

    fn show_find_replace_window(&mut self, ctx: &egui::Context) {
        let mut open = self.show_find_replace;
        egui::Window::new("Find / Replace")
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Find:");
                    ui.text_edit_singleline(&mut self.find_text);
                });
                ui.horizontal(|ui| {
                    ui.label("Replace:");
                    ui.text_edit_singleline(&mut self.replace_text);
                });
                ui.checkbox(&mut self.find_case_sensitive, "Case sensitive");
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Find Next").clicked() {
                        if self.find_next_and_highlight() {
                            self.status_bar.set_status("Match highlighted");
                        } else {
                            self.status_bar.set_status("No match");
                        }
                    }
                    if ui.button("Replace All").clicked() {
                        let n = self.replace_all();
                        self.status_bar.set_status(&format!("Replaced {} occurrence(s)", n));
                    }
                    if ui.button("Close").clicked() { /* handled by `open` */ }
                });
            });
        self.show_find_replace = open;
    }

    fn find_next_and_highlight(&mut self) -> bool {
        let needle = self.find_text.as_str();
        if needle.is_empty() { return false; }
        let hay = self.code_editor.get_content().to_string();
        let start = self.last_find_pos.min(hay.len());

        // Simple ASCII case-insensitive search if needed
        let found = if self.find_case_sensitive {
            hay[start..].find(needle).map(|i| start + i)
        } else {
            let hay_bytes = hay.as_bytes();
            let needle_bytes = needle.as_bytes();
            let mut pos = None;
            'outer: for i in start..=hay_bytes.len().saturating_sub(needle_bytes.len()) {
                for j in 0..needle_bytes.len() {
                    let a = hay_bytes[i + j];
                    let b = needle_bytes[j];
                    if a.to_ascii_lowercase() != b.to_ascii_lowercase() { continue 'outer; }
                }
                pos = Some(i);
                break;
            }
            pos
        };

        let Some(idx) = found else { return false; };
        let end_idx = idx + needle.len();

        let (s_line, s_col) = Self::index_to_line_col(&hay, idx);
        let (e_line, e_col) = Self::index_to_line_col(&hay, end_idx);

        use crate::components::code_editor::HighlightRange;
        self.code_editor.clear_highlights();
        self.code_editor.add_highlight(HighlightRange {
            start_line: s_line,
            start_col: s_col,
            end_line: e_line,
            end_col: e_col,
            color: egui::Color32::from_rgb(255, 230, 0),
            message: "Find result".to_string(),
        });
        self.ui_state.selected_tab_right = RightTab::TestCode;
        self.last_find_pos = end_idx;
        true
    }

    fn index_to_line_col(s: &str, byte_idx: usize) -> (usize, usize) {
        let mut line = 0usize;
        let mut col = 0usize;
        for (i, ch) in s.char_indices() {
            if i >= byte_idx { break; }
            if ch == '\n' { line += 1; col = 0; } else { col += 1; }
        }
        (line, col)
    }

    fn replace_all(&mut self) -> usize {
        let needle = self.find_text.clone();
        if needle.is_empty() { return 0; }
        let repl = self.replace_text.clone();
        let hay = self.code_editor.get_content().to_string();
        let bytes = hay.as_bytes();
        let nbytes = needle.as_bytes();
        if nbytes.is_empty() || bytes.len() < nbytes.len() { return 0; }

        let mut out = String::with_capacity(hay.len());
        let mut last = 0usize;
        let mut i = 0usize;
        let mut count = 0usize;
        while i + nbytes.len() <= bytes.len() {
            let mut match_here = true;
            for j in 0..nbytes.len() {
                let a = bytes[i + j];
                let b = nbytes[j];
                let eq = if self.find_case_sensitive { a == b } else { a.to_ascii_lowercase() == b.to_ascii_lowercase() };
                if !eq { match_here = false; break; }
            }
            if match_here {
                out.push_str(&hay[last..i]);
                out.push_str(&repl);
                i += nbytes.len();
                last = i;
                count += 1;
            } else {
                i += 1;
            }
        }
        out.push_str(&hay[last..]);
        if count > 0 {
            self.code_editor.set_content_with_undo(out);
            self.last_find_pos = 0;
            // Refresh highlights
            self.code_editor.clear_highlights();
        }
        count
    }



    // Apply light/dark theme immediately when setting changes
    fn apply_theme(&self, ctx: &egui::Context) {
        match self.settings.theme {
            Theme::Dark => ctx.set_visuals(egui::Visuals::dark()),
            Theme::Light => ctx.set_visuals(egui::Visuals::light()),
        }
    }

    // Handle user actions requested via MenuBar
    fn process_menu_actions(&mut self) {
        if self.ui_state.request_open_rule { self.ui_state.request_open_rule = false; self.menu_open_rule(); }
        if self.ui_state.request_save_rule { self.ui_state.request_save_rule = false; self.menu_save_rule(); }
        if self.ui_state.request_open_code { self.ui_state.request_open_code = false; self.menu_open_code(); }
        if self.ui_state.request_save_code { self.ui_state.request_save_code = false; self.menu_save_code(); }
        if self.ui_state.request_export_results { self.ui_state.request_export_results = false; self.menu_export_results(); }
        if self.ui_state.request_run_analysis { self.ui_state.request_run_analysis = false; self.start_analysis_async(); }
        if self.ui_state.request_stop_analysis { self.ui_state.request_stop_analysis = false; self.cancel_analysis(); }
        if self.ui_state.request_validate_rule { self.ui_state.request_validate_rule = false; self.menu_validate_rule(); }
        if self.ui_state.request_format_rule { self.ui_state.request_format_rule = false; self.menu_format_rule(); }
        if self.ui_state.request_load_examples { self.ui_state.request_load_examples = false; self.load_default_rule(); self.analysis_results.clear(); self.results_panel.set_selected_finding(None); self.results_panel.clear_filters(); self.update_code_highlights(); }
        // Edit actions
        if self.ui_state.request_undo { self.ui_state.request_undo = false; if self.code_editor.undo() { self.status_bar.set_status("Undone"); } else { self.status_bar.set_status("Nothing to undo"); } }
        if self.ui_state.request_redo { self.ui_state.request_redo = false; if self.code_editor.redo() { self.status_bar.set_status("Redone"); } else { self.status_bar.set_status("Nothing to redo"); } }
        if self.ui_state.request_copy { self.ui_state.request_copy = false; self.internal_clipboard = self.code_editor.get_content().to_string(); self.pending_clipboard_copy = Some(self.internal_clipboard.clone()); self.status_bar.set_status("Copied code to clipboard"); }
        if self.ui_state.request_cut { self.ui_state.request_cut = false; self.internal_clipboard = self.code_editor.get_content().to_string(); self.pending_clipboard_copy = Some(self.internal_clipboard.clone()); self.code_editor.set_content(""); self.status_bar.set_status("Cut code to clipboard"); }
        if self.ui_state.request_paste { self.ui_state.request_paste = false; if !self.internal_clipboard.is_empty() { self.code_editor.paste_append(&self.internal_clipboard.clone()); self.status_bar.set_status("Pasted from clipboard"); } else { self.status_bar.set_status("Clipboard empty"); } }
        if self.ui_state.request_find { self.ui_state.request_find = false; self.show_find_replace = true; }
        if self.ui_state.request_replace { self.ui_state.request_replace = false; self.show_find_replace = true; }
    }

    fn menu_open_rule(&mut self) {
        if let Some(path) = FileOperations::open_file_dialog("Open Rule", &FileOperations::get_rule_file_filters()) {
            if let Ok(content) = FileOperations::read_file(&path) {
                self.rule_editor.set_content(&content); // triggers parse
                self.status_bar.set_status(&format!("Loaded rule file: {}", path.display()));
            }
        }
    }

    fn menu_save_rule(&mut self) {
        if let Some(path) = FileOperations::save_file_dialog("Save Rule", &FileOperations::get_rule_file_filters()) {
            let content = self.rule_editor.get_content().to_string();
            if let Err(e) = FileOperations::write_file(&path, &content) {
                self.status_bar.analysis_failed(&format!("Failed to save rule: {}", e));
            } else {
                self.status_bar.set_status(&format!("Saved rule to {}", path.display()));
            }
        }
    }

    fn menu_open_code(&mut self) {
        if let Some(path) = FileOperations::open_file_dialog("Open Code", &FileOperations::get_source_file_filters()) {
            match FileOperations::read_file(&path) {
                Ok(content) => {
                    self.code_editor.set_content(&content);
                    if let Some(lang) = FileOperations::detect_language(&path) { self.code_editor.set_language(&lang); }
                    self.ui_state.selected_tab_right = RightTab::TestCode;
                    self.status_bar.set_status(&format!("Loaded source: {}", path.display()));
                }
                Err(e) => self.status_bar.analysis_failed(&format!("Failed to open code: {}", e)),
            }
        }
    }

    fn menu_save_code(&mut self) {
        if let Some(path) = FileOperations::save_file_dialog("Save Code", &FileOperations::get_source_file_filters()) {
            let content = self.code_editor.get_content().to_string();
            if let Err(e) = FileOperations::write_file(&path, &content) {
                self.status_bar.analysis_failed(&format!("Failed to save code: {}", e));
            } else {
                self.status_bar.set_status(&format!("Saved code to {}", path.display()));
            }
        }
    }

    fn serialize_sarif(&self) -> anyhow::Result<String> {
        use serde_json::json;
        let results: Vec<serde_json::Value> = self.analysis_results.iter().map(|f| {
            json!({
                "ruleId": f.rule_id,
                "level": match f.severity { astgrep_core::Severity::Error => "error", astgrep_core::Severity::Warning => "warning", astgrep_core::Severity::Info => "note", astgrep_core::Severity::Critical => "error" },
                "message": { "text": f.message },
                "locations": [{
                    "physicalLocation": {
                        "artifactLocation": { "uri": f.location.file.to_string_lossy() },
                        "region": {
                            "startLine": f.location.start_line,
                            "startColumn": f.location.start_column,
                            "endLine": f.location.end_line,
                            "endColumn": f.location.end_column
                        }
                    }
                }]
            })
        }).collect();
        let sarif = json!({
            "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
            "version": "2.1.0",
            "runs": [{
                "tool": {"driver": {"name": "astgrep", "informationUri": "https://github.com/c2j/astgrep"}},
                "results": results
            }]
        });
        Ok(serde_json::to_string_pretty(&sarif)?)
    }

    fn menu_export_results(&mut self) {
        if self.analysis_results.is_empty() {
            self.status_bar.set_status("No findings to export");
            return;
        }
        if let Some(path) = FileOperations::save_file_dialog("Export Results", &FileOperations::get_export_file_filters()) {
            let ext = FileOperations::get_extension(&path).unwrap_or_else(|| "json".to_string());
            let format = OutputFormat::from_str(&ext).unwrap_or(OutputFormat::Json);
            let content_res = match format {
                OutputFormat::Json => serde_json::to_string_pretty(&self.analysis_results).map_err(|e| anyhow::anyhow!(e)),
                OutputFormat::Yaml => serde_yaml::to_string(&self.analysis_results).map_err(|e| anyhow::anyhow!(e)),
                OutputFormat::Sarif => self.serialize_sarif(),
                _ => serde_json::to_string_pretty(&self.analysis_results).map_err(|e| anyhow::anyhow!(e)),
            };
            match content_res {
                Ok(data) => {
                    if let Err(e) = FileOperations::write_file(&path, &data) {
                        self.status_bar.analysis_failed(&format!("Failed to export: {}", e));
                    } else {
                        self.status_bar.set_status(&format!("Exported results to {}", path.display()));
                    }
                }
                Err(e) => self.status_bar.analysis_failed(&format!("Serialize failed: {}", e)),
            }
        }
    }

    fn menu_validate_rule(&mut self) {
        // Re-parse current YAML
        let current = self.rule_editor.get_content().to_string();
        self.rule_editor.set_content(&current);
        self.status_bar.set_status("Rule validated");
    }

    fn menu_format_rule(&mut self) {
        // Try to format YAML (RuleEditor implements formatting)
        if self.rule_editor.format_yaml() {
            self.status_bar.set_status("Rule YAML formatted");
        } else {
            self.status_bar.set_status("Nothing to format or invalid YAML");
        }
    }

    fn jump_to_location(&mut self, loc: astgrep_core::Location) {
        // Switch to code tab and highlight
        self.ui_state.selected_tab_right = RightTab::TestCode;
        self.code_editor.clear_highlights();
        use crate::components::code_editor::HighlightRange;
        let color = egui::Color32::from_rgb(255, 230, 0);
        self.code_editor.add_highlight(HighlightRange {
            start_line: loc.start_line.saturating_sub(1),
            start_col: loc.start_column.saturating_sub(1),
            end_line: loc.end_line.saturating_sub(1),
            end_col: loc.end_column.saturating_sub(1),
            color,
            message: "Jumped to finding".to_string(),
        });
        self.status_bar.set_status(&format!("Jumped to {}:{}", loc.start_line, loc.start_column));
    }


}
