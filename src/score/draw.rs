//! Drawing the contents of one bar of one track: notes, rests, stems, beams, effects.

use super::*;
use egui::{epaint::QuadraticBezierShape, Shape};

pub struct BarCtx<'a> {
    pub song: &'a Song,
    pub ti: usize,
    pub bi: usize,
    pub g: Geom,
    pub y0: f32,
    pub o: Pos2,
    pub z: f32,
    pub xs: &'a [f32],
    pub ws: &'a [f32],
    pub music_font: bool,
    pub lay: &'a Layout,
    pub block: usize,
    pub lyrics: &'a std::collections::HashMap<(usize, usize), String>,
    /// The voice drawn (0 = voice 1), as a bar of its own.
    pub voice: usize,
    pub vbar: &'a Bar,
    /// More than one voice has notes in this bar.
    pub multi: bool,
    /// Draw the rhythm under the tab for this voice.
    pub rhythm: bool,
    /// Voice 1 has notes in this bar (other voices then leave dynamics to it).
    pub vbar_main_used: bool,
    /// Drawing the lower staff of a grand staff.
    pub lower: bool,
}

/// Splits a bar of a keyboard track into the notes for the upper staff (middle C
/// and up) and the lower staff. A beat with no notes on a staff is a rest there.
pub fn split_grand(track: &Track, bar: &Bar, key: i8) -> (Bar, Bar) {
    let mut up = bar.clone();
    let mut low = bar.clone();
    for (j, beat) in bar.beats.iter().enumerate() {
        let lows: Vec<bool> = beat
            .notes
            .iter()
            .map(|n| note_staff_pos(track, beat, key, n) <= -3)
            .collect();
        let mut k = 0;
        up.beats[j].notes.retain(|_| {
            k += 1;
            !lows[k - 1]
        });
        let mut k = 0;
        low.beats[j].notes.retain(|_| {
            k += 1;
            lows[k - 1]
        });
    }
    (up, low)
}

/// The lower (bass clef) staff of a grand staff: its notes and rests only.
pub fn draw_lower_staff(p: &Painter, c: &BarCtx) {
    let bar = c.vbar;
    let mb = &c.song.master[c.bi];
    let g = &c.g;
    let z = c.z;
    let groups = beam_groups(bar, mb);
    for (bj, beat) in bar.beats.iter().enumerate() {
        let Some(&bx) = c.xs.get(bj) else { continue };
        let x = c.o.x + bx;
        if beat.is_rest() {
            if c.multi && c.voice > 0 {
                continue;
            }
            let shift = if c.multi {
                (if c.voice % 2 == 0 { -2.0 } else { 2.0 }) * g.ls
            } else {
                0.0
            };
            draw_rest(
                p,
                Pos2::new(x, c.y0 + g.std_top + 2.0 * g.ls + shift),
                beat.dur,
                g.ls,
                c.music_font,
            );
            if beat.dotted || beat.double_dot {
                p.circle_filled(
                    Pos2::new(x + 8.0 * z, c.y0 + g.std_top + 1.5 * g.ls),
                    1.5 * z,
                    ink(),
                );
            }
            continue;
        }
        draw_std_beat(p, c, beat, bj, x, &groups);
    }
}

thread_local! {
    static INK_NOW: std::cell::Cell<Color32> = const { std::cell::Cell::new(super::INK) };
    static CAPO_STD: std::cell::Cell<bool> = const { std::cell::Cell::new(true) };
}

/// Whether a capo changes the notes written in standard notation (a stylesheet choice).
pub fn set_capo_std(on: bool) {
    CAPO_STD.with(|x| x.set(on));
}

/// Font of a playing-technique label, in the handwritten look for the Jazz symbol style.
fn sym_font(st: &crate::style::Stylesheet, k: usize, fz: f32) -> FontId {
    let Some(sy) = st.symbols.get(k) else {
        return italic(9.0 * fz);
    };
    if st.symbol_style == 1 {
        FontSpec {
            family: "Ink Free".into(),
            ..sy.font.clone()
        }
        .id(fz)
    } else {
        sy.font.id(fz)
    }
}

fn sym_text(st: &crate::style::Stylesheet, k: usize) -> String {
    st.symbols
        .get(k)
        .map(|x| x.text.clone())
        .unwrap_or_default()
}

/// Beats that carry a running effect (let ring, palm mute). With "symbol lines
/// extend over rests", rests between two such beats carry it too.
fn run_flags(bar: &Bar, n: usize, extend: bool, has: impl Fn(&Beat) -> bool) -> Vec<bool> {
    let mut f: Vec<bool> = bar.beats.iter().take(n).map(&has).collect();
    if extend {
        for j in 0..f.len() {
            if f[j] || !bar.beats[j].is_rest() {
                continue;
            }
            let before = (0..j)
                .rev()
                .find(|&k| !bar.beats[k].is_rest())
                .map(|k| has(&bar.beats[k]))
                .unwrap_or(false);
            let after = (j + 1..f.len())
                .find(|&k| !bar.beats[k].is_rest())
                .map(|k| has(&bar.beats[k]))
                .unwrap_or(false);
            f[j] = before && after;
        }
    }
    f
}

/// Colour used for notes and symbols (grey for voices not being edited).
pub fn set_ink(c: Color32) {
    INK_NOW.with(|x| x.set(c));
}

fn ink() -> Color32 {
    INK_NOW.with(|x| x.get())
}

pub fn draw_bar_content(p: &Painter, c: &BarCtx) {
    let track = &c.song.tracks[c.ti];
    let bar = c.vbar;
    let mb = &c.song.master[c.bi];
    let g = &c.g;
    let z = c.z;
    let y0 = c.y0;
    let groups = beam_groups(bar, mb);
    let st = &c.song.style;
    let fz = c.lay.fz;
    let chord_font = if st.chord_style == 1 {
        FontSpec::new("Ink Free", st.font_chord.size, true, false).id(fz)
    } else {
        st.font_chord.id(fz)
    };
    let dk = st.chords_in_score_size / 9.0;
    let mut prev_dyn: Option<Dynamic> = if c.bi > 0 {
        track.bars[c.bi - 1]
            .voice(c.voice)
            .iter()
            .rev()
            .find(|b| !b.is_rest())
            .map(|b| b.dynamic)
    } else {
        None
    };
    for (bj, beat) in bar.beats.iter().enumerate() {
        let Some(&bx) = c.xs.get(bj) else { continue };
        let x = c.o.x + bx;
        let mut above = y0 + g.chord_y;
        if beat.chord_diagram && !beat.is_rest() && st.chords_in_score {
            let top = above - 46.0 * z * dk;
            draw_diagram(
                p,
                &c.song.tracks[c.ti],
                beat,
                Pos2::new(x, top),
                z * dk,
                st.chord_style,
            );
            if let Some(ch) = &beat.chord {
                p.text(
                    Pos2::new(x, top - 2.0 * z),
                    Align2::CENTER_BOTTOM,
                    ch,
                    chord_font.clone(),
                    ink(),
                );
            }
            above = top - 16.0 * z;
        } else if let Some(ch) = &beat.chord {
            p.text(
                Pos2::new(x, above),
                Align2::CENTER_BOTTOM,
                ch,
                chord_font.clone(),
                ink(),
            );
            above -= 14.0 * z;
        }
        if let Some(t) = &beat.text {
            p.text(
                Pos2::new(x - 4.0 * z, above),
                Align2::LEFT_BOTTOM,
                t,
                st.font_text.id(fz),
                ink(),
            );
        }
        // Small markings just above the top staff.
        let mut my = y0
            + if g.std {
                g.std_top - 9.0 * z
            } else {
                g.lane_top + 2.0 * z
            };
        if beat.fermata {
            let c0 = Pos2::new(x, my);
            let pts: Vec<Pos2> = (0..=12)
                .map(|i| {
                    let a = std::f32::consts::PI * i as f32 / 12.0;
                    Pos2::new(c0.x - a.cos() * 6.0 * z, c0.y - a.sin() * 5.0 * z)
                })
                .collect();
            p.add(Shape::line(pts, PStroke::new(1.3 * z, ink())));
            p.circle_filled(Pos2::new(c0.x, c0.y - 1.0 * z), 1.3 * z, ink());
            my -= 9.0 * z;
        }
        if beat.pick != 0 {
            if beat.pick == 1 {
                let r = Rect::from_center_size(Pos2::new(x, my), Vec2::new(8.0 * z, 6.0 * z));
                p.line_segment(
                    [r.left_bottom(), r.left_top()],
                    PStroke::new(1.2 * z, ink()),
                );
                p.line_segment([r.left_top(), r.right_top()], PStroke::new(2.2 * z, ink()));
                p.line_segment(
                    [r.right_top(), r.right_bottom()],
                    PStroke::new(1.2 * z, ink()),
                );
            } else {
                p.line_segment(
                    [
                        Pos2::new(x - 4.0 * z, my - 4.0 * z),
                        Pos2::new(x, my + 3.0 * z),
                    ],
                    PStroke::new(1.2 * z, ink()),
                );
                p.line_segment(
                    [
                        Pos2::new(x, my + 3.0 * z),
                        Pos2::new(x + 4.0 * z, my - 4.0 * z),
                    ],
                    PStroke::new(1.2 * z, ink()),
                );
            }
            my -= 10.0 * z;
        }
        if beat.technique != 0 {
            let (lab, f) = match beat.technique {
                1 => ("T".to_string(), bold(10.5 * fz)),
                2 => (
                    sym_text(st, crate::style::SYM_SLAP),
                    sym_font(st, crate::style::SYM_SLAP, fz),
                ),
                _ => (
                    sym_text(st, crate::style::SYM_POP),
                    sym_font(st, crate::style::SYM_POP, fz),
                ),
            };
            p.text(Pos2::new(x, my), Align2::CENTER_CENTER, lab, f, ink());
            my -= 11.0 * z;
        }
        if beat.fade_in {
            p.line_segment(
                [
                    Pos2::new(x - 2.0 * z, my),
                    Pos2::new(x + 10.0 * z, my - 3.5 * z),
                ],
                PStroke::new(1.0 * z, ink()),
            );
            p.line_segment(
                [
                    Pos2::new(x - 2.0 * z, my),
                    Pos2::new(x + 10.0 * z, my + 3.5 * z),
                ],
                PStroke::new(1.0 * z, ink()),
            );
            my -= 10.0 * z;
        }
        if beat.ottava != 0 && (bj == 0 || bar.beats[bj - 1].ottava != beat.ottava) {
            let lab = match beat.ottava {
                2 => "15ma",
                1 => "8va",
                -1 => "8vb",
                _ => "15mb",
            };
            p.text(
                Pos2::new(x - 4.0 * z, my),
                Align2::LEFT_CENTER,
                lab,
                serif_bi(11.0 * z),
                ink(),
            );
        }
        let _ = my;
        if let Some(l) = if c.voice == 0 {
            c.lyrics.get(&(c.bi, bj))
        } else {
            None
        }
        .or(beat.lyric.as_ref())
        {
            if st.lyrics_pos == 1 {
                p.text(
                    Pos2::new(x, y0 + g.chord_y + 9.0 * z),
                    Align2::CENTER_CENTER,
                    l,
                    st.font_lyrics.id(fz),
                    ink(),
                );
            } else if st.lyrics_pos == 0 {
                p.text(
                    Pos2::new(x, y0 + g.bottom - 9.0 * z),
                    Align2::CENTER_CENTER,
                    l,
                    st.font_lyrics.id(fz),
                    ink(),
                );
            }
        }
        if beat.is_rest() {
            if c.multi && c.voice > 0 && !g.std {
                continue;
            }
            let shift = if c.multi {
                (if c.voice % 2 == 0 { -2.0 } else { 2.0 }) * g.ls
            } else {
                0.0
            };
            if g.std {
                draw_rest(
                    p,
                    Pos2::new(x, y0 + g.std_top + 2.0 * g.ls + shift),
                    beat.dur,
                    g.ls,
                    c.music_font,
                );
                if beat.dotted || beat.double_dot {
                    p.circle_filled(
                        Pos2::new(x + 8.0 * z, y0 + g.std_top + 1.5 * g.ls),
                        1.5 * z,
                        ink(),
                    );
                }
                if beat.double_dot {
                    p.circle_filled(
                        Pos2::new(x + 12.0 * z, y0 + g.std_top + 1.5 * g.ls),
                        1.5 * z,
                        ink(),
                    );
                }
            } else if g.tab {
                let mid = Pos2::new(x, y0 + g.tab_top + (g.strings as f32 - 1.0) * g.ts * 0.5);
                if st.dash_quarter_rest && beat.dur == Dur::Quarter {
                    p.line_segment(
                        [Pos2::new(x - 4.0 * z, mid.y), Pos2::new(x + 4.0 * z, mid.y)],
                        PStroke::new(1.6 * z, ink()),
                    );
                } else {
                    draw_rest(p, mid, beat.dur, g.ls, c.music_font);
                }
            }
            continue;
        }
        if beat.hairpin != 0 && (bj == 0 || bar.beats[bj - 1].hairpin != beat.hairpin) {
            let mut e = bj;
            while e + 1 < bar.beats.len() && bar.beats[e + 1].hairpin == beat.hairpin {
                e += 1;
            }
            let x0 = x + 12.0 * z;
            let x1 = (c.o.x
                + c.xs.get(e).copied().unwrap_or(bx)
                + c.ws.get(e).copied().unwrap_or(20.0) * 0.55)
                .max(x0 + 16.0 * z);
            let hy = if g.std {
                y0 + g.std_top + 4.0 * g.ls + 22.0 * z
            } else {
                y0 + g.rhythm_top + 30.0 * z
            };
            let (open_x, tip_x) = if beat.hairpin == 1 {
                (x1, x0)
            } else {
                (x0, x1)
            };
            p.line_segment(
                [Pos2::new(tip_x, hy), Pos2::new(open_x, hy - 4.0 * z)],
                PStroke::new(1.0 * z, ink()),
            );
            p.line_segment(
                [Pos2::new(tip_x, hy), Pos2::new(open_x, hy + 4.0 * z)],
                PStroke::new(1.0 * z, ink()),
            );
        }
        if prev_dyn != Some(beat.dynamic)
            && (c.voice == 0 || !c.vbar_main_used)
            && !st.hide_dynamics
        {
            let lowest = std_positions(track, beat, mb.key)
                .iter()
                .map(|x| x.0)
                .min()
                .unwrap_or(0);
            let mut dy = y0 + g.std_top + 4.0 * g.ls + 22.0 * z;
            if lowest < 0 {
                dy = dy.max(y0 + g.std_top + 4.0 * g.ls - lowest as f32 * g.ls * 0.5 + 10.0 * z);
            }
            if !g.std {
                dy = y0 + g.rhythm_top + 30.0 * z;
            }
            p.text(
                Pos2::new(x - 2.0 * z, dy),
                Align2::LEFT_CENTER,
                beat.dynamic.label(),
                serif_bi(14.0 * z),
                ink(),
            );
        }
        prev_dyn = Some(beat.dynamic);
        if g.std {
            draw_std_beat(p, c, beat, bj, x, &groups);
        }
        if g.tab {
            draw_tab_beat(p, c, beat, bj, x);
        }
    }
    if g.tab {
        if c.rhythm {
            draw_tab_rhythm(p, c, bar, &groups);
        }
        draw_runs(p, c, bar);
    }
    draw_let_ring(p, c, bar);
}

pub fn draw_small_note(p: &Painter, c: Pos2, z: f32) {
    notehead(p, Pos2::new(c.x, c.y + 4.0 * z), z * 0.8, true);
    p.line_segment(
        [
            Pos2::new(c.x + 3.4 * z, c.y + 3.0 * z),
            Pos2::new(c.x + 3.4 * z, c.y - 9.0 * z),
        ],
        PStroke::new(1.1 * z, ink()),
    );
}

pub fn notehead(p: &Painter, c: Pos2, z: f32, filled: bool) {
    let rx = 4.4 * z;
    let ry = 3.1 * z;
    let rot: f32 = -0.35;
    let (sn, cs) = rot.sin_cos();
    let pts: Vec<Pos2> = (0..20)
        .map(|i| {
            let a = i as f32 / 20.0 * std::f32::consts::TAU;
            let (x, y) = (a.cos() * rx, a.sin() * ry);
            Pos2::new(c.x + x * cs - y * sn, c.y + x * sn + y * cs)
        })
        .collect();
    if filled {
        p.add(Shape::convex_polygon(pts, ink(), PStroke::NONE));
    } else {
        p.add(Shape::closed_line(pts, PStroke::new(1.4 * z, ink())));
    }
}

pub fn repeat_dots(p: &Painter, g: &Geom, y0: f32, x: f32, z: f32) {
    for (st, _) in g.std_staves() {
        p.circle_filled(Pos2::new(x, y0 + st + 1.5 * g.ls), 1.7 * z, ink());
        p.circle_filled(Pos2::new(x, y0 + st + 2.5 * g.ls), 1.7 * z, ink());
    }
    if g.tab {
        let mid = g.tab_top + (g.strings as f32 - 1.0) * g.ts * 0.5;
        p.circle_filled(Pos2::new(x, y0 + mid - g.ts * 0.5), 1.7 * z, ink());
        p.circle_filled(Pos2::new(x, y0 + mid + g.ts * 0.5), 1.7 * z, ink());
    }
}

pub fn clef(p: &Painter, pos: Pos2, g: &Geom, cl: Clef, music_font: bool) {
    let ls = g.ls;
    let bass = cl.is_bass();
    if cl == Clef::Percussion {
        // Two thick bars in the middle of the staff.
        for dx in [-3.0, 3.0] {
            p.rect_filled(
                Rect::from_min_max(
                    Pos2::new(pos.x + dx * g.z - 1.6 * g.z, pos.y + 1.0 * ls),
                    Pos2::new(pos.x + dx * g.z + 1.6 * g.z, pos.y + 3.0 * ls),
                ),
                0.0,
                ink(),
            );
        }
        return;
    }
    if music_font {
        if bass {
            p.text(
                Pos2::new(pos.x, pos.y + 1.0 * ls),
                Align2::CENTER_CENTER,
                "\u{1D122}",
                music(ls * 4.2),
                ink(),
            );
        } else {
            p.text(
                Pos2::new(pos.x, pos.y + 2.2 * ls),
                Align2::CENTER_CENTER,
                "\u{1D11E}",
                music(ls * 5.6),
                ink(),
            );
        }
        if cl.shift() != 0 {
            p.text(
                Pos2::new(pos.x, pos.y + 4.0 * ls + 10.0 * g.z),
                Align2::CENTER_CENTER,
                "8",
                fid(8.0 * g.z),
                ink(),
            );
        }
    } else {
        p.text(
            Pos2::new(pos.x, pos.y + 2.0 * ls),
            Align2::CENTER_CENTER,
            if bass { "F" } else { "G" },
            bold(ls * 3.5),
            ink(),
        );
    }
}

pub fn draw_rest(p: &Painter, c: Pos2, d: Dur, ls: f32, music_font: bool) {
    match d {
        Dur::Whole => {
            p.rect_filled(
                Rect::from_min_size(
                    Pos2::new(c.x - 4.0 * ls / 7.5, c.y - ls),
                    Vec2::new(8.0 * ls / 7.5, ls * 0.5),
                ),
                0.0,
                ink(),
            );
        }
        Dur::Half => {
            p.rect_filled(
                Rect::from_min_size(
                    Pos2::new(c.x - 4.0 * ls / 7.5, c.y - ls * 0.5),
                    Vec2::new(8.0 * ls / 7.5, ls * 0.5),
                ),
                0.0,
                ink(),
            );
        }
        _ => {
            if music_font {
                let ch = match d {
                    Dur::Quarter => "\u{1D13D}",
                    Dur::Eighth => "\u{1D13E}",
                    Dur::Sixteenth => "\u{1D13F}",
                    Dur::ThirtySecond => "\u{1D140}",
                    _ => "\u{1D141}",
                };
                p.text(
                    Pos2::new(c.x, c.y),
                    Align2::CENTER_CENTER,
                    ch,
                    music(ls * 4.0),
                    ink(),
                );
            } else {
                let s = PStroke::new(1.6, ink());
                p.line_segment(
                    [
                        Pos2::new(c.x - 2.0, c.y - 1.5 * ls),
                        Pos2::new(c.x + 2.0, c.y - 0.5 * ls),
                    ],
                    s,
                );
                p.line_segment(
                    [
                        Pos2::new(c.x + 2.0, c.y - 0.5 * ls),
                        Pos2::new(c.x - 2.0, c.y + 0.5 * ls),
                    ],
                    s,
                );
                p.line_segment(
                    [
                        Pos2::new(c.x - 2.0, c.y + 0.5 * ls),
                        Pos2::new(c.x + 2.0, c.y + 1.4 * ls),
                    ],
                    s,
                );
            }
        }
    }
}

/// Groups of beat indices joined by beams (one group per beat unit of the bar).
pub fn beam_groups(bar: &Bar, mb: &MasterBar) -> Vec<Vec<usize>> {
    let unit = if mb.den == 8 && mb.num % 3 == 0 {
        QUARTER * 3 / 2
    } else {
        QUARTER
    };
    let mut groups = vec![];
    let mut cur: Vec<usize> = vec![];
    let mut tick = 0u32;
    let mut cur_unit = u32::MAX;
    for (i, b) in bar.beats.iter().enumerate() {
        let u = tick / unit;
        let beamable = b.dur.flags() > 0 && !b.is_rest();
        if !beamable || u != cur_unit {
            if cur.len() > 1 {
                groups.push(std::mem::take(&mut cur));
            }
            cur.clear();
        }
        if beamable {
            if cur.is_empty() {
                cur_unit = u;
            }
            cur.push(i);
        }
        tick += b.ticks();
    }
    if cur.len() > 1 {
        groups.push(cur);
    }
    groups
}

/// Diatonic step number and alteration (-1, 0, 1) for a midi pitch.
fn spell(p: i32, flats: bool) -> (i32, i8) {
    const SH: [(i32, i8); 12] = [
        (0, 0),
        (0, 1),
        (1, 0),
        (1, 1),
        (2, 0),
        (3, 0),
        (3, 1),
        (4, 0),
        (4, 1),
        (5, 0),
        (5, 1),
        (6, 0),
    ];
    const FL: [(i32, i8); 12] = [
        (0, 0),
        (1, -1),
        (1, 0),
        (2, -1),
        (2, 0),
        (3, 0),
        (4, -1),
        (4, 0),
        (5, -1),
        (5, 0),
        (6, -1),
        (6, 0),
    ];
    let oct = p.div_euclid(12) - 1;
    let (l, a) = if flats {
        FL[p.rem_euclid(12) as usize]
    } else {
        SH[p.rem_euclid(12) as usize]
    };
    (oct * 7 + l, a)
}

const SHARP_ORDER: [i32; 7] = [3, 0, 4, 1, 5, 2, 6];
const FLAT_ORDER: [i32; 7] = [6, 2, 5, 1, 4, 0, 3];

/// The alteration a key signature gives to a letter (0 = C ... 6 = B).
pub fn key_alter(key: i8, letter: i32) -> i8 {
    if key > 0 && SHARP_ORDER[..(key as usize).min(7)].contains(&letter) {
        1
    } else if key < 0 && FLAT_ORDER[..((-key) as usize).min(7)].contains(&letter) {
        -1
    } else {
        0
    }
}

/// Staff positions of the key signature's accidentals on a treble staff.
pub fn key_positions(key: i8) -> Vec<i32> {
    const SH: [i32; 7] = [8, 5, 9, 6, 3, 7, 4];
    const FL: [i32; 7] = [4, 7, 3, 6, 2, 5, 1];
    if key > 0 {
        SH[..(key as usize).min(7)].to_vec()
    } else {
        FL[..((-key) as usize).min(7)].to_vec()
    }
}

/// Staff position, accidental to show (0 none, 1 sharp, -1 flat, 2 natural) and note.
fn std_positions<'a>(track: &Track, beat: &'a Beat, key: i8) -> Vec<(i32, i8, &'a Note)> {
    let cl = track.clef();
    let bottom = cl.bottom();
    beat.notes
        .iter()
        .map(|n| {
            if cl == Clef::Percussion {
                return (drum_staff_pos(track.pitch(n)).0, 0, n);
            }
            let capo = if CAPO_STD.with(|x| x.get()) {
                0
            } else {
                track.capo_at(n.string as usize) as i32
            };
            let pitch = track.pitch(n) as i32 - capo + cl.shift() - 12 * beat.ottava as i32;
            let flats = match n.spell {
                1 => false,
                2 => true,
                _ => key < 0,
            };
            let (d, alter) = spell(pitch, flats);
            let ka = key_alter(key, d.rem_euclid(7));
            let shown = if alter == ka {
                0
            } else if alter == 0 {
                2
            } else {
                alter
            };
            (d - bottom, shown, n)
        })
        .collect()
}

pub fn accidental_glyph(a: i8) -> &'static str {
    match a {
        1 => "\u{266F}",
        -1 => "\u{266D}",
        _ => "\u{266E}",
    }
}

fn stem_up_of(positions: &[(i32, i8, &Note)]) -> bool {
    if positions.is_empty() {
        return true;
    }
    let avg: f32 = positions.iter().map(|x| x.0 as f32).sum::<f32>() / positions.len() as f32;
    avg < 4.0
}

fn draw_std_beat(p: &Painter, c: &BarCtx, beat: &Beat, bj: usize, x: f32, groups: &[Vec<usize>]) {
    let g = &c.g;
    let z = c.z;
    let track = &c.song.tracks[c.ti];
    let bar = c.vbar;
    let bot = c.y0 + g.std_top + 4.0 * g.ls;
    let ypos = |pos: i32| bot - pos as f32 * g.ls * 0.5;
    let key = c.song.master[c.bi].key;
    let lift = if c.lower { 12 } else { 0 };
    let std_positions = |track: &Track, beat: &'_ Beat, key: i8| -> Vec<(i32, i8, Note)> {
        std_positions(track, beat, key)
            .into_iter()
            .map(|(a, b, n)| (a + lift, b, n.clone()))
            .collect()
    };
    let pos = std_positions(track, beat, key);
    let perc = track.clef() == Clef::Percussion;
    let group = groups.iter().find(|gg| gg.contains(&bj));
    // A beamed group shares one stem direction.
    let up = if c.multi {
        c.voice % 2 == 0
    } else {
        match group {
            Some(gr) => {
                let all: Vec<_> = gr
                    .iter()
                    .flat_map(|&j| std_positions(track, &bar.beats[j], key))
                    .collect();
                let all: Vec<(i32, i8, &Note)> = all.iter().map(|(a, b, n)| (*a, *b, n)).collect();
                stem_up_of(&all)
            }
            None => stem_up_of(&pos.iter().map(|(a, b, n)| (*a, *b, n)).collect::<Vec<_>>()),
        }
    };
    let up = if perc && !c.multi { true } else { up };
    let filled = beat.dur.index() >= 2;
    let mut minp = i32::MAX;
    let mut maxp = i32::MIN;
    let mut sorted = pos.clone();
    sorted.sort_by_key(|x| x.0);
    let mut last_pos = i32::MIN;
    let mut shifted = false;
    for (ps, sharp, n) in &sorted {
        let sharp: &i8 = sharp;
        let cross = n.dead || (perc && drum_staff_pos(track.pitch(n)).1);
        minp = minp.min(*ps);
        maxp = maxp.max(*ps);
        let lx = [x - 7.0 * z, x + 7.0 * z];
        if *ps <= -2 {
            let mut l = -2;
            while l >= *ps {
                p.line_segment(
                    [Pos2::new(lx[0], ypos(l)), Pos2::new(lx[1], ypos(l))],
                    PStroke::new(1.0, ink()),
                );
                l -= 2;
            }
        }
        if *ps >= 10 {
            let mut l = 10;
            while l <= *ps {
                p.line_segment(
                    [Pos2::new(lx[0], ypos(l)), Pos2::new(lx[1], ypos(l))],
                    PStroke::new(1.0, ink()),
                );
                l += 2;
            }
        }
        let mut hx = x;
        if last_pos != i32::MIN && *ps - last_pos == 1 && !shifted {
            hx += if up { 8.4 * z } else { -8.4 * z };
            shifted = true;
        } else {
            shifted = false;
        }
        last_pos = *ps;
        let cpos = Pos2::new(hx, ypos(*ps));
        if cross {
            let s = PStroke::new(1.5 * z, ink());
            p.line_segment(
                [
                    cpos + Vec2::new(-3.5, -3.5) * z,
                    cpos + Vec2::new(3.5, 3.5) * z,
                ],
                s,
            );
            p.line_segment(
                [
                    cpos + Vec2::new(-3.5, 3.5) * z,
                    cpos + Vec2::new(3.5, -3.5) * z,
                ],
                s,
            );
        } else if n.harmonic {
            let d = 4.0 * z;
            p.add(Shape::closed_line(
                vec![
                    cpos + Vec2::new(-d, 0.0),
                    cpos + Vec2::new(0.0, -d),
                    cpos + Vec2::new(d, 0.0),
                    cpos + Vec2::new(0.0, d),
                ],
                PStroke::new(1.3 * z, ink()),
            ));
        } else {
            notehead(p, cpos, z, filled);
        }
        if n.ghost {
            p.text(
                cpos + Vec2::new(-7.0 * z, 0.0),
                Align2::CENTER_CENTER,
                "(",
                fid(11.0 * z),
                ink(),
            );
            p.text(
                cpos + Vec2::new(7.0 * z, 0.0),
                Align2::CENTER_CENTER,
                ")",
                fid(11.0 * z),
                ink(),
            );
        }
        if *sharp != 0 {
            p.text(
                Pos2::new(x - 11.0 * z, cpos.y),
                Align2::CENTER_CENTER,
                accidental_glyph(*sharp),
                music(13.0 * z),
                ink(),
            );
        }
        if beat.dotted || beat.double_dot {
            let dy = if ps % 2 == 0 { -g.ls * 0.5 } else { 0.0 };
            p.circle_filled(Pos2::new(x + 8.0 * z, cpos.y + dy), 1.5 * z, ink());
            if beat.double_dot {
                p.circle_filled(Pos2::new(x + 12.0 * z, cpos.y + dy), 1.5 * z, ink());
            }
        }
        if let Some(gf) = n.grace {
            let _ = gf;
            let gc = Pos2::new(x - 18.0 * z, cpos.y);
            notehead(p, gc, z * 0.6, true);
            p.line_segment(
                [
                    gc + Vec2::new(2.6 * z, 0.0),
                    gc + Vec2::new(2.6 * z, -12.0 * z),
                ],
                PStroke::new(0.9 * z, ink()),
            );
            p.line_segment(
                [
                    gc + Vec2::new(-1.0 * z, -4.0 * z),
                    gc + Vec2::new(6.0 * z, -10.0 * z),
                ],
                PStroke::new(0.9 * z, ink()),
            );
        }
        if n.tie {
            let a = Pos2::new(x - 20.0 * z, cpos.y + 5.0 * z);
            let b = Pos2::new(x - 6.0 * z, cpos.y + 5.0 * z);
            p.add(QuadraticBezierShape::from_points_stroke(
                [a, Pos2::new((a.x + b.x) * 0.5, a.y + 5.0 * z), b],
                false,
                Color32::TRANSPARENT,
                PStroke::new(1.0 * z, ink()),
            ));
        }
    }
    if beat.dur == Dur::Whole {
        return;
    }
    let stem_len = 3.4 * g.ls;
    let sx = if up { x + 4.0 * z } else { x - 4.0 * z };
    let (from, mut to) = if up {
        (ypos(minp), ypos(maxp) - stem_len)
    } else {
        (ypos(maxp), ypos(minp) + stem_len)
    };
    if let Some(gr) = group {
        let mut extreme = to;
        for &j in gr {
            let pp = std_positions(track, &bar.beats[j], key);
            if pp.is_empty() {
                continue;
            }
            let mn = pp.iter().map(|x| x.0).min().unwrap();
            let mx = pp.iter().map(|x| x.0).max().unwrap();
            let t = if up {
                ypos(mx) - stem_len
            } else {
                ypos(mn) + stem_len
            };
            extreme = if up { extreme.min(t) } else { extreme.max(t) };
        }
        to = extreme;
        if gr[0] == bj {
            let xs: Vec<f32> = gr
                .iter()
                .map(|&j| c.o.x + c.xs[j] + if up { 4.0 * z } else { -4.0 * z })
                .collect();
            draw_beams(p, bar, gr, &xs, to, if up { 1.0 } else { -1.0 }, z);
        }
    }
    p.line_segment(
        [Pos2::new(sx, from), Pos2::new(sx, to)],
        PStroke::new(1.1 * z, ink()),
    );
    if group.is_none() && beat.dur.flags() > 0 {
        for f in 0..beat.dur.flags() {
            let fy = to
                + if up {
                    f as f32 * 5.0 * z
                } else {
                    -(f as f32) * 5.0 * z
                };
            let dir = if up { 1.0 } else { -1.0 };
            p.add(QuadraticBezierShape::from_points_stroke(
                [
                    Pos2::new(sx, fy),
                    Pos2::new(sx + 7.0 * z, fy + dir * 6.0 * z),
                    Pos2::new(sx + 5.0 * z, fy + dir * 13.0 * z),
                ],
                false,
                Color32::TRANSPARENT,
                PStroke::new(1.5 * z, ink()),
            ));
        }
    }
    if beat.notes.iter().any(|n| n.accent) {
        let ay = if up {
            ypos(minp) + 10.0 * z
        } else {
            ypos(maxp) - 10.0 * z
        };
        p.text(
            Pos2::new(x, ay),
            Align2::CENTER_CENTER,
            ">",
            bold(12.0 * z),
            ink(),
        );
    }
    if beat.notes.iter().any(|n| n.heavy_accent) {
        let ay = if up {
            ypos(minp) + 11.0 * z
        } else {
            ypos(maxp) - 11.0 * z
        };
        p.text(
            Pos2::new(x, ay),
            Align2::CENTER_CENTER,
            "^",
            bold(13.0 * z),
            ink(),
        );
    }
    if beat.tremolo {
        let my = (from + to) * 0.5;
        for k in 0..3 {
            let yy = my + (k as f32 - 1.0) * 3.5 * z;
            p.line_segment(
                [
                    Pos2::new(sx - 4.0 * z, yy + 2.0 * z),
                    Pos2::new(sx + 4.0 * z, yy - 2.0 * z),
                ],
                PStroke::new(1.6 * z, ink()),
            );
        }
    }
    if beat.notes.iter().any(|n| n.staccato) {
        let ay = if up {
            ypos(minp) + 8.0 * z
        } else {
            ypos(maxp) - 8.0 * z
        };
        p.circle_filled(Pos2::new(x, ay), 1.5 * z, ink());
    }
    let st = &c.song.style;
    if beat.tuplet > 0 && st.tuplet_brackets {
        // A bracket over the whole run of tuplet notes, with the number in the middle.
        if bj == 0 || bar.beats[bj - 1].tuplet != beat.tuplet {
            let mut e = bj;
            while e + 1 < bar.beats.len()
                && bar.beats[e + 1].tuplet == beat.tuplet
                && e + 1 - bj < beat.tuplet as usize
            {
                e += 1;
            }
            let x1 = c.o.x + c.xs.get(e).copied().unwrap_or(c.xs[bj]) + 6.0 * z;
            let x0 = x - 6.0 * z;
            let ty = if up { to - 9.0 * z } else { to + 9.0 * z };
            let hook = if up { 4.0 * z } else { -4.0 * z };
            let mid = (x0 + x1) * 0.5;
            let s1 = PStroke::new(0.9 * z, ink());
            p.line_segment([Pos2::new(x0, ty + hook), Pos2::new(x0, ty)], s1);
            p.line_segment([Pos2::new(x0, ty), Pos2::new(mid - 5.0 * z, ty)], s1);
            p.line_segment([Pos2::new(mid + 5.0 * z, ty), Pos2::new(x1, ty)], s1);
            p.line_segment([Pos2::new(x1, ty), Pos2::new(x1, ty + hook)], s1);
            p.text(
                Pos2::new(mid, ty),
                Align2::CENTER_CENTER,
                format!("{}", beat.tuplet),
                italic(10.0 * z),
                ink(),
            );
        }
    } else if beat.tuplet > 0 && group.map(|gr| gr[0] == bj).unwrap_or(true) {
        let ty = if up { to - 7.0 * z } else { to + 7.0 * z };
        p.text(
            Pos2::new(x + 8.0 * z, ty),
            Align2::CENTER_CENTER,
            format!("{}", beat.tuplet),
            italic(10.0 * z),
            ink(),
        );
    }
}

fn draw_beams(p: &Painter, bar: &Bar, gr: &[usize], xs: &[f32], y: f32, dir: f32, z: f32) {
    let bw = 3.2 * z;
    let rect = |a: f32, b: f32, level: usize| {
        let yy = y + dir * level as f32 * 5.0 * z;
        let (t, bt) = if dir > 0.0 {
            (yy, yy + bw)
        } else {
            (yy - bw, yy)
        };
        Rect::from_min_max(Pos2::new(a.min(b), t), Pos2::new(a.max(b) + 1.0 * z, bt))
    };
    p.rect_filled(rect(xs[0], *xs.last().unwrap(), 0), 0.0, ink());
    for level in 1..4 {
        for (k, &j) in gr.iter().enumerate() {
            let f = bar.beats[j].dur.flags();
            if f <= level {
                continue;
            }
            let next_has = gr
                .get(k + 1)
                .map(|&jj| bar.beats[jj].dur.flags() > level)
                .unwrap_or(false);
            let prev_has = k > 0 && bar.beats[gr[k - 1]].dur.flags() > level;
            if next_has {
                p.rect_filled(rect(xs[k], xs[k + 1], level), 0.0, ink());
            } else if !prev_has {
                let stub = if k + 1 < gr.len() { 6.0 * z } else { -6.0 * z };
                p.rect_filled(rect(xs[k], xs[k] + stub, level), 0.0, ink());
            }
        }
    }
}

fn draw_tab_rhythm(p: &Painter, c: &BarCtx, bar: &Bar, groups: &[Vec<usize>]) {
    let g = &c.g;
    let z = c.z;
    let st = &c.song.style;
    // Below the tablature (the usual place) or above it; `s` points from the
    // tablature towards the beams.
    let above = super::tab_rhythm_pos(st, c.voice, g.std) == 0;
    let (near0, far, s) = if above {
        let near = c.y0 + g.tab_top - 4.0 * z;
        (near, near - 18.0 * z, -1.0)
    } else {
        let near = c.y0 + g.rhythm_top + 4.0 * z;
        (near, near + 18.0 * z, 1.0)
    };
    let stroke = PStroke::new(1.0 * z, ink());
    let span = |a: f32, b: f32| (a.min(b), a.max(b));
    for (bj, beat) in bar.beats.iter().enumerate() {
        if beat.is_rest() || bj >= c.xs.len() {
            continue;
        }
        let x = c.o.x + c.xs[bj];
        // "Extend rhythmic inside tablature": the stem starts at the note itself.
        let near = if st.rhythm_extend && !beat.notes.is_empty() {
            let edge = if above {
                beat.notes.iter().map(|n| n.string).min().unwrap_or(0)
            } else {
                beat.notes.iter().map(|n| n.string).max().unwrap_or(0)
            };
            c.y0 + g.string_y((edge as usize).min(g.strings - 1)) + s * g.ts * 0.45
        } else {
            near0
        };
        match beat.dur {
            Dur::Whole => {}
            Dur::Half => {
                p.line_segment(
                    [Pos2::new(x, near0 + s * 9.0 * z), Pos2::new(x, far)],
                    stroke,
                );
            }
            _ => {
                p.line_segment([Pos2::new(x, near), Pos2::new(x, far)], stroke);
            }
        }
        if beat.dotted || beat.double_dot {
            p.circle_filled(Pos2::new(x + 4.0 * z, far - s * 2.0 * z), 1.3 * z, ink());
        }
        if beat.double_dot {
            p.circle_filled(Pos2::new(x + 7.5 * z, far - s * 2.0 * z), 1.3 * z, ink());
        }
        if beat.tremolo {
            for k in 0..3 {
                let yy = near0 + s * (5.0 + k as f32 * 3.0) * z;
                p.line_segment(
                    [
                        Pos2::new(x - 3.5 * z, yy + 1.5 * z),
                        Pos2::new(x + 3.5 * z, yy - 1.5 * z),
                    ],
                    PStroke::new(1.3 * z, ink()),
                );
            }
        }
        let in_group = groups.iter().any(|gr| gr.contains(&bj));
        if !in_group && beat.dur.flags() > 0 {
            for f in 0..beat.dur.flags() {
                let fy = far - s * f as f32 * 4.0 * z;
                p.line_segment(
                    [Pos2::new(x, fy), Pos2::new(x + 6.0 * z, fy - s * 5.0 * z)],
                    PStroke::new(1.4 * z, ink()),
                );
            }
        }
        if beat.tuplet > 0
            && groups
                .iter()
                .find(|gr| gr.contains(&bj))
                .map(|gr| gr[0] == bj)
                .unwrap_or(true)
        {
            p.text(
                Pos2::new(x + 6.0 * z, far + s * 7.0 * z),
                Align2::CENTER_CENTER,
                format!("{}", beat.tuplet),
                italic(9.0 * z),
                ink(),
            );
        }
    }
    let bw = 2.6 * z;
    for gr in groups {
        let xs: Vec<f32> = gr.iter().map(|&j| c.o.x + c.xs[j]).collect();
        let (t, b) = span(far - s * bw, far);
        p.rect_filled(
            Rect::from_min_max(Pos2::new(xs[0], t), Pos2::new(*xs.last().unwrap() + 1.0, b)),
            0.0,
            ink(),
        );
        for level in 1..4 {
            for (k, &j) in gr.iter().enumerate() {
                if bar.beats[j].dur.flags() <= level {
                    continue;
                }
                let yy = far - s * level as f32 * 4.5 * z;
                let (t, b) = span(yy - s * bw, yy);
                let next = gr
                    .get(k + 1)
                    .filter(|&&jj| bar.beats[jj].dur.flags() > level);
                let prev_has = k > 0 && bar.beats[gr[k - 1]].dur.flags() > level;
                if next.is_some() {
                    p.rect_filled(
                        Rect::from_min_max(Pos2::new(xs[k], t), Pos2::new(xs[k + 1] + 1.0, b)),
                        0.0,
                        ink(),
                    );
                } else if !prev_has {
                    let (a, bb) = if k + 1 < gr.len() {
                        (xs[k], xs[k] + 5.0 * z)
                    } else {
                        (xs[k] - 5.0 * z, xs[k])
                    };
                    p.rect_filled(
                        Rect::from_min_max(Pos2::new(a, t), Pos2::new(bb + 1.0, b)),
                        0.0,
                        ink(),
                    );
                }
            }
        }
    }
}

fn draw_tab_beat(p: &Painter, c: &BarCtx, beat: &Beat, bj: usize, x: f32) {
    let g = &c.g;
    let z = c.z;
    let y0 = c.y0;
    let st = &c.song.style;
    let fz = c.lay.fz;
    let font = st.font_fret.id(fz);
    // Tied notes in the tablature: always, or only where a system starts.
    let system_start = bj == 0 && c.lay.bar_pos.get(c.bi).map(|x| x.1 == 0).unwrap_or(false);
    const VOICE_INK: [(u8, u8, u8); 3] = [(40, 130, 60), (180, 50, 120), (100, 60, 190)];
    for n in &beat.notes {
        let s = n.string as usize;
        if s >= g.strings {
            continue;
        }
        if n.tie && !st.tied_in_tab && !system_start {
            continue;
        }
        let y = y0 + g.string_y(s);
        let label = if n.dead {
            "x".to_string()
        } else if n.harmonic {
            format!("<{}>", n.fret)
        } else if n.ghost || n.tie {
            format!("({})", n.fret)
        } else {
            format!("{}", n.fret)
        };
        let label = if n.pinch_harmonic {
            format!("<{}>", n.fret)
        } else {
            label
        };
        let col = if n.tie {
            Color32::from_gray(120)
        } else if st.color_voices && c.voice > 0 {
            let (r, g2, b) = VOICE_INK[(c.voice - 1).min(2)];
            Color32::from_rgb(r, g2, b)
        } else {
            ink()
        };
        let gal = p.layout_no_wrap(label, font.clone(), col);
        let sz = gal.size();
        let r = Rect::from_center_size(Pos2::new(x, y), Vec2::new(sz.x + 2.0 * z, g.ts * 0.8));
        p.rect_filled(r, 0.0, Color32::WHITE);
        p.galley(Pos2::new(x - sz.x * 0.5, y - sz.y * 0.5), gal, col);
        if st.circle_half_whole && matches!(beat.dur, Dur::Half | Dur::Whole) {
            p.circle_stroke(
                Pos2::new(x, y),
                (sz.x.max(g.ts * 0.6) * 0.5 + 2.0 * z).min(g.ts * 0.62),
                PStroke::new(0.9 * z, col),
            );
        }
        let right = x + sz.x * 0.5 + 1.0 * z;
        let left = x - sz.x * 0.5 - 1.0 * z;
        if let Some(gf) = n.grace {
            let gg = p.layout_no_wrap(format!("{gf}"), st.font_grace.id(fz), ink());
            let gs = gg.size();
            let gx = left - gs.x - 3.0 * z;
            p.rect_filled(
                Rect::from_min_size(
                    Pos2::new(gx - 1.0, y - gs.y * 0.5),
                    gs + Vec2::new(2.0, 0.0),
                ),
                0.0,
                Color32::WHITE,
            );
            p.galley(Pos2::new(gx, y - gs.y * 0.5), gg, ink());
        }
        if n.slide_in {
            p.line_segment(
                [
                    Pos2::new(left - 10.0 * z, y + 4.0 * z),
                    Pos2::new(left - 1.0 * z, y - 1.0 * z),
                ],
                PStroke::new(1.1 * z, ink()),
            );
        }
        if n.slide_out && !n.slide {
            p.line_segment(
                [
                    Pos2::new(right + 1.0 * z, y - 1.0 * z),
                    Pos2::new(right + 11.0 * z, y + 4.0 * z),
                ],
                PStroke::new(1.1 * z, ink()),
            );
        }
        if n.trill > 0 {
            p.text(
                Pos2::new(x, y0 + g.lane_top + 2.0 * z),
                Align2::CENTER_CENTER,
                "tr",
                serif_bi(11.0 * z),
                ink(),
            );
            if st.trill_second {
                p.text(
                    Pos2::new(right + 2.0 * z, y),
                    Align2::LEFT_CENTER,
                    format!("({})", n.trill),
                    fid(8.5 * z),
                    Color32::from_gray(90),
                );
            }
        }
        if n.pinch_harmonic {
            p.text(
                Pos2::new(x, y0 + g.lane_top + 2.0 * z),
                Align2::CENTER_CENTER,
                "P.H.",
                italic(8.5 * z),
                ink(),
            );
        }
        if n.hammer || n.slide {
            if let Some((nx_x, nx_note)) = next_note_x(c, bj, n.string) {
                if n.hammer {
                    let a = Pos2::new(x + 2.0 * z, y - 6.0 * z);
                    let b = Pos2::new(nx_x - 2.0 * z, y - 6.0 * z);
                    let mid = Pos2::new((a.x + b.x) * 0.5, y - 13.0 * z);
                    p.add(QuadraticBezierShape::from_points_stroke(
                        [a, mid, b],
                        false,
                        Color32::TRANSPARENT,
                        PStroke::new(1.0 * z, ink()),
                    ));
                    if st.hopo_letters {
                        let k = if nx_note.fret >= n.fret {
                            crate::style::SYM_HAMMER
                        } else {
                            crate::style::SYM_PULL
                        };
                        p.text(
                            Pos2::new(mid.x, y - 14.0 * z),
                            Align2::CENTER_BOTTOM,
                            sym_text(st, k),
                            sym_font(st, k, fz),
                            ink(),
                        );
                    }
                }
                if n.slide {
                    let up = nx_note.fret > n.fret;
                    let d = 3.5 * z;
                    p.line_segment(
                        [
                            Pos2::new(right + 1.0 * z, if up { y + d } else { y - d }),
                            Pos2::new(nx_x - 6.0 * z, if up { y - d } else { y + d }),
                        ],
                        PStroke::new(1.1 * z, ink()),
                    );
                    if st.slide_sl {
                        let k = crate::style::SYM_SLIDE;
                        p.text(
                            Pos2::new((right + nx_x) * 0.5, y - 7.0 * z),
                            Align2::CENTER_BOTTOM,
                            sym_text(st, k),
                            sym_font(st, k, fz),
                            ink(),
                        );
                    }
                }
            } else if n.slide {
                p.line_segment(
                    [
                        Pos2::new(right + 1.0, y + 3.0 * z),
                        Pos2::new(right + 12.0 * z, y - 3.0 * z),
                    ],
                    PStroke::new(1.1 * z, ink()),
                );
            }
        }
        if n.bend > 0 {
            let top = y0 + g.lane_top - 2.0 * z;
            let a = Pos2::new(right, y);
            let b = Pos2::new(right + 10.0 * z, top + 6.0 * z);
            let st = PStroke::new(1.1 * z, ink());
            if n.bend_kind >= 2 {
                // Pre-bend: straight up from the note.
                p.line_segment([Pos2::new(x, y - 5.0 * z), Pos2::new(x, b.y)], st);
                let b = Pos2::new(x, b.y);
                p.add(Shape::convex_polygon(
                    vec![
                        Pos2::new(b.x - 3.0 * z, b.y + 1.0 * z),
                        Pos2::new(b.x + 3.0 * z, b.y + 1.0 * z),
                        Pos2::new(b.x, b.y - 4.0 * z),
                    ],
                    ink(),
                    PStroke::NONE,
                ));
            } else {
                p.add(QuadraticBezierShape::from_points_stroke(
                    [a, Pos2::new(right + 10.0 * z, y), b],
                    false,
                    Color32::TRANSPARENT,
                    st,
                ));
                p.add(Shape::convex_polygon(
                    vec![
                        Pos2::new(b.x - 3.0 * z, b.y + 1.0 * z),
                        Pos2::new(b.x + 3.0 * z, b.y + 1.0 * z),
                        Pos2::new(b.x, b.y - 4.0 * z),
                    ],
                    ink(),
                    PStroke::NONE,
                ));
            }
            if n.bend_kind == 1 || n.bend_kind == 3 {
                // Release: curve back down.
                let sx = if n.bend_kind == 3 { x } else { b.x };
                let e = Pos2::new(sx + 12.0 * z, y - 6.0 * z);
                p.add(QuadraticBezierShape::from_points_stroke(
                    [
                        Pos2::new(sx, b.y + 2.0 * z),
                        Pos2::new(e.x, b.y + 2.0 * z),
                        e,
                    ],
                    false,
                    Color32::TRANSPARENT,
                    st,
                ));
                p.add(Shape::convex_polygon(
                    vec![
                        Pos2::new(e.x - 3.0 * z, e.y - 1.0 * z),
                        Pos2::new(e.x + 3.0 * z, e.y - 1.0 * z),
                        Pos2::new(e.x, e.y + 4.0 * z),
                    ],
                    ink(),
                    PStroke::NONE,
                ));
            }
            let lab = match n.bend {
                1 => "¼".to_string(),
                2 => "½".to_string(),
                4 => "full".to_string(),
                6 => "1½".to_string(),
                8 => "2".to_string(),
                q => format!("{}", q as f32 / 4.0),
            };
            p.text(
                Pos2::new(b.x, b.y - 5.0 * z),
                Align2::CENTER_BOTTOM,
                lab,
                fid(9.0 * z),
                ink(),
            );
        }
    }
    if beat.notes.iter().any(|n| n.vibrato || n.wide_vibrato) {
        let wide = beat.notes.iter().any(|n| n.wide_vibrato);
        let y = y0 + g.lane_top + 4.0 * z;
        let w = (c.ws.get(bj).copied().unwrap_or(20.0) * 0.8).max(14.0 * z);
        let pts: Vec<Pos2> = (0..=24)
            .map(|i| {
                let t = i as f32 / 24.0;
                Pos2::new(
                    x - 3.0 * z + t * w,
                    y + (t * w / (5.0 * z) * std::f32::consts::PI).sin()
                        * if wide { 3.2 } else { 2.0 }
                        * z,
                )
            })
            .collect();
        p.add(Shape::line(
            pts,
            PStroke::new(if wide { 2.0 } else { 1.3 } * z, ink()),
        ));
    }
    if beat.stroke != Stroke::None && beat.notes.len() > 1 {
        let ys: Vec<f32> = beat
            .notes
            .iter()
            .map(|n| y0 + g.string_y(n.string as usize))
            .collect();
        let (mn, mx) = (
            ys.iter().cloned().fold(f32::MAX, f32::min),
            ys.iter().cloned().fold(f32::MIN, f32::max),
        );
        let sx = x - 10.0 * z;
        p.line_segment(
            [Pos2::new(sx, mn - 3.0 * z), Pos2::new(sx, mx + 3.0 * z)],
            PStroke::new(1.1 * z, ink()),
        );
        let (tip, dir) = if beat.stroke == Stroke::Down {
            (mn - 4.0 * z, -1.0)
        } else {
            (mx + 4.0 * z, 1.0)
        };
        if st.arpeggio_arrow {
            p.add(Shape::convex_polygon(
                vec![
                    Pos2::new(sx - 3.0 * z, tip - dir * 5.0 * z),
                    Pos2::new(sx + 3.0 * z, tip - dir * 5.0 * z),
                    Pos2::new(sx, tip),
                ],
                ink(),
                PStroke::NONE,
            ));
        }
    }
    if !g.std && beat.notes.iter().any(|n| n.accent) {
        p.text(
            Pos2::new(x, y0 + g.lane_top - 4.0 * z),
            Align2::CENTER_CENTER,
            ">",
            bold(11.0 * z),
            ink(),
        );
    }
}

fn next_note_x(c: &BarCtx, bj: usize, s: u8) -> Option<(f32, Note)> {
    let tr = &c.song.tracks[c.ti];
    if bj + 1 < c.vbar.beats.len() {
        let n = c.vbar.beats[bj + 1].note_on(s)?.clone();
        return Some((c.o.x + *c.xs.get(bj + 1)?, n));
    }
    if c.bi + 1 >= tr.bars.len() {
        return None;
    }
    let nb = &tr.bars[c.bi + 1];
    let n = nb.voice(c.voice).first()?.note_on(s)?.clone();
    let (si, k) = c.lay.bar_pos[c.bi + 1];
    let (si0, _) = c.lay.bar_pos[c.bi];
    if si != si0 {
        return None;
    }
    let bl = &c.lay.systems[si].bars[k];
    let x = if c.voice == 0 {
        *bl.beats_x[c.block].first()?
    } else {
        *bl.voice_xs(nb.voice(c.voice)).0.first()?
    };
    Some((c.o.x + x, n))
}

fn draw_runs(p: &Painter, c: &BarCtx, bar: &Bar) {
    let g = &c.g;
    let z = c.z;
    let y = c.y0 + g.lane_top + 13.0 * z;
    let st = &c.song.style;
    let n = bar.beats.len().min(c.xs.len());
    let flags = run_flags(bar, n, st.lines_over_rests, |b: &Beat| {
        b.notes.iter().any(|n| n.palm_mute)
    });
    let mut j = 0;
    while j < n {
        if flags[j] {
            let start = j;
            while j + 1 < n && flags[j + 1] {
                j += 1;
            }
            let x0 = c.o.x + c.xs[start] - 6.0 * z;
            let x1 = c.o.x + c.xs[j] + 6.0 * z;
            let k = crate::style::SYM_PALM_MUTE;
            let gal = p.layout_no_wrap(sym_text(st, k), sym_font(st, k, c.lay.fz), ink());
            let w = gal.size().x;
            p.galley(Pos2::new(x0, y - gal.size().y * 0.5), gal, ink());
            if x1 > x0 + w + 4.0 * z {
                p.add(Shape::dashed_line(
                    &[Pos2::new(x0 + w + 2.0 * z, y), Pos2::new(x1, y)],
                    PStroke::new(1.0 * z, ink()),
                    3.0 * z,
                    2.0 * z,
                ));
                p.line_segment(
                    [Pos2::new(x1, y - 3.0 * z), Pos2::new(x1, y + 3.0 * z)],
                    PStroke::new(1.0 * z, ink()),
                );
            }
        }
        j += 1;
    }
}

/// "let ring" with a dashed line, in the gap between the notation and the tab
/// (above the tab when only the tab is shown). The line carries on across bar
/// lines while the notes keep ringing, and ends with a small tick.
fn draw_let_ring(p: &Painter, c: &BarCtx, bar: &Bar) {
    let g = &c.g;
    let z = c.z;
    let rings = |b: &Beat| b.notes.iter().any(|n| n.let_ring);
    let nbeats = bar.beats.len().min(c.xs.len());
    if nbeats == 0 {
        return;
    }
    let st = &c.song.style;
    let flags = run_flags(bar, nbeats, st.lines_over_rests, rings);
    let track = &c.song.tracks[c.ti];
    let prev_rings = c.bi > 0
        && track.bars[c.bi - 1]
            .voice(c.voice)
            .last()
            .map(rings)
            .unwrap_or(false);
    let next_rings = track
        .bars
        .get(c.bi + 1)
        .and_then(|b| b.voice(c.voice).first())
        .map(rings)
        .unwrap_or(false);
    // The bar's extent on the page, and whether it starts a line of music.
    let (si, k) = c.lay.bar_pos[c.bi];
    let bl = &c.lay.systems[si].bars[k];
    let bar_left = c.o.x + bl.x;
    let bar_right = c.o.x + bl.x + bl.w;
    let first_in_system = k == 0;
    let y = c.y0 + g.lane_top + 6.0 * z;
    let stroke = PStroke::new(1.0 * z, ink());
    let mut j = 0;
    while j < nbeats {
        if !flags[j] {
            j += 1;
            continue;
        }
        let start = j;
        while j + 1 < nbeats && flags[j + 1] {
            j += 1;
        }
        let continues_in = start == 0 && prev_rings;
        let continues_out = j + 1 == nbeats && next_rings;
        let x0 = if continues_in {
            bar_left
        } else {
            c.o.x + c.xs[start] - 6.0 * z
        };
        let x1 = if continues_out {
            bar_right
        } else {
            c.o.x + c.xs[j] + c.ws.get(j).copied().unwrap_or(16.0 * z) * 0.55
        };
        let mut line_from = x0;
        if !continues_in || first_in_system {
            let k = crate::style::SYM_LET_RING;
            let gal = p.layout_no_wrap(sym_text(st, k), sym_font(st, k, c.lay.fz), ink());
            let w = gal.size().x;
            let tx = if continues_in { x0 + 4.0 * z } else { x0 };
            p.galley(Pos2::new(tx, y - gal.size().y * 0.5), gal, ink());
            line_from = tx + w + 3.0 * z;
        }
        // A ring on a single short note still shows a bit of line after the words.
        let x1 = if continues_out {
            x1
        } else {
            x1.max(line_from + 10.0 * z)
        };
        if x1 > line_from {
            p.add(Shape::dashed_line(
                &[Pos2::new(line_from, y), Pos2::new(x1, y)],
                stroke,
                2.0 * z,
                2.0 * z,
            ));
        }
        if !continues_out {
            p.line_segment(
                [Pos2::new(x1, y - 3.5 * z), Pos2::new(x1, y + 3.5 * z)],
                stroke,
            );
        }
        j += 1;
    }
}

/// A small chord diagram made from the beat's notes; `top` is its top centre.
/// Style 0 Classic, 1 Jazz (light lines, small dots), 2 Rock (bold lines, big dots).
pub fn draw_diagram(p: &Painter, track: &Track, beat: &Beat, top: Pos2, z: f32, style: u8) {
    let (lw, dot) = match style {
        1 => (0.6, 2.0),
        2 => (1.1, 3.0),
        _ => (0.8, 2.5),
    };
    let n = track.strings();
    let gap = 6.0 * z;
    let fh = 7.5 * z;
    let w = gap * (n as f32 - 1.0);
    let left = top.x - w * 0.5;
    let y0 = top.y + 8.0 * z;
    let fretted: Vec<u8> = beat
        .notes
        .iter()
        .filter(|n| n.fret > 0)
        .map(|n| n.fret)
        .collect();
    let max = fretted.iter().copied().max().unwrap_or(0);
    let base = if max <= 5 {
        1
    } else {
        fretted.iter().copied().min().unwrap_or(1)
    };
    let line = PStroke::new(lw * z, ink());
    for s in 0..n {
        let x = left + s as f32 * gap;
        p.line_segment([Pos2::new(x, y0), Pos2::new(x, y0 + 5.0 * fh)], line);
    }
    for f in 0..=5 {
        let y = y0 + f as f32 * fh;
        let st = if f == 0 && base == 1 {
            PStroke::new(if style == 2 { 2.8 } else { 2.0 } * z, ink())
        } else {
            line
        };
        p.line_segment([Pos2::new(left, y), Pos2::new(left + w, y)], st);
    }
    if base > 1 {
        p.text(
            Pos2::new(left + w + 3.0 * z, y0 + fh * 0.5),
            Align2::LEFT_CENTER,
            format!("{base}fr"),
            fid(7.5 * z),
            ink(),
        );
    }
    // Low string on the left.
    for s in 0..n {
        let string = (n - 1 - s) as u8;
        let x = left + s as f32 * gap;
        match beat.note_on(string) {
            Some(nn) if nn.fret == 0 => {
                p.circle_stroke(
                    Pos2::new(x, y0 - 4.0 * z),
                    2.0 * z,
                    PStroke::new(0.9 * z, ink()),
                );
            }
            Some(nn) => {
                let row = nn.fret as i32 - base as i32;
                if (0..5).contains(&row) {
                    p.circle_filled(Pos2::new(x, y0 + (row as f32 + 0.5) * fh), dot * z, ink());
                }
            }
            None => {
                let d = 2.0 * z;
                let c = Pos2::new(x, y0 - 4.0 * z);
                p.line_segment(
                    [c + Vec2::new(-d, -d), c + Vec2::new(d, d)],
                    PStroke::new(0.9 * z, ink()),
                );
                p.line_segment(
                    [c + Vec2::new(-d, d), c + Vec2::new(d, -d)],
                    PStroke::new(0.9 * z, ink()),
                );
            }
        }
    }
}

/// Staff position (half-spaces above the bottom line) of a note in standard notation.
pub fn note_staff_pos(track: &Track, beat: &Beat, key: i8, n: &Note) -> i32 {
    std_positions(track, beat, key)
        .iter()
        .find(|(_, _, x)| std::ptr::eq(*x, n) || (x.string == n.string && x.fret == n.fret))
        .map(|x| x.0)
        .unwrap_or(4)
}

/// Sounding pitch for a staff position, using the key signature (and the octave sign).
pub fn staff_pos_pitch(track: &Track, key: i8, ottava: i8, pos: i32) -> i32 {
    const STEPS: [i32; 7] = [0, 2, 4, 5, 7, 9, 11];
    let cl = track.clef();
    if cl == Clef::Percussion {
        return drum_at_pos(pos) as i32;
    }
    let d = pos + cl.bottom();
    let letter = d.rem_euclid(7);
    let oct = d.div_euclid(7);
    let written = (oct + 1) * 12 + STEPS[letter as usize] + key_alter(key, letter) as i32;
    written - cl.shift() + 12 * ottava as i32
}

const LETTERS: [&str; 7] = ["C", "D", "E", "F", "G", "A", "B"];

fn acc_text(a: i8) -> &'static str {
    match a {
        1 => "\u{266F}",
        -1 => "\u{266D}",
        _ => "",
    }
}

/// Written name of a note as shown in the notation, e.g. "D 5" or "F\u{266F} 4".
pub fn written_name(track: &Track, beat: &Beat, key: i8, n: &Note) -> String {
    if track.clef() == Clef::Percussion {
        return drum_name(track.pitch(n)).to_string();
    }
    let pitch = track.pitch(n) as i32 + track.clef().shift() - 12 * beat.ottava as i32;
    let flats = match n.spell {
        1 => false,
        2 => true,
        _ => key < 0,
    };
    let (d, alter) = spell(pitch, flats);
    format!(
        "{}{} {}",
        LETTERS[d.rem_euclid(7) as usize],
        acc_text(alter),
        d.div_euclid(7)
    )
}

/// Written name of a line or space of the notation staff, with the key signature applied.
pub fn staff_pos_name(track: &Track, key: i8, pos: i32) -> String {
    if track.clef() == Clef::Percussion {
        return drum_name(drum_at_pos(pos)).to_string();
    }
    let d = pos + track.clef().bottom();
    let letter = d.rem_euclid(7);
    format!(
        "{}{} {}",
        LETTERS[letter as usize],
        acc_text(key_alter(key, letter)),
        d.div_euclid(7)
    )
}
