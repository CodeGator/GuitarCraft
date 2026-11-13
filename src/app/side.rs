//! The edition palette (left) and the settings panel (right).

use super::panels::BLUE;
use super::{App, InspectorTab};
use crate::model::*;
use egui::{
    Align2, Color32, FontFamily, FontId, Pos2, Rect, RichText, Sense, Ui, Vec2,
};

fn heading(ui: &mut Ui, t: &str) {
    ui.add_space(6.0);
    ui.label(
        RichText::new(t)
            .size(15.0)
            .family(FontFamily::Name("bold".into()))
            .color(Color32::from_gray(235)),
    );
    ui.add_space(2.0);
}

fn field_label(ui: &mut Ui, t: &str) {
    ui.label(RichText::new(t).size(12.5).color(Color32::from_gray(215)));
}

impl App {
    /// Applies a change to the note under the cursor (or every note of the beat).
    pub(super) fn note_edit(&mut self, f: impl Fn(&mut Note)) {
        // With a selection, every selected note changes.
        if let Some((sa, sb)) = self.selection() {
            self.edit(|s, t, _, _, _| {
                for bi in sa.0..=sb.0 {
                    let beats = &mut s.tracks[t].bars[bi].beats;
                    let from = if bi == sa.0 { sa.1 } else { 0 };
                    let to = if bi == sb.0 {
                        sb.1
                    } else {
                        beats.len().saturating_sub(1)
                    };
                    for beat in beats.iter_mut().take(to + 1).skip(from) {
                        for n in &mut beat.notes {
                            f(n);
                        }
                    }
                }
            });
            return;
        }
        if self.cur_beat().is_rest() {
            self.status = "Put a note on this beat first.".into();
            return;
        }
        let on_string = self.cur_beat().note_on(self.string as u8).is_some()
            && (self.std_cursor.is_none() || self.note_at_std().is_some());
        self.edit(|s, t, b, j, st| {
            let beat = &mut s.tracks[t].bars[b].beats[j];
            if on_string {
                if let Some(n) = beat.note_on_mut(st as u8) {
                    f(n);
                }
            } else {
                for n in &mut beat.notes {
                    f(n);
                }
            }
        });
    }

    pub(super) fn beat_edit(&mut self, f: impl Fn(&mut Beat)) {
        // With a selection, every selected beat changes.
        if let Some((sa, sb)) = self.selection() {
            self.edit(|s, t, _, _, _| {
                for bi in sa.0..=sb.0 {
                    let beats = &mut s.tracks[t].bars[bi].beats;
                    let from = if bi == sa.0 { sa.1 } else { 0 };
                    let to = if bi == sb.0 {
                        sb.1
                    } else {
                        beats.len().saturating_sub(1)
                    };
                    for beat in beats.iter_mut().take(to + 1).skip(from) {
                        f(beat);
                    }
                }
            });
            return;
        }
        self.edit(|s, t, b, j, _| f(&mut s.tracks[t].bars[b].beats[j]));
    }

    pub(super) fn bar_edit(&mut self, f: impl FnOnce(&mut MasterBar)) {
        self.edit(|s, _, b, _, _| f(&mut s.master[b]));
    }

    pub(super) fn inspector_ui(&mut self, ui: &mut Ui) {
        // Tabs across the top.
        let tabs = [(InspectorTab::Song, "SONG"), (InspectorTab::Track, "TRACK")];
        let full = ui.available_rect_before_wrap();
        let tw = full.width() / tabs.len() as f32;
        let (tr, _) = ui.allocate_exact_size(Vec2::new(full.width(), 26.0), Sense::hover());
        for (i, (t, lab)) in tabs.iter().enumerate() {
            let r = Rect::from_min_size(
                Pos2::new(tr.left() + i as f32 * tw, tr.top()),
                Vec2::new(tw, 26.0),
            );
            let resp = ui.interact(r, ui.id().with(("insp_tab", i)), Sense::click());
            let sel = self.inspector == *t;
            ui.painter().rect_filled(
                r,
                0.0,
                if sel {
                    BLUE
                } else if resp.hovered() {
                    Color32::from_rgb(66, 70, 80)
                } else {
                    Color32::from_rgb(54, 58, 67)
                },
            );
            ui.painter().text(
                r.center(),
                Align2::CENTER_CENTER,
                *lab,
                FontId::new(11.5, FontFamily::Name("bold".into())),
                Color32::WHITE,
            );
            if resp.clicked() {
                self.inspector = *t;
            }
        }
        egui::ScrollArea::vertical().id_salt("insp").show(ui, |ui| {
            egui::Frame::new()
                .inner_margin(egui::Margin::symmetric(10, 4))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    self.inspector_body(ui);
                });
        });
    }

    fn inspector_body(&mut self, ui: &mut Ui) {
        let before = self.song.clone();
        let mut changed = false;
        let fw = ui.available_width();
        match self.inspector {
            InspectorTab::Song => {
                heading(ui, "INFORMATION");
                // Title and artist; More shows the other fields, Less hides them again.
                let more = self.song_more;
                let s = &mut self.song;
                let mut fields: Vec<(&str, &mut String)> =
                    vec![("Title:", &mut s.title), ("Artist:", &mut s.artist)];
                if more {
                    fields.extend([
                        ("Subtitle:", &mut s.subtitle),
                        ("Album:", &mut s.album),
                        ("Words:", &mut s.words),
                        ("Music:", &mut s.music_by),
                        ("Copyright:", &mut s.copyright),
                        ("Transcriber:", &mut s.tabbed_by),
                        ("Notice:", &mut s.notice),
                        ("Instructions:", &mut s.instructions),
                    ]);
                }
                for (lab, v) in fields {
                    field_label(ui, lab);
                    // Enter finishes the field; Ctrl+Return starts a new line.
                    let edit = egui::TextEdit::multiline(v)
                        .desired_width(fw)
                        .desired_rows(1)
                        .return_key(Some(egui::KeyboardShortcut::new(
                            egui::Modifiers::COMMAND,
                            egui::Key::Enter,
                        )));
                    changed |= ui.add(edit).changed();
                }
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Ctrl+Return for new line.").color(Color32::from_gray(120)),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let lab = if more {
                            "Less \u{25B4}"
                        } else {
                            "More \u{25BE}"
                        };
                        if ui
                            .add(egui::Button::new(lab).fill(Color32::from_rgb(70, 74, 84)))
                            .clicked()
                        {
                            self.song_more = !more;
                        }
                    });
                });
                heading(ui, "MUSICAL NOTATION");
                ui.horizontal(|ui| {
                    field_label(ui, "Stylesheet:");
                    if ui
                        .add(egui::Button::new("Open...").fill(Color32::from_rgb(70, 74, 84)))
                        .on_hover_text(
                            "How the score looks: page, staves, header and footer, fonts (F7)",
                        )
                        .clicked()
                    {
                        self.open_stylesheet();
                    }
                });
            }
            InspectorTab::Track => {
                let ti = self.track;
                let mut play_tune = false;
                let t = &mut self.song.tracks[ti];
                let (old_tuning, old_capo) = (t.tuning.clone(), t.capo);
                heading(ui, "INFORMATION");
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    // Colour swatch: click to pick the next colour.
                    let (r, resp) = ui.allocate_exact_size(Vec2::splat(22.0), Sense::click());
                    ui.painter().rect_filled(
                        r,
                        3.0,
                        Color32::from_rgb(t.color[0], t.color[1], t.color[2]),
                    );
                    if resp
                        .on_hover_text("Track colour (click to change)")
                        .clicked()
                    {
                        let i = TRACK_COLORS
                            .iter()
                            .position(|c| *c == t.color)
                            .map(|i| (i + 1) % TRACK_COLORS.len())
                            .unwrap_or(0);
                        t.color = TRACK_COLORS[i];
                        changed = true;
                    }
                    let (ir, iresp) = ui.allocate_exact_size(Vec2::new(22.0, 22.0), Sense::click());
                    ui.painter().rect_filled(
                        ir,
                        3.0,
                        if iresp.hovered() {
                            Color32::from_gray(80)
                        } else {
                            Color32::from_gray(62)
                        },
                    );
                    super::trackicons::draw(
                        ui.painter(),
                        Rect::from_center_size(ir.center(), Vec2::new(10.0, 15.0)),
                        Color32::from_gray(225),
                        super::trackicons::icon_of(t),
                    );
                    let iresp = iresp.on_hover_text("Track icon");
                    let mut pop = egui::Popup::menu(&iresp)
                        .close_behavior(egui::PopupCloseBehavior::CloseOnClick);
                    if self.icon_menu_open {
                        pop = pop.open(true);
                    }
                    pop.show(|ui| {
                        let cur = super::trackicons::icon_of(t);
                        for (group, list) in super::trackicons::GROUPS {
                            ui.label(
                                RichText::new(group)
                                    .strong()
                                    .size(14.0)
                                    .color(Color32::WHITE),
                            );
                            for row in list.chunks(7) {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 4.0;
                                    for (id, name) in row {
                                        let (r, resp) = ui
                                            .allocate_exact_size(Vec2::splat(24.0), Sense::click());
                                        let on = *id == cur;
                                        if on {
                                            ui.painter().rect_filled(r, 4.0, BLUE);
                                        } else if resp.hovered() {
                                            ui.painter().rect_filled(
                                                r,
                                                4.0,
                                                Color32::from_gray(70),
                                            );
                                        }
                                        super::trackicons::draw(
                                            ui.painter(),
                                            Rect::from_center_size(
                                                r.center(),
                                                Vec2::new(13.0, 18.0),
                                            ),
                                            Color32::from_gray(235),
                                            *id,
                                        );
                                        if resp.on_hover_text(*name).clicked() {
                                            t.icon = *id;
                                            changed = true;
                                        }
                                    }
                                });
                            }
                        }
                    });
                    changed |= ui
                        .add(egui::TextEdit::singleline(&mut t.name).desired_width(fw - 112.0))
                        .on_hover_text("Track name")
                        .changed();
                    let auto = crate::score::short_name(&t.name);
                    changed |= ui
                        .add(
                            egui::TextEdit::singleline(&mut t.short_name)
                                .hint_text(auto)
                                .desired_width(52.0),
                        )
                        .on_hover_text("Short name beside the staves")
                        .changed();
                });
                heading(ui, "MUSICAL NOTATION");
                let kind = t.kind();
                ui.horizontal(|ui| {
                    field_label(ui, "Notation:");
                    ui.spacing_mut().item_spacing.x = 2.0;
                    let strings = kind == InstrumentType::Stringed;
                    // Tablature, standard notation, then the kinds not built yet.
                    let tab = ui.add_enabled(
                        strings,
                        egui::Button::selectable(t.has_tab(), "TAB").frame_when_inactive(true),
                    );
                    if tab
                        .on_hover_text("Tablature")
                        .on_disabled_hover_text("Only stringed instruments have tablature")
                        .clicked()
                    {
                        t.show_tab = !t.show_tab;
                        if !t.show_std && !t.show_tab {
                            t.show_std = true;
                        }
                        changed = true;
                    }
                    let std_on = !strings || t.show_std;
                    if ui
                        .add(
                            egui::Button::selectable(std_on, "\u{2669}")
                                .frame_when_inactive(true)
                                .min_size(Vec2::new(22.0, 0.0)),
                        )
                        .on_hover_text(if kind == InstrumentType::Drums {
                            "Percussion staff"
                        } else {
                            "Standard notation"
                        })
                        .clicked()
                        && strings
                    {
                        t.show_std = !t.show_std;
                        if !t.show_std && !t.show_tab {
                            t.show_tab = true;
                        }
                        changed = true;
                    }
                    ui.add_enabled(false, egui::Button::new("/").min_size(Vec2::new(18.0, 0.0)))
                        .on_disabled_hover_text("Slash notation isn't available yet");
                    ui.add_enabled(false, egui::Button::new("2").min_size(Vec2::new(18.0, 0.0)))
                        .on_disabled_hover_text("Numbered notation isn't available yet");
                    ui.add_space(4.0);
                    // One staff or a grand staff (keyboards).
                    let keys = kind == InstrumentType::Keyboard;
                    if ui
                        .add(
                            egui::Button::selectable(!t.grand(), "\u{25AD}")
                                .frame_when_inactive(true)
                                .min_size(Vec2::new(20.0, 0.0)),
                        )
                        .on_hover_text("Single staff")
                        .clicked()
                        && t.grand_staff
                    {
                        t.grand_staff = false;
                        changed = true;
                    }
                    let g = ui.add_enabled(
                        keys,
                        egui::Button::selectable(t.grand(), "\u{2630}")
                            .frame_when_inactive(true)
                            .min_size(Vec2::new(20.0, 0.0)),
                    );
                    if g.on_hover_text("Grand staff: two staves joined by a brace")
                        .on_disabled_hover_text("Only keyboards use a grand staff")
                        .clicked()
                        && !t.grand_staff
                    {
                        t.grand_staff = true;
                        changed = true;
                    }
                });
                let mut open_tuning = false;
                if kind == InstrumentType::Stringed {
                    ui.horizontal(|ui| {
                        field_label(ui, "Tuning:");
                        let names: String = t
                            .tuning
                            .iter()
                            .rev()
                            .map(|&n| super::tuning::named(n, t.tuning_flat))
                            .collect::<Vec<_>>()
                            .join(" ");
                        let label = if t.capo > 0 {
                            format!("\u{2699}  {names}   capo {}", t.capo)
                        } else {
                            format!("\u{2699}  {names}")
                        };
                        if ui
                            .add(egui::Button::new(label).min_size(Vec2::new(fw - 100.0, 0.0)))
                            .on_hover_text("Tuning and capo")
                            .clicked()
                        {
                            open_tuning = true;
                        }
                        if ui
                            .button("\u{25B6}")
                            .on_hover_text("Play the open strings")
                            .clicked()
                        {
                            play_tune = true;
                        }
                    });
                }
                if self.keep_pitch && (t.tuning != old_tuning || t.capo != old_capo) {
                    super::actions::refit_track(t, &old_tuning, old_capo);
                }
                heading(ui, "SOUNDS");
                egui::ComboBox::from_id_salt("inst")
                    .width(fw - 10.0)
                    .selected_text(format!("{}. {}", ti + 1, t.instrument.name()))
                    .show_ui(ui, |ui| {
                        for k in InstrumentType::ALL {
                            ui.label(
                                RichText::new(k.name().to_uppercase())
                                    .small()
                                    .color(Color32::from_gray(140)),
                            );
                            for i in k.instruments() {
                                if ui
                                    .selectable_label(t.instrument == i, format!("   {}", i.name()))
                                    .clicked()
                                    && t.instrument != i
                                {
                                    t.set_instrument(i);
                                    changed = true;
                                }
                            }
                        }
                    });
                heading(ui, "INTERPRETATION");
                egui::Grid::new("interp")
                    .num_columns(2)
                    .spacing(Vec2::new(8.0, 6.0))
                    .show(ui, |ui| {
                        field_label(ui, "Playing style:");
                        let cur = (t.playing_style as usize).min(PLAYING_STYLES.len() - 1);
                        egui::ComboBox::from_id_salt("style")
                            .width(90.0)
                            .selected_text(PLAYING_STYLES[cur].0)
                            .show_ui(ui, |ui| {
                                for (k, (name, _)) in PLAYING_STYLES.iter().enumerate() {
                                    if ui.selectable_label(cur == k, *name).clicked() && cur != k {
                                        t.playing_style = k as u8;
                                        changed = true;
                                    }
                                }
                            });
                        ui.end_row();
                        field_label(ui, "Palm mute:");
                        changed |= ui
                            .add(egui::Slider::new(&mut t.palm_mute, 0.0..=1.0).show_value(false))
                            .on_hover_text("Mute every note of this track (past halfway)")
                            .changed();
                        ui.end_row();
                        field_label(ui, "Accentuation:");
                        changed |= ui
                            .add(
                                egui::Slider::new(&mut t.accentuation, 0.0..=1.0).show_value(false),
                            )
                            .on_hover_text("Play the first beat of every bar louder")
                            .changed();
                        ui.end_row();
                        field_label(ui, "Auto let ring:");
                        changed |= toggle_switch(ui, &mut t.auto_let_ring);
                        ui.end_row();
                        field_label(ui, "Auto brush:");
                        changed |= toggle_switch(ui, &mut t.auto_brush);
                        ui.end_row();
                    });
                if play_tune {
                    self.play_tuning();
                }
                if open_tuning {
                    self.open_tuning();
                }
            }
        }
        if changed {
            self.undo.push(before);
            self.redo.clear();
            self.dirty = true;
            self.song.normalize();
            self.clamp_cursor();
            self.total_secs = self.song_seconds();
        }
    }
}

/// A small on/off switch with a label, like the original's settings toggles.
fn toggle_switch(ui: &mut Ui, on: &mut bool) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        let (r, resp) = ui.allocate_exact_size(Vec2::new(30.0, 16.0), Sense::click());
        if resp.clicked() {
            *on = !*on;
            changed = true;
        }
        let p = ui.painter();
        p.rect_filled(
            r,
            8.0,
            if *on {
                BLUE
            } else {
                Color32::from_rgb(70, 74, 84)
            },
        );
        let cx = if *on { r.right() - 8.0 } else { r.left() + 8.0 };
        p.circle_filled(Pos2::new(cx, r.center().y), 6.0, Color32::WHITE);
        ui.label(if *on { "On" } else { "Off" });
    });
    changed
}


/// Colours of voices 1-4, used for the cursor and markers.
pub(super) const VOICE_COLORS: [Color32; VOICES] = [
    Color32::from_rgb(240, 200, 80),
    Color32::from_rgb(110, 200, 120),
    Color32::from_rgb(230, 120, 180),
    Color32::from_rgb(160, 120, 230),
];
