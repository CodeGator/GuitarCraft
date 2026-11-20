//! The Chord window (Note > Chord..., or double-clicking a chord in the library), laid out like
//! Guitar Pro's: the chord is built from a tonality, a type and an extension, then altered,
//! inverted or thinned out, with a large diagram of the chosen fingering and a list of
//! alternative fingerings to pick from. The name field at the top is the chord's name as it will
//! appear on the score; typing in it moves the controls and using the controls rewrites it.

use super::App;
use super::stylesheet::{heading, switch};
use crate::chart::split_chord;
use crate::chordshape;
use crate::model::*;
use crate::theory;
use egui::{Align2, Color32, FontFamily, FontId, Pos2, Rect, RichText, Sense, Stroke, Vec2};

const BLUE: Color32 = Color32::from_rgb(77, 138, 255);
const LETTERS: [(&str, i32); 7] = [("C", 0), ("D", 2), ("E", 4), ("F", 5), ("G", 7), ("A", 9), ("B", 11)];
/// The chord's type: what the dropdown says, and what it adds to the name.
const TYPES: [(&str, &str); 7] = [("M", ""), ("m", "m"), ("dim", "dim"), ("aug", "aug"), ("sus2", "sus2"), ("sus4", "sus4"), ("5", "5")];
/// The extensions: what the dropdown says (None is no extension) and what they add.
const EXTS: [&str; 11] = ["", "6", "7", "maj7", "9", "maj9", "11", "13", "add9", "6/9", "add11"];

/// What the controls say. (Alterations: -1 lowered, 1 raised, 0 none.)
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Sel {
    pub letter: usize,
    pub acc: i8,
    pub ty: usize,
    pub ext: usize,
    pub a5: i8,
    pub a9: i8,
    pub a11: i8,
    pub a13: i8,
    /// A different bass note, as a letter and accidental.
    pub bass: Option<(usize, i8)>,
}

fn acc_text(acc: i8) -> &'static str {
    match acc {
        -1 => "b",
        1 => "#",
        _ => "",
    }
}

fn ext_label(e: usize) -> &'static str {
    if e == 0 { "None" } else { EXTS[e] }
}

impl Sel {
    pub fn root_pc(&self) -> i32 {
        (LETTERS[self.letter].1 + self.acc as i32).rem_euclid(12)
    }

    pub fn bass_pc(&self) -> Option<i32> {
        self.bass.map(|(l, a)| (LETTERS[l].1 + a as i32).rem_euclid(12)).filter(|b| *b != self.root_pc())
    }

    /// The name after the root and before the bass ("m7b5"), when it is a chord GuitarCraft reads.
    pub fn suffix(&self) -> Option<String> {
        let ext = EXTS[self.ext];
        let base: String = match (TYPES[self.ty].1, ext) {
            ("", e) => e.to_string(),
            ("m", "") => "m".into(),
            ("m", "maj7") => "m(maj7)".into(),
            ("m", "maj9") => "m(maj9)".into(),
            ("m", e) => format!("m{e}"),
            ("dim", "") => "dim".into(),
            ("dim", "7") => "dim7".into(),
            ("dim", "9") => "dim9".into(),
            ("aug", "") => "aug".into(),
            ("aug", "7") => "7#5".into(),
            ("aug", "maj7") => "maj7#5".into(),
            ("aug", "9") => "9#5".into(),
            ("sus2", "") => "sus2".into(),
            ("sus2", "7") => "7sus2".into(),
            ("sus4", "") => "sus4".into(),
            ("sus4", "7") => "7sus4".into(),
            ("sus4", "9") => "9sus4".into(),
            ("5", "") => "5".into(),
            ("5", "add9") => "5add9".into(),
            _ => return None,
        };
        let mut tokens = String::new();
        let add = |t: &mut String, a: i8, down: &str, up: &str| match a {
            -1 => t.push_str(down),
            1 => t.push_str(up),
            _ => {}
        };
        add(&mut tokens, self.a5, "b5", "#5");
        add(&mut tokens, self.a9, "b9", "#9");
        add(&mut tokens, self.a11, "", "#11");
        add(&mut tokens, self.a13, "b13", "");
        let s = if tokens.is_empty() {
            base
        } else if EXTS[self.ext].is_empty() {
            // A plain triad: the alteration goes in brackets, "C(b5)".
            format!("{base}({tokens})")
        } else if self.a9 != 0 || self.a11 != 0 || self.a13 != 0 {
            // An altered extension sits on a seventh chord: "7b5b9", "m7#11".
            let m = if TYPES[self.ty].1 == "m" { "m" } else { "" };
            format!("{m}7{tokens}")
        } else {
            format!("{base}{tokens}")
        };
        chordshape::formula(&s).map(|_| s)
    }

    /// The chord's whole name as written on the score.
    pub fn name(&self) -> Option<String> {
        let mut n = format!("{}{}{}", LETTERS[self.letter].0, acc_text(self.acc), self.suffix()?);
        if self.bass_pc().is_some() {
            let (l, a) = self.bass.unwrap_or((0, 0));
            n.push('/');
            n.push_str(LETTERS[l].0);
            n.push_str(acc_text(a));
        }
        Some(n)
    }

    /// Can this alteration be chosen with the chord as it is? (kind: 5, 9, 11 or 13)
    pub fn allows(&self, kind: u8, value: i8) -> bool {
        let triad_ok = matches!(TYPES[self.ty].1, "" | "m");
        let seventh = matches!(EXTS[self.ext], "7" | "9");
        let ok = match kind {
            5 => triad_ok,
            9 => triad_ok && seventh,
            11 => triad_ok && seventh && value == 1,
            _ => triad_ok && seventh && value == -1,
        };
        if !ok {
            return false;
        }
        let mut s = self.clone();
        match kind {
            5 => s.a5 = value,
            9 => s.a9 = value,
            11 => s.a11 = value,
            _ => s.a13 = value,
        }
        s.suffix().is_some()
    }
}

/// Reads a chord name into the controls. None when the name isn't one the controls can say.
pub fn parse(name: &str) -> Option<Sel> {
    let n = name.trim().replace('\u{266F}', "#").replace('\u{266D}', "b");
    let (_, rest, bass) = split_chord(&n)?;
    let mut ch = n.chars();
    let first = ch.next()?;
    let letter = LETTERS.iter().position(|l| l.0.starts_with(first))?;
    let acc = match ch.next() {
        Some('#') => 1,
        Some('b') => -1,
        _ => 0,
    };
    let root = Sel { letter, acc, ..Sel::default() };
    let flats = theory::flats_for(root.root_pc() as u8) || acc < 0;
    for ty in 0..TYPES.len() {
        for ext in 0..EXTS.len() {
            for a5 in [0, -1, 1] {
                for a9 in [0, -1, 1] {
                    for a11 in [0, 1] {
                        for a13 in [0, -1] {
                            let mut s = Sel { ty, ext, a5, a9, a11, a13, ..root.clone() };
                            if s.suffix().as_deref() == Some(rest.as_str()) {
                                s.bass = bass.map(|b| spell(b, flats));
                                return Some(s);
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

/// A note (0 to 11) as a letter and accidental.
fn spell(pc: i32, flats: bool) -> (usize, i8) {
    let pc = pc.rem_euclid(12);
    if let Some(l) = LETTERS.iter().position(|l| l.1 == pc) {
        return (l, 0);
    }
    if flats {
        let l = LETTERS.iter().position(|l| l.1 == (pc + 1) % 12).unwrap_or(0);
        (l, -1)
    } else {
        let l = LETTERS.iter().position(|l| l.1 == (pc + 11) % 12).unwrap_or(0);
        (l, 1)
    }
}

/// How the chord's notes are named in that chord: "T", "3", "5", "7", "9" ...
fn tone_label(iv: i32, tones: &[i32]) -> &'static str {
    let third = tones.contains(&3) || tones.contains(&4);
    let seventh = tones.contains(&10) || tones.contains(&11);
    match iv {
        0 => "T",
        1 => if third { "\u{266D}9" } else { "\u{266D}2" },
        2 => if third { "9" } else { "2" },
        3 | 4 => "3",
        5 => if third { "11" } else { "4" },
        6 => if tones.contains(&7) { "\u{266F}11" } else { "\u{266D}5" },
        7 => "5",
        8 => if tones.contains(&7) { "\u{266D}13" } else { "\u{266F}5" },
        9 => if seventh { "13" } else { "6" },
        _ => "7",
    }
}

/// The chord's notes above the root, lowest in the chord's stacking order first.
fn stacked(tones: &[i32]) -> Vec<i32> {
    let third = tones.contains(&3) || tones.contains(&4);
    let seventh = tones.contains(&10) || tones.contains(&11);
    let place = |iv: i32| match iv {
        1 if third => 13,
        2 if third => 14,
        5 if third => 17,
        6 if tones.contains(&7) => 18,
        8 if tones.contains(&7) => 20,
        9 if seventh => 21,
        x => x,
    };
    let mut v = tones.to_vec();
    v.sort_by_key(|&iv| place(iv));
    v
}

/// The window's state.
#[derive(Default, Clone)]
pub struct ChordEd {
    /// The name as it stands in the field.
    pub name: String,
    pub sel: Sel,
    /// The name the controls last wrote, to tell typing from clicking.
    composed: String,
    /// Typed names the controls can't say.
    custom: bool,
    pub omit: Vec<i32>,
    pub show_name: bool,
    pub show_diagram: bool,
    pub highlight_root: bool,
    /// The fingerings on offer, and which one is chosen.
    alts: Vec<[i8; 6]>,
    alts_key: String,
    pick: usize,
    /// How the window began, for Reset.
    start: Option<Box<ChordEd>>,
    /// A fingering the beat already had that the search wouldn't give, and the name it was for.
    given: Option<[i8; 6]>,
    given_for: String,
}

/// The open strings from the lowest to the highest, for a six-string track (standard
/// tuning for any other).
fn tuning_of(t: &Track) -> [i32; 6] {
    if t.has_strings() && t.strings() == 6 {
        let mut o = [0i32; 6];
        for (i, m) in t.tuning.iter().rev().enumerate().take(6) {
            o[i] = *m as i32;
        }
        o
    } else {
        [40, 45, 50, 55, 59, 64]
    }
}

impl ChordEd {
    /// Starts from the beat: its chord, or the chord its notes make, or C.
    pub fn open(beat: &Beat, detected: Option<String>) -> ChordEd {
        let name = beat.chord.clone().or(detected).unwrap_or_else(|| "C".into());
        let mut ed = ChordEd {
            name: name.clone(),
            show_name: !beat.chord_name_hidden,
            show_diagram: beat.chord_diagram,
            highlight_root: true,
            given: beat.chord_frets,
            given_for: beat.chord.clone().unwrap_or_default(),
            ..ChordEd::default()
        };
        ed.read_name();
        ed.composed = ed.name.clone();
        let snapshot = ed.clone();
        ed.start = Some(Box::new(snapshot));
        ed
    }

    /// The controls follow the name.
    pub fn read_name(&mut self) {
        match parse(&self.name) {
            Some(s) => {
                self.sel = s;
                self.custom = false;
            }
            None => self.custom = true,
        }
    }

    fn reset(&mut self) {
        if let Some(s) = self.start.take() {
            let keep = s.clone();
            *self = *s;
            self.start = Some(keep);
        }
    }

    /// The chord's notes above its root, from the name.
    fn tones(&self) -> Vec<i32> {
        let Some((_, rest, _)) = split_chord(&self.name.replace('\u{266F}', "#").replace('\u{266D}', "b")) else {
            return vec![];
        };
        chordshape::formula(&rest).map(|f| f.tones.iter().map(|t| t.0).collect()).unwrap_or_default()
    }

    /// The fingerings for the name, with the beat's own at the front when it isn't among them.
    fn refresh(&mut self, tuning: &[i32; 6]) {
        let key = format!("{}|{:?}|{:?}", self.name, self.omit, tuning);
        if key == self.alts_key {
            return;
        }
        self.alts_key = key;
        let n = self.name.replace('\u{266F}', "#").replace('\u{266D}', "b");
        let mut list = match split_chord(&n) {
            Some((root, rest, bass)) => {
                let omit: Vec<i32> = self.omit.iter().copied().filter(|iv| self.tones().contains(iv)).collect();
                chordshape::voicings_omitting(tuning, root, &rest, &omit, bass, 17)
            }
            None => vec![],
        };
        // The chord's easiest fingering first.
        list.sort_by_key(|v| (chordshape::lowest_fret(v) > 0 && !v.contains(&0)) as u8 * 100 + chordshape::lowest_fret(v) as u8);
        if let Some(g) = self.given
            && !list.contains(&g)
            && n == self.given_for.replace('\u{266F}', "#").replace('\u{266D}', "b")
        {
            list.insert(0, g);
        }
        let given = self.given.filter(|_| n == self.given_for.replace('\u{266F}', "#").replace('\u{266D}', "b"));
        self.pick = given.and_then(|g| list.iter().position(|v| *v == g)).unwrap_or(0).min(list.len().saturating_sub(1));
        self.alts = list;
    }

    /// The chosen fingering.
    pub fn frets(&self) -> Option<[i8; 6]> {
        self.alts.get(self.pick).copied()
    }
}

/// A fingering as marks on a grid: the fret the grid starts at (1 when the fingering fits in
/// the first five frets).
fn base_fret(f: &[i8; 6]) -> i8 {
    let played: Vec<i8> = f.iter().copied().filter(|&v| v > 0).collect();
    let max = played.iter().copied().max().unwrap_or(0);
    if max <= 5 { 1 } else { played.iter().copied().min().unwrap_or(1) }
}

/// Draws a fingering with its first string at `left` and its top fret line at `top`.
#[allow(clippy::too_many_arguments)]
fn paint_fingering(
    p: &egui::Painter,
    left: f32,
    top: f32,
    gap: f32,
    fh: f32,
    f: &[i8; 6],
    ink: Color32,
    big: Option<(&[i32; 6], i32, bool)>,
) {
    let base = base_fret(f);
    let line = Stroke::new(if big.is_some() { 1.2 } else { 0.8 }, ink);
    for s in 0..6 {
        let x = left + s as f32 * gap;
        p.line_segment([Pos2::new(x, top), Pos2::new(x, top + 5.0 * fh)], line);
    }
    for r in 0..=5 {
        let y = top + r as f32 * fh;
        let st = if r == 0 && base == 1 { Stroke::new(if big.is_some() { 3.5 } else { 2.2 }, ink) } else { line };
        p.line_segment([Pos2::new(left, y), Pos2::new(left + 5.0 * gap, y)], st);
    }
    let fs = if big.is_some() { 22.0 } else { 9.0 };
    if base > 1 {
        p.text(Pos2::new(left - gap * 0.55, top + fh * 0.5), Align2::RIGHT_CENTER, base.to_string(), FontId::new(fs, FontFamily::Name("serif-bold".into())), ink);
    }
    let fingers = chordshape::fingers(f);
    let rad = if big.is_some() { gap * 0.34 } else { gap * 0.3 };
    for (s, &fr) in f.iter().enumerate() {
        let x = left + s as f32 * gap;
        let mark = Pos2::new(x, top - if big.is_some() { 13.0 } else { 5.0 });
        let is_root = big.map(|(t, root, hl)| hl && (t[s] + fr.max(0) as i32).rem_euclid(12) == root).unwrap_or(false);
        let fill = if is_root { BLUE } else { Color32::from_gray(25) };
        match fr {
            -1 => {
                let d = if big.is_some() { 5.0 } else { 2.0 };
                p.line_segment([mark + Vec2::new(-d, -d), mark + Vec2::new(d, d)], line);
                p.line_segment([mark + Vec2::new(-d, d), mark + Vec2::new(d, -d)], line);
            }
            0 => {
                if is_root {
                    p.circle_filled(mark, rad * 0.8, BLUE);
                } else {
                    p.circle_stroke(mark, if big.is_some() { 5.0 } else { 2.0 }, line);
                }
            }
            fr => {
                let row = fr as i32 - base as i32;
                if (0..5).contains(&row) {
                    let c = Pos2::new(x, top + (row as f32 + 0.5) * fh);
                    if big.is_some() {
                        p.circle_filled(c, rad, if is_root { BLUE } else { ink });
                        p.text(c, Align2::CENTER_CENTER, fingers[s].max(1).to_string(), FontId::new(15.0, FontFamily::Name("bold".into())), Color32::WHITE);
                    } else {
                        p.circle_filled(c, rad, if ink == Color32::WHITE { ink } else { fill });
                    }
                }
            }
        }
        // The note under each played string.
        if let Some((t, _, _)) = big
            && fr >= 0
        {
            let pc = (t[s] + fr as i32).rem_euclid(12) as u8;
            p.text(Pos2::new(x, top + 5.0 * fh + 10.0), Align2::CENTER_TOP, theory::note_name(pc, false), FontId::new(14.0, FontFamily::Name("serif".into())), ink);
        }
    }
}

impl App {
    pub(super) fn open_chord_tool(&mut self) {
        let pitches: Vec<u8> = self.cur_beat().notes.iter().map(|n| self.tr().pitch(n)).collect();
        let detected = super::tools::detect_chord(&pitches);
        self.chord_ed = ChordEd::open(self.cur_beat(), detected);
        self.chord_open = true;
    }

    /// Plays the chosen fingering as a quick strum.
    fn strum(&mut self, f: [i8; 6], tuning: [i32; 6]) {
        let notes: Vec<(f32, u8, u8)> = (0..6)
            .filter(|&i| f[i] >= 0)
            .enumerate()
            .map(|(k, i)| (k as f32 * 0.045, (5 - i) as u8, f[i] as u8))
            .collect();
        self.play_cs(&notes, &tuning);
    }

    /// Puts the window's chord on the beat under the cursor (or takes the chord off it).
    pub(super) fn apply_chord(&mut self, ed: &ChordEd, delete: bool) {
        let six = self.tr().has_strings() && self.tr().strings() == 6;
        let name = ed.name.trim().to_string();
        let frets = ed.frets().filter(|_| six);
        let (show_name, show_diagram) = (ed.show_name, ed.show_diagram);
        self.edit(move |s, t, b, j, _| {
            let beat = &mut s.tracks[t].bars[b].beats[j];
            if delete || name.is_empty() {
                beat.chord = None;
                beat.chord_diagram = false;
                beat.chord_frets = None;
                beat.chord_name_hidden = false;
            } else {
                beat.chord = Some(name);
                beat.chord_diagram = show_diagram;
                beat.chord_frets = frets;
                beat.chord_name_hidden = !show_name;
            }
        });
        self.status = if delete { "Chord removed.".into() } else { "Chord changed.".into() };
    }

    pub(super) fn chord_window(&mut self, ctx: &egui::Context) {
        if !self.chord_open {
            return;
        }
        let tuning = tuning_of(self.tr());
        let six = self.tr().has_strings() && self.tr().strings() == 6;
        let had = self.cur_beat().chord.is_some();
        let mut ed = std::mem::take(&mut self.chord_ed);
        let mut open = true;
        let mut close = false;
        let mut apply = false;
        let mut delete = false;
        let mut play = false;
        let pitches: Vec<u8> = self.cur_beat().notes.iter().map(|n| self.tr().pitch(n)).collect();
        let detected = super::tools::detect_chord(&pitches);
        let screen = ctx.content_rect();

        // The name field and the controls stay in step.
        let before = ed.sel.clone();
        egui::Window::new("Chord")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .pivot(Align2::CENTER_CENTER)
            .default_pos(screen.center())
            .fixed_size(Vec2::new(650.0, 640.0))
            .constrain(true)
            .show(ctx, |ui| {
                ui.horizontal_top(|ui| {
                    // ----- the left column: how the chord is built
                    ui.vertical(|ui| {
                        ui.set_width(270.0);
                        egui::ScrollArea::vertical().id_salt("chord-left").max_height(575.0).auto_shrink([false, false]).show(ui, |ui| {
                            ui.set_width(262.0);
                            heading(ui, "CHORD");
                            heading(ui, "NAMING");
                            egui::Grid::new("naming").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                                ui.label("Tonality:");
                                ui.horizontal(|ui| {
                                    egui::ComboBox::from_id_salt("tonality").width(54.0).selected_text(LETTERS[ed.sel.letter].0).show_ui(ui, |ui| {
                                        for (i, l) in LETTERS.iter().enumerate() {
                                            ui.selectable_value(&mut ed.sel.letter, i, l.0);
                                        }
                                    });
                                    for (acc, label) in [(-1i8, "\u{266D}"), (1, "\u{266F}")] {
                                        let on = ed.sel.acc == acc;
                                        if ui.add(egui::Button::new(RichText::new(label).size(15.0)).selected(on).min_size(Vec2::new(40.0, 22.0))).clicked() {
                                            ed.sel.acc = if on { 0 } else { acc };
                                        }
                                    }
                                });
                                ui.end_row();
                                ui.label("Type:");
                                egui::ComboBox::from_id_salt("type").width(78.0).selected_text(TYPES[ed.sel.ty].0).show_ui(ui, |ui| {
                                    for (i, t) in TYPES.iter().enumerate() {
                                        ui.selectable_value(&mut ed.sel.ty, i, t.0);
                                    }
                                });
                                ui.end_row();
                                ui.label("Extension:");
                                let sel = ed.sel.clone();
                                egui::ComboBox::from_id_salt("extension").width(78.0).selected_text(if ed.sel.ext == 0 { "" } else { EXTS[ed.sel.ext] }).show_ui(ui, |ui| {
                                    for i in 0..EXTS.len() {
                                        let ok = Sel { ext: i, a5: 0, a9: 0, a11: 0, a13: 0, ..sel.clone() }.suffix().is_some();
                                        ui.add_enabled_ui(ok, |ui| ui.selectable_value(&mut ed.sel.ext, i, ext_label(i)));
                                    }
                                });
                                ui.end_row();
                            });
                            // A type that doesn't go with the extension takes it off.
                            if ed.sel.suffix().is_none() {
                                let s = Sel { ext: 0, a5: 0, a9: 0, a11: 0, a13: 0, ..ed.sel.clone() };
                                if s.suffix().is_some() {
                                    ed.sel = s;
                                }
                            }

                            heading(ui, "ALTERATIONS");
                            egui::Grid::new("alterations").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                                for (kind, label, down, up) in [(5u8, "5th:", "/5-", "/5+"), (9, "9th:", "/9-", "/9+"), (11, "11th:", "/11-", "/11+"), (13, "13th:", "/13-", "/13+")] {
                                    ui.label(label);
                                    ui.horizontal(|ui| {
                                        for (value, text) in [(-1i8, down), (1, up)] {
                                            let cur = match kind {
                                                5 => ed.sel.a5,
                                                9 => ed.sel.a9,
                                                11 => ed.sel.a11,
                                                _ => ed.sel.a13,
                                            };
                                            let can = ed.sel.allows(kind, value) || cur == value;
                                            let r = ui.add_enabled(can, egui::Button::new(text).selected(cur == value).min_size(Vec2::new(46.0, 22.0)));
                                            if r.clicked() {
                                                let v = if cur == value { 0 } else { value };
                                                match kind {
                                                    5 => ed.sel.a5 = v,
                                                    9 => ed.sel.a9 = v,
                                                    11 => ed.sel.a11 = v,
                                                    _ => ed.sel.a13 = v,
                                                }
                                            }
                                        }
                                    });
                                    ui.end_row();
                                }
                            });

                            heading(ui, "INVERSIONS");
                            let tones = ed.tones();
                            let order = stacked(&tones);
                            let flats = theory::flats_for(ed.sel.root_pc() as u8) || ed.sel.acc < 0;
                            egui::Grid::new("inversions").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                                ui.label("Bass:");
                                ui.horizontal(|ui| {
                                    let (bl, ba) = ed.sel.bass.unwrap_or((ed.sel.letter, ed.sel.acc));
                                    let mut l = bl;
                                    let mut a = ba;
                                    egui::ComboBox::from_id_salt("bass-letter").width(48.0).selected_text(LETTERS[l].0).show_ui(ui, |ui| {
                                        for (i, x) in LETTERS.iter().enumerate() {
                                            ui.selectable_value(&mut l, i, x.0);
                                        }
                                    });
                                    egui::ComboBox::from_id_salt("bass-acc").width(40.0).selected_text(match a { -1 => "\u{266D}", 1 => "\u{266F}", _ => "" }).show_ui(ui, |ui| {
                                        ui.selectable_value(&mut a, 0, "\u{266E}");
                                        ui.selectable_value(&mut a, -1, "\u{266D}");
                                        ui.selectable_value(&mut a, 1, "\u{266F}");
                                    });
                                    if (l, a) != (bl, ba) {
                                        ed.sel.bass = Some((l, a));
                                    }
                                });
                                ui.end_row();
                                ui.label("Inversion:");
                                let bass = ed.sel.bass_pc();
                                let cur = match bass {
                                    None => Some(0),
                                    Some(b) => order.iter().take(4).position(|iv| (ed.sel.root_pc() + iv).rem_euclid(12) == b),
                                };
                                let names = ["Root position", "1st inversion", "2nd inversion", "3rd inversion"];
                                egui::ComboBox::from_id_salt("inversion").width(120.0).selected_text(cur.map(|c| names[c]).unwrap_or("")).show_ui(ui, |ui| {
                                    for (k, n) in names.iter().enumerate().take(order.len().min(4)) {
                                        if ui.selectable_label(cur == Some(k), *n).clicked() {
                                            ed.sel.bass = if k == 0 { None } else { Some(spell(ed.sel.root_pc() + order[k], flats)) };
                                        }
                                    }
                                });
                                ui.end_row();
                            });

                            heading(ui, "OMISSIONS");
                            if tones.is_empty() {
                                ui.label(RichText::new("Type a chord name GuitarCraft knows.").weak());
                            } else {
                                let shown: Vec<i32> = order.iter().copied().take(6).collect();
                                ui.horizontal(|ui| {
                                    for &iv in &shown {
                                        let pc = (ed.sel.root_pc() + iv).rem_euclid(12) as u8;
                                        ui.vertical(|ui| {
                                            ui.set_width(34.0);
                                            ui.label(RichText::new(theory::note_name(pc, flats)).size(12.0).weak());
                                            let omitted = ed.omit.contains(&iv);
                                            if ui.add(egui::Button::new(tone_label(iv, &tones)).selected(!omitted).min_size(Vec2::new(32.0, 24.0))).clicked() {
                                                if omitted {
                                                    ed.omit.retain(|x| *x != iv);
                                                } else if ed.omit.len() + 1 < tones.len() {
                                                    ed.omit.push(iv);
                                                }
                                            }
                                        });
                                    }
                                });
                            }

                            heading(ui, "DISPLAY OPTIONS");
                            egui::Grid::new("display").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                                ui.label("Display name:");
                                ui.horizontal(|ui| {
                                    switch(ui, &mut ed.show_name);
                                });
                                ui.end_row();
                                ui.label("Display diagram:");
                                ui.horizontal(|ui| {
                                    switch(ui, &mut ed.show_diagram);
                                });
                                ui.end_row();
                            });
                        });
                    });

                    // ----- the right column: the name, the fingering and its alternatives
                    ui.vertical(|ui| {
                        ui.set_width(340.0);
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            let w = ui.add(egui::TextEdit::singleline(&mut ed.name).desired_width(250.0).font(FontId::proportional(15.0)).margin(Vec2::new(6.0, 5.0)));
                            if w.changed() {
                                ed.read_name();
                                ed.composed = ed.name.clone();
                            }
                            ui.menu_button("\u{22EE}", |ui| {
                                if ui.add_enabled(detected.is_some(), egui::Button::new("Name from the notes on this beat")).clicked() {
                                    if let Some(d) = &detected {
                                        ed.name = d.clone();
                                        ed.read_name();
                                        ed.composed = ed.name.clone();
                                    }
                                    ui.close();
                                }
                                if ui.button("Start again").clicked() {
                                    ed.name = "C".into();
                                    ed.sel = Sel::default();
                                    ed.omit.clear();
                                    ed.custom = false;
                                    ed.composed = "C".into();
                                    ui.close();
                                }
                            });
                            if ui.add_enabled(ed.frets().is_some(), egui::Button::new("\u{25B6}").min_size(Vec2::new(30.0, 26.0))).on_hover_text("Hear the chord").clicked() {
                                play = true;
                            }
                        });
                        if ed.custom && !ed.name.trim().is_empty() {
                            ui.label(RichText::new("The controls can't show this name, but it will be used as typed.").small().weak());
                        }
                        ui.add_space(4.0);
                        ed.refresh(&tuning);
                        let (area, _) = ui.allocate_exact_size(Vec2::new(334.0, 262.0), Sense::hover());
                        let p = ui.painter();
                        p.rect_filled(area, 2.0, Color32::WHITE);
                        let root = ed.sel.root_pc();
                        if !ed.name.trim().is_empty() {
                            p.text(Pos2::new(area.center().x, area.top() + 12.0), Align2::CENTER_TOP, &ed.name, FontId::new(32.0, FontFamily::Name("serif-bold".into())), Color32::BLACK);
                        }
                        match ed.frets() {
                            Some(f) => paint_fingering(p, area.center().x - 2.5 * 33.0, area.top() + 86.0, 33.0, 27.0, &f, Color32::BLACK, Some((&tuning, root, ed.highlight_root))),
                            None => {
                                p.text(area.center() + Vec2::new(0.0, 20.0), Align2::CENTER_CENTER, "No fingering for this chord.", FontId::proportional(13.0), Color32::from_gray(120));
                            }
                        }
                        ui.horizontal(|ui| {
                            ui.add_space(140.0);
                            ui.label("Highlight root note:");
                            switch(ui, &mut ed.highlight_root);
                        });
                        ui.add_space(4.0);
                        ui.label(RichText::new("ALTERNATIVES").size(15.0).family(FontFamily::Name("bold".into())).color(Color32::from_gray(240)));
                        let (box_r, _) = ui.allocate_exact_size(Vec2::new(334.0, 192.0), Sense::hover());
                        ui.painter().rect_filled(box_r, 2.0, Color32::WHITE);
                        let mut inner = ui.new_child(egui::UiBuilder::new().max_rect(box_r));
                        egui::ScrollArea::vertical().id_salt("alts").auto_shrink([false, false]).show(&mut inner, |ui| {
                            let cols = 4usize;
                            let (cw, ch) = (83.0, 90.0);
                            let rows = ed.alts.len().div_ceil(cols).max(1);
                            let (r, _) = ui.allocate_exact_size(Vec2::new(cols as f32 * cw, rows as f32 * ch), Sense::hover());
                            for (k, f) in ed.alts.clone().iter().enumerate() {
                                let cell = Rect::from_min_size(r.min + Vec2::new((k % cols) as f32 * cw, (k / cols) as f32 * ch), Vec2::new(cw, ch)).shrink(2.0);
                                let resp = ui.interact(cell, ui.id().with(("alt", k)), Sense::click());
                                let on = k == ed.pick;
                                let p = ui.painter();
                                if on {
                                    p.rect_filled(cell, 2.0, BLUE);
                                } else if resp.hovered() {
                                    p.rect_filled(cell, 2.0, Color32::from_gray(235));
                                }
                                let ink = if on { Color32::WHITE } else { Color32::BLACK };
                                paint_fingering(p, cell.center().x - 2.5 * 10.0, cell.top() + 20.0, 10.0, 11.5, f, ink, None);
                                if resp.clicked() {
                                    ed.pick = k;
                                }
                            }
                        });
                        if !six {
                            ui.label(RichText::new("Shown on a six-string guitar; only the name is kept on this track.").small().weak());
                        }
                    });
                });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button("Reset").clicked() {
                        ed.reset();
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Cancel").clicked() {
                            close = true;
                        }
                        if ui.add(egui::Button::new(RichText::new("OK").strong()).fill(BLUE).min_size(Vec2::new(60.0, 24.0))).clicked() {
                            apply = true;
                        }
                        if ui.add_enabled(had, egui::Button::new("Delete").min_size(Vec2::new(60.0, 24.0))).clicked() {
                            delete = true;
                        }
                    });
                });
            });

        // Using the controls rewrites the name.
        if ed.sel != before
            && let Some(n) = ed.sel.name()
        {
            ed.name = n.clone();
            ed.composed = n;
            ed.custom = false;
            let tones = ed.tones();
            ed.omit.retain(|iv| tones.contains(iv));
        }
        if close || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            open = false;
        }

        if play && let Some(f) = ed.frets() {
            self.strum(f, tuning);
        }
        if apply || delete {
            self.apply_chord(&ed, delete);
            open = false;
        }
        self.chord_ed = ed;
        self.chord_open = open;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_read_into_the_controls_and_written_back() {
        for n in [
            "C", "Am", "G7", "Cmaj7", "F#m7b5", "Bbmaj7", "Dsus4", "E7#9", "A7sus4", "C/E", "Gadd9", "Dm(maj7)", "Edim7", "C(b5)", "Bb6", "Em9", "F#dim", "Caug", "D9", "G5", "A7b5", "Em7#11",
        ] {
            let sel = parse(n).unwrap_or_else(|| panic!("{n} should be read"));
            assert_eq!(sel.name().as_deref(), Some(n), "{n}");
        }
        assert!(parse("Hello").is_none());
        assert!(parse("Cfoo").is_none());
    }

    #[test]
    fn every_choice_the_controls_offer_makes_a_name_the_app_can_draw() {
        let mut offered = 0;
        for ty in 0..TYPES.len() {
            for ext in 0..EXTS.len() {
                let s = Sel { ty, ext, ..Sel::default() };
                let Some(name) = s.name() else { continue };
                offered += 1;
                assert!(crate::chart::shape(&name).is_some() || crate::chart::split_chord(&name).is_some(), "{name}");
                for (kind, v) in [(5u8, -1i8), (5, 1), (9, -1), (9, 1), (11, 1), (13, -1)] {
                    if s.allows(kind, v) {
                        let mut t = s.clone();
                        match kind {
                            5 => t.a5 = v,
                            9 => t.a9 = v,
                            11 => t.a11 = v,
                            _ => t.a13 = v,
                        }
                        let altered = t.name().expect("an allowed alteration has a name");
                        assert!(parse(&altered).is_some(), "{altered} should be read back");
                    }
                }
            }
        }
        assert!(offered >= 30, "only {offered} combinations");
        // Alterations wait for a chord that has something to alter.
        let plain = Sel::default();
        assert!(plain.allows(5, 1));
        assert!(!plain.allows(9, -1));
        let seventh = Sel { ext: 2, ..Sel::default() };
        assert!(seventh.allows(9, -1) && seventh.allows(11, 1) && seventh.allows(13, -1));
        assert!(!seventh.allows(11, -1));
    }

    #[test]
    fn inversions_follow_the_stacking_of_the_chord() {
        assert_eq!(stacked(&[0, 4, 7, 10]), vec![0, 4, 7, 10]);
        assert_eq!(stacked(&[0, 4, 7, 10, 2]), vec![0, 4, 7, 10, 2]);
        assert_eq!(tone_label(4, &[0, 4, 7]), "3");
        assert_eq!(tone_label(3, &[0, 3, 7]), "3");
        assert_eq!(spell(3, true), (2, -1));
        assert_eq!(spell(1, false), (0, 1));
    }

    #[test]
    fn a_fingering_is_found_for_the_chord_and_leaving_a_note_out_changes_it() {
        let tuning = [40, 45, 50, 55, 59, 64];
        let c = chordshape::voicings_omitting(&tuning, 0, "", &[], None, 17);
        assert!(c.contains(&[-1, 3, 2, 0, 1, 0]), "{c:?}");
        let no5 = chordshape::voicings_omitting(&tuning, 0, "", &[7], None, 17);
        assert!(!no5.is_empty());
        for v in &no5 {
            for (s, &f) in v.iter().enumerate() {
                if f >= 0 {
                    assert_ne!((tuning[s] + f as i32).rem_euclid(12), 7, "a fifth in {v:?}");
                }
            }
        }
    }
}
