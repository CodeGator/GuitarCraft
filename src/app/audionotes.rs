//! Note > Audio Note Settings, as in Guitar Pro 8: how long a note sounds, how much early or
//! late it starts and how loud it is, apart from what is written. The window stays open while
//! the cursor moves, shows the settings of the note (or selection) it is on, and changes them at
//! once. While it is open the score marks the notes that have settings of their own.

use super::App;
use crate::model::{Note, NotePlayback};
use egui::{Align2, RichText, Vec2};

/// The open window.
#[derive(Debug, Default)]
pub struct AudioNotes {
    /// The notes changed last and the undo steps there were afterwards: further changes to the
    /// same notes join that one step.
    last: Option<(Vec<Target>, usize)>,
}

/// A note: its track, bar, voice, beat and string.
type Target = (usize, usize, usize, usize, u8);

/// The controls' ranges. The duration's is Guitar Pro's; the others' aren't published.
const PERCENT: std::ops::RangeInclusive<u16> = 25..=1200;
const TICKS: std::ops::RangeInclusive<i16> = -480..=480;
const DECIBELS: std::ops::RangeInclusive<i8> = -30..=30;

impl App {
    pub(super) fn open_audio_notes(&mut self) {
        self.audio_notes = Some(AudioNotes::default());
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
        let note = self.song.tracks[track].bars[bar].voice(voice).get(beat).and_then(|b| b.note_on(string));
        note.map(|n: &Note| n.playback).unwrap_or_default()
    }

    /// Changes one setting of every target, as one undo step with the changes just before to
    /// the same notes.
    pub(super) fn set_note_playback(&mut self, change: impl Fn(&mut NotePlayback)) {
        let targets = self.audio_targets();
        if targets.is_empty() {
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
        let shown = targets.first().map(|&t| self.playback_of(t)).unwrap_or_default();
        let mixed = targets.iter().any(|&t| self.playback_of(t) != shown);
        let mut now = shown;
        let mut reset = false;
        let mut open = true;
        egui::Window::new("Audio Note Settings")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            // To the right, clear of the start of the lines of music.
            .default_pos(ctx.content_rect().right_center() - Vec2::new(24.0, 0.0))
            .pivot(Align2::RIGHT_CENTER)
            .show(ctx, |ui| {
                ui.style_mut().text_styles.iter_mut().for_each(|(_, f)| f.size = f.size.max(15.0));
                ui.set_min_width(260.0);
                ui.add_enabled_ui(!targets.is_empty(), |ui| {
                    egui::Grid::new("audio-notes").num_columns(2).spacing([12.0, 10.0]).show(ui, |ui| {
                        ui.label("Duration:");
                        ui.add(egui::DragValue::new(&mut now.percent).range(PERCENT).clamp_existing_to_range(false).suffix("%"));
                        ui.end_row();
                        ui.label("Offset:");
                        ui.add(egui::DragValue::new(&mut now.offset).range(TICKS).clamp_existing_to_range(false).suffix(" ticks"));
                        ui.end_row();
                        ui.label("Relative velocity:");
                        ui.add(egui::DragValue::new(&mut now.db).range(DECIBELS).clamp_existing_to_range(false).custom_formatter(|v, _| if v > 0.0 { format!("+{v:.0} dB") } else { format!("{v:.0} dB") }).custom_parser(|s| s.trim().trim_end_matches("dB").trim().parse().ok()));
                        ui.end_row();
                    });
                    ui.add_space(8.0);
                    reset = ui.button("Reset").clicked();
                });
                let note = if targets.is_empty() {
                    "Put the cursor on a note, or select notes.".to_string()
                } else if mixed {
                    format!("{} notes with different settings: a change applies to all.", targets.len())
                } else if targets.len() > 1 {
                    format!("{} notes.", targets.len())
                } else {
                    String::new()
                };
                if !note.is_empty() {
                    ui.add_space(6.0);
                    ui.label(RichText::new(note).weak());
                }
            });
        // Only the control that moved is applied, so the others keep each note's own value.
        if reset {
            self.set_note_playback(|p| *p = NotePlayback::default());
        } else if now.percent != shown.percent {
            self.set_note_playback(move |p| p.percent = now.percent);
        } else if now.offset != shown.offset {
            self.set_note_playback(move |p| p.offset = now.offset);
        } else if now.db != shown.db {
            self.set_note_playback(move |p| p.db = now.db);
        }
        if !open {
            self.audio_notes = None;
        }
    }
}
