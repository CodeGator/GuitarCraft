//! Small painted widgets and icons.

use egui::{Color32, Pos2, Rect, Stroke as PStroke, Vec2};

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

pub type Icon = fn(&egui::Painter, Rect, Color32);

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
