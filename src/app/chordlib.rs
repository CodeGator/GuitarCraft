//! The chord diagram library (the palette's Chords button): every chord the
//! current track uses, drawn as a diagram. Click one to put it on the beat
//! under the cursor; the add button opens the chord editor, the remove button
//! takes the selected chord off every beat of the track.

use super::App;
use crate::model::*;
use egui::{Align2, Color32, FontFamily, FontId, Pos2, Rect, RichText, Sense, Vec2};

const CELL: Vec2 = Vec2::new(70.0, 92.0);

impl App {
    /// The track's chords: each name once, with the first beat that uses it.
    fn track_chords(&self) -> Vec<(String, Beat)> {
        let mut out: Vec<(String, Beat)> = vec![];
        for bar in &self.tr().bars {
            for b in &bar.beats {
                if let Some(name) = &b.chord {
                    if !out.iter().any(|(n, _)| n == name) {
                        out.push((name.clone(), b.clone()));
                    }
                }
            }
        }
        out
    }

    pub(super) fn chord_library_window(&mut self, ctx: &egui::Context) {
        if !self.chord_lib_open {
            return;
        }
        let mut open = true;
        let chords = self.track_chords();
        let mut put: Option<(String, Beat)> = None;
        let mut add = false;
        let mut remove = false;
        let selected = self.cur_beat().chord.clone();
        egui::Window::new("Chord Library")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_size(Vec2::new(300.0, 300.0))
            .default_pos(Pos2::new(250.0, 140.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("CHORD DIAGRAM LIBRARY").size(15.0).family(FontFamily::Name("bold".into())).color(Color32::from_gray(235)));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add_enabled(selected.is_some(), egui::Button::new(RichText::new("\u{2715}").size(14.0))).on_hover_text("Remove this chord from every beat of the track").on_disabled_hover_text("Put the cursor on a beat with a chord to remove it").clicked() {
                            remove = true;
                        }
                        if ui.button(RichText::new("\u{25A6}+").size(14.0)).on_hover_text("Add a chord\u{2026}").clicked() {
                            add = true;
                        }
                    });
                });
                ui.add_space(4.0);
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    let w = ui.available_width().max(CELL.x);
                    let cols = ((w - 8.0) / CELL.x).floor().max(1.0) as usize;
                    let rows = chords.len().div_ceil(cols).max(1);
                    let h = (rows as f32 * CELL.y + 12.0).max(ui.available_height());
                    let (area, _) = ui.allocate_exact_size(Vec2::new(w, h), Sense::hover());
                    ui.painter().rect_filled(area, 2.0, Color32::WHITE);
                    if chords.is_empty() {
                        ui.painter().text(area.center_top() + Vec2::new(0.0, 30.0), Align2::CENTER_CENTER, "No chords yet: press + to add one.", FontId::proportional(12.0), Color32::from_gray(120));
                    }
                    let tr = self.tr().clone();
                    for (k, (name, beat)) in chords.iter().enumerate() {
                        let cell = Rect::from_min_size(area.min + Vec2::new(6.0 + (k % cols) as f32 * CELL.x, 6.0 + (k / cols) as f32 * CELL.y), CELL - Vec2::splat(4.0));
                        let resp = ui.interact(cell, ui.id().with(("chord", k)), Sense::click());
                        let p = ui.painter();
                        if selected.as_deref() == Some(name.as_str()) {
                            p.rect_filled(cell, 3.0, Color32::from_rgb(200, 220, 250));
                        } else if resp.hovered() {
                            p.rect_filled(cell, 3.0, Color32::from_gray(235));
                        }
                        p.text(Pos2::new(cell.center().x, cell.top() + 4.0), Align2::CENTER_TOP, name, FontId::new(13.0, FontFamily::Name("serif-bold".into())), Color32::BLACK);
                        crate::score::draw_diagram(p, &tr, beat, Pos2::new(cell.center().x, cell.top() + 22.0), 1.15, self.song.style.chord_style, Color32::BLACK);
                        if resp.on_hover_text(format!("Put {name} on this beat")).clicked() {
                            put = Some((name.clone(), beat.clone()));
                        }
                    }
                });
            });
        if let Some((name, sample)) = put {
            let notes_empty = self.cur_beat().is_rest();
            self.edit(move |s, t, b, j, _| {
                let beat = &mut s.tracks[t].bars[b].beats[j];
                beat.chord = Some(name.clone());
                beat.chord_diagram = sample.chord_diagram;
                // An empty beat gets the chord's notes too.
                if notes_empty {
                    beat.notes = sample.notes.clone();
                }
            });
            self.status = "Chord added.".into();
        }
        if add {
            self.open_chord_tool();
        }
        if remove {
            if let Some(name) = selected {
                self.edit(move |s, t, _, _, _| {
                    for bar in &mut s.tracks[t].bars {
                        for b in &mut bar.beats {
                            if b.chord.as_deref() == Some(name.as_str()) {
                                b.chord = None;
                                b.chord_diagram = false;
                            }
                        }
                    }
                });
                self.status = "Chord removed from the track.".into();
            }
        }
        self.chord_lib_open = open;
    }
}
