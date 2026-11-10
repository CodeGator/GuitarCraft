//! The editing cursor in the standard-notation staff (as opposed to the tab).
//! In the notation the cursor sits on a line or space; in the tab on a string.

use super::App;
use crate::model::*;
use crate::score;

impl App {
    fn key_here(&self) -> i8 {
        self.song.master[self.bar].key
    }

    /// Staff position of a note of the current beat.
    pub(super) fn staff_pos_of(&self, n: &Note) -> i32 {
        score::note_staff_pos(self.tr(), self.cur_beat(), self.key_here(), n)
    }

    /// The note of the current beat sitting on the notation cursor's line or space.
    pub(super) fn note_at_std(&self) -> Option<Note> {
        let pos = self.std_cursor?;
        self.cur_beat()
            .notes
            .iter()
            .find(|n| self.staff_pos_of(n) == pos)
            .cloned()
    }

    /// Keeps the cursor in a staff that is shown, and the string in step with the note under it.
    pub(super) fn fix_cursor_mode(&mut self) {
        let (std, tab) = score::staves(self.tr(), self.notation);
        if !std {
            self.std_cursor = None;
        } else if !tab && self.std_cursor.is_none() {
            self.enter_std();
        }
        if let Some(n) = self.note_at_std() {
            self.string = n.string as usize;
        }
    }

    /// Moves the cursor into the notation staff, onto the note of the current string if there is one.
    pub(super) fn enter_std(&mut self) {
        let pos = self
            .cur_beat()
            .note_on(self.string as u8)
            .cloned()
            .map(|n| self.staff_pos_of(&n))
            .unwrap_or(4);
        self.std_cursor = Some(pos);
    }

    /// Up/down in the notation: one line or space at a time.
    pub(super) fn std_step(&mut self, dir: i32) {
        if let Some(p) = self.std_cursor {
            let low = if self.tr().grand() { -20 } else { -8 };
            self.std_cursor = Some((p + dir).clamp(low, 16));
            self.last_digit = None;
        }
    }

    /// Tab key: notation and tab of a track, then the next track.
    pub(super) fn tab_key(&mut self, dir: i32) {
        let (std, tab) = score::staves(self.tr(), self.notation);
        let both = std && tab;
        match (self.std_cursor.is_some(), dir > 0, both) {
            (true, true, true) => self.std_cursor = None,
            (false, false, true) => self.enter_std(),
            (true, false, true) => {
                self.next_staff(-1);
                self.std_cursor = None;
            }
            _ => {
                let in_std = self.std_cursor.is_some();
                self.next_staff(dir);
                if in_std || (both && dir > 0) {
                    self.enter_std();
                }
            }
        }
    }

    /// A digit in the notation: put the note at the cursor's pitch on string 1-9 (0 = best string).
    pub(super) fn std_digit(&mut self, d: u8) {
        let Some(pos) = self.std_cursor else { return };
        let track = self.tr().clone();
        // Without strings, any digit just puts the note in.
        let d = if track.has_strings() { d } else { 0 };
        let existing = self.note_at_std();
        let pitch = existing
            .as_ref()
            .map(|n| track.pitch(n) as i32)
            .unwrap_or_else(|| {
                score::staff_pos_pitch(&track, self.key_here(), self.cur_beat().ottava, pos)
            });
        let fret_on = |s: usize| pitch - track.tuning[s] as i32 - track.capo_at(s) as i32;
        let fits = |s: usize| (0..=track.frets as i32).contains(&fret_on(s));
        let free = |s: usize| {
            self.cur_beat()
                .note_on(s as u8)
                .map(|n| Some(n.string) == existing.as_ref().map(|e| e.string))
                .unwrap_or(true)
        };
        let string = if d == 0 {
            match (0..track.strings())
                .filter(|&s| fits(s) && free(s))
                .min_by_key(|&s| fret_on(s))
            {
                Some(s) => s,
                None => {
                    self.status = format!(
                        "{} can't be played on this instrument.",
                        note_name_oct(pitch.clamp(0, 127) as u8)
                    );
                    return;
                }
            }
        } else {
            let s = d as usize - 1;
            if s >= track.strings() {
                self.status = format!("This track has {} strings.", track.strings());
                return;
            }
            if !fits(s) {
                self.status = format!(
                    "{} can't be played on string {}.",
                    note_name_oct(pitch.clamp(0, 127) as u8),
                    s + 1
                );
                return;
            }
            s
        };
        // Moving an existing note to another string: one undo step for both changes.
        let moved = match &existing {
            Some(e) if e.string as usize != string => {
                let es = e.string;
                self.edit(move |so, t, b, j, _| {
                    so.tracks[t].bars[b].beats[j]
                        .notes
                        .retain(|n| n.string != es)
                });
                true
            }
            _ => false,
        };
        self.set_fret(string, fret_on(string) as u8);
        if moved {
            self.undo.pop();
        }
        self.string = string;
        self.std_cursor = Some(pos);
    }

    /// Delete in the notation removes only the note under the cursor.
    pub(super) fn std_delete(&mut self) {
        if self.selection().is_some() {
            self.delete_at_cursor();
            return;
        }
        if let Some(n) = self.note_at_std() {
            let s = n.string;
            self.edit(move |so, t, b, j, _| {
                so.tracks[t].bars[b].beats[j]
                    .notes
                    .retain(|x| x.string != s)
            });
        } else if self.cur_beat().is_rest()
            && self.song.tracks[self.track].bars[self.bar]
                .voice(self.voice)
                .len()
                > 1
        {
            self.edit(|so, t, b, j, _| {
                so.tracks[t].bars[b].beats.remove(j);
            });
        }
    }

    /// Name of the note (or notation line/space) under the cursor, for the toolbar display.
    pub(super) fn cursor_note_name(&self) -> Option<String> {
        let track = self.tr();
        let beat = self.cur_beat();
        let key = self.key_here();
        match self.std_cursor {
            Some(pos) => Some(match self.note_at_std() {
                Some(n) => score::written_name(track, beat, key, &n),
                None => score::staff_pos_name(track, key, pos),
            }),
            None => beat
                .note_on(self.string as u8)
                .map(|n| score::written_name(track, beat, key, n)),
        }
    }
}
