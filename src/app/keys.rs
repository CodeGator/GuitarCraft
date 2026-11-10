//! Keyboard shortcuts, following the original's layout where it makes sense.

use super::popups::Popup;
use super::{App, InspectorTab, Pending};
use crate::model::*;
use egui::{Key, Modifiers};

impl App {
    pub(super) fn handle_keys(&mut self, ctx: &egui::Context) {
        if self.pending.is_some() || self.dialog_open() {
            return;
        }
        let typing = ctx.egui_wants_keyboard_input();
        let events = ctx.input(|i| i.events.clone());
        let now = ctx.input(|i| i.time);
        for ev in events {
            match ev {
                egui::Event::Copy if !typing => {
                    if ctx.input(|i| i.modifiers.shift) {
                        self.all_track_copy();
                    } else {
                        self.copy();
                        self.last_copy_all = false;
                    }
                    ctx.copy_text("guitarcraft-beats".into());
                }
                egui::Event::Cut if !typing => {
                    if ctx.input(|i| i.modifiers.shift) {
                        self.all_track_cut();
                    } else {
                        self.copy();
                        self.last_copy_all = false;
                        self.delete_at_cursor();
                    }
                    ctx.copy_text("guitarcraft-beats".into());
                }
                egui::Event::Paste(_) if !typing => self.paste_any(),
                egui::Event::Text(t) if !typing => self.symbol_key(&t),
                egui::Event::Key { key, pressed: true, modifiers, .. } => {
                    if modifiers.command && !modifiers.alt {
                        match key {
                            Key::S if modifiers.shift => self.save(true),
                            Key::S => self.save(false),
                            Key::O | Key::B => self.request(Pending::Open),
                            Key::N => self.request(Pending::New),
                            Key::W => self.request(Pending::Close),
                            Key::E => {
                                self.cmd_open = !self.cmd_open;
                                self.cmd_text.clear();
                            }
                            _ => {}
                        }
                    }
                    if typing {
                        continue;
                    }
                    self.key(key, modifiers, now);
                }
                _ => {}
            }
        }
    }

    /// Shortcuts typed as symbols (they depend on the keyboard layout).
    fn symbol_key(&mut self, t: &str) {
        match t {
            ";" => self.toggle_note_flag("Accent", |n| &mut n.accent),
            ":" => self.toggle_note_flag("Heavy accent", |n| &mut n.heavy_accent),
            "!" => self.toggle_note_flag("Staccato", |n| &mut n.staccato),
            "[" => self.bar_edit(|m| m.repeat_open = !m.repeat_open),
            "]" => self.bar_edit(|m| m.repeat_close = if m.repeat_close > 0 { 0 } else { 2 }),
            "<" => self.beat_edit(|b| b.fade_in = !b.fade_in),
            "/" => self.beat_edit(|b| b.tuplet = if b.tuplet == 3 { 0 } else { 3 }),
            "\"" => self.beat_edit(|b| b.tremolo = !b.tremolo),
            "(" | ")" => self.beat_edit(|b| b.technique = if b.technique == 1 { 0 } else { 1 }),
            "$" => self.beat_edit(|b| b.technique = if b.technique == 2 { 0 } else { 2 }),
            _ => {}
        }
    }

    pub(super) fn key(&mut self, key: Key, m: Modifiers, now: f64) {
        let cmd = m.command;
        let digit = matches!(key, Key::Num0 | Key::Num1 | Key::Num2 | Key::Num3 | Key::Num4 | Key::Num5 | Key::Num6 | Key::Num7 | Key::Num8 | Key::Num9);
        if !digit {
            self.last_digit = None;
        }
        // Function keys.
        match key {
            Key::F1 => self.guide_open = true,
            Key::F2 => self.show_palette = !self.show_palette,
            Key::F3 => self.single_track = !self.single_track,
            Key::F4 => self.check_bar_duration(),
            Key::F7 => self.open_stylesheet(),
            Key::F8 => self.show_tracks = !self.show_tracks,
            Key::F9 => self.looping = !self.looping,
            Key::F10 => self.open_automations(self.bar),
            Key::F11 => self.toggle_fullscreen(),
            Key::Escape if self.fullscreen => self.toggle_fullscreen(),
            Key::F5 => {
                if self.show_inspector && self.inspector == InspectorTab::Song {
                    self.show_inspector = false;
                } else {
                    self.show_inspector = true;
                    self.inspector = InspectorTab::Song;
                }
            }
            Key::F6 if !cmd => {
                if self.show_inspector && self.inspector == InspectorTab::Track {
                    self.show_inspector = false;
                } else {
                    self.show_inspector = true;
                    self.inspector = InspectorTab::Track;
                }
            }
            Key::F6 if cmd => self.show_keyboard = !self.show_keyboard,
            _ => {}
        }
        if cmd && m.alt {
            match key {
                Key::ArrowUp => self.move_track(-1),
                Key::ArrowDown => self.move_track(1),
                Key::ArrowLeft => self.goto_section(-1),
                Key::ArrowRight => self.goto_section(1),
                Key::Y => self.toggle_note_flag("Artificial harmonic", |n| &mut n.pinch_harmonic),
                Key::Num8 => self.note_edit(|x| x.spell = (x.spell + 1) % 3),
                Key::G => self.note_edit(|x| x.grace = if x.grace.is_some() { None } else { Some(x.fret.saturating_sub(2)) }),
                _ => {}
            }
            self.clamp_cursor();
            return;
        }
        if m.alt && m.shift {
            match key {
                Key::ArrowUp => self.transpose(1),
                Key::ArrowDown => self.transpose(-1),
                Key::PageUp => self.shift_octave(1),
                Key::PageDown => self.shift_octave(-1),
                _ => {}
            }
            self.clamp_cursor();
            return;
        }
        if cmd {
            match key {
                Key::Z if m.shift => self.do_redo(),
                Key::Z => self.do_undo(),
                Key::Y => self.do_redo(),
                Key::M if m.shift => {
                    self.vfrom = self.voice;
                    self.vto = if self.voice == 0 { 1 } else { 0 };
                    self.vrange = if self.selection().is_some() { super::voices::VoiceRange::Selection } else { super::voices::VoiceRange::Bar };
                    self.voices_open = true;
                }
                Key::C if m.shift => self.all_track_copy(),
                Key::X if m.shift => self.all_track_cut(),
                Key::W if m.shift => self.request(Pending::Close),
                Key::ArrowRight if m.shift => {
                    self.update_anchor(true);
                    self.bar = (self.bar + 1).min(self.song.bar_count() - 1);
                    self.beat = self.song.tracks[self.track].bars[self.bar].voice(self.voice).len() - 1;
                }
                Key::ArrowLeft if m.shift => {
                    self.update_anchor(true);
                    self.bar = self.bar.saturating_sub(1);
                    self.beat = 0;
                }
                Key::Home if m.shift => {
                    self.update_anchor(true);
                    self.bar = 0;
                    self.beat = 0;
                }
                Key::End if m.shift => {
                    self.update_anchor(true);
                    self.bar = self.song.bar_count() - 1;
                    self.beat = self.song.tracks[self.track].bars[self.bar].voice(self.voice).len() - 1;
                }
                Key::Num1 => self.set_voice(0),
                Key::Num2 => self.set_voice(1),
                Key::Num3 => self.set_voice(2),
                Key::Num4 => self.set_voice(3),
                Key::Num7 => self.note_edit(|x| x.spell = 2),
                Key::Num8 => self.note_edit(|x| x.spell = 0),
                Key::Num9 => self.note_edit(|x| x.spell = 1),
                Key::Period => self.beat_edit(|b| {
                    b.double_dot = !b.double_dot;
                    b.dotted = false;
                }),
                Key::E => {}
                Key::A => {
                    self.sel_anchor = Some((0, 0));
                    self.bar = self.song.bar_count() - 1;
                    self.beat = self.song.tracks[self.track].bars[self.bar].voice(self.voice).len() - 1;
                }
                Key::G => self.goto_open = true,
                Key::K => self.open_popup(Popup::KeySig),
                Key::T => self.open_popup(Popup::TimeSig),
                Key::Slash => self.bar_edit(|mb| mb.triplet_feel = !mb.triplet_feel),
                Key::D => self.beat_edit(|b| b.stroke = if b.stroke == Stroke::Down { Stroke::None } else { Stroke::Down }),
                Key::U => self.beat_edit(|b| b.stroke = if b.stroke == Stroke::Up { Stroke::None } else { Stroke::Up }),
                Key::Insert if m.shift => self.add_track(false),
                Key::Delete if m.shift => self.remove_track(),
                Key::Insert => self.insert_bar(self.bar),
                Key::Delete => self.delete_bar(self.bar),
                Key::Enter => self.bar_edit(|mb| mb.line_break = !mb.line_break),
                Key::ArrowRight => self.goto_bar(self.bar + 1),
                Key::ArrowLeft => self.goto_bar(self.bar.saturating_sub(1)),
                Key::Home => {
                    self.stop();
                    self.goto_bar(0);
                }
                Key::End => {
                    self.stop();
                    self.goto_bar(self.song.bar_count() - 1);
                }
                Key::ArrowUp => self.track = self.track.saturating_sub(1),
                Key::ArrowDown => self.track = (self.track + 1).min(self.song.tracks.len() - 1),
                Key::Space => self.play_from_start(),
                Key::Plus | Key::Equals => self.zoom_step(1),
                Key::Minus => self.zoom_step(-1),
                _ => {}
            }
            self.clamp_cursor();
            return;
        }
        if m.alt && !cmd {
            match key {
                Key::Num1 => self.move_to_voice(0),
                Key::Num2 => self.move_to_voice(1),
                Key::Num3 => self.move_to_voice(2),
                Key::Num4 => self.move_to_voice(3),
                Key::ArrowLeft => self.step(-1),
                Key::ArrowRight => self.step(1),
                Key::ArrowUp => self.move_note_string(-1),
                Key::ArrowDown => self.move_note_string(1),
                _ => {}
            }
            self.clamp_cursor();
            return;
        }
        if m.shift && !digit {
            match key {
                Key::Tab => self.tab_key(-1),
                Key::Home => {
                    self.update_anchor(true);
                    self.beat = 0;
                }
                Key::End => {
                    self.update_anchor(true);
                    self.beat = self.song.tracks[self.track].bars[self.bar].voice(self.voice).len() - 1;
                }
                Key::L => self.tie_beat(),
                Key::P => {
                    let on = !self.cur_beat().notes.iter().all(|n| n.palm_mute);
                    self.edit(|s, t, b, j, _| {
                        for n in &mut s.tracks[t].bars[b].beats[j].notes {
                            n.palm_mute = on;
                        }
                    });
                }
                Key::ArrowRight => self.move_right(true),
                Key::ArrowLeft => self.move_left(true),
                Key::Delete => self.delete_beats(),
                Key::Insert => self.edit_section(),
                Key::D => self.beat_edit(|b| b.pick = if b.pick == 1 { 0 } else { 1 }),
                Key::U => self.beat_edit(|b| b.pick = if b.pick == 2 { 0 } else { 2 }),
                Key::H => self.toggle_note_flag("Hammer-on / pull-off", |n| &mut n.hammer),
                Key::Plus | Key::Equals => self.set_duration(self.cur_beat().dur.shorter()),
                Key::Minus => self.set_duration(self.cur_beat().dur.longer()),
                Key::Period => self.beat_edit(|b| {
                    b.dotted = !b.dotted;
                    b.double_dot = false;
                }),
                _ => {}
            }
            self.clamp_cursor();
            return;
        }
        if m.shift {
            // Shift + digit types a symbol, handled as text.
            return;
        }
        match key {
            Key::ArrowRight => self.move_right(false),
            Key::ArrowLeft => self.move_left(false),
            Key::ArrowUp if self.std_cursor.is_some() => self.std_step(1),
            Key::ArrowDown if self.std_cursor.is_some() => self.std_step(-1),
            Key::ArrowUp => self.string = self.string.saturating_sub(1),
            Key::ArrowDown => self.string = (self.string + 1).min(self.tr().strings() - 1),
            Key::Home => self.beat = 0,
            Key::End => self.beat = self.song.tracks[self.track].bars[self.bar].voice(self.voice).len() - 1,
            Key::PageDown => self.goto_bar(self.bar + 4),
            Key::PageUp => self.goto_bar(self.bar.saturating_sub(4)),
            Key::Insert => self.insert_beat(),
            Key::Delete | Key::Backspace if self.std_cursor.is_some() => self.std_delete(),
            Key::Delete | Key::Backspace => self.delete_at_cursor(),
            Key::Space => self.toggle_play(),
            Key::Escape => {
                if self.playing {
                    self.stop();
                }
                self.sel_anchor = None;
            }
            Key::Plus | Key::Equals => self.set_duration(self.cur_beat().dur.shorter()),
            Key::Minus => self.set_duration(self.cur_beat().dur.longer()),
            Key::Tab => self.tab_key(1),
            Key::C => self.copy_last_beat(),
            Key::D => self.dir_open = true,
            Key::K => self.open_popup(Popup::KeySig),
            Key::Period => self.beat_edit(|b| {
                b.dotted = !b.dotted;
                b.double_dot = false;
            }),
            Key::R => self.edit(|s, t, b, j, _| s.tracks[t].bars[b].beats[j].notes.clear()),
            Key::F => self.beat_edit(|b| b.fermata = !b.fermata),
            Key::T => self.open_popup(Popup::Text),
            Key::A => self.open_chord_tool(),
            Key::H => self.toggle_note_flag("Hammer-on / pull-off", |n| &mut n.hammer),
            Key::S => self.toggle_note_flag("Slide", |n| &mut n.slide),
            Key::B => self.open_popup(Popup::Bend),
            Key::V => self.toggle_note_flag("Vibrato", |n| &mut n.vibrato),
            Key::P => self.toggle_note_flag("Palm mute", |n| &mut n.palm_mute),
            Key::I => self.toggle_note_flag("Let ring", |n| &mut n.let_ring),
            Key::L => self.toggle_note_flag("Tie", |n| &mut n.tie),
            Key::X => self.toggle_note_flag("Dead note", |n| &mut n.dead),
            Key::O => self.toggle_note_flag("Ghost note", |n| &mut n.ghost),
            Key::G => self.note_edit(|x| x.grace = if x.grace.is_some() { None } else { Some(x.fret.saturating_sub(2)) }),
            Key::Y => self.toggle_note_flag("Natural harmonic", |n| &mut n.harmonic),
            Key::N => self.open_popup(Popup::Trill),
            Key::Num0 if self.std_cursor.is_some() => self.std_digit(0),
            Key::Num0 => self.type_digit(0, now),
            Key::Num1 if self.std_cursor.is_some() => self.std_digit(1),
            Key::Num1 => self.type_digit(1, now),
            Key::Num2 if self.std_cursor.is_some() => self.std_digit(2),
            Key::Num2 => self.type_digit(2, now),
            Key::Num3 if self.std_cursor.is_some() => self.std_digit(3),
            Key::Num3 => self.type_digit(3, now),
            Key::Num4 if self.std_cursor.is_some() => self.std_digit(4),
            Key::Num4 => self.type_digit(4, now),
            Key::Num5 if self.std_cursor.is_some() => self.std_digit(5),
            Key::Num5 => self.type_digit(5, now),
            Key::Num6 if self.std_cursor.is_some() => self.std_digit(6),
            Key::Num6 => self.type_digit(6, now),
            Key::Num7 if self.std_cursor.is_some() => self.std_digit(7),
            Key::Num7 => self.type_digit(7, now),
            Key::Num8 if self.std_cursor.is_some() => self.std_digit(8),
            Key::Num8 => self.type_digit(8, now),
            Key::Num9 if self.std_cursor.is_some() => self.std_digit(9),
            Key::Num9 => self.type_digit(9, now),
            _ => return,
        }
        self.clamp_cursor();
    }

}

pub const SHORTCUTS: [(&str, &str); 47] = [
    ("Arrow keys", "Move between beats and strings"),
    ("Tab / Shift + Tab", "Switch between notation and tab, then the next track"),
    ("Ctrl + 1 / 2 / 3 / 4", "Edit voice 1, 2, 3 or 4"),
    ("Alt + 1 / 2 / 3 / 4", "Move the note (or the selected notes) to voice 1, 2, 3 or 4"),
    ("Ctrl + Shift + M", "Move, copy or swap voices"),
    ("Home / End", "First / last beat of the bar"),
    ("Ctrl + \u{2190} \u{2192}", "Previous / next bar"),
    ("Ctrl + \u{2191} \u{2193}", "Previous / next track"),
    ("Shift + \u{2190} \u{2192}", "Select beats"),
    ("Ctrl + Shift + \u{2190} \u{2192}", "Select bars"),
    ("Alt + \u{2190} \u{2192}", "Step backward / forward and hear the beat"),
    ("Alt + \u{2191} \u{2193}", "Move the note to the next string up / down"),
    ("0 \u{2013} 9", "Tab: type a fret. Notation: 1\u{2013}9 picks the string, 0 the best one"),
    ("+ / \u{2212}", "Shorter / longer note"),
    (". / Ctrl + .", "Dotted / double-dotted note"),
    ("/", "Triplet"),
    ("R", "Rest"),
    ("L / Shift + L", "Tie note / tie beat"),
    ("C", "Copy the last beat here"),
    ("F", "Fermata"),
    ("T / A", "Text / chord"),
    ("G", "Grace note"),
    ("O", "Ghost note"),
    ("; / : / !", "Accent / heavy accent / staccato"),
    ("H  S  B  V  N", "Hammer-on, slide, bend, vibrato, trill"),
    ("I / P / Shift + P", "Let ring / palm mute on note / on beat"),
    ("X", "Dead note"),
    ("Y / Ctrl + Alt + Y", "Natural / artificial harmonic"),
    ("Ctrl + 7 / 8 / 9", "Spell with flats / automatically / with sharps"),
    ("Shift + D / U", "Pickstroke down / up"),
    ("Ctrl + D / U", "Brush down / up"),
    ("\" / ( ) / $", "Tremolo picking / tapping / slap"),
    ("<", "Fade in"),
    ("[ / ]", "Repeat open / close"),
    ("D", "Directions (Segno, Coda, D.C., D.S.)"),
    ("Ctrl + K / T / /", "Key / time signature / triplet feel"),
    ("Ctrl + Ins / Ctrl + Del", "Insert / delete a bar"),
    ("Ins / Del / Shift + Del", "Insert a beat / delete a note / delete beats"),
    ("Ctrl + G / Ctrl + E", "Go to a bar / command palette"),
    ("Alt + Shift + \u{2191} \u{2193}", "Notes up / down a semitone"),
    ("Space / Ctrl + Space / Esc", "Play-pause / play from the start / stop"),
    ("F9 / F10", "Loop / tempo automations"),
    ("F2 / F8 / F5 / F6", "Palette / tracks / song / track settings"),
    ("F3 / F4", "Multitrack / check bar duration"),
    ("Ctrl + + / \u{2212}, F11", "Zoom in / out, full screen"),
    ("Ctrl + C / X / V (+ Shift)", "Copy / cut / paste (all tracks)"),
    ("Ctrl + Z / Y, N / O / S / W", "Undo / redo, new / open / save / close"),
];
