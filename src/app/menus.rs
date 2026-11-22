//! The menu bar, laid out like the original's menus.

use super::dock::Pane;
use super::popups::Popup;
use super::{App, InspectorTab, Pending};
use crate::model::*;
use crate::score::Notation;
use super::shortcuts::{Act, sc};
use egui::{Key, Modifiers, Response, RichText, Ui};

fn item(ui: &mut Ui, label: &str, sc: &str) -> Response {
    ui.add(egui::Button::new(label).shortcut_text(sc))
}

/// A menu item that can be greyed out; true when clicked.
pub(super) fn item_if(ui: &mut Ui, on: bool, label: &str, sc: &str) -> bool {
    ui.add_enabled(on, egui::Button::new(label).shortcut_text(sc)).clicked()
}

fn check(ui: &mut Ui, on: bool, label: &str, sc: &str) -> Response {
    let text = format!("{}  {}", if on { "\u{2714}" } else { "   " }, label);
    ui.add(egui::Button::new(text).shortcut_text(sc))
}

impl App {
    pub(super) fn menu_bar(&mut self, ui: &mut Ui) {
        egui::MenuBar::new().ui(ui, |ui| {
            let file = ui.menu_button("File", |ui| self.file_menu(ui));
            if self.file_menu_demo.is_some() {
                // Pictures for checks: the File menu shown open (with Import or Export).
                let at = file.response.rect.left_bottom();
                egui::Area::new(egui::Id::new("file-menu-demo")).order(egui::Order::Foreground).fixed_pos(at).show(ui.ctx(), |ui| {
                    egui::Frame::menu(ui.style()).show(ui, |ui| {
                        ui.set_min_width(230.0);
                        ui.style_mut().visuals.button_frame = false;
                        self.file_menu(ui);
                    });
                });
            }
            // The start screen (Browse) has no song to work on: only File (open and
            // import), Tools (its song-free tools) and Help are available.
            let home = self.show_home;
            ui.add_enabled_ui(!home, |ui| {
                ui.menu_button("Edit", |ui| self.edit_menu(ui));
                ui.menu_button("Track", |ui| self.track_menu(ui));
                ui.menu_button("Bar", |ui| self.bar_menu(ui));
                ui.menu_button("Note", |ui| self.note_menu(ui));
                ui.menu_button("Effects", |ui| self.effects_menu(ui));
                ui.menu_button("Section", |ui| self.section_menu(ui));
            });
            ui.menu_button("Tools", |ui| self.tools_menu(ui));
            ui.add_enabled_ui(!home, |ui| {
                ui.menu_button("Sound", |ui| self.sound_menu(ui));
                ui.menu_button("View", |ui| self.view_menu(ui));
                ui.menu_button("Window", |ui| self.window_menu(ui));
            });
            ui.menu_button("Help", |ui| self.help_menu(ui));
        });
    }

    fn file_menu(&mut self, ui: &mut Ui) {
        // On the start screen there is no open song: saving, closing, exporting,
        // printing and the stylesheet are greyed out.
        let song = !self.show_home;
        if item(ui, "New", &sc(Act::New)).clicked() {
            self.request(Pending::New);
            ui.close();
        }
        if item(ui, "New Chord Chart", &sc(Act::NewChart)).clicked() {
            self.request(Pending::NewChart);
            ui.close();
        }
        ui.menu_button("New from Template", |ui| {
            for (i, name) in super::home::TEMPLATES.iter().enumerate() {
                if ui.button(*name).clicked() {
                    self.request(Pending::Template(i));
                    ui.close();
                }
            }
            // Your own templates (File > Save As Template).
            ui.separator();
            ui.label(RichText::new("My Templates").weak().size(11.5));
            let mine = Self::user_templates();
            if mine.is_empty() {
                ui.label(RichText::new("None yet: use File > Save As Template").weak().size(11.5));
            }
            for p in mine {
                let name = p
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default();
                if ui.button(name).clicked() {
                    self.open_template_file(p);
                    ui.close();
                }
            }
            ui.separator();
            if ui.button("Open My Templates Folder").clicked() {
                self.open_templates_folder();
                ui.close();
            }
        });
        if item(ui, "Open...", &sc(Act::Open)).clicked() {
            self.request(Pending::Open);
            ui.close();
        }
        ui.menu_button("Open Recent", |ui| {
            if self.recent.is_empty() {
                ui.label(RichText::new("No recent files").weak());
            }
            for p in self.recent.clone() {
                let name = p
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default();
                if ui
                    .button(name)
                    .on_hover_text(p.display().to_string())
                    .clicked()
                {
                    self.open_path(p);
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
        if item(ui, "Browse", &sc(Act::Browse)).clicked() {
            self.show_home = true;
            ui.close();
        }
        ui.separator();
        if item_if(ui, !self.tabs.is_empty() && song, "Close All", &sc(Act::CloseAll)) {
            self.request(Pending::CloseAll);
            ui.close();
        }
        if item_if(ui, song, "Close", &sc(Act::Close)) {
            self.request(Pending::Close);
            ui.close();
        }
        if item_if(ui, song, "Save", &sc(Act::Save)) {
            self.save(false);
            ui.close();
        }
        if item_if(ui, song, "Save As...", &sc(Act::SaveAs)) {
            self.save(true);
            ui.close();
        }
        if item_if(ui, song, "Save As Template...", "") {
            self.save_as_template();
            ui.close();
        }
        if item_if(ui, song && self.path.is_some(), "Open Containing Folder", "") {
            self.open_containing_folder();
            ui.close();
        }
        ui.separator();
        // Import, Export and Batch Converter, in Guitar Pro 8's order. (Scores only:
        // a chord chart has its own File menu.)
        let demo = self.file_menu_demo;
        let import = ui.menu_button("Import", |ui| self.import_menu(ui));
        let export = ui.add_enabled_ui(song, |ui| ui.menu_button("Export", |ui| self.export_menu(ui))).inner;
        if let Some(e) = demo {
            // Pictures for checks: the submenu shown open.
            let r = if e { export.response.rect } else { import.response.rect };
            egui::Area::new(egui::Id::new("file-sub-demo")).order(egui::Order::Foreground).fixed_pos(r.right_top()).show(ui.ctx(), |ui| {
                egui::Frame::menu(ui.style()).show(ui, |ui| {
                    ui.set_min_width(140.0);
                    ui.style_mut().visuals.button_frame = false;
                    if e {
                        self.export_menu(ui)
                    } else {
                        self.import_menu(ui)
                    }
                });
            });
        }
        if item(ui, "Batch Converter...", "").clicked() {
            self.open_batch();
            ui.close();
        }
        ui.separator();
        if item_if(ui, song, "Print...", &sc(Act::Print)) {
            ui.close();
            self.print_song();
        }
        ui.separator();
        if item_if(ui, song, "Stylesheet...", &sc(Act::Stylesheet)) {
            self.open_stylesheet();
            ui.close();
        }
        if item(ui, "Preferences...", &sc(Act::Prefs)).clicked() {
            self.open_prefs();
            ui.close();
        }
        ui.separator();
        if item(ui, "Quit", &sc(Act::Quit)).clicked() {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            ui.close();
        }
    }

    /// File > Import: song files from other programs, as in Guitar Pro 8.
    fn import_menu(&mut self, ui: &mut Ui) {
        if ui.button("MIDI...").clicked() {
            self.import_midi_dialog();
            ui.close();
        }
        if ui.button("MusicXML...").on_hover_text(".musicxml, .xml or compressed .mxl").clicked() {
            self.import_musicxml_dialog();
            ui.close();
        }
        if ui.button("ASCII...").on_hover_text("Text tab, added to the song as a new track").clicked() {
            self.import_ascii_dialog();
            ui.close();
        }
        for (name, why) in [("TablEdit...", "TablEdit"), ("PowerTab...", "Power Tab")] {
            ui.add_enabled(false, egui::Button::new(name))
                .on_disabled_hover_text(format!("Not available: the {why} file format isn't publicly documented, so GuitarCraft can't read it yet."));
        }
    }

    /// File > Export: the song in other formats, as in Guitar Pro 8.
    fn export_menu(&mut self, ui: &mut Ui) {
        if ui.button("GPX...").on_hover_text("Guitar Pro 6").clicked() {
            self.export_gpx();
            ui.close();
        }
        if ui.button("GP5...").on_hover_text("Guitar Pro 5").clicked() {
            self.export_gp5();
            ui.close();
        }
        if ui.button("MIDI...").clicked() {
            self.export_midi();
            ui.close();
        }
        if ui.button("ASCII...").on_hover_text("Text tab of the selected track").clicked() {
            self.export_ascii();
            ui.close();
        }
        if ui.button("MusicXML...").clicked() {
            self.export_musicxml();
            ui.close();
        }
        if ui.button("PDF...").clicked() {
            self.open_export(super::printing::ExportKind::Pdf);
            ui.close();
        }
        if ui.button("SVG...").clicked() {
            self.open_export(super::printing::ExportKind::Svg);
            ui.close();
        }
        if ui.button("PNG...").clicked() {
            self.open_export(super::printing::ExportKind::Png);
            ui.close();
        }
        if ui.button("Audio...").on_hover_text("A WAV recording of the song, played by GuitarCraft").clicked() {
            self.export_wav();
            ui.close();
        }
    }

    fn edit_menu(&mut self, ui: &mut Ui) {
        if item_if(ui, !self.undo.is_empty(), "Undo", &sc(Act::Undo)) {
            self.do_undo();
            ui.close();
        }
        if item_if(ui, !self.redo.is_empty(), "Redo", &sc(Act::Redo)) {
            self.do_redo();
            ui.close();
        }
        ui.separator();
        self.clipboard_items(ui);
        ui.separator();
        ui.menu_button("Voices", |ui| {
            for v in 0..VOICES {
                if check(
                    ui,
                    self.voice == v,
                    &format!("Voice {}", v + 1),
                    &sc([Act::Voice1, Act::Voice2, Act::Voice3, Act::Voice4][v]),
                )
                .clicked()
                {
                    self.set_voice(v);
                    ui.close();
                }
            }
            ui.separator();
            for v in 0..VOICES {
                if item_if(
                    ui,
                    v != self.voice,
                    &format!("Move Note to Voice {}", v + 1),
                    &sc([Act::MoveToVoice1, Act::MoveToVoice2, Act::MoveToVoice3, Act::MoveToVoice4][v]),
                )
                {
                    self.move_to_voice(v);
                    ui.close();
                }
            }
        });
        if item(ui, "Select All", &sc(Act::SelectAll)).clicked() {
            self.key(Key::A, Modifiers::COMMAND, 0.0);
            ui.close();
        }
        ui.separator();
        ui.menu_button("Automations", |ui| {
            if item(ui, "Edit Automations", &sc(Act::Automations)).clicked() {
                self.open_automations(self.bar);
                ui.close();
            }
        });
        if item(ui, "Go to...", &sc(Act::GoTo)).clicked() {
            self.goto_text = format!("{}", self.bar + 1);
            self.goto_open = true;
            ui.close();
        }
    }

    fn track_menu(&mut self, ui: &mut Ui) {
        if item(ui, "Add...", &sc(Act::AddTrack)).clicked() {
            self.open_add_track(false);
            ui.close();
        }
        if item_if(ui, self.song.tracks.len() > 1, "Delete", &sc(Act::DeleteTrack)) {
            self.remove_track();
            ui.close();
        }
        if item(ui, "Duplicate", "").clicked() {
            self.duplicate_track();
            ui.close();
        }
        ui.separator();
        if item_if(ui, self.track > 0, "Move Up", "") {
            self.move_track(-1);
            ui.close();
        }
        if item_if(
            ui,
            self.track + 1 < self.song.tracks.len(),
            "Move Down",
            "",
        )
        {
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
            self.show_panel(Pane::Inspector);
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
        if item_if(
            ui,
            self.track + 1 < self.song.tracks.len(),
            "Next Track",
            &sc(Act::NextTrack),
        )
        {
            self.track += 1;
            self.clamp_cursor();
            ui.close();
        }
        if item_if(ui, self.track > 0, "Previous Track", &sc(Act::PrevTrack)) {
            self.track -= 1;
            self.clamp_cursor();
            ui.close();
        }
    }

    pub(super) fn bar_menu(&mut self, ui: &mut Ui) {
        #[cfg(test)]
        super::flows::MENU_ENABLED.with(|c| c.set(ui.is_enabled()));
        let mb = self.song.master[self.bar].clone();
        if item(ui, "Insert Bar", &sc(Act::InsertBar)).clicked() {
            self.insert_bar(self.bar);
            ui.close();
        }
        if item_if(ui, self.song.bar_count() > 1, "Delete Bar", &sc(Act::DeleteBar)) {
            self.delete_bar(self.bar);
            ui.close();
        }
        if item(ui, "Multirest", &sc(Act::Multirest)).clicked() {
            self.toggle_multirest();
            ui.close();
        }
        ui.separator();
        if item(ui, "Key Signature...", &sc(Act::KeySig)).clicked() {
            self.open_popup(Popup::KeySig);
            ui.close();
        }
        if item(ui, "Time Signature...", &sc(Act::TimeSig)).clicked() {
            self.open_popup(Popup::TimeSig);
            ui.close();
        }
        if item(ui, "Tempo...", &sc(Act::Automations)).clicked() {
            self.open_automations(self.bar);
            ui.close();
        }
        if check(ui, mb.triplet_feel, "Triplet Feel", &sc(Act::TripletFeel)).clicked() {
            self.bar_edit(|m| m.triplet_feel = !m.triplet_feel);
            ui.close();
        }
        if check(ui, mb.double_bar, "Double Barline", "").clicked() {
            self.bar_edit(|m| m.double_bar = !m.double_bar);
            ui.close();
        }
        ui.separator();
        if check(ui, mb.repeat_open, "Repeat Open", &sc(Act::RepeatOpen)).clicked() {
            self.bar_edit(|m| m.repeat_open = !m.repeat_open);
            ui.close();
        }
        if check(ui, mb.endings != 0, "Alternate Endings...", "").clicked() {
            self.open_popup(Popup::Endings);
            ui.close();
        }
        if check(ui, mb.repeat_close > 0, "Repeat Close...", &sc(Act::RepeatClose)).clicked() {
            self.open_popup(Popup::Repeat);
            ui.close();
        }
        if check(
            ui,
            mb.direction_target != 0 || mb.direction_jump != 0,
            "Directions...",
            "D",
        )
        .clicked()
        {
            self.dir_open = true;
            ui.close();
        }
        ui.separator();
        if check(ui, mb.line_break, "Force Break Line", &sc(Act::ForceBreak)).clicked() {
            self.bar_edit(|m| m.line_break = !m.line_break);
            ui.close();
        }
    }

    pub(super) fn note_menu(&mut self, ui: &mut Ui) {
        let cur = self.cur_beat().clone();
        let has = !cur.is_rest();
        let n = cur
            .note_on(self.string as u8)
            .or(cur.notes.first())
            .cloned()
            .unwrap_or_default();
        if item(ui, "Insert a Beat", &sc(Act::InsertBeat)).clicked() {
            self.insert_beat();
            ui.close();
        }
        if item(ui, "Delete the Beats", &sc(Act::DeleteBeats)).clicked() {
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
            if check(ui, cur.dotted, "Dotted", &sc(Act::Dotted)).clicked() {
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
            if check(ui, n.ghost, "Ghost Note", &sc(Act::Ghost)).clicked() {
                self.toggle_note_flag("Ghost note", |n| &mut n.ghost);
                ui.close();
            }
            if check(ui, n.accent, "Accented Note", &sc(Act::Accent)).clicked() {
                self.toggle_note_flag("Accent", |n| &mut n.accent);
                ui.close();
            }
            if check(ui, n.heavy_accent, "Heavily Accented Note", &sc(Act::HeavyAccent)).clicked() {
                self.toggle_note_flag("Heavy accent", |n| &mut n.heavy_accent);
                ui.close();
            }
            if check(ui, n.staccato, "Staccato", &sc(Act::Staccato)).clicked() {
                self.toggle_note_flag("Staccato", |n| &mut n.staccato);
                ui.close();
            }
            ui.separator();
            if check(ui, n.tie, "Tie Note", &sc(Act::TieNote)).clicked() {
                self.toggle_note_flag("Tie", |n| &mut n.tie);
                ui.close();
            }
        });
        if item(ui, "Rest", &sc(Act::Rest)).clicked() {
            self.key(Key::R, Modifiers::NONE, 0.0);
            ui.close();
        }
        if check(ui, cur.fermata, "Fermata", &sc(Act::Fermata)).clicked() {
            self.beat_edit(|b| b.fermata = !b.fermata);
            ui.close();
        }
        ui.separator();
        ui.add_enabled_ui(has, |ui| {
            ui.menu_button("Accidentals", |ui| {
                for (v, l) in [
                    (0u8, "Automatic"),
                    (1, "Sharps (\u{266F})"),
                    (2, "Flats (\u{266D})"),
                ] {
                    if check(ui, n.spell == v, l, "").clicked() {
                        self.note_edit(|x| x.spell = v);
                        ui.close();
                    }
                }
            });
            if item(ui, "One Semitone Down", &sc(Act::SemitoneDown)).clicked() {
                self.transpose(-1);
                ui.close();
            }
            if item(ui, "One Semitone Up", &sc(Act::SemitoneUp)).clicked() {
                self.transpose(1);
                ui.close();
            }
            if item(ui, "One Octave Down", &sc(Act::OctaveDown)).clicked() {
                self.shift_octave(-1);
                ui.close();
            }
            if item(ui, "One Octave Up", &sc(Act::OctaveUp)).clicked() {
                self.shift_octave(1);
                ui.close();
            }
        });
        ui.separator();
        if check(ui, cur.pick == 1, "Pickstroke Down", &sc(Act::PickDown)).clicked() {
            self.beat_edit(|b| b.pick = if b.pick == 1 { 0 } else { 1 });
            ui.close();
        }
        if check(ui, cur.pick == 2, "Pickstroke Up", &sc(Act::PickUp)).clicked() {
            self.beat_edit(|b| b.pick = if b.pick == 2 { 0 } else { 2 });
            ui.close();
        }
        ui.separator();
        if item(ui, "Chord...", &sc(Act::Chord)).clicked() {
            self.open_chord_tool();
            ui.close();
        }
        if item(ui, "Text...", &sc(Act::Text)).clicked() {
            self.open_popup(Popup::Text);
            ui.close();
        }
        ui.separator();
        ui.menu_button("Octave Sign", |ui| {
            for (v, l) in [
                (0i8, "None"),
                (1, "8va"),
                (2, "15ma"),
                (-1, "8vb"),
                (-2, "15mb"),
            ] {
                if check(ui, cur.ottava == v, l, "").clicked() {
                    self.beat_edit(|b| b.ottava = v);
                    ui.close();
                }
            }
        });
    }

    pub(super) fn effects_menu(&mut self, ui: &mut Ui) {
        let cur = self.cur_beat().clone();
        let has = !cur.is_rest();
        let n = cur
            .note_on(self.string as u8)
            .or(cur.notes.first())
            .cloned()
            .unwrap_or_default();
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
            if check(ui, n.trill > 0, "Trill...", &sc(Act::Trill)).clicked() {
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
            if check(ui, n.let_ring, "Let Ring", &sc(Act::LetRing)).clicked() {
                self.toggle_note_flag("Let ring", |n| &mut n.let_ring);
                ui.close();
            }
            if check(ui, n.hammer, "Hammer On / Pull Off", &sc(Act::Hammer)).clicked() {
                self.toggle_note_flag("Hammer-on / pull-off", |n| &mut n.hammer);
                ui.close();
            }
        });
        if check(ui, cur.technique == 1, "Tapping", &sc(Act::Tapping)).clicked() {
            self.beat_edit(|b| b.technique = if b.technique == 1 { 0 } else { 1 });
            ui.close();
        }
        ui.separator();
        if check(ui, cur.technique == 2, "Slap", &sc(Act::Slap)).clicked() {
            self.beat_edit(|b| b.technique = if b.technique == 2 { 0 } else { 2 });
            ui.close();
        }
        if check(ui, cur.technique == 3, "Pop", "").clicked() {
            self.beat_edit(|b| b.technique = if b.technique == 3 { 0 } else { 3 });
            ui.close();
        }
        ui.separator();
        ui.add_enabled_ui(has, |ui| {
            if check(ui, n.dead, "Dead Note", &sc(Act::DeadNote)).clicked() {
                self.toggle_note_flag("Dead note", |n| &mut n.dead);
                ui.close();
            }
            if check(ui, n.palm_mute, "Palm Mute on Note", &sc(Act::PalmMuteNote)).clicked() {
                self.toggle_note_flag("Palm mute", |n| &mut n.palm_mute);
                ui.close();
            }
            ui.separator();
            if check(ui, n.bend > 0, "Bend...", &sc(Act::Bend)).clicked() {
                self.open_popup(Popup::Bend);
                ui.close();
            }
            ui.menu_button("Slide", |ui| {
                if check(ui, n.slide, "Legato Slide", &sc(Act::Slide)).clicked() {
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
                if check(ui, n.vibrato, "Slight", &sc(Act::Vibrato)).clicked() {
                    self.toggle_note_flag("Vibrato", |n| &mut n.vibrato);
                    ui.close();
                }
                if check(ui, n.wide_vibrato, "Wide", "").clicked() {
                    self.toggle_note_flag("Wide vibrato", |n| &mut n.wide_vibrato);
                    ui.close();
                }
            });
            ui.separator();
            if check(ui, n.harmonic, "Natural Harmonic", &sc(Act::NaturalHarmonic)).clicked() {
                self.toggle_note_flag("Natural harmonic", |n| &mut n.harmonic);
                ui.close();
            }
            if check(ui, n.pinch_harmonic, "Artificial Harmonic", &sc(Act::ArtificialHarmonic)).clicked() {
                self.toggle_note_flag("Artificial harmonic", |n| &mut n.pinch_harmonic);
                ui.close();
            }
        });
        ui.separator();
        if check(ui, cur.stroke == Stroke::Down, "Brush Downstroke", &sc(Act::BrushDown)).clicked() {
            self.beat_edit(|b| {
                b.stroke = if b.stroke == Stroke::Down {
                    Stroke::None
                } else {
                    Stroke::Down
                }
            });
            ui.close();
        }
        if check(ui, cur.stroke == Stroke::Up, "Brush Upstroke", &sc(Act::BrushUp)).clicked() {
            self.beat_edit(|b| {
                b.stroke = if b.stroke == Stroke::Up {
                    Stroke::None
                } else {
                    Stroke::Up
                }
            });
            ui.close();
        }
        ui.separator();
        if check(ui, cur.fade_in, "Fade In", &sc(Act::FadeIn)).clicked() {
            self.beat_edit(|b| b.fade_in = !b.fade_in);
            ui.close();
        }
    }

    fn section_menu(&mut self, ui: &mut Ui) {
        let has = self.song.master[self.bar].marker.is_some();
        if check(ui, has, "Edit...", &sc(Act::Section)).clicked() {
            self.edit_section();
            ui.close();
        }
        if has && item(ui, "Remove", "").clicked() {
            self.bar_edit(|m| m.marker = None);
            ui.close();
        }
        ui.separator();
        if item(ui, "Previous Section", &sc(Act::PrevSection)).clicked() {
            self.goto_section(-1);
            ui.close();
        }
        if item(ui, "Next Section", &sc(Act::NextSection)).clicked() {
            self.goto_section(1);
            ui.close();
        }
        ui.separator();
        let marks: Vec<(usize, String)> = self
            .song
            .master
            .iter()
            .enumerate()
            .filter_map(|(i, m)| m.marker.clone().map(|n| (i, n)))
            .collect();
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
        if item(ui, "Tuner...", "").clicked() {
            self.open_tuner();
            ui.close();
        }
        if item(ui, "Chords & Scales...", "").clicked() {
            self.open_chord_scales(super::chordscales::Mode::Scales);
            ui.close();
        }
        // On the start screen only the song-free tools work.
        let song = !self.show_home;
        if item_if(ui, song, "Transpose...", "") {
            self.transpose_open = true;
            ui.close();
        }
        ui.separator();
        if item_if(ui, song, "Move/Swap Voices...", &sc(Act::MoveVoices)) {
            self.vfrom = self.voice;
            self.vto = if self.voice == 0 { 1 } else { 0 };
            self.vrange = if self.selection().is_some() {
                super::voices::VoiceRange::Selection
            } else {
                super::voices::VoiceRange::Bar
            };
            self.voices_open = true;
            ui.close();
        }
        if item_if(ui, song, "Check Bar Duration", &sc(Act::CheckBars)) {
            self.check_bar_duration();
            ui.close();
        }
        if item_if(ui, song, "Complete Bars with Rests", "") {
            self.complete_bars();
            ui.close();
        }
        ui.separator();
        if item_if(ui, song, "Show Command Palette", &sc(Act::CommandPalette)) {
            self.cmd_open = true;
            self.cmd_text.clear();
            ui.close();
        }
    }

    fn sound_menu(&mut self, ui: &mut Ui) {
        if item(ui, if self.playing { "Pause" } else { "Play" }, &sc(Act::PlayPause)).clicked() {
            self.toggle_play();
            ui.close();
        }
        if item(ui, "Play from the Beginning", &sc(Act::PlayFromStart)).clicked() {
            self.play_from_start();
            ui.close();
        }
        if item(ui, "Stop", &sc(Act::Stop)).clicked() {
            self.stop();
            ui.close();
        }
        ui.separator();
        if item(ui, "First Bar", &sc(Act::FirstBar)).clicked() {
            self.stop();
            self.goto_bar(0);
            ui.close();
        }
        if item(ui, "Rewind", &sc(Act::PrevBar)).clicked() {
            self.goto_bar(self.bar.saturating_sub(1));
            ui.close();
        }
        if item(ui, "Fast Forward", &sc(Act::NextBar)).clicked() {
            self.goto_bar(self.bar + 1);
            ui.close();
        }
        if item(ui, "Last Bar", &sc(Act::LastBar)).clicked() {
            self.stop();
            self.goto_bar(self.song.bar_count() - 1);
            ui.close();
        }
        ui.separator();
        if item(ui, "Step Backward", &sc(Act::StepBack)).clicked() {
            self.step(-1);
            ui.close();
        }
        if item(ui, "Step Forward", &sc(Act::StepForward)).clicked() {
            self.step(1);
            ui.close();
        }
        ui.separator();
        if check(ui, self.looping, "Loop", &sc(Act::Loop)).clicked() {
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
                if check(
                    ui,
                    (self.speed - v).abs() < 0.5,
                    &format!("{}%", v as i32),
                    "",
                )
                .clicked()
                {
                    self.speed = v;
                    ui.close();
                }
            }
        });
        if item(ui, "Speed Trainer...", "").clicked() {
            self.trainer_open = true;
            ui.close();
        }
        ui.add(
            egui::Slider::new(&mut self.speed, 10.0..=300.0)
                .suffix("%")
                .text("Speed"),
        );
        ui.add(egui::Slider::new(&mut self.master_volume, 0.0..=1.5).text("Volume"));
    }

    fn view_menu(&mut self, ui: &mut Ui) {
        for (pane, label, keys) in [
            (Pane::Palette, "Show Editing Palette", sc(Act::ShowPalette)),
            (Pane::Inspector, "Show Inspector", String::new()),
            (Pane::Tracks, "Show Global View", sc(Act::ShowGlobalView)),
            (Pane::Fretboard, "Show Fretboard View", String::new()),
            (Pane::Keyboard, "Show Keyboard View", sc(Act::KeyboardView)),
        ] {
            if pane == Pane::Fretboard {
                ui.separator();
            }
            if check(ui, self.panel_shown(pane), label, &keys).clicked() {
                self.toggle_panel(pane);
            }
        }
        ui.menu_button("Pop Out Panel", |ui| {
            for pane in Pane::PANELS {
                let out = self.popped.contains(&pane);
                if ui.selectable_label(out, pane.title()).on_hover_text("A panel in a window of its own, which can leave the main window").clicked() {
                    if out {
                        self.dock_panel(pane);
                    } else {
                        self.pop_out(pane);
                    }
                    ui.close();
                }
            }
        });
        if ui.add_enabled(!self.popped.is_empty(), egui::Button::new("Dock All Panels")).clicked() {
            for pane in std::mem::take(&mut self.popped) {
                self.show_panel(pane);
            }
            ui.close();
        }
        if item(ui, "Reset Panel Layout", "").clicked() {
            self.reset_panel_layout();
            ui.close();
        }
        ui.separator();
        if check(ui, !self.single_track, "Multitrack", &sc(Act::Multitrack)).clicked() {
            self.single_track = !self.single_track;
        }
        ui.menu_button("Notation", |ui| {
            for (v, l) in [
                (Notation::Both, "Standard + Tablature"),
                (Notation::TabOnly, "Tablature Only"),
                (Notation::StandardOnly, "Standard Only"),
            ] {
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
        if item(ui, "Zoom In", &sc(Act::ZoomIn)).clicked() {
            self.zoom_step(1);
        }
        if item(ui, "Zoom Out", &sc(Act::ZoomOut)).clicked() {
            self.zoom_step(-1);
        }
        ui.separator();
        for (v, l) in [
            (0u8, "Page - Vertical"),
            (1, "Page - Horizontal"),
            (2, "Page - Grid"),
            (3, "Screen - Vertical"),
            (4, "Screen - Horizontal"),
        ] {
            if check(ui, self.view_mode == v, l, "").clicked() {
                self.view_mode = v;
                ui.close();
            }
        }
        ui.separator();
        if check(ui, self.fullscreen, "Full Screen Mode", &sc(Act::FullScreen)).clicked() {
            self.toggle_fullscreen();
            ui.close();
        }
    }

    fn window_menu(&mut self, ui: &mut Ui) {
        // The open files, one to a tab.
        for (i, (name, _)) in self.tab_labels().into_iter().enumerate() {
            let on = !self.show_home && i == self.active_tab;
            if check(ui, on, &format!("{} {name}", i + 1), "").clicked() {
                self.switch_to(i);
                ui.close();
            }
        }
        if !self.tabs.is_empty() {
            ui.separator();
            if item(ui, "Next Tab", &sc(Act::NextTab)).clicked() {
                self.cycle_tab(1);
                ui.close();
            }
            if item(ui, "Previous Tab", &sc(Act::PrevTab)).clicked() {
                self.cycle_tab(-1);
                ui.close();
            }
            ui.separator();
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
        if item(ui, "Help Center...", &sc(Act::Help)).clicked() {
            self.guide_open = true;
            ui.close();
        }
        ui.separator();
        if item(ui, "Check for Updates...", "").clicked() {
            let ctx = ui.ctx().clone();
            self.check_for_updates(&ctx);
            ui.close();
        }
    }
}
