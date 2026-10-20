// Drawing: top bar, mixer lanes, tab score, transport bar and the palette and help overlays.
use crate::app::{Action, App, Status, View};
use crate::score::{Measure, Score};
use crate::widgets::*;
use eframe::egui::{self, Align, Align2, Color32, Id, Layout, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, UiBuilder, Vec2};

const HEADER_W: f32 = 248.0;
const CHORD_H: f32 = 40.0;
const GAP: f32 = 6.0;
// The full transport and top bar need about 1280 pt; below that they switch to the compact layout, which
// needs about 990 pt (main.rs sets the window minimum from it).
const COMPACT_BELOW: f32 = 1300.0;

fn compact(ctx: &egui::Context) -> bool {
    ctx.screen_rect().width() < COMPACT_BELOW
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_keys(ctx);
        self.tick();
        let mut actions = vec![];

        egui::TopBottomPanel::top("top")
            .exact_height(56.0)
            .frame(egui::Frame::new().fill(GROUND).inner_margin(egui::Margin::symmetric(20, 0)))
            .show(ctx, |ui| self.top_bar(ui, &mut actions));
        if self.ready() {
            egui::TopBottomPanel::bottom("transport")
                .exact_height(84.0)
                .frame(egui::Frame::new().fill(PANEL).inner_margin(egui::Margin::symmetric(20, 0)))
                .show(ctx, |ui| self.transport(ui, &mut actions));
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(GROUND).inner_margin(egui::Margin { left: 16, right: 16, top: 8, bottom: 12 }))
            .show(ctx, |ui| match &self.status {
                Status::Ready => {
                    self.banner(ui);
                    match self.view {
                        View::Mixer => self.mixer(ui, &mut actions),
                        View::Tab => self.tab_view(ui, &mut actions),
                    }
                }
                _ => self.empty_state(ui),
            });
        if self.palette.is_some() {
            self.palette_overlay(ctx, &mut actions);
        }
        if self.help {
            self.help_overlay(ctx);
        }
        for action in actions {
            self.run(action);
        }
        let playing = self.player.lock().unwrap().playing;
        if playing || self.busy() {
            ctx.request_repaint();
        }
    }
}

impl App {
    // Title and BPM leading, the Mixer/Tab switch in the centre, open, palette and shortcuts trailing.
    // Below the compact breakpoint the trailing buttons use short labels and the title truncates.
    fn top_bar(&mut self, ui: &mut Ui, actions: &mut Vec<Action>) {
        let rect = ui.max_rect();
        let compact = compact(ui.ctx());
        let switch = Rect::from_center_size(rect.center(), Vec2::new(176.0, 30.0));
        let leading = Rect::from_min_max(rect.min, Pos2::new(switch.left() - 12.0, rect.bottom()));
        ui.scope_builder(UiBuilder::new().max_rect(leading).layout(Layout::left_to_right(Align::Center)), |ui| {
            let bpm = self.tab.analysis.as_ref().filter(|_| self.ready()).map(|a| format!("{} BPM", a.bpm.round()));
            let bpm_w = bpm.as_deref().map_or(0.0, |b| text_width(ui, b, mono(12.0)) + 10.0);
            let title_w = (leading.width() - bpm_w).max(40.0);
            ui.allocate_ui(Vec2::new(title_w, 30.0), |ui| {
                ui.add(egui::Label::new(egui::RichText::new(self.title.as_deref().unwrap_or("Urdr")).font(bold(15.0)).color(TEXT)).truncate());
            });
            if let Some(bpm) = bpm {
                ui.label(egui::RichText::new(bpm).font(mono(12.0)).color(SECONDARY));
            }
        });
        let trailing = Rect::from_min_max(Pos2::new(switch.right() + 12.0, rect.top()), rect.max);
        ui.scope_builder(UiBuilder::new().max_rect(trailing).layout(Layout::right_to_left(Align::Center)), |ui| {
            let (help, palette, folder, audio) = if compact { ("", "", "Folder", "Audio") } else { ("Shortcuts", "Commands", "Open folder", "Open audio") };
            if pill(ui, help, "?", self.help).on_hover_text("Show shortcuts (?)").clicked() {
                actions.push(Action::Help);
            }
            if pill(ui, palette, "Ctrl+K", false).on_hover_text("Command palette (Ctrl+K)").clicked() {
                actions.push(Action::Palette);
            }
            if pill(ui, folder, "Shift+O", false).on_hover_text("Open song folder (Shift+O)").clicked() {
                actions.push(Action::OpenFolder);
            }
            if pill(ui, audio, "O", false).on_hover_text("Open audio file (O)").clicked() {
                actions.push(Action::OpenAudio);
            }
        });
        if self.ready() {
            ui.scope_builder(UiBuilder::new().max_rect(switch).layout(Layout::left_to_right(Align::Center)), |ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                if pill(ui, "Mixer", "", self.view == View::Mixer).clicked() && self.view != View::Mixer {
                    actions.push(Action::SwitchView);
                }
                if pill(ui, "Tab", "", self.view == View::Tab).clicked() && self.view != View::Tab {
                    actions.push(Action::SwitchView);
                }
                let (r, _) = ui.allocate_exact_size(Vec2::new(chip_width(ui, "V"), 30.0), Sense::hover());
                chip(ui, r.left_center(), "V");
            });
        }
    }

    fn banner(&mut self, ui: &mut Ui) {
        let mut lines = vec![];
        if self.analyzing {
            lines.push(("Finding chords and beats...".to_string(), SECONDARY));
        } else if let Some(e) = &self.analysis_error {
            lines.push((format!("No chords: {e}."), FAILURE));
        }
        if let Some(e) = &self.audio_error {
            lines.push((format!("Audio output unavailable ({e}); playback runs silently."), SECONDARY));
        }
        if let Some(n) = &self.notice {
            lines.push((n.clone(), SECONDARY));
        }
        for (line, color) in lines {
            ui.label(egui::RichText::new(line).font(font(13.0)).color(color));
        }
        ui.add_space(4.0);
    }

    fn empty_state(&mut self, ui: &mut Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() * 0.33);
            match &self.status {
                Status::Separating(f) => {
                    ui.label(egui::RichText::new("Separating stems").font(bold(20.0)).color(TEXT));
                    ui.add_space(10.0);
                    let (rect, _) = ui.allocate_exact_size(Vec2::new(360.0, 6.0), Sense::hover());
                    ui.painter().rect_filled(rect, 3.0, RAISED);
                    ui.painter().rect_filled(Rect::from_min_size(rect.min, Vec2::new(rect.width() * *f as f32, 6.0)), 3.0, WHITE);
                }
                Status::Failed(message) => {
                    ui.label(egui::RichText::new("Could not open the song").font(bold(20.0)).color(TEXT));
                    ui.add_space(8.0);
                    ui.add(egui::Label::new(egui::RichText::new(message).font(font(14.0)).color(FAILURE)).wrap());
                }
                _ => {
                    ui.label(egui::RichText::new("Open a song to start practising").font(bold(22.0)).color(TEXT));
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new("Press O for an audio file, Shift+O for a song folder made on the Mac, Ctrl+K for every command").font(font(14.0)).color(SECONDARY));
                }
            }
            if let Some(n) = &self.notice {
                ui.add_space(8.0);
                ui.label(egui::RichText::new(n).font(font(13.0)).color(SECONDARY));
            }
        });
    }

    // Lanes share the height above a chord lane that closes the stack; one x scale for everything time-based.
    fn mixer(&mut self, ui: &mut Ui, actions: &mut Vec<Action>) {
        let area = ui.available_rect_before_wrap();
        let mut p = self.player.lock().unwrap();
        let count = p.stems.len().max(1) as f32;
        let lane_h = ((area.height() - CHORD_H - GAP * count) / count).clamp(52.0, 128.0);
        let x0 = area.left() + HEADER_W + GAP;
        let x1 = area.right();
        let duration = p.duration().max(0.001);
        let x_of = |t: f64| x0 + (t / duration) as f32 * (x1 - x0);
        let t_of = |x: f32| (((x - x0) / (x1 - x0)) as f64 * duration).clamp(0.0, duration);
        let position = p.position();
        let painter = ui.painter().clone();

        let gains = crate::audio::gains(&p.stems);
        let mut y = area.top();
        let mut volume_changes = vec![];
        for (i, stem) in p.stems.iter().enumerate() {
            let hue = stem_color(&stem.name);
            let audible = gains[i] > 0.0;
            let head = Rect::from_min_size(Pos2::new(area.left(), y), Vec2::new(HEADER_W, lane_h));
            let lane = Rect::from_min_max(Pos2::new(x0, y), Pos2::new(x1, y + lane_h));
            for r in [head, lane] {
                painter.rect_filled(r, 12.0, PANEL);
                if stem.solo {
                    painter.rect_filled(r, 12.0, hue.gamma_multiply(0.12));
                    painter.rect_stroke(r, 12.0, Stroke::new(1.0_f32, hue), StrokeKind::Inside);
                }
            }
            let icon = Rect::from_min_size(head.min + Vec2::new(12.0, 10.0), Vec2::splat(30.0));
            stem_icon(ui, icon, &stem.name, audible);
            let mut name = stem.name.clone();
            if let Some(first) = name.get_mut(0..1) {
                first.make_ascii_uppercase();
            }
            painter.text(Pos2::new(icon.right() + 10.0, icon.center().y), Align2::LEFT_CENTER, &name, bold(14.0), if audible { TEXT } else { SECONDARY });
            let name_w = text_width(ui, &name, bold(14.0));
            chip(ui, Pos2::new(icon.right() + 18.0 + name_w, icon.center().y), &(i + 1).to_string());
            let m = Rect::from_min_size(Pos2::new(head.right() - 66.0, icon.center().y - 11.0), Vec2::new(24.0, 22.0));
            let s = m.translate(Vec2::new(30.0, 0.0));
            let (mf, mc) = if stem.muted { (WHITE, Color32::BLACK) } else { (RAISED, SECONDARY) };
            if square(ui, m, Id::new(("mute", i)), "M", mf, mc, 6.0).clicked() {
                actions.push(Action::Mute(i));
            }
            let (sf, sc) = if stem.solo { (hue, Color32::BLACK) } else { (RAISED, SECONDARY) };
            if square(ui, s, Id::new(("solo", i)), "S", sf, sc, 6.0).clicked() {
                actions.push(Action::Solo(i));
            }
            if lane_h >= 70.0 {
                let slider = Rect::from_min_max(Pos2::new(head.left() + 14.0, head.bottom() - 26.0), Pos2::new(head.right() - 16.0, head.bottom() - 10.0));
                let level = if audible { hue } else { SECONDARY.gamma_multiply(0.6) };
                if let Some(v) = volume_slider(ui, slider, Id::new(("volume", i)), stem.volume, level) {
                    volume_changes.push((i, v));
                }
            }

            let mid = lane.center().y;
            let half = lane_h / 2.0 - 8.0;
            let mut x = lane.left() + 4.0;
            let buckets = stem.peaks.len() as f32;
            while x < lane.right() - 4.0 {
                let from = ((x - x0) / (x1 - x0) * buckets) as usize;
                let to = (((x + 3.0 - x0) / (x1 - x0) * buckets) as usize).max(from + 1).min(stem.peaks.len());
                let peak = stem.peaks[from.min(stem.peaks.len() - 1)..to].iter().copied().fold(0.0, f32::max);
                let h = (peak.sqrt() * half).max(1.0);
                let played = t_of(x) <= position;
                // Full when played, 38 % ahead of the playhead, and dim for a lane that cannot be heard.
                let alpha = match (audible, played) {
                    (false, true) => 0.3,
                    (false, false) => 0.14,
                    (true, true) => 1.0,
                    (true, false) => 0.38,
                };
                painter.rect_filled(Rect::from_min_max(Pos2::new(x, mid - h), Pos2::new(x + 2.0, mid + h)), 1.0, hue.gamma_multiply(alpha));
                x += 3.0;
            }
            y += lane_h + GAP;
        }
        for (i, v) in volume_changes {
            p.stems[i].volume = v;
        }

        // Chord lane on the same timeline as the stems.
        let chord_head = Rect::from_min_size(Pos2::new(area.left(), y), Vec2::new(HEADER_W, CHORD_H));
        painter.rect_filled(chord_head, 12.0, PANEL);
        painter.text(chord_head.left_center() + Vec2::new(14.0, 0.0), Align2::LEFT_CENTER, "Chords", bold(14.0), TEXT);
        let chord_lane = Rect::from_min_max(Pos2::new(x0, y), Pos2::new(x1, y + CHORD_H));
        painter.rect_filled(chord_lane, 12.0, PANEL);
        if let Some(a) = &self.tab.analysis {
            let current = a.chord_index(position);
            let lane = painter.with_clip_rect(chord_lane.shrink(2.0));
            for (i, chord) in a.chords.iter().enumerate().filter(|(_, c)| c.label != "N") {
                let r = Rect::from_min_max(Pos2::new(x_of(chord.start) + 1.0, chord_lane.top() + 5.0), Pos2::new(x_of(chord.end) - 1.0, chord_lane.bottom() - 13.0));
                let now = current == Some(i);
                lane.rect_filled(r, 6.0, if now { WHITE } else { RAISED });
                lane.with_clip_rect(r.intersect(chord_lane)).text(r.left_center() + Vec2::new(6.0, 0.0), Align2::LEFT_CENTER, chord.name(), bold(13.0), if now { Color32::BLACK } else { SECONDARY });
            }
            for &b in &a.beats {
                let strong = a.downbeats.contains(&b);
                let x = x_of(b);
                let h = if strong { 7.0 } else { 3.0 };
                lane.line_segment([Pos2::new(x, chord_lane.bottom() - 3.0 - h), Pos2::new(x, chord_lane.bottom() - 3.0)], Stroke::new(1.0_f32, white(if strong { 70 } else { 30 })));
            }
        } else {
            painter.text(chord_lane.left_center() + Vec2::new(12.0, 0.0), Align2::LEFT_CENTER, if self.analyzing { "Finding chords..." } else { "No chord analysis" }, font(13.0), SECONDARY);
        }
        let bottom = chord_lane.bottom();

        // One loop region and playhead across every stem lane and the chord lane; click seeks, drag sets the loop.
        let timeline = Rect::from_min_max(Pos2::new(x0, area.top()), Pos2::new(x1, bottom));
        let response = ui.interact(timeline, Id::new("timeline"), Sense::click_and_drag());
        if response.drag_started() {
            self.drag_from = response.interact_pointer_pos().map(|pos| t_of(pos.x));
        }
        let live = match (self.drag_from, response.interact_pointer_pos()) {
            (Some(a), Some(pos)) if response.dragged() => Some((a, t_of(pos.x))),
            _ => None,
        };
        if response.drag_stopped() {
            if let (Some(a), Some(pos)) = (self.drag_from.take(), response.interact_pointer_pos()) {
                let b = t_of(pos.x);
                if (a - b).abs() > 0.05 {
                    p.set_loop(a, b);
                }
            }
        }
        if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                p.seek(t_of(pos.x));
            }
        }
        let region = live.or(match (p.loop_start, p.loop_end) {
            (Some(a), Some(b)) => Some((a, b)),
            (Some(a), None) => Some((a, a)),
            _ => None,
        });
        if let Some((a, b)) = region {
            let (a, b) = (a.min(b), a.max(b));
            let r = Rect::from_min_max(Pos2::new(x_of(a), timeline.top()), Pos2::new(x_of(b), timeline.bottom()));
            let on = p.loop_on || live.is_some();
            painter.rect_filled(r, 0.0, MINT.gamma_multiply(if on { 0.14 } else { 0.06 }));
            let edge = Stroke::new(1.5_f32, MINT.gamma_multiply(if on { 0.7 } else { 0.3 }));
            painter.line_segment([r.left_top(), r.left_bottom()], edge);
            painter.line_segment([r.right_top(), r.right_bottom()], edge);
        }
        // 2 pt mint line with a 10 pt knob, both inside the lane bounds.
        let px = x_of(position).clamp(timeline.left() + 5.0, timeline.right() - 5.0);
        painter.line_segment([Pos2::new(px, timeline.top() + 5.0), Pos2::new(px, timeline.bottom())], Stroke::new(2.0_f32, MINT));
        painter.circle_filled(Pos2::new(px, timeline.top() + 5.0), 5.0, MINT);
        if let Some(pos) = response.hover_pos() {
            painter.text(Pos2::new(pos.x + 8.0, timeline.top() + 4.0), Align2::LEFT_TOP, format_time(t_of(pos.x)), mono(11.0), SECONDARY);
        }
        ui.advance_cursor_after_rect(Rect::from_min_max(area.min, Pos2::new(x1, bottom)));
    }

    fn tab_view(&mut self, ui: &mut Ui, actions: &mut Vec<Action>) {
        let (position, synth_on, tone) = {
            let p = self.player.lock().unwrap();
            (p.position(), p.synth_on, p.tone_name())
        };
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new("Guitar tab").font(bold(18.0)).color(TEXT));
            ui.add_space(12.0);
            if self.tab.score.is_some() && pill(ui, if self.tab.editing { "Editing" } else { "Edit" }, "E", self.tab.editing).clicked() {
                actions.push(Action::ToggleEdit);
            }
            if pill(ui, if self.tab.transcribing { "Transcribing..." } else { "Generate tab" }, "T", false).clicked() {
                actions.push(Action::GenerateTab);
            }
            if self.tab.edited {
                ui.label(egui::RichText::new("Edited by hand").font(font(13.0)).color(SECONDARY));
            }
            if synth_on {
                ui.label(egui::RichText::new(format!("Synth: {tone}")).font(font(13.0)).color(SECONDARY));
            }
            if self.tab.editing {
                ui.label(egui::RichText::new("Digits type a fret, Shift+Up/Down moves the note to another string, Delete removes, Ctrl+Z undoes, Esc stops").font(font(12.0)).color(SECONDARY));
            }
        });
        // Save and transcription errors sit right above the score.
        if let Some(e) = &self.tab.error {
            ui.label(egui::RichText::new(e).font(font(13.0)).color(FAILURE));
        }
        ui.add_space(6.0);
        let frame = egui::Frame::new().fill(PANEL).corner_radius(12.0).inner_margin(egui::Margin::same(10));
        frame.show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            if let Some(score) = self.tab.score.clone() {
                self.score_view(ui, &score, position);
            } else if !self.tab.notes.is_empty() {
                scrolling_tab(ui, &self.tab.notes, position);
            } else {
                ui.label(egui::RichText::new("Press T to transcribe the guitar into tab").font(font(14.0)).color(SECONDARY));
            }
        });
    }

    // Songsterr-style tab: rows of measures with chords above, rhythm below and a cursor that follows playback.
    fn score_view(&mut self, ui: &mut Ui, score: &Score, position: f64) {
        // Leave room for the scroll bar.
        let width = ui.available_width() - 14.0;
        let per_row = (((width - LABEL_W) / MIN_MEASURE_W) as usize).max(1);
        let measure_w = (width - LABEL_W) / per_row as f32;
        let selection = if self.tab.editing { self.tab.selection } else { None };
        let current_row = selection.map(|s| s.measure).or_else(|| score.measure_index(position)).map(|m| m / per_row);
        let scroll_to = if current_row != self.last_row { current_row } else { None };
        self.last_row = current_row;
        let mut clicked = None;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            for (row_index, row) in score.measures.chunks(per_row).enumerate() {
                let (rect, response) = ui.allocate_exact_size(Vec2::new(width, ROW_H), Sense::click());
                if scroll_to == Some(row_index) {
                    ui.scroll_to_rect(rect, Some(Align::Center));
                }
                if ui.is_rect_visible(rect) {
                    let painter = ui.painter_at(rect);
                    draw_row(ui, &painter, rect, row, measure_w, score.beats_per_bar(), position, selection);
                }
                if response.clicked() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        clicked = row_click(rect, row, measure_w, pos, self.tab.editing);
                    }
                }
            }
        });
        match clicked {
            Some(RowClick::Seek(t)) => self.player.lock().unwrap().seek(t),
            Some(RowClick::Select(s)) => {
                self.tab.selection = Some(s);
                self.tab.pending_digit = None;
            }
            None => {}
        }
    }

    // Three columns: chord left, play on the window's centre line, practice controls right. Below the compact
    // breakpoint the captions and skip labels go (tooltips keep the keys) and the controls narrow.
    fn transport(&mut self, ui: &mut Ui, actions: &mut Vec<Action>) {
        let rect = ui.max_rect();
        let compact = compact(ui.ctx());
        let center = Pos2::new(ui.ctx().screen_rect().center().x, rect.center().y);
        let p = self.player.lock().unwrap();
        let position = p.position();

        // Play centred, skips beside it, time to the left.
        let play = Rect::from_center_size(center, Vec2::splat(50.0));
        ui.scope_builder(UiBuilder::new().max_rect(play), |ui| {
            if play_button(ui, p.playing).on_hover_text("Play / pause (Space)").clicked() {
                actions.push(Action::PlayPause);
            }
        });
        ui.painter().text(play.center_top() - Vec2::new(0.0, 2.0), Align2::CENTER_BOTTOM, "Space", chip_font(), white(70));
        let time = format!("{} / {}", format_time(position), format_time(p.duration()));
        let time_w = text_width(ui, &time, mono(12.0));
        let back_rect;
        if compact {
            let back = Rect::from_center_size(Pos2::new(play.left() - 25.0, center.y), Vec2::splat(30.0));
            let forward = Rect::from_center_size(Pos2::new(play.right() + 25.0, center.y), Vec2::splat(30.0));
            if skip_button(ui, back, Id::new("back"), false).on_hover_text("Back 5 seconds (Left)").clicked() {
                actions.push(Action::Skip(-5));
            }
            if skip_button(ui, forward, Id::new("forward"), true).on_hover_text("Forward 5 seconds (Right)").clicked() {
                actions.push(Action::Skip(5));
            }
            back_rect = back;
        } else {
            let before = Rect::from_min_max(Pos2::new(center.x - 260.0, rect.top()), Pos2::new(play.left() - 10.0, rect.bottom()));
            let response = ui.scope_builder(UiBuilder::new().max_rect(before).layout(Layout::right_to_left(Align::Center)), |ui| {
                pill(ui, "-5s", "Left", false).on_hover_text("Back 5 seconds (Left)")
            });
            if response.inner.clicked() {
                actions.push(Action::Skip(-5));
            }
            back_rect = response.inner.rect;
            let after = Rect::from_min_max(Pos2::new(play.right() + 10.0, rect.top()), Pos2::new(center.x + 200.0, rect.bottom()));
            ui.scope_builder(UiBuilder::new().max_rect(after).layout(Layout::left_to_right(Align::Center)), |ui| {
                if pill(ui, "+5s", "Right", false).on_hover_text("Forward 5 seconds (Right)").clicked() {
                    actions.push(Action::Skip(5));
                }
            });
        }
        let time_right = back_rect.left() - 12.0;
        ui.painter().text(Pos2::new(time_right, center.y), Align2::RIGHT_CENTER, &time, mono(12.0), SECONDARY);

        // Current and next chord, clipped so a long name never runs under the time.
        if let Some(a) = &self.tab.analysis {
            let column = ui.painter().with_clip_rect(Rect::from_min_max(rect.min, Pos2::new(time_right - time_w - 12.0, rect.bottom())));
            let current = a.chord_index(position).map(|i| a.chords[i].name()).unwrap_or_else(|| "-".into());
            column.text(Pos2::new(rect.left(), rect.center().y - 9.0), Align2::LEFT_CENTER, &current, bold(if compact { 28.0 } else { 34.0 }), TEXT);
            if let Some(next) = a.next_chord(position) {
                column.text(Pos2::new(rect.left(), rect.center().y + 22.0), Align2::LEFT_CENTER, format!("then {}", next.name()), font(13.0), SECONDARY);
            }
        }

        // Steppers and toggles on the right; full width shows a caption with the keys above each group.
        let gap = if compact { 8.0 } else { 12.0 };
        let mut x = rect.right();
        let group = |ui: &mut Ui, x: &mut f32, caption: &str, keys: &str, width: f32, body: &mut dyn FnMut(&mut Ui, Rect)| {
            let r = Rect::from_min_max(Pos2::new(*x - width, rect.top() + 12.0), Pos2::new(*x, rect.bottom() - 12.0));
            if compact {
                body(ui, Rect::from_min_max(Pos2::new(r.left(), center.y - 15.0), Pos2::new(r.right(), center.y + 15.0)));
            } else {
                ui.painter().text(r.left_top(), Align2::LEFT_TOP, caption, bold(11.0), SECONDARY);
                let cw = text_width(ui, caption, bold(11.0));
                ui.painter().text(r.left_top() + Vec2::new(cw + 6.0, 0.0), Align2::LEFT_TOP, keys, chip_font(), white(90));
                body(ui, Rect::from_min_max(Pos2::new(r.left(), r.top() + 18.0), r.max));
            }
            *x -= width + gap;
        };
        let tone = if compact && p.tone_name() == "Overdriven" { "Drive" } else { p.tone_name() };
        let synth_on = p.synth_on;
        let has_tab = !self.tab.notes.is_empty();
        let (synth_w, tone_w) = if compact { (50.0, 64.0) } else { (56.0, 88.0) };
        group(ui, &mut x, "Guitar", "Y U", synth_w + 6.0 + tone_w, &mut |ui, r| {
            let a = Rect::from_min_size(r.min, Vec2::new(synth_w, 30.0));
            let b = Rect::from_min_size(r.min + Vec2::new(synth_w + 6.0, 0.0), Vec2::new(tone_w, 30.0));
            let (f, c) = if synth_on { (WHITE, Color32::BLACK) } else { (RAISED, if has_tab { TEXT } else { SECONDARY }) };
            if square(ui, a, Id::new("synth"), if synth_on { "Synth" } else { "Orig" }, f, c, 8.0).on_hover_text("Synth guitar (Y)").clicked() {
                actions.push(Action::ToggleSynth);
            }
            if square(ui, b, Id::new("tone"), tone, RAISED, TEXT, 8.0).on_hover_text("Next synth tone (U)").clicked() {
                actions.push(Action::NextTone);
            }
        });
        let loop_text = match (p.loop_start, p.loop_end) {
            (Some(a), Some(b)) if compact => format!("{}-{}", format_time(a), format_time(b)),
            (Some(a), Some(b)) => format!("{} - {}", format_time(a), format_time(b)),
            (Some(a), None) => format!("{} - ...", format_time(a)),
            _ => if compact { "Loop".into() } else { "Off".into() },
        };
        let loop_on = p.loop_on;
        let loop_w = if compact { 84.0 } else { 112.0 };
        group(ui, &mut x, "Loop", "[ ] L \\", loop_w, &mut |ui, r| {
            let a = Rect::from_min_size(r.min, Vec2::new(loop_w, 30.0));
            let (f, c) = if loop_on { (MINT, Color32::BLACK) } else { (RAISED, TEXT) };
            if square(ui, a, Id::new("loop"), &loop_text, f, c, 8.0).on_hover_text("Toggle loop (L), set start [ and end ], clear \\").clicked() {
                actions.push(Action::ToggleLoop);
            }
        });
        let (button, value_w) = if compact { (24.0, 38.0) } else { (26.0, 40.0) };
        let stepper_w = button * 2.0 + value_w;
        let stepper = |ui: &mut Ui, r: Rect, id: &str, value: String, minus: (Action, &str), plus: (Action, &str), actions: &mut Vec<Action>| {
            let a = Rect::from_min_size(r.min, Vec2::new(button, 30.0));
            let b = Rect::from_min_size(r.min + Vec2::new(button + value_w, 0.0), Vec2::new(button, 30.0));
            if square(ui, a, Id::new((id, "-")), "-", RAISED, TEXT, 8.0).on_hover_text(minus.1).clicked() {
                actions.push(minus.0);
            }
            ui.painter().text(Pos2::new(r.left() + button + value_w / 2.0, a.center().y), Align2::CENTER_CENTER, value, mono(13.0), TEXT);
            if square(ui, b, Id::new((id, "+")), "+", RAISED, TEXT, 8.0).on_hover_text(plus.1).clicked() {
                actions.push(plus.0);
            }
        };
        let semitones = p.semitones;
        let speed = (p.rate * 100.0).round() as i32;
        drop(p);
        group(ui, &mut x, "Pitch", "- = 0", stepper_w, &mut |ui, r| {
            let value = if semitones > 0 { format!("+{semitones}") } else { semitones.to_string() };
            stepper(ui, r, "pitch", value, (Action::Pitch(-1), "Pitch down (-), reset (0)"), (Action::Pitch(1), "Pitch up (=)"), actions);
        });
        group(ui, &mut x, "Speed", ", . /", stepper_w, &mut |ui, r| {
            stepper(ui, r, "speed", format!("{speed}%"), (Action::Speed(-1), "Slower (,), reset (/)"), (Action::Speed(1), "Faster (.)"), actions);
        });
    }

    fn palette_overlay(&mut self, ctx: &egui::Context, actions: &mut Vec<Action>) {
        if backdrop(ctx) {
            self.palette = None;
            return;
        }
        let matches = self.palette_matches();
        let mut close = false;
        let palette = self.palette.as_mut().unwrap();
        palette.selected = palette.selected.min(matches.len().saturating_sub(1));
        card(ctx, "palette", 440.0, |ui| {
            let edit = ui.add(
                egui::TextEdit::singleline(&mut palette.query)
                    .hint_text("Type a command")
                    .font(font(17.0))
                    .text_color(TEXT)
                    .frame(false)
                    .desired_width(f32::INFINITY),
            );
            if !palette.focused {
                edit.request_focus();
                palette.focused = true;
            }
            if edit.changed() {
                palette.selected = 0;
            }
            ui.add_space(6.0);
            ui.painter().line_segment([ui.cursor().left_top(), ui.cursor().left_top() + Vec2::new(ui.available_width(), 0.0)], Stroke::new(1.0_f32, hairline()));
            ui.add_space(6.0);
            egui::ScrollArea::vertical().max_height(420.0).show(ui, |ui| {
                for (i, command) in matches.iter().enumerate() {
                    let (rect, response) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 30.0), Sense::click());
                    let selected = i == palette.selected;
                    if selected {
                        ui.painter().rect_filled(rect, 6.0, white(20));
                        ui.scroll_to_rect(rect, None);
                    }
                    ui.painter().text(rect.left_center() + Vec2::new(10.0, 0.0), Align2::LEFT_CENTER, &command.title, font(14.0), TEXT);
                    let w = chip_width(ui, command.hint);
                    chip(ui, rect.right_center() - Vec2::new(w + 10.0, 0.0), command.hint);
                    if response.clicked() {
                        actions.push(command.action);
                        close = true;
                    }
                }
            });
        });
        if close {
            self.palette = None;
        }
    }

    fn help_overlay(&mut self, ctx: &egui::Context) {
        if backdrop(ctx) {
            self.help = false;
            return;
        }
        let commands = self.commands();
        // Two columns so every shortcut fits without scrolling.
        card(ctx, "help", 760.0, |ui| {
            ui.label(egui::RichText::new("Shortcuts").font(bold(17.0)).color(TEXT));
            ui.add_space(6.0);
            let mut rows: Vec<(String, &str)> = commands.iter().filter(|c| !c.hint.is_empty()).map(|c| (c.title.clone(), c.hint)).collect();
            rows.push(("Command palette".into(), "Ctrl+K"));
            rows.push(("Close this".into(), "Esc"));
            let half = rows.len().div_ceil(2);
            ui.columns(2, |columns| {
                for (column, chunk) in columns.iter_mut().zip(rows.chunks(half)) {
                    for (title, hint) in chunk {
                        let (rect, _) = column.allocate_exact_size(Vec2::new(column.available_width(), 23.0), Sense::hover());
                        column.painter().text(rect.left_center(), Align2::LEFT_CENTER, title, font(13.0), TEXT);
                        let w = chip_width(column, hint);
                        chip(column, rect.right_center() - Vec2::new(w + 12.0, 0.0), hint);
                    }
                }
            });
        });
    }
}

// Dim backdrop that blocks clicks to the app below; true when it was clicked, which closes the overlay.
fn backdrop(ctx: &egui::Context) -> bool {
    let screen = ctx.screen_rect();
    egui::Area::new(Id::new("backdrop"))
        .order(egui::Order::Middle)
        .fixed_pos(screen.min)
        .show(ctx, |ui| {
            let (rect, response) = ui.allocate_exact_size(screen.size(), Sense::click());
            ui.painter().rect_filled(rect, 0.0, Color32::from_black_alpha(140));
            response.clicked()
        })
        .inner
}

fn card(ctx: &egui::Context, id: &str, width: f32, body: impl FnOnce(&mut Ui)) {
    egui::Area::new(Id::new(id)).order(egui::Order::Foreground).anchor(Align2::CENTER_TOP, Vec2::new(0.0, 70.0)).show(ctx, |ui| {
        egui::Frame::new()
            .fill(RAISED)
            .stroke(Stroke::new(1.0_f32, hairline()))
            .corner_radius(14.0)
            .shadow(egui::Shadow { offset: [0, 12], blur: 30, spread: 0, color: Color32::from_black_alpha(128) })
            .inner_margin(egui::Margin::same(16))
            .show(ui, |ui| {
                ui.set_width(width);
                body(ui);
            });
    });
}

const LINE_GAP: f32 = 13.0;
const STAFF_TOP: f32 = 36.0;
const ROW_H: f32 = 140.0;
const LABEL_W: f32 = 22.0;
const MIN_MEASURE_W: f32 = 260.0;

fn padding(m: &Measure) -> f32 {
    if m.number == 1 {
        34.0
    } else {
        14.0
    }
}

fn beat_x(rect: Rect, beat: f64, m: &Measure, index: usize, width: f32) -> f32 {
    let start = rect.left() + LABEL_W + index as f32 * width + padding(m);
    start + (beat / m.beat_count() as f64) as f32 * (width - padding(m) - 10.0)
}

fn string_y(rect: Rect, string: i32) -> f32 {
    rect.top() + STAFF_TOP + (string - 1) as f32 * LINE_GAP
}

enum RowClick {
    Seek(f64),
    Select(crate::editor::Selection),
}

// While editing a click picks a grid spot; otherwise it seeks.
fn row_click(rect: Rect, row: &[Measure], width: f32, pos: Pos2, editing: bool) -> Option<RowClick> {
    let index = ((pos.x - rect.left() - LABEL_W) / width).floor();
    if index < 0.0 || index as usize >= row.len() {
        return None;
    }
    let index = index as usize;
    let m = &row[index];
    let (l, r) = (beat_x(rect, 0.0, m, index, width), beat_x(rect, m.beat_count() as f64, m, index, width));
    let count = m.beat_count() as f64;
    let beat = (((pos.x - l) / (r - l)) as f64 * count).clamp(0.0, count);
    if editing {
        let string = (((pos.y - rect.top() - STAFF_TOP) / LINE_GAP).round() as i32 + 1).clamp(1, 6);
        Some(RowClick::Select(crate::editor::Selection { measure: m.number - 1, slot: ((beat * 4.0).round() / 4.0).min(count - 0.25), string }))
    } else {
        Some(RowClick::Seek(m.time_at_beat(beat)))
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_row(ui: &Ui, p: &egui::Painter, rect: Rect, row: &[Measure], width: f32, beats_per_bar: usize, position: f64, selection: Option<crate::editor::Selection>) {
    let line = white(45);
    let bar = white(90);
    let end = rect.left() + LABEL_W + row.len() as f32 * width;
    for (i, name) in ["e", "B", "G", "D", "A", "E"].iter().enumerate() {
        let y = string_y(rect, i as i32 + 1);
        p.text(Pos2::new(rect.left() + LABEL_W / 2.0, y), Align2::CENTER_CENTER, *name, mono(10.0), SECONDARY);
        p.line_segment([Pos2::new(rect.left() + LABEL_W, y), Pos2::new(end, y)], Stroke::new(1.0_f32, line));
    }
    for (index, m) in row.iter().enumerate() {
        let left = rect.left() + LABEL_W + index as f32 * width;
        p.line_segment([Pos2::new(left, string_y(rect, 1)), Pos2::new(left, string_y(rect, 6))], Stroke::new(1.0_f32, bar));
        p.text(Pos2::new(left + 3.0, rect.top() + STAFF_TOP - 26.0), Align2::LEFT_CENTER, m.number.to_string(), font(10.5), SECONDARY);
        if m.number == 1 {
            p.text(Pos2::new(left + 16.0, string_y(rect, 2)), Align2::CENTER_CENTER, beats_per_bar.to_string(), bold(24.0), TEXT);
            p.text(Pos2::new(left + 16.0, string_y(rect, 2) + LINE_GAP * 2.5), Align2::CENTER_CENTER, "4", bold(24.0), TEXT);
        }
        for chord in &m.chords {
            p.text(Pos2::new(beat_x(rect, chord.slot, m, index, width), rect.top() + STAFF_TOP - 14.0), Align2::LEFT_CENTER, &chord.name, bold(13.5), TEXT);
        }
        if m.start() <= position && position < m.end() {
            let cx = beat_x(rect, m.beat_position(position), m, index, width);
            let capsule = Rect::from_min_size(Pos2::new(cx - 4.0, string_y(rect, 1) - 9.0), Vec2::new(22.0, LINE_GAP * 5.0 + 18.0));
            p.rect_filled(capsule, 11.0, MINT.gamma_multiply(0.14));
            p.rect_stroke(capsule, 11.0, Stroke::new(2.0_f32, MINT), StrokeKind::Middle);
        }
        for placed in &m.notes {
            let text = placed.note.fret.to_string();
            let sounding = placed.note.start <= position && position < placed.note.end;
            let nx = beat_x(rect, placed.slot, m, index, width);
            let y = string_y(rect, placed.note.string);
            let w = text_width(ui, &text, mono(12.0));
            p.rect_filled(Rect::from_min_size(Pos2::new(nx - 1.0, y - 7.0), Vec2::new(w + 2.0, 14.0)), 0.0, PANEL);
            p.text(Pos2::new(nx, y), Align2::LEFT_CENTER, text, mono(12.0), if sounding { MINT } else { TEXT });
        }
        draw_rhythm(p, rect, m, index, width);
        if let Some(sel) = selection.filter(|s| s.measure == m.number - 1) {
            let r = Rect::from_min_size(Pos2::new(beat_x(rect, sel.slot, m, index, width) - 4.0, string_y(rect, sel.string) - 8.0), Vec2::new(22.0, 16.0));
            p.rect_stroke(r, 3.0, Stroke::new(2.0_f32, MINT), StrokeKind::Middle);
        }
    }
    p.line_segment([Pos2::new(end, string_y(rect, 1)), Pos2::new(end, string_y(rect, 6))], Stroke::new(1.0_f32, bar));
}

// shortcut: beams group notes per beat and use one beam level for the whole group, not per-note subdivisions.
fn draw_rhythm(p: &egui::Painter, rect: Rect, m: &Measure, index: usize, width: f32) {
    let top = string_y(rect, 6) + 12.0;
    let bottom = top + 18.0;
    let color = SECONDARY;
    let mut slots: Vec<(f64, i32)> = vec![];
    for n in &m.notes {
        match slots.iter_mut().find(|s| s.0 == n.slot) {
            Some(s) => s.1 = s.1.min(n.sixteenths),
            None => slots.push((n.slot, n.sixteenths)),
        }
    }
    slots.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut beat = 0;
    while beat < m.beat_count() {
        let group: Vec<&(f64, i32)> = slots.iter().filter(|s| s.0 as usize == beat).collect();
        beat += 1;
        if group.is_empty() {
            continue;
        }
        let xs: Vec<f32> = group.iter().map(|s| beat_x(rect, s.0, m, index, width) + 4.0).collect();
        for &x in &xs {
            p.line_segment([Pos2::new(x, top), Pos2::new(x, bottom)], Stroke::new(1.0_f32, color));
        }
        let shortest = group.iter().map(|s| s.1).min().unwrap();
        if shortest >= 4 {
            continue;
        }
        for level in 0..(if shortest == 1 { 2 } else { 1 }) {
            let y = bottom - level as f32 * 4.0;
            if xs.len() > 1 {
                p.line_segment([Pos2::new(xs[0], y), Pos2::new(*xs.last().unwrap(), y)], Stroke::new(2.5_f32, color));
            } else {
                p.line_segment([Pos2::new(xs[0], y), Pos2::new(xs[0] + 6.0, y - 5.0)], Stroke::new(1.5_f32, color));
            }
        }
    }
}

// Six-line tab that scrolls under a fixed playhead, for a tab without a beat grid.
fn scrolling_tab(ui: &mut Ui, notes: &[crate::song::TabNote], position: f64) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 18.0 * 7.0), Sense::hover());
    let p = ui.painter_at(rect);
    let per_second = 250.0;
    let head = rect.left() + rect.width() * 0.25;
    let y = |s: i32| rect.top() + 18.0 * s as f32;
    for s in 1..=6 {
        p.line_segment([Pos2::new(rect.left(), y(s)), Pos2::new(rect.right(), y(s))], Stroke::new(1.0_f32, white(45)));
    }
    for n in notes {
        let x = head + ((n.start - position) * per_second) as f32;
        if x < rect.left() - 30.0 || x > rect.right() {
            continue;
        }
        let sounding = n.start <= position && position < n.end;
        p.rect_filled(Rect::from_min_size(Pos2::new(x - 2.0, y(n.string) - 7.0), Vec2::new(18.0, 14.0)), 0.0, PANEL);
        p.text(Pos2::new(x, y(n.string)), Align2::LEFT_CENTER, n.fret.to_string(), mono(12.0), if sounding { MINT } else { TEXT });
    }
    p.line_segment([Pos2::new(head, rect.top() + 9.0), Pos2::new(head, rect.bottom() - 9.0)], Stroke::new(2.0_f32, MINT));
}
