//! Track icons: a small picture of the instrument, chosen from the list beside
//! the track name and shown in the tracks panel and the top display. Each has a
//! number that is saved with the track (as in .gp files).

use crate::model::{Instrument, Track};
use egui::{Color32, Painter, Pos2, Rect, Shape, Stroke, Vec2, epaint::QuadraticBezierShape};

/// Groups of icons: (group name, [(number, name)]).
pub const GROUPS: [(&str, &[(u8, &str)]); 3] = [
    (
        "Guitars & Basses",
        &[
            (22, "Steel-string acoustic guitar"),
            (6, "Nylon-string acoustic guitar"),
            (9, "12-string acoustic guitar"),
            (1, "Electric guitar (Strat type)"),
            (5, "Electric bass"),
            (4, "Classical guitar"),
            (2, "Electric guitar (Les Paul type)"),
            (3, "Electric guitar"),
            (14, "Electric guitar (hooked head)"),
            (10, "Upright bass"),
            (7, "Ukulele"),
            (8, "Banjo"),
            (11, "Mandolin"),
            (13, "Folk guitar"),
        ],
    ),
    (
        "Orchestral",
        &[
            (15, "Piano"),
            (16, "Electric piano"),
            (17, "Violin"),
            (18, "Trumpet"),
            (19, "Saxophone"),
            (20, "Flute"),
            (12, "Voice"),
            (21, "Strings"),
            (23, "Synthesizer"),
        ],
    ),
    (
        "Percussions",
        &[(24, "Drum kit"), (25, "Maracas"), (26, "Conga")],
    ),
];

/// The usual icon of an instrument.
pub fn default_icon(i: Instrument) -> u8 {
    use Instrument::*;
    match i {
        SteelGuitar => 22,
        NylonGuitar => 6,
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
    GROUPS
        .iter()
        .any(|(_, list)| list.iter().any(|(k, _)| *k == id))
}

/// The icon a track shows: the one chosen, or its instrument's usual one.
pub fn icon_of(t: &Track) -> u8 {
    if known(t.icon) {
        t.icon
    } else {
        default_icon(t.instrument)
    }
}

/// Draws in an icon's own small grid: 16 units tall, 0 in the middle, turned by
/// `rot` (radians) around the centre. Detail is cut out in the panel's dark colour.
struct Pen<'a> {
    p: &'a Painter,
    c: Pos2,
    s: f32,
    rot: f32,
    ink: Color32,
}

const CUT: Color32 = Color32::from_rgb(38, 40, 46);

impl Pen<'_> {
    fn pt(&self, x: f32, y: f32) -> Pos2 {
        let (sn, cs) = self.rot.sin_cos();
        let (x, y) = (x * self.s, y * self.s);
        Pos2::new(self.c.x + x * cs - y * sn, self.c.y + x * sn + y * cs)
    }
    fn poly(&self, pts: &[(f32, f32)], col: Color32) {
        let v: Vec<Pos2> = pts.iter().map(|&(x, y)| self.pt(x, y)).collect();
        self.p.add(Shape::convex_polygon(v, col, Stroke::NONE));
    }
    fn rect(&self, x0: f32, y0: f32, x1: f32, y1: f32, col: Color32) {
        self.poly(&[(x0, y0), (x1, y0), (x1, y1), (x0, y1)], col);
    }
    fn line(&self, a: (f32, f32), b: (f32, f32), w: f32, col: Color32) {
        self.p.line_segment([self.pt(a.0, a.1), self.pt(b.0, b.1)], Stroke::new(w * self.s, col));
    }
    fn dot(&self, x: f32, y: f32, r: f32, col: Color32) {
        self.p.circle_filled(self.pt(x, y), r * self.s, col);
    }
    fn ring(&self, x: f32, y: f32, r: f32, w: f32, col: Color32) {
        self.p.circle_stroke(self.pt(x, y), r * self.s, Stroke::new(w * self.s, col));
    }
}

/// The top edge of a headstock.
#[derive(Clone, Copy, PartialEq)]
enum Top {
    Round,
    Flat,
    OpenBook,
    Flare,
}

/// A front-on headstock in Guitar Pro's style: a large solid shape, short peg
/// bars sticking out of each side (`left`, `right`), and a short neck below.
/// `hw` is the half width; `slots` cuts the two string slots of nylon-string
/// guitars.
fn headstock(pen: &Pen, left: usize, right: usize, hw: f32, top: Top, slots: bool) {
    let ink = pen.ink;
    let mut body: Vec<(f32, f32)> = match top {
        Top::Round => (0..=8)
            .map(|k| {
                let x = -hw + 2.0 * hw * k as f32 / 8.0;
                (x, -8.0 + 1.6 * (x / hw).powi(2))
            })
            .collect(),
        Top::Flat => vec![(-hw, -7.2), (-hw + 0.8, -8.0), (hw - 0.8, -8.0), (hw, -7.2)],
        Top::OpenBook => vec![(-hw, -6.8), (-hw * 0.5, -8.2), (0.0, -7.3), (hw * 0.5, -8.2), (hw, -6.8)],
        Top::Flare => vec![(-hw - 0.6, -8.0), (hw + 0.6, -8.0)],
    };
    if top == Top::OpenBook {
        // Not convex: draw it as two halves.
        pen.poly(&[(-hw, -6.8), (-hw * 0.5, -8.2), (0.0, -7.3), (0.0, 3.2), (-1.4, 4.4), (-hw, 2.4)], ink);
        pen.poly(&[(0.0, -7.3), (hw * 0.5, -8.2), (hw, -6.8), (hw, 2.4), (1.4, 4.4), (0.0, 3.2)], ink);
    } else {
        let (bx, by) = if top == Top::Flare { (hw * 0.55, 2.0) } else { (hw, 2.4) };
        body.extend([(bx, by), (1.4, 4.4), (-1.4, 4.4), (-bx, by)]);
        pen.poly(&body, ink);
    }
    // Pegs: short bars, evenly spaced down each side.
    for (side, n) in [(-1.0f32, left), (1.0f32, right)] {
        if n == 0 {
            continue;
        }
        let (y0, y1) = (-6.6, 1.2);
        let step = (y1 - y0) / n as f32;
        let thick = (step * 0.36).min(0.8);
        for i in 0..n {
            let y = y0 + step * (i as f32 + 0.5);
            // A flared head is narrower lower down.
            let edge = if top == Top::Flare { hw + 0.6 - (y + 8.0) * 0.18 } else { hw };
            let (a, b) = if side < 0.0 { (-edge - 2.2, -edge + 0.2) } else { (edge - 0.2, edge + 2.2) };
            pen.rect(a, y - thick, b, y + thick, ink);
        }
    }
    // The neck.
    pen.rect(-1.25, 4.0, 1.25, 8.0, ink);
    if slots {
        pen.rect(-1.9, -6.0, -0.7, 1.0, CUT);
        pen.rect(0.7, -6.0, 1.9, 1.0, CUT);
    }
}

/// An electric guitar or bass head with every peg on one side, drawn tilted as
/// in Guitar Pro. `wide` for a bass; `shape` 0 rounded (Strat-like), 1 pointed,
/// 2 hooked.
fn inline_head(pen: &Pen, pegs: usize, wide: bool, shape: u8) {
    let ink = pen.ink;
    let pen = Pen { rot: 0.42, ..*pen };
    let w = if wide { 3.3 } else { 2.8 };
    let top: Vec<(f32, f32)> = match shape {
        1 => vec![(-w, -6.8), (0.2, -8.6), (w + 0.6, -6.6)],
        2 => vec![(-w, -6.6), (-w + 0.4, -8.2), (w * 0.3, -8.4), (w + 0.9, -7.2), (w + 0.4, -6.0)],
        _ => vec![(-w, -6.8), (-w * 0.5, -8.2), (w * 0.6, -8.2), (w + 0.6, -6.6)],
    };
    let mut body = top;
    body.extend([(w * 0.95, -1.0), (1.3, 3.2), (-1.3, 3.2), (-w, -1.0)]);
    pen.poly(&body, ink);
    pen.rect(-1.25, 3.0, 1.25, 8.4, ink);
    // Tuners along the left edge, with their holes in the head.
    let step = 7.0 / pegs as f32;
    for i in 0..pegs {
        let y = -7.0 + step * (i as f32 + 0.5);
        pen.rect(-w - 1.3, y - step * 0.22, -w + 0.2, y + step * 0.22, ink);
        pen.dot(-w + 1.0, y, if wide { 0.55 } else { 0.4 }, CUT);
    }
}

/// Draws icon `id` in `r` in colour `c`: a light, solid picture of the
/// instrument (its headstock for the guitar family), with dark cut-out detail.
pub fn draw(p: &Painter, r: Rect, c: Color32, id: u8) {
    let s = r.height() / 16.0;
    let pen = Pen { p, c: r.center(), s, rot: 0.0, ink: c };
    match id {
        // Acoustic guitars: three pegs a side.
        22 => headstock(&pen, 3, 3, 3.5, Top::Round, false),
        6 => headstock(&pen, 3, 3, 3.4, Top::Round, true),
        9 => headstock(&pen, 6, 6, 3.7, Top::Flat, true),
        4 => headstock(&pen, 3, 3, 3.7, Top::Flat, true),
        2 => headstock(&pen, 3, 3, 3.6, Top::OpenBook, false),
        13 => headstock(&pen, 3, 3, 3.3, Top::Round, true),
        // Electric guitars and the bass: pegs in a line, head tilted.
        1 => inline_head(&pen, 6, false, 0),
        3 => inline_head(&pen, 6, false, 1),
        14 => inline_head(&pen, 6, false, 2),
        5 => inline_head(&pen, 4, true, 0),
        10 => {
            // Upright bass: the scroll, the peg box with its pegs, the neck.
            pen.dot(-0.4, -6.4, 2.2, c);
            pen.ring(-0.4, -6.4, 1.1, 0.5, CUT);
            pen.rect(-1.5, -5.0, 1.5, 3.0, c);
            for y in [-3.0f32, 0.6] {
                pen.rect(-3.6, y - 0.6, 3.6, y + 0.6, c);
            }
            pen.rect(-1.0, 3.0, 1.0, 8.0, c);
        }
        7 => {
            // Ukulele: a small rounded head, two pegs a side.
            let pen = Pen { s: s * 0.8, c: pen.pt(0.0, 1.4), ..pen };
            headstock(&pen, 2, 2, 3.2, Top::Round, false);
        }
        8 => {
            // Banjo: a flared head, two pegs a side, and the fifth-string peg on the neck.
            headstock(&pen, 2, 2, 2.6, Top::Flare, false);
            pen.rect(1.25, 5.4, 3.0, 6.4, c);
        }
        11 => {
            // Mandolin: a long narrow head with four pegs a side.
            headstock(&pen, 4, 4, 2.5, Top::Round, false);
        }
        23 => {
            // Synthesizer: a box with a display window.
            pen.rect(-6.5, -4.6, 6.5, 4.6, c);
            pen.rect(-5.4, -3.5, 5.4, 3.5, CUT);
            pen.rect(-4.0, -1.4, 4.0, 1.4, c);
            pen.dot(-2.4, 0.0, 0.6, CUT);
            pen.dot(2.4, 0.0, 0.6, CUT);
        }
        15 | 16 => {
            // A keyboard: white keys with black keys between them.
            pen.rect(-6.5, -4.0, 6.5, 5.0, c);
            for i in 1..5 {
                let x = -6.5 + i as f32 * 2.6;
                pen.line((x, -4.0), (x, 5.0), 0.35, CUT);
            }
            for i in [1, 2, 4] {
                let x = -6.5 + i as f32 * 2.6;
                pen.rect(x - 0.75, -4.0, x + 0.75, 1.0, CUT);
            }
            if id == 16 {
                for x in [-4.5f32, -2.5, 4.5] {
                    pen.dot(x, -6.0, 0.8, c);
                }
            }
            if id == 23 {
                for i in 0..8 {
                    let x0 = -6.0 + i as f32 * 1.5;
                    let y = |x: f32| -6.3 - (x * 1.4).sin() * 1.3;
                    pen.line((x0, y(x0)), (x0 + 1.5, y(x0 + 1.5)), 0.6, c);
                }
            }
        }
        17 => {
            // Violin, seen from the front: the scroll, the peg box with two pegs a
            // side, and the top of the body.
            pen.dot(0.0, -7.0, 1.6, c);
            pen.dot(0.0, -7.0, 0.6, CUT);
            pen.rect(-1.2, -5.6, 1.2, 0.6, c);
            for y in [-4.2f32, -1.4] {
                pen.rect(-3.4, y - 0.55, 3.4, y + 0.55, c);
            }
            pen.rect(-0.8, 0.6, 0.8, 3.0, c);
            pen.poly(&[(-3.6, 3.0), (3.6, 3.0), (4.2, 5.6), (3.4, 8.0), (-3.4, 8.0), (-4.2, 5.6)], c);
            pen.line((-1.6, 4.0), (-1.6, 7.2), 0.45, CUT);
            pen.line((1.6, 4.0), (1.6, 7.2), 0.45, CUT);
        }
        18 => {
            // Trumpet: tube, valves and bell.
            pen.rect(-6.5, -0.6, 3.0, 0.6, c);
            pen.rect(-6.5, 2.2, 1.5, 3.2, c);
            pen.poly(&[(3.0, -0.8), (6.8, -3.4), (6.8, 3.4), (3.0, 0.8)], c);
            for x in [-3.0f32, -1.4, 0.2] {
                pen.rect(x - 0.45, -3.8, x + 0.45, 2.2, c);
            }
        }
        19 => {
            // Saxophone: neck, body and the upturned bell.
            pen.line((1.5, -8.0), (-1.0, -5.5), 1.0, c);
            pen.line((-1.0, -5.5), (-1.6, 3.5), 2.0, c);
            p.add(QuadraticBezierShape::from_points_stroke(
                [pen.pt(-1.6, 3.5), pen.pt(-1.0, 8.0), pen.pt(3.2, 4.0)],
                false,
                Color32::TRANSPARENT,
                Stroke::new(2.0 * s, c),
            ));
            pen.dot(3.4, 3.6, 1.6, c);
            for y in [-2.5f32, 0.0, 2.5] {
                pen.dot(-0.2, y, 0.5, CUT);
            }
        }
        20 => {
            // Flute: a long thin tube with keys.
            pen.line((-7.0, 6.0), (7.0, -6.0), 1.3, c);
            for t in [-3.0f32, -1.0, 1.0, 3.0] {
                pen.dot(t, -t * 6.0 / 7.0, 0.5, CUT);
            }
        }
        12 => {
            // Microphone: a capsule with its grille, in a U-shaped holder on a stand.
            pen.poly(&[(-2.4, -6.0), (-1.4, -7.8), (1.4, -7.8), (2.4, -6.0), (2.4, -0.4), (1.4, 1.2), (-1.4, 1.2), (-2.4, -0.4)], c);
            for y in [-5.4f32, -3.8, -2.2] {
                pen.line((-1.6, y), (1.6, y), 0.45, CUT);
            }
            p.add(QuadraticBezierShape::from_points_stroke(
                [pen.pt(-3.6, -2.4), pen.pt(-3.6, 3.6), pen.pt(0.0, 3.6)],
                false,
                Color32::TRANSPARENT,
                Stroke::new(0.9 * s, c),
            ));
            p.add(QuadraticBezierShape::from_points_stroke(
                [pen.pt(3.6, -2.4), pen.pt(3.6, 3.6), pen.pt(0.0, 3.6)],
                false,
                Color32::TRANSPARENT,
                Stroke::new(0.9 * s, c),
            ));
            pen.line((0.0, 3.6), (0.0, 7.2), 0.9, c);
            pen.line((-2.6, 7.4), (2.6, 7.4), 1.0, c);
        }
        21 => {
            // Strings section: three bows over a bar.
            for k in 0..3 {
                let x = -4.5 + k as f32 * 4.5;
                pen.line((x - 1.0, -7.0), (x + 1.0, 5.0), 0.7, c);
            }
            pen.rect(-6.5, 5.0, 6.5, 7.0, c);
        }
        24 => {
            // Drum: shell, head and two crossed sticks.
            pen.rect(-5.5, -1.0, 5.5, 6.0, c);
            p.add(Shape::ellipse_filled(pen.pt(0.0, -1.0), Vec2::new(5.5 * s, 1.8 * s), c));
            p.add(Shape::ellipse_stroke(pen.pt(0.0, -1.0), Vec2::new(5.5 * s, 1.8 * s), Stroke::new(0.5 * s, CUT)));
            for x in [-2.75f32, 0.0, 2.75] {
                pen.line((x, 1.0), (x, 6.0), 0.35, CUT);
            }
            pen.line((-5.0, -8.0), (1.5, -2.0), 0.7, c);
            pen.line((5.0, -8.0), (-1.5, -2.0), 0.7, c);
        }
        25 => {
            // Maracas.
            for (dx, ang) in [(-2.5f32, -0.3f32), (2.5, 0.3)] {
                let pen = Pen { rot: ang, ..Pen { c: pen.pt(dx, 0.0), ..pen } };
                pen.dot(0.0, -3.0, 2.8, c);
                pen.rect(-0.5, -0.5, 0.5, 7.0, c);
            }
        }
        26 => {
            // Conga: a tall barrel with its head on top.
            pen.poly(&[(-4.0, -6.0), (4.0, -6.0), (4.6, 0.0), (3.0, 7.0), (-3.0, 7.0), (-4.6, 0.0)], c);
            pen.line((-4.0, -6.0), (4.0, -6.0), 0.6, CUT);
            pen.line((-4.5, -4.8), (4.5, -4.8), 0.4, CUT);
        }
        _ => headstock(&pen, 3, 3, 3.0, Top::Round, false),
    }
}
