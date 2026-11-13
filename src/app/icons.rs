//! Toolbar and panel widgets in a flat icon style.

use super::widgets::Icon;
use crate::model::Dur;
use egui::{Align2, Color32, FontId, Pos2, Rect, Sense, Stroke as PStroke, Ui, Vec2};

const SEL_BLUE: Color32 = Color32::from_rgb(61, 123, 224);

pub fn icon_toggle(ui: &mut Ui, on: bool, tip: &str, icon: Icon) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(Vec2::new(30.0, 30.0), Sense::click());
    let p = ui.painter();
    if on {
        p.rect_filled(r.shrink(1.0), 4.0, SEL_BLUE);
    } else if resp.hovered() {
        p.rect_filled(r.shrink(1.0), 4.0, Color32::from_gray(80));
    }
    icon(
        p,
        Rect::from_center_size(r.center(), Vec2::splat(16.0)),
        Color32::from_gray(235),
    );
    resp.on_hover_text(tip)
}

pub fn play_button(ui: &mut Ui, playing: bool) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(Vec2::new(36.0, 34.0), Sense::click());
    let p = ui.painter();
    let c = r.center();
    let col = if resp.hovered() {
        Color32::WHITE
    } else {
        Color32::from_gray(232)
    };
    p.circle_stroke(c, 15.0, PStroke::new(1.8, col));
    if playing {
        p.rect_filled(
            Rect::from_center_size(c + Vec2::new(-3.5, 0.0), Vec2::new(4.0, 13.0)),
            1.0,
            col,
        );
        p.rect_filled(
            Rect::from_center_size(c + Vec2::new(3.5, 0.0), Vec2::new(4.0, 13.0)),
            1.0,
            col,
        );
    } else {
        p.add(egui::Shape::convex_polygon(
            vec![
                c + Vec2::new(-4.0, -8.0),
                c + Vec2::new(8.0, 0.0),
                c + Vec2::new(-4.0, 8.0),
            ],
            col,
            PStroke::NONE,
        ));
    }
    resp
}

pub fn glyph_button(ui: &mut Ui, on: bool, text: &str, font: FontId, tip: &str) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(Vec2::new(25.0, 25.0), Sense::click());
    let p = ui.painter();
    if on {
        p.rect_filled(r, 4.0, SEL_BLUE);
    } else if resp.hovered() {
        p.rect_filled(r, 4.0, Color32::from_gray(70));
    }
    let col = if ui.is_enabled() {
        Color32::from_gray(232)
    } else {
        Color32::from_gray(110)
    };
    let gal = p.layout(text.to_string(), font, col, 40.0);
    let sz = gal.size();
    p.galley(r.center() - sz * 0.5, gal, col);
    resp.on_hover_text(tip).on_disabled_hover_text(tip)
}

/// A palette button with a drawn picture: `draw` paints it in the given colour
/// inside the button's inner square.
pub fn paint_button(
    ui: &mut Ui,
    on: bool,
    tip: &str,
    draw: impl FnOnce(&egui::Painter, Rect, Color32),
) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(Vec2::new(25.0, 25.0), Sense::click());
    let p = ui.painter();
    if on {
        p.rect_filled(r, 4.0, SEL_BLUE);
    } else if resp.hovered() {
        p.rect_filled(r, 4.0, Color32::from_gray(70));
    }
    let col = if ui.is_enabled() {
        Color32::from_gray(232)
    } else {
        Color32::from_gray(110)
    };
    draw(p, r.shrink(4.0), col);
    resp.on_hover_text(tip).on_disabled_hover_text(tip)
}

pub fn note_icon_button(ui: &mut Ui, d: Dur, on: bool) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(Vec2::new(25.0, 25.0), Sense::click());
    let p = ui.painter();
    if on {
        p.rect_filled(r, 4.0, SEL_BLUE);
    } else if resp.hovered() {
        p.rect_filled(r, 4.0, Color32::from_gray(70));
    }
    let col = Color32::from_gray(235);
    let c = Pos2::new(r.center().x - 2.5, r.center().y + 6.0);
    let filled = d.index() >= 2;
    let pts: Vec<Pos2> = (0..16)
        .map(|i| {
            let a = i as f32 / 16.0 * std::f32::consts::TAU;
            let (x, y) = (a.cos() * 4.0, a.sin() * 2.8);
            let (s, co) = (-0.35f32).sin_cos();
            Pos2::new(c.x + x * co - y * s, c.y + x * s + y * co)
        })
        .collect();
    if filled {
        p.add(egui::Shape::convex_polygon(pts, col, PStroke::NONE));
    } else {
        p.add(egui::Shape::closed_line(pts, PStroke::new(1.3, col)));
    }
    if d != Dur::Whole {
        let sx = c.x + 3.7;
        p.line_segment(
            [Pos2::new(sx, c.y - 1.0), Pos2::new(sx, c.y - 15.0)],
            PStroke::new(1.2, col),
        );
        for f in 0..d.flags() {
            let y = c.y - 15.0 + f as f32 * 3.2;
            p.line_segment(
                [Pos2::new(sx, y), Pos2::new(sx + 5.0, y + 5.0)],
                PStroke::new(1.3, col),
            );
        }
    }
    resp
}

pub fn icon_home(p: &egui::Painter, r: Rect, c: Color32) {
    let s = PStroke::new(1.6, c);
    p.add(egui::Shape::closed_line(
        vec![
            Pos2::new(r.left() + 1.0, r.center().y),
            Pos2::new(r.center().x, r.top() + 1.0),
            Pos2::new(r.right() - 1.0, r.center().y),
            Pos2::new(r.right() - 3.0, r.center().y),
            Pos2::new(r.right() - 3.0, r.bottom()),
            Pos2::new(r.left() + 3.0, r.bottom()),
            Pos2::new(r.left() + 3.0, r.center().y),
        ],
        s,
    ));
    p.rect_filled(
        Rect::from_min_max(
            Pos2::new(r.center().x - 2.0, r.bottom() - 6.0),
            Pos2::new(r.center().x + 2.0, r.bottom()),
        ),
        0.0,
        c,
    );
}

fn panel_icon(p: &egui::Painter, r: Rect, c: Color32, part: Rect) {
    p.rect_stroke(
        r.shrink(1.0),
        2.0,
        PStroke::new(1.4, c),
        egui::StrokeKind::Inside,
    );
    p.rect_filled(part, 0.0, c);
}
pub fn icon_panel_left(p: &egui::Painter, r: Rect, c: Color32) {
    panel_icon(
        p,
        r,
        c,
        Rect::from_min_size(
            r.min + Vec2::new(1.0, 1.0),
            Vec2::new(5.0, r.height() - 2.0),
        ),
    );
}
pub fn icon_panel_bottom(p: &egui::Painter, r: Rect, c: Color32) {
    panel_icon(
        p,
        r,
        c,
        Rect::from_min_max(
            Pos2::new(r.left() + 1.0, r.bottom() - 6.0),
            r.max - Vec2::splat(1.0),
        ),
    );
}
pub fn icon_panel_right(p: &egui::Painter, r: Rect, c: Color32) {
    panel_icon(
        p,
        r,
        c,
        Rect::from_min_max(
            Pos2::new(r.right() - 6.0, r.top() + 1.0),
            r.max - Vec2::splat(1.0),
        ),
    );
}
pub fn icon_zoom(p: &egui::Painter, r: Rect, c: Color32) {
    p.circle_stroke(
        r.center() + Vec2::new(-2.0, -2.0),
        5.0,
        PStroke::new(1.6, c),
    );
    p.line_segment(
        [r.center() + Vec2::new(1.5, 1.5), r.right_bottom()],
        PStroke::new(2.0, c),
    );
}
pub fn icon_loop(p: &egui::Painter, r: Rect, c: Color32) {
    let pts: Vec<Pos2> = (0..=20)
        .map(|i| {
            let a = 0.6 + i as f32 / 20.0 * 5.2;
            r.center() + Vec2::new(a.cos() * 7.0, a.sin() * 5.5)
        })
        .collect();
    let end = *pts.last().unwrap();
    p.add(egui::Shape::line(pts, PStroke::new(1.5, c)));
    p.add(egui::Shape::convex_polygon(
        vec![
            end + Vec2::new(-3.0, -3.0),
            end + Vec2::new(3.0, -1.0),
            end + Vec2::new(-1.0, 3.0),
        ],
        c,
        PStroke::NONE,
    ));
}
pub fn icon_metronome(p: &egui::Painter, r: Rect, c: Color32) {
    let s = PStroke::new(1.5, c);
    p.add(egui::Shape::closed_line(
        vec![
            Pos2::new(r.center().x - 3.0, r.top() + 1.0),
            Pos2::new(r.center().x + 3.0, r.top() + 1.0),
            Pos2::new(r.right() - 2.0, r.bottom()),
            Pos2::new(r.left() + 2.0, r.bottom()),
        ],
        s,
    ));
    p.line_segment(
        [
            Pos2::new(r.center().x, r.bottom() - 3.0),
            Pos2::new(r.right(), r.top() + 3.0),
        ],
        s,
    );
}
pub fn icon_countin(p: &egui::Painter, r: Rect, c: Color32) {
    p.text(
        r.center(),
        Align2::CENTER_CENTER,
        "1·2",
        FontId::new(11.0, egui::FontFamily::Name("bold".into())),
        c,
    );
}
pub fn icon_fretboard(p: &egui::Painter, r: Rect, c: Color32) {
    let s = PStroke::new(1.2, c);
    p.rect_stroke(
        Rect::from_min_max(
            Pos2::new(r.left(), r.top() + 3.0),
            Pos2::new(r.right(), r.bottom() - 3.0),
        ),
        1.0,
        s,
        egui::StrokeKind::Inside,
    );
    for k in 1..4 {
        let x = r.left() + k as f32 * r.width() / 4.0;
        p.line_segment(
            [Pos2::new(x, r.top() + 3.0), Pos2::new(x, r.bottom() - 3.0)],
            s,
        );
    }
    for k in 1..3 {
        let y = r.top() + 3.0 + k as f32 * (r.height() - 6.0) / 3.0;
        p.line_segment(
            [Pos2::new(r.left(), y), Pos2::new(r.right(), y)],
            PStroke::new(0.8, c),
        );
    }
}

pub fn eye_icon(p: &egui::Painter, r: Rect, c: Color32, open: bool) {
    let mut pts: Vec<Pos2> = (0..=16)
        .map(|i| {
            let t = i as f32 / 16.0 * std::f32::consts::PI;
            Pos2::new(
                r.left() + r.width() * (1.0 - t.cos()) * 0.5,
                r.center().y - t.sin() * r.height() * 0.35,
            )
        })
        .collect();
    pts.extend((1..16).map(|i| {
        let t = i as f32 / 16.0 * std::f32::consts::PI;
        Pos2::new(
            r.left() + r.width() * (1.0 + t.cos()) * 0.5,
            r.center().y + t.sin() * r.height() * 0.35,
        )
    }));
    p.add(egui::Shape::closed_line(pts, PStroke::new(1.3, c)));
    if open {
        p.circle_filled(r.center(), r.height() * 0.2, c);
    } else {
        p.line_segment([r.left_top(), r.right_bottom()], PStroke::new(1.3, c));
    }
}

pub fn speaker_icon(p: &egui::Painter, r: Rect, c: Color32) {
    let cy = r.center().y;
    p.add(egui::Shape::convex_polygon(
        vec![
            Pos2::new(r.left() + 1.0, cy - 2.5),
            Pos2::new(r.left() + 4.0, cy - 2.5),
            Pos2::new(r.left() + 8.0, r.top() + 1.0),
            Pos2::new(r.left() + 8.0, r.bottom() - 1.0),
            Pos2::new(r.left() + 4.0, cy + 2.5),
            Pos2::new(r.left() + 1.0, cy + 2.5),
        ],
        c,
        PStroke::NONE,
    ));
    p.line_segment(
        [
            Pos2::new(r.right() - 4.0, cy - 3.0),
            Pos2::new(r.right(), cy + 3.0),
        ],
        PStroke::new(1.3, c),
    );
    p.line_segment(
        [
            Pos2::new(r.right() - 4.0, cy + 3.0),
            Pos2::new(r.right(), cy - 3.0),
        ],
        PStroke::new(1.3, c),
    );
}

pub fn headphones_icon(p: &egui::Painter, r: Rect, c: Color32) {
    let pts: Vec<Pos2> = (0..=16)
        .map(|i| {
            let t = i as f32 / 16.0 * std::f32::consts::PI;
            Pos2::new(
                r.center().x - t.cos() * r.width() * 0.42,
                r.center().y + 1.0 - t.sin() * r.height() * 0.45,
            )
        })
        .collect();
    p.add(egui::Shape::line(pts, PStroke::new(1.4, c)));
    p.rect_filled(
        Rect::from_min_size(Pos2::new(r.left(), r.center().y), Vec2::new(3.5, 6.0)),
        1.0,
        c,
    );
    p.rect_filled(
        Rect::from_min_size(
            Pos2::new(r.right() - 3.5, r.center().y),
            Vec2::new(3.5, 6.0),
        ),
        1.0,
        c,
    );
}

pub fn knob(p: &egui::Painter, c: Pos2, rad: f32, value: f32) {
    p.circle_filled(c, rad, Color32::from_rgb(30, 32, 37));
    p.circle_stroke(c, rad, PStroke::new(1.3, Color32::from_gray(200)));
    let a = -std::f32::consts::FRAC_PI_2 + value.clamp(-1.0, 1.0) * 2.4;
    p.line_segment(
        [c, c + Vec2::new(a.cos(), a.sin()) * (rad - 1.5)],
        PStroke::new(1.6, Color32::from_gray(230)),
    );
}

pub fn flag_icon(p: &egui::Painter, r: Rect, c: Color32) {
    p.line_segment([r.left_top(), r.left_bottom()], PStroke::new(1.3, c));
    p.add(egui::Shape::convex_polygon(
        vec![
            r.left_top(),
            Pos2::new(r.right(), r.top() + 2.5),
            Pos2::new(r.left(), r.top() + 6.0),
        ],
        c,
        PStroke::NONE,
    ));
}

pub fn icon_keyboard(p: &egui::Painter, r: Rect, c: Color32) {
    let body = Rect::from_min_max(
        Pos2::new(r.left(), r.top() + 2.0),
        Pos2::new(r.right(), r.bottom() - 2.0),
    );
    p.rect_stroke(body, 1.0, PStroke::new(1.2, c), egui::StrokeKind::Inside);
    for k in 1..4 {
        let x = body.left() + k as f32 * body.width() / 4.0;
        p.line_segment(
            [Pos2::new(x, body.top()), Pos2::new(x, body.bottom())],
            PStroke::new(1.0, c),
        );
    }
    for k in [1.0f32, 2.0, 3.0] {
        let x = body.left() + k * body.width() / 4.0;
        p.rect_filled(
            Rect::from_center_size(
                Pos2::new(x, body.top() + body.height() * 0.3),
                Vec2::new(2.5, body.height() * 0.6),
            ),
            0.0,
            c,
        );
    }
}

/// Multitrack: several staves stacked, like a page of a full score.
pub fn multitrack_icon(p: &egui::Painter, r: Rect, c: Color32) {
    let s = PStroke::new(1.0, c);
    for k in 0..3 {
        let y0 = r.top() + k as f32 * r.height() / 3.0 + 1.0;
        for l in 0..2 {
            let y = y0 + l as f32 * 2.5;
            p.line_segment([Pos2::new(r.left(), y), Pos2::new(r.right(), y)], s);
        }
    }
    p.line_segment([r.left_top(), r.left_bottom()], s);
}
