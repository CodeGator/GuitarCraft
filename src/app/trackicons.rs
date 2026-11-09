//! Track icons: a small picture of the instrument, chosen from the list beside
//! the track name and shown in the tracks panel and the top display. Each has a
//! number that is saved with the track (as in .gp files).

use crate::model::{Instrument, Track};
use egui::{epaint::QuadraticBezierShape, Color32, Painter, Pos2, Rect, Shape, Stroke, Vec2};

/// Groups of icons: (group name, [(number, name)]).
pub const GROUPS: [(&str, &[(u8, &str)]); 3] = [
    (
        "Guitars & Basses",
        &[
            (1, "Electric guitar"),
            (2, "Electric guitar (single cut)"),
            (3, "Electric guitar (angular)"),
            (22, "Acoustic guitar"),
            (4, "Classical guitar"),
            (6, "Jazz guitar"),
            (9, "12-string guitar"),
            (5, "Electric bass"),
            (10, "Acoustic bass"),
            (7, "Ukulele"),
            (8, "Banjo"),
            (11, "Mandolin"),
            (13, "Resonator"),
            (14, "7-string guitar"),
        ],
    ),
    (
        "Orchestral",
        &[(15, "Piano"), (16, "Electric piano"), (17, "Violin"), (18, "Trumpet"), (19, "Saxophone"), (20, "Flute"), (12, "Voice"), (21, "Strings"), (23, "Synthesizer")],
    ),
    ("Percussions", &[(24, "Drum kit"), (25, "Maracas"), (26, "Conga")]),
];

/// The usual icon of an instrument.
pub fn default_icon(i: Instrument) -> u8 {
    use Instrument::*;
    match i {
        SteelGuitar => 22,
        NylonGuitar => 4,
        CleanGuitar => 1,
        OverdriveGuitar => 2,
        DistortionGuitar => 3,
        Bass => 5,
        Ukulele => 7,
        Banjo => 8,
        Piano => 15,
        ElectricPiano | Organ => 16,
        Synth => 23,
        LeadVocals | Choir => 12,
        Violin | Cello => 17,
        Strings => 21,
        Trumpet => 18,
        Saxophone => 19,
        Flute => 20,
        Drums => 24,
    }
}

pub fn known(id: u8) -> bool {
    GROUPS.iter().any(|(_, list)| list.iter().any(|(k, _)| *k == id))
}

/// The icon a track shows: the one chosen, or its instrument's usual one.
pub fn icon_of(t: &Track) -> u8 {
    if known(t.icon) {
        t.icon
    } else {
        default_icon(t.instrument)
    }
}

/// A guitar-like body: lower and upper bulge sizes, neck length, sound hole, cutaway.
fn guitar(p: &Painter, r: Rect, c: Color32, low: f32, up: f32, neck: f32, hole: bool, cut: bool) {
    let w = r.width();
    let cx = r.center().x;
    let lb = Pos2::new(cx, r.bottom() - w * low);
    let ub = Pos2::new(cx, lb.y - w * (low + up) * 0.85);
    let top = (ub.y - w * up - r.height() * neck).max(r.top() - 2.0);
    p.rect_filled(Rect::from_min_max(Pos2::new(cx - 0.9, top + 2.0), Pos2::new(cx + 0.9, ub.y)), 0.0, c);
    p.rect_filled(Rect::from_min_max(Pos2::new(cx - 1.8, top), Pos2::new(cx + 1.8, top + 3.0)), 1.0, c);
    p.circle_filled(lb, w * low, c);
    if cut {
        // A cutaway: the upper bulge sits to one side.
        p.circle_filled(Pos2::new(ub.x - w * up * 0.35, ub.y), w * up * 0.85, c);
    } else {
        p.circle_filled(ub, w * up, c);
    }
    if hole {
        p.circle_filled(Pos2::new(cx, (lb.y + ub.y) * 0.5), w * 0.13, Color32::from_black_alpha(150));
    }
}

/// Draws icon `id` in `r` in colour `c`.
pub fn draw(p: &Painter, r: Rect, c: Color32, id: u8) {
    let st = Stroke::new(1.4, c);
    let cx = r.center().x;
    let cy = r.center().y;
    let w = r.width();
    let h = r.height();
    match id {
        1 => guitar(p, r, c, 0.42, 0.32, 0.25, false, true),
        2 => guitar(p, r, c, 0.45, 0.36, 0.22, false, false),
        3 => {
            // An angular body.
            let pts = vec![Pos2::new(cx - w * 0.55, r.bottom()), Pos2::new(cx, r.bottom() - h * 0.5), Pos2::new(cx + w * 0.55, r.bottom())];
            p.add(Shape::convex_polygon(pts, c, Stroke::NONE));
            p.rect_filled(Rect::from_min_max(Pos2::new(cx - 0.9, r.top()), Pos2::new(cx + 0.9, r.bottom() - h * 0.3)), 0.0, c);
            p.rect_filled(Rect::from_min_max(Pos2::new(cx - 1.8, r.top() - 1.0), Pos2::new(cx + 1.8, r.top() + 2.0)), 1.0, c);
        }
        22 | 13 => {
            guitar(p, r, c, 0.46, 0.36, 0.18, true, false);
            if id == 13 {
                p.circle_stroke(Pos2::new(cx, r.bottom() - w * 0.46), w * 0.3, Stroke::new(1.0, Color32::from_black_alpha(160)));
            }
        }
        4 => guitar(p, r, c, 0.44, 0.34, 0.22, true, false),
        6 => {
            guitar(p, r, c, 0.5, 0.38, 0.12, false, true);
            p.line_segment([Pos2::new(cx - w * 0.25, cy + h * 0.15), Pos2::new(cx - w * 0.2, cy + h * 0.35)], Stroke::new(1.0, Color32::from_black_alpha(160)));
            p.line_segment([Pos2::new(cx + w * 0.25, cy + h * 0.15), Pos2::new(cx + w * 0.2, cy + h * 0.35)], Stroke::new(1.0, Color32::from_black_alpha(160)));
        }
        9 | 14 => {
            guitar(p, r, c, 0.44, 0.34, 0.22, id == 9, id == 14);
            p.text(Pos2::new(r.right() + 1.0, r.top() + 2.0), egui::Align2::LEFT_TOP, if id == 9 { "12" } else { "7" }, egui::FontId::proportional(6.5), c);
        }
        5 => guitar(p, r, c, 0.36, 0.28, 0.42, false, true),
        10 => guitar(p, r, c, 0.4, 0.32, 0.38, true, false),
        7 => guitar(p, Rect::from_center_size(Pos2::new(cx, cy + h * 0.1), Vec2::new(w * 0.8, h * 0.8)), c, 0.42, 0.34, 0.18, true, false),
        8 => {
            let body = Pos2::new(cx, r.bottom() - w * 0.45);
            p.rect_filled(Rect::from_min_max(Pos2::new(cx - 0.9, r.top()), Pos2::new(cx + 0.9, body.y)), 0.0, c);
            p.circle_filled(body, w * 0.5, c);
            p.circle_filled(body, w * 0.33, Color32::from_gray(225));
        }
        11 => {
            let body = Pos2::new(cx, r.bottom() - w * 0.45);
            p.rect_filled(Rect::from_min_max(Pos2::new(cx - 0.9, r.top() + h * 0.2), Pos2::new(cx + 0.9, body.y)), 0.0, c);
            p.add(Shape::convex_polygon(
                (0..16).map(|k| {
                    let a = k as f32 / 16.0 * std::f32::consts::TAU;
                    Pos2::new(body.x + a.cos() * w * 0.45, body.y + a.sin() * w * 0.55)
                }).collect(),
                c,
                Stroke::NONE,
            ));
            p.circle_filled(body, w * 0.12, Color32::from_black_alpha(150));
        }
        15 | 16 | 23 => {
            let k = Rect::from_center_size(r.center(), Vec2::new(w + 3.0, h * 0.7));
            p.rect_filled(k, 1.5, c);
            for i in 1..4 {
                let x = k.left() + k.width() * i as f32 / 4.0;
                p.line_segment([Pos2::new(x, k.top()), Pos2::new(x, k.bottom())], Stroke::new(1.0, Color32::from_black_alpha(160)));
            }
            for i in [1, 3] {
                let x = k.left() + k.width() * i as f32 / 4.0;
                p.rect_filled(Rect::from_center_size(Pos2::new(x, k.top() + k.height() * 0.3), Vec2::new(2.4, k.height() * 0.6)), 0.0, Color32::from_black_alpha(200));
            }
            if id == 16 {
                p.circle_filled(Pos2::new(k.right() - 2.0, k.top() - 2.5), 1.5, c);
            }
            if id == 23 {
                let y = k.top() - 3.0;
                p.add(Shape::line((0..=8).map(|i| Pos2::new(k.left() + k.width() * i as f32 / 8.0, y - (i as f32 * 1.6).sin() * 1.6)).collect(), Stroke::new(1.0, c)));
            }
        }
        17 => {
            guitar(p, Rect::from_center_size(Pos2::new(cx, cy + 1.0), Vec2::new(w * 0.85, h)), c, 0.4, 0.32, 0.3, false, false);
            p.line_segment([Pos2::new(cx - w * 0.15, cy + h * 0.25), Pos2::new(cx - w * 0.1, cy + h * 0.1)], Stroke::new(0.8, Color32::from_black_alpha(160)));
            p.line_segment([Pos2::new(r.right(), r.top() + 2.0), Pos2::new(r.left() - 1.0, r.bottom() - 1.0)], Stroke::new(0.8, c));
        }
        18 => {
            p.line_segment([Pos2::new(r.left() - 1.0, cy), Pos2::new(r.right() - 2.0, cy)], Stroke::new(2.4, c));
            p.add(Shape::convex_polygon(vec![Pos2::new(r.right() - 3.0, cy - 1.0), Pos2::new(r.right() + 2.0, cy - 5.0), Pos2::new(r.right() + 2.0, cy + 5.0), Pos2::new(r.right() - 3.0, cy + 1.0)], c, Stroke::NONE));
            p.circle_stroke(Pos2::new(cx - 1.0, cy - 3.5), 2.5, st);
        }
        19 => {
            p.add(QuadraticBezierShape::from_points_stroke([Pos2::new(cx + w * 0.2, r.top()), Pos2::new(cx - w * 0.6, r.bottom() + 2.0), Pos2::new(cx + w * 0.45, r.bottom() - h * 0.25)], false, Color32::TRANSPARENT, Stroke::new(2.6, c)));
            p.circle_filled(Pos2::new(cx + w * 0.45, r.bottom() - h * 0.25), 2.5, c);
        }
        20 => {
            p.line_segment([Pos2::new(r.left() - 1.0, r.bottom()), Pos2::new(r.right() + 1.0, r.top())], Stroke::new(2.0, c));
            for k in 1..4 {
                let t = k as f32 / 4.0;
                p.circle_filled(Pos2::new(r.left() + w * t, r.bottom() - h * t), 0.9, Color32::from_black_alpha(200));
            }
        }
        12 => {
            let head = Pos2::new(cx, r.top() + w * 0.4);
            p.circle_filled(head, w * 0.4, c);
            p.line_segment([Pos2::new(cx, head.y + w * 0.4), Pos2::new(cx, r.bottom())], Stroke::new(2.2, c));
            p.line_segment([Pos2::new(cx - 3.0, r.bottom()), Pos2::new(cx + 3.0, r.bottom())], st);
        }
        21 => {
            for k in 0..3 {
                let x = r.left() + w * (0.15 + k as f32 * 0.35);
                p.line_segment([Pos2::new(x, r.top() + 2.0), Pos2::new(x, r.bottom())], Stroke::new(1.6, c));
            }
            p.rect_filled(Rect::from_min_max(Pos2::new(r.left() - 1.0, r.bottom() - 3.0), Pos2::new(r.right() + 1.0, r.bottom())), 1.0, c);
        }
        24 => {
            let top = Rect::from_center_size(Pos2::new(cx, r.top() + 4.0), Vec2::new(w + 3.0, 5.0));
            p.rect_filled(Rect::from_min_max(Pos2::new(top.left(), top.center().y), Pos2::new(top.right(), r.bottom() - 1.0)), 1.0, c);
            p.add(Shape::ellipse_filled(top.center(), top.size() * 0.5, c));
            p.add(Shape::ellipse_stroke(top.center(), top.size() * 0.5, Stroke::new(1.0, Color32::from_black_alpha(170))));
            p.line_segment([Pos2::new(top.left() + 2.0, r.top() - 3.0), Pos2::new(top.center().x, top.center().y)], st);
        }
        25 => {
            for (dx, dy) in [(-2.5f32, 0.0f32), (2.5, 2.0)] {
                let b = Pos2::new(cx + dx, r.top() + h * 0.3 + dy);
                p.circle_filled(b, w * 0.3, c);
                p.line_segment([b, Pos2::new(b.x + dx * 0.5, r.bottom())], Stroke::new(1.4, c));
            }
        }
        26 => {
            let pts = vec![Pos2::new(cx - w * 0.45, r.top() + 2.0), Pos2::new(cx + w * 0.45, r.top() + 2.0), Pos2::new(cx + w * 0.3, r.bottom()), Pos2::new(cx - w * 0.3, r.bottom())];
            p.add(Shape::convex_polygon(pts, c, Stroke::NONE));
            p.add(Shape::ellipse_filled(Pos2::new(cx, r.top() + 2.0), Vec2::new(w * 0.45, 1.6), Color32::from_gray(225)));
        }
        _ => guitar(p, r, c, 0.42, 0.32, 0.25, false, false),
    }
}
