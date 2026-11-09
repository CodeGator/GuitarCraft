//! Commands used by the menus and shortcuts, plus the small dialogs and the keyboard view.

use super::panels::BLUE;
use super::App;
use crate::audio;
use crate::fileio;
use crate::model::*;
use egui::{Align2, Color32, FontId, Pos2, Rect, RichText, Sense, Ui, Vec2};
use std::path::PathBuf;

impl App {
    pub(super) fn dialog_open(&self) -> bool {
        self.goto_open || self.transpose_open || self.cmd_open || self.addtrack_open || self.chord_open || self.popup.is_some() || self.voices_open || self.auto_open
    }

    pub(super) fn check_bar_duration(&mut self) {
        let tr = &self.song.tracks[self.track];
        let n = self.song.bar_count();
        let found = (1..=n).map(|k| (self.bar + k) % n).find(|&b| {
            let bar = &tr.bars[b];
            bar.wrong_length(self.song.master[b].ticks())
        });
        match found {
            Some(b) => {
                self.goto_bar(b);
                self.status = format!("Bar {} doesn't add up to its time signature.", b + 1);
            }
            None => self.status = "Every bar in this track has the right length.".into(),
        }
    }

    pub(super) fn toggle_fullscreen(&mut self) {
        self.fullscreen = !self.fullscreen;
        self.fullscreen_changed = true;
    }

    pub(super) fn zoom_step(&mut self, dir: i32) {
        const STEPS: [f32; 9] = [0.5, 0.6, 0.75, 0.9, 1.0, 1.1, 1.25, 1.5, 2.0];
        let i = STEPS.iter().position(|&z| z >= self.zoom - 0.001).unwrap_or(4) as i32;
        let j = (i + dir).clamp(0, STEPS.len() as i32 - 1) as usize;
        self.zoom = STEPS[j];
    }

    pub(super) fn play_from_start(&mut self) {
        self.stop();
        self.goto_bar(0);
        self.play();
    }

    pub(super) fn move_track(&mut self, dir: i32) {
        let i = self.track as i32;
        let j = i + dir;
        if j < 0 || j >= self.song.tracks.len() as i32 {
            return;
        }
        self.edit(|s, _, _, _, _| s.tracks.swap(i as usize, j as usize));
        self.track = j as usize;
    }

    pub(super) fn duplicate_track(&mut self) {
        let ti = self.track;
        self.edit(|s, _, _, _, _| {
            let mut t = s.tracks[ti].clone();
            t.name = format!("{} (copy)", t.name);
            t.solo = false;
            s.tracks.insert(ti + 1, t);
        });
        self.track = ti + 1;
        self.clamp_cursor();
    }

    pub(super) fn goto_section(&mut self, dir: i32) {
        let marks: Vec<usize> = self.song.master.iter().enumerate().filter(|(_, m)| m.marker.is_some()).map(|(i, _)| i).collect();
        let target = if dir > 0 { marks.iter().find(|&&b| b > self.bar) } else { marks.iter().rev().find(|&&b| b < self.bar) };
        if let Some(&b) = target {
            self.goto_bar(b);
        }
    }

    pub(super) fn edit_section(&mut self) {
        self.open_popup(super::popups::Popup::Section);
    }

    pub(super) fn shift_octave(&mut self, dir: i32) {
        let maxf = self.tr().frets as i32;
        let ok = self.cur_beat().notes.iter().all(|n| (0..=maxf).contains(&(n.fret as i32 + 12 * dir)));
        if !ok {
            self.status = "That would go past the end of the fretboard.".into();
            return;
        }
        self.transpose(12 * dir);
    }

    /// Deletes the selected beats, or the beat under the cursor.
    pub(super) fn delete_beats(&mut self) {
        if self.selection().is_some() {
            self.delete_at_cursor();
            return;
        }
        if self.song.tracks[self.track].bars[self.bar].voice(self.voice).len() > 1 {
            self.edit(|s, t, b, j, _| {
                s.tracks[t].bars[b].beats.remove(j);
            });
        } else {
            self.edit(|s, t, b, _, _| s.tracks[t].bars[b] = Bar::empty());
        }
    }

    fn bar_range(&self) -> (usize, usize) {
        match self.selection() {
            Some((a, b)) => (a.0, b.0),
            None => (self.bar, self.bar),
        }
    }

    /// Copies whole bars of every track.
    pub(super) fn all_track_copy(&mut self) {
        let (a, b) = self.bar_range();
        self.bar_clipboard = (a..=b).map(|i| (self.song.master[i].clone(), self.song.tracks.iter().map(|t| t.bars[i].clone()).collect())).collect();
        self.status = format!("Copied {} bar(s) of every track.", b - a + 1);
    }

    pub(super) fn all_track_cut(&mut self) {
        self.all_track_copy();
        let (a, b) = self.bar_range();
        if b - a + 1 >= self.song.bar_count() {
            self.status = "A song needs at least one bar.".into();
            return;
        }
        self.edit(|s, _, _, _, _| {
            s.master.drain(a..=b);
            for t in &mut s.tracks {
                t.bars.drain(a..=b);
            }
        });
        self.sel_anchor = None;
        self.goto_bar(a.min(self.song.bar_count() - 1));
    }

    /// Inserts copied bars before the current bar.
    pub(super) fn all_track_paste(&mut self) {
        if self.bar_clipboard.is_empty() {
            return;
        }
        let clip = self.bar_clipboard.clone();
        let at = self.bar;
        let n = clip.len();
        self.edit(|s, _, _, _, _| {
            for (k, (mb, bars)) in clip.into_iter().enumerate() {
                s.master.insert(at + k, mb);
                for (ti, t) in s.tracks.iter_mut().enumerate() {
                    t.bars.insert(at + k, bars.get(ti).cloned().unwrap_or_else(Bar::empty));
                }
            }
        });
        self.status = format!("Pasted {n} bar(s).");
    }

    /// Fills bars that are too short with rests and trims bars that are too long.
    pub(super) fn complete_bars(&mut self) {
        self.edit(|s, t, _, _, _| {
            for (bi, bar) in s.tracks[t].bars.iter_mut().enumerate() {
                let need = s.master[bi].ticks();
                if bar.beats.len() == 1 && bar.is_empty() {
                    continue;
                }
                while bar.ticks() > need && bar.beats.len() > 1 && bar.beats.last().map(|b| b.is_rest()).unwrap_or(false) {
                    bar.beats.pop();
                }
                let mut left = need.saturating_sub(bar.ticks());
                for d in Dur::ALL {
                    while left >= d.ticks() {
                        bar.beats.push(Beat::rest(d));
                        left -= d.ticks();
                    }
                }
            }
        });
        self.status = "Short bars were filled with rests.".into();
    }

    fn templates_dir() -> Option<PathBuf> {
        let base = std::env::var_os("APPDATA").or_else(|| std::env::var_os("HOME"))?;
        Some(PathBuf::from(base).join("GuitarCraft").join("Templates"))
    }

    pub(super) fn user_templates() -> Vec<PathBuf> {
        let Some(d) = Self::templates_dir() else { return vec![] };
        let mut v: Vec<PathBuf> = std::fs::read_dir(d).map(|r| r.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.extension().map(|e| e == fileio::GP_EXT || fileio::OLD_EXTS.iter().any(|x| e == *x)).unwrap_or(false)).collect()).unwrap_or_default();
        v.sort();
        v
    }

    pub(super) fn save_as_template(&mut self) {
        let Some(d) = Self::templates_dir() else { return };
        let _ = std::fs::create_dir_all(&d);
        let mut t = self.song.clone();
        for tr in &mut t.tracks {
            for b in &mut tr.bars {
                *b = Bar::empty();
            }
        }
        let name = if self.song.title.trim().is_empty() { "My template".to_string() } else { self.song.title.clone() };
        let p = d.join(format!("{}.{}", super::widgets::sanitize(&name), fileio::GP_EXT));
        match fileio::save_song(&t, &p) {
            Ok(()) => self.status = format!("Saved the template \u{201C}{name}\u{201D}. Find it in File \u{25B8} New from Template."),
            Err(e) => self.status = format!("Couldn't save the template: {e}"),
        }
    }

    pub(super) fn open_template_file(&mut self, p: PathBuf) {
        match fileio::load_song(&p) {
            Ok(mut s) => {
                self.stop();
                s.title = "Untitled".into();
                self.song = s;
                self.path = None;
                self.reset_after_load();
                self.show_home = false;
            }
            Err(e) => self.status = format!("Couldn't open the template: {e}"),
        }
    }

    pub(super) fn open_containing_folder(&mut self) {
        let Some(p) = self.path.clone() else {
            self.status = "Save the song first.".into();
            return;
        };
        #[cfg(target_os = "windows")]
        let r = std::process::Command::new("explorer").arg(format!("/select,{}", p.display())).spawn();
        #[cfg(target_os = "macos")]
        let r = std::process::Command::new("open").arg("-R").arg(&p).spawn();
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        let r = std::process::Command::new("xdg-open").arg(p.parent().unwrap_or(&p)).spawn();
        if let Err(e) = r {
            self.status = format!("Couldn't open the folder: {e}");
        }
    }

    pub(super) fn close_song(&mut self) {
        self.stop();
        self.song = Song::new_blank();
        self.path = None;
        self.reset_after_load();
        self.show_home = true;
    }

    /// Plays the open strings from lowest to highest.
    pub(super) fn play_tuning(&mut self) {
        if !self.sound_allowed {
            return;
        }
        self.ensure_audio();
        let Some(a) = &self.audio else { return };
        if a.error.is_some() {
            return;
        }
        let sr = a.sample_rate();
        let tr = self.song.tracks[self.track].clone();
        if let Ok(mut st) = a.state.lock() {
            for (k, s) in (0..tr.strings()).rev().enumerate() {
                let mut ev = audio::note_preview(&tr, self.track, &Note::new(s as u8, 0), sr);
                ev.start = (k as f32 * sr * 0.35) as u64;
                ev.len = (sr * 0.9) as u64;
                st.preview.push(ev);
            }
        }
    }

    pub(super) fn transpose_song(&mut self, semis: i32, all_tracks: bool) {
        let cur = self.track;
        let mut clipped = false;
        let range = if self.transpose_sel_only { self.selection().map(|(a, b)| (a.0, b.0)).unwrap_or((self.bar, self.bar)) } else { (0, self.song.bar_count() - 1) };
        let diatonic = self.transpose_diatonic;
        let change_key = self.transpose_key && !diatonic;
        self.edit(|s, _, _, _, _| {
            let keys: Vec<i8> = s.master.iter().map(|m| m.key).collect();
            for (ti, t) in s.tracks.iter_mut().enumerate() {
                if !all_tracks && ti != cur {
                    continue;
                }
                let maxf = t.frets as i32;
                let nstr = t.tuning.len();
                let tuning = t.tuning.clone();
                for (bi, bar) in t.bars.iter_mut().enumerate() {
                    if bi < range.0 || bi > range.1 {
                        continue;
                    }
                    let key = keys[bi];
                    for beat in bar.all_beats_mut() {
                        let mut out: Vec<Note> = vec![];
                        for n in beat.notes.drain(..) {
                            let from = tuning[n.string as usize] as i32 + n.fret as i32;
                            let target = if diatonic { diatonic_shift(from, key, semis) } else { from + semis };
                            // Same string if possible, otherwise the nearest free string.
                            let mut placed = None;
                            let mut order: Vec<usize> = (0..nstr).collect();
                            order.sort_by_key(|&s| (s as i32 - n.string as i32).abs());
                            for s in order {
                                let f = target - tuning[s] as i32;
                                if (0..=maxf).contains(&f) && !out.iter().any(|o| o.string as usize == s) {
                                    placed = Some((s, f));
                                    break;
                                }
                            }
                            match placed {
                                Some((s, f)) => {
                                    let mut m = n.clone();
                                    m.string = s as u8;
                                    m.fret = f as u8;
                                    out.push(m);
                                }
                                None => clipped = true,
                            }
                        }
                        out.sort_by_key(|n| n.string);
                        beat.notes = out;
                    }
                }
            }
        });
        if change_key {
            let (a, b) = range;
            self.edit(|s, _, _, _, _| {
                for m in &mut s.master[a..=b] {
                    let k = (m.key as i32 + semis * 7).rem_euclid(12);
                    m.key = if k > 6 { (k - 12) as i8 } else { k as i8 };
                }
            });
        }
        self.status = if clipped {
            "Transposed. Some notes didn't fit on the fretboard and were removed.".into()
        } else {
            format!("Transposed by {semis:+} semitone(s).")
        };
    }

    pub(super) fn small_dialogs(&mut self, ctx: &egui::Context) {
        if self.goto_open {
            let mut open = true;
            let mut go = false;
            egui::Window::new("Go to").collapsible(false).resizable(false).open(&mut open).anchor(Align2::CENTER_CENTER, Vec2::ZERO).show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(format!("Bar (1\u{2013}{}):", self.song.bar_count()));
                    let r = ui.add(egui::TextEdit::singleline(&mut self.goto_text).desired_width(60.0));
                    r.request_focus();
                    if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        go = true;
                    }
                });
                let marks: Vec<(usize, String)> = self.song.master.iter().enumerate().filter_map(|(i, m)| m.marker.clone().map(|n| (i, n))).collect();
                if !marks.is_empty() {
                    ui.label(RichText::new("Or a section:").weak());
                    ui.horizontal_wrapped(|ui| {
                        for (i, n) in marks {
                            if ui.button(n).clicked() {
                                self.goto_text = format!("{}", i + 1);
                                go = true;
                            }
                        }
                    });
                }
                if ui.button("Go").clicked() {
                    go = true;
                }
            });
            if go {
                if let Ok(b) = self.goto_text.trim().parse::<usize>() {
                    self.goto_bar(b.saturating_sub(1));
                }
                open = false;
            }
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                open = false;
            }
            self.goto_open = open;
        }
        if self.transpose_open {
            let mut open = true;
            let mut apply = false;
            egui::Window::new("Transpose").collapsible(false).resizable(false).open(&mut open).anchor(Align2::CENTER_CENTER, Vec2::ZERO).show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Semitones:");
                    ui.add(egui::DragValue::new(&mut self.transpose_amt).range(-24..=24));
                });
                ui.horizontal(|ui| {
                    ui.radio_value(&mut self.transpose_diatonic, false, "Semitones");
                    ui.radio_value(&mut self.transpose_diatonic, true, "Steps of the key's scale");
                });
                ui.horizontal(|ui| {
                    ui.radio_value(&mut self.transpose_all, false, "This track");
                    ui.radio_value(&mut self.transpose_all, true, "Every track");
                });
                ui.horizontal(|ui| {
                    ui.radio_value(&mut self.transpose_sel_only, false, "All bars");
                    ui.radio_value(&mut self.transpose_sel_only, true, "Selected bars");
                });
                ui.add_enabled(!self.transpose_diatonic, egui::Checkbox::new(&mut self.transpose_key, "Change the key signature too"));
                ui.label(RichText::new("Notes keep their string where they fit, otherwise they move to a nearby string.").small().weak());
                ui.horizontal(|ui| {
                    if ui.button("Transpose").clicked() {
                        apply = true;
                    }
                    if ui.button("Cancel").clicked() {
                        self.transpose_open = false;
                    }
                });
            });
            if apply && self.transpose_amt != 0 {
                let (a, all) = (self.transpose_amt, self.transpose_all);
                self.transpose_song(a, all);
                open = false;
            }
            self.transpose_open = open && self.transpose_open;
        }
    }

    /// The piano keyboard view: shows the notes of the current beat (or what is playing) and adds notes when clicked.
    pub(super) fn keyboard_panel(&mut self, ui: &mut Ui, sounding: &[(usize, u8, u8)]) {
        let full = ui.max_rect();
        let p = ui.painter().clone();
        let side = Rect::from_min_size(full.min, Vec2::new(56.0, full.height()));
        p.rect_filled(side, 0.0, Color32::from_rgb(40, 43, 50));
        let tr = self.song.tracks[self.track].clone();
        let mut lit: Vec<(u8, Color32)> = vec![];
        if self.playing {
            for &(t, s, f) in sounding {
                if t == self.track {
                    lit.push((tr.pitch(&Note::new(s, f)), Color32::from_rgb(240, 130, 20)));
                }
            }
        } else {
            for n in &self.cur_beat().notes {
                lit.push((tr.pitch(n), BLUE));
            }
        }
        // Octave buttons.
        for (k, lab) in [(-1i32, "<"), (1, ">")] {
            let r = Rect::from_min_size(Pos2::new(side.left() + 6.0 + if k > 0 { 22.0 } else { 0.0 }, side.top() + 8.0), Vec2::new(20.0, 20.0));
            let resp = ui.interact(r, ui.id().with(("kbo", k)), Sense::click()).on_hover_text("Shift the keyboard by an octave");
            p.rect_filled(r, 3.0, if resp.hovered() { Color32::from_gray(90) } else { Color32::from_gray(70) });
            p.text(r.center(), Align2::CENTER_CENTER, lab, FontId::proportional(14.0), Color32::WHITE);
            if resp.clicked() {
                self.kb_octave = (self.kb_octave + k).clamp(0, 5);
            }
        }
        p.text(Pos2::new(side.center().x, side.top() + 44.0), Align2::CENTER_CENTER, format!("C{}", self.kb_octave + 1), FontId::proportional(11.0), Color32::from_gray(180));
        let area = Rect::from_min_max(Pos2::new(side.right() + 2.0, full.top() + 2.0), full.max - Vec2::new(2.0, 2.0));
        // As many keys as fit, up to a full 88-key piano (52 white keys).
        let max_white = 52 - (self.kb_octave - 1).max(0) * 7;
        let n_white = ((area.width() / 17.0) as i32).clamp(14, max_white.max(14));
        let ww = area.width() / n_white as f32;
        let start = 12 * (self.kb_octave + 1) + 12; // C of the chosen octave
        let is_black = |m: i32| matches!(m.rem_euclid(12), 1 | 3 | 6 | 8 | 10);
        let lit_col = |m: i32| lit.iter().find(|(x, _)| *x as i32 == m).map(|(_, c)| *c);
        let mut whites = vec![];
        let mut m = start;
        while whites.len() < n_white as usize {
            if !is_black(m) {
                whites.push(m);
            }
            m += 1;
        }
        let mut keys: Vec<(i32, Rect)> = vec![];
        for (i, &w) in whites.iter().enumerate() {
            let r = Rect::from_min_size(Pos2::new(area.left() + i as f32 * ww, area.top()), Vec2::new(ww - 1.0, area.height()));
            p.rect_filled(r, 2.0, lit_col(w).unwrap_or(Color32::from_gray(245)));
            if w % 12 == 0 {
                p.text(Pos2::new(r.center().x, r.bottom() - 9.0), Align2::CENTER_CENTER, format!("C{}", w / 12 - 1), FontId::proportional(9.0), Color32::from_gray(120));
            }
            keys.push((w, r));
        }
        let mut blacks: Vec<(i32, Rect)> = vec![];
        for (i, &w) in whites.iter().enumerate() {
            if is_black(w + 1) && i + 1 < whites.len() {
                let r = Rect::from_min_size(Pos2::new(area.left() + (i as f32 + 1.0) * ww - ww * 0.32, area.top()), Vec2::new(ww * 0.62, area.height() * 0.6));
                p.rect_filled(r, 2.0, lit_col(w + 1).unwrap_or(Color32::from_rgb(28, 30, 34)));
                blacks.push((w + 1, r));
            }
        }
        let resp = ui.interact(area, ui.id().with("kbd"), Sense::click());
        if resp.clicked() {
            if let Some(pp) = resp.interact_pointer_pos() {
                let hit = blacks.iter().find(|(_, r)| r.contains(pp)).or_else(|| keys.iter().find(|(_, r)| r.contains(pp))).map(|(m, _)| *m);
                if let Some(m) = hit {
                    self.place_pitch(m);
                }
            }
        }
    }

    /// Puts a note of this pitch on the current beat, choosing a string it fits on.
    fn place_pitch(&mut self, pitch: i32) {
        let tr = self.tr().clone();
        let fret_on = |s: usize| pitch - tr.tuning[s] as i32 - tr.capo_at(s) as i32;
        let fits = |s: usize| (0..=tr.frets as i32).contains(&fret_on(s));
        let mut choice = if fits(self.string) { Some(self.string) } else { None };
        if choice.is_none() {
            choice = (0..tr.strings()).filter(|&s| fits(s)).min_by_key(|&s| fret_on(s));
        }
        match choice {
            Some(s) => {
                self.string = s;
                self.set_fret(s, fret_on(s) as u8);
            }
            None => self.status = format!("{} can't be played on this instrument.", note_name_oct(pitch.clamp(0, 127) as u8)),
        }
    }
}

impl App {
    /// Pastes beats, or whole bars after an all-track copy.
    pub(super) fn paste_any(&mut self) {
        if self.last_copy_all && !self.bar_clipboard.is_empty() {
            self.all_track_paste();
        } else {
            self.paste();
        }
    }

    /// Moves one beat and plays it, without starting playback.
    pub(super) fn step(&mut self, dir: i32) {
        if dir < 0 {
            self.move_left(false);
        } else {
            let nb = self.song.tracks[self.track].bars[self.bar].voice(self.voice).len();
            if self.beat + 1 < nb {
                self.beat += 1;
            } else if self.bar + 1 < self.song.bar_count() {
                self.bar += 1;
                self.beat = 0;
            }
            self.scroll_to_cursor = true;
        }
        for n in self.cur_beat().notes.clone() {
            if !n.tie {
                self.preview(&n);
            }
        }
    }

    /// Moves the note under the cursor to the neighbouring string, keeping its pitch.
    pub(super) fn move_note_string(&mut self, dir: i32) {
        let s = self.string as i32;
        let to = s + dir;
        let tr = self.tr().clone();
        if to < 0 || to >= tr.strings() as i32 {
            return;
        }
        let Some(n) = self.cur_beat().note_on(s as u8).cloned() else { return };
        if self.cur_beat().note_on(to as u8).is_some() {
            self.status = "That string already has a note.".into();
            return;
        }
        let pitch = tr.tuning[s as usize] as i32 + n.fret as i32;
        let f = pitch - tr.tuning[to as usize] as i32;
        if !(0..=tr.frets as i32).contains(&f) {
            self.status = "The note doesn't fit on that string.".into();
            return;
        }
        self.edit(|so, t, b, j, _| {
            let beat = &mut so.tracks[t].bars[b].beats[j];
            if let Some(x) = beat.note_on_mut(s as u8) {
                x.string = to as u8;
                x.fret = f as u8;
            }
            beat.notes.sort_by_key(|n| n.string);
        });
        self.string = to as usize;
    }

    /// Next or previous visible track staff, same bar.
    pub(super) fn next_staff(&mut self, dir: i32) {
        let shown = self.shown_tracks();
        if shown.len() < 2 {
            return;
        }
        let i = shown.iter().position(|&t| t == self.track).unwrap_or(0) as i32;
        let j = (i + dir).rem_euclid(shown.len() as i32) as usize;
        self.track = shown[j];
        self.clamp_cursor();
        self.scroll_to_cursor = true;
    }

    /// Copies the previous beat onto the cursor's beat and moves on.
    pub(super) fn copy_last_beat(&mut self) {
        let prev = if self.beat > 0 {
            Some(self.song.tracks[self.track].bars[self.bar].voice(self.voice)[self.beat - 1].clone())
        } else if self.bar > 0 {
            self.song.tracks[self.track].bars[self.bar - 1].voice(self.voice).last().cloned()
        } else {
            None
        };
        if let Some(p) = prev {
            self.edit(move |s, t, b, j, _| s.tracks[t].bars[b].beats[j] = p);
            self.move_right(false);
        }
    }

    /// Ties every note of the beat to the same note in the previous beat.
    pub(super) fn tie_beat(&mut self) {
        let prev = if self.beat > 0 {
            Some(self.song.tracks[self.track].bars[self.bar].voice(self.voice)[self.beat - 1].clone())
        } else if self.bar > 0 {
            self.song.tracks[self.track].bars[self.bar - 1].voice(self.voice).last().cloned()
        } else {
            None
        };
        let Some(p) = prev else { return };
        if p.is_rest() {
            return;
        }
        self.beat_edit(move |b| {
            b.notes = p
                .notes
                .iter()
                .map(|n| {
                    let mut x = Note::new(n.string, n.fret);
                    x.tie = true;
                    x
                })
                .collect();
        });
    }
}

/// After a tuning or capo change, moves notes so they keep sounding the same.
pub fn refit_track(t: &mut Track, old_tuning: &[u8], old_capo: u8) {
    refit_notes(t, old_tuning, old_capo);
}

pub fn shortcut_rows(ui: &mut Ui) {
    egui::Grid::new("keys").striped(true).show(ui, |ui| {
        for (k, d) in super::keys::SHORTCUTS {
            ui.label(RichText::new(k).strong());
            ui.label(d);
            ui.end_row();
        }
    });
}


/// Moves a pitch by steps of the major scale of a key (sharps > 0, flats < 0).
pub fn diatonic_shift(pitch: i32, key: i8, steps: i32) -> i32 {
    const MAJOR: [i32; 7] = [0, 2, 4, 5, 7, 9, 11];
    let root = (key as i32 * 7).rem_euclid(12);
    let rel = pitch - root;
    let oct = rel.div_euclid(12);
    let pc = rel.rem_euclid(12);
    // Nearest scale degree at or below the note; keep any chromatic offset.
    let deg = MAJOR.iter().rposition(|&d| d <= pc).unwrap_or(0) as i32;
    let extra = pc - MAJOR[deg as usize];
    let nd = deg + steps;
    let noct = oct + nd.div_euclid(7);
    root + noct * 12 + MAJOR[nd.rem_euclid(7) as usize] + extra
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diatonic_steps() {
        // C major: C -> E (3 degrees up as a third), G -> B.
        assert_eq!(diatonic_shift(60, 0, 2), 64);
        assert_eq!(diatonic_shift(67, 0, 2), 71);
        // G major: B -> D going up two steps.
        assert_eq!(diatonic_shift(59, 1, 2), 62);
        assert_eq!(diatonic_shift(60, 0, -1), 59);
    }
}
