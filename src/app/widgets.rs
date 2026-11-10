//! Small painted widgets and icons.
#![allow(dead_code)]

use crate::model::{Dur, QUARTER};
use egui::{Align2, Color32, FontId, Pos2, Rect, RichText, Sense, Stroke as PStroke, Ui, Vec2};

pub fn sanitize(s: &str) -> String {
    let t: String = s
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if t.trim().is_empty() {
        "song".into()
    } else {
        t
    }
}

pub fn fmt_time(s: f32) -> String {
    format!("{}:{:02}", (s / 60.0) as u32, (s % 60.0) as u32)
}

pub fn fmt_beats(ticks: u32, den: u8) -> String {
    let unit = QUARTER * 4 / den.max(1) as u32;
    let v = ticks as f32 / unit as f32;
    if (v - v.round()).abs() < 0.01 {
        format!("{}", v.round() as i32)
    } else {
        format!("{v:.2}")
    }
}

pub fn section(ui: &mut Ui, title: &str) {
    ui.label(
        RichText::new(title.to_uppercase())
            .size(10.5)
            .color(Color32::from_gray(150))
            .strong(),
    );
    ui.add_space(2.0);
}

pub fn small_toggle(ui: &mut Ui, on: bool, label: &str, tip: &str) -> egui::Response {
    let b = egui::Button::selectable(on, RichText::new(label).size(12.0))
        .frame_when_inactive(true)
        .min_size(Vec2::new(48.0, 24.0));
    ui.add(b).on_hover_text(tip)
}

pub fn toggle(ui: &mut Ui, v: &mut bool, label: &str, tip: &str) {
    if ui
        .add(
            egui::Button::selectable(*v, label)
                .frame_when_inactive(true)
                .min_size(Vec2::new(0.0, 28.0)),
        )
        .on_hover_text(tip)
        .clicked()
    {
        *v = !*v;
    }
}

pub fn dur_button(ui: &mut Ui, d: Dur, selected: bool) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(Vec2::new(36.0, 36.0), Sense::click());
    let hovered = resp.hovered();
    let p = ui.painter();
    let bg = if selected {
        Color32::from_rgb(55, 100, 180)
    } else if hovered {
        Color32::from_gray(70)
    } else {
        Color32::from_gray(55)
    };
    p.rect_filled(r, 4.0, bg);
    let col = Color32::from_gray(235);
    let c = Pos2::new(r.center().x - 3.0, r.center().y + 7.0);
    let filled = d.index() >= 2;
    let pts: Vec<Pos2> = (0..16)
        .map(|i| {
            let a = i as f32 / 16.0 * std::f32::consts::TAU;
            let (x, y) = (a.cos() * 4.6, a.sin() * 3.2);
            let (s, co) = (-0.35f32).sin_cos();
            Pos2::new(c.x + x * co - y * s, c.y + x * s + y * co)
        })
        .collect();
    if filled {
        p.add(egui::Shape::convex_polygon(pts, col, PStroke::NONE));
    } else {
        p.add(egui::Shape::closed_line(pts, PStroke::new(1.5, col)));
    }
    if d != Dur::Whole {
        let sx = c.x + 4.2;
        p.line_segment(
            [Pos2::new(sx, c.y - 1.0), Pos2::new(sx, c.y - 18.0)],
            PStroke::new(1.3, col),
        );
        for f in 0..d.flags() {
            let y = c.y - 18.0 + f as f32 * 4.0;
            p.line_segment(
                [Pos2::new(sx, y), Pos2::new(sx + 6.0, y + 6.0)],
                PStroke::new(1.5, col),
            );
        }
    }
    resp
}

pub type Icon = fn(&egui::Painter, Rect, Color32);

pub fn tool_button(ui: &mut Ui, label: &str, tip: &str, icon: Icon) -> egui::Response {
    let text_w = if label.is_empty() {
        0.0
    } else {
        label.chars().count() as f32 * 7.0 + 6.0
    };
    let (r, resp) = ui.allocate_exact_size(Vec2::new(30.0 + text_w, 30.0), Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(r, 4.0, Color32::from_gray(70));
    }
    let col = Color32::from_gray(225);
    let ir = Rect::from_center_size(Pos2::new(r.left() + 15.0, r.center().y), Vec2::splat(16.0));
    icon(ui.painter(), ir, col);
    if !label.is_empty() {
        ui.painter().text(
            Pos2::new(r.left() + 28.0, r.center().y),
            Align2::LEFT_CENTER,
            label,
            FontId::proportional(12.5),
            col,
        );
    }
    resp.on_hover_text(tip)
}

pub fn icon_new(p: &egui::Painter, r: Rect, c: Color32) {
    let s = PStroke::new(1.4, c);
    p.add(egui::Shape::closed_line(
        vec![
            r.left_top() + Vec2::new(2.0, 0.0),
            Pos2::new(r.right() - 5.0, r.top()),
            Pos2::new(r.right() - 1.0, r.top() + 4.0),
            r.right_bottom() + Vec2::new(-1.0, 0.0),
            r.left_bottom() + Vec2::new(2.0, 0.0),
        ],
        s,
    ));
}
pub fn icon_open(p: &egui::Painter, r: Rect, c: Color32) {
    let s = PStroke::new(1.4, c);
    p.add(egui::Shape::closed_line(
        vec![
            r.left_top() + Vec2::new(0.0, 2.0),
            r.left_top() + Vec2::new(6.0, 2.0),
            r.left_top() + Vec2::new(8.0, 4.0),
            Pos2::new(r.right(), r.top() + 4.0),
            r.right_bottom() + Vec2::new(0.0, -2.0),
            r.left_bottom() + Vec2::new(0.0, -2.0),
        ],
        s,
    ));
}
pub fn icon_save(p: &egui::Painter, r: Rect, c: Color32) {
    let s = PStroke::new(1.4, c);
    p.rect_stroke(r.shrink(1.0), 2.0, s, egui::StrokeKind::Inside);
    p.rect_filled(
        Rect::from_min_size(r.min + Vec2::new(4.0, 2.0), Vec2::new(8.0, 4.0)),
        0.0,
        c,
    );
    p.rect_stroke(
        Rect::from_min_size(r.min + Vec2::new(4.0, 9.0), Vec2::new(8.0, 5.0)),
        0.0,
        s,
        egui::StrokeKind::Inside,
    );
}
pub fn icon_undo(p: &egui::Painter, r: Rect, c: Color32) {
    let s = PStroke::new(1.6, c);
    p.add(egui::epaint::QuadraticBezierShape::from_points_stroke(
        [
            r.left_center() + Vec2::new(3.0, -2.0),
            Pos2::new(r.right() + 3.0, r.top() - 1.0),
            r.right_bottom() + Vec2::new(-3.0, -1.0),
        ],
        false,
        Color32::TRANSPARENT,
        s,
    ));
    p.add(egui::Shape::convex_polygon(
        vec![
            r.left_center() + Vec2::new(-1.0, -2.0),
            r.left_center() + Vec2::new(6.0, -7.0),
            r.left_center() + Vec2::new(6.0, 3.0),
        ],
        c,
        PStroke::NONE,
    ));
}
pub fn icon_redo(p: &egui::Painter, r: Rect, c: Color32) {
    let s = PStroke::new(1.6, c);
    p.add(egui::epaint::QuadraticBezierShape::from_points_stroke(
        [
            r.right_center() + Vec2::new(-3.0, -2.0),
            Pos2::new(r.left() - 3.0, r.top() - 1.0),
            r.left_bottom() + Vec2::new(3.0, -1.0),
        ],
        false,
        Color32::TRANSPARENT,
        s,
    ));
    p.add(egui::Shape::convex_polygon(
        vec![
            r.right_center() + Vec2::new(1.0, -2.0),
            r.right_center() + Vec2::new(-6.0, 3.0),
            r.right_center() + Vec2::new(-6.0, -7.0),
        ],
        c,
        PStroke::NONE,
    ));
}
pub fn icon_play(p: &egui::Painter, r: Rect, _c: Color32) {
    p.add(egui::Shape::convex_polygon(
        vec![
            r.left_top() + Vec2::new(3.0, 0.0),
            r.right_center(),
            r.left_bottom() + Vec2::new(3.0, 0.0),
        ],
        Color32::from_rgb(110, 210, 120),
        PStroke::NONE,
    ));
}
pub fn icon_pause(p: &egui::Painter, r: Rect, c: Color32) {
    p.rect_filled(
        Rect::from_min_size(r.min + Vec2::new(2.0, 0.0), Vec2::new(4.5, 16.0)),
        1.0,
        c,
    );
    p.rect_filled(
        Rect::from_min_size(r.min + Vec2::new(9.5, 0.0), Vec2::new(4.5, 16.0)),
        1.0,
        c,
    );
}
pub fn icon_stop(p: &egui::Painter, r: Rect, c: Color32) {
    p.rect_filled(r.shrink(2.0), 2.0, c);
}
fn tri(p: &egui::Painter, r: Rect, c: Color32, right: bool, x0: f32) {
    let (a, b) = if right {
        (r.left() + x0, r.left() + x0 + 7.0)
    } else {
        (r.left() + x0 + 7.0, r.left() + x0)
    };
    p.add(egui::Shape::convex_polygon(
        vec![
            Pos2::new(a, r.top() + 2.0),
            Pos2::new(b, r.center().y),
            Pos2::new(a, r.bottom() - 2.0),
        ],
        c,
        PStroke::NONE,
    ));
}
pub fn icon_prev(p: &egui::Painter, r: Rect, c: Color32) {
    tri(p, r, c, false, 1.0);
    tri(p, r, c, false, 8.0);
}
pub fn icon_next(p: &egui::Painter, r: Rect, c: Color32) {
    tri(p, r, c, true, 1.0);
    tri(p, r, c, true, 8.0);
}
pub fn icon_first(p: &egui::Painter, r: Rect, c: Color32) {
    p.rect_filled(
        Rect::from_min_size(
            Pos2::new(r.left() + 1.0, r.top() + 2.0),
            Vec2::new(2.0, 12.0),
        ),
        0.0,
        c,
    );
    tri(p, r, c, false, 4.0);
    tri(p, r, c, false, 10.0);
}
pub fn icon_last(p: &egui::Painter, r: Rect, c: Color32) {
    tri(p, r, c, true, 0.0);
    tri(p, r, c, true, 6.0);
    p.rect_filled(
        Rect::from_min_size(
            Pos2::new(r.right() - 3.0, r.top() + 2.0),
            Vec2::new(2.0, 12.0),
        ),
        0.0,
        c,
    );
}

pub use super::icons::*;
