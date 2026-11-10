//! The fretboard view at the bottom of the window.

use crate::model::*;
use egui::{Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, Ui, Vec2};

pub enum FretAction {
    None,
    Toggle(u8, u8),
}

/// Draws the neck. `marks` are (string, fret, colour) dots; returns a click.
pub fn fretboard(
    ui: &mut Ui,
    track: &Track,
    marks: &[(u8, u8, Color32)],
    cursor_string: usize,
) -> FretAction {
    let frets = track.frets.clamp(12, 30) as usize;
    let n = track.strings().max(1);
    let avail = ui.available_size();
    let h = avail.y.clamp(80.0, 190.0);
    let (resp, p) = ui.allocate_painter(Vec2::new(avail.x, h), Sense::click());
    let r = resp.rect;
    p.rect_filled(r, 4.0, Color32::from_rgb(34, 36, 40));
    let left = r.left() + 46.0;
    let right = r.right() - 14.0;
    let top = r.top() + 16.0;
    let bottom = r.bottom() - 20.0;
    let neck = Rect::from_min_max(Pos2::new(left, top - 6.0), Pos2::new(right, bottom + 6.0));
    // Wood.
    p.rect_filled(neck, 3.0, Color32::from_rgb(92, 58, 36));
    // Fret positions using a gently compressed scale so high frets stay usable.
    let scale = |f: f32| -> f32 {
        let full = 1.0 - 2f32.powf(-(frets as f32 + 1.0) / 17.0);
        let x = (1.0 - 2f32.powf(-f / 17.0)) / full;
        left + 26.0 + x * (right - left - 26.0)
    };
    let sy = |s: usize| top + s as f32 * (bottom - top) / (n as f32 - 1.0).max(1.0);
    // Nut.
    p.rect_filled(
        Rect::from_min_max(
            Pos2::new(left + 22.0, top - 6.0),
            Pos2::new(left + 27.0, bottom + 6.0),
        ),
        0.0,
        Color32::from_rgb(235, 225, 200),
    );
    for f in 1..=frets {
        let x = scale(f as f32);
        p.line_segment(
            [Pos2::new(x, top - 6.0), Pos2::new(x, bottom + 6.0)],
            Stroke::new(2.0, Color32::from_rgb(190, 190, 195)),
        );
        p.text(
            Pos2::new((x + scale(f as f32 - 1.0)) * 0.5, bottom + 14.0),
            Align2::CENTER_CENTER,
            format!("{f}"),
            FontId::proportional(10.0),
            Color32::from_gray(150),
        );
    }
    // Inlays.
    let mid = (top + bottom) * 0.5;
    for f in [3, 5, 7, 9, 15, 17, 19, 21] {
        if f <= frets {
            let x = (scale(f as f32) + scale(f as f32 - 1.0)) * 0.5;
            p.circle_filled(Pos2::new(x, mid), 4.5, Color32::from_rgb(230, 220, 200));
        }
    }
    for f in [12, 24] {
        if f <= frets {
            let x = (scale(f as f32) + scale(f as f32 - 1.0)) * 0.5;
            let d = (bottom - top) * 0.22;
            p.circle_filled(Pos2::new(x, mid - d), 4.5, Color32::from_rgb(230, 220, 200));
            p.circle_filled(Pos2::new(x, mid + d), 4.5, Color32::from_rgb(230, 220, 200));
        }
    }
    // Strings.
    for s in 0..n {
        let y = sy(s);
        let w = 1.0 + s as f32 * 0.35 * (6.0 / n as f32);
        let col = if s == cursor_string {
            Color32::from_rgb(255, 230, 150)
        } else {
            Color32::from_rgb(210, 205, 195)
        };
        p.line_segment(
            [Pos2::new(left + 22.0, y), Pos2::new(right, y)],
            Stroke::new(w, col),
        );
        let open = track.tuning.get(s).copied().unwrap_or(40) + track.capo_at(s);
        p.text(
            Pos2::new(r.left() + 22.0, y),
            Align2::CENTER_CENTER,
            note_name(open),
            FontId::proportional(12.0),
            if s == cursor_string {
                Color32::from_rgb(255, 210, 120)
            } else {
                Color32::from_gray(200)
            },
        );
    }
    let pos_of = |s: u8, f: u8| -> Pos2 {
        let x = if f == 0 {
            left + 14.0
        } else {
            (scale(f as f32) + scale(f as f32 - 1.0)) * 0.5
        };
        Pos2::new(x, sy(s as usize))
    };
    // Hover hint.
    if let Some(hp) = resp.hover_pos()
        && let Some((s, f)) = locate(hp, n, frets, &sy, &scale, left)
    {
        let c = pos_of(s, f);
        p.circle_stroke(c, 8.0, Stroke::new(1.5, Color32::from_white_alpha(140)));
        let pitch = track.tuning.get(s as usize).copied().unwrap_or(40) as u32
            + f as u32
            + track.capo_at(s as usize) as u32;
        p.text(
            Pos2::new(r.right() - 16.0, r.top() + 4.0),
            Align2::RIGHT_TOP,
            format!(
                "String {} · Fret {} · {}",
                s + 1,
                f,
                note_name_oct(pitch.min(127) as u8)
            ),
            FontId::proportional(11.0),
            Color32::from_gray(200),
        );
    }
    for &(s, f, col) in marks {
        if (s as usize) < n {
            let c = pos_of(s, f);
            p.circle_filled(c, 8.0, col);
            p.text(
                c,
                Align2::CENTER_CENTER,
                format!("{f}"),
                FontId::proportional(10.0),
                Color32::WHITE,
            );
        }
    }
    if resp.clicked()
        && let Some(hp) = resp.interact_pointer_pos()
        && let Some((s, f)) = locate(hp, n, frets, &sy, &scale, left)
    {
        return FretAction::Toggle(s, f);
    }
    FretAction::None
}

fn locate(
    hp: Pos2,
    n: usize,
    frets: usize,
    sy: &dyn Fn(usize) -> f32,
    scale: &dyn Fn(f32) -> f32,
    left: f32,
) -> Option<(u8, u8)> {
    let mut best_s = 0;
    let mut bd = f32::MAX;
    for s in 0..n {
        let d = (sy(s) - hp.y).abs();
        if d < bd {
            bd = d;
            best_s = s;
        }
    }
    if bd > 14.0 || hp.x < left {
        return None;
    }
    if hp.x < scale(0.0) + 1.0 {
        return Some((best_s as u8, 0));
    }
    for f in 1..=frets {
        if hp.x < scale(f as f32) {
            return Some((best_s as u8, f as u8));
        }
    }
    None
}
