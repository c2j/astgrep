//! Code editor component with syntax highlighting

use egui;
use crate::app::AppSettings;

/// Code editor component
pub struct CodeEditor {
    /// Source code content
    content: String,

    /// Current language
    language: String,

    /// Line numbers visibility
    show_line_numbers: bool,

    /// Current font size (synced from AppSettings)
    font_size: f32,

    /// Current cursor position
    cursor_pos: usize,

    /// Highlighted ranges (for showing analysis results)
    highlighted_ranges: Vec<HighlightRange>,

    /// Simple undo stack (stores previous contents)
    undo_stack: Vec<String>,
    /// Simple redo stack
    redo_stack: Vec<String>,

    /// Toolbar action: request to load example for current language
    request_load_examples: bool,
}

#[derive(Clone)]
pub struct HighlightRange {
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
    pub color: egui::Color32,
    pub message: String,
}

impl CodeEditor {
    pub fn new() -> Self {
        Self {
            content: String::new(),
            language: "java".to_string(),
            show_line_numbers: true,
            font_size: 14.0,
            cursor_pos: 0,
            highlighted_ranges: Vec::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            request_load_examples: false,
        }
    }

    pub fn set_content(&mut self, content: &str) {
        self.content = content.to_string();
    }

    /// Set content and record undo snapshot
    pub fn set_content_with_undo(&mut self, new_content: String) {
        let prev = std::mem::replace(&mut self.content, new_content);
        self.undo_stack.push(prev);
        self.redo_stack.clear();
    }

    pub fn get_content(&self) -> &str {
        &self.content
    }

    pub fn set_language(&mut self, language: &str) {
        self.language = language.to_string();
    }
    /// Take and clear the one-shot request flag for loading examples
    pub fn take_request_load_examples(&mut self) -> bool {
        let r = self.request_load_examples;
        self.request_load_examples = false;
        r
    }


    pub fn get_language(&self) -> String {
        self.language.clone()
    }

    pub fn add_highlight(&mut self, range: HighlightRange) {
        self.highlighted_ranges.push(range);
    }

    pub fn clear_highlights(&mut self) {
        self.highlighted_ranges.clear();
    }

    pub fn show(&mut self, ui: &mut egui::Ui, settings: &mut AppSettings) -> bool {
        let mut analyze_clicked = false;

        // Sync local flags from global settings
        self.show_line_numbers = settings.show_line_numbers;
        self.font_size = settings.font_size;

        ui.vertical(|ui| {
            // Toolbar
            ui.horizontal(|ui| {
                ui.label("Language:");
                egui::ComboBox::from_id_source("language_selector")
                    .selected_text(&self.language)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.language, "java".to_string(), "Java");
                        ui.selectable_value(&mut self.language, "javascript".to_string(), "JavaScript");
                        ui.selectable_value(&mut self.language, "python".to_string(), "Python");
                        ui.selectable_value(&mut self.language, "sql".to_string(), "SQL");
                        ui.selectable_value(&mut self.language, "xml".to_string(), "XML");
                        ui.selectable_value(&mut self.language, "php".to_string(), "PHP");
                    });

                if ui.button("📋 Load Example").clicked() {
                    self.request_load_examples = true;
                }

                ui.separator();

                if ui.button("📁 Open").clicked() {
                    self.open_file();
                }

                if ui.button("💾 Save").clicked() {
                    self.save_file();
                }

                if ui.button("🔄 Clear").clicked() {
                    let prev = std::mem::take(&mut self.content);
                    self.undo_stack.push(prev);
                    self.redo_stack.clear();
                    self.clear_highlights();
                }

                ui.separator();

                // Toggle global setting for line numbers
                let mut show_ln = settings.show_line_numbers;
                if ui.checkbox(&mut show_ln, "Line numbers").changed() {
                    settings.show_line_numbers = show_ln;
                    self.show_line_numbers = show_ln;
                }

                ui.separator();

                if ui.button("🔍 Analyze").clicked() {
                    analyze_clicked = true;
                }
            });

            ui.separator();

            // Code editor area
            egui::ScrollArea::both()
                .id_source("code_editor_scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    self.show_code_editor(ui, settings);
                });
        });

        analyze_clicked
    }

    fn show_code_editor(&mut self, ui: &mut egui::Ui, settings: &AppSettings) {
        // 在渲染前快照一份内容，用于本帧的撤销记录（避免多处编辑重复入栈）
        let pre_content_snapshot = self.content.clone();
        let mut snapshot_taken = false;

        // 使用更简单的方法：在TextEdit前面添加行号前缀
        if self.show_line_numbers {
            // 创建带行号的显示文本
            let lines: Vec<&str> = self.content.lines().collect();
            let line_count = lines.len().max(1);
            let digits = (line_count as f32).log10().floor() as usize + 1;

            // 构建带行号的文本用于显示
            let mut display_content = String::new();
            for (i, line) in lines.iter().enumerate() {
                display_content.push_str(&format!("{:>width$} │ {}\n", i + 1, line, width = digits));
            }

            // 如果内容为空，至少显示一行
            if display_content.is_empty() {
                display_content = format!("{:>width$} │ \n", 1, width = digits);
            }

            // 移除最后的换行符
            if display_content.ends_with('\n') {
                display_content.pop();
            }

            // 单一滚动容器：去掉内部额外的 ScrollArea，避免错位
            ui.horizontal(|ui| {
                let font_id = egui::FontId::monospace(settings.font_size);
                let line_count = self.content.lines().count().max(1);
                let line_height = ui.fonts(|f| f.row_height(&font_id));

                // 行号列宽度（按位数 + 少许间距）
                let digits = (line_count as f32).log10().floor() as usize + 1;
                let digit_w = ui.fonts(|f| f.glyph_width(&font_id, '0'));
                let line_number_width = digit_w * (digits as f32 + 2.0);

                // 渲染文本编辑器并获取精确的文本绘制起点
                let te = egui::TextEdit::multiline(&mut self.content)
                    .font(font_id.clone())
                    .code_editor()
                    .desired_rows(line_count)
                    .desired_width(ui.available_width() - line_number_width - 6.0)
                    .interactive(true);
                let output = te.show(ui);

                if output.response.changed() && !snapshot_taken {
                    self.undo_stack.push(pre_content_snapshot.clone());
                    self.redo_stack.clear();
                    snapshot_taken = true;
                }

                let text_rect = output.response.rect;
                let text_draw_pos = output.text_draw_pos; // 精确对齐用

                // 行号区域：与文本同高并靠左
                let line_number_rect = egui::Rect::from_min_size(
                    egui::pos2(text_rect.min.x - line_number_width - 4.0, text_rect.min.y),
                    egui::vec2(line_number_width, text_rect.height()),
                );

                // 背景
                ui.painter().rect_filled(
                    line_number_rect,
                    egui::Rounding::ZERO,
                    ui.style().visuals.code_bg_color.gamma_multiply(0.8),
                );

                // 绘制行号（以 TextEdit 的实际 text_draw_pos.y 作为对齐基准）
                for (i, _) in self.content.lines().enumerate() {
                    let y = text_draw_pos.y + (i as f32) * line_height;
                    let has_highlight = self.highlighted_ranges.iter().any(|h| h.start_line == i);

                    ui.painter().text(
                        egui::pos2(line_number_rect.max.x - 16.0, y),
                        egui::Align2::RIGHT_TOP,
                        (i + 1).to_string(),
                        font_id.clone(),
                        ui.style().visuals.text_color().gamma_multiply(0.6),
                    );

                    let marker = if has_highlight { "*" } else { " " };
                    ui.painter().text(
                        egui::pos2(line_number_rect.max.x - 8.0, y),
                        egui::Align2::RIGHT_TOP,
                        marker.to_string(),
                        font_id.clone(),
                        if has_highlight { egui::Color32::from_rgb(255, 0, 0) } else { ui.style().visuals.text_color().gamma_multiply(0.6) },
                    );
                }

                // 分隔线
                ui.painter().vline(
                    line_number_rect.max.x,
                    line_number_rect.y_range(),
                    egui::Stroke::new(1.0, ui.style().visuals.text_color().gamma_multiply(0.3)),
                );
            });
        } else {
            // 没有行号时，直接显示编辑器
            let resp = ui.add(
                egui::TextEdit::multiline(&mut self.content)
                    .font(egui::FontId::monospace(settings.font_size))
                    .code_editor()
                    .desired_width(ui.available_width())
                    .desired_rows(30)
            );
            if resp.changed() && !snapshot_taken {
                self.undo_stack.push(pre_content_snapshot);
                self.redo_stack.clear();
            }
        }
    }

    /// Undo last change, returns true if changed
    pub fn undo(&mut self) -> bool {
        if let Some(prev) = self.undo_stack.pop() {
            let curr = std::mem::replace(&mut self.content, prev);
            self.redo_stack.push(curr);
            true
        } else { false }
    }

    /// Redo last undone change, returns true if changed
    pub fn redo(&mut self) -> bool {
        if let Some(next) = self.redo_stack.pop() {
            let curr = std::mem::replace(&mut self.content, next);
            self.undo_stack.push(curr);
            true
        } else { false }
    }

    /// Append given text at the end and record undo snapshot
    pub fn paste_append(&mut self, text: &str) {
        self.undo_stack.push(self.content.clone());
        self.redo_stack.clear();
        self.content.push_str(text);
    }

    /// 最终版本的高亮绘制方法 - 修复偏移
    fn draw_highlights_for_display_final(&self, ui: &mut egui::Ui, display_content: &str, text_rect: egui::Rect) {
        if self.highlighted_ranges.is_empty() {
            return;
        }

        let font_id = egui::FontId::monospace(self.font_size);
        let line_height = ui.fonts(|f| f.row_height(&font_id));
        let display_lines: Vec<&str> = display_content.lines().collect();

        println!("🎨 Drawing {} highlights (CORRECTED method)", self.highlighted_ranges.len());

        for highlight in &self.highlighted_ranges {
            println!("🎯 Highlighting line {} (0-based: {})", highlight.start_line + 1, highlight.start_line);

            if highlight.start_line < display_lines.len() {
                // 修正偏移：减去一行的高度来对齐
                let corrected_y = text_rect.min.y + (highlight.start_line as f32 * line_height) - line_height;

                // 高亮整行
                let highlight_rect = egui::Rect::from_min_size(
                    egui::pos2(text_rect.min.x + 4.0, corrected_y),
                    egui::vec2(text_rect.width() - 8.0, line_height)
                );

                // 使用明显的黄色高亮
                ui.painter().rect_filled(
                    highlight_rect,
                    egui::Rounding::same(3.0),
                    egui::Color32::from_rgba_unmultiplied(255, 255, 0, 100) // 黄色背景
                );

                // 绘制橙色边框
                ui.painter().rect_stroke(
                    highlight_rect,
                    egui::Rounding::same(3.0),
                    egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 165, 0)) // 橙色边框
                );

                println!("✅ Drew CORRECTED highlight at y={}, rect={:?}", corrected_y, highlight_rect);
                println!("📍 Line content: '{}'", display_lines.get(highlight.start_line).unwrap_or(&"<out of bounds>"));
            }
        }
    }

    /// Overlay高亮绘制方法 - 在文本上方绘制
    fn draw_highlights_for_display_overlay(&self, ui: &mut egui::Ui, display_content: &str, text_rect: egui::Rect) {
        if self.highlighted_ranges.is_empty() {
            return;
        }

        let font_id = egui::FontId::monospace(self.font_size);
        let line_height = ui.fonts(|f| f.row_height(&font_id));
        let display_lines: Vec<&str> = display_content.lines().collect();

        println!("🎨 Drawing {} highlights (overlay method)", self.highlighted_ranges.len());
        println!("📍 Text rect: {:?}", text_rect);
        println!("📏 Line height: {}", line_height);
        println!("📄 Total lines: {}", display_lines.len());

        // 打印前几行内容以便调试
        for (i, line) in display_lines.iter().enumerate().take(15) {
            println!("📝 Line {}: '{}'", i + 1, line);
        }

        for highlight in &self.highlighted_ranges {
            println!("🎯 Highlighting line {} (0-based: {})", highlight.start_line + 1, highlight.start_line);

            if highlight.start_line < display_lines.len() {
                // 使用文本区域的实际位置，但需要考虑TextEdit内部的padding
                // TextEdit通常有一些内部padding，我们需要调整
                let text_padding = 4.0; // TextEdit的内部padding
                let y_pos = text_rect.min.y + text_padding + (highlight.start_line as f32 * line_height);

                // 高亮整行，使用文本区域的宽度
                let highlight_rect = egui::Rect::from_min_size(
                    egui::pos2(text_rect.min.x + text_padding, y_pos),
                    egui::vec2(text_rect.width() - 2.0 * text_padding, line_height)
                );

                // 绘制高亮背景，使用更明显的颜色
                ui.painter().rect_filled(
                    highlight_rect,
                    egui::Rounding::same(2.0),
                    egui::Color32::from_rgba_unmultiplied(255, 255, 0, 120) // 增加透明度到120
                );

                // 绘制边框使其更明显
                ui.painter().rect_stroke(
                    highlight_rect,
                    egui::Rounding::same(2.0),
                    egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 150, 0)) // 增加边框宽度
                );

                println!("✅ Drew overlay highlight at y={} (adjusted), rect={:?}", y_pos, highlight_rect);
                println!("📍 Expected line content: '{}'", display_lines.get(highlight.start_line).unwrap_or(&"<out of bounds>"));
            }
        }
    }

    /// 简化的高亮绘制方法
    fn draw_highlights_for_display_simple(&self, ui: &mut egui::Ui, display_content: &str) {
        if self.highlighted_ranges.is_empty() {
            return;
        }

        let font_id = egui::FontId::monospace(self.font_size);
        let line_height = ui.fonts(|f| f.row_height(&font_id));
        let display_lines: Vec<&str> = display_content.lines().collect();

        println!("🎨 Drawing {} highlights (simple method)", self.highlighted_ranges.len());

        for highlight in &self.highlighted_ranges {
            println!("🎯 Highlighting line {} (0-based: {})", highlight.start_line + 1, highlight.start_line);

            if highlight.start_line < display_lines.len() {
                let y_pos = ui.min_rect().min.y + (highlight.start_line as f32 * line_height);

                // 简单地高亮整行
                let highlight_rect = egui::Rect::from_min_size(
                    egui::pos2(ui.min_rect().min.x, y_pos),
                    egui::vec2(ui.available_width(), line_height)
                );

                // 绘制高亮背景
                ui.painter().rect_filled(
                    highlight_rect,
                    egui::Rounding::same(2.0),
                    highlight.color.gamma_multiply(0.3)
                );

                println!("✅ Drew simple highlight at y={}, full width", y_pos);
            }
        }
    }

    /// 为显示文本绘制高亮（修复版本）
    fn draw_highlights_for_display(&self, ui: &mut egui::Ui, display_content: &str, text_rect: egui::Rect) {
        if self.highlighted_ranges.is_empty() {
            return;
        }

        let font_id = egui::FontId::monospace(self.font_size);
        let line_height = ui.fonts(|f| f.row_height(&font_id));
        let char_width = ui.fonts(|f| f.glyph_width(&font_id, ' '));
        let display_lines: Vec<&str> = display_content.lines().collect();

        println!("🎨 Drawing {} highlights for display", self.highlighted_ranges.len());
        println!("📏 Line height: {}, Char width: {}", line_height, char_width);
        println!("📄 Display has {} lines, Text rect: {:?}", display_lines.len(), text_rect);

        for highlight in &self.highlighted_ranges {
            println!("🎯 Processing highlight for lines {}-{} (0-based: {}-{})",
                highlight.start_line + 1, highlight.end_line + 1,
                highlight.start_line, highlight.end_line);

            // 重要：分析结果的行号是基于原始代码的（0-based）
            // 我们需要在显示文本中找到对应的行
            for original_line_num in highlight.start_line..=highlight.end_line {
                // 在显示文本中，每一行都对应原始代码的一行
                // 显示行号 = 原始行号（因为显示文本就是原始代码加上行号前缀）
                let display_line_num = original_line_num;

                if display_line_num < display_lines.len() {
                    // 计算Y位置：基于文本区域的顶部
                    let y_pos = text_rect.min.y + (display_line_num as f32 * line_height);

                    // 获取显示行内容
                    let display_line = display_lines[display_line_num];

                    if let Some(pipe_pos) = display_line.find('│') {
                        // 行号前缀宽度：行号 + " │ "
                        let prefix_width = (pipe_pos + 2) as f32 * char_width;

                        // 获取代码部分（去掉行号前缀）
                        // 需要正确处理UTF-8字符边界
                        let code_part = if let Some(pipe_char_pos) = display_line.char_indices().find(|(_, c)| *c == '│') {
                            // 找到 '│' 字符后，跳过它和后面的空格
                            let after_pipe = pipe_char_pos.0 + '│'.len_utf8();
                            if after_pipe + 1 < display_line.len() {
                                &display_line[after_pipe + 1..] // 跳过 '│' 和一个空格
                            } else {
                                ""
                            }
                        } else {
                            display_line // 如果没有找到 '│'，使用整行
                        };

                        // 对于简化的高亮，我们高亮整行代码（去掉前导空格）
                        let trimmed_code = code_part.trim_start();
                        let leading_spaces = code_part.chars().count() - trimmed_code.chars().count();

                        // 计算高亮的起始和结束位置（使用字符计数）
                        let start_col = leading_spaces; // 从代码开始处高亮
                        let end_col = code_part.chars().count(); // 高亮到行尾

                        // 计算X位置
                        let start_x = text_rect.min.x + prefix_width + (start_col as f32 * char_width);
                        let end_x = text_rect.min.x + prefix_width + (end_col as f32 * char_width);

                        // 创建高亮矩形
                        let highlight_rect = egui::Rect::from_min_size(
                            egui::pos2(start_x, y_pos),
                            egui::vec2((end_x - start_x).max(char_width * 2.0), line_height)
                        );

                        println!("📍 Line {}: y={:.1}, prefix_width={:.1}, start_x={:.1}, end_x={:.1}",
                            display_line_num + 1, y_pos, prefix_width, start_x, end_x);
                        println!("📝 Display line: '{}'", display_line);
                        println!("🔍 Code part: '{}', leading_spaces: {}, start_col: {}, end_col: {}",
                            code_part, leading_spaces, start_col, end_col);

                        // 确保高亮区域在文本区域内
                        if highlight_rect.intersects(text_rect) {
                            let clipped_rect = highlight_rect.intersect(text_rect);

                            // 绘制高亮背景
                            ui.painter().rect_filled(
                                clipped_rect,
                                egui::Rounding::same(2.0),
                                highlight.color.gamma_multiply(0.4)
                            );

                            // 绘制边框
                            ui.painter().rect_stroke(
                                clipped_rect,
                                egui::Rounding::same(2.0),
                                egui::Stroke::new(1.5, highlight.color)
                            );

                            println!("✅ Drew highlight rect: {:?}", clipped_rect);
                        } else {
                            println!("❌ Highlight rect does not intersect with text rect");
                        }
                    } else {
                        println!("❌ No pipe separator found in line: '{}'", display_line);
                    }
                } else {
                    println!("❌ Display line {} out of bounds (max: {})", display_line_num, display_lines.len());
                }
            }
        }
    }

    fn draw_highlights(&self, ui: &mut egui::Ui) {
        // Draw highlight backgrounds for analysis results
        let font_id = egui::FontId::monospace(self.font_size);
        let line_height = ui.fonts(|f| f.row_height(&font_id));
        let lines: Vec<&str> = self.content.lines().collect();

        for highlight in &self.highlighted_ranges {
            // Calculate the position for each highlighted line
            for line_num in highlight.start_line..=highlight.end_line.min(lines.len().saturating_sub(1)) {
                let y_offset = line_num as f32 * line_height;

                // Calculate the highlight rectangle for this line
                let start_x = if line_num == highlight.start_line {
                    // For the first line, start from the start column
                    self.calculate_text_width(&lines[line_num][..highlight.start_col.min(lines[line_num].len())], ui, &font_id)
                } else {
                    0.0 // For continuation lines, start from the beginning
                };

                let end_x = if line_num == highlight.end_line {
                    // For the last line, end at the end column
                    self.calculate_text_width(&lines[line_num][..highlight.end_col.min(lines[line_num].len())], ui, &font_id)
                } else {
                    // For continuation lines, highlight the entire line
                    self.calculate_text_width(lines[line_num], ui, &font_id)
                };

                let rect = egui::Rect::from_min_size(
                    egui::pos2(ui.min_rect().min.x + start_x, ui.min_rect().min.y + y_offset),
                    egui::vec2(end_x - start_x, line_height)
                );

                // Draw the highlight background
                ui.painter().rect_filled(
                    rect,
                    egui::Rounding::same(2.0),
                    highlight.color.gamma_multiply(0.3)
                );

                // Draw a border for better visibility
                ui.painter().rect_stroke(
                    rect,
                    egui::Rounding::same(2.0),
                    egui::Stroke::new(1.0, highlight.color)
                );
            }
        }
    }

    fn calculate_text_width(&self, text: &str, ui: &egui::Ui, font_id: &egui::FontId) -> f32 {
        // Calculate the width of text using the specified monospace font
        ui.fonts(|fonts| fonts.glyph_width(font_id, ' ')) * text.len() as f32
    }

    /// 绘制行号（与代码完美对齐）
    fn draw_line_numbers(&self, ui: &mut egui::Ui, font_id: &egui::FontId, scroll_offset: egui::Vec2) {
        let line_height = ui.fonts(|f| f.row_height(font_id));
        let lines: Vec<&str> = self.content.lines().collect();

        // 计算可见行的范围
        let visible_start = (-scroll_offset.y / line_height).floor().max(0.0) as usize;
        let visible_end = ((-scroll_offset.y + ui.available_height()) / line_height).ceil() as usize;
        let visible_end = visible_end.min(lines.len());

        // 绘制可见范围内的行号
        for line_num in visible_start..visible_end {
            let y_pos = line_num as f32 * line_height + scroll_offset.y;

            if y_pos >= -line_height && y_pos <= ui.available_height() {
                let line_rect = egui::Rect::from_min_size(
                    egui::pos2(ui.min_rect().min.x, ui.min_rect().min.y + y_pos),
                    egui::vec2(ui.available_width(), line_height)
                );

                ui.allocate_ui_at_rect(line_rect, |ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(format!("{:>3}", line_num + 1))
                                .font(font_id.clone())
                                .color(egui::Color32::from_gray(128))
                        );
                    });
                });
            }
        }
    }

    /// 绘制高亮（与新的布局兼容）
    fn draw_highlights_custom(&self, ui: &mut egui::Ui, font_id: &egui::FontId, scroll_offset: egui::Vec2) {
        let line_height = ui.fonts(|f| f.row_height(font_id));
        let lines: Vec<&str> = self.content.lines().collect();
        let char_width = ui.fonts(|f| f.glyph_width(font_id, ' '));

        for highlight in &self.highlighted_ranges {
            // 计算高亮区域的位置
            for line_num in highlight.start_line..=highlight.end_line.min(lines.len().saturating_sub(1)) {
                let y_pos = line_num as f32 * line_height + scroll_offset.y;

                // 只绘制可见的高亮
                if y_pos >= -line_height && y_pos <= ui.available_height() {
                    let start_x = if line_num == highlight.start_line {
                        highlight.start_col as f32 * char_width
                    } else {
                        0.0
                    };

                    let end_x = if line_num == highlight.end_line {
                        highlight.end_col as f32 * char_width
                    } else {
                        lines[line_num].len() as f32 * char_width
                    };

                    let highlight_rect = egui::Rect::from_min_size(
                        egui::pos2(ui.min_rect().min.x + start_x, ui.min_rect().min.y + y_pos),
                        egui::vec2(end_x - start_x, line_height)
                    );

                    // 绘制高亮背景
                    ui.painter().rect_filled(
                        highlight_rect,
                        egui::Rounding::same(2.0),
                        highlight.color.gamma_multiply(0.3)
                    );

                    // 绘制边框
                    ui.painter().rect_stroke(
                        highlight_rect,
                        egui::Rounding::same(2.0),
                        egui::Stroke::new(1.0, highlight.color)
                    );
                }
            }
        }
    }

    fn open_file(&mut self) {
        // Use rfd to open file dialog
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Source Files", &["java", "js", "py", "php", "sql", "xml"])
            .add_filter("All Files", &["*"])
            .pick_file()
        {
            if let Ok(content) = std::fs::read_to_string(&path) {
                self.content = content;

                // Detect language from file extension
                if let Some(extension) = path.extension().and_then(|e| e.to_str()) {
                    match extension {
                        "java" => self.language = "java".to_string(),
                        "js" | "jsx" => self.language = "javascript".to_string(),
                        "py" => self.language = "python".to_string(),
                        "sql" => self.language = "sql".to_string(),
                        "xml" => self.language = "xml".to_string(),
                        "php" => self.language = "php".to_string(),
                        _ => {}
                    }
                }
            }
        }
    }

    fn save_file(&self) {
        // Use rfd to save file dialog
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Source Files", &["java", "js", "py", "php", "sql", "xml"])
            .add_filter("All Files", &["*"])
            .save_file()
        {
            if let Err(e) = std::fs::write(&path, &self.content) {
                eprintln!("Failed to save file: {}", e);
            }
        }
    }

    /// Get the current line and column of the cursor
    pub fn get_cursor_position(&self) -> (usize, usize) {
        let mut line = 1;
        let mut col = 1;

        for (i, ch) in self.content.char_indices() {
            if i >= self.cursor_pos {
                break;
            }

            if ch == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }

        (line, col)
    }

    /// Highlight a specific range in the code
    pub fn highlight_range(&mut self, start_line: usize, start_col: usize, end_line: usize, end_col: usize, color: egui::Color32, message: String) {
        self.highlighted_ranges.push(HighlightRange {
            start_line,
            start_col,
            end_line,
            end_col,
            color,
            message,
        });
    }
}
