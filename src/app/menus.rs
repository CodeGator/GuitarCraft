//! The menu bar, laid out like the original's menus.

use super::popups::Popup;
use super::{App, InspectorTab, Pending};
use crate::model::*;
use crate::score::Notation;
use egui::{Key, Modifiers, Response, RichText, Ui};

fn item(ui: &mut Ui, label: &str, sc: &str) -> Response {
    ui.add(egui::Button::new(label).shortcut_text(sc))
}

fn item_if(ui: &mut Ui, on: bool, label: &str, sc: &str) -> Response {
    ui.add_enabled(on, egui::Button::new(label).shortcut_text(sc))
}

fn check(ui: &mut Ui, on: bool, label: &str, sc: &str) -> Response {
    let text = format!("{}  {}", if on { "\u{2714}" } else { "   " }, label);
    ui.add(egui::Button::new(text).shortcut_text(sc))
}

impl App {
    pub(super) fn menu_bar(&mut self, ui: &mut Ui) {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| self.file_menu(ui));
            ui.menu_button("Edit", |ui| self.edit_menu(ui));
            ui.menu_button("Track", |ui| self.track_menu(ui));
            ui.menu_button("Bar", |ui| self.bar_menu(ui));
            ui.menu_button("Note", |ui| self.note_menu(ui));
            ui.menu_button("Effects", |ui| self.effects_menu(ui));
            ui.menu_button("Section", |ui| self.section_menu(ui));
            ui.menu_button("Tools", |ui| self.tools_menu(ui));
            ui.menu_button("Sound", |ui| self.sound_menu(ui));
            ui.menu_button("View", |ui| self.view_menu(ui));
            ui.menu_button("Window", |ui| self.window_menu(ui));
            ui.menu_button("Help", |ui| self.help_menu(ui));
        });
    }

    fn file_menu(&mut self, ui: &mut Ui) {
        if item(ui, "New", "Ctrl+N").clicked() {
            self.request(Pending::New);
            ui.close();
        }
        ui.menu_button("New from Template", |ui| {
            for (i, name) in super::home::TEMPLATES.iter().enumerate() {
                if ui.button(*name).clicked() {
                    self.request(Pending::Template(i));
                    ui.close();
                }
            }
            let mine = Self::user_templates();
            if !mine.is_empty() {
                ui.separator();
                for p in mine {
                    let name = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                    if ui.button(name).clicked() {
                        if self.dirty {
                            self.status = "Save or undo your changes first.".into();
                        } else {
                            self.open_template_file(p);
                        }
                        ui.close();
                    }
                }
            }
        });
        if item(ui, "Open...", "Ctrl+O").clicked() {
            self.request(Pending::Open);
            ui.close();
        }
        ui.menu_button("Open Recent", |ui| {
            if self.recent.is_empty() {
                ui.label(RichText::new("No recent files").weak());
            }
            for p in self.recent.clone() {
                let name = p.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                if ui.button(name).on_hover_text(p.display().to_string()).clicked() {
                    if self.dirty {
                        self.status = "Save or undo your changes first.".into();
                    } else {
                        self.open_path(p);
                    }
                    ui.close();
                }
            }
        });
        ui.menu_button("Open Example", |ui| {
            if ui.button("Morning Walk (demo song)").clicked() {
                self.request(Pending::Demo);
                ui.close();
            }
        });
        if item(ui, "Browse", "Ctrl+B").clicked() {
            self.show_home = true;
            ui.close();
        }
        ui.separator();
        if item(ui, "Close", "Ctrl+W").clicked() {
            self.request(Pending::Close);
            ui.close();
        }
        if item(ui, "Save", "Ctrl+S").clicked() {
            self.save(false);
            ui.close();
        }
        if item(ui, "Save As...", "Ctrl+Shift+S").clicked() {
            self.save(true);
            ui.close();
        }
        if item(ui, "Save As Template...", "").clicked() {
            self.save_as_template();
            ui.close();
        }
        if item_if(ui, self.path.is_some(), "Open Containing Folder", "").clicked() {
            self.open_containing_folder();
            ui.close();
        }
        ui.separator();
        ui.menu_button("Import", |ui| {
            if ui.button("Guitar Pro file (.gp, .gpx, .gp3\u{2013}.gp5)...").clicked() {
                self.request(Pending::Open);
                ui.close();
            }
            if ui.button("ASCII tab into a new track...").clicked() {
                self.import_ascii_dialog();
                ui.close();
            }
            if ui.button("MIDI...").clicked() {
                self.import_midi_dialog();
                ui.close();
            }
        });
        ui.menu_button("Export", |ui| {
            if ui.button("MIDI...").clicked() {
                self.export_midi();
                ui.close();
            }
            if ui.button("ASCII tab of this track...").clicked() {
                self.export_ascii();
                ui.close();
            }
            if ui.button("Audio (WAV)...").clicked() {
                self.export_wav();
                ui.close();
            }
        });
        ui.separator();
        if item(ui, "Quit", "Alt+F4").clicked() {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            ui.close();
        }
    }

    fn edit_menu(&mut self, ui: &mut Ui) {
        if item_if(ui, !self.undo.is_empty(), "Undo", "Ctrl+Z").clicked() {
            self.do_undo();
            ui.close();
        }
        if item_if(ui, !self.redo.is_empty(), "Redo", "Ctrl+Y").clicked() {
            self.do_redo();
            ui.close();
        }
        ui.separator();
        if item(ui, "Cut", "Ctrl+X").clicked() {
            self.copy();
            self.last_copy_all = false;
            self.delete_at_cursor();
            ui.close();
        }
        if item(ui, "Copy", "Ctrl+C").clicked() {
            self.copy();
            self.last_copy_all = false;
            ui.close();
        }
        if item_if(ui, !self.clipboard.is_empty() || !self.bar_clipboard.is_empty(), "Paste", "Ctrl+V").clicked() {
            self.paste_any();
            ui.close();
        }
        if item(ui, "All-Track Cut", "Ctrl+Shift+X").clicked() {
            self.all_track_cut();
            ui.close();
        }
        if item(ui, "All-Track Copy", "Ctrl+Shift+C").clicked() {
            self.all_track_copy();
            ui.close();
        }
        ui.separator();
        ui.menu_button("Voices", |ui| {
            for v in 0..VOICES {
                if check(ui, self.voice == v, &format!("Voice {}", v + 1), &format!("Ctrl+{}", v + 1)).clicked() {
                    self.set_voice(v);
                    ui.close();
                }
            }
            ui.separator();
            for v in 0..VOICES {
                if item_if(ui, v != self.voice, &format!("Move Note to Voice {}", v + 1), &format!("Alt+{}", v + 1)).clicked() {
                    self.move_to_voice(v);
                    ui.close();
                }
            }
        });
        if item(ui, "Select All", "Ctrl+A").clicked() {
            self.key(Key::A, Modifiers::COMMAND, 0.0);
            ui.close();
        }
        ui.separator();
        ui.menu_button("Automations", |ui| {
            if item(ui, "Edit Automations", "F10").clicked() {
                self.open_automations(self.bar);
                ui.close();
            }
        });
        if item(ui, "Go to...", "Ctrl+G").clicked() {
            self.goto_text = format!("{}", self.bar + 1);
            self.goto_open = true;
            ui.close();
        }
    }

    fn track_menu(&mut self, ui: &mut Ui) {
        if item(ui, "Add...", "Ctrl+Shift+Ins").clicked() {
            self.addtrack_open = true;
            ui.close();
        }
        if item_if(ui, self.song.tracks.len() > 1, "Delete", "Ctrl+Shift+Del").clicked() {
            self.remove_track();
            ui.close();
        }
        if item(ui, "Duplicate", "").clicked() {
            self.duplicate_track();
            ui.close();
        }
        ui.separator();
        if item_if(ui, self.track > 0, "Move Up", "Ctrl+Alt+Up").clicked() {
            self.move_track(-1);
            ui.close();
        }
        if item_if(ui, self.track + 1 < self.song.tracks.len(), "Move Down", "Ctrl+Alt+Down").clicked() {
            self.move_track(1);
            ui.close();
        }
        ui.separator();
        if item(ui, "Show Lyrics Editor", "").clicked() {
            self.lyrics_open = true;
            ui.close();
        }
        if item(ui, "Track Settings...", "").clicked() {
            self.inspector = InspectorTab::Track;
            self.show_inspector = true;
            ui.close();
        }
        ui.separator();
        for i in 0..self.song.tracks.len() {
            let name = format!("{} {}", i + 1, self.song.tracks[i].name);
            if check(ui, self.track == i, &name, "").clicked() {
                self.track = i;
                self.clamp_cursor();
                ui.close();
            }
        }
        ui.separator();
        if item_if(ui, self.track + 1 < self.song.tracks.len(), "Next Track", "Ctrl+Down").clicked() {
            self.track += 1;
            self.clamp_cursor();
            ui.close();
        }
        if item_if(ui, self.track > 0, "Previous Track", "Ctrl+Up").clicked() {
            self.track -= 1;
            self.clamp_cursor();
            ui.close();
        }
    }

    fn bar_menu(&mut self, ui: &mut Ui) {
        let mb = self.song.master[self.bar].clone();
        if item(ui, "Insert Bar", "Ctrl+Ins").clicked() {
            self.insert_bar(self.bar);
            ui.close();
        }
        if item_if(ui, self.song.bar_count() > 1, "Delete Bar", "Ctrl+Del").clicked() {
            self.delete_bar(self.bar);
            ui.close();
        }
        ui.separator();
        if item(ui, "Key Signature...", "Ctrl+K").clicked() {
            self.open_popup(Popup::KeySig);
            ui.close();
        }
        if item(ui, "Time Signature...", "Ctrl+T").clicked() {
            self.open_popup(Popup::TimeSig);
            ui.close();
        }
        if item(ui, "Tempo...", "F10").clicked() {
            self.open_automations(self.bar);
            ui.close();
        }
        if check(ui, mb.triplet_feel, "Triplet Feel", "Ctrl+/").clicked() {
            self.bar_edit(|m| m.triplet_feel = !m.triplet_feel);
            ui.close();
        }
        if check(ui, mb.double_bar, "Double Barline", "").clicked() {
            self.bar_edit(|m| m.double_bar = !m.double_bar);
            ui.close();
        }
        ui.separator();
        if check(ui, mb.repeat_open, "Repeat Open", "[").clicked() {
            self.bar_edit(|m| m.repeat_open = !m.repeat_open);
            ui.close();
        }
        if check(ui, mb.endings != 0, "Alternate Endings...", "").clicked() {
            self.open_popup(Popup::Endings);
            ui.close();
        }
        if check(ui, mb.repeat_close > 0, "Repeat Close...", "]").clicked() {
            self.open_popup(Popup::Repeat);
            ui.close();
        }
        if check(ui, mb.direction_target != 0 || mb.direction_jump != 0, "Directions...", "D").clicked() {
            self.dir_open = true;
            ui.close();
        }
        ui.separator();
        if check(ui, mb.line_break, "Force Break Line", "Ctrl+Return").clicked() {
            self.bar_edit(|m| m.line_break = !m.line_break);
            ui.close();
        }
    }

    fn note_menu(&mut self, ui: &mut Ui) {
        let cur = self.cur_beat().clone();
        let has = !cur.is_rest();
        let n = cur.note_on(self.string as u8).or(cur.notes.first()).cloned().unwrap_or_default();
        if item(ui, "Insert a Beat", "Ins").clicked() {
            self.insert_beat();
            ui.close();
        }
        if item(ui, "Delete the Beats", "Shift+Del").clicked() {
            self.delete_beats();
            ui.close();
        }
        ui.separator();
        ui.menu_button("Duration", |ui| {
            for d in Dur::ALL {
                if check(ui, cur.dur == d, d.name(), "").clicked() {
                    self.set_duration(d);
                    ui.close();
                }
            }
            ui.separator();
            if check(ui, cur.dotted, "Dotted", ".").clicked() {
                self.beat_edit(|b| {
                    b.dotted = !b.dotted;
                    b.double_dot = false;
                });
            }
            if check(ui, cur.double_dot, "Double Dotted", "").clicked() {
                self.beat_edit(|b| {
                    b.double_dot = !b.double_dot;
                    b.dotted = false;
                });
            }
            ui.separator();
            for k in [3u8, 5, 6, 7, 9] {
                if check(ui, cur.tuplet == k, &format!("Tuplet {k}"), "").clicked() {
                    self.beat_edit(|b| b.tuplet = if b.tuplet == k { 0 } else { k });
                }
            }
        });
        ui.menu_button("Dynamic", |ui| {
            for d in Dynamic::ALL {
                if check(ui, cur.dynamic == d, d.label(), "").clicked() {
                    self.beat_edit(|b| b.dynamic = d);
                    ui.close();
                }
            }
        });
        ui.separator();
        ui.add_enabled_ui(has, |ui| {
            if check(ui, n.ghost, "Ghost Note", "O").clicked() {
                self.toggle_note_flag("Ghost note", |n| &mut n.ghost);
                ui.close();
            }
            if check(ui, n.accent, "Accented Note", ";").clicked() {
                self.toggle_note_flag("Accent", |n| &mut n.accent);
                ui.close();
            }
            if check(ui, n.heavy_accent, "Heavily Accented Note", ":").clicked() {
                self.toggle_note_flag("Heavy accent", |n| &mut n.heavy_accent);
                ui.close();
            }
            if check(ui, n.staccato, "Staccato", "!").clicked() {
                self.toggle_note_flag("Staccato", |n| &mut n.staccato);
                ui.close();
            }
            ui.separator();
            if check(ui, n.tie, "Tie Note", "L").clicked() {
                self.toggle_note_flag("Tie", |n| &mut n.tie);
                ui.close();
            }
        });
        if item(ui, "Rest", "R").clicked() {
            self.key(Key::R, Modifiers::NONE, 0.0);
            ui.close();
        }
        if check(ui, cur.fermata, "Fermata", "F").clicked() {
            self.beat_edit(|b| b.fermata = !b.fermata);
            ui.close();
        }
        ui.separator();
        ui.add_enabled_ui(has, |ui| {
            ui.menu_button("Accidentals", |ui| {
                for (v, l) in [(0u8, "Automatic"), (1, "Sharps (\u{266F})"), (2, "Flats (\u{266D})")] {
                    if check(ui, n.spell == v, l, "").clicked() {
                        self.note_edit(|x| x.spell = v);
                        ui.close();
                    }
                }
            });
            if item(ui, "One Semitone Down", "Alt+Shift+Down").clicked() {
                self.transpose(-1);
                ui.close();
            }
            if item(ui, "One Semitone Up", "Alt+Shift+Up").clicked() {
                self.transpose(1);
                ui.close();
            }
            if item(ui, "One Octave Down", "Alt+Shift+PgDown").clicked() {
                self.shift_octave(-1);
                ui.close();
            }
            if item(ui, "One Octave Up", "Alt+Shift+PgUp").clicked() {
                self.shift_octave(1);
                ui.close();
            }
        });
        ui.separator();
        if check(ui, cur.pick == 1, "Pickstroke Down", "Shift+D").clicked() {
            self.beat_edit(|b| b.pick = if b.pick == 1 { 0 } else { 1 });
            ui.close();
        }
        if check(ui, cur.pick == 2, "Pickstroke Up", "Shift+U").clicked() {
            self.beat_edit(|b| b.pick = if b.pick == 2 { 0 } else { 2 });
            ui.close();
        }
        ui.separator();
        if item(ui, "Chord...", "A").clicked() {
            self.open_chord_tool();
            ui.close();
        }
        if item(ui, "Text...", "T").clicked() {
            self.open_popup(Popup::Text);
            ui.close();
        }
        ui.separator();
        ui.menu_button("Octave Sign", |ui| {
            for (v, l) in [(0i8, "None"), (1, "8va"), (2, "15ma"), (-1, "8vb"), (-2, "15mb")] {
                if check(ui, cur.ottava == v, l, "").clicked() {
                    self.beat_edit(|b| b.ottava = v);
                    ui.close();
                }
            }
        });
    }

    fn effects_menu(&mut self, ui: &mut Ui) {
        let cur = self.cur_beat().clone();
        let has = !cur.is_rest();
        let n = cur.note_on(self.string as u8).or(cur.notes.first()).cloned().unwrap_or_default();
        ui.add_enabled_ui(has, |ui| {
            ui.menu_button("Grace Note", |ui| {
                if check(ui, n.grace.is_none(), "None", "").clicked() {
                    self.note_edit(|x| x.grace = None);
                    ui.close();
                }
                if check(ui, n.grace.is_some(), "Before the Beat", "").clicked() {
                    self.note_edit(|x| x.grace = Some(x.fret.saturating_sub(2)));
                    ui.close();
                }
            });
            if item(ui, "Grace Note Fret...", "").clicked() {
                self.open_popup(Popup::Grace);
                ui.close();
            }
            if check(ui, n.trill > 0, "Trill...", "N").clicked() {
                self.open_popup(Popup::Trill);
                ui.close();
            }
        });
        ui.menu_button("Tremolo", |ui| {
            if check(ui, cur.tremolo, "Tremolo Picking", "").clicked() {
                self.beat_edit(|b| b.tremolo = !b.tremolo);
                ui.close();
            }
        });
        ui.separator();
        ui.add_enabled_ui(has, |ui| {
            if check(ui, n.let_ring, "Let Ring", "I").clicked() {
                self.toggle_note_flag("Let ring", |n| &mut n.let_ring);
                ui.close();
            }
            if check(ui, n.hammer, "Hammer On / Pull Off", "H").clicked() {
                self.toggle_note_flag("Hammer-on / pull-off", |n| &mut n.hammer);
                ui.close();
            }
        });
        if check(ui, cur.technique == 1, "Tapping", ")").clicked() {
            self.beat_edit(|b| b.technique = if b.technique == 1 { 0 } else { 1 });
            ui.close();
        }
        ui.separator();
        if check(ui, cur.technique == 2, "Slap", "$").clicked() {
            self.beat_edit(|b| b.technique = if b.technique == 2 { 0 } else { 2 });
            ui.close();
        }
        if check(ui, cur.technique == 3, "Pop", "").clicked() {
            self.beat_edit(|b| b.technique = if b.technique == 3 { 0 } else { 3 });
            ui.close();
        }
        ui.separator();
        ui.add_enabled_ui(has, |ui| {
            if check(ui, n.dead, "Dead Note", "X").clicked() {
                self.toggle_note_flag("Dead note", |n| &mut n.dead);
                ui.close();
            }
            if check(ui, n.palm_mute, "Palm Mute on Note", "P").clicked() {
                self.toggle_note_flag("Palm mute", |n| &mut n.palm_mute);
                ui.close();
            }
            ui.separator();
            if check(ui, n.bend > 0, "Bend...", "B").clicked() {
                self.open_popup(Popup::Bend);
                ui.close();
            }
            ui.menu_button("Slide", |ui| {
                if check(ui, n.slide, "Legato Slide", "S").clicked() {
                    self.toggle_note_flag("Slide", |n| &mut n.slide);
                    ui.close();
                }
                if check(ui, n.slide_in, "Slide In from Below", "").clicked() {
                    self.toggle_note_flag("Slide in", |n| &mut n.slide_in);
                    ui.close();
                }
                if check(ui, n.slide_out, "Slide Out Downwards", "").clicked() {
                    self.toggle_note_flag("Slide out", |n| &mut n.slide_out);
                    ui.close();
                }
            });
            ui.menu_button("Vibrato", |ui| {
                if check(ui, n.vibrato, "Slight", "V").clicked() {
                    self.toggle_note_flag("Vibrato", |n| &mut n.vibrato);
                    ui.close();
                }
                if check(ui, n.wide_vibrato, "Wide", "").clicked() {
                    self.toggle_note_flag("Wide vibrato", |n| &mut n.wide_vibrato);
                    ui.close();
                }
            });
            ui.separator();
            if check(ui, n.harmonic, "Natural Harmonic", "Y").clicked() {
                self.toggle_note_flag("Natural harmonic", |n| &mut n.harmonic);
                ui.close();
            }
            if check(ui, n.pinch_harmonic, "Artificial Harmonic", "Ctrl+Alt+Y").clicked() {
                self.toggle_note_flag("Artificial harmonic", |n| &mut n.pinch_harmonic);
                ui.close();
            }
        });
        ui.separator();
        if check(ui, cur.stroke == Stroke::Down, "Brush Downstroke", "Ctrl+D").clicked() {
            self.beat_edit(|b| b.stroke = if b.stroke == Stroke::Down { Stroke::None } else { Stroke::Down });
            ui.close();
        }
        if check(ui, cur.stroke == Stroke::Up, "Brush Upstroke", "Ctrl+U").clicked() {
            self.beat_edit(|b| b.stroke = if b.stroke == Stroke::Up { Stroke::None } else { Stroke::Up });
            ui.close();
        }
        ui.separator();
        if check(ui, cur.fade_in, "Fade In", "<").clicked() {
            self.beat_edit(|b| b.fade_in = !b.fade_in);
            ui.close();
        }
    }

    fn section_menu(&mut self, ui: &mut Ui) {
        let has = self.song.master[self.bar].marker.is_some();
        if check(ui, has, "Edit...", "Shift+Ins").clicked() {
            self.edit_section();
            ui.close();
        }
        if has && item(ui, "Remove", "").clicked() {
            self.bar_edit(|m| m.marker = None);
            ui.close();
        }
        ui.separator();
        if item(ui, "Previous Section", "Ctrl+Alt+Left").clicked() {
            self.goto_section(-1);
            ui.close();
        }
        if item(ui, "Next Section", "Ctrl+Alt+Right").clicked() {
            self.goto_section(1);
            ui.close();
        }
        ui.separator();
        let marks: Vec<(usize, String)> = self.song.master.iter().enumerate().filter_map(|(i, m)| m.marker.clone().map(|n| (i, n))).collect();
        if marks.is_empty() {
            ui.label(RichText::new("No sections yet").weak());
        }
        for (i, n) in marks {
            if ui.button(n).clicked() {
                self.goto_bar(i);
                ui.close();
            }
        }
    }

    fn tools_menu(&mut self, ui: &mut Ui) {
        if item(ui, "Scales...", "").clicked() {
            self.scale_open = true;
            ui.close();
        }
        if item(ui, "Transpose...", "").clicked() {
            self.transpose_open = true;
            ui.close();
        }
        ui.separator();
        if item(ui, "Move/Swap Voices...", "Ctrl+Shift+M").clicked() {
            self.vfrom = self.voice;
            self.vto = if self.voice == 0 { 1 } else { 0 };
            self.vrange = if self.selection().is_some() { super::voices::VoiceRange::Selection } else { super::voices::VoiceRange::Bar };
            self.voices_open = true;
            ui.close();
        }
        if item(ui, "Check Bar Duration", "F4").clicked() {
            self.check_bar_duration();
            ui.close();
        }
        if item(ui, "Complete Bars with Rests", "").clicked() {
            self.complete_bars();
            ui.close();
        }
        ui.separator();
        if item(ui, "Show Command Palette", "Ctrl+E").clicked() {
            self.cmd_open = true;
            self.cmd_text.clear();
            ui.close();
        }
    }

    fn sound_menu(&mut self, ui: &mut Ui) {
        if item(ui, if self.playing { "Pause" } else { "Play" }, "Space").clicked() {
            self.toggle_play();
            ui.close();
        }
        if item(ui, "Play from the Beginning", "Ctrl+Space").clicked() {
            self.play_from_start();
            ui.close();
        }
        if item(ui, "Stop", "Esc").clicked() {
            self.stop();
            ui.close();
        }
        ui.separator();
        if item(ui, "First Bar", "Ctrl+Home").clicked() {
            self.stop();
            self.goto_bar(0);
            ui.close();
        }
        if item(ui, "Rewind", "Ctrl+Left").clicked() {
            self.goto_bar(self.bar.saturating_sub(1));
            ui.close();
        }
        if item(ui, "Fast Forward", "Ctrl+Right").clicked() {
            self.goto_bar(self.bar + 1);
            ui.close();
        }
        if item(ui, "Last Bar", "Ctrl+End").clicked() {
            self.stop();
            self.goto_bar(self.song.bar_count() - 1);
            ui.close();
        }
        ui.separator();
        if item(ui, "Step Backward", "Alt+Left").clicked() {
            self.step(-1);
            ui.close();
        }
        if item(ui, "Step Forward", "Alt+Right").clicked() {
            self.step(1);
            ui.close();
        }
        ui.separator();
        if check(ui, self.looping, "Loop", "F9").clicked() {
            self.looping = !self.looping;
        }
        if check(ui, self.count_in, "Count-in", "").clicked() {
            self.count_in = !self.count_in;
        }
        if check(ui, self.metronome, "Metronome", "").clicked() {
            self.metronome = !self.metronome;
        }
        ui.separator();
        if check(ui, self.play_while_editing, "Play While Editing", "").clicked() {
            self.play_while_editing = !self.play_while_editing;
        }
        ui.separator();
        ui.menu_button("Relative Speed", |ui| {
            for v in [25.0f32, 50.0, 60.0, 70.0, 80.0, 90.0, 100.0] {
                if check(ui, (self.speed - v).abs() < 0.5, &format!("{}%", v as i32), "").clicked() {
                    self.speed = v;
                    ui.close();
                }
            }
        });
        if item(ui, "Speed Trainer...", "").clicked() {
            self.trainer_open = true;
            ui.close();
        }
        ui.add(egui::Slider::new(&mut self.speed, 10.0..=300.0).suffix("%").text("Speed"));
        ui.add(egui::Slider::new(&mut self.master_volume, 0.0..=1.5).text("Volume"));
    }

    fn view_menu(&mut self, ui: &mut Ui) {
        if check(ui, self.show_palette, "Show Editing Palette", "F2").clicked() {
            self.show_palette = !self.show_palette;
        }
        if check(ui, self.show_inspector, "Show Inspector", "").clicked() {
            self.show_inspector = !self.show_inspector;
        }
        if check(ui, self.show_tracks, "Show Global View", "F8").clicked() {
            self.show_tracks = !self.show_tracks;
        }
        ui.separator();
        if check(ui, self.show_fretboard, "Show Fretboard View", "").clicked() {
            self.show_fretboard = !self.show_fretboard;
        }
        if check(ui, self.show_keyboard, "Show Keyboard View", "Ctrl+F6").clicked() {
            self.show_keyboard = !self.show_keyboard;
        }
        ui.separator();
        if check(ui, !self.single_track, "Multitrack", "F3").clicked() {
            self.single_track = !self.single_track;
        }
        ui.menu_button("Notation", |ui| {
            for (v, l) in [(Notation::Both, "Standard + Tablature"), (Notation::TabOnly, "Tablature Only"), (Notation::StandardOnly, "Standard Only")] {
                if check(ui, self.notation == v, l, "").clicked() {
                    self.notation = v;
                    ui.close();
                }
            }
        });
        if check(ui, self.follow, "Follow Playback", "").clicked() {
            self.follow = !self.follow;
        }
        ui.separator();
        if item(ui, "Zoom In", "Ctrl++").clicked() {
            self.zoom_step(1);
        }
        if item(ui, "Zoom Out", "Ctrl+-").clicked() {
            self.zoom_step(-1);
        }
        ui.separator();
        for (v, l) in [(0u8, "Page - Vertical"), (1, "Page - Horizontal"), (2, "Page - Grid"), (3, "Screen - Vertical"), (4, "Screen - Horizontal")] {
            if check(ui, self.view_mode == v, l, "").clicked() {
                self.view_mode = v;
                ui.close();
            }
        }
        ui.separator();
        if check(ui, self.fullscreen, "Full Screen Mode", "F11").clicked() {
            self.toggle_fullscreen();
            ui.close();
        }
    }

    fn window_menu(&mut self, ui: &mut Ui) {
        let title = if self.song.title.is_empty() { "Untitled".to_string() } else { self.song.title.clone() };
        let name = if self.song.artist.is_empty() { title } else { format!("{} - {}", self.song.artist, title) };
        if check(ui, !self.show_home, &format!("1 {name}"), "").clicked() {
            self.show_home = false;
            ui.close();
        }
        if check(ui, self.show_home, "Start Screen", "").clicked() {
            self.show_home = true;
            ui.close();
        }
    }

    fn help_menu(&mut self, ui: &mut Ui) {
        if item(ui, "About GuitarCraft", "").clicked() {
            self.show_about = true;
            ui.close();
        }
        if item(ui, "User Guide...", "").clicked() {
            self.guide_open = true;
            ui.close();
        }
        if item(ui, "Keyboard Shortcuts", "").clicked() {
            self.show_help = true;
            ui.close();
        }
        if item(ui, "Help Center...", "F1").clicked() {
            self.guide_open = true;
            ui.close();
        }
    }
}
