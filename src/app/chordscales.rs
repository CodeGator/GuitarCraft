//! Tools > Chords & Scales: a pop-up toolkit for building and identifying
//! chords, scales and arpeggios on a fretboard.
//!
//! - Scales and Arpeggios: choose a root and a type, see every note on the neck
//!   (as note names or intervals), or just one part of the neck (the positions).
//! - Chords: choose a root and a chord type, then one of its fingerings; the
//!   fretboard shows the notes with finger numbers.
//! - Identify (the magnifier): click notes on the fretboard, one per string, and
//!   the chord is named, with its other names and the scales that hold those notes.
//! - Matching Chords for a scale, Matching Scales for a chord or arpeggio, play the
//!   notes, put a chord on the beat under the cursor, or copy its name.

use super::App;
use crate::chordshape;
use crate::model::{Note, Track};
use crate::theory::{self, CHORDS, SCALES};
use egui::{Align, Align2, Color32, FontFamily, FontId, Layout, Pos2, Rect, RichText, Sense, Stroke, Ui, Vec2};

const BLUE: Color32 = Color32::from_rgb(61, 123, 224);
const ROW_BG: Color32 = Color32::from_rgb(44, 47, 54);
const ROW_HOVER: Color32 = Color32::from_rgb(66, 70, 80);
const HEAD_BG: Color32 = Color32::from_rgb(30, 32, 38);
/// The frets shown.
const FRETS: u8 = 22;

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum Mode {
    #[default]
    Scales,
    Arpeggios,
    Chords,
}

/// Six-string tunings to choose from, low string to high, as MIDI notes.
const TUNINGS: [(&str, [i32; 6]); 8] = [
    ("Standard", [40, 45, 50, 55, 59, 64]),
    ("Drop D", [38, 45, 50, 55, 59, 64]),
    ("Half step down", [39, 44, 49, 54, 58, 63]),
    ("Whole step down", [38, 43, 48, 53, 57, 62]),
    ("DADGAD", [38, 45, 50, 55, 57, 62]),
    ("Open G", [38, 43, 50, 55, 59, 62]),
    ("Open D", [38, 45, 50, 54, 57, 62]),
    ("Open E", [40, 47, 52, 56, 59, 64]),
];

/// The tool's state (kept while the window is closed).
#[derive(Default)]
pub struct ChordScales {
    pub open: bool,
    pub mode: Mode,
    /// The magnifier: name chords from notes clicked on the fretboard.
    pub identify: bool,
    pub root: u8,
    pub scale: usize,
    /// The arpeggio's chord type.
    pub arp: usize,
    pub chord: usize,
    /// Scales and arpeggios: show only the part of the neck starting at this fret.
    pub pos: Option<u8>,
    pub voicing: usize,
    /// 0 note names, 1 intervals, 2 finger numbers (chords).
    pub label: u8,
    /// An entry of TUNINGS, or the current track's tuning after them.
    pub tuning: usize,
    /// Identify: the fret clicked on each string, low string first.
    pub picks: [Option<u8>; 6],
    /// Showing the matching chords or scales instead of the types.
    pub related: bool,
    pub auto_play: bool,
    /// Show the scale or arpeggio on the main window's fretboard too.
    pub overlay: bool,
    /// The matching list shows scales for the notes in the song, not for the chosen scale.
    from_song: bool,
    /// Bring the chosen rows into view (after jumping to a chord or scale).
    scroll: bool,
}

/// What the window asks the app to do.
enum Act {
    Play(Vec<(f32, u8, u8)>, [i32; 6]),
    Put { name: String, frets: [i8; 6] },
    Copy(String),
}

/// What the window needs to know about the song.
struct Env {
    /// The current track's tuning when it has six strings.
    track_tuning: Option<[i32; 6]>,
    /// A score is open with a beat under the cursor (not a chord chart).
    can_put: bool,
    /// The notes (0 to 11) in the selected bars, or the bar under the cursor, and what to call them.
    song_pcs: Vec<u8>,
    song_label: String,
}

#[derive(Clone, Copy, PartialEq)]
enum MarkKind {
    Root,
    Tone,
    Finger,
    Pick,
    Muted,
    Ring,
}

struct Mark {
    /// String, 0 = lowest.
    s: usize,
    fret: u8,
    text: String,
    kind: MarkKind,
}

fn tuning_text(t: &[i32; 6]) -> String {
    t.iter().map(|m| theory::note_name(m.rem_euclid(12) as u8, false)).collect::<Vec<_>>().join("")
}

impl ChordScales {
    /// The scale or arpeggio to show on the main fretboard, when that is switched on.
    pub fn overlay_pattern(&self) -> Option<(u8, Vec<u8>)> {
        if !self.overlay {
            return None;
        }
        match self.mode {
            Mode::Scales => Some((self.root, SCALES[self.scale].iv.to_vec())),
            Mode::Arpeggios => Some((self.root, theory::chord_pattern(self.arp))),
            Mode::Chords => None,
        }
    }

    fn tuning_choices(&self, env: &Env) -> Vec<(String, [i32; 6])> {
        let mut v: Vec<(String, [i32; 6])> = TUNINGS.iter().map(|t| (t.0.to_string(), t.1)).collect();
        if let Some(t) = env.track_tuning {
            v.push(("Current track".into(), t));
        }
        v
    }

    fn tuning_arr(&self, env: &Env) -> [i32; 6] {
        let all = self.tuning_choices(env);
        all.get(self.tuning).map(|t| t.1).unwrap_or(TUNINGS[0].1)
    }

    fn pattern(&self) -> Vec<u8> {
        match self.mode {
            Mode::Scales => SCALES[self.scale].iv.to_vec(),
            _ => theory::chord_pattern(if self.mode == Mode::Arpeggios { self.arp } else { self.chord }),
        }
    }

    fn goto_chord(&mut self, root: u8, kind: usize) {
        self.mode = Mode::Chords;
        self.identify = false;
        self.related = false;
        self.root = root;
        self.chord = kind;
        self.voicing = 0;
        self.scroll = true;
    }

    fn goto_scale(&mut self, root: u8, scale: usize) {
        self.mode = Mode::Scales;
        self.identify = false;
        self.related = false;
        self.root = root;
        self.scale = scale;
        self.pos = None;
        self.scroll = true;
    }

    fn voicings(&self, tuning: &[i32; 6]) -> Vec<[i8; 6]> {
        chordshape::voicings(tuning, self.root as i32, CHORDS[self.chord].suffix, None, 17)
    }

    /// Where the identified notes lie: the lowest note's pitch class and all pitch classes.
    fn picked(&self, tuning: &[i32; 6]) -> (u8, Vec<u8>) {
        let notes: Vec<i32> = (0..6).filter_map(|s| self.picks[s].map(|f| tuning[s] + f as i32)).collect();
        let bass = notes.iter().copied().min().map(|m| m.rem_euclid(12) as u8).unwrap_or(0);
        let mut pcs: Vec<u8> = notes.iter().map(|m| m.rem_euclid(12) as u8).collect();
        pcs.sort();
        pcs.dedup();
        (bass, pcs)
    }

    fn found(&self, tuning: &[i32; 6]) -> Vec<theory::Found> {
        let (bass, pcs) = self.picked(tuning);
        theory::identify(&pcs, bass)
    }

    /// The notes of the pattern on the neck, all of it or one position.
    fn pattern_marks(&self, tuning: &[i32; 6], window: Option<u8>) -> Vec<Mark> {
        let iv = self.pattern();
        let (names, labels): (Vec<String>, Vec<String>) = match self.mode {
            Mode::Scales => (theory::spell_pattern(self.root, &iv), theory::scale_intervals(&iv)),
            _ => {
                let kind = if self.mode == Mode::Arpeggios { self.arp } else { self.chord };
                (theory::spell_pattern(self.root, &iv), theory::chord_notes(kind).into_iter().map(|n| n.1).collect())
            }
        };
        let mut out = vec![];
        for (s, open) in tuning.iter().enumerate() {
            for f in 0..=FRETS {
                let rel = (open + f as i32 - self.root as i32).rem_euclid(12) as u8;
                let Some(k) = iv.iter().position(|&x| x == rel) else { continue };
                if let Some(w) = window
                    && !(f >= w && f < w + theory::POSITION_SPAN)
                {
                    continue;
                }
                let text = if self.label == 1 { labels[k].clone() } else { names[k].clone() };
                out.push(Mark { s, fret: f, text, kind: if rel == 0 { MarkKind::Root } else { MarkKind::Tone } });
            }
        }
        out
    }

    /// The starts of the parts of the neck that hold notes of the pattern.
    fn positions(&self, tuning: &[i32; 6]) -> Vec<u8> {
        theory::POSITIONS.iter().copied().filter(|&p| !self.pattern_marks(tuning, Some(p)).is_empty()).collect()
    }

    /// A fingering as marks: finger numbers, muted strings and open strings.
    fn chord_marks(&self, frets: &[i8; 6]) -> Vec<Mark> {
        let fingers = chordshape::fingers(frets);
        let mut out = vec![];
        for s in 0..6 {
            let f = frets[s];
            if f < 0 {
                out.push(Mark { s, fret: 0, text: "\u{00D7}".into(), kind: MarkKind::Muted });
                continue;
            }
            if f == 0 {
                out.push(Mark { s, fret: 0, text: String::new(), kind: MarkKind::Ring });
                continue;
            }
            out.push(Mark { s, fret: f as u8, text: format!("{}", fingers[s]), kind: MarkKind::Finger });
        }
        out
    }
}

/// Marks for a chord fingering, labelled as chosen: finger numbers, note names or intervals.
fn voicing_marks(cs: &ChordScales, tuning: &[i32; 6], frets: &[i8; 6]) -> Vec<Mark> {
    let mut marks = cs.chord_marks(frets);
    if cs.label == 2 {
        return marks;
    }
    let notes = theory::chord_notes(cs.chord);
    let pattern: Vec<u8> = notes.iter().map(|n| n.0).collect();
    let names = theory::spell_pattern(cs.root, &pattern);
    for m in &mut marks {
        if m.kind != MarkKind::Finger {
            continue;
        }
        let rel = (tuning[m.s] + m.fret as i32 - cs.root as i32).rem_euclid(12) as u8;
        if let Some(k) = pattern.iter().position(|&x| x == rel) {
            m.text = if cs.label == 1 { notes[k].1.clone() } else { names[k].clone() };
            m.kind = if rel == 0 { MarkKind::Root } else { MarkKind::Tone };
        }
    }
    marks
}

// ----- drawing

/// The x of fret `f`'s wire: frets get closer together up the neck, as on a guitar.
fn fret_x(nut: f32, right: f32, f: u8) -> f32 {
    let total = 1.0 - 2f32.powf(-(FRETS as f32) / 12.0);
    nut + (right - nut) * (1.0 - 2f32.powf(-(f as f32) / 12.0)) / total
}

/// Draws the fretboard (strings low to high from the bottom up) and returns the
/// string and fret under a click, when `clickable`.
fn fretboard(ui: &mut Ui, marks: &[Mark], clickable: bool, name: &str, tuning: &[i32; 6]) -> Option<(usize, u8)> {
    let w = ui.available_width();
    let (r, resp) = ui.allocate_exact_size(Vec2::new(w, 216.0), if clickable { Sense::click() } else { Sense::hover() });
    let p = ui.painter_at(r);
    p.rect_filled(r, 4.0, Color32::from_rgb(22, 23, 27));
    let nut = r.left() + 64.0;
    let right = r.right() - 10.0;
    let (top, bottom) = (r.top() + 22.0, r.top() + 146.0);
    let sy = |s: usize| top + (5 - s) as f32 * (bottom - top) / 5.0;
    // The wood, the nut and the frets.
    let board = Rect::from_min_max(Pos2::new(nut, top - 14.0), Pos2::new(right, bottom + 14.0));
    p.rect_filled(board, 2.0, Color32::from_rgb(66, 38, 24));
    p.rect_filled(Rect::from_min_max(Pos2::new(nut, top - 14.0), Pos2::new(right, top - 4.0)), 0.0, Color32::from_rgb(76, 45, 29));
    for f in 1..=FRETS {
        let x = fret_x(nut, right, f);
        p.line_segment([Pos2::new(x, top - 14.0), Pos2::new(x, bottom + 14.0)], Stroke::new(2.2, Color32::from_rgb(168, 168, 174)));
    }
    p.rect_filled(Rect::from_min_max(Pos2::new(nut - 5.0, top - 14.0), Pos2::new(nut, bottom + 14.0)), 0.0, Color32::from_rgb(232, 226, 200));
    // Position markers and fret numbers.
    let mid = |f: u8| (fret_x(nut, right, f - 1) + fret_x(nut, right, f)) * 0.5;
    for f in [3u8, 5, 7, 9, 12, 15, 17, 19, 21] {
        let ys: Vec<f32> = if f == 12 { vec![(sy(4) + sy(3)) * 0.5, (sy(2) + sy(1)) * 0.5] } else { vec![(sy(3) + sy(2)) * 0.5] };
        for y in ys {
            p.circle_filled(Pos2::new(mid(f), y), 5.0, Color32::from_rgba_unmultiplied(214, 204, 170, 70));
        }
        p.text(Pos2::new(mid(f), bottom + 26.0), Align2::CENTER_CENTER, format!("{f}"), FontId::proportional(11.0), Color32::from_gray(150));
    }
    // The strings, the wound ones thicker.
    for s in 0..6 {
        let col = if s < 3 { Color32::from_rgb(196, 160, 100) } else { Color32::from_rgb(220, 220, 226) };
        let width = [2.8, 2.3, 1.9, 1.5, 1.2, 1.0][s];
        p.line_segment([Pos2::new(r.left() + 28.0, sy(s)), Pos2::new(right, sy(s))], Stroke::new(width, col));
        p.text(Pos2::new(r.left() + 13.0, sy(s)), Align2::CENTER_CENTER, theory::note_name(tuning[s].rem_euclid(12) as u8, false), FontId::proportional(11.5), Color32::from_gray(140));
    }
    let cx = |f: u8| if f == 0 { nut - 22.0 } else { mid(f) };
    // What the pointer points at (to click in identify mode).
    let cell = resp.hover_pos().and_then(|pos| {
        let s = (0..6).min_by(|&a, &b| (sy(a) - pos.y).abs().partial_cmp(&(sy(b) - pos.y).abs()).unwrap_or(std::cmp::Ordering::Equal))?;
        if (sy(s) - pos.y).abs() > 16.0 || pos.x < r.left() + 30.0 || pos.x > right {
            return None;
        }
        let f = if pos.x < nut { 0 } else { (1..=FRETS).find(|&f| pos.x < fret_x(nut, right, f))? };
        Some((s, f))
    });
    if clickable
        && let Some((s, f)) = cell
    {
        p.circle_filled(Pos2::new(cx(f), sy(s)), 11.0, Color32::from_white_alpha(40));
    }
    // The marks: roots blue, the other notes dark, fingers red.
    for m in marks {
        let c = Pos2::new(cx(m.fret), sy(m.s));
        match m.kind {
            MarkKind::Muted => {
                p.text(c, Align2::CENTER_CENTER, &m.text, FontId::new(18.0, FontFamily::Name("bold".into())), Color32::from_gray(225));
            }
            MarkKind::Ring => {
                p.circle_stroke(c, 7.0, Stroke::new(1.8, Color32::from_gray(235)));
            }
            _ => {
                let fill = match m.kind {
                    MarkKind::Root => Color32::from_rgb(70, 130, 205),
                    MarkKind::Finger => Color32::from_rgb(200, 40, 40),
                    MarkKind::Pick => Color32::from_rgb(232, 142, 30),
                    _ => Color32::from_rgb(24, 24, 28),
                };
                p.circle_filled(c, 12.0, fill);
                p.circle_stroke(c, 12.0, Stroke::new(1.0, Color32::from_white_alpha(90)));
                let size = if m.text.chars().count() > 2 { 9.5 } else { 12.5 };
                p.text(c, Align2::CENTER_CENTER, &m.text, FontId::new(size, FontFamily::Name("bold".into())), Color32::WHITE);
            }
        }
    }
    if !name.is_empty() {
        p.text(Pos2::new(r.right() - 12.0, r.bottom() - 6.0), Align2::RIGHT_BOTTOM, name, FontId::new(17.0, FontFamily::Name("bold".into())), Color32::from_gray(225));
    }
    if clickable && resp.clicked() { cell } else { None }
}

/// A small chord diagram of a fingering, for the list of fingerings.
fn mini_diagram(p: &egui::Painter, area: Rect, frets: &[i8; 6], col: Color32) {
    let lo = chordshape::lowest_fret(frets);
    let hi = frets.iter().copied().max().unwrap_or(0);
    let base = if hi <= 4 { 1 } else { lo.max(1) };
    let (gx, gy) = (area.left() + 6.0, area.top() + 11.0);
    let (dx, dy) = (7.0, 6.5);
    let st = Stroke::new(0.8, col);
    for s in 0..6 {
        p.line_segment([Pos2::new(gx + s as f32 * dx, gy), Pos2::new(gx + s as f32 * dx, gy + 4.0 * dy)], st);
    }
    for f in 0..=4 {
        let w = if f == 0 && base == 1 { Stroke::new(2.0, col) } else { st };
        p.line_segment([Pos2::new(gx, gy + f as f32 * dy), Pos2::new(gx + 5.0 * dx, gy + f as f32 * dy)], w);
    }
    for (s, &f) in frets.iter().enumerate() {
        let x = gx + s as f32 * dx;
        match f {
            -1 => {
                p.text(Pos2::new(x, gy - 5.0), Align2::CENTER_CENTER, "\u{00D7}", FontId::proportional(9.0), col);
            }
            0 => {
                p.circle_stroke(Pos2::new(x, gy - 5.0), 2.2, st);
            }
            f => {
                let row = f - base;
                if (0..4).contains(&row) {
                    p.circle_filled(Pos2::new(x, gy + (row as f32 + 0.5) * dy), 2.7, col);
                }
            }
        }
    }
    if base > 1 {
        p.text(Pos2::new(gx + 5.0 * dx + 5.0, gy + dy * 0.5), Align2::LEFT_CENTER, format!("{base}"), FontId::proportional(9.0), col);
    }
}

fn header_row(ui: &mut Ui, w: f32, text: &str) {
    let (r, _) = ui.allocate_exact_size(Vec2::new(w, 20.0), Sense::hover());
    ui.painter().rect_filled(r, 0.0, HEAD_BG);
    ui.painter().text(Pos2::new(r.left() + 8.0, r.center().y), Align2::LEFT_CENTER, text, FontId::new(12.0, FontFamily::Name("bold".into())), Color32::from_gray(170));
}

/// A row of a list: highlighted when chosen.
fn list_row(ui: &mut Ui, w: f32, h: f32, on: bool, scroll: bool, text: &str) -> (egui::Response, Rect) {
    let (r, resp) = ui.allocate_exact_size(Vec2::new(w, h), Sense::click());
    let bg = if on { BLUE } else if resp.hovered() { ROW_HOVER } else { ROW_BG };
    ui.painter().rect_filled(r.shrink2(Vec2::new(0.0, 0.5)), 0.0, bg);
    ui.painter().text(Pos2::new(r.left() + 10.0, r.center().y), Align2::LEFT_CENTER, text, FontId::proportional(15.0), Color32::WHITE);
    if on && scroll {
        resp.scroll_to_me(Some(Align::Center));
    }
    (resp, r)
}

/// A button in the column at the right.
fn side_button(ui: &mut Ui, w: f32, text: &str, glyph: &str, on: Option<bool>, enabled: bool) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(Vec2::new(w, 34.0), if enabled { Sense::click() } else { Sense::hover() });
    let bg = if !enabled {
        Color32::from_rgb(40, 43, 49)
    } else if on == Some(true) {
        BLUE
    } else if resp.is_pointer_button_down_on() {
        Color32::from_rgb(90, 96, 112)
    } else if resp.hovered() {
        Color32::from_rgb(78, 83, 96)
    } else {
        Color32::from_rgb(58, 62, 72)
    };
    ui.painter().rect_filled(r, 6.0, bg);
    let ink = if enabled { Color32::from_gray(238) } else { Color32::from_gray(110) };
    ui.painter().text(Pos2::new(r.left() + 12.0, r.center().y), Align2::LEFT_CENTER, text, FontId::proportional(14.0), ink);
    if let Some(v) = on {
        // A little switch.
        let sw = Rect::from_center_size(Pos2::new(r.right() - 24.0, r.center().y), Vec2::new(32.0, 16.0));
        ui.painter().rect_filled(sw, 8.0, if v { Color32::WHITE } else { Color32::from_gray(30) });
        let x = if v { sw.right() - 8.0 } else { sw.left() + 8.0 };
        ui.painter().circle_filled(Pos2::new(x, sw.center().y), 6.0, if v { BLUE } else { Color32::from_gray(120) });
    } else if !glyph.is_empty() {
        ui.painter().text(Pos2::new(r.right() - 14.0, r.center().y), Align2::CENTER_CENTER, glyph, FontId::proportional(15.0), ink);
    }
    resp
}

// ----- the window

impl ChordScales {
    fn ui(&mut self, ui: &mut Ui, env: &Env, act: &mut Option<Act>) {
        ui.spacing_mut().item_spacing = Vec2::new(8.0, 6.0);
        let tunings = self.tuning_choices(env);
        if self.tuning >= tunings.len() {
            self.tuning = 0;
        }
        let tuning = self.tuning_arr(env);
        // ---- tabs, tuning
        ui.horizontal(|ui| {
            for (m, name) in [(Mode::Scales, "Scales"), (Mode::Arpeggios, "Arpeggios"), (Mode::Chords, "Chords")] {
                let on = self.mode == m;
                let (r, resp) = ui.allocate_exact_size(Vec2::new(112.0, 30.0), Sense::click());
                ui.painter().rect_filled(r, 5.0, if on { BLUE } else if resp.hovered() { ROW_HOVER } else { Color32::from_rgb(52, 56, 66) });
                ui.painter().text(r.center(), Align2::CENTER_CENTER, name, FontId::new(14.0, FontFamily::Name("bold".into())), if on { Color32::WHITE } else { Color32::from_gray(200) });
                if resp.clicked() {
                    self.mode = m;
                    self.related = false;
                    self.pos = None;
                    self.voicing = 0;
                    self.scroll = true;
                }
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let cur = tunings.get(self.tuning).map(|t| format!("6 String \u{00B7} {} {}", t.0, tuning_text(&t.1))).unwrap_or_default();
                egui::ComboBox::from_id_salt("cs-tuning").selected_text(cur).width(250.0).show_ui(ui, |ui| {
                    for (k, (name, t)) in tunings.iter().enumerate() {
                        if ui.selectable_label(self.tuning == k, format!("{name}  {}", tuning_text(t))).clicked() {
                            self.tuning = k;
                            self.voicing = 0;
                        }
                    }
                });
                ui.label(RichText::new("Tuning").color(Color32::from_gray(160)));
            });
        });
        // ---- browse or identify
        ui.horizontal(|ui| {
            for (id, name, tip) in [(false, "\u{2630} Browse", "Choose a root and a type from the lists"), (true, "\u{1F50D} Identify", "Click notes on the fretboard to name the chord and find scales that hold them")] {
                let on = self.identify == id;
                let (r, resp) = ui.allocate_exact_size(Vec2::new(100.0, 24.0), Sense::click());
                ui.painter().rect_filled(r, 5.0, if on { Color32::from_rgb(86, 92, 108) } else if resp.hovered() { ROW_HOVER } else { Color32::from_rgb(46, 49, 57) });
                ui.painter().text(r.center(), Align2::CENTER_CENTER, name, FontId::proportional(13.0), if on { Color32::WHITE } else { Color32::from_gray(180) });
                if resp.on_hover_text(tip).clicked() {
                    self.identify = id;
                    self.related = false;
                }
            }
            if self.identify {
                ui.label(RichText::new("Click the fretboard: one note on each string you play (click it again to take it off).").size(12.0).color(Color32::from_gray(160)));
            }
        });
        // ---- the fretboard and what it shows
        let (marks, name, clickable) = if self.identify {
            let marks: Vec<Mark> = (0..6)
                .map(|s| match self.picks[s] {
                    Some(f) => {
                        let pc = (tuning[s] + f as i32).rem_euclid(12) as u8;
                        Mark { s, fret: f, text: theory::note_name(pc, false), kind: MarkKind::Pick }
                    }
                    None => Mark { s, fret: 0, text: "\u{00D7}".into(), kind: MarkKind::Muted },
                })
                .filter(|m| !(m.kind == MarkKind::Muted && self.picks.iter().all(|p| p.is_none())))
                .collect();
            let name = self.found(&tuning).first().map(|f| theory::chord_display(f.root, f.kind, Some(f.bass))).unwrap_or_default();
            (marks, name, true)
        } else if self.mode == Mode::Chords {
            let list = self.voicings(&tuning);
            let v = self.voicing.min(list.len().saturating_sub(1));
            let marks = list.get(v).map(|f| voicing_marks(self, &tuning, f)).unwrap_or_default();
            (marks, theory::chord_display(self.root, self.chord, None), false)
        } else {
            let name = if self.mode == Mode::Scales {
                format!("{} {}", theory::note_name(self.root, theory::flats_for(self.root)), SCALES[self.scale].name)
            } else {
                format!("{} {} arpeggio", theory::note_name(self.root, theory::chord_flats(self.root, self.arp)), CHORDS[self.arp].label)
            };
            (self.pattern_marks(&tuning, self.pos), name, false)
        };
        if let Some((s, f)) = fretboard(ui, &marks, clickable && self.identify, &name, &tuning) {
            // One note on a string; clicking it again takes it off.
            self.picks[s] = if self.picks[s] == Some(f) { None } else { Some(f) };
            if self.auto_play {
                *act = Some(Act::Play(self.pick_notes(), tuning));
            }
        }
        ui.add_space(2.0);
        if self.identify {
            self.identify_panel(ui, &tuning, act);
        } else {
            self.browse_panel(ui, env, &tuning, act);
        }
        self.scroll = false;
    }

    fn pick_notes(&self) -> Vec<(f32, u8, u8)> {
        (0..6).filter_map(|s| self.picks[s].map(|f| (s, f))).enumerate().map(|(i, (s, f))| (i as f32 * 0.045, (5 - s) as u8, f)).collect()
    }

    fn browse_panel(&mut self, ui: &mut Ui, env: &Env, tuning: &[i32; 6], act: &mut Option<Act>) {
        let h = 252.0;
        let avail = ui.available_width();
        let (w_root, w_side, w_third) = (150.0, 196.0, 176.0);
        let w_type = (avail - w_root - w_side - w_third - 3.0 * ui.spacing().item_spacing.x - 6.0).max(200.0);
        let scroll = self.scroll;
        ui.horizontal_top(|ui| {
            // ---- roots
            ui.allocate_ui_with_layout(Vec2::new(w_root, h), Layout::top_down(Align::Min), |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                egui::ScrollArea::vertical().id_salt("cs-roots").max_height(h).auto_shrink([false, false]).show(ui, |ui| {
                    for pc in 0..12u8 {
                        let (resp, _) = list_row(ui, w_root - 8.0, 30.0, self.root == pc, scroll, &theory::root_label(pc));
                        if resp.clicked() {
                            self.root = pc;
                            self.voicing = 0;
                        }
                    }
                });
            });
            // ---- types, or the matching chords or scales
            ui.allocate_ui_with_layout(Vec2::new(w_type, h), Layout::top_down(Align::Min), |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                if self.related {
                    self.related_list(ui, env, w_type, h);
                } else {
                    self.type_list(ui, w_type, h, scroll);
                }
            });
            // ---- positions or fingerings
            ui.allocate_ui_with_layout(Vec2::new(w_third, h), Layout::top_down(Align::Min), |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                if self.mode == Mode::Chords {
                    self.voicing_list(ui, w_third, h, tuning, scroll, act);
                } else {
                    self.position_list(ui, w_third, h, tuning, scroll);
                }
            });
            // ---- buttons
            ui.allocate_ui_with_layout(Vec2::new(w_side, h), Layout::top_down(Align::Min), |ui| {
                ui.spacing_mut().item_spacing.y = 6.0;
                self.buttons(ui, w_side, env, tuning, act);
            });
        });
    }

    fn type_list(&mut self, ui: &mut Ui, w: f32, h: f32, scroll: bool) {
        egui::ScrollArea::vertical().id_salt(("cs-types", self.mode as u8)).max_height(h).auto_shrink([false, false]).show(ui, |ui| {
            let w = w - 8.0;
            if self.mode == Mode::Scales {
                let mut group = "";
                for (k, s) in SCALES.iter().enumerate() {
                    if s.group != group {
                        group = s.group;
                        header_row(ui, w, group);
                    }
                    let (resp, _) = list_row(ui, w, 30.0, self.scale == k, scroll, s.name);
                    if resp.clicked() {
                        self.scale = k;
                        self.pos = None;
                    }
                }
            } else {
                let mut group = "";
                for (k, c) in CHORDS.iter().enumerate() {
                    if c.group != group {
                        group = c.group;
                        header_row(ui, w, group);
                    }
                    let cur = if self.mode == Mode::Chords { self.chord } else { self.arp };
                    let (resp, _) = list_row(ui, w, 30.0, cur == k, scroll, c.label);
                    if resp.clicked() {
                        if self.mode == Mode::Chords {
                            self.chord = k;
                            self.voicing = 0;
                        } else {
                            self.arp = k;
                            self.pos = None;
                        }
                    }
                }
            }
        });
    }

    /// The chords on a scale, or the scales that fit a chord or arpeggio.
    fn related_list(&mut self, ui: &mut Ui, env: &Env, w: f32, h: f32) {
        let w = w - 8.0;
        let mut jump: Option<(bool, u8, usize)> = None;
        if self.from_song {
            header_row(ui, w, &format!("Scales that fit the notes in the {}", env.song_label));
            if list_row(ui, w, 26.0, false, false, "\u{25C0} Back").0.clicked() {
                self.related = false;
                self.from_song = false;
            }
            egui::ScrollArea::vertical().id_salt("cs-related").max_height(h - 46.0).auto_shrink([false, false]).show(ui, |ui| {
                if env.song_pcs.is_empty() {
                    ui.label(RichText::new("There are no notes there yet.").color(Color32::from_gray(160)));
                }
                for (root, scale) in theory::scales_containing(&env.song_pcs, env.song_pcs.first().copied().unwrap_or(0)).into_iter().take(80) {
                    let text = format!("{} {}", theory::note_name(root, theory::flats_for(root)), SCALES[scale].name);
                    if list_row(ui, w, 28.0, false, false, &text).0.clicked() {
                        jump = Some((false, root, scale));
                    }
                }
            });
        } else if self.mode == Mode::Scales {
            header_row(ui, w, "Matching Chords \u{2013} click one to see it");
            if list_row(ui, w, 26.0, false, false, "\u{25C0} Back to the scales").0.clicked() {
                self.related = false;
            }
            egui::ScrollArea::vertical().id_salt("cs-related").max_height(h - 46.0).auto_shrink([false, false]).show(ui, |ui| {
                for (root, kind, degree) in theory::chords_for_scale(self.root, self.scale) {
                    let text = format!("{:<5}{}", crate::score::roman(degree as u8), theory::chord_display(root, kind, None));
                    if list_row(ui, w, 28.0, false, false, &text).0.clicked() {
                        jump = Some((true, root, kind));
                    }
                }
            });
        } else {
            let kind = if self.mode == Mode::Arpeggios { self.arp } else { self.chord };
            header_row(ui, w, "Matching Scales \u{2013} click one to see it");
            if list_row(ui, w, 26.0, false, false, "\u{25C0} Back").0.clicked() {
                self.related = false;
            }
            egui::ScrollArea::vertical().id_salt("cs-related").max_height(h - 46.0).auto_shrink([false, false]).show(ui, |ui| {
                for (root, scale) in theory::scales_for_chord(self.root, kind) {
                    let text = format!("{} {}", theory::note_name(root, theory::flats_for(root)), SCALES[scale].name);
                    if list_row(ui, w, 28.0, false, false, &text).0.clicked() {
                        jump = Some((false, root, scale));
                    }
                }
            });
        }
        match jump {
            Some((true, root, kind)) => self.goto_chord(root, kind),
            Some((false, root, scale)) => self.goto_scale(root, scale),
            None => {}
        }
    }

    fn position_list(&mut self, ui: &mut Ui, w: f32, h: f32, tuning: &[i32; 6], scroll: bool) {
        header_row(ui, w - 8.0, "Position");
        let positions = self.positions(tuning);
        egui::ScrollArea::vertical().id_salt("cs-pos").max_height(h - 20.0).auto_shrink([false, false]).show(ui, |ui| {
            if list_row(ui, w - 8.0, 30.0, self.pos.is_none(), scroll, "All").0.clicked() {
                self.pos = None;
            }
            for p in positions {
                if list_row(ui, w - 8.0, 30.0, self.pos == Some(p), scroll, &theory::position_name(p)).0.clicked() {
                    self.pos = Some(p);
                }
            }
        });
    }

    fn voicing_list(&mut self, ui: &mut Ui, w: f32, h: f32, tuning: &[i32; 6], scroll: bool, act: &mut Option<Act>) {
        header_row(ui, w - 8.0, "Fingerings");
        let list = self.voicings(tuning);
        if list.is_empty() {
            ui.label(RichText::new("No playable fingering found.").color(Color32::from_gray(160)));
            return;
        }
        egui::ScrollArea::vertical().id_salt("cs-voicings").max_height(h - 20.0).auto_shrink([false, false]).show(ui, |ui| {
            for (k, f) in list.iter().enumerate() {
                let lo = chordshape::lowest_fret(f);
                let name = if lo <= 3 && f.contains(&0) || lo == 0 { "Open".to_string() } else { crate::score::roman(lo as u8).to_string() };
                let on = self.voicing == k;
                let (resp, r) = list_row(ui, w - 8.0, 46.0, on, scroll, &name);
                mini_diagram(&ui.painter().clone(), Rect::from_min_size(Pos2::new(r.right() - 66.0, r.top() + 5.0), Vec2::new(60.0, 40.0)), f, Color32::from_gray(235));
                if resp.clicked() {
                    self.voicing = k;
                    if self.auto_play {
                        *act = Some(Act::Play(chord_notes_to_play(f), *tuning));
                    }
                }
            }
        });
    }

    fn buttons(&mut self, ui: &mut Ui, w: f32, env: &Env, tuning: &[i32; 6], act: &mut Option<Act>) {
        match self.mode {
            Mode::Chords => {
                let label = ["Notes", "Intervals", "Fingers"][self.label.min(2) as usize];
                // Chords can show fingers; scales and arpeggios only notes or intervals.
                if side_button(ui, w, label, "\u{21BB}", None, true).clicked() {
                    self.label = (self.label + 1) % 3;
                }
                if side_button(ui, w, "Matching Scales", "\u{27A6}", None, true).clicked() {
                    self.related = !self.related;
                }
                let list = self.voicings(tuning);
                let cur = list.get(self.voicing.min(list.len().saturating_sub(1))).copied();
                if side_button(ui, w, "\u{25B6} Play", "", None, cur.is_some()).clicked()
                    && let Some(f) = cur
                {
                    *act = Some(Act::Play(chord_notes_to_play(&f), *tuning));
                }
                let mut auto = self.auto_play;
                if side_button(ui, w, "Auto-strum", "", Some(auto), true).clicked() {
                    auto = !auto;
                    self.auto_play = auto;
                }
                let name = theory::chord_name(self.root, self.chord, None);
                if let Some(f) = cur
                    && env.can_put
                    && side_button(ui, w, "Put on beat", "", None, true).on_hover_text("Name the beat under the cursor and show its diagram (an empty beat gets the notes too)").clicked()
                {
                    *act = Some(Act::Put { name: name.clone(), frets: f });
                }
                if side_button(ui, w, &format!("Copy \u{201C}{name}\u{201D}"), "", None, true).on_hover_text("Copy the chord name to paste into a chart or text").clicked() {
                    *act = Some(Act::Copy(name));
                }
            }
            m => {
                let label = ["Notes", "Intervals"][(self.label % 2) as usize];
                if side_button(ui, w, label, "\u{21BB}", None, true).clicked() {
                    self.label = (self.label + 1) % 2;
                }
                let matching = if m == Mode::Scales { "Matching Chords" } else { "Matching Scales" };
                if side_button(ui, w, matching, "\u{27A6}", None, true).clicked() {
                    self.related = !self.related || self.from_song;
                    self.from_song = false;
                }
                if m == Mode::Scales && env.can_put && side_button(ui, w, &format!("Scales in the {}", env.song_label), "\u{27A6}", None, true).on_hover_text("Find the scales that hold every note of the selected bars (or this bar)").clicked() {
                    self.related = true;
                    self.from_song = true;
                }
                if m == Mode::Arpeggios && side_button(ui, w, "Show as Chord", "\u{27A6}", None, true).clicked() {
                    let (r, k) = (self.root, self.arp);
                    self.goto_chord(r, k);
                }
                if side_button(ui, w, "\u{25B6} Play", "", None, true).clicked() {
                    *act = Some(Act::Play(self.pattern_notes(tuning), *tuning));
                }
                let mut on = self.overlay;
                if side_button(ui, w, "On main fretboard", "", Some(on), true).on_hover_text("Also show these notes on the fretboard under the score").clicked() {
                    on = !on;
                    self.overlay = on;
                }
            }
        }
    }

    /// The notes of the scale or arpeggio, going up from the root to its octave.
    fn pattern_notes(&self, tuning: &[i32; 6]) -> Vec<(f32, u8, u8)> {
        let low = tuning[0];
        let base = low + (self.root as i32 - low).rem_euclid(12);
        let mut iv = self.pattern();
        iv.sort();
        let step = if self.mode == Mode::Arpeggios { 0.22 } else { 0.26 };
        iv.iter()
            .chain(std::iter::once(&12u8))
            .enumerate()
            .filter_map(|(k, i)| {
                let pitch = base + *i as i32;
                // The string where this pitch is nearest the nut.
                (0..6)
                    .filter_map(|s| {
                        let f = pitch - tuning[s];
                        (0..=FRETS as i32).contains(&f).then_some((s, f as u8))
                    })
                    .min_by_key(|&(_, f)| f)
                    .map(|(s, f)| (k as f32 * step, (5 - s) as u8, f))
            })
            .collect()
    }

    fn identify_panel(&mut self, ui: &mut Ui, tuning: &[i32; 6], act: &mut Option<Act>) {
        let h = 252.0;
        let avail = ui.available_width();
        let (w_side, w_scales) = (196.0, 260.0);
        let w_names = (avail - w_side - w_scales - 3.0 * ui.spacing().item_spacing.x - 6.0).max(220.0);
        let found = self.found(tuning);
        let (bass, pcs) = self.picked(tuning);
        let mut jump: Option<(bool, u8, usize)> = None;
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(Vec2::new(w_names, h), Layout::top_down(Align::Min), |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                let w = w_names - 8.0;
                if pcs.len() < 2 {
                    header_row(ui, w, "Chord Name");
                    ui.add_space(10.0);
                    ui.label(RichText::new("Pick at least two notes on the fretboard.").color(Color32::from_gray(170)));
                    return;
                }
                egui::ScrollArea::vertical().id_salt("cs-names").max_height(h).auto_shrink([false, false]).show(ui, |ui| {
                    header_row(ui, w, "Chord Name");
                    match found.first() {
                        Some(f) => {
                            let text = theory::chord_display(f.root, f.kind, Some(f.bass));
                            let (r, resp) = ui.allocate_exact_size(Vec2::new(w, 42.0), Sense::click());
                            ui.painter().rect_filled(r, 0.0, if resp.hovered() { ROW_HOVER } else { Color32::from_rgb(52, 56, 66) });
                            ui.painter().text(Pos2::new(r.left() + 10.0, r.center().y), Align2::LEFT_CENTER, &text, FontId::new(20.0, FontFamily::Name("bold".into())), Color32::WHITE);
                            if resp.clicked() {
                                jump = Some((true, f.root, f.kind));
                            }
                        }
                        None => {
                            ui.add_space(8.0);
                            ui.label(RichText::new("These notes don't make a chord GuitarCraft knows.").color(Color32::from_gray(170)));
                        }
                    }
                    if found.len() > 1 {
                        header_row(ui, w, "Also Named");
                        for f in found.iter().skip(1) {
                            if list_row(ui, w, 30.0, false, false, &theory::chord_display(f.root, f.kind, Some(f.bass))).0.clicked() {
                                jump = Some((true, f.root, f.kind));
                            }
                        }
                    }
                });
            });
            ui.allocate_ui_with_layout(Vec2::new(w_scales, h), Layout::top_down(Align::Min), |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                header_row(ui, w_scales - 8.0, "Scales with these notes");
                if pcs.is_empty() {
                    return;
                }
                egui::ScrollArea::vertical().id_salt("cs-scales-with").max_height(h - 20.0).auto_shrink([false, false]).show(ui, |ui| {
                    for (root, scale) in theory::scales_containing(&pcs, bass).into_iter().take(60) {
                        let text = format!("{} {}", theory::note_name(root, theory::flats_for(root)), SCALES[scale].name);
                        if list_row(ui, w_scales - 8.0, 28.0, false, false, &text).0.clicked() {
                            jump = Some((false, root, scale));
                        }
                    }
                });
            });
            ui.allocate_ui_with_layout(Vec2::new(w_side, h), Layout::top_down(Align::Min), |ui| {
                ui.spacing_mut().item_spacing.y = 6.0;
                if side_button(ui, w_side, "\u{2715} Clear", "", None, self.picks.iter().any(|p| p.is_some())).clicked() {
                    self.picks = [None; 6];
                }
                if side_button(ui, w_side, "\u{25B6} Play", "", None, pcs.len() >= 2).clicked() {
                    *act = Some(Act::Play(self.pick_notes(), *tuning));
                }
                let mut auto = self.auto_play;
                if side_button(ui, w_side, "Auto-strum", "", Some(auto), true).clicked() {
                    auto = !auto;
                    self.auto_play = auto;
                }
                if let Some(f) = found.first()
                    && side_button(ui, w_side, &format!("Copy \u{201C}{}\u{201D}", theory::chord_name(f.root, f.kind, Some(f.bass))), "", None, true).clicked()
                {
                    *act = Some(Act::Copy(theory::chord_name(f.root, f.kind, Some(f.bass))));
                }
            });
        });
        match jump {
            Some((true, root, kind)) => {
                // Go to the chord, on the fingering just played when it is one of its fingerings.
                let picks = self.picks;
                self.goto_chord(root, kind);
                let played: [i8; 6] = std::array::from_fn(|s| picks[s].map(|f| f as i8).unwrap_or(-1));
                if let Some(k) = self.voicings(tuning).iter().position(|v| *v == played) {
                    self.voicing = k;
                }
            }
            Some((false, root, scale)) => self.goto_scale(root, scale),
            None => {}
        }
    }
}

/// A fingering's notes as a strum, low string first.
fn chord_notes_to_play(f: &[i8; 6]) -> Vec<(f32, u8, u8)> {
    (0..6).filter(|&s| f[s] >= 0).enumerate().map(|(i, s)| (i as f32 * 0.045, (5 - s) as u8, f[s] as u8)).collect()
}

impl App {
    /// Opens the Chords & Scales tool on one of its tabs.
    pub(super) fn open_chord_scales(&mut self, mode: Mode) {
        self.cs.open = true;
        self.cs.mode = mode;
        self.cs.identify = false;
        self.cs.related = false;
        self.cs.scroll = true;
    }

    pub(super) fn chord_scales_window(&mut self, ctx: &egui::Context) {
        if !self.cs.open {
            return;
        }
        let tr = self.tr();
        // The current track's tuning, lowest string first, when it is a six-string.
        let track_tuning: Option<[i32; 6]> = (tr.has_strings() && tr.strings() == 6).then(|| std::array::from_fn(|s| tr.tuning[5 - s] as i32));
        let can_put = self.chart.is_none();
        let (song_pcs, song_label) = if can_put { self.song_notes() } else { (vec![], String::new()) };
        let env = Env { track_tuning, can_put, song_pcs, song_label };
        let mut cs = std::mem::take(&mut self.cs);
        let mut act: Option<Act> = None;
        let mut open = true;
        let screen = ctx.content_rect();
        egui::Window::new("Chords & Scales")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(960.0_f32.min(screen.width() - 40.0))
            .min_width(820.0)
            .pivot(Align2::CENTER_CENTER)
            .default_pos(screen.center())
            .constrain(true)
            .show(ctx, |ui| cs.ui(ui, &env, &mut act));
        cs.open = open && !ctx.input(|i| i.key_pressed(egui::Key::Escape) && !ctx.egui_wants_keyboard_input());
        self.cs = cs;
        match act {
            Some(Act::Play(notes, tuning)) => self.play_cs(&notes, &tuning),
            Some(Act::Copy(name)) => {
                ctx.copy_text(name.clone());
                self.status = format!("Copied {name}.");
            }
            Some(Act::Put { name, frets }) => self.put_chord(name, frets),
            None => {}
        }
    }

    /// The notes (0 to 11) in the selected bars, or the bar under the cursor, with a name for where they are.
    fn song_notes(&self) -> (Vec<u8>, String) {
        let (a, b, label) = match self.selection() {
            Some((a, b)) => (a.0, b.0, "selection"),
            None => (self.bar, self.bar, "bar"),
        };
        let tr = self.tr();
        let mut pcs: Vec<u8> = (a..=b.min(tr.bars.len().saturating_sub(1)))
            .flat_map(|bi| tr.bars[bi].beats.iter().flat_map(|bt| bt.notes.iter().map(|n| tr.pitch(n) % 12)))
            .collect();
        pcs.sort();
        pcs.dedup();
        (pcs, label.to_string())
    }

    /// Plays notes (when, string counted from the highest, fret) as a preview.
    fn play_cs(&mut self, notes: &[(f32, u8, u8)], tuning: &[i32; 6]) {
        if !self.sound_allowed {
            return;
        }
        self.ensure_audio();
        let Some(a) = &self.audio else { return };
        if a.error.is_some() {
            return;
        }
        let sr = a.sample_rate();
        let mut tr: Track = if self.tr().has_strings() { self.tr().clone() } else { Track::default() };
        tr.tuning = tuning.iter().rev().map(|&m| m as u8).collect();
        tr.capo = 0;
        let ti = self.track;
        if let Ok(mut st) = a.state.lock() {
            for &(t, s, f) in notes {
                let mut ev = crate::audio::note_preview(&tr, ti, &Note::new(s, f), sr);
                ev.start = (t * sr) as u64;
                ev.len = (sr * 1.3) as u64;
                st.preview.push(ev);
            }
        }
    }

    /// Puts a chord on the beat under the cursor: its name and diagram, and its notes on an empty beat.
    fn put_chord(&mut self, name: String, frets: [i8; 6]) {
        let tr = self.tr();
        if !tr.has_strings() || tr.strings() != 6 {
            self.status = "Chords are put on beats of a six-string track.".into();
            return;
        }
        let mut notes: Vec<Note> = (0..6).filter(|&s| frets[s] >= 0).map(|s| Note::new((5 - s) as u8, frets[s] as u8)).collect();
        notes.sort_by_key(|n| n.string);
        let empty = self.cur_beat().is_rest();
        let shown = name.clone();
        self.edit(move |s, t, b, j, _| {
            let beat = &mut s.tracks[t].bars[b].beats[j];
            beat.chord = Some(name);
            beat.chord_diagram = true;
            if empty {
                beat.notes = notes;
            }
        });
        self.status = format!("Put {shown} on this beat.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chord_marks_show_fingers_and_muted_strings() {
        let cs = ChordScales { mode: Mode::Chords, ..Default::default() };
        // C major: x32010
        let marks = cs.chord_marks(&[-1, 3, 2, 0, 1, 0]);
        assert_eq!(marks.iter().filter(|m| m.kind == MarkKind::Muted).count(), 1);
        assert_eq!(marks.iter().filter(|m| m.kind == MarkKind::Ring).count(), 2);
        let fingered: Vec<(usize, u8, String)> = marks.iter().filter(|m| m.kind == MarkKind::Finger).map(|m| (m.s, m.fret, m.text.clone())).collect();
        assert_eq!(fingered, vec![(1, 3, "3".to_string()), (2, 2, "2".to_string()), (4, 1, "1".to_string())]);
    }

    #[test]
    fn scale_marks_cover_the_neck_and_a_position() {
        let tuning = TUNINGS[0].1;
        let cs = ChordScales { mode: Mode::Scales, root: 2, scale: 0, ..Default::default() };
        let all = cs.pattern_marks(&tuning, None);
        // D major on the open D string: D at fret 0, E at 2, F# at 4.
        assert!(all.iter().any(|m| m.s == 2 && m.fret == 0 && m.text == "D" && m.kind == MarkKind::Root));
        assert!(all.iter().any(|m| m.s == 2 && m.fret == 4 && m.text == "F\u{266F}"));
        assert!(!all.iter().any(|m| m.s == 2 && m.fret == 1));
        let pos = cs.pattern_marks(&tuning, Some(5));
        assert!(!pos.is_empty() && pos.iter().all(|m| (5..10).contains(&m.fret)));
        assert!(cs.positions(&tuning).contains(&0));
        // Intervals instead of names.
        let cs = ChordScales { label: 1, ..cs };
        assert!(cs.pattern_marks(&tuning, None).iter().any(|m| m.text == "R"));
    }

    #[test]
    fn identify_picks_name_the_chord() {
        let tuning = TUNINGS[0].1;
        // Open E major: 022100.
        let mut cs = ChordScales::default();
        for (s, f) in [0, 2, 2, 1, 0, 0].iter().enumerate() {
            cs.picks[s] = Some(*f);
        }
        let found = cs.found(&tuning);
        assert_eq!(theory::chord_name(found[0].root, found[0].kind, Some(found[0].bass)), "E");
        assert_eq!(cs.pick_notes().len(), 6);
        // The same notes with the G# lowest is E/G#.
        let mut cs = ChordScales::default();
        cs.picks[0] = Some(4);
        cs.picks[2] = Some(2);
        cs.picks[4] = Some(0);
        let found = cs.found(&tuning);
        assert_eq!(theory::chord_name(found[0].root, found[0].kind, Some(found[0].bass)), "E/G#");
    }

    #[test]
    fn scales_play_from_the_root_to_its_octave() {
        let tuning = TUNINGS[0].1;
        let cs = ChordScales { mode: Mode::Scales, root: 4, scale: 0, ..Default::default() };
        let notes = cs.pattern_notes(&tuning);
        assert_eq!(notes.len(), 8);
        // E major from the open low E: string 6 counted from the top is 5.
        assert_eq!((notes[0].1, notes[0].2), (5, 0));
        assert!(notes.windows(2).all(|w| w[0].0 < w[1].0));
    }
}
