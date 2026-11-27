//! Note > Audio Note Settings, as in Guitar Pro 8: how long a note sounds, how much early or
//! late it starts and how loud it is, apart from what is written. The window stays open while
//! the cursor moves, shows the settings of the note (or selection) it is on, and changes them at
//! once. While it is open the score marks the notes that have settings of their own. The window
//! looks as Guitar Pro 8's does: a heading, a number box with arrows for each setting, and Reset.

use super::App;
use crate::model::{Note, NotePlayback};
use egui::{Align2, Color32, FontFamily, FontId, Id, Key, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};
use std::ops::RangeInclusive;

/// The open window.
#[derive(Debug, Default)]
pub struct AudioNotes {
    /// The notes changed last and the undo steps there were afterwards: further changes to the
    /// same notes join that one step.
    last: Option<(Vec<Target>, usize)>,
    /// The notes there were to change when typing in a box began: what is typed goes to them,
    /// even if it is finished by clicking another note.
    typing: Option<Vec<Target>>,
}

/// A note: its track, bar, voice, beat and string.
type Target = (usize, usize, usize, usize, u8);

/// The settings' ranges. The duration's is Guitar Pro's; the others' aren't published.
const PERCENT: RangeInclusive<i32> = 25..=1200;
const TICKS: RangeInclusive<i32> = -480..=480;
const DECIBELS: RangeInclusive<i32> = -30..=30;

/// The window's colours and sizes, from a picture of the original (its text a little larger).
const TITLE_BG: Color32 = Color32::from_rgb(243, 243, 243);
const BODY_BG: Color32 = Color32::from_rgb(43, 45, 54);
const FIELD_BG: Color32 = Color32::from_rgb(14, 15, 19);
const FIELD_EDGE: Color32 = Color32::from_rgb(66, 70, 84);
const BUTTON_BG: Color32 = Color32::from_rgb(76, 86, 114);
const FOCUS: Color32 = Color32::from_rgb(90, 150, 240);
const WIDTH: f32 = 238.0;
const FIELD: Vec2 = Vec2::new(96.0, 26.0);
const ARROWS: f32 = 20.0;
const TEXT: f32 = 15.0;

/// A press held on an arrow starts repeating after this long, then steps this often (seconds).
const HOLD: f64 = 0.4;
const REPEAT: f64 = 0.06;

/// Where each box and its arrows are found (also by tests).
fn field_id(label: &str) -> Id {
    Id::new(("audio-note-setting", label))
}

/// A number as typed, with or without its unit: "80", "80%", "+3 dB", "-6 ticks".
fn parse_number(text: &str) -> Option<i32> {
    let text = text.trim();
    let text = text.strip_prefix('+').unwrap_or(text);
    let end = text.char_indices().find(|&(i, c)| !(c.is_ascii_digit() || (c == '-' && i == 0))).map_or(text.len(), |(i, _)| i);
    text[..end].parse().ok()
}

/// A box with a number and a pair of arrows. The number can be typed (Return or clicking
/// elsewhere takes it, Escape leaves it), stepped with the arrows (held, they repeat), with the
/// Up and Down keys while typing, or with the mouse wheel. Returns the new value, and whether
/// it was typed.
fn spin_box(ui: &mut Ui, label: &str, value: i32, range: RangeInclusive<i32>, shown: String) -> (Option<i32>, bool, bool) {
    let id = field_id(label);
    let text_id = id.with("text");
    let (rect, _) = ui.allocate_exact_size(FIELD, Sense::hover());
    let enabled = ui.is_enabled();
    let focused = ui.memory(|m| m.has_focus(text_id));
    let ink = if enabled { Color32::WHITE } else { Color32::from_gray(120) };
    let edge = if focused { FOCUS } else { FIELD_EDGE };
    ui.painter().rect(rect, 3.0, FIELD_BG, Stroke::new(1.0, edge), StrokeKind::Inside);

    let mut stepped = 0;
    let column = Rect::from_min_max(Pos2::new(rect.right() - ARROWS, rect.top()), rect.max);
    let halves = [(1, "up", column.top(), column.center().y), (-1, "down", column.center().y, column.bottom())];
    for (step, name, top, bottom) in halves {
        let half = Rect::from_min_max(Pos2::new(column.left(), top), Pos2::new(column.right(), bottom));
        let arrow = ui.interact(half, id.with(name), Sense::click_and_drag());
        // Held down, the arrow repeats; the click that ends such a press adds nothing.
        let repeats = id.with((name, "repeats"));
        if arrow.is_pointer_button_down_on() {
            let held = ui.input(|i| i.pointer.press_start_time().map_or(0.0, |at| i.time - at));
            let due = if held < HOLD { 0 } else { ((held - HOLD) / REPEAT) as i32 + 1 };
            let done: i32 = ui.data(|d| d.get_temp(repeats)).unwrap_or(0);
            if due > done {
                stepped += step * (due - done).min(3);
                ui.data_mut(|d| d.insert_temp(repeats, due));
            }
            ui.ctx().request_repaint();
        }
        if arrow.clicked() && ui.data(|d| d.get_temp::<i32>(repeats)).unwrap_or(0) == 0 {
            stepped += step;
        }
        if !arrow.is_pointer_button_down_on() {
            ui.data_mut(|d| d.remove::<i32>(repeats));
        }
        if arrow.hovered() && enabled {
            ui.painter().rect_filled(half.shrink(1.0), 2.0, Color32::from_rgb(40, 43, 54));
        }
        // A small triangle pointing the way it steps.
        let c = half.center();
        let tip = c + Vec2::new(0.0, -2.5 * step as f32);
        let base = c.y + 2.0 * step as f32;
        ui.painter().add(egui::Shape::convex_polygon(
            vec![tip, Pos2::new(c.x - 4.0, base), Pos2::new(c.x + 4.0, base)],
            if enabled { Color32::from_gray(170) } else { Color32::from_gray(90) },
            Stroke::NONE,
        ));
    }
    if ui.rect_contains_pointer(rect) && enabled {
        stepped += ui.input(|i| {
            i.events
                .iter()
                .map(|e| match e {
                    egui::Event::MouseWheel { delta, .. } if delta.y > 0.0 => 1,
                    egui::Event::MouseWheel { delta, .. } if delta.y < 0.0 => -1,
                    _ => 0,
                })
                .sum::<i32>()
        });
    }
    if focused {
        stepped += ui.input_mut(|i| {
            i32::from(i.consume_key(egui::Modifiers::NONE, Key::ArrowUp)) - i32::from(i.consume_key(egui::Modifiers::NONE, Key::ArrowDown))
        });
    }

    // The number: what is being typed while the box has the keyboard.
    let mut text: String = ui.data(|d| d.get_temp(text_id)).filter(|_| focused).unwrap_or(shown);
    let inside = Rect::from_min_max(rect.min + Vec2::new(7.0, 3.0), Pos2::new(column.left() - 2.0, rect.bottom() - 3.0));
    let edit = ui.put(
        inside,
        egui::TextEdit::singleline(&mut text).id(text_id).frame(egui::Frame::NONE).margin(Vec2::ZERO).font(FontId::proportional(TEXT)).text_color(ink),
    );
    let began = edit.gained_focus();
    if began && let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), text_id) {
        // All of it chosen, so typing replaces it.
        let all = egui::text::CCursorRange::two(egui::text::CCursor::new(0), egui::text::CCursor::new(text.chars().count()));
        state.cursor.set_char_range(Some(all));
        state.store(ui.ctx(), text_id);
    }
    let finished = edit.lost_focus() && !ui.input(|i| i.key_pressed(Key::Escape));
    let typed = finished.then(|| ui.data(|d| d.get_temp::<String>(text_id))).flatten();
    ui.data_mut(|d| d.insert_temp(text_id, text.clone()));
    if finished {
        // What is in the box now, which may have been typed in this very frame.
        let number = parse_number(&text).or_else(|| typed.as_deref().and_then(parse_number));
        ui.data_mut(|d| d.remove::<String>(text_id));
        let new = number.map(|n| n.clamp(*range.start(), *range.end())).filter(|&n| n != value);
        return (new, true, began);
    }
    // A step stays within the range; a value from a file that is outside it moves towards it.
    let new = (stepped != 0).then(|| (value + stepped).clamp((*range.start()).min(value), (*range.end()).max(value)));
    if new.is_some() && focused {
        ui.data_mut(|d| d.remove::<String>(text_id));
        ui.memory_mut(|m| m.surrender_focus(text_id));
    }
    (new.filter(|&n| n != value), false, began)
}

/// One of the three settings.
#[derive(Clone, Copy)]
enum Setting {
    Duration,
    Offset,
    Velocity,
}

impl Setting {
    const ALL: [Setting; 3] = [Setting::Duration, Setting::Offset, Setting::Velocity];

    fn label(self) -> &'static str {
        match self {
            Setting::Duration => "Duration:",
            Setting::Offset => "Offset:",
            Setting::Velocity => "Relative Velocity:",
        }
    }

    fn range(self) -> RangeInclusive<i32> {
        match self {
            Setting::Duration => PERCENT,
            Setting::Offset => TICKS,
            Setting::Velocity => DECIBELS,
        }
    }

    fn get(self, p: NotePlayback) -> i32 {
        match self {
            Setting::Duration => p.percent as i32,
            Setting::Offset => p.offset as i32,
            Setting::Velocity => p.db as i32,
        }
    }

    fn set(self, p: &mut NotePlayback, value: i32) {
        match self {
            Setting::Duration => p.percent = value.clamp(1, u16::MAX as i32) as u16,
            Setting::Offset => p.offset = value.clamp(i16::MIN as i32, i16::MAX as i32) as i16,
            Setting::Velocity => p.db = value.clamp(i8::MIN as i32, i8::MAX as i32) as i8,
        }
    }

    fn show(self, value: i32) -> String {
        match self {
            Setting::Duration => format!("{value}%"),
            Setting::Offset => format!("{value} ticks"),
            Setting::Velocity if value > 0 => format!("+{value} dB"),
            Setting::Velocity => format!("{value} dB"),
        }
    }
}

impl App {
    pub(super) fn open_audio_notes(&mut self) {
        if self.audio_notes.is_none() {
            self.audio_notes = Some(AudioNotes::default());
        }
    }

    /// The notes the settings apply to: those of the selection, or else the note at the cursor
    /// (every note of the beat when the cursor isn't on one). An arpeggio's notes have none.
    fn audio_targets(&self) -> Vec<Target> {
        let (track, voice) = (self.track, self.voice);
        let bars = &self.song.tracks[track].bars;
        let (from, to) = self.selection().unwrap_or(((self.bar, self.beat), (self.bar, self.beat)));
        let at_cursor = self.selection().is_none()
            && self.cur_beat().note_on(self.string as u8).is_some()
            && (self.std_cursor.is_none() || self.note_at_std().is_some());
        let mut out = vec![];
        for (bi, bar) in bars.iter().enumerate().take(to.0 + 1).skip(from.0) {
            for (j, beat) in bar.voice(voice).iter().enumerate() {
                if (bi == from.0 && j < from.1) || (bi == to.0 && j > to.1) || beat.fx.arpeggio != 0 {
                    continue;
                }
                let notes = beat.notes.iter().filter(|n| !at_cursor || n.string as usize == self.string);
                out.extend(notes.map(|n| (track, bi, voice, j, n.string)));
            }
        }
        out
    }

    fn playback_of(&self, (track, bar, voice, beat, string): Target) -> NotePlayback {
        let note = self.song.tracks[track].bars.get(bar).and_then(|b| b.voice(voice).get(beat)).and_then(|b| b.note_on(string));
        note.map(|n: &Note| n.playback).unwrap_or_default()
    }

    /// Changes settings of the notes at the cursor or in the selection.
    pub(super) fn set_note_playback(&mut self, change: impl Fn(&mut NotePlayback)) {
        self.set_playback_of(self.audio_targets(), change);
    }

    /// Changes settings of `targets` (notes of the track and voice being edited), as one undo
    /// step with the changes just before to the same notes.
    fn set_playback_of(&mut self, targets: Vec<Target>, change: impl Fn(&mut NotePlayback)) {
        let here = |t: &Target| t.0 == self.track && t.2 == self.voice;
        if targets.is_empty() || !targets.iter().all(here) {
            return;
        }
        let steps = self.undo.len();
        let joins = self.audio_notes.as_ref().and_then(|w| w.last.as_ref()).is_some_and(|(t, n)| *t == targets && *n == steps);
        let apply = targets.clone();
        self.edit(move |song, _, _, _, _| {
            // During an edit the voice being edited stands where voice 1 is.
            for (track, bar, _, beat, string) in apply {
                let beats = &mut song.tracks[track].bars[bar].beats;
                if let Some(n) = beats.get_mut(beat).and_then(|b| b.note_on_mut(string)) {
                    change(&mut n.playback);
                }
            }
        });
        if joins {
            self.undo.pop();
        }
        let steps = self.undo.len();
        if let Some(w) = self.audio_notes.as_mut() {
            w.last = Some((targets, steps));
        }
    }

    pub(super) fn audio_notes_window(&mut self, ctx: &egui::Context) {
        if self.audio_notes.is_none() || self.show_home || self.chart.is_some() {
            return;
        }
        let targets = self.audio_targets();
        // With several notes, the first one's settings are shown.
        let shown = targets.first().map(|&t| self.playback_of(t)).unwrap_or_default();
        let mut close = false;
        let mut reset = false;
        let mut changed: Option<(Setting, i32, bool)> = None;
        let mut began = false;
        egui::Window::new("Audio note settings")
            .id(Id::new("audio-note-settings"))
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .frame(egui::Frame::window(&ctx.global_style()).fill(BODY_BG).inner_margin(0.0).corner_radius(6.0))
            // To the right, clear of the start of the lines of music.
            .default_pos(ctx.content_rect().right_center() - Vec2::new(24.0, 0.0))
            .pivot(Align2::RIGHT_CENTER)
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                ui.set_width(WIDTH);
                // The title bar, with its close button; the window is dragged by it.
                let (bar, _) = ui.allocate_exact_size(Vec2::new(WIDTH, 30.0), Sense::hover());
                let p = ui.painter();
                p.rect_filled(bar, egui::CornerRadius { nw: 6, ne: 6, sw: 0, se: 0 }, TITLE_BG);
                p.text(bar.left_center() + Vec2::new(10.0, 0.0), Align2::LEFT_CENTER, "Audio note settings", FontId::proportional(13.5), Color32::from_gray(40));
                let cross = Rect::from_min_max(Pos2::new(bar.right() - 36.0, bar.top()), bar.max);
                let x = ui.interact(cross, field_id("close"), Sense::click()).on_hover_text("Close");
                if x.hovered() {
                    ui.painter().rect_filled(cross, egui::CornerRadius { nw: 0, ne: 6, sw: 0, se: 0 }, Color32::from_rgb(200, 60, 60));
                }
                let ink = if x.hovered() { Color32::WHITE } else { Color32::from_gray(60) };
                for dx in [-4.0, 4.0] {
                    ui.painter().line_segment([cross.center() + Vec2::new(-dx, -4.0), cross.center() + Vec2::new(dx, 4.0)], Stroke::new(1.3, ink));
                }
                close |= x.clicked();

                ui.add_space(12.0);
                let (heading, _) = ui.allocate_exact_size(Vec2::new(WIDTH, 20.0), Sense::hover());
                let bold = FontId::new(15.0, FontFamily::Name("bold".into()));
                ui.painter().text(heading.left_center() + Vec2::new(12.0, 0.0), Align2::LEFT_CENTER, "AUDIO NOTE SETTINGS", bold, Color32::WHITE);
                ui.add_space(14.0);
                ui.add_enabled_ui(!targets.is_empty(), |ui| {
                    for setting in Setting::ALL {
                        ui.horizontal(|ui| {
                            // The names end where the boxes begin.
                            let (name, _) = ui.allocate_exact_size(Vec2::new(WIDTH - FIELD.x - 18.0, FIELD.y), Sense::hover());
                            let ink = if ui.is_enabled() { Color32::from_gray(235) } else { Color32::from_gray(130) };
                            ui.painter().text(name.right_center(), Align2::RIGHT_CENTER, setting.label(), FontId::proportional(TEXT), ink);
                            ui.add_space(8.0);
                            let value = setting.get(shown);
                            let (new, typed, started) = spin_box(ui, setting.label(), value, setting.range(), setting.show(value));
                            if let Some(new) = new {
                                changed = Some((setting, new, typed));
                            }
                            began |= started;
                        });
                        ui.add_space(7.0);
                    }
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.add_space(12.0);
                        let button = egui::Button::new(egui::RichText::new("Reset").size(TEXT).color(Color32::WHITE)).fill(BUTTON_BG).corner_radius(5.0);
                        reset = ui.add_sized([66.0, 28.0], button).clicked();
                    });
                });
                ui.add_space(14.0);
            });
        // Only the setting that changed is applied, so the others keep each note's own value.
        // Something typed goes to the notes there were when the typing began.
        if let Some((setting, value, typed)) = changed {
            let typing = self.audio_notes.as_ref().and_then(|w| w.typing.clone()).filter(|_| typed);
            self.set_playback_of(typing.unwrap_or_else(|| targets.clone()), move |p| setting.set(p, value));
        } else if reset {
            self.set_note_playback(|p| *p = NotePlayback::default());
        }
        let in_box = ctx.memory(|m| m.focused()).is_some_and(|f| Setting::ALL.iter().any(|s| field_id(s.label()).with("text") == f));
        if let Some(w) = self.audio_notes.as_mut() {
            if began {
                w.typing = Some(targets);
            } else if !in_box {
                w.typing = None;
            }
        }
        if close {
            self.audio_notes = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_are_read_with_or_without_their_units() {
        for (text, n) in [("80", Some(80)), (" 80% ", Some(80)), ("+3 dB", Some(3)), ("-6 dB", Some(-6)), ("5 ticks", Some(5)), ("", None), ("dB", None), ("-", None)] {
            assert_eq!(parse_number(text), n, "{text:?}");
        }
        assert_eq!((Setting::Velocity.show(3), Setting::Velocity.show(0), Setting::Offset.show(-5)), ("+3 dB".into(), "0 dB".into(), "-5 ticks".to_string()));
    }
}
