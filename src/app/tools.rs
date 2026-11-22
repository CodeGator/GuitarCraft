//! Tool windows: directions, add track, chords, scales, lyrics, command palette,
//! speed trainer and the built-in user guide.

use super::App;
use crate::model::*;
use egui::{Align2, RichText, Vec2};

pub const NOTE_NAMES: [&str; 12] = [
    "C",
    "C\u{266F}",
    "D",
    "E\u{266D}",
    "E",
    "F",
    "F\u{266F}",
    "G",
    "A\u{266D}",
    "A",
    "B\u{266D}",
    "B",
];

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

/// The commands offered by the command palette.
pub type Cmd = fn(&mut App);

pub fn commands() -> Vec<(&'static str, Cmd)> {
    vec![
        ("add-bar: Insert a bar after this one", |a| {
            a.insert_bar(a.bar + 1)
        }),
        ("delete-bar: Delete this bar", |a| a.delete_bar(a.bar)),
        ("palm-mute: Palm mute", |a| {
            a.toggle_note_flag("Palm mute", |n| &mut n.palm_mute)
        }),
        ("let-ring: Let ring", |a| {
            a.toggle_note_flag("Let ring", |n| &mut n.let_ring)
        }),
        ("dead-note: Dead note", |a| {
            a.toggle_note_flag("Dead note", |n| &mut n.dead)
        }),
        ("hammer-on-pull-off: Hammer-on / pull-off", |a| {
            a.toggle_note_flag("Hammer-on / pull-off", |n| &mut n.hammer)
        }),
        ("slide: Legato slide", |a| {
            a.toggle_note_flag("Slide", |n| &mut n.slide)
        }),
        ("vibrato: Vibrato", |a| {
            a.toggle_note_flag("Vibrato", |n| &mut n.vibrato)
        }),
        ("bend: Bend (cycles full, half...)", |a| a.cycle_bend()),
        ("harmonic: Natural harmonic", |a| {
            a.toggle_note_flag("Natural harmonic", |n| &mut n.harmonic)
        }),
        ("tie-note: Tie note", |a| {
            a.toggle_note_flag("Tie", |n| &mut n.tie)
        }),
        ("rest: Make a rest", |a| {
            a.edit(|s, t, b, j, _| s.tracks[t].bars[b].beats[j].notes.clear())
        }),
        ("fermata: Fermata", |a| {
            a.beat_edit(|b| b.fermata = !b.fermata)
        }),
        ("crescendo: Crescendo", |a| {
            a.beat_edit(|b| b.hairpin = if b.hairpin == 1 { 0 } else { 1 })
        }),
        ("decrescendo: Decrescendo", |a| {
            a.beat_edit(|b| b.hairpin = if b.hairpin == 2 { 0 } else { 2 })
        }),
        ("fade-in: Fade in", |a| {
            a.beat_edit(|b| b.fade_in = !b.fade_in)
        }),
        ("tremolo-picking: Tremolo picking", |a| {
            a.beat_edit(|b| b.tremolo = !b.tremolo)
        }),
        ("double-barline: Double bar line", |a| {
            a.bar_edit(|m| m.double_bar = !m.double_bar)
        }),
        ("repeat-open: Start repeat", |a| {
            a.bar_edit(|m| m.repeat_open = !m.repeat_open)
        }),
        ("repeat-close: End repeat", |a| {
            a.bar_edit(|m| m.repeat_close = if m.repeat_close > 0 { 0 } else { 2 })
        }),
        ("force-break-line: New line after this bar", |a| {
            a.bar_edit(|m| m.line_break = !m.line_break)
        }),
        ("triplet-feel: Triplet feel", |a| {
            a.bar_edit(|m| m.triplet_feel = !m.triplet_feel)
        }),
        ("direction: Directions (Segno, Coda, D.C., D.S.)", |a| {
            a.dir_open = true
        }),
        ("chord: Chord name and diagram", |a| a.open_chord_tool()),
        ("lyrics: Lyrics editor", |a| a.lyrics_open = true),
        ("transpose: Transpose", |a| a.transpose_open = true),
        ("scales: Chords & Scales", |a| a.open_chord_scales(super::chordscales::Mode::Scales)),
        ("section: Name this section", |a| a.edit_section()),
        ("go-to: Go to a bar", |a| a.goto_open = true),
        ("check-bars: Check bar duration", |a| a.check_bar_duration()),
        ("complete-bars: Complete bars with rests", |a| {
            a.complete_bars()
        }),
        ("add-track: Add a track", |a| a.open_add_track(false)),
        ("tempo: Edit the tempo (automations)", |a| {
            a.open_automations(a.bar)
        }),
        ("move-voice: Move the note(s) to voice 2", |a| {
            a.move_to_voice(1)
        }),
        ("swap-voices: Move, copy or swap voices", |a| {
            a.voices_open = true
        }),
        ("duplicate-track: Duplicate this track", |a| {
            a.duplicate_track()
        }),
        ("multitrack: Show all tracks / one track", |a| {
            a.single_track = !a.single_track
        }),
        ("fretboard: Show or hide the fretboard", |a| {
            a.show_fretboard = !a.show_fretboard
        }),
        ("keyboard: Show or hide the keyboard", |a| {
            a.show_keyboard = !a.show_keyboard
        }),
        ("loop: Loop playback", |a| a.looping = !a.looping),
        ("metronome: Metronome", |a| a.metronome = !a.metronome),
        ("count-in: Count-in", |a| a.count_in = !a.count_in),
        ("play: Play / pause", |a| a.toggle_play()),
        ("play-from-start: Play from the beginning", |a| {
            a.play_from_start()
        }),
        ("zoom-in: Zoom in", |a| a.zoom_step(1)),
        ("zoom-out: Zoom out", |a| a.zoom_step(-1)),
        ("export-midi: Export MIDI", |a| a.export_midi()),
        ("export-ascii: Export ASCII tab", |a| a.export_ascii()),
        ("export-audio: Export audio (WAV)", |a| a.export_wav()),
        ("import-ascii: Import ASCII tab into a new track", |a| {
            a.import_ascii_dialog()
        }),
        ("save: Save", |a| a.save(false)),
        ("undo: Undo", |a| a.do_undo()),
        ("redo: Redo", |a| a.do_redo()),
        ("user-guide: Open the user guide", |a| a.guide_open = true),
    ]
}

/// Help > User Guide. Shortcuts are written as `({Act})`, the name of an action in
/// the shortcut list: the guide shows the person's key for it, and leaves the
/// brackets out when it has none (see `shortcuts::fill_keys`). Keys that can't be
/// changed (arrows, digits, Tab) are written as they are.
const GUIDE: [(&str, &str); 14] = [
    (
        "The main screen",
        "The toolbar is at the top: start screen, the three panel buttons (palette, tracks, settings), zoom, undo/redo, the transport buttons and the display that shows the track, bar, time signature, time and tempo. The editing palette is on the left, the score in the middle, the Song/Track/Bar/Beat settings on the right and the tracks list with the mixer and bar overview at the bottom.",
    ),
    (
        "Moving around",
        "Click anywhere in the score to place the cursor, or use the arrow keys: left and right move between beats, up and down between strings. Next bar ({NextBar}) and previous bar ({PrevBar}) jump a bar, next track ({NextTrack}) and previous track ({PrevTrack}) change track, and Tab moves to the next track's staff. Click a square in the bar overview, a bar number or a section name to jump there. Drag across the score to select beats.",
    ),
    (
        "Writing notes",
        "Put the cursor on a string and type the fret number (type two digits quickly for frets above 9). Shorter ({Shorter}) and longer ({Longer}) change the note's length; Rest ({Rest}) makes a rest and Dotted ({Dotted}) dots it. Moving right past the last beat adds a new beat with the same length, and a new bar when the bar is full. The fretboard and keyboard views (View menu) also add notes when clicked.",
    ),
    (
        "Effects and symbols",
        "Use the palette, the Note and Effects menus or the shortcuts: hammer-on ({Hammer}), slide ({Slide}), bend ({Bend}), vibrato ({Vibrato}), palm mute ({PalmMuteNote}), let ring ({LetRing}), dead note ({DeadNote}), ghost note ({Ghost}), harmonic ({NaturalHarmonic}), trill ({Trill}), fermata ({Fermata}). Bars that are too short or too long are shaded red; Tools \u{25B8} Check Bar Duration ({CheckBars}) finds them.",
    ),
    (
        "Bars, repeats and directions",
        "Bar \u{25B8} Insert/Delete bar changes every track at once. Repeat open ({RepeatOpen}) and repeat close ({RepeatClose}) mark repeats; alternate endings choose which pass plays a bar. Bar \u{25B8} Directions ({Directions}) adds Segno, Coda and Fine signs and D.C./D.S. jumps; playback follows them. Force Break Line ({ForceBreak}) starts a new line after the bar.",
    ),
    (
        "Sections",
        "Section \u{25B8} Edit ({Section}) names the bar's section; names written in [brackets] get a frame. Next section ({NextSection}) and previous section ({PrevSection}) move between sections, and double-clicking a section under the bar overview selects it.",
    ),
    (
        "Tracks and the mixer",
        "Add tracks from the Track menu ({AddTrack}) or the + in the tracks list. Each row has show/hide (eye), mute, solo, volume and pan. Right-click a track for more. The Track tab sets the name, tuning, capo, sound and interpretation; changing the tuning can keep the notes' pitch.",
    ),
    (
        "Lyrics and chords",
        "Track \u{25B8} Show Lyrics Editor: type the words, separate syllables with spaces or hyphens, join words with +, and put comments in [brackets]. Note \u{25B8} Chord ({Chord}) names a chord, can detect it from the notes, and can show a diagram above the staff.",
    ),
    (
        "Playing back",
        "Play ({PlayPause}) plays from the cursor and pauses, Play from the Beginning ({PlayFromStart}) starts at the top, and Stop ({Stop}) stops. Click the score while playing to jump there. Loop ({Loop}) repeats the selection; the speed button sets a slower or faster speed, and the speed trainer raises it on each loop. The metronome and count-in are in the toolbar and the Sound menu.",
    ),
    (
        "MIDI",
        "Preferences (File \u{25B8} Preferences) > Audio & MIDI chooses a MIDI output and input. With an output, playback and clicked notes also go to that device, using each track's instrument, volume, pan and bends; the built-in sound is silent unless you turn on both. With an input, notes you play on a MIDI keyboard are entered at the cursor and the cursor moves on; notes played together become one chord. Playback on a MIDI output doesn't follow volume or pan changes made while it plays.",
    ),
    (
        "Voices",
        "Each track has four voices, for parts with their own rhythm on one staff. Pick the voice to edit with the 1\u{2013}4 buttons at the top of the palette or Voice 1 ({Voice1}) to Voice 4 ({Voice4}). Move to Voice 1 ({MoveToVoice1}) to Voice 4 ({MoveToVoice4}) moves the note under the cursor, or every note you've selected, into another voice at the same place in the bar. Tools \u{25B8} Move/Swap Voices ({MoveVoices}) moves, copies or swaps voices for the selection, the bar or the whole track.",
    ),
    (
        "Copy and paste",
        "Copy ({Copy}), Cut ({Cut}) and Paste ({Paste}) work on beats of the current track; pasting goes in at the cursor. All-Track Copy ({AllTrackCopy}) and All-Track Cut ({AllTrackCut}) take whole bars of every track, and the next paste inserts those bars.",
    ),
    (
        "Files",
        "Open songs saved by GuitarCraft and Guitar Pro files (.gp, .gpx, .gp5, .gp4, .gp3), MIDI files and text tabs. Export MIDI, a text tab of the current track, or an audio file (WAV) played with the built-in sound. File \u{25B8} Save As Template keeps the tracks for new songs.",
    ),
    (
        "Views",
        "View \u{25B8} Page - Vertical, Page - Horizontal, Page - Grid, Screen - Vertical and Screen - Horizontal change how the music is laid out. Ctrl + mouse wheel, Zoom In ({ZoomIn}) and Zoom Out ({ZoomOut}) zoom. The palette ({ShowPalette}), tracks list ({ShowGlobalView}), song settings ({SongInspector}), track settings ({TrackInspector}) and full screen ({FullScreen}) can be shown or hidden, and Multitrack ({Multitrack}) switches between all tracks and one track.",
    ),
];

impl App {
    pub(super) fn export_wav(&mut self) {
        let Some(p) = self.export_path("WAV audio", "wav") else {
            return;
        };
        let samples = crate::audio::render_song(&self.song, 44100.0, self.speed / 100.0);
        self.write_export(&p, crate::fileio::wav_bytes(&samples, 44100));
    }

    /// Adds a text tab file as a new track.
    pub(super) fn import_ascii_dialog(&mut self) {
        if self.screenshot.is_some() {
            return;
        }
        let Some(p) = self
            .file_dialog()
            .add_filter("Text tab", &["txt", "tab"])
            .pick_file()
        else {
            return;
        };
        let text = match std::fs::read(&p) {
            Ok(d) => String::from_utf8_lossy(&d).to_string(),
            Err(e) => {
                self.status = super::notice::failed(format!("Couldn't read the file: {e}"));
                return;
            }
        };
        match crate::textmidi::import_ascii(&text) {
            Ok(mut t) => {
                t.name = p
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or(t.name);
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
            Err(e) => self.status = super::notice::failed(e),
        }
    }

    /// File > Import > MusicXML: .musicxml, .xml or compressed .mxl.
    pub(super) fn import_musicxml_dialog(&mut self) {
        if self.screenshot.is_some() {
            return;
        }
        if let Some(p) = self.file_dialog().add_filter("MusicXML", &["musicxml", "xml", "mxl"]).pick_file() {
            self.open_path(p);
        }
    }

    pub(super) fn import_midi_dialog(&mut self) {
        if self.screenshot.is_some() {
            return;
        }
        if let Some(p) = self
            .file_dialog()
            .add_filter("MIDI", &["mid", "midi"])
            .pick_file()
        {
            self.open_path(p);
        }
    }

    pub(super) fn tool_windows(&mut self, ctx: &egui::Context) {
        self.directions_window(ctx);
        self.add_track_window(ctx);
        self.tuning_window(ctx);
        self.tuner_window(ctx);
        self.chord_window(ctx);
        self.chord_library_window(ctx);
        self.export_window(ctx);
        self.batch_window(ctx);
        self.print_preview_window(ctx);
        self.special_paste_window(ctx);
        self.chord_scales_window(ctx);
        self.lyrics_window(ctx);
        self.command_palette(ctx);
        self.trainer_window(ctx);
        self.guide_window(ctx);
        self.stylesheet_window(ctx);
        self.prefs_window(ctx);
    }

    fn directions_window(&mut self, ctx: &egui::Context) {
        if !self.dir_open {
            return;
        }
        let mut open = true;
        let b = self.bar;
        let mut mb = self.song.master[b].clone();
        egui::Window::new(format!("Directions \u{2013} bar {}", b + 1)).open(&mut open).resizable(false).collapsible(false).show(ctx, |ui| {
            ui.set_min_width(super::widgets::DIALOG_W);
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
        let hits: Vec<(&str, Cmd)> = list
            .into_iter()
            .filter(|(n, _)| {
                q.trim().is_empty()
                    || q == "?"
                    || q.split_whitespace().all(|w| n.to_lowercase().contains(w))
            })
            .collect();
        let mut run: Option<Cmd> = None;
        let mut close = false;
        egui::Window::new("Command palette")
            .title_bar(false)
            .resizable(false)
            .anchor(Align2::CENTER_TOP, Vec2::new(0.0, 90.0))
            .fixed_size(Vec2::new(460.0, 0.0))
            .show(ctx, |ui| {
                let r = ui.add(
                    egui::TextEdit::singleline(&mut self.cmd_text)
                        .hint_text("Type a command, e.g. palm, fermata, add-bar\u{2026}")
                        .desired_width(f32::INFINITY),
                );
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
                egui::ScrollArea::vertical()
                    .max_height(300.0)
                    .show(ui, |ui| {
                        for (k, (name, f)) in hits.iter().enumerate() {
                            let (pre, desc) = name.split_once(": ").unwrap_or((name, ""));
                            let resp = ui.selectable_label(
                                k == self.cmd_sel,
                                RichText::new(format!("{pre}   \u{2014} {desc}")),
                            );
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
            ui.set_min_width(super::widgets::DIALOG_W);
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
        egui::Window::new("GuitarCraft user guide")
            .open(&mut open)
            .default_width(560.0)
            .default_height(520.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (title, body) in GUIDE {
                        ui.label(RichText::new(title).strong().size(15.0));
                        ui.label(super::shortcuts::fill_keys(body));
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
#[test]
fn every_guide_shortcut_names_an_action() {
    for (title, body) in GUIDE {
        let filled = super::shortcuts::fill_keys(body);
        assert!(!filled.contains('{') && !filled.contains('}'), "{title}: an unknown action in {{...}}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chord_names() {
        assert_eq!(
            detect_chord(&[43, 47, 50, 55, 59, 67]).as_deref(),
            Some("G")
        );
        assert_eq!(
            detect_chord(&[40, 47, 52, 55, 59, 64]).as_deref(),
            Some("Em")
        );
        assert_eq!(detect_chord(&[48, 52, 55, 58]).as_deref(), Some("C7"));
    }
}
