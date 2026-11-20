//! The menu shown when right-clicking the score (as in Guitar Pro 8): cut,
//! copy and paste, the all-track versions, Special Paste, and the Bar, Note
//! and Effects menus. Special Paste (Ctrl+Shift+V) pastes several times and
//! chooses which extras come along: chord diagrams, texts, timers, sections
//! and tempo changes (the last two only after an all-track copy).

use super::App;
use super::shortcuts::{Act, sc};
use egui::{Align2, Color32, RichText, Ui, Vec2};

pub struct SpecialPaste {
    pub times: u32,
    pub chords: bool,
    pub texts: bool,
    pub timers: bool,
    pub sections: bool,
    pub tempo: bool,
}

use super::menus::item_if as item;

impl App {
    fn has_clipboard(&self) -> bool {
        !self.clipboard.is_empty() || !self.bar_clipboard.is_empty()
    }

    /// Cut, copy, paste and Special Paste (shared by the Edit menu and the score's menu).
    pub(super) fn clipboard_items(&mut self, ui: &mut Ui) {
        if item(ui, true, "Cut", &sc(Act::Cut)) {
            self.copy();
            self.last_copy_all = false;
            self.delete_at_cursor();
            ui.close();
        }
        if item(ui, true, "Copy", &sc(Act::Copy)) {
            self.copy();
            self.last_copy_all = false;
            ui.close();
        }
        let paste = self.has_clipboard();
        if item(ui, paste, "Paste", &sc(Act::Paste)) {
            self.paste_any();
            ui.close();
        }
        if item(ui, true, "All-Track Cut", &sc(Act::AllTrackCut)) {
            self.all_track_cut();
            ui.close();
        }
        if item(ui, true, "All-Track Copy", &sc(Act::AllTrackCopy)) {
            self.all_track_copy();
            ui.close();
        }
        if item(ui, paste, "Special Paste...", &sc(Act::SpecialPaste)) {
            self.open_special_paste();
            ui.close();
        }
    }

    /// The menu of a right-click on the score.
    pub(super) fn score_context_menu(&mut self, ui: &mut Ui) {
        ui.set_min_width(190.0);
        self.clipboard_items(ui);
        ui.separator();
        ui.menu_button("Bar", |ui| self.bar_menu(ui));
        ui.separator();
        ui.menu_button("Note", |ui| self.note_menu(ui));
        ui.separator();
        ui.menu_button("Effects", |ui| self.effects_menu(ui));
    }

    pub(super) fn open_special_paste(&mut self) {
        if !self.has_clipboard() {
            self.status = "Copy something first.".into();
            return;
        }
        self.special_paste = Some(SpecialPaste { times: 1, chords: true, texts: true, timers: true, sections: true, tempo: true });
    }

    pub(super) fn special_paste_window(&mut self, ctx: &egui::Context) {
        let Some(mut sp) = self.special_paste.take() else { return };
        let all_track = self.last_copy_all && !self.bar_clipboard.is_empty();
        let (mut ok, mut cancel) = (false, false);
        let mut open = true;
        egui::Window::new("Special Paste")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.set_width(300.0);
                ui.label(RichText::new("SPECIAL PASTE").strong().size(15.0));
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label("Number of pastes:");
                    ui.add(egui::DragValue::new(&mut sp.times).range(1..=64));
                });
                ui.add_space(4.0);
                ui.label(RichText::new("Paste these too:").color(Color32::from_gray(200)));
                ui.checkbox(&mut sp.chords, "Chord diagrams");
                ui.checkbox(&mut sp.texts, "Texts");
                ui.checkbox(&mut sp.timers, "Timers");
                ui.add_enabled_ui(all_track, |ui| {
                    ui.checkbox(&mut sp.sections, "Sections");
                    ui.checkbox(&mut sp.tempo, "Tempo changes (master track automations)");
                })
                .response
                .on_disabled_hover_text(format!("Sections and tempo changes need an all-track copy ({}).", super::shortcuts::sc(super::shortcuts::Act::AllTrackCopy)));
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Cancel").clicked() {
                            cancel = true;
                        }
                        if ui.add(egui::Button::new(RichText::new("OK").color(Color32::WHITE)).fill(super::panels::BLUE)).clicked() {
                            ok = true;
                        }
                    });
                });
            });
        if ok {
            self.special_paste_apply(&sp);
            return;
        }
        if open && !cancel && !ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.special_paste = Some(sp);
        }
    }

    /// Pastes the clipboard `times` times, leaving out the extras that are unticked.
    fn special_paste_apply(&mut self, sp: &SpecialPaste) {
        let keep = (self.clipboard.clone(), self.bar_clipboard.clone());
        let strip = |b: &mut crate::model::Beat| {
            if !sp.chords {
                b.chord = None;
                b.chord_diagram = false;
            }
            if !sp.texts {
                b.text = None;
            }
            if !sp.timers {
                b.fx.timer = false;
            }
        };
        for b in &mut self.clipboard {
            strip(b);
        }
        for (mb, bars) in &mut self.bar_clipboard {
            if !sp.sections {
                mb.marker = None;
            }
            if !sp.tempo {
                mb.tempo = None;
                mb.tempo_ramp = false;
            }
            for bar in bars.iter_mut() {
                for b in bar.all_beats_mut() {
                    strip(b);
                }
            }
        }
        for _ in 0..sp.times.max(1) {
            self.paste_any();
        }
        self.clipboard = keep.0;
        self.bar_clipboard = keep.1;
        self.status = format!("Pasted {} time(s).", sp.times.max(1));
    }
}
