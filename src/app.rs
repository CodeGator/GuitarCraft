//! The main window: state, editing commands, playback and keyboard input.
//! The panels are drawn in `panels.rs`.

mod actions;
mod automation;
mod bottom;
mod cursor;
mod home;
mod icons;
mod keys;
mod menus;
mod panels;
mod popups;
mod side;
mod stylesheet;
mod tools;
pub mod trackicons;
mod tuning;
mod voices;
mod widgets;

use crate::audio::{self, Audio, ScheduleOptions, TimelineBar};
use crate::fileio;
use crate::model::*;
use crate::score::Notation;
use egui::Key;
use std::path::PathBuf;
use widgets::sanitize;

#[derive(Clone, Copy, PartialEq)]
enum Pending {
    New,
    Open,
    Demo,
    Template(usize),
    Close,
    Quit,
}

#[derive(Clone, Copy, PartialEq)]
enum InspectorTab {
    Song,
    Track,
}

pub struct App {
    song: Song,
    path: Option<PathBuf>,
    dirty: bool,
    track: usize,
    bar: usize,
    beat: usize,
    string: usize,
    sel_anchor: Option<(usize, usize)>,
    undo: Vec<Song>,
    redo: Vec<Song>,
    clipboard: Vec<Beat>,
    cur_dur: Dur,
    zoom: f32,
    notation: Notation,
    audio: Option<Audio>,
    sound_allowed: bool,
    playing: bool,
    timeline: Vec<TimelineBar>,
    start_offset: u64,
    play_start_bar: usize,
    play_beat: Option<(usize, usize)>,
    /// Where playback is: the bar and the exact tick within it.
    play_tick: Option<(usize, f32)>,
    play_secs: f32,
    total_secs: f32,
    metronome: bool,
    count_in: bool,
    looping: bool,
    speed: f32,
    master_volume: f32,
    status: String,
    last_digit: Option<(u8, f64, usize, usize, usize)>,
    show_fretboard: bool,
    show_tracks: bool,
    show_palette: bool,
    show_inspector: bool,
    inspector: InspectorTab,
    follow: bool,
    scroll_to_cursor: bool,
    music_font: bool,
    screenshot: Option<PathBuf>,
    frame: u64,
    shot_requested: bool,
    pending: Option<Pending>,
    show_help: bool,
    show_about: bool,
    title_cache: String,
    show_home: bool,
    single_track: bool,
    toast: (String, f64),
    recent: Vec<PathBuf>,
    home_tab: usize,
    home_sel: usize,
    home_search: String,
    preview: Option<(String, Song)>,
    show_keyboard: bool,
    kb_octave: i32,
    play_while_editing: bool,
    /// 0 page vertical, 1 page horizontal, 2 page grid, 3 screen vertical, 4 screen horizontal.
    view_mode: u8,
    dir_open: bool,
    addtrack_open: bool,
    at_name: String,
    at_inst: Instrument,
    at_tuning: Vec<u8>,
    at_grand: bool,
    /// Where to suggest saving a Guitar Pro file opened in another format.
    save_hint: Option<PathBuf>,
    /// The song tab shows every information field (More) or only title and artist (Less).
    song_more: bool,
    /// The icon list beside the track name is open (for checking pictures).
    icon_menu_open: bool,
    tuning_dlg: Option<tuning::TuningDlg>,
    at_color: usize,
    chord_open: bool,
    chord_text: String,
    chord_root: usize,
    chord_type: usize,
    scale_open: bool,
    scale_root: u8,
    scale_kind: usize,
    scale_shown: bool,
    scale_found: Vec<(u8, usize, u32)>,
    lyrics_open: bool,
    lyrics_snap: bool,
    cmd_open: bool,
    cmd_text: String,
    cmd_sel: usize,
    trainer_open: bool,
    trainer_on: bool,
    trainer_start: f32,
    trainer_end: f32,
    trainer_step: f32,
    guide_open: bool,
    last_copy_all: bool,
    keep_pitch: bool,
    transpose_sel_only: bool,
    transpose_key: bool,
    transpose_diatonic: bool,
    last_play_pos: u64,
    metro_flash: bool,
    /// Staff position of the cursor when it is in the notation staff (None = in the tab).
    std_cursor: Option<i32>,
    /// The voice being edited (0 = voice 1).
    voice: usize,
    voices_open: bool,
    auto_open: bool,
    auto_sel: usize,
    auto_snap: bool,
    auto_drag: bool,
    vaction: voices::VoiceAction,
    vfrom: usize,
    vto: usize,
    vrange: voices::VoiceRange,
    popup: Option<popups::Popup>,
    pop_bar: MasterBar,
    pop_beat: Beat,
    pop_note: Note,
    pop_text: String,
    pop_until_next: bool,
    goto_open: bool,
    goto_text: String,
    transpose_open: bool,
    transpose_amt: i32,
    transpose_all: bool,
    bar_clipboard: Vec<(MasterBar, Vec<Bar>)>,
    fullscreen: bool,
    fullscreen_changed: bool,
    focus_request: Option<String>,
    /// The Stylesheet window, while open.
    style_dlg: Option<stylesheet::StyleDlg>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, screenshot: Option<PathBuf>, open: Option<PathBuf>) -> App {
        let music_font = crate::setup_fonts(&cc.egui_ctx);
        crate::setup_style(&cc.egui_ctx);
        let mut app = App {
            song: Song::demo(),
            path: None,
            dirty: false,
            track: 0,
            bar: 0,
            beat: 0,
            string: 0,
            sel_anchor: None,
            undo: vec![],
            redo: vec![],
            clipboard: vec![],
            cur_dur: Dur::Eighth,
            zoom: 1.1,
            notation: Notation::Both,
            audio: None,
            sound_allowed: screenshot.is_none(),
            playing: false,
            timeline: vec![],
            start_offset: 0,
            play_start_bar: 0,
            play_beat: None,
            play_tick: None,
            play_secs: 0.0,
            total_secs: 0.0,
            metronome: false,
            count_in: false,
            looping: false,
            speed: 100.0,
            master_volume: 0.9,
            status: "Ready. Click the score or use the arrow keys, then type fret numbers.".into(),
            last_digit: None,
            show_fretboard: false,
            show_tracks: true,
            show_palette: true,
            show_inspector: true,
            inspector: InspectorTab::Song,
            follow: true,
            scroll_to_cursor: false,
            music_font,
            screenshot,
            frame: 0,
            shot_requested: false,
            pending: None,
            show_help: false,
            show_about: false,
            title_cache: String::new(),
            show_home: false,
            single_track: false,
            toast: (String::new(), 0.0),
            recent: vec![],
            home_tab: 0,
            home_sel: 0,
            home_search: String::new(),
            preview: None,
            show_keyboard: false,
                    kb_octave: 1,
            play_while_editing: true,
            view_mode: 0,
            dir_open: false,
            addtrack_open: false,
            at_name: String::new(),
            at_inst: Instrument::SteelGuitar,
            at_tuning: vec![64, 59, 55, 50, 45, 40],
            at_grand: true,
            save_hint: None,
            song_more: false,
            icon_menu_open: false,
            tuning_dlg: None,
            at_color: 2,
            chord_open: false,
            chord_text: String::new(),
            chord_root: 0,
            chord_type: 0,
            scale_open: false,
            scale_root: 0,
            scale_kind: 0,
            scale_shown: false,
            scale_found: vec![],
            lyrics_open: false,
            lyrics_snap: false,
            cmd_open: false,
            cmd_text: String::new(),
            cmd_sel: 0,
            trainer_open: false,
            trainer_on: false,
            trainer_start: 60.0,
            trainer_end: 100.0,
            trainer_step: 5.0,
            guide_open: false,
            last_copy_all: false,
            keep_pitch: true,
            transpose_sel_only: false,
            transpose_key: false,
            transpose_diatonic: false,
            last_play_pos: 0,
            metro_flash: false,
            std_cursor: None,
            voice: 0,
            voices_open: false,
            auto_open: false,
            auto_sel: 0,
            auto_snap: false,
            auto_drag: false,
            vaction: voices::VoiceAction::Move,
            vfrom: 0,
            vto: 1,
            vrange: voices::VoiceRange::Bar,
            popup: None,
            pop_bar: MasterBar::default(),
            pop_beat: Beat::default(),
            pop_note: Note::default(),
            pop_text: String::new(),
            pop_until_next: true,
            goto_open: false,
            goto_text: String::new(),
            transpose_open: false,
            transpose_amt: 2,
            transpose_all: true,
            bar_clipboard: vec![],
            fullscreen: false,
            fullscreen_changed: false,
            focus_request: None,
            style_dlg: None,
        };
        if app.screenshot.is_none() {
            app.recent = home::load_recent();
        }
        if app.recent.is_empty() {
            app.home_tab = 2;
        }
        if let Some(p) = open {
            app.open_path(p);
        }
        if let Some(b) = std::env::var("GUITARCRAFT_BAR").ok().and_then(|t| t.parse::<usize>().ok()) {
            app.bar = b.saturating_sub(1);
            app.clamp_cursor();
            app.scroll_to_cursor = true;
        }
        // Extra views for checking pictures: --show keyboard,fretboard,track,screen
        if let Ok(v) = std::env::var("GUITARCRAFT_SHOW") {
            for part in v.split(',') {
                match part.trim() {
                    "keyboard" => app.show_keyboard = true,
                    "fretboard" => app.show_fretboard = true,
                    "track" => app.inspector = InspectorTab::Track,
                    "bar" => app.open_popup(popups::Popup::TimeSig),
                    "key" => app.open_popup(popups::Popup::KeySig),
                    "single" => app.single_track = true,
                    "more" => app.song_more = true,
                    "playing" => {
                        // A picture of playback part way through bar 1 (no sound).
                        app.playing = true;
                        app.play_beat = Some((0, 3));
                        app.play_tick = Some((0, 1.6 * crate::model::QUARTER as f32));
                    }
                    "short" => {
                        // Bar 1 one beat short and bar 2 half a beat long, for checking.
                        let t = &mut app.song.tracks[0];
                        t.bars[0].beats.pop();
                        t.bars[0].beats.pop();
                        let extra = t.bars[1].beats[0].clone();
                        t.bars[1].beats.push(extra);
                    }
                    "icons" => {
                        app.inspector = InspectorTab::Track;
                        app.icon_menu_open = true;
                    }
                    "tuning" => {
                        app.inspector = InspectorTab::Track;
                        app.open_tuning();
                    }
                    "partial" => {
                        app.inspector = InspectorTab::Track;
                        app.open_tuning();
                        if let Some(d) = app.tuning_dlg.as_mut() {
                            d.capo = 2;
                            d.partial = 4;
                            d.partial_strings = 0b001110;
                        }
                    }
                    "addtrack2" => {
                        app.addtrack_open = true;
                        app.at_inst = Instrument::Piano;
                        app.at_name = "Grand Piano".into();
                    }
                    "band" => {
                        // A band with vocals, piano, guitar, bass and drums, for checking.
                        app.song = home::band_song();
                        app.track = 0;
                        app.bar = 0;
                        app.beat = 0;
                        app.total_secs = app.song_seconds();
                    }
                    "ring" => {
                        // Select bars 3-4 and turn let ring on, for checking.
                        app.bar = 2;
                        app.beat = 0;
                        app.sel_anchor = Some((2, 0));
                        app.bar = 3;
                        app.beat = app.song.tracks[0].bars[3].beats.len() - 1;
                        app.toggle_note_flag("Let ring", |n| &mut n.let_ring);
                    }
                    "beat" => app.open_popup(popups::Popup::Text),
                    "section" => app.open_popup(popups::Popup::Section),
                    "bend" => app.open_popup(popups::Popup::Bend),
                    "screen" => app.view_mode = 3,
                    "grid" => app.view_mode = 2,
                    "guide" => app.guide_open = true,
                    "palette-cmd" => app.cmd_open = true,
                    "directions" => app.dir_open = true,
                    "chord" => app.chord_open = true,
                    "scales" => app.scale_open = true,
                    "lyrics" => app.lyrics_open = true,
                    "addtrack" => app.addtrack_open = true,
                    "std" => {
                        // Cursor on the lowest note of the beat, in the notation.
                        if let Some(n) = app.cur_beat().notes.iter().max_by_key(|n| n.string).cloned() {
                            app.string = n.string as usize;
                        }
                        app.enter_std();
                    }
                    "tab1" => app.string = 1,
                    "voicetool" => app.voices_open = true,
                    "tempo" => {
                        app.song.master[4].tempo = Some(112);
                        app.song.master[8].tempo_ramp = false;
                        app.song.master[4].tempo_ramp = true;
                        app.song.master[8].tempo = Some(84);
                        app.open_automations(4);
                    }
                    "move2" => {
                        // Move the cursor's note (bar 1, low string) to voice 2, for checking.
                        app.string = 5;
                        app.move_to_voice(1);
                    }
                    "voice2" => {
                        // A second voice in bar 1 (bass in half notes) for checking pictures.
                        let b = &mut app.song.tracks[0].bars[0];
                        b.pad_voices();
                        b.voices[0] = vec![
                            Beat { notes: vec![Note::new(5, 3)], ..Beat::rest(Dur::Half) },
                            Beat { notes: vec![Note::new(4, 2)], ..Beat::rest(Dur::Half) },
                        ];
                        app.voice = 1;
                        app.string = 5;
                    }
                    "help" => app.show_help = true,
                    // The Stylesheet window on one of its tabs, for checking pictures.
                    "stylesheet" | "stylesheet1" | "stylesheet2" | "stylesheet3" | "stylesheet4" | "stylesheet5" => {
                        app.open_stylesheet();
                        if let Some(d) = app.style_dlg.as_mut() {
                            d.tab = part.trim().trim_start_matches("stylesheet").parse::<usize>().unwrap_or(1).clamp(1, 5) - 1;
                        }
                    }
                    "style-rock" => app.song.style = crate::style::Stylesheet::rock(),
                    "style-classic" => app.song.style = crate::style::Stylesheet::classic(),
                    "style-jazz" => app.song.style = crate::style::Stylesheet::jazz(),
                    _ => {}
                }
            }
        }
        if std::env::var("GUITARCRAFT_HOME").is_ok() {
            app.show_home = true;
        }
        if let Some(z) = std::env::var("GUITARCRAFT_ZOOM").ok().and_then(|z| z.parse::<f32>().ok()) {
            app.zoom = z.clamp(0.3, 3.0);
        }
        if let Some(t) = std::env::var("GUITARCRAFT_TRACK").ok().and_then(|t| t.parse::<usize>().ok()) {
            app.track = t.saturating_sub(1);
            app.clamp_cursor();
        }
        app.total_secs = app.song_seconds();
        app
    }

    // ----- helpers -----

    fn tr(&self) -> &Track {
        &self.song.tracks[self.track]
    }
    fn cur_beat(&self) -> &Beat {
        &self.song.tracks[self.track].bars[self.bar].voice(self.voice)[self.beat]
    }
    fn clamp_cursor(&mut self) {
        self.track = self.track.min(self.song.tracks.len() - 1);
        self.bar = self.bar.min(self.song.bar_count() - 1);
        let nb = self.song.tracks[self.track].bars[self.bar].voice(self.voice).len();
        self.beat = self.beat.min(nb.saturating_sub(1));
        self.string = self.string.min(self.tr().strings() - 1);
    }
    fn selection(&self) -> Option<((usize, usize), (usize, usize))> {
        let a = self.sel_anchor?;
        let c = (self.bar, self.beat);
        if a == c {
            return None;
        }
        Some((a.min(c), a.max(c)))
    }

    fn snapshot(&mut self) {
        self.undo.push(self.song.clone());
        if self.undo.len() > 200 {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.dirty = true;
    }

    fn edit(&mut self, f: impl FnOnce(&mut Song, usize, usize, usize, usize)) {
        self.snapshot();
        let (t, b, j, s) = (self.track, self.bar, self.beat, self.string);
        // Editing voice 2-4: put that voice where voice 1 normally is for the change.
        self.song.swap_in(t, self.voice);
        f(&mut self.song, t, b, j, s);
        self.song.swap_out_all();
        self.song.normalize();
        self.clamp_cursor();
        self.total_secs = self.song_seconds();
    }

    fn do_undo(&mut self) {
        if let Some(s) = self.undo.pop() {
            self.redo.push(std::mem::replace(&mut self.song, s));
            self.song.normalize();
            self.clamp_cursor();
            self.dirty = true;
            self.status = "Undone.".into();
        }
    }
    fn do_redo(&mut self) {
        if let Some(s) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.song, s));
            self.song.normalize();
            self.clamp_cursor();
            self.dirty = true;
            self.status = "Redone.".into();
        }
    }

    fn song_seconds(&self) -> f32 {
        let mut t = 0.0;
        for bi in self.song.play_order() {
            t += self.song.master[bi].ticks() as f32 / QUARTER as f32 * 60.0 / self.song.tempo_at(bi) as f32;
        }
        t / (self.speed / 100.0)
    }

    fn preview(&mut self, n: &Note) {
        if self.playing || !self.sound_allowed || !self.play_while_editing {
            return;
        }
        self.ensure_audio();
        if let Some(a) = &self.audio {
            if a.error.is_none() {
                let sr = a.sample_rate();
                let ev = audio::note_preview(&self.song.tracks[self.track], self.track, n, sr);
                if let Ok(mut st) = a.state.lock() {
                    st.preview.push(ev);
                }
            }
        }
    }

    fn ensure_audio(&mut self) {
        if self.audio.is_none() {
            self.audio = Some(if self.sound_allowed { Audio::new() } else { Audio::silent() });
            if let Some(e) = self.audio.as_ref().and_then(|a| a.error.clone()) {
                self.status = format!("Sound unavailable: {e}");
            }
        }
    }

    // ----- cursor movement -----

    fn bar_full(&self, bar: usize) -> bool {
        let b = &self.song.tracks[self.track].bars[bar];
        let v = b.voice(self.voice);
        v.iter().map(|x| x.ticks()).sum::<u32>() >= self.song.master[bar].ticks() || (v.len() == 1 && v[0].is_rest() && v[0].dur == Dur::Whole)
    }

    fn move_right(&mut self, extend: bool) {
        self.update_anchor(extend);
        let nb = self.song.tracks[self.track].bars[self.bar].voice(self.voice).len();
        if self.beat + 1 < nb {
            self.beat += 1;
        } else if !extend && !self.bar_full(self.bar) {
            let d = self.cur_beat().dur;
            let dotted = self.cur_beat().dotted;
            let tup = self.cur_beat().tuplet;
            self.edit(|s, t, b, _, _| {
                let mut nbt = Beat::rest(d);
                nbt.dotted = dotted;
                nbt.tuplet = tup;
                s.tracks[t].bars[b].beats.push(nbt)
            });
            self.beat = nb;
        } else if self.bar + 1 < self.song.bar_count() {
            self.bar += 1;
            self.beat = 0;
        } else if !extend {
            self.edit(|s, _, _, _, _| {
                let mut mb = s.master.last().cloned().unwrap_or_default();
                mb.marker = None;
                mb.repeat_open = false;
                mb.repeat_close = 0;
                mb.tempo = None;
                s.master.push(mb);
            });
            self.bar += 1;
            self.beat = 0;
            self.status = "Added a new bar at the end.".into();
        }
        self.scroll_to_cursor = true;
    }

    fn move_left(&mut self, extend: bool) {
        self.update_anchor(extend);
        if self.beat > 0 {
            self.beat -= 1;
        } else if self.bar > 0 {
            self.bar -= 1;
            self.beat = self.song.tracks[self.track].bars[self.bar].voice(self.voice).len() - 1;
        }
        self.scroll_to_cursor = true;
    }

    fn update_anchor(&mut self, extend: bool) {
        if extend {
            if self.sel_anchor.is_none() {
                self.sel_anchor = Some((self.bar, self.beat));
            }
        } else {
            self.sel_anchor = None;
        }
    }

    /// Chooses the voice to edit (0 = voice 1).
    fn set_voice(&mut self, v: usize) {
        self.voice = v.min(VOICES - 1);
        self.sel_anchor = None;
        self.clamp_cursor();
        self.status = format!("Editing voice {}.", self.voice + 1);
    }

    fn goto_bar(&mut self, bar: usize) {
        self.bar = bar.min(self.song.bar_count() - 1);
        self.beat = 0;
        self.sel_anchor = None;
        self.scroll_to_cursor = true;
    }

    // ----- editing -----

    fn set_fret(&mut self, string: usize, fret: u8) {
        let cur_dur = self.cur_dur;
        self.edit(|s, t, b, j, _| {
            let bar = &mut s.tracks[t].bars[b];
            let placeholder = bar.beats.len() == 1 && bar.beats[0].is_rest() && bar.beats[0].dur == Dur::Whole;
            let beat = &mut bar.beats[j];
            if placeholder {
                beat.dur = cur_dur;
            }
            if let Some(n) = beat.note_on_mut(string as u8) {
                n.fret = fret;
                n.tie = false;
                n.dead = false;
            } else {
                beat.notes.push(Note::new(string as u8, fret));
                beat.notes.sort_by_key(|n| n.string);
            }
        });
        let n = Note::new(string as u8, fret);
        if self.std_cursor.is_some() {
            if let Some(placed) = self.cur_beat().note_on(string as u8).cloned() {
                self.std_cursor = Some(self.staff_pos_of(&placed));
            }
        }
        self.preview(&n);
        self.status = format!("String {}, fret {} ({})", string + 1, fret, note_name_oct(self.tr().pitch(&n)));
    }

    fn type_digit(&mut self, d: u8, now: f64) {
        let pos = (self.bar, self.beat, self.string);
        let max = self.tr().frets;
        let mut fret = d;
        if let Some((prev, t, b, j, s)) = self.last_digit {
            if (b, j, s) == pos && now - t < 0.8 {
                let combined = prev as u32 * 10 + d as u32;
                if combined <= max as u32 {
                    fret = combined as u8;
                    // Replace the one-digit entry instead of adding an undo step.
                    self.undo.pop();
                }
            }
        }
        self.set_fret(self.string, fret);
        self.last_digit = Some((fret, now, pos.0, pos.1, pos.2));
    }

    fn delete_at_cursor(&mut self) {
        if let Some((a, b)) = self.selection() {
            self.edit(|s, t, _, _, _| {
                for bi in (a.0..=b.0).rev() {
                    let beats = &mut s.tracks[t].bars[bi].beats;
                    let from = if bi == a.0 { a.1 } else { 0 };
                    let to = if bi == b.0 { b.1 } else { beats.len() - 1 };
                    for j in (from..=to.min(beats.len() - 1)).rev() {
                        beats.remove(j);
                    }
                    if beats.is_empty() {
                        beats.push(Beat::rest(Dur::Whole));
                    }
                }
            });
            self.bar = a.0;
            self.beat = a.1;
            self.sel_anchor = None;
            self.clamp_cursor();
            return;
        }
        let has_note = self.cur_beat().note_on(self.string as u8).is_some();
        if has_note {
            self.edit(|s, t, b, j, st| s.tracks[t].bars[b].beats[j].notes.retain(|n| n.string != st as u8));
        } else if self.cur_beat().is_rest() && self.song.tracks[self.track].bars[self.bar].voice(self.voice).len() > 1 {
            self.edit(|s, t, b, j, _| {
                s.tracks[t].bars[b].beats.remove(j);
            });
        } else if !self.cur_beat().is_rest() {
            self.edit(|s, t, b, j, _| s.tracks[t].bars[b].beats[j].notes.clear());
        }
    }

    fn insert_beat(&mut self) {
        let d = self.cur_dur;
        self.edit(|s, t, b, j, _| s.tracks[t].bars[b].beats.insert(j, Beat::rest(d)));
    }

    fn set_duration(&mut self, d: Dur) {
        self.cur_dur = d;
        self.edit(|s, t, b, j, _| s.tracks[t].bars[b].beats[j].dur = d);
        self.status = format!("{} selected.", d.name());
    }

    fn toggle_note_flag(&mut self, name: &str, f: fn(&mut Note) -> &mut bool) {
        // With a selection, the effect goes on (or off) for every selected note.
        if let Some((sa, sb)) = self.selection() {
            let mut all_on = true;
            let mut any = false;
            for bi in sa.0..=sb.0 {
                let beats = self.song.tracks[self.track].bars[bi].voice(self.voice);
                let from = if bi == sa.0 { sa.1 } else { 0 };
                let to = if bi == sb.0 { sb.1 } else { beats.len().saturating_sub(1) };
                for beat in beats.iter().take(to + 1).skip(from) {
                    for n in &beat.notes {
                        any = true;
                        let mut c = n.clone();
                        all_on &= *f(&mut c);
                    }
                }
            }
            if !any {
                self.status = "The selection has no notes.".into();
                return;
            }
            let on = !all_on;
            self.edit(|s, t, _, _, _| {
                for bi in sa.0..=sb.0 {
                    let beats = &mut s.tracks[t].bars[bi].beats;
                    let from = if bi == sa.0 { sa.1 } else { 0 };
                    let to = if bi == sb.0 { sb.1 } else { beats.len().saturating_sub(1) };
                    for beat in beats.iter_mut().take(to + 1).skip(from) {
                        for n in &mut beat.notes {
                            *f(n) = on;
                        }
                    }
                }
            });
            self.status = format!("{name} {} for the selected notes.", if on { "on" } else { "off" });
            return;
        }
        if self.cur_beat().is_rest() {
            self.status = "Put a note on this beat first.".into();
            return;
        }
        let on_string = self.cur_beat().note_on(self.string as u8).is_some() && (self.std_cursor.is_none() || self.note_at_std().is_some());
        self.edit(|s, t, b, j, st| {
            let beat = &mut s.tracks[t].bars[b].beats[j];
            if on_string {
                if let Some(n) = beat.note_on_mut(st as u8) {
                    let v = f(n);
                    *v = !*v;
                }
            } else {
                let newv = !beat.notes.iter_mut().all(|n| *f(n));
                for n in &mut beat.notes {
                    *f(n) = newv;
                }
            }
        });
        let on = self
            .cur_beat()
            .note_on(self.string as u8)
            .or(self.cur_beat().notes.first())
            .map(|n| {
                let mut c = n.clone();
                *f(&mut c)
            })
            .unwrap_or(false);
        self.status = format!("{name}: {}.", if on { "on" } else { "off" });
    }

    fn cycle_bend(&mut self) {
        if self.cur_beat().is_rest() {
            self.status = "Put a note on this beat first.".into();
            return;
        }
        self.edit(|s, t, b, j, st| {
            let beat = &mut s.tracks[t].bars[b].beats[j];
            let idx = beat.notes.iter().position(|n| n.string == st as u8).unwrap_or(0);
            let n = &mut beat.notes[idx];
            n.bend = match n.bend {
                0 => 4,
                4 => 2,
                2 => 6,
                6 => 8,
                _ => 0,
            };
        });
    }

    fn insert_bar(&mut self, at: usize) {
        self.edit(|s, _, _, _, _| {
            let mut mb = s.master[at.min(s.master.len() - 1)].clone();
            mb.marker = None;
            mb.repeat_open = false;
            mb.repeat_close = 0;
            mb.tempo = None;
            s.master.insert(at, mb);
            for t in &mut s.tracks {
                t.bars.insert(at, Bar::empty());
            }
        });
        self.goto_bar(at);
        self.status = format!("Inserted bar {}.", at + 1);
    }

    fn delete_bar(&mut self, at: usize) {
        if self.song.bar_count() <= 1 {
            return;
        }
        self.edit(|s, _, _, _, _| {
            s.master.remove(at);
            for t in &mut s.tracks {
                t.bars.remove(at);
            }
        });
        self.goto_bar(at.min(self.song.bar_count() - 1));
        self.status = format!("Deleted bar {}.", at + 1);
    }

    fn copy(&mut self) {
        let mut out = vec![];
        if let Some((a, b)) = self.selection() {
            for bi in a.0..=b.0 {
                let beats = &self.song.tracks[self.track].bars[bi].voice(self.voice);
                let from = if bi == a.0 { a.1 } else { 0 };
                let to = if bi == b.0 { b.1 } else { beats.len() - 1 };
                out.extend_from_slice(&beats[from..=to.min(beats.len() - 1)]);
            }
        } else {
            out.push(self.cur_beat().clone());
        }
        let n = out.len();
        self.clipboard = out;
        self.status = format!("Copied {n} beat(s).");
    }

    fn paste(&mut self) {
        if self.clipboard.is_empty() {
            return;
        }
        let clip = self.clipboard.clone();
        let n = clip.len();
        self.edit(|s, t, b, j, _| {
            let bar = &mut s.tracks[t].bars[b];
            let placeholder = bar.beats.len() == 1 && bar.beats[0].is_rest() && bar.beats[0].dur == Dur::Whole;
            if placeholder {
                bar.beats.clear();
                bar.beats.extend(clip);
            } else {
                for (k, bt) in clip.into_iter().enumerate() {
                    bar.beats.insert(j + k, bt);
                }
            }
        });
        self.status = format!("Pasted {n} beat(s).");
    }

    fn transpose(&mut self, delta: i32) {
        let sel = self.selection();
        let (a, b) = sel.unwrap_or(((self.bar, self.beat), (self.bar, self.beat)));
        let maxf = self.tr().frets as i32;
        self.edit(|s, t, _, _, _| {
            for bi in a.0..=b.0 {
                let beats = &mut s.tracks[t].bars[bi].beats;
                let from = if bi == a.0 { a.1 } else { 0 };
                let to = if bi == b.0 { b.1 } else { beats.len() - 1 };
                let to = to.min(beats.len() - 1);
                for beat in &mut beats[from..=to] {
                    for n in &mut beat.notes {
                        n.fret = (n.fret as i32 + delta).clamp(0, maxf) as u8;
                    }
                }
            }
        });
        self.status = format!("Moved notes {} a half step.", if delta > 0 { "up" } else { "down" });
    }

    fn add_track(&mut self, bass: bool) {
        self.edit(|s, _, _, _, _| {
            let mut t = Track::default();
            if bass {
                t.name = "Electric Bass".into();
                t.instrument = Instrument::Bass;
                t.tuning = vec![43, 38, 33, 28];
            } else {
                t.name = format!("Guitar {}", s.tracks.len() + 1);
            }
            t.color = TRACK_COLORS[s.tracks.len() % TRACK_COLORS.len()];
            s.tracks.push(t);
        });
        self.track = self.song.tracks.len() - 1;
        self.clamp_cursor();
        self.inspector = InspectorTab::Track;
    }

    fn remove_track(&mut self) {
        if self.song.tracks.len() <= 1 {
            self.status = "A song needs at least one track.".into();
            return;
        }
        let ti = self.track;
        self.edit(|s, _, _, _, _| {
            s.tracks.remove(ti);
        });
        self.track = self.track.min(self.song.tracks.len() - 1);
        self.clamp_cursor();
    }

    // ----- files -----

    fn new_song(&mut self) {
        self.stop();
        self.song = Song::new_blank();
        self.song.style = stylesheet::default_style();
        self.path = None;
        self.reset_after_load();
        self.show_home = false;
        self.status = "New song. Click a string and type fret numbers.".into();
    }

    fn reset_after_load(&mut self) {
        self.dirty = false;
        self.undo.clear();
        self.redo.clear();
        // Start on the first track the score shows (hidden tracks stay out of view).
        self.track = self.song.tracks.iter().position(|t| t.visible).unwrap_or(0);
        self.bar = 0;
        self.beat = 0;
        self.string = 0;
        self.sel_anchor = None;
        self.total_secs = self.song_seconds();
        self.inspector = InspectorTab::Song;
    }

    fn open_dialog(&mut self) {
        if self.screenshot.is_some() {
            return;
        }
        let f = rfd::FileDialog::new()
            .add_filter("Guitar Pro files", &["gp", "gpx", "gp3", "gp4", "gp5"])
            .add_filter("Older GuitarCraft files", &fileio::OLD_EXTS)
            .pick_file();
        if let Some(p) = f {
            self.open_path(p);
        }
    }

    fn open_path(&mut self, p: PathBuf) {
        match fileio::load_song(&p) {
            Ok(song) => {
                self.stop();
                self.song = song;
                // Files saved straight back: Guitar Pro 5 and this app's own. Other
                // Guitar Pro files are saved next to the original as .gp5.
                let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
                let native = [fileio::GP_EXT, fileio::GP5_EXT].contains(&ext.as_str());
                self.path = if native { Some(p.clone()) } else { None };
                self.save_hint = if native { None } else { Some(p.with_extension(fileio::GP_EXT)) };
                self.reset_after_load();
                self.add_recent(&p);
                self.show_home = false;
                self.status = format!("Opened {}.", p.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default());
            }
            Err(e) => self.status = format!("Couldn't open the file: {e}"),
        }
    }

    fn save(&mut self, save_as: bool) {
        if self.screenshot.is_some() {
            return;
        }
        let path = if save_as || self.path.is_none() {
            let mut d = rfd::FileDialog::new()
                .add_filter("Guitar Pro file", &[fileio::GP_EXT])
                .add_filter("Guitar Pro 5 file (older)", &[fileio::GP5_EXT]);
            match (&self.save_hint, &self.path) {
                (_, Some(p)) | (Some(p), None) => {
                    if let Some(dir) = p.parent() {
                        d = d.set_directory(dir);
                    }
                    let stem = p.file_stem().map(|x| x.to_string_lossy().to_string()).unwrap_or_else(|| sanitize(&self.song.title));
                    d = d.set_file_name(format!("{stem}.{}", fileio::GP_EXT));
                }
                _ => d = d.set_file_name(format!("{}.{}", sanitize(&self.song.title), fileio::GP_EXT)),
            }
            d.save_file()
        } else {
            self.path.clone()
        };
        if let Some(p) = path {
            match fileio::save_song(&self.song, &p) {
                Ok(()) => {
                    self.dirty = false;
                    let gp = p.extension().map(|e| e.eq_ignore_ascii_case(fileio::GP5_EXT)).unwrap_or(false);
                    let lost = if gp { fileio::gp5_losses(&self.song) } else { vec![] };
                    self.status = if lost.is_empty() {
                        format!("Saved {}.", p.display())
                    } else {
                        format!("Saved {} (Guitar Pro 5 can't keep: {}).", p.display(), lost.join(", "))
                    };
                    self.save_hint = None;
                    self.add_recent(&p);
                    self.path = Some(p);
                }
                Err(e) => self.status = format!("Couldn't save: {e}"),
            }
        }
    }

    fn export_midi(&mut self) {
        if let Some(p) = rfd::FileDialog::new().add_filter("MIDI file", &["mid"]).set_file_name(format!("{}.mid", sanitize(&self.song.title))).save_file() {
            match std::fs::write(&p, fileio::export_midi(&self.song)) {
                Ok(()) => self.status = format!("Exported {}.", p.display()),
                Err(e) => self.status = format!("Couldn't export: {e}"),
            }
        }
    }

    fn export_ascii(&mut self) {
        if let Some(p) = rfd::FileDialog::new().add_filter("Text file", &["txt"]).set_file_name(format!("{}.txt", sanitize(&self.song.title))).save_file() {
            match std::fs::write(&p, fileio::export_ascii(&self.song, self.track)) {
                Ok(()) => self.status = format!("Exported {}.", p.display()),
                Err(e) => self.status = format!("Couldn't export: {e}"),
            }
        }
    }

    fn add_recent(&mut self, p: &std::path::Path) {
        if self.screenshot.is_some() {
            return;
        }
        let p = p.to_path_buf();
        self.recent.retain(|x| x != &p);
        self.recent.insert(0, p);
        self.recent.truncate(20);
        home::save_recent(&self.recent);
    }

    /// Tracks shown in the score.
    fn shown_tracks(&self) -> Vec<usize> {
        if self.single_track {
            return vec![self.track];
        }
        (0..self.song.tracks.len()).filter(|&i| self.song.tracks[i].visible || i == self.track).collect()
    }

    fn request(&mut self, what: Pending) {
        if self.dirty {
            self.pending = Some(what);
        } else {
            self.run_pending(what);
        }
    }

    fn run_pending(&mut self, what: Pending) {
        match what {
            Pending::New => self.new_song(),
            Pending::Open => self.open_dialog(),
            Pending::Demo => {
                self.stop();
                self.song = Song::demo();
                self.path = None;
                self.reset_after_load();
                self.show_home = false;
            }
            Pending::Template(k) => {
                self.stop();
                self.song = home::template(k);
                self.path = None;
                self.reset_after_load();
                self.show_home = false;
            }
            Pending::Close => self.close_song(),
            Pending::Quit => {
                self.dirty = false;
                self.pending = None;
            }
        }
    }

    // ----- playback -----

    fn toggle_play(&mut self) {
        if self.playing {
            self.pause();
        } else {
            if self.trainer_on {
                self.speed = self.trainer_start;
                self.looping = true;
            }
            self.play();
        }
    }

    fn play(&mut self) {
        self.ensure_audio();
        let Some(a) = &self.audio else { return };
        if let Some(e) = &a.error {
            self.status = format!("Sound unavailable: {e}");
            return;
        }
        let sr = a.sample_rate();
        let sel = if self.looping { self.selection() } else { None };
        // Looping without a selection repeats the whole song, starting from the cursor's bar.
        let whole_loop = self.looping && sel.is_none();
        let opt = ScheduleOptions {
            from_bar: if whole_loop { 0 } else { self.bar },
            metronome: self.metronome,
            count_in: self.count_in && !(whole_loop && self.bar > 0),
            speed: self.speed / 100.0,
            only_bars: sel.map(|(x, y)| (x.0, y.0)),
        };
        let mut sch = audio::build_schedule(&self.song, sr, &opt);
        let at_tick = |t: &audio::TimelineBar, tick: u32| t.start + (tick as f64 * t.samples_per_tick) as u64;
        let song_end = sch.timeline.last().map(|t| at_tick(t, t.ticks)).unwrap_or(sch.end);
        let mut start_pos = 0u64;
        let loop_range = match (sel, self.looping) {
            (Some((a0, b0)), true) => {
                // Exactly the highlighted beats: from the first one's start to the last one's end.
                let bars = &self.song.tracks[self.track].bars;
                let start_tick = bars[a0.0].beat_start(self.voice, a0.1);
                let end_tick = bars[b0.0].beat_start(self.voice, b0.1) + bars[b0.0].voice(self.voice).get(b0.1).map(|x| x.ticks()).unwrap_or(0);
                let first = sch.timeline.first().cloned();
                let last = sch.timeline.last().cloned();
                match (first, last) {
                    (Some(f), Some(l)) => {
                        let ls = at_tick(&f, start_tick);
                        let le = at_tick(&l, end_tick.min(l.ticks)).max(ls + 1);
                        // Notes of the first bar before the selection aren't played.
                        let off = sch.start_offset;
                        sch.events.retain(|e| e.start >= ls || e.start < off || e.track == usize::MAX);
                        if off == 0 {
                            start_pos = ls;
                        }
                        Some((ls, le))
                    }
                    _ => None,
                }
            }
            (None, true) => {
                let from = sch.timeline.iter().find(|t| t.bar == self.bar).map(|t| t.start).unwrap_or(sch.start_offset);
                if from > sch.start_offset {
                    start_pos = from;
                }
                Some((sch.start_offset, song_end))
            }
            _ => None,
        };
        if let Ok(mut st) = a.state.lock() {
            st.stop_all();
            st.next_ev = sch.events.partition_point(|e| e.start < start_pos);
            st.events = sch.events;
            st.pos = start_pos;
            st.end = sch.end;
            st.master = self.master_volume;
            st.loop_range = loop_range;
            st.playing = true;
        }
        self.timeline = sch.timeline;
        self.start_offset = sch.start_offset;
        self.play_start_bar = self.bar;
        self.playing = true;
        self.last_play_pos = 0;
        self.status = "Playing…".into();
    }

    fn pause(&mut self) {
        if let Some(a) = &self.audio {
            if let Ok(mut st) = a.state.lock() {
                st.stop_all();
            }
        }
        if let (Some((b, j)), None) = (self.play_beat, self.selection()) {
            self.bar = b;
            self.beat = j;
            self.clamp_cursor();
        }
        self.playing = false;
        self.play_beat = None;
        self.play_tick = None;
        self.status = "Paused.".into();
    }

    fn stop(&mut self) {
        if let Some(a) = &self.audio {
            if let Ok(mut st) = a.state.lock() {
                st.stop_all();
            }
        }
        if self.playing {
            self.bar = self.play_start_bar.min(self.song.bar_count() - 1);
            self.beat = 0;
        }
        self.playing = false;
        self.play_beat = None;
        self.play_tick = None;
        self.play_secs = 0.0;
    }

    fn poll_playback(&mut self) -> Vec<(usize, u8, u8)> {
        let mut sounding = vec![];
        if !self.playing {
            if let Some(a) = &self.audio {
                if let Ok(st) = a.state.lock() {
                    sounding = st.sounding.clone();
                }
            }
            return sounding;
        }
        let Some(a) = &self.audio else { return sounding };
        let (pos, still, sr) = match a.state.lock() {
            Ok(st) => {
                sounding = st.sounding.clone();
                (st.pos, st.playing, st.sample_rate)
            }
            Err(_) => return sounding,
        };
        if !still {
            self.playing = false;
            self.play_beat = None;
            self.play_tick = None;
            self.status = "Finished.".into();
            return sounding;
        }
        // The loop came round again: the speed trainer raises the speed.
        if pos < self.last_play_pos && self.looping && self.trainer_on && self.speed < self.trainer_end {
            self.speed = (self.speed + self.trainer_step).min(self.trainer_end);
            self.last_play_pos = 0;
            let keep = (self.bar, self.beat);
            if let Some((b, _)) = self.timeline.first().map(|t| (t.bar, 0)) {
                self.bar = b;
                self.beat = 0;
            }
            self.play();
            self.bar = keep.0;
            self.beat = keep.1;
            self.status = format!("Speed trainer: {}%", self.speed.round());
            return sounding;
        }
        self.last_play_pos = pos;
        self.play_secs = pos.saturating_sub(self.start_offset) as f32 / sr;
        let idx = self.timeline.partition_point(|t| t.start <= pos);
        if idx > 0 {
            let tb = &self.timeline[idx - 1];
            let exact = (pos - tb.start) as f64 / tb.samples_per_tick;
            self.play_tick = Some((tb.bar, exact as f32));
            let tick = exact as u32;
            let beat_len = QUARTER * 4 / self.song.master[tb.bar].den.max(1) as u32;
            self.metro_flash = tick % beat_len < beat_len / 5;
            let beats = &self.song.tracks[self.track].bars[tb.bar].voice(self.voice);
            let mut acc = 0;
            let mut j = 0;
            for (k, b) in beats.iter().enumerate() {
                if acc <= tick {
                    j = k;
                }
                acc += b.ticks();
            }
            let new = Some((tb.bar, j));
            if new != self.play_beat && self.follow {
                self.scroll_to_cursor = true;
            }
            self.play_beat = new;
        }
        sounding
    }

    // ----- input -----

    /// Shortcuts that work on the start screen too.
    fn handle_global_keys(&mut self, ctx: &egui::Context) {
        if self.pending.is_some() {
            return;
        }
        let events = ctx.input(|i| i.events.clone());
        for ev in events {
            if let egui::Event::Key { key, pressed: true, modifiers, .. } = ev {
                if modifiers.command {
                    match key {
                        Key::O => self.request(Pending::Open),
                        Key::N => self.request(Pending::New),
                        _ => {}
                    }
                } else if key == Key::Escape {
                    self.show_home = false;
                } else if key == Key::F1 {
                    self.show_help = true;
                }
            }
        }
    }

    fn handle_screenshot(&mut self, ctx: &egui::Context) {
        let Some(path) = self.screenshot.clone() else { return };
        self.frame += 1;
        ctx.request_repaint();
        if self.frame >= 120 && !self.shot_requested {
            self.shot_requested = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
        let shot = ctx.input(|i| {
            i.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(img) = shot {
            let [w, h] = img.size;
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            match image::RgbaImage::from_raw(w as u32, h as u32, img.as_raw().to_vec()) {
                Some(buf) => {
                    if let Err(e) = buf.save(&path) {
                        eprintln!("Couldn't save the screenshot: {e}");
                    }
                }
                None => eprintln!("Screenshot had an unexpected size"),
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if self.frame > 600 {
            eprintln!("Screenshot timed out");
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}
