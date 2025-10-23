//! astgrep GUI Playground
//! 
//! A graphical user interface for the astgrep static code analysis tool,
//! providing an interactive playground for testing rules and analyzing code.

use egui;
use anyhow::Result;

mod app;
mod components;
mod utils;

use app::CrGuiApp;

fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Configure native options
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 900.0])
            .with_min_inner_size([800.0, 600.0])
            .with_icon(
                // Add an icon if available
                eframe::icon_data::from_png_bytes(&[])
                    .unwrap_or_default()
            ),
        ..Default::default()
    };

    // Run the application
    eframe::run_native(
        "astgrep Playground",
        options,
        Box::new(|cc| {
            // Configure egui style
            configure_style(&cc.egui_ctx);
            
            Box::new(CrGuiApp::new(cc))
        }),
    )
    .map_err(|e| anyhow::anyhow!("Failed to run GUI application: {}", e))
}

fn configure_style(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();

    // Configure fonts with Chinese support
    let mut fonts = egui::FontDefinitions::default();

    // Try to load system fonts for Chinese support
    // On macOS, try to load PingFang, Hiragino Sans GB, or STHeiti
    #[cfg(target_os = "macos")]
    {
        let font_paths = [
            ("/System/Library/Fonts/PingFang.ttc", "PingFang"),
            ("/System/Library/Fonts/Hiragino Sans GB.ttc", "Hiragino Sans GB"),
            ("/System/Library/Fonts/STHeiti Medium.ttc", "STHeiti"),
        ];

        for (path, name) in &font_paths {
            if let Ok(font_data) = std::fs::read(path) {
                fonts.font_data.insert(
                    name.to_string(),
                    egui::FontData::from_owned(font_data),
                );

                // Insert at the beginning of the proportional font list
                fonts.families
                    .entry(egui::FontFamily::Proportional)
                    .or_default()
                    .insert(0, name.to_string());

                fonts.families
                    .entry(egui::FontFamily::Monospace)
                    .or_default()
                    .insert(0, name.to_string());

                tracing::info!("Loaded Chinese font: {} from {}", name, path);
                break; // Use the first available font
            }
        }
    }

    // On Windows, try to load Microsoft YaHei
    #[cfg(target_os = "windows")]
    {
        if let Ok(font_data) = std::fs::read("C:\\Windows\\Fonts\\msyh.ttc") {
            fonts.font_data.insert(
                "Microsoft YaHei".to_owned(),
                egui::FontData::from_owned(font_data),
            );

            fonts.families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "Microsoft YaHei".to_owned());

            fonts.families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .insert(0, "Microsoft YaHei".to_owned());
        }
    }

    // On Linux, try to load Noto Sans CJK SC
    #[cfg(target_os = "linux")]
    {
        let possible_paths = [
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
        ];

        for path in &possible_paths {
            if let Ok(font_data) = std::fs::read(path) {
                fonts.font_data.insert(
                    "Noto Sans CJK".to_owned(),
                    egui::FontData::from_owned(font_data),
                );

                fonts.families
                    .entry(egui::FontFamily::Proportional)
                    .or_default()
                    .insert(0, "Noto Sans CJK".to_owned());

                fonts.families
                    .entry(egui::FontFamily::Monospace)
                    .or_default()
                    .insert(0, "Noto Sans CJK".to_owned());
                break;
            }
        }
    }

    ctx.set_fonts(fonts);

    // Configure spacing and sizing
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(10.0, 5.0);
    style.spacing.menu_margin = egui::style::Margin::same(6.0);

    ctx.set_style(style);
}
