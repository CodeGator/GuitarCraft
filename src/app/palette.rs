//! The edition palette on the left, laid out like the original's: voices,
//! lyrics and chords, then five sections of tools (bars, notes, effects,
//! notation, automations), eight to a row, in the user guide's order.

use super::icons::{glyph_button, note_icon_button, paint_button};
use super::popups::Popup;
use super::App;
use crate::model::*;
use egui::{
    Align2, Color32, FontFamily, FontId, Painter, Pos2, Rect, RichText, Sense, Shape, Stroke as PStroke,
    Ui, Vec2,
};

type NoteFlag = fn(&mut Note) -> &mut bool;

fn divider(ui: &mut Ui) {
    ui.add_space(4.0);
    let r = ui.available_rect_before_wrap();
    ui.painter().line_segment(
        [Pos2::new(r.left(), r.top()), Pos2::new(r.right(), r.top())],
        PStroke::new(1.0, Color32::from_rgb(90, 102, 130)),
    );
    ui.add_space(5.0);
}

fn row(ui: &mut Ui, f: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        f(ui);
    });
}

fn st(c: Color32, w: f32) -> PStroke {
    PStroke::new(w, c)
}

/// A sine-like wave from `a` to `b`.
fn wave(p: &Painter, a: Pos2, b: Pos2, amp: f32, turns: f32, c: Color32, w: f32) {
    let pts: Vec<Pos2> = (0..=24)
        .map(|i| {
            let t = i as f32 / 24.0;
            let d = b - a;
            let n = Vec2::new(-d.y, d.x).normalized();
            a + d * t + n * (t * turns * std::f32::consts::TAU).sin() * amp
        })
        .collect();
    p.add(Shape::line(pts, st(c, w)));
}

/// A zigzag from `a` to `b` with `n` points.
fn zigzag(p: &Painter, a: Pos2, b: Pos2, amp: f32, n: usize, c: Color32, w: f32) {
    let d = b - a;
    let nrm = Vec2::new(-d.y, d.x).normalized();
    let pts: Vec<Pos2> = (0..=n)
        .map(|i| a + d * (i as f32 / n as f32) + nrm * if i % 2 == 0 { amp } else { -amp })
        .collect();
    p.add(Shape::line(pts, st(c, w)));
}

fn arrow_head(p: &Painter, tip: Pos2, dir: Vec2, c: Color32) {
    let d = dir.normalized();
    let n = Vec2::new(-d.y, d.x);
    p.add(Shape::convex_polygon(vec![tip, tip - d * 4.0 + n * 2.6, tip - d * 4.0 - n * 2.6], c, PStroke::NONE));
}

fn small(p: &Painter, pos: Pos2, t: &str, size: f32, c: Color32) {
    p.text(pos, Align2::CENTER_CENTER, t, FontId::proportional(size), c);
}

/// A small notehead with a stem (up) and `flags` flags; `slash` crosses it (grace note).
fn tiny_note(p: &Painter, c: Pos2, col: Color32, flags: usize, slash: bool) {
    p.add(Shape::ellipse_filled(c, Vec2::new(2.6, 1.9), col));
    let top = c + Vec2::new(2.3, -10.0);
    p.line_segment([c + Vec2::new(2.3, 0.0), top], st(col, 1.0));
    for f in 0..flags {
        let y = f as f32 * 2.6;
        p.line_segment([top + Vec2::new(0.0, y), top + Vec2::new(3.5, y + 4.0)], st(col, 1.0));
    }
    if slash {
        p.line_segment([c + Vec2::new(-1.0, -3.0), c + Vec2::new(6.5, -9.0)], st(col, 1.0));
    }
}

/// Two eighth notes joined by a beam (`beam` false: separate flags).
fn beamed(p: &Painter, r: Rect, col: Color32, beam: bool, secondary_break: bool, label: Option<&str>) {
    let y = r.top() + if label.is_some() { 10.0 } else { 13.0 };
    let xs = [r.left() + 3.0, r.right() - 5.0];
    for &x in &xs {
        p.add(Shape::ellipse_filled(Pos2::new(x, y), Vec2::new(2.6, 1.9), col));
        p.line_segment([Pos2::new(x + 2.3, y), Pos2::new(x + 2.3, r.top())], st(col, 1.0));
    }
    if beam {
        p.rect_filled(Rect::from_min_max(Pos2::new(xs[0] + 2.3, r.top()), Pos2::new(xs[1] + 2.8, r.top() + 2.0)), 0.0, col);
        if secondary_break {
            for &x in &xs {
                p.line_segment([Pos2::new(x + 2.3, r.top() + 4.0), Pos2::new(x + 6.0, r.top() + 4.0)], st(col, 1.6));
            }
        }
    } else {
        for &x in &xs {
            p.line_segment([Pos2::new(x + 2.3, r.top()), Pos2::new(x + 5.5, r.top() + 4.0)], st(col, 1.0));
        }
    }
    if let Some(t) = label {
        small(p, Pos2::new(r.center().x, r.bottom() - 1.0), t, 6.0, col);
    }
}

/// Bars for the volume and pan automation icons; `master` adds an "M".
fn level_bars(p: &Painter, r: Rect, col: Color32, master: bool, pan: bool) {
    let n = if pan { 6 } else { 4 };
    for i in 0..n {
        let x = r.left() + i as f32 * (r.width() / n as f32);
        let h = if pan { [0.5, 0.35, 0.8, 0.35, 0.6, 0.9][i] } else { 0.3 + 0.2 * i as f32 } * r.height() * if master { 0.75 } else { 1.0 };
        p.rect_filled(Rect::from_min_max(Pos2::new(x, r.bottom() - h), Pos2::new(x + r.width() / n as f32 - 1.2, r.bottom())), 0.0, col);
    }
    if master {
        p.text(Pos2::new(r.right(), r.top()), Align2::RIGHT_TOP, "M", FontId::new(8.0, FontFamily::Name("bold".into())), col);
    }
}

impl App {
    pub(super) fn palette(&mut self, ui: &mut Ui) {
        let cur = self.cur_beat().clone();
        let mb = self.song.master[self.bar].clone();
        let simile = self.song.tracks[self.track].bars[self.bar].simile;
        let n = cur.note_on(self.string as u8).or(cur.notes.first()).cloned().unwrap_or_default();
        let has = !cur.is_rest();
        let lh_names: [&str; 5] = if self.song.style.lh_type == 1 { ["P", "1", "2", "3", "4"] } else { ["T", "1", "2", "3", "4"] };
        let rh_names: [&str; 5] = match self.song.style.rh_type {
            1 => ["p", "i", "m", "a", "x"],
            2 => ["p", "i", "m", "a", "e"],
            3 => ["t", "i", "m", "a", "o"],
            _ => ["p", "i", "m", "a", "c"],
        };
        ui.spacing_mut().item_spacing = Vec2::new(2.0, 2.0);
        let sym = |s: f32| FontId::new(s, FontFamily::Name("music".into()));
        let bolds = |s: f32| FontId::new(s, FontFamily::Name("bold".into()));
        let ital = |s: f32| FontId::new(s, FontFamily::Name("serif-bi".into()));
        let serif = |s: f32| FontId::new(s, FontFamily::Name("serif-bold".into()));

        // ---- Voices, the voice tool, lyrics and chords.
        row(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 3.0;
            for (v, &dot) in super::side::VOICE_COLORS.iter().enumerate() {
                let used = self.song.tracks[self.track].bars[self.bar].voice_used(v);
                let on = self.voice == v;
                let (r, resp) = ui.allocate_exact_size(Vec2::new(24.0, 24.0), Sense::click());
                let fill = if on {
                    Color32::from_rgb(240, 160, 50)
                } else if resp.hovered() {
                    Color32::from_gray(80)
                } else {
                    Color32::from_gray(62)
                };
                ui.painter().rect_filled(r, 4.0, fill);
                ui.painter().text(r.center(), Align2::CENTER_CENTER, format!("{}", v + 1), bolds(13.0), if on { Color32::BLACK } else { Color32::from_gray(230) });
                if used && !on {
                    ui.painter().circle_filled(Pos2::new(r.right() - 4.0, r.top() + 4.0), 2.5, dot);
                }
                if resp.on_hover_text(format!("Edit voice {} (Ctrl+{})", v + 1, v + 1)).clicked() {
                    self.set_voice(v);
                }
            }
            if paint_button(ui, self.voices_open, "Move or swap voices\u{2026}", |p, r, c| {
                for i in 0..4 {
                    let y = r.top() + 3.0 + i as f32 * 3.5;
                    p.line_segment([Pos2::new(r.left(), y), Pos2::new(r.right(), y)], st(c, 0.8));
                }
                tiny_note(p, Pos2::new(r.left() + 5.0, r.bottom() - 3.0), c, 1, false);
                tiny_note(p, Pos2::new(r.right() - 6.0, r.bottom() - 6.0), c, 0, false);
            })
            .clicked()
            {
                self.voices_open = true;
            }
        });
        ui.add_space(2.0);
        row(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            ui.add_space(14.0);
            if ui.add(egui::Button::new(RichText::new("\u{1F3A4} Lyrics").size(12.0)).min_size(Vec2::new(74.0, 22.0))).on_hover_text("Lyrics editor").clicked() {
                self.lyrics_open = true;
            }
            if ui.add(egui::Button::selectable(self.chord_lib_open, RichText::new("\u{25A6} Chords").size(12.0)).frame_when_inactive(true).min_size(Vec2::new(78.0, 22.0))).on_hover_text("The song's chord diagram library").clicked() {
                self.chord_lib_open = !self.chord_lib_open;
            }
        });
        divider(ui);

        // ---- Bars.
        row(ui, |ui| {
            ui.add_enabled_ui(false, |ui| {
                glyph_button(ui, false, "\u{1D11E}", sym(17.0), "Clef (GuitarCraft takes the clef from the instrument)");
            });
            let ks = match mb.key {
                k if k > 0 => "\u{266F}".repeat(k.min(3) as usize),
                k if k < 0 => "\u{266D}".repeat((-k).min(3) as usize),
                _ => "\u{266F}\u{266F}\u{266F}".into(),
            };
            if glyph_button(ui, mb.key != 0, &ks, sym(11.0), "Key signature\u{2026} (Ctrl+K)").clicked() {
                self.open_popup(Popup::KeySig);
            }
            if glyph_button(ui, false, &format!("{}\n{}", mb.num, mb.den), bolds(9.5), "Time signature\u{2026} (Ctrl+T)").clicked() {
                self.open_popup(Popup::TimeSig);
            }
            if glyph_button(ui, mb.triplet_feel, "\u{266B}\u{00B3}", sym(14.0), "Triplet feel (swing eighths) from this bar").clicked() {
                self.bar_edit(|m| m.triplet_feel = !m.triplet_feel);
            }
            if paint_button(ui, mb.free_time, "Free time: played without following the tempo", |p, r, c| {
                let x = r.center().x;
                let mut y = r.top();
                while y < r.bottom() {
                    p.line_segment([Pos2::new(x, y), Pos2::new(x, (y + 2.0).min(r.bottom()))], st(c, 1.4));
                    y += 3.6;
                }
            })
            .clicked()
            {
                self.bar_edit(|m| m.free_time = !m.free_time);
            }
            if glyph_button(ui, mb.double_bar, "\u{2016}", FontId::proportional(18.0), "Double bar line").clicked() {
                self.bar_edit(|m| m.double_bar = !m.double_bar);
            }
            for (v, tip) in [(1u8, "Repeat the previous bar"), (2, "Repeat the previous two bars")] {
                if paint_button(ui, simile == v, tip, |p, r, c| {
                    let cy = r.center().y;
                    for k in 0..v {
                        let dx = k as f32 * 5.0 - (v as f32 - 1.0) * 2.5;
                        p.line_segment([Pos2::new(r.center().x - 4.0 + dx, cy + 5.0), Pos2::new(r.center().x + 4.0 + dx, cy - 5.0)], st(c, 2.2));
                    }
                    p.circle_filled(Pos2::new(r.left() + 2.5, cy - 2.5), 1.3, c);
                    p.circle_filled(Pos2::new(r.right() - 2.5, cy + 2.5), 1.3, c);
                })
                .clicked()
                {
                    self.edit(move |s, t, b, _, _| {
                        let bar = &mut s.tracks[t].bars[b];
                        bar.simile = if bar.simile == v { 0 } else { v };
                    });
                }
            }
        });
        row(ui, |ui| {
            if glyph_button(ui, mb.repeat_open, "\u{1D106}", sym(20.0), "Repeat start").clicked() {
                self.bar_edit(|m| m.repeat_open = !m.repeat_open);
            }
            let el = match mb.endings {
                0 => "1.".to_string(),
                e => (0..8).filter(|k| e & (1 << k) != 0).map(|k| format!("{}.", k + 1)).collect::<Vec<_>>().join(""),
            };
            if glyph_button(ui, mb.endings != 0, &format!("\u{231C}{el}"), serif(11.0), "Alternate ending: click to cycle 1st, 2nd, 3rd, 4th time, off").clicked() {
                self.bar_edit(|m| m.endings = match m.endings {
                    0 => 1,
                    1 => 2,
                    2 => 4,
                    4 => 8,
                    _ => 0,
                });
            }
            if glyph_button(ui, mb.repeat_close > 0, "\u{1D107}", sym(20.0), "Repeat close (plays twice)").clicked() {
                self.bar_edit(|m| m.repeat_close = if m.repeat_close > 0 { 0 } else { 2 });
            }
            if paint_button(ui, mb.direction_target != 0 || mb.direction_jump != 0, "Directions\u{2026} (Coda, Segno, Fine and jumps)", |p, r, c| {
                let ctr = r.center();
                p.circle_stroke(ctr, 5.0, st(c, 1.6));
                p.line_segment([Pos2::new(ctr.x, r.top()), Pos2::new(ctr.x, r.bottom())], st(c, 1.4));
                p.line_segment([Pos2::new(r.left() + 1.0, ctr.y), Pos2::new(r.right() - 1.0, ctr.y)], st(c, 1.4));
            })
            .clicked()
            {
                self.dir_open = true;
            }
            for (v, lab, tip) in [
                (1i8, "8va", "8va: play an octave higher"),
                (-1, "8vb", "8vb: play an octave lower"),
                (2, "15ma", "15ma: play two octaves higher"),
                (-2, "15mb", "15mb: play two octaves lower"),
            ] {
                if glyph_button(ui, cur.ottava == v, lab, ital(10.5), tip).clicked() {
                    self.beat_edit(|b| b.ottava = if b.ottava == v { 0 } else { v });
                }
            }
        });
        divider(ui);

        // ---- Notes: durations, rest.
        row(ui, |ui| {
            for d in Dur::ALL {
                if note_icon_button(ui, d, cur.dur == d).on_hover_text(format!("{} (+ / \u{2212} change it)", d.name())).clicked() {
                    self.set_duration(d);
                }
            }
            if glyph_button(ui, cur.is_rest(), "\u{1D13D}", sym(20.0), "Rest (R)").clicked() && !cur.is_rest() {
                self.key(egui::Key::R, egui::Modifiers::NONE, 0.0);
            }
        });
        // Dots, tuplets, ties, fermata.
        row(ui, |ui| {
            if glyph_button(ui, cur.dotted && !cur.double_dot, "\u{2022}", FontId::proportional(13.0), "Dotted note (.)").clicked() {
                self.beat_edit(|b| {
                    b.dotted = !(b.dotted && !b.double_dot);
                    b.double_dot = false;
                });
            }
            if glyph_button(ui, cur.double_dot, "\u{2022}\u{2022}", FontId::proportional(13.0), "Double-dotted note").clicked() {
                self.beat_edit(|b| {
                    b.double_dot = !b.double_dot;
                    b.dotted = false;
                });
            }
            if glyph_button(ui, cur.tuplet == 3, "\u{231C}3\u{231D}", bolds(10.0), "Triplet (/)").clicked() {
                self.beat_edit(|b| b.tuplet = if b.tuplet == 3 { 0 } else { 3 });
            }
            let resp = glyph_button(ui, cur.tuplet > 0 && cur.tuplet != 3, "n:m", ital(10.5), "Custom n-tuplet: choose 5, 6, 7, 9, 10, 11 or 12 notes");
            egui::Popup::menu(&resp).show(|ui| {
                for k in [5u8, 6, 7, 9, 10, 11, 12] {
                    if ui.selectable_label(cur.tuplet == k, format!("{k} notes")).clicked() {
                        self.beat_edit(|b| b.tuplet = if b.tuplet == k { 0 } else { k });
                    }
                }
                if ui.button("None").clicked() {
                    self.beat_edit(|b| b.tuplet = 0);
                }
            });
            ui.add_enabled_ui(false, |ui| {
                glyph_button(ui, false, "\u{231C}n:m\u{231D}", ital(8.5), "Nested n-tuplet (not in GuitarCraft yet)");
            });
            if glyph_button(ui, n.tie, "\u{203F}", FontId::proportional(18.0), "Tie to the previous note (I)").clicked() {
                self.toggle_note_flag("Tie", |n| &mut n.tie);
            }
            let all_tied = has && cur.notes.iter().all(|x| x.tie);
            if paint_button(ui, all_tied, "Multiple ties: tie every note of the beat to the previous beat", |p, r, c| {
                for k in 0..3 {
                    let y = r.top() + 5.0 + k as f32 * 4.5;
                    let pts = [Pos2::new(r.left() + 1.0, y), Pos2::new(r.center().x, y + 3.0), Pos2::new(r.right() - 1.0, y)];
                    p.add(egui::epaint::QuadraticBezierShape::from_points_stroke(pts, false, Color32::TRANSPARENT, st(c, 1.1)));
                }
            })
            .clicked()
                && has
            {
                self.beat_edit(move |b| {
                    for x in &mut b.notes {
                        x.tie = !all_tied;
                    }
                });
            }
            if paint_button(ui, cur.fermata, "Fermata (hold)", |p, r, c| {
                let ctr = Pos2::new(r.center().x, r.bottom() - 4.0);
                let pts: Vec<Pos2> = (0..=12)
                    .map(|i| {
                        let a = std::f32::consts::PI * i as f32 / 12.0;
                        Pos2::new(ctr.x - a.cos() * 7.0, ctr.y - a.sin() * 7.0)
                    })
                    .collect();
                p.add(Shape::line(pts, st(c, 1.6)));
                p.circle_filled(Pos2::new(ctr.x, ctr.y - 1.0), 1.6, c);
            })
            .clicked()
            {
                self.beat_edit(|b| b.fermata = !b.fermata);
            }
        });
        // Accidentals and semitone steps.
        ui.add_enabled_ui(has, |ui| {
            row(ui, |ui| {
                ui.add_enabled_ui(false, |ui| {
                    glyph_button(ui, false, "\u{1D12B}", sym(16.0), "Double flat (not in GuitarCraft yet)");
                });
                if glyph_button(ui, has && n.spell == 2, "\u{266D}", sym(16.0), "Flat: write the note with a flat").clicked() {
                    self.note_edit(|x| x.spell = 2);
                }
                if glyph_button(ui, has && n.spell == 0, "\u{266E}", sym(16.0), "Natural: spell the note from the key signature").clicked() {
                    self.note_edit(|x| x.spell = 0);
                }
                if glyph_button(ui, has && n.spell == 1, "\u{266F}", sym(16.0), "Sharp: write the note with a sharp").clicked() {
                    self.note_edit(|x| x.spell = 1);
                }
                ui.add_enabled_ui(false, |ui| {
                    glyph_button(ui, false, "\u{1D12A}", sym(15.0), "Double sharp (not in GuitarCraft yet)");
                });
                if glyph_button(ui, false, "\u{266D}\u{266F}", sym(13.0), "Change accidental: write the same note the other way (F\u{266F} \u{2194} G\u{266D})").clicked() {
                    self.note_edit(|x| x.spell = if x.spell == 2 { 1 } else { 2 });
                }
                if paint_button(ui, false, "Lower by a semitone (Ctrl+\u{2193})", |p, r, c| {
                    tiny_note(p, Pos2::new(r.center().x + 1.0, r.top() + 11.0), c, 0, false);
                    p.line_segment([Pos2::new(r.left() + 2.0, r.top()), Pos2::new(r.left() + 2.0, r.bottom() - 2.0)], st(c, 1.2));
                    arrow_head(p, Pos2::new(r.left() + 2.0, r.bottom()), Vec2::new(0.0, 1.0), c);
                })
                .clicked()
                {
                    self.transpose(-1);
                }
                if paint_button(ui, false, "Raise by a semitone (Ctrl+\u{2191})", |p, r, c| {
                    tiny_note(p, Pos2::new(r.left() + 4.0, r.bottom() - 2.0), c, 0, false);
                    p.line_segment([Pos2::new(r.right() - 2.0, r.bottom()), Pos2::new(r.right() - 2.0, r.top() + 2.0)], st(c, 1.2));
                    arrow_head(p, Pos2::new(r.right() - 2.0, r.top()), Vec2::new(0.0, -1.0), c);
                })
                .clicked()
                {
                    self.transpose(1);
                }
            });
        });
        // Dynamics and hairpins.
        row(ui, |ui| {
            for d in Dynamic::ALL {
                if glyph_button(ui, cur.dynamic == d && has, d.label(), ital(13.0), "Loudness from this beat on").clicked() {
                    self.beat_edit(|b| b.dynamic = d);
                }
            }
        });
        row(ui, |ui| {
            for (v, tip) in [(1u8, "Crescendo (getting louder)"), (2, "Decrescendo (getting softer)")] {
                if paint_button(ui, cur.hairpin == v, tip, |p, r, c| {
                    let (open, tip_x) = if v == 1 { (r.right(), r.left()) } else { (r.left(), r.right()) };
                    let y = r.center().y;
                    p.line_segment([Pos2::new(tip_x, y), Pos2::new(open, y - 4.0)], st(c, 1.2));
                    p.line_segment([Pos2::new(tip_x, y), Pos2::new(open, y + 4.0)], st(c, 1.2));
                })
                .clicked()
                {
                    self.beat_edit(|b| b.hairpin = if b.hairpin == v { 0 } else { v });
                }
            }
        });
        divider(ui);

        // ---- Effects.
        ui.add_enabled_ui(has, |ui| {
            let flags: [(&str, FontId, &str, bool, NoteFlag); 4] = [
                ("( )", FontId::proportional(13.0), "Ghost note (G)", n.ghost, |n| &mut n.ghost),
                (">", bolds(15.0), "Accented note (A)", n.accent, |n| &mut n.accent),
                ("^", bolds(15.0), "Heavily accented note", n.heavy_accent, |n| &mut n.heavy_accent),
                ("\u{2022}", FontId::proportional(9.0), "Staccato", n.staccato, |n| &mut n.staccato),
            ];
            row(ui, |ui| {
                for (lab, font, tip, on, f) in flags.iter() {
                    if glyph_button(ui, has && *on, lab, font.clone(), tip).clicked() {
                        self.toggle_note_flag(tip, *f);
                    }
                }
                if paint_button(ui, has && n.fx.legato, "Legato: slur to the next note, played without a new attack", |p, r, c| {
                    tiny_note(p, Pos2::new(r.left() + 3.0, r.bottom() - 2.0), c, 0, false);
                    tiny_note(p, Pos2::new(r.right() - 6.0, r.bottom() - 4.0), c, 0, false);
                    let pts = [Pos2::new(r.left() + 3.0, r.top() + 2.0), Pos2::new(r.center().x, r.top() - 3.0), Pos2::new(r.right() - 2.0, r.top() + 1.0)];
                    p.add(egui::epaint::QuadraticBezierShape::from_points_stroke(pts, false, Color32::TRANSPARENT, st(c, 1.1)));
                })
                .clicked()
                {
                    self.toggle_note_flag("Legato", |n| &mut n.fx.legato);
                }
                if glyph_button(ui, has && n.let_ring, "let\nring", FontId::new(8.5, FontFamily::Name("italic".into())), "Let ring (L)").clicked() {
                    self.toggle_note_flag("Let ring", |n| &mut n.let_ring);
                }
                let ped = ["Ped.", "Ped.", "\u{2731}"][cur.fx.pedal.min(2) as usize];
                if glyph_button(ui, cur.fx.pedal != 0, ped, ital(10.0), "Sustain pedal: click for pressed (Ped.), again for released (*), again for none").clicked() {
                    self.beat_edit(|b| b.fx.pedal = (b.fx.pedal + 1) % 3);
                }
                if glyph_button(ui, has && n.palm_mute, "P.M.", FontId::new(10.0, FontFamily::Name("italic".into())), "Palm mute (P)").clicked() {
                    self.toggle_note_flag("Palm mute", |n| &mut n.palm_mute);
                }
            });
            row(ui, |ui| {
                if glyph_button(ui, has && n.dead, "\u{2716}", FontId::proportional(12.0), "Dead note (X)").clicked() {
                    self.toggle_note_flag("Dead note", |n| &mut n.dead);
                }
                if glyph_button(ui, cur.fx.dead_slap, "X", bolds(17.0), "Dead slap (bass)").clicked() {
                    self.beat_edit(|b| b.fx.dead_slap = !b.fx.dead_slap);
                }
                if glyph_button(ui, has && n.harmonic, "\u{25C7}", sym(15.0), "Natural harmonic (O)").clicked() {
                    self.toggle_note_flag("Natural harmonic", |n| &mut n.harmonic);
                }
                let ah_on = has && (n.pinch_harmonic || n.fx.art_harmonic != 0);
                let resp = glyph_button(ui, ah_on, "\u{25C6}", sym(15.0), "Artificial harmonic: A.H., T.H., P.H. or S.H.");
                egui::Popup::menu(&resp).show(|ui| {
                    for (k, lab) in [(1u8, "A.H. \u{2013} artificial"), (2, "T.H. \u{2013} tapped"), (9, "P.H. \u{2013} pinch"), (3, "S.H. \u{2013} semi")] {
                        let on = if k == 9 { n.pinch_harmonic } else { n.fx.art_harmonic == k };
                        if ui.selectable_label(on, lab).clicked() {
                            self.note_edit(move |x| {
                                let was = if k == 9 { x.pinch_harmonic } else { x.fx.art_harmonic == k };
                                x.pinch_harmonic = false;
                                x.fx.art_harmonic = 0;
                                if !was {
                                    if k == 9 {
                                        x.pinch_harmonic = true;
                                    } else {
                                        x.fx.art_harmonic = k;
                                    }
                                }
                            });
                        }
                    }
                });
                if paint_button(ui, has && n.bend > 0, "Bend\u{2026} (B)", |p, r, c| {
                    let pts = [Pos2::new(r.left() + 1.0, r.bottom() - 1.0), Pos2::new(r.right() - 3.0, r.bottom() - 1.0), Pos2::new(r.right() - 3.0, r.top() + 4.0)];
                    p.add(egui::epaint::QuadraticBezierShape::from_points_stroke(pts, false, Color32::TRANSPARENT, st(c, 1.3)));
                    arrow_head(p, Pos2::new(r.right() - 3.0, r.top()), Vec2::new(0.0, -1.0), c);
                })
                .clicked()
                {
                    self.open_popup(Popup::Bend);
                }
                if paint_button(ui, cur.fx.whammy, "Whammy bar", |p, r, c| {
                    p.line_segment([Pos2::new(r.left() + 1.0, r.top() + 7.0), Pos2::new(r.center().x, r.bottom())], st(c, 1.3));
                    p.line_segment([Pos2::new(r.center().x, r.bottom()), Pos2::new(r.right() - 1.0, r.top() + 7.0)], st(c, 1.3));
                    small(p, Pos2::new(r.center().x, r.top() + 3.0), "x", 10.0, c);
                })
                .clicked()
                {
                    self.beat_edit(|b| b.fx.whammy = !b.fx.whammy);
                }
                if paint_button(ui, has && (n.vibrato || n.wide_vibrato), "Left-hand vibrato: click for slight, again for wide, again for none (V)", |p, r, c| {
                    wave(p, Pos2::new(r.left(), r.center().y), Pos2::new(r.right(), r.center().y), if n.wide_vibrato { 3.0 } else { 2.0 }, 2.5, c, 1.4);
                })
                .clicked()
                {
                    self.note_edit(|x| {
                        let (v, w) = match (x.vibrato, x.wide_vibrato) {
                            (false, false) => (true, false),
                            (true, false) => (false, true),
                            _ => (false, false),
                        };
                        x.vibrato = v;
                        x.wide_vibrato = w;
                    });
                }
                if paint_button(ui, cur.fx.bridge_vibrato != 0, "Bridge vibrato: click for slight, again for wide, again for none", |p, r, c| {
                    zigzag(p, Pos2::new(r.left(), r.center().y), Pos2::new(r.right(), r.center().y), if cur.fx.bridge_vibrato == 2 { 4.0 } else { 3.0 }, 6, c, 1.3);
                })
                .clicked()
                {
                    self.beat_edit(|b| b.fx.bridge_vibrato = (b.fx.bridge_vibrato + 1) % 3);
                }
            });
            row(ui, |ui| {
                for (v, tip) in [(1u8, "Pick scrape upwards"), (2u8, "Pick scrape downwards")] {
                    if paint_button(ui, cur.fx.scrape == v, tip, |p, r, c| {
                        let (a, b) = if v == 1 { (r.left_bottom() + Vec2::new(3.0, -2.0), r.right_top() + Vec2::new(-1.0, 4.0)) } else { (r.left_top() + Vec2::new(3.0, 4.0), r.right_bottom() + Vec2::new(-1.0, -2.0)) };
                        wave(p, a, b, 1.6, 2.5, c, 1.2);
                        small(p, if v == 1 { r.left_bottom() + Vec2::new(2.0, -2.0) } else { r.left_top() + Vec2::new(2.0, 2.0) }, "x", 9.0, c);
                    })
                    .clicked()
                    {
                        self.beat_edit(|b| b.fx.scrape = if b.fx.scrape == v { 0 } else { v });
                    }
                }
                // Slides: legato, shift, in from below/above, out downwards/upwards.
                let slides: [(&str, bool, u8); 6] = [
                    ("Legato slide to the next note (S)", n.slide, 0),
                    ("Shift slide to the next note (attacked again)", n.fx.shift_slide, 1),
                    ("Slide in from below", n.slide_in, 2),
                    ("Slide in from above", n.fx.slide_in_above, 3),
                    ("Slide out downwards", n.slide_out, 4),
                    ("Slide out upwards", n.fx.slide_out_up, 5),
                ];
                for (tip, on, k) in slides {
                    if paint_button(ui, has && on, tip, |p, r, c| {
                        let y = r.center().y + 2.0;
                        let (l, rr) = (r.left(), r.right());
                        match k {
                            0 | 1 => {
                                small(p, Pos2::new(l + 2.0, y), "1", 8.0, c);
                                small(p, Pos2::new(rr - 2.0, y), "3", 8.0, c);
                                p.line_segment([Pos2::new(l + 5.0, y + 2.0), Pos2::new(rr - 5.0, y - 2.0)], st(c, 1.1));
                                if k == 0 {
                                    let pts = [Pos2::new(l + 2.0, y - 6.0), Pos2::new(r.center().x, y - 11.0), Pos2::new(rr - 2.0, y - 6.0)];
                                    p.add(egui::epaint::QuadraticBezierShape::from_points_stroke(pts, false, Color32::TRANSPARENT, st(c, 1.0)));
                                }
                            }
                            2 | 3 => {
                                small(p, Pos2::new(rr - 2.0, y), "3", 8.0, c);
                                let (a, b) = if k == 2 { (Pos2::new(l, y + 4.0), Pos2::new(rr - 6.0, y)) } else { (Pos2::new(l, y - 4.0), Pos2::new(rr - 6.0, y)) };
                                p.line_segment([a, b], st(c, 1.1));
                            }
                            _ => {
                                small(p, Pos2::new(l + 2.0, y), "3", 8.0, c);
                                let (a, b) = if k == 4 { (Pos2::new(l + 6.0, y), Pos2::new(rr, y + 4.0)) } else { (Pos2::new(l + 6.0, y), Pos2::new(rr, y - 4.0)) };
                                p.line_segment([a, b], st(c, 1.1));
                            }
                        }
                    })
                    .clicked()
                    {
                        match k {
                            0 => self.toggle_note_flag("Legato slide", |n| &mut n.slide),
                            1 => self.toggle_note_flag("Shift slide", |n| &mut n.fx.shift_slide),
                            2 => self.toggle_note_flag("Slide in from below", |n| &mut n.slide_in),
                            3 => self.toggle_note_flag("Slide in from above", |n| &mut n.fx.slide_in_above),
                            4 => self.toggle_note_flag("Slide out downwards", |n| &mut n.slide_out),
                            _ => self.toggle_note_flag("Slide out upwards", |n| &mut n.fx.slide_out_up),
                        }
                    }
                }
            });
            row(ui, |ui| {
                if glyph_button(ui, has && n.fx.lh_tap, "L.H.", ital(10.0), "Left-hand tapping: the note sounds without being picked").clicked() {
                    self.toggle_note_flag("Left-hand tapping", |n| &mut n.fx.lh_tap);
                }
                if paint_button(ui, has && n.hammer, "Hammer-on / pull-off (H)", |p, r, c| {
                    small(p, Pos2::new(r.center().x, r.top() + 3.0), "H/P", 8.5, c);
                    let pts = [Pos2::new(r.left() + 1.0, r.bottom() - 4.0), Pos2::new(r.center().x, r.top() + 6.0), Pos2::new(r.right() - 1.0, r.bottom() - 4.0)];
                    p.add(egui::epaint::QuadraticBezierShape::from_points_stroke(pts, false, Color32::TRANSPARENT, st(c, 1.1)));
                })
                .clicked()
                {
                    self.toggle_note_flag("Hammer-on / pull-off", |n| &mut n.hammer);
                }
                for (v, lab, tip) in [(1u8, "tap", "Tapping"), (2, "slap", "Slap"), (3, "pop", "Pop")] {
                    if glyph_button(ui, cur.technique == v, lab, ital(11.0), tip).clicked() {
                        self.beat_edit(|b| b.technique = if b.technique == v { 0 } else { v });
                    }
                }
                for (v, lab, tip) in [(1u8, "\u{270B}", "Finger golpe: strike the body with a finger"), (2, "\u{270B}T", "Thumb golpe: strike the body with the thumb")] {
                    if glyph_button(ui, cur.fx.golpe == v, lab, sym(13.0), tip).clicked() {
                        self.beat_edit(|b| b.fx.golpe = if b.fx.golpe == v { 0 } else { v });
                    }
                }
                let sn = char::from_u32(0x2460 + n.string as u32).map(|c| c.to_string()).unwrap_or_else(|| "\u{2465}".into());
                if glyph_button(ui, has && n.fx.string_number, &sn, sym(15.0), "String number shown on the notation").clicked() {
                    self.toggle_note_flag("String number", |n| &mut n.fx.string_number);
                }
            });
            row(ui, |ui| {
                for (s, lab, tip) in [(Stroke::Down, "\u{2191}", "Downward brush (low string to high)"), (Stroke::Up, "\u{2193}", "Upward brush (high string to low)")] {
                    if glyph_button(ui, cur.stroke == s, lab, FontId::proportional(16.0), tip).clicked() {
                        let v = if cur.stroke == s { Stroke::None } else { s };
                        self.beat_edit(|b| b.stroke = v);
                    }
                }
                for (v, tip) in [(1u8, "Downward arpeggio (low string to high)"), (2u8, "Upward arpeggio (high string to low)")] {
                    if paint_button(ui, cur.fx.arpeggio == v, tip, |p, r, c| {
                        let x = r.center().x;
                        let (a, b) = if v == 1 { (Pos2::new(x, r.bottom()), Pos2::new(x, r.top() + 4.0)) } else { (Pos2::new(x, r.top()), Pos2::new(x, r.bottom() - 4.0)) };
                        wave(p, a, b, 1.6, 3.0, c, 1.1);
                        arrow_head(p, if v == 1 { Pos2::new(x, r.top()) } else { Pos2::new(x, r.bottom()) }, if v == 1 { Vec2::new(0.0, -1.0) } else { Vec2::new(0.0, 1.0) }, c);
                    })
                    .clicked()
                    {
                        self.beat_edit(|b| b.fx.arpeggio = if b.fx.arpeggio == v { 0 } else { v });
                    }
                }
                if glyph_button(ui, cur.fx.rasgueado, "rasg.", ital(10.0), "Rasgueado").clicked() {
                    self.beat_edit(|b| b.fx.rasgueado = !b.fx.rasgueado);
                }
                for (v, lab, tip) in [(1u8, "\u{2293}", "Downward pickstroke"), (2, "V", "Upward pickstroke")] {
                    if glyph_button(ui, cur.pick == v, lab, if v == 1 { sym(16.0) } else { FontId::proportional(14.0) }, tip).clicked() {
                        self.beat_edit(|b| b.pick = if b.pick == v { 0 } else { v });
                    }
                }
            });
            row(ui, |ui| {
                for (on_beat, tip) in [(false, "Grace note before the beat"), (true, "Grace note on the beat")] {
                    let on = has && n.grace.is_some() && cur.fx.grace_on_beat == on_beat;
                    if paint_button(ui, on, tip, |p, r, c| tiny_note(p, Pos2::new(r.center().x - 2.0, r.bottom() - 3.0), c, 1, !on_beat)).clicked() {
                        if on {
                            self.note_edit(|x| x.grace = None);
                        } else {
                            self.note_edit(|x| {
                                if x.grace.is_none() {
                                    x.grace = Some(x.fret.saturating_sub(2));
                                }
                            });
                            self.beat_edit(move |b| b.fx.grace_on_beat = on_beat);
                        }
                    }
                }
                if glyph_button(ui, has && n.trill > 0, "tr.", ital(13.0), "Trill\u{2026}").clicked() {
                    self.open_popup(Popup::Trill);
                }
                if glyph_button(ui, cur.tremolo, "\u{2AFD}", sym(15.0), "Tremolo picking").clicked() {
                    self.beat_edit(|b| b.tremolo = !b.tremolo);
                }
                for (v, tip) in [(1u8, "Upper mordent"), (2, "Lower mordent"), (3, "Turn"), (4, "Inverted turn")] {
                    if paint_button(ui, has && n.fx.ornament == v, tip, |p, r, c| {
                        let y = r.center().y;
                        match v {
                            1 | 2 => {
                                zigzag(p, Pos2::new(r.left(), y), Pos2::new(r.right(), y), 3.0, 4, c, 1.5);
                                if v == 2 {
                                    p.line_segment([Pos2::new(r.center().x, y - 6.0), Pos2::new(r.center().x, y + 6.0)], st(c, 1.1));
                                }
                            }
                            _ => {
                                let s = if v == 3 { 1.0 } else { -1.0 };
                                let pts: Vec<Pos2> = (0..=20)
                                    .map(|i| {
                                        let t = i as f32 / 20.0;
                                        Pos2::new(r.left() + t * r.width(), y - s * (t * std::f32::consts::TAU).sin() * 3.5)
                                    })
                                    .collect();
                                p.add(Shape::line(pts, st(c, 1.6)));
                            }
                        }
                    })
                    .clicked()
                    {
                        self.note_edit(move |x| x.fx.ornament = if x.fx.ornament == v { 0 } else { v });
                    }
                }
            });
            row(ui, |ui| {
                for (k, tip) in [(0u8, "Fade in"), (1, "Fade out"), (2, "Volume swell (fade in, then out)")] {
                    let on = match k {
                        0 => cur.fade_in,
                        1 => cur.fx.fade_out,
                        _ => cur.fx.swell,
                    };
                    if paint_button(ui, on, tip, |p, r, c| {
                        let y = r.center().y;
                        let w = if k == 2 { r.width() * 0.5 } else { r.width() };
                        let wedge = |x0: f32, x1: f32| {
                            p.line_segment([Pos2::new(x0, y), Pos2::new(x1, y - 4.0)], st(c, 1.2));
                            p.line_segment([Pos2::new(x0, y), Pos2::new(x1, y + 4.0)], st(c, 1.2));
                        };
                        match k {
                            0 => wedge(r.left(), r.left() + w),
                            1 => wedge(r.right(), r.right() - w),
                            _ => {
                                wedge(r.left(), r.center().x);
                                wedge(r.right(), r.center().x);
                            }
                        }
                    })
                    .clicked()
                    {
                        self.beat_edit(move |b| {
                            let was = match k {
                                0 => b.fade_in,
                                1 => b.fx.fade_out,
                                _ => b.fx.swell,
                            };
                            b.fade_in = false;
                            b.fx.fade_out = false;
                            b.fx.swell = false;
                            match k {
                                0 => b.fade_in = !was,
                                1 => b.fx.fade_out = !was,
                                _ => b.fx.swell = !was,
                            }
                        });
                    }
                }
                for (v, mark, tip) in [(1u8, "o", "Wah-wah open"), (2, "+", "Wah-wah closed")] {
                    if paint_button(ui, cur.fx.wah == v, tip, |p, r, c| {
                        p.add(Shape::convex_polygon(vec![Pos2::new(r.left(), r.bottom()), Pos2::new(r.right(), r.bottom()), Pos2::new(r.right(), r.bottom() - 3.0), Pos2::new(r.left(), r.bottom() - 6.0)], c, PStroke::NONE));
                        small(p, Pos2::new(r.center().x, r.top() + 4.0), mark, 11.0, c);
                    })
                    .clicked()
                    {
                        self.beat_edit(|b| b.fx.wah = if b.fx.wah == v { 0 } else { v });
                    }
                }
                // Fingerings: a list of fingers for each hand.
                for (left, tip) in [(true, "Left-hand fingering"), (false, "Right-hand fingering")] {
                    let cur_f = if left { n.fx.lh_finger } else { n.fx.rh_finger };
                    let names = if left { lh_names } else { rh_names };
                    let lab = if cur_f > 0 { names[(cur_f - 1) as usize].to_string() } else if left { "LH".into() } else { "RH".into() };
                    let resp = glyph_button(ui, has && cur_f > 0, &lab, bolds(if cur_f > 0 { 13.0 } else { 9.5 }), tip);
                    egui::Popup::menu(&resp).show(|ui| {
                        if ui.selectable_label(cur_f == 0, "None").clicked() {
                            self.note_edit(move |x| if left { x.fx.lh_finger = 0 } else { x.fx.rh_finger = 0 });
                        }
                        for (k, name) in names.iter().enumerate() {
                            let v = k as u8 + 1;
                            if ui.selectable_label(cur_f == v, *name).clicked() {
                                self.note_edit(move |x| if left { x.fx.lh_finger = v } else { x.fx.rh_finger = v });
                            }
                        }
                    });
                }
            });
        });
        divider(ui);

        // ---- Notation and design.
        row(ui, |ui| {
            if paint_button(ui, cur.chord.is_some(), "Chord\u{2026} (A)", |p, r, c| {
                for i in 0..5 {
                    let x = r.left() + 2.0 + i as f32 * 3.5;
                    p.line_segment([Pos2::new(x, r.top() + 1.0), Pos2::new(x, r.bottom())], st(c, 0.9));
                }
                for i in 0..5 {
                    let y = r.top() + 1.0 + i as f32 * 4.0;
                    p.line_segment([Pos2::new(r.left() + 2.0, y), Pos2::new(r.left() + 16.0, y)], st(c, if i == 0 { 1.8 } else { 0.9 }));
                }
            })
            .clicked()
            {
                self.open_chord_tool();
            }
            if paint_button(ui, self.scale_open, "Scales\u{2026}", |p, r, c| {
                for i in 0..6 {
                    let y = r.top() + 1.0 + i as f32 * 3.2;
                    p.line_segment([Pos2::new(r.left(), y), Pos2::new(r.right(), y)], st(c, 0.8));
                }
                for i in 0..5 {
                    let x = r.left() + i as f32 * 4.2;
                    p.line_segment([Pos2::new(x, r.top() + 1.0), Pos2::new(x, r.top() + 17.0)], st(c, 0.8));
                }
                for (x, y) in [(2.0, 4.0), (10.0, 7.0), (6.0, 10.0), (14.0, 13.0)] {
                    p.circle_filled(Pos2::new(r.left() + x, r.top() + y), 1.4, c);
                }
            })
            .clicked()
            {
                self.scale_open = true;
            }
            if paint_button(ui, cur.fx.slash, "Slash notation for this beat", |p, r, c| {
                p.line_segment([Pos2::new(r.left() + 5.0, r.bottom() - 1.0), Pos2::new(r.left() + 10.0, r.bottom() - 6.0)], st(c, 2.4));
                p.line_segment([Pos2::new(r.left() + 10.0, r.bottom() - 4.0), Pos2::new(r.left() + 10.0, r.top())], st(c, 1.0));
            })
            .clicked()
            {
                self.beat_edit(|b| b.fx.slash = !b.fx.slash);
            }
            let resp = glyph_button(ui, cur.fx.barre > 0, "BV", serif(12.0), "Barre: choose the fret, full or half");
            egui::Popup::menu(&resp).show(|ui| {
                if ui.selectable_label(cur.fx.barre == 0, "None").clicked() {
                    self.beat_edit(|b| b.fx.barre = 0);
                }
                let mut half = cur.fx.barre_half;
                if ui.checkbox(&mut half, "Half barre").changed() {
                    self.beat_edit(move |b| b.fx.barre_half = half);
                }
                ui.horizontal_wrapped(|ui| {
                    ui.set_max_width(170.0);
                    for f in 1..=12u8 {
                        if ui.selectable_label(cur.fx.barre == f, roman(f)).clicked() {
                            self.beat_edit(move |b| b.fx.barre = f);
                        }
                    }
                });
            });
            if glyph_button(ui, cur.fx.timer, "2:51", serif(11.0), "Timer: shows the time since the start of the song").clicked() {
                self.beat_edit(|b| b.fx.timer = !b.fx.timer);
            }
            if glyph_button(ui, cur.text.is_some(), "TXT", serif(10.0), "Text\u{2026} (T)").clicked() {
                self.open_popup(Popup::Text);
            }
            if paint_button(ui, mb.marker.is_some(), "Section\u{2026} (Shift+Ins)", |p, r, c| {
                p.rect_stroke(r.shrink(1.0), 3.0, st(c, 1.2), egui::StrokeKind::Inside);
                p.text(r.center(), Align2::CENTER_CENTER, "A", FontId::new(11.0, FontFamily::Name("serif-bold".into())), c);
            })
            .clicked()
            {
                self.open_popup(Popup::Section);
            }
        });
        row(ui, |ui| {
            let beams: [(u8, &str, bool, bool, Option<&str>); 4] = [
                (0, "Automatic beaming", true, false, Some("AUTO")),
                (1, "Force a beam to the previous note", true, false, None),
                (2, "Break the beam before this note", false, false, None),
                (3, "Break the secondary beams before this note", true, true, None),
            ];
            for (v, tip, beam, sec, label) in beams {
                if paint_button(ui, cur.fx.beam == v && v != 0, tip, |p, r, c| beamed(p, r, c, beam, sec, label)).clicked() {
                    self.beat_edit(move |b| b.fx.beam = v);
                }
            }
            for (v, tip) in [(0u8, "Automatic stem direction"), (1, "Stem up"), (2, "Stem down")] {
                if paint_button(ui, cur.fx.stem == v && v != 0, tip, |p, r, c| match v {
                    0 => beamed(p, r, c, false, false, Some("AUTO")),
                    1 => {
                        tiny_note(p, Pos2::new(r.right() - 7.0, r.bottom() - 2.0), c, 0, false);
                        p.line_segment([Pos2::new(r.left() + 3.0, r.bottom()), Pos2::new(r.left() + 3.0, r.top() + 3.0)], st(c, 1.2));
                        arrow_head(p, Pos2::new(r.left() + 3.0, r.top()), Vec2::new(0.0, -1.0), c);
                    }
                    _ => {
                        let h = Pos2::new(r.left() + 5.0, r.top() + 3.0);
                        p.add(Shape::ellipse_filled(h, Vec2::new(2.6, 1.9), c));
                        p.line_segment([h + Vec2::new(-2.3, 0.0), h + Vec2::new(-2.3, 10.0)], st(c, 1.0));
                        p.line_segment([Pos2::new(r.right() - 3.0, r.top()), Pos2::new(r.right() - 3.0, r.bottom() - 3.0)], st(c, 1.2));
                        arrow_head(p, Pos2::new(r.right() - 3.0, r.bottom()), Vec2::new(0.0, 1.0), c);
                    }
                })
                .clicked()
                {
                    self.beat_edit(move |b| b.fx.stem = v);
                }
            }
            if paint_button(ui, cur.fx.force_group, "Force groups of the same tuplets", |p, r, c| {
                for x in [r.left() + 1.0, r.center().x + 1.0] {
                    p.line_segment([Pos2::new(x, r.top() + 4.0), Pos2::new(x + 7.0, r.top() + 4.0)], st(c, 1.2));
                    p.add(Shape::convex_polygon(vec![Pos2::new(x + 1.0, r.top() + 6.0), Pos2::new(x + 6.0, r.top() + 6.0), Pos2::new(x + 3.5, r.top() + 10.0)], c, PStroke::NONE));
                }
                p.line_segment([Pos2::new(r.left() + 1.0, r.bottom() - 2.0), Pos2::new(r.right() - 1.0, r.bottom() - 2.0)], st(c, 1.2));
            })
            .clicked()
            {
                self.beat_edit(|b| b.fx.force_group = !b.fx.force_group);
            }
        });
        divider(ui);

        // ---- Automations.
        row(ui, |ui| {
            if glyph_button(ui, mb.tempo.is_some(), "\u{2669}=", sym(13.0), "Tempo automation\u{2026} (F10)").clicked() {
                self.open_automations(self.bar);
            }
            ui.add_enabled_ui(false, |ui| {
                for (master, pan, tip) in [
                    (true, false, "Master volume automation (not in GuitarCraft yet)"),
                    (true, true, "Master pan automation (not in GuitarCraft yet)"),
                    (false, false, "Track volume automation (not in GuitarCraft yet)"),
                    (false, true, "Track pan automation (not in GuitarCraft yet)"),
                ] {
                    paint_button(ui, false, tip, |p, r, c| level_bars(p, r, c, master, pan));
                }
            });
        });
        divider(ui);
        ui.label(RichText::new("Click a string on the score and type fret numbers. Arrow keys move, F1 lists every shortcut.").size(11.0).color(Color32::from_gray(140)));
    }
}

/// Roman numerals for barre frets.
pub fn roman(n: u8) -> String {
    const R: [&str; 25] = ["", "I", "II", "III", "IV", "V", "VI", "VII", "VIII", "IX", "X", "XI", "XII", "XIII", "XIV", "XV", "XVI", "XVII", "XVIII", "XIX", "XX", "XXI", "XXII", "XXIII", "XXIV"];
    R.get(n as usize).copied().unwrap_or("").to_string()
}
