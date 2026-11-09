//! The automation editor for tempo: a graph of tempo points, one per bar where the
//! tempo changes. Each point either holds until the next one or changes gradually
//! towards it. Opened with F10, Edit > Automations, or by clicking a tempo.

use super::App;
use egui::{Align2, Color32, FontId, Pos2, Rect, RichText, Sense, Stroke, Vec2};

impl App {
    /// Opens the editor with the tempo point that applies at a bar selected.
    pub(super) fn open_automations(&mut self, bar: usize) {
        self.auto_open = true;
        self.auto_sel = bar.min(self.song.bar_count() - 1);
        self.auto_snap = false;
    }

    /// The tempo points: (bar, bpm, gradual change to the next point).
    fn points(&self) -> Vec<(usize, u32, bool)> {
        self.song.tempo_points()
    }

    /// Sets a point (creating it if needed), without making an undo step.
    fn set_point(&mut self, bar: usize, bpm: u32, ramp: bool) {
        let bpm = bpm.clamp(20, 400);
        if bar == 0 {
            self.song.tempo = bpm;
            self.song.master[0].tempo = None;
            self.song.master[0].tempo_ramp = ramp;
        } else if bar < self.song.bar_count() {
            self.song.master[bar].tempo = Some(bpm);
            self.song.master[bar].tempo_ramp = ramp;
        }
        self.dirty = true;
        self.total_secs = self.song_seconds();
    }

    fn remove_point(&mut self, bar: usize) {
        if bar == 0 || bar >= self.song.bar_count() {
            return;
        }
        self.song.master[bar].tempo = None;
        self.song.master[bar].tempo_ramp = false;
        self.dirty = true;
        self.total_secs = self.song_seconds();
    }

    /// One undo step per gesture.
    fn auto_begin(&mut self) {
        if !self.auto_snap {
            self.snapshot();
            self.auto_snap = true;
        }
    }

    pub(super) fn automation_window(&mut self, ctx: &egui::Context) {
        if !self.auto_open {
            return;
        }
        // A gesture ends when the mouse is released and nothing is being typed.
        if !ctx.input(|i| i.pointer.any_down()) && !ctx.egui_wants_keyboard_input() {
            self.auto_snap = false;
        }
        let mut open = true;
        let n = self.song.bar_count();
        egui::Window::new("Automations").open(&mut open).default_size(Vec2::new(820.0, 300.0)).anchor(Align2::CENTER_TOP, Vec2::new(0.0, 120.0)).resizable(false).collapsible(false).show(ctx, |ui| {
            ui.horizontal_top(|ui| {
                // ---- left panel: the selected point
                ui.vertical(|ui| {
                    ui.set_width(230.0);
                    ui.label(RichText::new("TEMPO").strong());
                    ui.label(RichText::new("Applies to every track.").small().weak());
                    ui.add_space(6.0);
                    let pts = self.points();
                    let sel = self.auto_sel.min(n - 1);
                    let existing = pts.iter().find(|p| p.0 == sel).copied();
                    let (mut bpm, mut ramp) = match existing {
                        Some((_, b, r)) => (b, r),
                        None => (self.song.tempo_at(sel), false),
                    };
                    ui.horizontal(|ui| {
                        ui.label("Bar");
                        let mut b1 = sel + 1;
                        if ui.add(egui::DragValue::new(&mut b1).range(1..=n)).changed() {
                            self.auto_sel = b1 - 1;
                        }
                        if existing.is_none() {
                            ui.label(RichText::new("(no change yet)").weak());
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Tempo");
                        if ui.add(egui::DragValue::new(&mut bpm).range(20..=400).suffix(" bpm")).changed() {
                            self.auto_begin();
                            self.set_point(sel, bpm, ramp);
                        }
                    });
                    ui.add_space(4.0);
                    ui.label("Until the next point:");
                    let before = ramp;
                    ui.radio_value(&mut ramp, false, "Constant");
                    ui.radio_value(&mut ramp, true, "Progressive (gradual change)");
                    if ramp != before {
                        self.auto_begin();
                        self.set_point(sel, bpm, ramp);
                    }
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if ui.add_enabled(sel > 0 && existing.is_some(), egui::Button::new("Delete point")).clicked() {
                            self.auto_begin();
                            self.remove_point(sel);
                        }
                        if ui.button("Remove automations").on_hover_text("Keep only the starting tempo").clicked() {
                            self.auto_begin();
                            for b in 1..n {
                                self.remove_point(b);
                            }
                            self.song.master[0].tempo_ramp = false;
                        }
                    });
                    ui.add_space(6.0);
                    ui.label(RichText::new("Click the graph to add a point; drag a point up or down to change the tempo, left or right to move it. Delete removes the selected point.").small().weak());
                });
                ui.separator();
                // ---- right: the graph
                self.tempo_graph(ui, n);
            });
        });
        if open && ctx.input(|i| i.key_pressed(egui::Key::Delete)) && !ctx.egui_wants_keyboard_input() && self.auto_sel > 0 {
            self.auto_begin();
            let b = self.auto_sel;
            self.remove_point(b);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            open = false;
        }
        self.auto_open = open;
    }

    fn tempo_graph(&mut self, ui: &mut egui::Ui, n: usize) {
        let size = Vec2::new(ui.available_width().max(520.0), 230.0);
        let (rect, resp) = ui.allocate_exact_size(size, Sense::click_and_drag());
        let p = ui.painter().clone();
        p.rect_filled(rect, 4.0, Color32::from_rgb(24, 26, 31));
        let area = Rect::from_min_max(rect.min + Vec2::new(40.0, 10.0), rect.max - Vec2::new(10.0, 22.0));
        let pts = self.points();
        let lo_v = pts.iter().map(|p| p.1).min().unwrap_or(120);
        let hi_v = pts.iter().map(|p| p.1).max().unwrap_or(120);
        let mut lo = (lo_v.saturating_sub(30) / 10 * 10).max(20) as f32;
        let mut hi = ((hi_v + 30).div_ceil(10) * 10).min(400) as f32;
        if hi - lo < 60.0 {
            lo = (lo - 20.0).max(20.0);
            hi = (lo + 80.0).min(400.0);
        }
        let x_of = |b: f32| area.left() + b / n as f32 * area.width();
        let y_of = |v: f32| area.bottom() - (v - lo) / (hi - lo) * area.height();
        let bar_of = |x: f32| (((x - area.left()) / area.width() * n as f32).round() as i32).clamp(0, n as i32 - 1) as usize;
        let val_of = |y: f32| (lo + (area.bottom() - y) / area.height() * (hi - lo)).round().clamp(20.0, 400.0) as u32;
        // Grid.
        let mut v = (lo / 20.0).ceil() * 20.0;
        while v <= hi {
            let y = y_of(v);
            p.line_segment([Pos2::new(area.left(), y), Pos2::new(area.right(), y)], Stroke::new(1.0, Color32::from_gray(45)));
            p.text(Pos2::new(area.left() - 6.0, y), Align2::RIGHT_CENTER, format!("{}", v as i32), FontId::proportional(10.0), Color32::from_gray(140));
            v += 20.0;
        }
        for b in 0..=n {
            let x = x_of(b as f32);
            p.line_segment([Pos2::new(x, area.top()), Pos2::new(x, area.bottom())], Stroke::new(1.0, Color32::from_gray(if b % 4 == 0 { 52 } else { 36 })));
            if b < n && (b % 4 == 0 || n <= 16) {
                p.text(Pos2::new(x + 3.0, area.bottom() + 10.0), Align2::LEFT_CENTER, format!("{}", b + 1), FontId::proportional(10.0), Color32::from_gray(140));
            }
        }
        // Cursor bar.
        let cx = x_of(self.bar as f32);
        p.rect_filled(Rect::from_min_max(Pos2::new(cx, area.top()), Pos2::new(x_of(self.bar as f32 + 1.0), area.bottom())), 0.0, Color32::from_rgba_unmultiplied(80, 140, 240, 30));
        // Tempo line.
        let line = Stroke::new(2.0, Color32::from_rgb(110, 200, 120));
        for (k, &(b, val, ramp)) in pts.iter().enumerate() {
            let (x0, y0) = (x_of(b as f32), y_of(val as f32));
            match pts.get(k + 1) {
                Some(&(nb, nv, _)) => {
                    let (x1, y1) = (x_of(nb as f32), y_of(nv as f32));
                    if ramp {
                        p.line_segment([Pos2::new(x0, y0), Pos2::new(x1, y1)], line);
                    } else {
                        p.line_segment([Pos2::new(x0, y0), Pos2::new(x1, y0)], line);
                        p.line_segment([Pos2::new(x1, y0), Pos2::new(x1, y1)], line);
                    }
                }
                None => {
                    p.line_segment([Pos2::new(x0, y0), Pos2::new(area.right(), y0)], line);
                }
            }
        }
        for &(b, val, _) in &pts {
            let c = Pos2::new(x_of(b as f32), y_of(val as f32));
            let sel = b == self.auto_sel;
            p.circle_filled(c, if sel { 6.0 } else { 5.0 }, if sel { Color32::from_rgb(245, 165, 50) } else { Color32::from_rgb(110, 200, 120) });
            if sel {
                p.text(c + Vec2::new(8.0, -10.0), Align2::LEFT_BOTTOM, format!("{val} bpm"), FontId::proportional(11.0), Color32::WHITE);
            }
        }
        // Interaction.
        let near = |pos: Pos2| pts.iter().find(|&&(b, v, _)| (Pos2::new(x_of(b as f32), y_of(v as f32)) - pos).length() < 9.0).map(|p| p.0);
        if resp.drag_started() {
            if let Some(pos) = resp.interact_pointer_pos() {
                match near(pos) {
                    Some(b) => {
                        self.auto_sel = b;
                        self.auto_drag = true;
                    }
                    None => self.auto_drag = false,
                }
            }
        }
        if resp.dragged() && self.auto_drag {
            if let Some(pos) = resp.interact_pointer_pos() {
                let cur = self.auto_sel;
                let ramp = pts.iter().find(|p| p.0 == cur).map(|p| p.2).unwrap_or(false);
                let val = val_of(pos.y);
                let mut to = bar_of(pos.x);
                if cur == 0 || (to != cur && (to == 0 || pts.iter().any(|p| p.0 == to))) {
                    to = cur;
                }
                self.auto_begin();
                if to != cur {
                    self.remove_point(cur);
                    self.auto_sel = to;
                }
                self.set_point(to, val, ramp);
            }
        }
        if resp.drag_stopped() {
            self.auto_drag = false;
        }
        if resp.clicked() {
            if let Some(pos) = resp.interact_pointer_pos() {
                match near(pos) {
                    Some(b) => self.auto_sel = b,
                    None => {
                        let b = bar_of(pos.x);
                        self.auto_begin();
                        if pts.iter().any(|p| p.0 == b) {
                            self.auto_sel = b;
                        } else {
                            self.set_point(b, val_of(pos.y), false);
                            self.auto_sel = b;
                        }
                    }
                }
            }
        }
    }
}
