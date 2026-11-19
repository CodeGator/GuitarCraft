//! The Add Track window (Track > Add, the + in the track list, and the first
//! thing a new empty document shows): a kind of instrument (Stringed,
//! Keyboards, Vocals, Orchestra, Drums), a group and an instrument from the
//! lists, then the new track's settings: colour, name and short name, notation,
//! strings and tuning, and a sound to hear. For a new empty document the window
//! also has Skip (keep the default guitar track) and Cancel (don't create the
//! document), laid out like Guitar Pro 8's.

use super::App;
use crate::model::*;
use egui::{Align2, Color32, FontFamily, FontId, Pos2, RichText, Sense, Vec2};

const BLUE: Color32 = Color32::from_rgb(76, 140, 245);
const TILE: Color32 = Color32::from_rgb(78, 84, 104);
const LIST_BG: Color32 = Color32::from_rgb(26, 28, 34);

/// An instrument in the lists: what the list says, and what a new track is named.
type Entry = (Instrument, &'static str, &'static str);

/// The groups of instruments of a kind: (group name, instruments).
pub fn groups(kind: InstrumentType) -> Vec<(&'static str, Vec<Entry>)> {
    use Instrument::*;
    match kind {
        InstrumentType::Stringed => vec![
            ("Acoustic Guitar", vec![(SteelGuitar, "Steel", "Steel Guitar"), (NylonGuitar, "Nylon", "Nylon Guitar")]),
            (
                "Electric Guitar",
                vec![(CleanGuitar, "Clean", "Electric Guitar Clean"), (OverdriveGuitar, "Overdrive", "Electric Guitar Overdrive"), (DistortionGuitar, "Distortion", "Electric Guitar Distortion")],
            ),
            ("Bass", vec![(Bass, "Electric Bass", "Electric Bass")]),
            ("Other", vec![(Ukulele, "Ukulele", "Ukulele"), (Banjo, "Banjo", "Banjo")]),
        ],
        InstrumentType::Keyboard => vec![("Keyboards", vec![(Piano, "Grand Piano", "Piano"), (ElectricPiano, "Electric Piano", "Electric Piano"), (Organ, "Organ", "Organ"), (Synth, "Synth Lead", "Synth")])],
        InstrumentType::Vocal => vec![("Vocals", vec![(LeadVocals, "Lead Vocals", "Lead Vocals"), (Choir, "Choir", "Choir")])],
        InstrumentType::Orchestra => vec![
            ("Strings", vec![(Violin, "Violin", "Violin"), (Cello, "Cello", "Cello"), (Strings, "String Ensemble", "Strings")]),
            ("Brass", vec![(Trumpet, "Trumpet", "Trumpet")]),
            ("Woodwind", vec![(Saxophone, "Saxophone", "Saxophone"), (Flute, "Flute", "Flute")]),
        ],
        InstrumentType::Drums => vec![("Drums", vec![(Drums, "Drum Kit", "Drums")])],
    }
}

/// The name a track of an instrument starts with.
pub fn default_name(i: Instrument) -> &'static str {
    for k in InstrumentType::ALL {
        for (_, items) in groups(k) {
            if let Some(e) = items.iter().find(|e| e.0 == i) {
                return e.2;
            }
        }
    }
    i.name()
}

/// A short name from a track name: "Resonator" gives "reso.".
pub fn short_name(name: &str) -> String {
    let word = name.split_whitespace().next().unwrap_or("");
    let cut: String = word.chars().take(4).collect::<String>().to_lowercase();
    if cut.is_empty() { String::new() } else { format!("{cut}.") }
}

/// The window's state.
pub struct AddTrack {
    pub open: bool,
    /// For a new empty document: Create replaces its placeholder track, Skip keeps it,
    /// Cancel discards the document.
    pub new_doc: bool,
    pub kind: InstrumentType,
    pub group: usize,
    pub inst: Instrument,
    pub name: String,
    pub short: String,
    pub tuning: Vec<u8>,
    pub grand: bool,
    pub color: usize,
    pub show_std: bool,
    pub show_tab: bool,
}

impl Default for AddTrack {
    fn default() -> Self {
        let mut a = AddTrack {
            open: false,
            new_doc: false,
            kind: InstrumentType::Stringed,
            group: 0,
            inst: Instrument::SteelGuitar,
            name: String::new(),
            short: String::new(),
            tuning: STANDARD_GUITAR.to_vec(),
            grand: true,
            color: 2,
            show_std: true,
            show_tab: true,
        };
        a.choose(Instrument::SteelGuitar);
        a
    }
}

impl AddTrack {
    /// Chooses an instrument: its kind and group are shown, and its name and tuning filled in.
    pub fn choose(&mut self, i: Instrument) {
        self.kind = i.kind();
        self.group = groups(self.kind).iter().position(|g| g.1.iter().any(|e| e.0 == i)).unwrap_or(0);
        self.inst = i;
        self.name = default_name(i).to_string();
        self.short = short_name(&self.name);
        self.tuning = i.default_tuning();
        self.grand = matches!(i, Instrument::Piano | Instrument::ElectricPiano | Instrument::Organ);
    }

    /// The track the window describes.
    pub fn build(&self) -> Track {
        let name = if self.name.trim().is_empty() { default_name(self.inst).to_string() } else { self.name.trim().to_string() };
        let mut t = Track::of_instrument(self.inst, &name);
        t.short_name = self.short.trim().to_string();
        if t.has_strings() {
            t.tuning = self.tuning.clone();
            t.show_std = self.show_std || !self.show_tab;
            t.show_tab = self.show_tab;
        }
        t.grand_staff = t.kind() == InstrumentType::Keyboard && self.grand;
        t.color = TRACK_COLORS[self.color % TRACK_COLORS.len()];
        t
    }
}

fn list_row(ui: &mut egui::Ui, w: f32, on: bool, text: &str) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(Vec2::new(w, 24.0), Sense::click());
    if on {
        ui.painter().rect_filled(r, 3.0, BLUE);
    } else if resp.hovered() {
        ui.painter().rect_filled(r, 3.0, Color32::from_rgb(48, 52, 62));
    }
    ui.painter().text(Pos2::new(r.left() + 10.0, r.center().y), Align2::LEFT_CENTER, text, FontId::proportional(13.5), Color32::WHITE);
    resp
}

fn toggle(ui: &mut egui::Ui, on: bool, text: &str, tip: &str) -> bool {
    let (r, resp) = ui.allocate_exact_size(Vec2::new(116.0, 26.0), Sense::click());
    ui.painter().rect_filled(r, 4.0, if on { BLUE } else if resp.hovered() { Color32::from_rgb(88, 94, 116) } else { TILE });
    ui.painter().text(r.center(), Align2::CENTER_CENTER, text, FontId::proportional(13.0), Color32::WHITE);
    resp.on_hover_text(tip).clicked()
}

impl App {
    /// Opens the window. `new_doc`: the first thing a new empty document shows.
    pub(super) fn open_add_track(&mut self, new_doc: bool) {
        self.at = AddTrack { open: true, new_doc, ..AddTrack::default() };
    }

    pub(super) fn add_track_window(&mut self, ctx: &egui::Context) {
        if !self.at.open {
            return;
        }
        let mut open = true;
        let (mut create, mut skip, mut cancel, mut hear) = (false, false, false, false);
        let mut at = std::mem::take(&mut self.at);
        egui::Window::new("Add Track")
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.set_width(540.0);
                ui.spacing_mut().item_spacing = Vec2::new(6.0, 7.0);
                ui.label(RichText::new("ADD TRACK").size(18.0).family(FontFamily::Name("bold".into())).color(Color32::from_gray(240)));
                // ---- kinds
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 3.0;
                    let w = (ui.available_width() - 3.0 * (InstrumentType::ALL.len() as f32 - 1.0)) / InstrumentType::ALL.len() as f32;
                    for k in InstrumentType::ALL {
                        let (r, resp) = ui.allocate_exact_size(Vec2::new(w, 46.0), Sense::click());
                        let on = at.kind == k;
                        ui.painter().rect_filled(r, 4.0, if on { BLUE } else if resp.hovered() { Color32::from_rgb(88, 94, 116) } else { TILE });
                        ui.painter().text(r.center(), Align2::CENTER_CENTER, k.name(), FontId::new(14.0, FontFamily::Name("bold".into())), Color32::WHITE);
                        if resp.clicked()
                            && !on
                            && let Some(first) = groups(k).first().and_then(|g| g.1.first())
                        {
                            at.choose(first.0);
                        }
                    }
                });
                // ---- groups and instruments
                let gs = groups(at.kind);
                ui.horizontal_top(|ui| {
                    for col in 0..2 {
                        egui::Frame::new().fill(LIST_BG).inner_margin(egui::Margin::same(3)).show(ui, |ui| {
                            ui.set_width(256.0);
                            ui.set_height(176.0);
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 1.0;
                                if col == 0 {
                                    for (g, (name, items)) in gs.iter().enumerate() {
                                        if list_row(ui, 250.0, at.group == g, name).clicked()
                                            && at.group != g
                                            && let Some(first) = items.first()
                                        {
                                            at.choose(first.0);
                                        }
                                    }
                                } else if let Some((_, items)) = gs.get(at.group) {
                                    for e in items {
                                        if list_row(ui, 250.0, at.inst == e.0, e.1).clicked() && at.inst != e.0 {
                                            at.choose(e.0);
                                        }
                                    }
                                }
                            });
                        });
                    }
                });
                // ---- settings
                let label = gs.get(at.group).and_then(|g| g.1.iter().find(|e| e.0 == at.inst)).map(|e| e.1).unwrap_or("");
                ui.add_space(4.0);
                ui.label(RichText::new(format!("{} SETTINGS", label.to_uppercase())).size(14.0).family(FontFamily::Name("bold".into())).color(Color32::from_gray(235)));
                egui::Grid::new("at-settings").num_columns(2).spacing(Vec2::new(10.0, 7.0)).show(ui, |ui| {
                    ui.label("Information:");
                    ui.horizontal(|ui| {
                        // The track's colour: a chip that opens the palette.
                        let c = TRACK_COLORS[at.color % TRACK_COLORS.len()];
                        let (r, resp) = ui.allocate_exact_size(Vec2::new(28.0, 24.0), Sense::click());
                        ui.painter().rect_filled(r, 4.0, Color32::from_rgb(c[0], c[1], c[2]));
                        ui.painter().rect_stroke(r, 4.0, egui::Stroke::new(1.0, Color32::from_white_alpha(80)), egui::StrokeKind::Inside);
                        egui::Popup::menu(&resp).show(|ui| {
                            ui.horizontal_wrapped(|ui| {
                                ui.set_max_width(190.0);
                                for (k, c) in TRACK_COLORS.iter().enumerate() {
                                    let (r, resp) = ui.allocate_exact_size(Vec2::splat(22.0), Sense::click());
                                    ui.painter().rect_filled(r, 3.0, Color32::from_rgb(c[0], c[1], c[2]));
                                    if at.color == k {
                                        ui.painter().rect_stroke(r, 3.0, egui::Stroke::new(2.0, Color32::WHITE), egui::StrokeKind::Outside);
                                    }
                                    if resp.clicked() {
                                        at.color = k;
                                    }
                                }
                            });
                        });
                        if ui.add(egui::TextEdit::singleline(&mut at.name).desired_width(250.0)).changed() {
                            at.short = short_name(&at.name);
                        }
                        ui.add(egui::TextEdit::singleline(&mut at.short).desired_width(90.0).hint_text("short name"));
                    });
                    ui.end_row();
                    let kind = at.kind;
                    ui.label("Notation:");
                    ui.horizontal(|ui| match kind {
                        InstrumentType::Stringed => {
                            if toggle(ui, at.show_std, "Standard notation", "Show the notes on a staff") {
                                at.show_std = !at.show_std || !at.show_tab;
                            }
                            if toggle(ui, at.show_tab, "Tablature", "Show the notes as fret numbers on the strings") {
                                at.show_tab = !at.show_tab || !at.show_std;
                            }
                        }
                        InstrumentType::Keyboard => {
                            if toggle(ui, !at.grand, "Single staff", "One staff") {
                                at.grand = false;
                            }
                            if toggle(ui, at.grand, "Grand staff", "Treble and bass staves") {
                                at.grand = true;
                            }
                        }
                        InstrumentType::Drums => {
                            ui.label("Percussion staff");
                        }
                        _ => {
                            ui.label("Standard notation");
                        }
                    });
                    ui.end_row();
                    if kind == InstrumentType::Stringed {
                        ui.label("Upper staff:");
                        ui.horizontal(|ui| {
                            let mut n = at.tuning.len();
                            egui::ComboBox::from_id_salt("at-strings").width(96.0).selected_text(format!("{n} strings")).show_ui(ui, |ui| {
                                for k in [4usize, 5, 6, 7] {
                                    ui.selectable_value(&mut n, k, format!("{k} strings"));
                                }
                            });
                            if n != at.tuning.len() {
                                at.tuning = tuning_presets(n)[0].1.clone();
                            }
                            egui::ComboBox::from_id_salt("at-tuning").width(300.0).selected_text(tuning_name(&at.tuning)).show_ui(ui, |ui| {
                                for (name, notes) in tuning_presets(at.tuning.len()) {
                                    if ui.selectable_label(at.tuning == notes, name).clicked() {
                                        at.tuning = notes;
                                    }
                                }
                            });
                        });
                        ui.end_row();
                    }
                    ui.label("Sound:");
                    ui.horizontal(|ui| {
                        let (r, _) = ui.allocate_exact_size(Vec2::new(300.0, 26.0), Sense::hover());
                        ui.painter().rect_filled(r, 4.0, TILE);
                        ui.painter().text(Pos2::new(r.left() + 10.0, r.center().y), Align2::LEFT_CENTER, at.inst.name(), FontId::proportional(13.0), Color32::WHITE);
                        let (pr, presp) = ui.allocate_exact_size(Vec2::new(30.0, 26.0), Sense::click());
                        ui.painter().rect_filled(pr, 4.0, if presp.hovered() { Color32::from_rgb(88, 94, 116) } else { TILE });
                        ui.painter().add(egui::Shape::convex_polygon(
                            vec![Pos2::new(pr.center().x - 4.0, pr.center().y - 6.0), Pos2::new(pr.center().x + 6.0, pr.center().y), Pos2::new(pr.center().x - 4.0, pr.center().y + 6.0)],
                            Color32::WHITE,
                            egui::Stroke::NONE,
                        ));
                        if presp.on_hover_text("Hear the instrument").clicked() {
                            hear = true;
                        }
                    });
                    ui.end_row();
                });
                ui.add_space(6.0);
                ui.horizontal(|ui| ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add(egui::Button::new(RichText::new("Cancel").size(13.5)).min_size(Vec2::new(64.0, 28.0))).clicked() {
                        cancel = true;
                    }
                    if at.new_doc && ui.add(egui::Button::new(RichText::new("Skip").size(13.5)).min_size(Vec2::new(54.0, 28.0))).on_hover_text("Start with a plain guitar track").clicked() {
                        skip = true;
                    }
                    if ui.add(egui::Button::new(RichText::new("Create").size(13.5).color(Color32::WHITE)).fill(BLUE).min_size(Vec2::new(66.0, 28.0))).clicked() {
                        create = true;
                    }
                }));
            });
        if hear {
            let t = at.build();
            self.preview_new_track(&t);
        }
        let new_doc = at.new_doc;
        let track = create.then(|| at.build());
        self.at = at;
        if !open || cancel {
            self.at.open = false;
            if new_doc {
                // No document is made.
                self.close_song();
            }
        } else if skip {
            self.at.open = false;
        } else if let Some(t) = track {
            self.at.open = false;
            if new_doc {
                self.song.tracks = vec![t];
                self.song.normalize();
                self.track = 0;
                self.clamp_cursor();
                self.total_secs = self.song_seconds();
                self.status = "Track added. Click a string and type fret numbers.".into();
            } else {
                self.edit(move |s, _, _, _, _| s.tracks.push(t));
                self.track = self.song.tracks.len() - 1;
                self.clamp_cursor();
            }
        }
    }

    /// Plays the open strings (or a chord, or the drums) of a track that isn't in the song yet.
    fn preview_new_track(&mut self, t: &Track) {
        if !self.sound_allowed {
            return;
        }
        self.ensure_audio();
        let Some(a) = &self.audio else { return };
        if a.error.is_some() {
            return;
        }
        let sr = a.sample_rate();
        let notes: Vec<Note> = match t.kind() {
            InstrumentType::Stringed => (0..t.strings()).rev().map(|s| Note::new(s as u8, 0)).collect(),
            InstrumentType::Drums => [(0u8, 36u8), (1, 38), (2, 42), (3, 38)].iter().map(|&(s, p)| Note::new(s, p)).collect(),
            _ => [60u8, 64, 67, 72].iter().enumerate().map(|(s, &p)| Note::new(s as u8, p)).collect(),
        };
        let ti = self.track;
        if let Ok(mut st) = a.state.lock() {
            for (k, n) in notes.iter().enumerate() {
                let mut ev = crate::audio::note_preview(t, ti, n, sr);
                ev.start = (k as f32 * sr * 0.3) as u64;
                ev.len = (sr * 0.9) as u64;
                st.preview.push(ev);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_instrument_is_in_the_lists_once() {
        for i in Instrument::ALL {
            let n: usize = InstrumentType::ALL.iter().flat_map(|k| groups(*k)).map(|g| g.1.iter().filter(|e| e.0 == i).count()).sum();
            assert_eq!(n, 1, "{i:?}");
            assert!(groups(i.kind()).iter().any(|g| g.1.iter().any(|e| e.0 == i)), "{i:?} is under its own kind");
        }
    }

    #[test]
    fn choosing_fills_in_names_and_builds_the_track() {
        let mut a = AddTrack::default();
        assert_eq!((a.name.as_str(), a.short.as_str()), ("Steel Guitar", "stee."));
        a.choose(Instrument::Piano);
        assert_eq!((a.kind, a.group, a.name.as_str()), (InstrumentType::Keyboard, 0, "Piano"));
        let t = a.build();
        assert_eq!(t.instrument, Instrument::Piano);
        assert!(t.grand_staff);
        a.choose(Instrument::Bass);
        a.show_tab = false;
        a.show_std = false;
        let t = a.build();
        assert!(t.show_std && !t.show_tab, "one kind of notation always stays on");
        assert_eq!(t.tuning, STANDARD_BASS.to_vec());
        assert_eq!(short_name("Resonator"), "reso.");
    }
}
