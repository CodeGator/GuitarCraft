//! The drumkit view: the kit written on a percussion staff, and below it every drum sound by
//! number. A click puts the sound on the cursor's beat, or takes it off again. The boxed
//! numbers are the number keys that choose among the sounds on the notation cursor's line.

use super::side::VOICE_COLORS;
use super::App;
use crate::drums::{self, drum_name, drum_staff_pos, drums_on_line, DrumSound};
use crate::model::*;
use crate::score;
use egui::epaint::TextShape;
use egui::{Align2, Color32, FontId, Painter, Pos2, Rect, Sense, Stroke as PStroke, StrokeKind, Ui, Vec2};
use serde::{Deserialize, Serialize};

/// Which drum names are written above the staff.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum DrumNames {
    All,
    Hover,
    Hidden,
}

/// The notes the view marks, besides nothing while the song plays.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum DrumMarks {
    Beat,
    BeatAndBar,
    BeatAndNext,
}

/// The view's settings (the gear button), saved with the preferences.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct DrumViewPrefs {
    pub staff: bool,
    pub list: bool,
    pub general_midi_only: bool,
    pub names: DrumNames,
    pub marks: DrumMarks,
    pub large_text: bool,
}

impl Default for DrumViewPrefs {
    fn default() -> Self {
        DrumViewPrefs { staff: true, list: true, general_midi_only: false, names: DrumNames::All, marks: DrumMarks::Beat, large_text: false }
    }
}

const SIDE_W: f32 = 56.0;
const SIDE_BG: Color32 = Color32::from_rgb(40, 43, 50);
const LIST_BG: Color32 = Color32::from_rgb(28, 30, 34);
const PLAYING: Color32 = Color32::from_rgb(240, 130, 20);
const BADGE: Color32 = Color32::from_rgb(214, 158, 30);

/// The sounds marked in the view: those of the beat (or sounding now), and others shown fainter.
struct Marked {
    now: Vec<u8>,
    also: Vec<u8>,
    color: Color32,
}

/// `c` mixed with `into`: `t` = 0 gives `c`, 1 gives `into`.
fn mix(c: Color32, into: Color32, t: f32) -> Color32 {
    let m = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t) as u8;
    Color32::from_rgb(m(c.r(), into.r()), m(c.g(), into.g()), m(c.b(), into.b()))
}

impl App {
    pub(super) fn has_drum_track(&self) -> bool {
        self.song.tracks.iter().any(|t| t.kind() == InstrumentType::Drums)
    }

    pub(super) fn on_drum_track(&self) -> bool {
        self.tr().kind() == InstrumentType::Drums
    }

    /// Takes the drum sound off the cursor's beat if it is there, and puts it on if not.
    pub(super) fn toggle_drum(&mut self, sound: u8) {
        let on = self.cur_beat().notes.iter().find(|n| self.tr().pitch(n) == sound).map(|n| n.string);
        let Some(string) = on else {
            self.put_drum(sound, None);
            return;
        };
        self.edit(move |so, t, b, j, _| so.tracks[t].bars[b].beats[j].notes.retain(|n| n.string != string));
        if self.std_cursor.is_some() {
            self.std_cursor = Some(drum_staff_pos(sound).0);
        }
        self.status = format!("Removed {}.", drum_name(sound));
    }

    /// Puts a drum sound on the cursor's beat, in place of the note on `replace` if given.
    pub(super) fn put_drum(&mut self, sound: u8, replace: Option<u8>) {
        let strings = self.tr().strings();
        let free = replace.map(|s| s as usize).or_else(|| (0..strings).find(|&s| self.cur_beat().note_on(s as u8).is_none()));
        let Some(s) = free else {
            self.refuse(format!("A beat holds at most {strings} drum sounds."));
            return;
        };
        let fret = sound as i32 - self.tr().pitch(&Note::new(s as u8, 0)) as i32;
        if !(0..=self.tr().frets as i32).contains(&fret) {
            self.refuse(format!("{} can't be played on this track.", drum_name(sound)));
            return;
        }
        self.string = s;
        self.set_fret(s, fret as u8);
    }

    fn drum_marks(&self, sounding: &[(usize, u8, u8)]) -> Marked {
        let tr = self.tr();
        if self.playing {
            let now = sounding.iter().filter(|x| x.0 == self.track).map(|&(_, s, f)| tr.pitch(&Note::new(s, f))).collect();
            return Marked { now, also: vec![], color: PLAYING };
        }
        let pitches = |b: &Beat| b.notes.iter().map(|n| tr.pitch(n)).collect::<Vec<u8>>();
        let voice = tr.bars[self.bar].voice(self.voice);
        let also = match self.prefs.drum_view.marks {
            DrumMarks::Beat => vec![],
            DrumMarks::BeatAndBar => voice.iter().flat_map(pitches).collect(),
            DrumMarks::BeatAndNext => voice
                .get(self.beat + 1)
                .or_else(|| tr.bars.get(self.bar + 1).and_then(|b| b.voice(self.voice).first()))
                .map(pitches)
                .unwrap_or_default(),
        };
        Marked { now: pitches(self.cur_beat()), also, color: VOICE_COLORS[self.voice] }
    }

    /// A message in the middle of the panel, when there is nothing to show.
    fn drum_message(ui: &mut Ui, text: String) {
        ui.add_space(30.0);
        ui.vertical_centered(|ui| ui.label(egui::RichText::new(text).size(14.0).color(Color32::from_gray(180))));
    }

    pub(super) fn drum_panel(&mut self, ui: &mut Ui, sounding: &[(usize, u8, u8)]) {
        if !self.has_drum_track() {
            Self::drum_message(ui, "This song has no drum track. Add one with Track \u{25B8} Add Track.".into());
            return;
        }
        if !self.on_drum_track() {
            Self::drum_message(ui, format!("{} is not a drum track.", self.tr().name));
            let first = self.song.tracks.iter().position(|t| t.kind() == InstrumentType::Drums);
            ui.add_space(6.0);
            let go = ui.vertical_centered(|ui| ui.button("Go to the drum track").clicked()).inner;
            if let (true, Some(t)) = (go, first) {
                self.track = t;
                self.clamp_cursor();
                self.scroll_to_cursor = true;
            }
            return;
        }
        let full = ui.max_rect();
        let side = Rect::from_min_size(full.min, Vec2::new(SIDE_W, full.height()));
        self.drum_side(ui, side);
        let area = Rect::from_min_max(Pos2::new(side.right(), full.top()), full.max);
        let view = self.prefs.drum_view.clone();
        let marked = self.drum_marks(sounding);
        let mut clicked = None;
        let mut body = ui.new_child(egui::UiBuilder::new().max_rect(area));
        body.set_clip_rect(area.intersect(ui.clip_rect()));
        egui::ScrollArea::both().id_salt("drumkit").auto_shrink([false, false]).show(&mut body, |ui| {
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            // Room for the scroll bar.
            let width = area.width() - 10.0;
            if view.staff {
                clicked = clicked.or(self.drum_staff(ui, &view, &marked, width));
            }
            if view.list {
                clicked = clicked.or(drum_list(ui, &view, &marked, width));
            }
        });
        ui.allocate_rect(full, Sense::hover());
        if let Some(sound) = clicked {
            self.toggle_drum(sound);
            self.clamp_cursor();
        }
    }

    /// The strip on the left: the previous and next beat, and the settings.
    fn drum_side(&mut self, ui: &mut Ui, side: Rect) {
        ui.painter().rect_filled(side, 0.0, SIDE_BG);
        let button = |ui: &mut Ui, r: Rect, label: &str, tip: &str| {
            let resp = ui.interact(r, ui.id().with(("drum-button", label)), Sense::click()).on_hover_text(tip);
            ui.painter().rect_filled(r, 4.0, Color32::from_gray(if resp.hovered() { 96 } else { 70 }));
            ui.painter().text(r.center(), Align2::CENTER_CENTER, label, FontId::proportional(16.0), Color32::WHITE);
            resp
        };
        let size = Vec2::new(22.0, 24.0);
        let left = Rect::from_min_size(side.min + Vec2::new(5.0, 8.0), size);
        if button(ui, left, "<", "Previous beat").clicked() {
            self.move_left(false);
            self.clamp_cursor();
        }
        if button(ui, left.translate(Vec2::new(24.0, 0.0)), ">", "Next beat").clicked() {
            self.move_right(false);
            self.clamp_cursor();
        }
        let gear = Rect::from_min_size(side.min + Vec2::new(17.0, 40.0), size);
        let resp = button(ui, gear, "\u{2699}", "Drumkit view settings");
        let mut v = self.prefs.drum_view.clone();
        egui::Popup::menu(&resp).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).show(|ui| {
            let staff = ui.checkbox(&mut v.staff, "Staff").changed();
            let list = ui.checkbox(&mut v.list, "List of all sounds").changed();
            // One of the two always shows.
            if !v.staff && !v.list {
                v.list = staff;
                v.staff = list;
            }
            ui.checkbox(&mut v.general_midi_only, "General MIDI sounds only");
            ui.checkbox(&mut v.large_text, "Larger text");
            ui.separator();
            ui.label(egui::RichText::new("Names above the staff").strong());
            ui.radio_value(&mut v.names, DrumNames::All, "All");
            ui.radio_value(&mut v.names, DrumNames::Hover, "Under the mouse");
            ui.radio_value(&mut v.names, DrumNames::Hidden, "None");
            ui.separator();
            ui.label(egui::RichText::new("Mark the notes of").strong());
            ui.radio_value(&mut v.marks, DrumMarks::Beat, "The beat");
            ui.radio_value(&mut v.marks, DrumMarks::BeatAndBar, "The beat and its bar");
            ui.radio_value(&mut v.marks, DrumMarks::BeatAndNext, "The beat and the next one");
        });
        if v != self.prefs.drum_view {
            self.prefs.drum_view = v;
            self.prefs.save();
        }
    }

    /// The kit on a staff, a column for each sound. Returns the sound clicked.
    fn drum_staff(&self, ui: &mut Ui, view: &DrumViewPrefs, marked: &Marked, width: f32) -> Option<u8> {
        let s = if view.large_text { 1.25 } else { 1.0 };
        let kit: Vec<u8> = drums::STAFF_KIT.iter().copied().filter(|p| !view.general_midi_only || drums::GENERAL_MIDI.contains(p)).collect();
        let (col, left) = (28.0 * s, 70.0 * s);
        let names_h = if view.names == DrumNames::Hidden { 6.0 } else { 120.0 * s };
        // The last names lean out past their columns.
        let right = if view.names == DrumNames::Hidden { 14.0 } else { 80.0 * s };
        let size = Vec2::new((left + col * kit.len() as f32 + right).max(width), names_h + 116.0 * s);
        let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
        let p = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
        p.rect_filled(rect, 0.0, Color32::WHITE);
        let ink = Color32::BLACK;
        let grey = Color32::from_gray(120);
        let x0 = rect.left() + left;
        let number_y = rect.top() + names_h + 8.0 * s;
        let badge_y = number_y + 18.0 * s;
        let top_line = badge_y + 34.0 * s;
        let ls = 10.0 * s;
        let bottom = top_line + 4.0 * ls;
        let ypos = |pos: i32| bottom - pos as f32 * ls * 0.5;
        let hovered = resp.hover_pos().and_then(|at| {
            let i = ((at.x - x0) / col).floor();
            (i >= 0.0 && (i as usize) < kit.len()).then_some(i as usize)
        });
        // Column tints go under the staff lines.
        for (i, sound) in kit.iter().enumerate() {
            let r = Rect::from_min_max(Pos2::new(x0 + i as f32 * col + 1.0, number_y - 9.0 * s), Pos2::new(x0 + (i + 1) as f32 * col - 1.0, rect.bottom() - 4.0));
            let tint = if marked.now.contains(sound) {
                Some(mix(marked.color, Color32::WHITE, 0.55))
            } else if marked.also.contains(sound) {
                Some(mix(marked.color, Color32::WHITE, 0.82))
            } else {
                (hovered == Some(i)).then_some(Color32::from_gray(232))
            };
            if let Some(t) = tint {
                p.rect_filled(r, 3.0, t);
            }
            if hovered == Some(i) {
                p.rect_stroke(r, 3.0, PStroke::new(1.0, grey), StrokeKind::Inside);
            }
        }
        let end = x0 + col * kit.len() as f32 + 6.0;
        for line in 0..5 {
            let y = top_line + line as f32 * ls;
            p.line_segment([Pos2::new(rect.left() + 20.0 * s, y), Pos2::new(end, y)], PStroke::new(1.0, ink));
        }
        for dx in [0.0, 8.0] {
            let x = rect.left() + (30.0 + dx) * s;
            p.rect_filled(Rect::from_min_max(Pos2::new(x, top_line + ls), Pos2::new(x + 4.0 * s, top_line + 3.0 * ls)), 0.0, ink);
        }
        p.text(Pos2::new(x0 - 6.0, number_y), Align2::RIGHT_CENTER, "MIDI", FontId::proportional(12.0 * s), grey);
        // The number keys of the notation cursor's line.
        let keys = if self.playing { vec![] } else { self.std_cursor.map(drums_on_line).unwrap_or_default() };
        for (i, &sound) in kit.iter().enumerate() {
            let cx = x0 + (i as f32 + 0.5) * col;
            let (pos, head) = drum_staff_pos(sound);
            let shown = match view.names {
                DrumNames::All => true,
                DrumNames::Hover => hovered == Some(i),
                DrumNames::Hidden => false,
            };
            if shown {
                leaning_text(&p, Pos2::new(cx, number_y - 12.0 * s), drum_name(sound), 13.0 * s, ink);
            }
            p.text(Pos2::new(cx, number_y), Align2::CENTER_CENTER, sound.to_string(), FontId::proportional(12.5 * s), grey);
            if let Some(k) = keys.iter().position(|&x| x == sound) {
                let b = Rect::from_center_size(Pos2::new(cx, badge_y), Vec2::new(16.0, 17.0) * s);
                p.rect_stroke(b, 3.0, PStroke::new(1.3, BADGE), StrokeKind::Inside);
                p.text(b.center(), Align2::CENTER_CENTER, (k + 1).to_string(), FontId::proportional(12.0 * s), mix(BADGE, ink, 0.3));
            }
            // Ledger lines.
            let ledgers: Vec<i32> = if pos >= 10 { (10..=pos).step_by(2).collect() } else { (pos..=-2).filter(|l| l % 2 == 0).collect() };
            for l in ledgers {
                p.line_segment([Pos2::new(cx - 9.0 * s, ypos(l)), Pos2::new(cx + 9.0 * s, ypos(l))], PStroke::new(1.0, ink));
            }
            let color = if marked.now.contains(&sound) { mix(marked.color, ink, 0.6) } else { ink };
            let at = Pos2::new(cx, ypos(pos));
            score::drum_note(&p, at, 1.25 * s, sound, head, true, color);
        }
        let sound = hovered.map(|i| kit[i]);
        resp.clicked().then_some(sound).flatten()
    }
}

/// Text leaning up to the right, its start just above `at`.
fn leaning_text(p: &Painter, at: Pos2, text: &str, size: f32, color: Color32) {
    let angle = -std::f32::consts::FRAC_PI_3;
    let galley = p.layout_no_wrap(text.to_owned(), FontId::proportional(size), color);
    // The text turns about its top left corner: move that corner so the middle of the left
    // edge lands on `at`.
    let half = galley.size().y * 0.5;
    let corner = at + Vec2::new(angle.sin(), -angle.cos()) * half;
    p.add(TextShape::new(corner, galley, color).with_angle(angle));
}

/// Every sound by number, in as many columns as fit. Returns the sound clicked.
fn drum_list(ui: &mut Ui, view: &DrumViewPrefs, marked: &Marked, width: f32) -> Option<u8> {
    let s = if view.large_text { 1.2 } else { 1.0 };
    let sounds: Vec<&DrumSound> = drums::SOUNDS.iter().filter(|d| !view.general_midi_only || drums::GENERAL_MIDI.contains(&d.midi)).collect();
    let (cw, rh) = (182.0 * s, 20.0 * s);
    let cols = ((width / cw).floor() as usize).max(1);
    let rows = sounds.len().div_ceil(cols);
    let size = Vec2::new((cols as f32 * cw).max(width), rows as f32 * rh + 2.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let p = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
    p.rect_filled(rect, 0.0, LIST_BG);
    let mut hit = None;
    for (i, d) in sounds.iter().enumerate() {
        let min = rect.min + Vec2::new((i / rows) as f32 * cw, (i % rows) as f32 * rh);
        let cell = Rect::from_min_size(min + Vec2::splat(1.0), Vec2::new(cw - 2.0, rh - 1.0));
        let over = resp.hover_pos().is_some_and(|at| cell.contains(at));
        if over {
            hit = Some(d.midi);
        }
        let fill = if marked.now.contains(&d.midi) {
            mix(marked.color, Color32::BLACK, 0.45)
        } else if marked.also.contains(&d.midi) {
            mix(marked.color, Color32::BLACK, 0.75)
        } else if over {
            Color32::from_gray(55)
        } else {
            Color32::BLACK
        };
        p.rect_filled(cell, 2.0, fill);
        let font = FontId::proportional(12.5 * s);
        p.text(Pos2::new(cell.left() + 4.0, cell.center().y), Align2::LEFT_CENTER, format!("{:03}", d.midi), font.clone(), Color32::from_gray(140));
        p.text(Pos2::new(cell.left() + 32.0 * s, cell.center().y), Align2::LEFT_CENTER, d.name, font, Color32::from_gray(240));
    }
    resp.clicked().then_some(hit).flatten()
}
