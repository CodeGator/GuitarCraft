//! Moving, copying and swapping notes between voices.

use super::App;
use crate::model::*;
use egui::{Align2, RichText, Vec2};

/// What the Move/Swap Voices assistant does.
#[derive(Clone, Copy, PartialEq)]
pub enum VoiceAction {
    Move,
    Copy,
    Swap,
}

/// Which bars it works on.
#[derive(Clone, Copy, PartialEq)]
pub enum VoiceRange {
    Selection,
    Bar,
    Track,
}

impl App {
    /// Alt+1..4: moves the note under the cursor, or every note of the selected
    /// beats, into another voice at the same place in the bar.
    pub(super) fn move_to_voice(&mut self, target: usize) {
        if target == self.voice {
            self.status = format!("The notes are already in voice {}.", target + 1);
            return;
        }
        // (bar, beat, strings to move) in the current voice.
        let mut items: Vec<(usize, usize, Vec<u8>)> = vec![];
        if let Some((a, b)) = self.selection() {
            for bi in a.0..=b.0 {
                let n = self.song.tracks[self.track].bars[bi].voice(self.voice).len();
                let from = if bi == a.0 { a.1 } else { 0 };
                let to = if bi == b.0 { b.1 } else { n.saturating_sub(1) };
                for j in from..=to.min(n.saturating_sub(1)) {
                    let strings: Vec<u8> = self.song.tracks[self.track].bars[bi].voice(self.voice)[j].notes.iter().map(|x| x.string).collect();
                    if !strings.is_empty() {
                        items.push((bi, j, strings));
                    }
                }
            }
        } else {
            let beat = self.cur_beat();
            let under = if self.std_cursor.is_some() { self.note_at_std() } else { beat.note_on(self.string as u8).cloned() };
            let strings: Vec<u8> = match under {
                Some(n) => vec![n.string],
                None => beat.notes.iter().map(|x| x.string).collect(),
            };
            if strings.is_empty() {
                self.status = "There is no note here to move.".into();
                return;
            }
            items.push((self.bar, self.beat, strings));
        }
        let (moved, blocked) = self.transfer(self.voice, target, &items, false);
        self.report(moved, blocked, target, "Moved");
        if self.selection().is_none() && moved > 0 && self.cur_beat().is_rest() {
            // Follow the note into its new voice.
            let t = self.song.tracks[self.track].bars[self.bar].beat_start(self.voice, self.beat);
            self.voice = target;
            self.beat = self.beat_at_tick(self.bar, t);
            self.clamp_cursor();
        }
    }

    fn report(&mut self, moved: usize, blocked: usize, target: usize, verb: &str) {
        self.status = match (moved, blocked) {
            (0, 0) => "Nothing to do.".into(),
            (m, 0) => format!("{verb} {m} note(s) to voice {}.", target + 1),
            (0, b) => format!("Voice {} already has notes there ({b} note(s) not moved).", target + 1),
            (m, b) => format!("{verb} {m} note(s) to voice {}; {b} didn't fit because voice {} already plays there.", target + 1, target + 1),
        };
    }

    /// Index of the beat of the current voice that starts at or covers a tick.
    fn beat_at_tick(&self, bar: usize, t: u32) -> usize {
        let beats = self.song.tracks[self.track].bars[bar].voice(self.voice);
        let mut acc = 0;
        for (j, b) in beats.iter().enumerate() {
            if acc + b.ticks() > t {
                return j;
            }
            acc += b.ticks();
        }
        beats.len().saturating_sub(1)
    }

    /// Moves (or copies) notes of the given beats from voice `from` to voice `to`.
    /// Returns how many notes were placed and how many didn't fit.
    fn transfer(&mut self, from: usize, to: usize, items: &[(usize, usize, Vec<u8>)], copy: bool) -> (usize, usize) {
        let mut moved = 0;
        let mut blocked = 0;
        let items = items.to_vec();
        // Work on the real voices, without the editing swap.
        let keep = self.voice;
        self.voice = 0;
        let ti = self.track;
        let mut placed: Vec<(usize, usize, Vec<u8>)> = vec![];
        self.edit(|s, _, _, _, _| {
            for (bi, j, strings) in &items {
                let need = s.master[*bi].ticks();
                let bar = &mut s.tracks[ti].bars[*bi];
                let Some(src) = bar.voice(from).get(*j).cloned() else { continue };
                let t = bar.beat_start(from, *j);
                let notes: Vec<Note> = src.notes.iter().filter(|n| strings.contains(&n.string)).cloned().collect();
                if notes.is_empty() {
                    continue;
                }
                if bar.put_notes(to, t, &src, &notes, need) {
                    moved += notes.len();
                    placed.push((*bi, *j, notes.iter().map(|n| n.string).collect()));
                } else {
                    blocked += notes.len();
                }
            }
            if !copy {
                for (bi, j, strings) in &placed {
                    if let Some(b) = s.tracks[ti].bars[*bi].voice_mut(from).get_mut(*j) {
                        b.notes.retain(|n| !strings.contains(&n.string));
                    }
                }
            }
        });
        self.voice = keep;
        if moved == 0 {
            // Nothing changed: drop the undo step.
            self.undo.pop();
        }
        (moved, blocked)
    }

    /// The Move/Copy/Swap Voices assistant.
    pub(super) fn voices_window(&mut self, ctx: &egui::Context) {
        if !self.voices_open {
            return;
        }
        let mut open = true;
        let mut apply = false;
        let has_sel = self.selection().is_some();
        if !has_sel && self.vrange == VoiceRange::Selection {
            self.vrange = VoiceRange::Bar;
        }
        egui::Window::new("Move/Swap Voices").collapsible(false).resizable(false).anchor(Align2::CENTER_TOP, Vec2::new(0.0, 140.0)).open(&mut open).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.radio_value(&mut self.vaction, VoiceAction::Move, "Move");
                ui.radio_value(&mut self.vaction, VoiceAction::Copy, "Copy");
                ui.radio_value(&mut self.vaction, VoiceAction::Swap, "Swap");
            });
            ui.horizontal(|ui| {
                ui.label(if self.vaction == VoiceAction::Swap { "Swap voice" } else { "From voice" });
                for v in 0..VOICES {
                    ui.selectable_value(&mut self.vfrom, v, format!("{}", v + 1));
                }
            });
            ui.horizontal(|ui| {
                ui.label(if self.vaction == VoiceAction::Swap { "with voice" } else { "To voice    " });
                for v in 0..VOICES {
                    ui.selectable_value(&mut self.vto, v, format!("{}", v + 1));
                }
            });
            ui.horizontal(|ui| {
                ui.label("In");
                ui.add_enabled_ui(has_sel, |ui| {
                    ui.radio_value(&mut self.vrange, VoiceRange::Selection, "the selection");
                });
                ui.radio_value(&mut self.vrange, VoiceRange::Bar, "this bar");
                ui.radio_value(&mut self.vrange, VoiceRange::Track, "the whole track");
            });
            ui.label(
                RichText::new(match self.vaction {
                    VoiceAction::Swap => "Swapping exchanges the two voices of each bar in the range.",
                    _ => "Notes go to the same place in the other voice; a note can't go where that voice already plays a note.",
                })
                .small()
                .weak(),
            );
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.add_enabled(self.vfrom != self.vto, egui::Button::new(RichText::new("  OK  ").strong())).clicked() {
                    apply = true;
                }
                if ui.button("Cancel").clicked() {
                    self.voices_open = false;
                }
            });
        });
        if apply {
            self.run_voice_assistant();
            open = false;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            open = false;
        }
        self.voices_open = open && self.voices_open;
    }

    fn run_voice_assistant(&mut self) {
        let (from, to) = (self.vfrom, self.vto);
        // Beats in range, as (bar, first beat, last beat) of the source voice.
        let ranges: Vec<(usize, usize, usize)> = match self.vrange {
            VoiceRange::Selection => match self.selection() {
                Some((a, b)) => (a.0..=b.0)
                    .map(|bi| {
                        let n = self.song.tracks[self.track].bars[bi].voice(from).len();
                        let s = if bi == a.0 && from == self.voice { a.1 } else { 0 };
                        let e = if bi == b.0 && from == self.voice { b.1 } else { n.saturating_sub(1) };
                        (bi, s, e)
                    })
                    .collect(),
                None => vec![],
            },
            VoiceRange::Bar => vec![(self.bar, 0, usize::MAX)],
            VoiceRange::Track => (0..self.song.bar_count()).map(|b| (b, 0, usize::MAX)).collect(),
        };
        if self.vaction == VoiceAction::Swap {
            let bars: Vec<usize> = ranges.iter().map(|r| r.0).collect();
            let ti = self.track;
            let keep = self.voice;
            self.voice = 0;
            self.edit(|s, _, _, _, _| {
                for bi in &bars {
                    let bar = &mut s.tracks[ti].bars[*bi];
                    let a = bar.voice_mut(from).clone();
                    let b = std::mem::replace(bar.voice_mut(to), a);
                    *bar.voice_mut(from) = b;
                }
            });
            self.voice = keep;
            self.clamp_cursor();
            self.status = format!("Swapped voices {} and {} in {} bar(s).", from + 1, to + 1, bars.len());
            return;
        }
        let mut items = vec![];
        for (bi, s, e) in ranges {
            let beats = self.song.tracks[self.track].bars[bi].voice(from);
            for (j, b) in beats.iter().enumerate().skip(s) {
                if j > e {
                    break;
                }
                if !b.notes.is_empty() {
                    items.push((bi, j, b.notes.iter().map(|n| n.string).collect::<Vec<u8>>()));
                }
            }
        }
        let copy = self.vaction == VoiceAction::Copy;
        let (moved, blocked) = self.transfer(from, to, &items, copy);
        self.clamp_cursor();
        self.report(moved, blocked, to, if copy { "Copied" } else { "Moved" });
    }
}
