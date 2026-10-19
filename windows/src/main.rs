// Urdr for Windows: the Mac app's stem player, chord lane and tab editor on egui, reading the same song folders.
#![cfg_attr(windows, windows_subsystem = "windows")]

mod analysis;
mod app;
mod audio;
mod editor;
mod engine;
mod score;
mod song;
mod synth;
mod views;
mod widgets;

use eframe::egui;
use std::sync::Arc;

// Segoe UI when Windows has it; egui's bundled font otherwise (Wine, Linux).
fn fonts() -> egui::FontDefinitions {
    let mut fonts = egui::FontDefinitions::default();
    let mut proportional = fonts.families[&egui::FontFamily::Proportional].clone();
    let mut bold = proportional.clone();
    let dir = std::path::Path::new("C:\\Windows\\Fonts");
    if let (Ok(regular), Ok(semibold)) = (std::fs::read(dir.join("segoeui.ttf")), std::fs::read(dir.join("seguisb.ttf"))) {
        fonts.font_data.insert("segoe".into(), Arc::new(egui::FontData::from_owned(regular)));
        fonts.font_data.insert("segoe-semibold".into(), Arc::new(egui::FontData::from_owned(semibold)));
        proportional.insert(0, "segoe".into());
        bold.insert(0, "segoe-semibold".into());
    }
    fonts.families.insert(egui::FontFamily::Proportional, proportional);
    fonts.families.insert(egui::FontFamily::Name("bold".into()), bold);
    fonts
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_title("Urdr").with_inner_size([1280.0, 800.0]).with_min_inner_size([900.0, 560.0]),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native(
        "Urdr",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_fonts(fonts());
            let mut visuals = egui::Visuals::dark();
            visuals.panel_fill = widgets::GROUND;
            visuals.window_fill = widgets::RAISED;
            visuals.extreme_bg_color = widgets::PANEL;
            visuals.override_text_color = Some(widgets::TEXT);
            visuals.selection.bg_fill = egui::Color32::from_white_alpha(40);
            cc.egui_ctx.set_visuals(visuals);
            Ok(Box::new(app::App::new()))
        }),
    )
}
