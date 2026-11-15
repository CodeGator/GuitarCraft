//! The automation editor for tempo: a graph of tempo points along the song.
//! Each point either holds until the next one or changes gradually towards
//! it. For a score the points sit on bars (opened with F10, Edit >
//! Automations, or by clicking a tempo); a chord chart uses the same editor
//! with its sections in playing order (opened by clicking its BPM).

use super::App;
use egui::{Align2, Color32, FontId, Pos2, Rect, RichText, Sense, Stroke, Ui, Vec2};

/// A tempo point: (place along the song, bpm, gradual change to the next point).
pub type Point = (usize, u32, bool);

/// What the song's places are and what to call them.
pub struct TempoAxis<'a> {
    /// "Bar" or "Section".
    pub unit: &'a str,
    /// How many places there are.
    pub n: usize,
    /// The name shown under the graph and beside the place number.
    pub label: &'a dyn Fn(usize) -> String,
    /// A place to mark on the graph (the score's cursor bar).
    pub cursor: Option<usize>,
}

/// The editor's state between frames.
#[derive(Default, Clone, Copy)]
pub struct TempoEdit {
    pub sel: usize,
    pub drag: bool,
}

fn set_point(pts: &mut Vec<Point>, at: usize, bpm: u32, ramp: bool) {
    let bpm = bpm.clamp(20, 400);
    match pts.iter_mut().find(|p| p.0 == at) {
        Some(p) => {
            p.1 = bpm;
            p.2 = ramp;
        }
        None => {
            pts.push((at, bpm, ramp));
            pts.sort_by_key(|p| p.0);
        }
    }
}

fn remove_point(pts: &mut Vec<Point>, at: usize) {
    if at > 0 {
        pts.retain(|p| p.0 != at);
    }
}

/// The tempo editor: the selected point's settings on the left and the graph
/// on the right. Returns the new points when something changed.
pub fn tempo_editor(ui: &mut Ui, ed: &mut TempoEdit, pts: &[Point], axis: &TempoAxis) -> Option<Vec<Point>> {
    let n = axis.n.max(1);
    let mut new = pts.to_vec();
    let mut changed = false;
    ui.horizontal_top(|ui| {
        // ---- left panel: the selected point
        ui.vertical(|ui| {
            ui.set_width(230.0);
            ui.label(RichText::new("TEMPO").strong());
            ui.label(RichText::new("Applies to every track.").small().weak());
            ui.add_space(6.0);
            let sel = ed.sel.min(n - 1);
            let existing = new.iter().find(|p| p.0 == sel).copied();
            let (mut bpm, mut ramp) = match existing {
                Some((_, b, r)) => (b, r),
                None => (tempo_at(&new, sel), false),
            };
            ui.horizontal(|ui| {
                ui.label(axis.unit);
                let mut b1 = sel + 1;
                if ui.add(egui::DragValue::new(&mut b1).range(1..=n)).changed() {
                    ed.sel = b1 - 1;
                }
                let name = (axis.label)(sel);
                if name != (sel + 1).to_string() {
                    ui.label(RichText::new(name).color(Color32::from_gray(200)));
                }
                if existing.is_none() {
                    ui.label(RichText::new("(no change yet)").weak());
                }
            });
            ui.horizontal(|ui| {
                ui.label("Tempo");
                if ui.add(egui::DragValue::new(&mut bpm).range(20..=400).suffix(" bpm")).changed() {
                    set_point(&mut new, sel, bpm, ramp);
                    changed = true;
                }
            });
            ui.add_space(4.0);
            ui.label("Until the next point:");
            let before = ramp;
            ui.radio_value(&mut ramp, false, "Constant");
            ui.radio_value(&mut ramp, true, "Progressive (gradual change)");
            if ramp != before {
                set_point(&mut new, sel, bpm, ramp);
                changed = true;
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.add_enabled(sel > 0 && existing.is_some(), egui::Button::new("Delete point")).clicked() {
                    remove_point(&mut new, sel);
                    changed = true;
                }
                if ui.button("Remove automations").on_hover_text("Keep only the starting tempo").clicked() {
                    new.retain(|p| p.0 == 0);
                    if let Some(p) = new.first_mut() {
                        p.2 = false;
                    }
                    changed = true;
                }
            });
            ui.add_space(6.0);
            ui.label(RichText::new("Click the graph to add a point; drag a point up or down to change the tempo, left or right to move it. Delete removes the selected point.").small().weak());
        });
        ui.separator();
        // ---- right: the graph
        if tempo_graph(ui, ed, &mut new, axis) {
            changed = true;
        }
    });
    // Delete removes the selected point (not while typing).
    if ui.ctx().input(|i| i.key_pressed(egui::Key::Delete)) && !ui.ctx().egui_wants_keyboard_input() && ed.sel > 0 && new.iter().any(|p| p.0 == ed.sel) {
        remove_point(&mut new, ed.sel);
        changed = true;
    }
    changed.then_some(new)
}

/// The tempo at a place, following gradual changes between points.
pub fn tempo_at(pts: &[Point], at: usize) -> u32 {
    let k = pts.iter().rposition(|p| p.0 <= at).unwrap_or(0);
    let Some(&(b0, v0, ramp)) = pts.get(k) else { return 120 };
    match pts.get(k + 1) {
        Some(&(b1, v1, _)) if ramp && b1 > b0 => {
            let f = (at - b0) as f32 / (b1 - b0) as f32;
            (v0 as f32 + (v1 as f32 - v0 as f32) * f).round() as u32
        }
        _ => v0,
    }
}

fn tempo_graph(ui: &mut Ui, ed: &mut TempoEdit, pts: &mut Vec<Point>, axis: &TempoAxis) -> bool {
    let n = axis.n.max(1);
    let mut changed = false;
    let size = Vec2::new(ui.available_width().max(520.0), 230.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click_and_drag());
    let p = ui.painter().clone();
    p.rect_filled(rect, 4.0, Color32::from_rgb(24, 26, 31));
    let area = Rect::from_min_max(rect.min + Vec2::new(40.0, 10.0), rect.max - Vec2::new(10.0, 22.0));
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
    // Names under the graph: every place when there's room, otherwise every 2nd, 4th ...
    let per = area.width() / n as f32;
    let widest = (0..n).map(|b| (axis.label)(b).chars().count()).max().unwrap_or(1) as f32 * 6.0 + 6.0;
    let mut step = 1;
    while (step as f32) * per < widest && step < n {
        step *= 2;
    }
    if n > 16 && step < 4 {
        step = 4;
    }
    for b in 0..=n {
        let x = x_of(b as f32);
        p.line_segment([Pos2::new(x, area.top()), Pos2::new(x, area.bottom())], Stroke::new(1.0, Color32::from_gray(if b % 4 == 0 { 52 } else { 36 })));
        if b < n && b % step == 0 {
            p.text(Pos2::new(x + 3.0, area.bottom() + 10.0), Align2::LEFT_CENTER, (axis.label)(b), FontId::proportional(10.0), Color32::from_gray(140));
        }
    }
    // The cursor's place.
    if let Some(c) = axis.cursor {
        p.rect_filled(
            Rect::from_min_max(Pos2::new(x_of(c as f32), area.top()), Pos2::new(x_of(c as f32 + 1.0), area.bottom())),
            0.0,
            Color32::from_rgba_unmultiplied(80, 140, 240, 30),
        );
    }
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
    for &(b, val, _) in pts.iter() {
        let c = Pos2::new(x_of(b as f32), y_of(val as f32));
        let sel = b == ed.sel;
        p.circle_filled(c, if sel { 6.0 } else { 5.0 }, if sel { Color32::from_rgb(245, 165, 50) } else { Color32::from_rgb(110, 200, 120) });
        if sel {
            p.text(c + Vec2::new(8.0, -10.0), Align2::LEFT_BOTTOM, format!("{val} bpm"), FontId::proportional(11.0), Color32::WHITE);
        }
    }
    // Interaction.
    let snapshot = pts.clone();
    let near = |pos: Pos2| snapshot.iter().find(|&&(b, v, _)| (Pos2::new(x_of(b as f32), y_of(v as f32)) - pos).length() < 9.0).map(|p| p.0);
    if resp.drag_started()
        && let Some(pos) = resp.interact_pointer_pos()
    {
        match near(pos) {
            Some(b) => {
                ed.sel = b;
                ed.drag = true;
            }
            None => ed.drag = false,
        }
    }
    if resp.dragged()
        && ed.drag
        && let Some(pos) = resp.interact_pointer_pos()
    {
        let cur = ed.sel;
        let ramp = snapshot.iter().find(|p| p.0 == cur).map(|p| p.2).unwrap_or(false);
        let val = val_of(pos.y);
        let mut to = bar_of(pos.x);
        if cur == 0 || (to != cur && (to == 0 || snapshot.iter().any(|p| p.0 == to))) {
            to = cur;
        }
        if to != cur {
            remove_point(pts, cur);
            ed.sel = to;
        }
        set_point(pts, to, val, ramp);
        changed = true;
    }
    if resp.drag_stopped() {
        ed.drag = false;
    }
    if resp.clicked()
        && let Some(pos) = resp.interact_pointer_pos()
    {
        match near(pos) {
            Some(b) => ed.sel = b,
            None => {
                let b = bar_of(pos.x);
                if !snapshot.iter().any(|p| p.0 == b) {
                    set_point(pts, b, val_of(pos.y), false);
                    changed = true;
                }
                ed.sel = b;
            }
        }
    }
    changed
}

impl App {
    /// Opens the editor with the tempo point that applies at a bar selected.
    pub(super) fn open_automations(&mut self, bar: usize) {
        self.auto_open = true;
        self.auto_sel = bar.min(self.song.bar_count() - 1);
        self.auto_snap = false;
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
        let pts = self.song.tempo_points();
        let mut ed = TempoEdit { sel: self.auto_sel, drag: self.auto_drag };
        let label = |b: usize| (b + 1).to_string();
        let axis = TempoAxis { unit: "Bar", n, label: &label, cursor: Some(self.bar) };
        let mut result = None;
        egui::Window::new("Automations")
            .open(&mut open)
            .default_size(Vec2::new(820.0, 300.0))
            .anchor(Align2::CENTER_TOP, Vec2::new(0.0, 120.0))
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                result = tempo_editor(ui, &mut ed, &pts, &axis);
            });
        self.auto_sel = ed.sel;
        self.auto_drag = ed.drag;
        if let Some(new) = result {
            self.auto_begin();
            let first = new.first().copied().unwrap_or((0, self.song.tempo, false));
            self.song.tempo = first.1;
            if let Some(m) = self.song.master.first_mut() {
                m.tempo = None;
                m.tempo_ramp = first.2;
            }
            for (b, m) in self.song.master.iter_mut().enumerate().skip(1) {
                let p = new.iter().find(|p| p.0 == b);
                m.tempo = p.map(|p| p.1);
                m.tempo_ramp = p.map(|p| p.2).unwrap_or(false);
            }
            self.dirty = true;
            self.total_secs = self.song_seconds();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            open = false;
        }
        self.auto_open = open;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn points_are_set_moved_and_followed() {
        let mut pts = vec![(0, 100, false)];
        set_point(&mut pts, 4, 80, false);
        set_point(&mut pts, 2, 90, true);
        assert_eq!(pts, vec![(0, 100, false), (2, 90, true), (4, 80, false)]);
        assert_eq!(tempo_at(&pts, 1), 100);
        assert_eq!(tempo_at(&pts, 3), 85);
        remove_point(&mut pts, 0);
        assert_eq!(pts.len(), 3);
        remove_point(&mut pts, 2);
        assert_eq!(pts, vec![(0, 100, false), (4, 80, false)]);
    }
}
