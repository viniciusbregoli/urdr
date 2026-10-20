// The reviewed design system (DESIGN.md at the repo root) and the few painted controls the app needs.
use eframe::egui::{self, Align2, Color32, CornerRadius, FontId, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind, Ui, Vec2};

pub const GROUND: Color32 = Color32::from_rgb(14, 15, 17);
pub const PANEL: Color32 = Color32::from_rgb(23, 25, 29);
pub const RAISED: Color32 = Color32::from_rgb(33, 36, 41);
pub const TEXT: Color32 = Color32::from_rgb(242, 245, 247);
pub const SECONDARY: Color32 = Color32::from_rgb(156, 163, 176);
pub const WHITE: Color32 = Color32::WHITE;
// Mint marks playback and selection, nothing else.
pub const MINT: Color32 = Color32::from_rgb(92, 242, 209);
// Error text only.
pub const FAILURE: Color32 = Color32::from_rgb(255, 107, 102);

// White at `alpha` / 255 blended in display space. egui's from_white_alpha premultiplies in linear space,
// which makes a 9 % overlay look like 40 %.
pub fn white(alpha: u8) -> Color32 {
    Color32::from_rgba_premultiplied(alpha, alpha, alpha, alpha)
}

pub fn hairline() -> Color32 {
    white(20)
}

// One hue per stem, used only for that stem.
pub fn stem_color(name: &str) -> Color32 {
    match name {
        "vocals" => Color32::from_rgb(255, 107, 153),
        "guitar" => Color32::from_rgb(255, 181, 71),
        "drums" => Color32::from_rgb(166, 140, 250),
        "bass" => Color32::from_rgb(56, 214, 240),
        "piano" => Color32::from_rgb(110, 232, 161),
        _ => Color32::from_rgb(148, 163, 184),
    }
}

pub fn font(size: f32) -> FontId {
    FontId::proportional(size)
}

pub fn bold(size: f32) -> FontId {
    FontId::new(size, egui::FontFamily::Name("bold".into()))
}

// Key chips: a proportional semibold face (Segoe UI on Windows) so O and 0 stay distinct.
pub fn chip_font() -> FontId {
    bold(10.5)
}

pub fn mono(size: f32) -> FontId {
    FontId::monospace(size)
}

pub fn text_width(ui: &Ui, text: &str, font: FontId) -> f32 {
    ui.fonts(|f| f.layout_no_wrap(text.into(), font, TEXT).size().x)
}

// Key hint chip, like the Mac's KeyChip. Returns its width.
pub fn chip(ui: &Ui, left_center: Pos2, text: &str) -> f32 {
    if text.is_empty() {
        return 0.0;
    }
    let w = chip_width(ui, text);
    let rect = Rect::from_min_size(left_center - Vec2::new(0.0, 9.0), Vec2::new(w, 18.0));
    ui.painter().rect_filled(rect, 4.0, RAISED);
    ui.painter().rect_stroke(rect, 4.0, Stroke::new(1.0_f32, hairline()), StrokeKind::Inside);
    ui.painter().text(rect.center(), Align2::CENTER_CENTER, text, chip_font(), SECONDARY);
    w
}

pub fn chip_width(ui: &Ui, text: &str) -> f32 {
    if text.is_empty() {
        0.0
    } else {
        text_width(ui, text, chip_font()) + 10.0
    }
}

// Chrome button with an optional key chip; white when active.
pub fn pill(ui: &mut Ui, label: &str, key: &str, active: bool) -> Response {
    let label_w = text_width(ui, label, font(14.0));
    let key_w = chip_width(ui, key);
    let pad = if key.is_empty() || label.is_empty() { 24.0 } else { 32.0 };
    let size = Vec2::new(label_w + key_w + pad, 30.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let (fill, color) = if active { (WHITE, Color32::BLACK) } else if response.hovered() { (Color32::from_rgb(44, 48, 54), TEXT) } else { (RAISED, TEXT) };
    ui.painter().rect_filled(rect, 8.0, fill);
    ui.painter().text(Pos2::new(rect.left() + 12.0, rect.center().y), Align2::LEFT_CENTER, label, font(14.0), color);
    if !key.is_empty() {
        // A chip-only button (compact top bar) centres the chip.
        let at = Pos2::new(if label.is_empty() { rect.left() + 12.0 } else { rect.left() + 20.0 + label_w }, rect.center().y);
        if active {
            let r = Rect::from_min_size(at - Vec2::new(0.0, 9.0), Vec2::new(key_w, 18.0));
            ui.painter().rect_filled(r, 4.0, Color32::from_black_alpha(30));
            ui.painter().text(r.center(), Align2::CENTER_CENTER, key, chip_font(), Color32::from_gray(60));
        } else {
            chip(ui, at, key);
        }
    }
    response
}

// Fixed-size button: 8 radius for controls, 6 for the mute and solo pills.
pub fn square(ui: &mut Ui, rect: Rect, id: egui::Id, label: &str, fill: Color32, color: Color32, radius: f32) -> Response {
    let response = ui.interact(rect, id, Sense::click());
    let fill = if response.hovered() && fill == RAISED { Color32::from_rgb(44, 48, 54) } else { fill };
    ui.painter().rect_filled(rect, radius, fill);
    ui.painter().text(rect.center(), Align2::CENTER_CENTER, label, bold(13.0), color);
    response
}

// Icon-only skip button for the compact transport: two triangles pointing the way it jumps.
pub fn skip_button(ui: &mut Ui, rect: Rect, id: egui::Id, forward: bool) -> Response {
    let response = ui.interact(rect, id, Sense::click());
    ui.painter().rect_filled(rect, 8.0, if response.hovered() { Color32::from_rgb(44, 48, 54) } else { RAISED });
    let c = rect.center();
    let dir = if forward { 1.0 } else { -1.0 };
    for dx in [-4.0, 4.0] {
        let tip = c + Vec2::new(dx + dir * 4.0, 0.0);
        let back = c + Vec2::new(dx - dir * 4.0, 0.0);
        let points = vec![tip, back + Vec2::new(0.0, -5.0), back + Vec2::new(0.0, 5.0)];
        ui.painter().add(Shape::convex_polygon(points, TEXT, Stroke::NONE));
    }
    response
}

// White 48 pt round play button with a black icon.
pub fn play_button(ui: &mut Ui, playing: bool) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(50.0), Sense::click());
    let c = rect.center();
    let radius = if response.hovered() { 25.0 } else { 24.0 };
    ui.painter().circle_filled(c, radius, WHITE);
    if playing {
        for dx in [-5.0, 5.0] {
            ui.painter().rect_filled(Rect::from_center_size(c + Vec2::new(dx, 0.0), Vec2::new(5.0, 18.0)), 1.5, Color32::BLACK);
        }
    } else {
        let points = vec![c + Vec2::new(-6.0, -10.0), c + Vec2::new(11.0, 0.0), c + Vec2::new(-6.0, 10.0)];
        ui.painter().add(Shape::convex_polygon(points, Color32::BLACK, Stroke::NONE));
    }
    response
}

// Thin volume slider in the stem colour. Returns the new value while dragged.
pub fn volume_slider(ui: &mut Ui, rect: Rect, id: egui::Id, value: f32, color: Color32) -> Option<f32> {
    let response = ui.interact(rect, id, Sense::click_and_drag());
    let track = Rect::from_center_size(rect.center(), Vec2::new(rect.width(), 4.0));
    ui.painter().rect_filled(track, 2.0, white(25));
    let x = track.left() + track.width() * value;
    ui.painter().rect_filled(Rect::from_min_max(track.min, Pos2::new(x, track.max.y)), 2.0, color);
    ui.painter().circle_filled(Pos2::new(x, track.center().y), if response.hovered() || response.dragged() { 7.0 } else { 6.0 }, WHITE);
    if response.dragged() || response.clicked() {
        let pointer = response.interact_pointer_pos()?;
        return Some(((pointer.x - track.left()) / track.width()).clamp(0.0, 1.0));
    }
    None
}

fn arc(center: Pos2, radius: f32, from: f32, to: f32) -> Vec<Pos2> {
    (0..=12).map(|i| {
        let a = from + (to - from) * i as f32 / 12.0;
        center + Vec2::new(a.cos(), a.sin()) * radius
    }).collect()
}

// Small line icons per stem on a tinted disc; grey when the stem cannot be heard.
pub fn stem_icon(ui: &Ui, rect: Rect, name: &str, audible: bool) {
    let color = if audible { stem_color(name) } else { SECONDARY.gamma_multiply(0.7) };
    let p = ui.painter();
    p.circle_filled(rect.center(), rect.width() / 2.0, color.gamma_multiply(0.16));
    let c = rect.center();
    let s = Stroke::new(1.8_f32, color);
    let v = |x: f32, y: f32| c + Vec2::new(x, y);
    match name {
        "vocals" => {
            p.rect_filled(Rect::from_center_size(v(0.0, -3.0), Vec2::new(7.0, 11.0)), 3.5, color);
            p.add(Shape::line(arc(v(0.0, -1.0), 6.0, 0.0, std::f32::consts::PI), s));
            p.line_segment([v(0.0, 5.0), v(0.0, 8.0)], s);
            p.line_segment([v(-3.5, 8.0), v(3.5, 8.0)], s);
        }
        "guitar" => {
            p.circle_stroke(v(-3.0, 4.0), 4.5, s);
            p.circle_stroke(v(0.5, 0.5), 3.0, s);
            p.line_segment([v(1.5, -0.5), v(7.0, -7.0)], s);
            p.line_segment([v(5.5, -8.5), v(8.5, -5.5)], s);
        }
        "bass" => {
            p.circle_stroke(v(-3.5, 4.5), 4.0, s);
            p.line_segment([v(-1.0, 2.0), v(8.0, -8.0)], s);
            p.line_segment([v(6.0, -9.0), v(9.0, -6.0)], s);
            p.line_segment([v(-5.0, 4.5), v(-2.0, 4.5)], s);
        }
        "drums" => {
            p.rect_stroke(Rect::from_center_size(v(0.0, 3.0), Vec2::new(15.0, 8.0)), 3.0, s, StrokeKind::Middle);
            p.line_segment([v(-7.0, -8.0), v(-1.0, -1.0)], s);
            p.line_segment([v(7.0, -8.0), v(1.0, -1.0)], s);
        }
        "piano" => {
            let body = Rect::from_center_size(c, Vec2::new(16.0, 12.0));
            p.rect_stroke(body, 2.0, s, StrokeKind::Middle);
            for i in 1..4 {
                let x = body.left() + body.width() * i as f32 / 4.0;
                p.line_segment([Pos2::new(x, body.top()), Pos2::new(x, body.bottom())], Stroke::new(1.2_f32, color));
            }
            for i in [1.0, 3.0] {
                let x = body.left() + body.width() * i / 4.0;
                p.rect_filled(Rect::from_center_size(Pos2::new(x, body.top() + 3.5), Vec2::new(2.6, 7.0)), CornerRadius::ZERO, color);
            }
        }
        _ => {
            for (i, h) in [6.0, 12.0, 8.0, 4.0].iter().enumerate() {
                let x = c.x - 6.0 + i as f32 * 4.0;
                p.line_segment([Pos2::new(x, c.y - h / 2.0), Pos2::new(x, c.y + h / 2.0)], Stroke::new(2.2_f32, color));
            }
        }
    }
}

pub fn format_time(seconds: f64) -> String {
    let total = seconds.max(0.0) as i64;
    format!("{}:{:02}", total / 60, total % 60)
}
