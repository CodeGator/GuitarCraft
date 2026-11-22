//! Transposing a chord chart: the Transpose window (Edit > Transpose, Ctrl+T)
//! moves the chords up or down by half steps or to another key, for the whole
//! chart (chords, key and diagrams) or one section, spelled with sharps or
//! flats. The toolbar's ♭ and ♯ buttons move the whole chart a half step.

use super::App;
use super::stylesheet::BLUE;
use crate::chart::{self, Spelling};
use egui::{Align, Align2, Color32, Layout, RichText, Vec2};

pub struct XposeDlg {
    pub semis: i32,
    pub spelling: Spelling,
    /// Only this section (index), or the whole chart.
    pub only: Option<usize>,
}

/// "Up a whole step", "Down 3 half steps" ...
fn describe(semis: i32) -> String {
    let n = semis.abs();
    let dir = if semis > 0 { "Up" } else { "Down" };
    match n {
        0 => "No change".into(),
        1 => format!("{dir} a half step"),
        2 => format!("{dir} a whole step"),
        12 => format!("{dir} an octave (same chord names)"),
        _ => format!("{dir} {n} half steps"),
    }
}

impl App {
    pub(super) fn open_transpose(&mut self, only: Option<usize>) {
        if let Some(st) = &mut self.chart {
            st.xpose = Some(XposeDlg { semis: 2, spelling: Spelling::Auto, only });
        }
    }

    /// The toolbar's quick half steps (the whole chart).
    pub(super) fn transpose_step(&mut self, semis: i32) {
        if let Some(st) = &mut self.chart {
            st.doc.transpose(semis, Spelling::Auto, None);
            let key = if st.doc.key.trim().is_empty() { String::new() } else { format!(" Key: {}.", st.doc.key.trim()) };
            self.status = format!("Transposed {}.{key}", describe(semis).to_lowercase());
        }
    }

    pub(super) fn transpose_window(&mut self, ctx: &egui::Context) {
        let Some(st) = &mut self.chart else { return };
        let Some(mut dlg) = st.xpose.take() else { return };
        let mut open = true;
        let (mut ok, mut cancel) = (false, false);
        let doc = &st.doc;
        let sel_name = st.doc.sections.get(st.sel).map(|s| s.name.clone()).unwrap_or_default();
        let key = chart::split_key(&doc.key);
        egui::Window::new("Transpose")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.set_width(super::widgets::DIALOG_W);
                ui.label(RichText::new("TRANSPOSE").strong().size(15.0));
                ui.add_space(4.0);
                ui.label("Chords in:");
                ui.radio_value(&mut dlg.only, None, "The whole chart (its key and chord diagrams too)");
                ui.radio_value(&mut dlg.only, Some(st.sel), format!("Only \u{201C}{sel_name}\u{201D}"));
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label("Move by:");
                    if ui.button("\u{2212}").on_hover_text("Down a half step").clicked() {
                        dlg.semis -= 1;
                    }
                    ui.add(egui::DragValue::new(&mut dlg.semis).range(-11..=11).custom_formatter(|v, _| format!("{:+}", v as i32)).suffix(" half steps"));
                    if ui.button("+").on_hover_text("Up a half step").clicked() {
                        dlg.semis += 1;
                    }
                    dlg.semis = dlg.semis.clamp(-11, 11);
                    ui.label(RichText::new(describe(dlg.semis)).color(Color32::from_gray(170)));
                });
                if let Some((root, minor)) = key
                    && dlg.only.is_none()
                {
                    ui.horizontal(|ui| {
                        ui.label("Or to the key of:");
                        let flats = doc.use_flats(dlg.semis, dlg.spelling);
                        let now = chart::key_names(minor, flats)[(root + dlg.semis).rem_euclid(12) as usize].clone();
                        egui::ComboBox::from_id_salt("xpose-key").selected_text(now).width(80.0).show_ui(ui, |ui| {
                            for k in 0..12 {
                                let to_flats = doc.use_flats(k - root, dlg.spelling);
                                let name = chart::key_names(minor, to_flats)[k as usize].clone();
                                // The shorter way there: at most 6 half steps up, 5 down.
                                let mut d = (k - root).rem_euclid(12);
                                if d > 6 {
                                    d -= 12;
                                }
                                if ui.selectable_label((root + dlg.semis).rem_euclid(12) == k, name).clicked() {
                                    dlg.semis = d;
                                }
                            }
                        });
                    });
                }
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label("Spell with:");
                    ui.radio_value(&mut dlg.spelling, Spelling::Auto, "As the key suggests");
                    ui.radio_value(&mut dlg.spelling, Spelling::Sharps, "Sharps \u{266F}");
                    ui.radio_value(&mut dlg.spelling, Spelling::Flats, "Flats \u{266D}");
                });
                // What will change.
                ui.add_space(6.0);
                let mut after = doc.clone();
                after.transpose(dlg.semis, dlg.spelling, dlg.only);
                let chords_of = |c: &chart::ChordChart| match dlg.only {
                    Some(i) => {
                        let mut one = c.clone();
                        one.map = vec![chart::MapEntry::new(c.sections.get(i).map(|s| s.id).unwrap_or(0), 1)];
                        one.chords_used()
                    }
                    None => c.chords_used(),
                };
                let (a, b) = (chords_of(doc), chords_of(&after));
                let pairs: Vec<String> = a.iter().zip(b.iter()).take(14).map(|(x, y)| format!("{x} \u{2192} {y}")).collect();
                egui::Frame::new().fill(Color32::from_rgb(30, 32, 38)).corner_radius(4.0).inner_margin(egui::Margin::same(8)).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    if dlg.only.is_none() && !doc.key.trim().is_empty() {
                        ui.label(format!("Key: {} \u{2192} {}", doc.key.trim(), after.key.trim()));
                    }
                    ui.label(if pairs.is_empty() { "No chords to change.".to_string() } else { format!("Chords: {}", pairs.join(",  ")) });
                });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("Cancel").clicked() {
                            cancel = true;
                        }
                        if ui.add_enabled(dlg.semis != 0, egui::Button::new(RichText::new("Transpose").color(Color32::WHITE)).fill(BLUE)).clicked() {
                            ok = true;
                        }
                    });
                });
            });
        if ok {
            st.doc.transpose(dlg.semis, dlg.spelling, dlg.only);
            self.status = format!("Transposed {}.", describe(dlg.semis).to_lowercase());
            return;
        }
        if open && !cancel && !ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            st.xpose = Some(dlg);
        }
    }
}
