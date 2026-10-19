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
    // The transport needs about 1280 pt: the centred play cluster reaches 125 pt right of centre and the practice
    // controls take 502 pt from the right edge. 1300 keeps a margin so they never overlap.
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_title("Urdr").with_inner_size([1360.0, 820.0]).with_min_inner_size([1300.0, 560.0]),
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
            visuals.selection.bg_fill = widgets::white(40);
            // Panel separators are the toolbar and transport hairlines.
            visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0_f32, widgets::hairline());
            cc.egui_ctx.set_visuals(visuals);
            let mut app = app::App::new(true);
            // `urdr <song>` opens an audio file or a song folder straight from the command line; flags are skipped.
            if let Some(arg) = std::env::args_os().skip(1).find(|a| !a.to_string_lossy().starts_with('-')) {
                app.open_argument(std::path::Path::new(&arg));
            }
            Ok(Box::new(app))
        }),
    )
}
