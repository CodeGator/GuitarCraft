//! Tool windows: directions, add track, chords, scales, lyrics, command palette,
//! speed trainer and the built-in user guide.

use super::App;
use crate::model::*;
use egui::{Align2, Color32, RichText, Vec2};

pub const NOTE_NAMES: [&str; 12] = ["C", "C\u{266F}", "D", "E\u{266D}", "E", "F", "F\u{266F}", "G", "A\u{266D}", "A", "B\u{266D}", "B"];

/// Chord types: name suffix and intervals from the root.
pub const CHORD_TYPES: [(&str, &[u8]); 16] = [
    ("", &[0, 4, 7]),
    ("m", &[0, 3, 7]),
    ("7", &[0, 4, 7, 10]),
    ("maj7", &[0, 4, 7, 11]),
    ("m7", &[0, 3, 7, 10]),
    ("5", &[0, 7]),
    ("sus2", &[0, 2, 7]),
    ("sus4", &[0, 5, 7]),
    ("dim", &[0, 3, 6]),
    ("aug", &[0, 4, 8]),
    ("6", &[0, 4, 7, 9]),
    ("m6", &[0, 3, 7, 9]),
    ("add9", &[0, 2, 4, 7]),
    ("9", &[0, 2, 4, 7, 10]),
    ("m7b5", &[0, 3, 6, 10]),
    ("dim7", &[0, 3, 6, 9]),
];

/// Scales: name and intervals.
pub const SCALES: [(&str, &[u8]); 14] = [
    ("Major (Ionian)", &[0, 2, 4, 5, 7, 9, 11]),
    ("Natural minor (Aeolian)", &[0, 2, 3, 5, 7, 8, 10]),
    ("Major pentatonic", &[0, 2, 4, 7, 9]),
    ("Minor pentatonic", &[0, 3, 5, 7, 10]),
    ("Blues", &[0, 3, 5, 6, 7, 10]),
    ("Dorian", &[0, 2, 3, 5, 7, 9, 10]),
    ("Phrygian", &[0, 1, 3, 5, 7, 8, 10]),
    ("Lydian", &[0, 2, 4, 6, 7, 9, 11]),
    ("Mixolydian", &[0, 2, 4, 5, 7, 9, 10]),
    ("Locrian", &[0, 1, 3, 5, 6, 8, 10]),
    ("Harmonic minor", &[0, 2, 3, 5, 7, 8, 11]),
    ("Melodic minor", &[0, 2, 3, 5, 7, 9, 11]),
    ("Whole tone", &[0, 2, 4, 6, 8, 10]),
    ("Chromatic", &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]),
];

/// Names the chord formed by a set of pitches, if it matches a known shape.
pub fn detect_chord(pitches: &[u8]) -> Option<String> {
    if pitches.len() < 2 {
        return None;
    }
    let bass = *pitches.iter().min()? % 12;
    let mut pcs: Vec<u8> = pitches.iter().map(|p| p % 12).collect();
    pcs.sort();
    pcs.dedup();
    let mut best: Option<(i32, String)> = None;
    for root in 0..12u8 {
        for (suffix, iv) in CHORD_TYPES {
            let shape: Vec<u8> = iv.iter().map(|i| (root + i) % 12).collect();
            if !pcs.iter().all(|p| shape.contains(p)) {
                continue;
            }
            let missing = shape.iter().filter(|s| !pcs.contains(s)).count() as i32;
            // Prefer complete chords, root in the bass, simple shapes.
            let score = missing * 10 + if root == bass { 0 } else { 3 } + iv.len() as i32;
            if missing <= 1 && best.as_ref().map(|b| score < b.0).unwrap_or(true) {
                let mut name = format!("{}{}", NOTE_NAMES[root as usize], suffix);
                if root != bass {
                    name = format!("{}/{}", name, NOTE_NAMES[bass as usize]);
                }
                best = Some((score, name));
            }
        }
    }
    best.map(|b| b.1)
}

/// Scales containing the pitch classes, best matches first: (root, scale index, coverage %).
pub fn find_scales(pcs: &[u8]) -> Vec<(u8, usize, u32)> {
    if pcs.is_empty() {
        return vec![];
    }
    let mut out = vec![];
    for root in 0..12u8 {
        for (k, (_, iv)) in SCALES.iter().enumerate().take(SCALES.len() - 1) {
            let set: Vec<u8> = iv.iter().map(|i| (root + i) % 12).collect();
            let hit = pcs.iter().filter(|p| set.contains(p)).count();
            let pct = (hit * 100 / pcs.len()) as u32;
            if pct >= 80 {
                out.push((root, k, pct));
            }
        }
    }
    out.sort_by(|a, b| b.2.cmp(&a.2).then(SCALES[a.1].1.len().cmp(&SCALES[b.1].1.len())));
    out.truncate(12);
    out
}

/// The commands offered by the command palette.
pub type Cmd = fn(&mut App);

pub fn commands() -> Vec<(&'static str, Cmd)> {
    vec![
        ("add-bar: Insert a bar after this one", |a| a.insert_bar(a.bar + 1)),
        ("delete-bar: Delete this bar", |a| a.delete_bar(a.bar)),
        ("palm-mute: Palm mute", |a| a.toggle_note_flag("Palm mute", |n| &mut n.palm_mute)),
        ("let-ring: Let ring", |a| a.toggle_note_flag("Let ring", |n| &mut n.let_ring)),
        ("dead-note: Dead note", |a| a.toggle_note_flag("Dead note", |n| &mut n.dead)),
        ("hammer-on-pull-off: Hammer-on / pull-off", |a| a.toggle_note_flag("Hammer-on / pull-off", |n| &mut n.hammer)),
        ("slide: Legato slide", |a| a.toggle_note_flag("Slide", |n| &mut n.slide)),
        ("vibrato: Vibrato", |a| a.toggle_note_flag("Vibrato", |n| &mut n.vibrato)),
        ("bend: Bend (cycles full, half...)", |a| a.cycle_bend()),
        ("harmonic: Natural harmonic", |a| a.toggle_note_flag("Natural harmonic", |n| &mut n.harmonic)),
        ("tie-note: Tie note", |a| a.toggle_note_flag("Tie", |n| &mut n.tie)),
        ("rest: Make a rest", |a| a.edit(|s, t, b, j, _| s.tracks[t].bars[b].beats[j].notes.clear())),
        ("fermata: Fermata", |a| a.beat_edit(|b| b.fermata = !b.fermata)),
        ("crescendo: Crescendo", |a| a.beat_edit(|b| b.hairpin = if b.hairpin == 1 { 0 } else { 1 })),
        ("decrescendo: Decrescendo", |a| a.beat_edit(|b| b.hairpin = if b.hairpin == 2 { 0 } else { 2 })),
        ("fade-in: Fade in", |a| a.beat_edit(|b| b.fade_in = !b.fade_in)),
        ("tremolo-picking: Tremolo picking", |a| a.beat_edit(|b| b.tremolo = !b.tremolo)),
        ("double-barline: Double bar line", |a| a.bar_edit(|m| m.double_bar = !m.double_bar)),
        ("repeat-open: Start repeat", |a| a.bar_edit(|m| m.repeat_open = !m.repeat_open)),
        ("repeat-close: End repeat", |a| a.bar_edit(|m| m.repeat_close = if m.repeat_close > 0 { 0 } else { 2 })),
        ("force-break-line: New line after this bar", |a| a.bar_edit(|m| m.line_break = !m.line_break)),
        ("triplet-feel: Triplet feel", |a| a.bar_edit(|m| m.triplet_feel = !m.triplet_feel)),
        ("direction: Directions (Segno, Coda, D.C., D.S.)", |a| a.dir_open = true),
        ("chord: Chord name and diagram", |a| a.open_chord_tool()),
        ("lyrics: Lyrics editor", |a| a.lyrics_open = true),
        ("transpose: Transpose", |a| a.transpose_open = true),
        ("scale: Scales", |a| a.scale_open = true),
        ("section: Name this section", |a| a.edit_section()),
        ("go-to: Go to a bar", |a| a.goto_open = true),
        ("check-bars: Check bar duration", |a| a.check_bar_duration()),
        ("complete-bars: Complete bars with rests", |a| a.complete_bars()),
        ("add-track: Add a track", |a| a.addtrack_open = true),
        ("tempo: Edit the tempo (automations)", |a| a.open_automations(a.bar)),
        ("move-voice: Move the note(s) to voice 2", |a| a.move_to_voice(1)),
        ("swap-voices: Move, copy or swap voices", |a| a.voices_open = true),
        ("duplicate-track: Duplicate this track", |a| a.duplicate_track()),
        ("multitrack: Show all tracks / one track", |a| a.single_track = !a.single_track),
        ("fretboard: Show or hide the fretboard", |a| a.show_fretboard = !a.show_fretboard),
        ("keyboard: Show or hide the keyboard", |a| a.show_keyboard = !a.show_keyboard),
        ("loop: Loop playback", |a| a.looping = !a.looping),
        ("metronome: Metronome", |a| a.metronome = !a.metronome),
        ("count-in: Count-in", |a| a.count_in = !a.count_in),
        ("play: Play / pause", |a| a.toggle_play()),
        ("play-from-start: Play from the beginning", |a| a.play_from_start()),
        ("zoom-in: Zoom in", |a| a.zoom_step(1)),
        ("zoom-out: Zoom out", |a| a.zoom_step(-1)),
        ("export-midi: Export MIDI", |a| a.export_midi()),
        ("export-ascii: Export ASCII tab", |a| a.export_ascii()),
        ("export-audio: Export audio (WAV)", |a| a.export_wav()),
        ("import-ascii: Import ASCII tab into a new track", |a| a.import_ascii_dialog()),
        ("save: Save", |a| a.save(false)),
        ("undo: Undo", |a| a.do_undo()),
        ("redo: Redo", |a| a.do_redo()),
        ("user-guide: Open the user guide", |a| a.guide_open = true),
    ]
}

const GUIDE: [(&str, &str); 13] = [
    ("The main screen", "The toolbar is at the top: start screen, the three panel buttons (palette, tracks, settings), zoom, undo/redo, the transport buttons and the display that shows the track, bar, time signature, time and tempo. The editing palette is on the left, the score in the middle, the Song/Track/Bar/Beat settings on the right and the tracks list with the mixer and bar overview at the bottom."),
    ("Moving around", "Click anywhere in the score to place the cursor, or use the arrow keys: left and right move between beats, up and down between strings. Ctrl + arrows jump a bar or a track, Tab moves to the next track's staff. Click a square in the bar overview, a bar number or a section name to jump there. Drag across the score to select beats."),
    ("Writing notes", "Put the cursor on a string and type the fret number (type two digits quickly for frets above 9). + makes the note shorter and \u{2212} longer; R makes a rest; . dots it. Moving right past the last beat adds a new beat with the same length, and a new bar when the bar is full. The fretboard and keyboard views (View menu) also add notes when clicked."),
    ("Effects and symbols", "Use the palette, the Note and Effects menus or the shortcuts (H hammer-on, S slide, B bend, V vibrato, P palm mute, I let ring, X dead note, O ghost note, Y harmonic, N trill, F fermata). Bars that are too short or too long are shaded red; Tools \u{25B8} Check Bar Duration (F4) finds them."),
    ("Bars, repeats and directions", "Bar \u{25B8} Insert/Delete bar changes every track at once. Repeat open/close are [ and ]; alternate endings choose which pass plays a bar. Bar \u{25B8} Directions (D) adds Segno, Coda and Fine signs and D.C./D.S. jumps; playback follows them. Ctrl+Enter forces a new line after the bar."),
    ("Sections", "Section \u{25B8} Edit (Shift+Ins) names the bar's section; names written in [brackets] get a frame. Ctrl+Alt+Left/Right moves between sections, and double-clicking a section under the bar overview selects it."),
    ("Tracks and the mixer", "Add tracks from the Track menu (Ctrl+Shift+Ins) or the + in the tracks list. Each row has show/hide (eye), mute, solo, volume and pan. Right-click a track for more. The Track tab sets the name, tuning, capo, sound and interpretation; changing the tuning can keep the notes' pitch."),
    ("Lyrics and chords", "Track \u{25B8} Show Lyrics Editor: type the words, separate syllables with spaces or hyphens, join words with +, and put comments in [brackets]. Note \u{25B8} Chord (A) names a chord, can detect it from the notes, and can show a diagram above the staff."),
    ("Playing back", "Space plays from the cursor and pauses, Ctrl+Space plays from the start, Esc stops. Click the score while playing to jump there. Loop (F9) repeats the selection; the speed button sets a slower or faster speed, and the speed trainer raises it on each loop. The metronome and count-in are in the toolbar and the Sound menu."),
    ("Voices", "Each track has four voices, for parts with their own rhythm on one staff. Pick the voice to edit with the 1\u{2013}4 buttons at the top of the palette or Ctrl+1\u{2013}4. Alt+1\u{2013}4 moves the note under the cursor, or every note you've selected, into another voice at the same place in the bar. Tools \u{25B8} Move/Swap Voices (Ctrl+Shift+M) moves, copies or swaps voices for the selection, the bar or the whole track."),
    ("Copy and paste", "Ctrl+C / Ctrl+X / Ctrl+V copy, cut and paste beats of the current track; pasting goes in at the cursor. Ctrl+Shift+C / Ctrl+Shift+X copy or cut whole bars of every track, and the next paste inserts those bars."),
    ("Files", "Open songs saved by GuitarCraft and Guitar Pro files (.gp, .gpx, .gp5, .gp4, .gp3), MIDI files and text tabs. Export MIDI, a text tab of the current track, or an audio file (WAV) played with the built-in sound. File \u{25B8} Save As Template keeps the tracks for new songs."),
    ("Views", "View \u{25B8} Page - Vertical, Page - Horizontal, Page - Grid, Screen - Vertical and Screen - Horizontal change how the music is laid out. Ctrl + mouse wheel or Ctrl +/\u{2212} zoom. F2, F8, F5/F6 and F11 show or hide the palette, tracks list, settings and full screen. F3 switches between all tracks and one track."),
];

impl App {
    pub(super) fn open_chord_tool(&mut self) {
        let pitches: Vec<u8> = self.cur_beat().notes.iter().map(|n| self.tr().pitch(n)).collect();
        self.chord_text = self.cur_beat().chord.clone().or_else(|| detect_chord(&pitches)).unwrap_or_default();
        self.chord_open = true;
    }

    pub(super) fn export_wav(&mut self) {
        if self.screenshot.is_some() {
            return;
        }
        let Some(p) = rfd::FileDialog::new().add_filter("WAV audio", &["wav"]).set_file_name(format!("{}.wav", super::widgets::sanitize(&self.song.title))).save_file() else { return };
        let samples = crate::audio::render_song(&self.song, 44100.0, self.speed / 100.0);
        match std::fs::write(&p, crate::fileio::wav_bytes(&samples, 44100)) {
            Ok(()) => self.status = format!("Exported {}.", p.display()),
            Err(e) => self.status = format!("Couldn't export: {e}"),
        }
    }

    /// Adds a text tab file as a new track.
    pub(super) fn import_ascii_dialog(&mut self) {
        if self.screenshot.is_some() {
            return;
        }
        let Some(p) = rfd::FileDialog::new().add_filter("Text tab", &["txt", "tab"]).pick_file() else { return };
        let text = match std::fs::read(&p) {
            Ok(d) => String::from_utf8_lossy(&d).to_string(),
            Err(e) => {
                self.status = format!("Couldn't read the file: {e}");
                return;
            }
        };
        match crate::textmidi::import_ascii(&text) {
            Ok(mut t) => {
                t.name = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or(t.name);
                t.color = TRACK_COLORS[self.song.tracks.len() % TRACK_COLORS.len()];
                let n = t.bars.len();
                self.edit(move |s, _, _, _, _| {
                    while s.master.len() < n {
                        s.master.push(MasterBar::default());
                    }
                    s.tracks.push(t);
                });
                self.track = self.song.tracks.len() - 1;
                self.clamp_cursor();
                self.status = "Imported the tab. Tools \u{25B8} Check Bar Duration finds bars whose rhythm needs fixing.".into();
            }
            Err(e) => self.status = e,
        }
    }

    pub(super) fn import_midi_dialog(&mut self) {
        if self.screenshot.is_some() {
            return;
        }
        if let Some(p) = rfd::FileDialog::new().add_filter("MIDI", &["mid", "midi"]).pick_file() {
            if self.dirty {
                self.status = "Save or undo your changes first.".into();
            } else {
                self.open_path(p);
            }
        }
    }

    pub(super) fn tool_windows(&mut self, ctx: &egui::Context) {
        self.directions_window(ctx);
        self.add_track_window(ctx);
        self.tuning_window(ctx);
        self.chord_window(ctx);
        self.scale_window(ctx);
        self.lyrics_window(ctx);
        self.command_palette(ctx);
        self.trainer_window(ctx);
        self.guide_window(ctx);
        self.stylesheet_window(ctx);
    }

    fn directions_window(&mut self, ctx: &egui::Context) {
        if !self.dir_open {
            return;
        }
        let mut open = true;
        let b = self.bar;
        let mut mb = self.song.master[b].clone();
        egui::Window::new(format!("Directions \u{2013} bar {}", b + 1)).open(&mut open).resizable(false).collapsible(false).show(ctx, |ui| {
            ui.label(RichText::new("Target at the start of the bar").strong());
            ui.horizontal_wrapped(|ui| {
                for (k, name) in TARGET_NAMES.iter().enumerate() {
                    ui.radio_value(&mut mb.direction_target, k as u8, *name);
                }
            });
            ui.add_space(6.0);
            ui.label(RichText::new("Jump at the end of the bar").strong());
            egui::Grid::new("jumps").num_columns(2).show(ui, |ui| {
                for (k, name) in JUMP_NAMES.iter().enumerate() {
                    ui.radio_value(&mut mb.direction_jump, k as u8, *name);
                    if k % 2 == 1 {
                        ui.end_row();
                    }
                }
            });
            ui.add_space(4.0);
            ui.label(RichText::new("D.C. goes back to the start, D.S. to the Segno; \u{201C}al Fine\u{201D} stops at Fine, \u{201C}al Coda\u{201D} jumps from To Coda to the Coda.").small().weak());
        });
        if mb != self.song.master[b] {
            self.bar_edit(move |m| *m = mb);
        }
        self.dir_open = open;
    }

    fn add_track_window(&mut self, ctx: &egui::Context) {
        if !self.addtrack_open {
            return;
        }
        let mut open = true;
        let mut create = false;
        egui::Window::new("Add a track").open(&mut open).resizable(false).collapsible(false).anchor(Align2::CENTER_CENTER, Vec2::ZERO).show(ctx, |ui| {
            // Instrument type first (stringed, keyboards, vocals, orchestra, drums), then the instrument.
            ui.horizontal(|ui| {
                for k in InstrumentType::ALL {
                    if ui.add(egui::Button::selectable(self.at_inst.kind() == k, k.name()).frame_when_inactive(true).min_size(Vec2::new(78.0, 26.0))).clicked() && self.at_inst.kind() != k {
                        let i = k.instruments()[0];
                        self.at_inst = i;
                        self.at_tuning = i.default_tuning();
                        self.at_grand = true;
                        self.at_name = i.name().to_string();
                    }
                }
            });
            ui.add_space(6.0);
            egui::Grid::new("addtr").num_columns(2).spacing(Vec2::new(8.0, 6.0)).show(ui, |ui| {
                ui.label("Instrument:");
                egui::ComboBox::from_id_salt("at_inst").width(200.0).selected_text(self.at_inst.name()).show_ui(ui, |ui| {
                    for i in self.at_inst.kind().instruments() {
                        if ui.selectable_value(&mut self.at_inst, i, i.name()).changed() {
                            self.at_tuning = i.default_tuning();
                            if self.at_name.is_empty() || Instrument::ALL.iter().any(|x| x.name() == self.at_name) {
                                self.at_name = i.name().to_string();
                            }
                        }
                    }
                });
                ui.end_row();
                ui.label("Name:");
                ui.add(egui::TextEdit::singleline(&mut self.at_name).desired_width(200.0));
                ui.end_row();
                let kind = self.at_inst.kind();
                ui.label("Notation:");
                ui.label(match kind {
                    InstrumentType::Stringed => "Standard notation and tablature",
                    InstrumentType::Drums => "Percussion staff",
                    _ => "Standard notation",
                });
                ui.end_row();
                if kind == InstrumentType::Keyboard {
                    ui.label("Staff:");
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut self.at_grand, false, "Simple");
                        ui.selectable_value(&mut self.at_grand, true, "Grand staff");
                    });
                    ui.end_row();
                }
                if kind == InstrumentType::Stringed {
                    ui.label("Strings:");
                    let mut n = self.at_tuning.len();
                    egui::ComboBox::from_id_salt("at_str").width(60.0).selected_text(format!("{n}")).show_ui(ui, |ui| {
                        for k in [4usize, 5, 6, 7] {
                            ui.selectable_value(&mut n, k, format!("{k}"));
                        }
                    });
                    if n != self.at_tuning.len() {
                        self.at_tuning = tuning_presets(n)[0].1.clone();
                    }
                    ui.end_row();
                    ui.label("Tuning:");
                    egui::ComboBox::from_id_salt("at_tun").width(200.0).selected_text(tuning_name(&self.at_tuning)).show_ui(ui, |ui| {
                        for (name, notes) in tuning_presets(self.at_tuning.len()) {
                            if ui.selectable_label(self.at_tuning == notes, name).clicked() {
                                self.at_tuning = notes;
                            }
                        }
                    });
                    ui.end_row();
                }
                ui.label("Colour:");
                ui.horizontal(|ui| {
                    for (k, c) in TRACK_COLORS.iter().enumerate() {
                        let (r, resp) = ui.allocate_exact_size(Vec2::splat(18.0), egui::Sense::click());
                        ui.painter().rect_filled(r, 3.0, Color32::from_rgb(c[0], c[1], c[2]));
                        if self.at_color == k {
                            ui.painter().rect_stroke(r, 3.0, egui::Stroke::new(2.0, Color32::WHITE), egui::StrokeKind::Outside);
                        }
                        if resp.clicked() {
                            self.at_color = k;
                        }
                    }
                });
                ui.end_row();
            });
            ui.add_space(6.0);
            if ui.button(RichText::new("Create").strong()).clicked() {
                create = true;
            }
        });
        if create {
            let name = if self.at_name.trim().is_empty() { self.at_inst.name().to_string() } else { self.at_name.clone() };
            let mut t = Track::of_instrument(self.at_inst, &name);
            if t.has_strings() {
                t.tuning = self.at_tuning.clone();
            }
            t.grand_staff = t.kind() == InstrumentType::Keyboard && self.at_grand;
            t.color = TRACK_COLORS[self.at_color % TRACK_COLORS.len()];
            self.edit(move |s, _, _, _, _| s.tracks.push(t));
            self.track = self.song.tracks.len() - 1;
            self.clamp_cursor();
            open = false;
        }
        self.addtrack_open = open;
    }

    fn chord_window(&mut self, ctx: &egui::Context) {
        if !self.chord_open {
            return;
        }
        let mut open = true;
        let pitches: Vec<u8> = self.cur_beat().notes.iter().map(|n| self.tr().pitch(n)).collect();
        let detected = detect_chord(&pitches);
        let mut apply: Option<(Option<String>, bool)> = None;
        let mut diagram = self.cur_beat().chord_diagram;
        egui::Window::new("Chord").open(&mut open).resizable(false).collapsible(false).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Name:");
                ui.add(egui::TextEdit::singleline(&mut self.chord_text).desired_width(120.0));
            });
            ui.label(RichText::new("Build it:").weak());
            ui.horizontal_wrapped(|ui| {
                for (k, n) in NOTE_NAMES.iter().enumerate() {
                    if ui.selectable_label(self.chord_root == k, *n).clicked() {
                        self.chord_root = k;
                        self.chord_text = format!("{}{}", NOTE_NAMES[k], CHORD_TYPES[self.chord_type].0);
                    }
                }
            });
            ui.horizontal_wrapped(|ui| {
                for (k, (suffix, _)) in CHORD_TYPES.iter().enumerate() {
                    let lab = if suffix.is_empty() { "maj" } else { suffix };
                    if ui.selectable_label(self.chord_type == k, lab).clicked() {
                        self.chord_type = k;
                        self.chord_text = format!("{}{}", NOTE_NAMES[self.chord_root], suffix);
                    }
                }
            });
            ui.horizontal(|ui| {
                match &detected {
                    Some(d) => {
                        ui.label(format!("The notes on this beat make {d}."));
                        if ui.button("Use it").clicked() {
                            self.chord_text = d.clone();
                        }
                    }
                    None => {
                        ui.label(RichText::new("Put two or more notes on the beat to detect a chord.").weak());
                    }
                }
            });
            ui.checkbox(&mut diagram, "Show a diagram of this beat's notes above the staff");
            ui.horizontal(|ui| {
                if ui.button(RichText::new("OK").strong()).clicked() {
                    let name = if self.chord_text.trim().is_empty() { None } else { Some(self.chord_text.trim().to_string()) };
                    apply = Some((name, diagram));
                }
                if ui.button("Remove chord").clicked() {
                    apply = Some((None, false));
                }
            });
        });
        if let Some((name, dia)) = apply {
            self.edit(move |s, t, b, j, _| {
                let beat = &mut s.tracks[t].bars[b].beats[j];
                beat.chord = name;
                beat.chord_diagram = dia;
            });
            open = false;
        }
        self.chord_open = open;
    }

    fn scale_window(&mut self, ctx: &egui::Context) {
        if !self.scale_open {
            return;
        }
        let mut open = true;
        let mut play = false;
        egui::Window::new("Scales").open(&mut open).resizable(false).collapsible(false).show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label("Key:");
                for (k, n) in NOTE_NAMES.iter().enumerate() {
                    if ui.selectable_label(self.scale_root == k as u8, *n).clicked() {
                        self.scale_root = k as u8;
                    }
                }
            });
            egui::ComboBox::from_id_salt("scale").width(240.0).selected_text(SCALES[self.scale_kind].0).show_ui(ui, |ui| {
                for (k, (n, _)) in SCALES.iter().enumerate() {
                    ui.selectable_value(&mut self.scale_kind, k, *n);
                }
            });
            let iv = SCALES[self.scale_kind].1;
            let names: Vec<&str> = iv.iter().map(|i| NOTE_NAMES[((self.scale_root + i) % 12) as usize]).collect();
            ui.label(RichText::new(names.join("  ")).strong().size(15.0));
            let steps: Vec<String> = iv.windows(2).map(|w| format!("{}", w[1] - w[0])).chain(std::iter::once(format!("{}", 12 - iv.last().unwrap()))).collect();
            ui.label(RichText::new(format!("Steps in semitones: {}", steps.join("-"))).weak());
            ui.horizontal(|ui| {
                let mut show = self.scale_shown;
                if ui.checkbox(&mut show, "Show on the fretboard").changed() {
                    self.scale_shown = show;
                    if show {
                        self.show_fretboard = true;
                    }
                }
                if ui.button("\u{25B6} Play").clicked() {
                    play = true;
                }
            });
            ui.separator();
            if ui.button("Find scales in the selection (or this bar)").clicked() {
                let (a, b) = match self.selection() {
                    Some((a, b)) => (a.0, b.0),
                    None => (self.bar, self.bar),
                };
                let tr = &self.song.tracks[self.track];
                let mut pcs: Vec<u8> = (a..=b).flat_map(|bi| tr.bars[bi].beats.iter().flat_map(|bt| bt.notes.iter().map(|n| tr.pitch(n) % 12))).collect();
                pcs.sort();
                pcs.dedup();
                self.scale_found = find_scales(&pcs);
            }
            for &(root, k, pct) in self.scale_found.clone().iter() {
                if ui.selectable_label(false, format!("{} {}   ({pct}%)", NOTE_NAMES[root as usize], SCALES[k].0)).clicked() {
                    self.scale_root = root;
                    self.scale_kind = k;
                }
            }
        });
        if play {
            self.play_scale();
        }
        if !open {
            self.scale_shown = false;
        }
        self.scale_open = open;
    }

    fn play_scale(&mut self) {
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
        let low = *tr.tuning.iter().min().unwrap_or(&40) as i32;
        let base = low + ((self.scale_root as i32 - low).rem_euclid(12));
        let iv = SCALES[self.scale_kind].1;
        if let Ok(mut st) = a.state.lock() {
            for (k, i) in iv.iter().chain(std::iter::once(&12)).enumerate() {
                let pitch = base + *i as i32;
                let s = tr.strings() - 1;
                let fret = (pitch - tr.tuning[s] as i32 - tr.capo_at(s) as i32).clamp(0, 30) as u8;
                let mut ev = crate::audio::note_preview(&tr, self.track, &Note::new(s as u8, fret), sr);
                ev.start = (k as f32 * sr * 0.28) as u64;
                ev.len = (sr * 0.3) as u64;
                st.preview.push(ev);
            }
        }
    }

    fn lyrics_window(&mut self, ctx: &egui::Context) {
        if !self.lyrics_open {
            return;
        }
        let mut open = true;
        let ti = self.track;
        let mut text = self.song.tracks[ti].lyrics.clone();
        let mut start = self.song.tracks[ti].lyrics_bar + 1;
        egui::Window::new(format!("Lyrics \u{2013} {}", self.song.tracks[ti].name)).open(&mut open).default_width(420.0).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Starts at bar");
                ui.add(egui::DragValue::new(&mut start).range(1..=self.song.bar_count()));
            });
            ui.add(egui::TextEdit::multiline(&mut text).desired_rows(8).desired_width(f32::INFINITY).hint_text("Hel-lo ev-ery-bo-dy"));
            ui.label(RichText::new("Each syllable goes on the next beat with notes. Separate syllables with a space or a hyphen; two spaces leave a beat empty; + keeps words together; [text in brackets] isn't shown.").small().weak());
        });
        let t = &self.song.tracks[ti];
        if text != t.lyrics || start - 1 != t.lyrics_bar {
            // One undo step for the whole time the editor is open.
            if !self.lyrics_snap {
                self.snapshot();
                self.lyrics_snap = true;
            }
            self.song.tracks[ti].lyrics = text;
            self.song.tracks[ti].lyrics_bar = start - 1;
            self.dirty = true;
        }
        if !open {
            self.lyrics_snap = false;
        }
        self.lyrics_open = open;
    }

    fn command_palette(&mut self, ctx: &egui::Context) {
        if !self.cmd_open {
            return;
        }
        let list = commands();
        let q = self.cmd_text.to_lowercase();
        let hits: Vec<(&str, Cmd)> = list.into_iter().filter(|(n, _)| q.trim().is_empty() || q == "?" || q.split_whitespace().all(|w| n.to_lowercase().contains(w))).collect();
        let mut run: Option<Cmd> = None;
        let mut close = false;
        egui::Window::new("Command palette").title_bar(false).resizable(false).anchor(Align2::CENTER_TOP, Vec2::new(0.0, 90.0)).fixed_size(Vec2::new(460.0, 0.0)).show(ctx, |ui| {
            let r = ui.add(egui::TextEdit::singleline(&mut self.cmd_text).hint_text("Type a command, e.g. palm, fermata, add-bar\u{2026}").desired_width(f32::INFINITY));
            r.request_focus();
            if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                self.cmd_sel = (self.cmd_sel + 1).min(hits.len().saturating_sub(1));
            }
            if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                self.cmd_sel = self.cmd_sel.saturating_sub(1);
            }
            if r.changed() {
                self.cmd_sel = 0;
            }
            egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                for (k, (name, f)) in hits.iter().enumerate() {
                    let (pre, desc) = name.split_once(": ").unwrap_or((name, ""));
                    let resp = ui.selectable_label(k == self.cmd_sel, RichText::new(format!("{pre}   \u{2014} {desc}")));
                    if k == self.cmd_sel {
                        resp.scroll_to_me(None);
                    }
                    if resp.clicked() {
                        run = Some(*f);
                    }
                }
            });
            if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                run = hits.get(self.cmd_sel).map(|h| h.1);
            }
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                close = true;
            }
        });
        if let Some(f) = run {
            self.cmd_open = false;
            self.cmd_text.clear();
            f(self);
        }
        if close {
            self.cmd_open = false;
        }
    }

    fn trainer_window(&mut self, ctx: &egui::Context) {
        if !self.trainer_open {
            return;
        }
        let mut open = true;
        egui::Window::new("Speed trainer").open(&mut open).resizable(false).collapsible(false).show(ctx, |ui| {
            ui.checkbox(&mut self.trainer_on, "Raise the speed each time the loop repeats");
            egui::Grid::new("trainer").num_columns(2).show(ui, |ui| {
                ui.label("Start at");
                ui.add(egui::DragValue::new(&mut self.trainer_start).range(10.0..=300.0).suffix("%"));
                ui.end_row();
                ui.label("End at");
                ui.add(egui::DragValue::new(&mut self.trainer_end).range(10.0..=300.0).suffix("%"));
                ui.end_row();
                ui.label("Step");
                ui.add(egui::DragValue::new(&mut self.trainer_step).range(1.0..=50.0).suffix("%"));
                ui.end_row();
            });
            ui.label(RichText::new("Select the bars to practise, then press Play: Loop turns on automatically.").small().weak());
        });
        self.trainer_open = open;
    }

    fn guide_window(&mut self, ctx: &egui::Context) {
        if !self.guide_open {
            return;
        }
        let mut open = true;
        egui::Window::new("GuitarCraft user guide").open(&mut open).default_width(560.0).default_height(520.0).show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                for (title, body) in GUIDE {
                    ui.label(RichText::new(title).strong().size(15.0));
                    ui.label(body);
                    ui.add_space(8.0);
                }
                ui.label(RichText::new("All keyboard shortcuts").strong().size(15.0));
                super::actions::shortcut_rows(ui);
            });
        });
        self.guide_open = open;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chord_names() {
        assert_eq!(detect_chord(&[43, 47, 50, 55, 59, 67]).as_deref(), Some("G"));
        assert_eq!(detect_chord(&[40, 47, 52, 55, 59, 64]).as_deref(), Some("Em"));
        assert_eq!(detect_chord(&[48, 52, 55, 58]).as_deref(), Some("C7"));
    }

    #[test]
    fn scale_finder() {
        let found = find_scales(&[0, 2, 4, 5, 7, 9, 11]);
        assert!(found.iter().any(|&(r, k, p)| r == 0 && k == 0 && p == 100));
    }
}
