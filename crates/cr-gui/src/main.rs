//! CR-SemService GUI Playground
//! 
//! A graphical user interface for the CR-SemService static code analysis tool,
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
        "CR-SemService Playground",
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
    
    // Configure fonts
    let fonts = egui::FontDefinitions::default();
    
    // Use built-in monospace font for now
    // In a real implementation, you could include custom fonts

    
    ctx.set_fonts(fonts);
    
    // Configure spacing and sizing
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(10.0, 5.0);
    style.spacing.menu_margin = egui::style::Margin::same(6.0);
    
    ctx.set_style(style);
}
