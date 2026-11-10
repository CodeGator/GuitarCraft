//! Small popup dialogs for bar and beat settings (key and time signature,
//! section, tempo, repeats, endings, text, bend, trill, grace note), opened from
//! the menus, the palette, shortcuts or by clicking on the score.

use super::App;
use egui::{Align2, Key, RichText, Vec2};

#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Popup {
    TimeSig,
    KeySig,
    Section,
    Tempo,
    Repeat,
    Endings,
    Text,
    Bend,
    Trill,
    Grace,
}

impl Popup {
    fn title(self) -> &'static str {
        match self {
            Popup::TimeSig => "Time Signature",
            Popup::KeySig => "Key Signature",
            Popup::Section => "Section",
            Popup::Tempo => "Tempo",
            Popup::Repeat => "Repeat Close",
            Popup::Endings => "Alternate Endings",
            Popup::Text => "Text",
            Popup::Bend => "Bend",
            Popup::Trill => "Trill",
            Popup::Grace => "Grace Note",
        }
    }
    fn needs_note(self) -> bool {
        matches!(self, Popup::Bend | Popup::Trill | Popup::Grace)
    }
}

pub fn key_name(k: i8) -> String {
    const SHARP: [&str; 8] = [
        "C major / A minor",
        "G major / E minor",
        "D major / B minor",
        "A major / F\u{266F} minor",
        "E major / C\u{266F} minor",
        "B major / G\u{266F} minor",
        "F\u{266F} major / D\u{266F} minor",
        "C\u{266F} major / A\u{266F} minor",
    ];
    const FLAT: [&str; 8] = [
        "C major / A minor",
        "F major / D minor",
        "B\u{266D} major / G minor",
        "E\u{266D} major / C minor",
        "A\u{266D} major / F minor",
        "D\u{266D} major / B\u{266D} minor",
        "G\u{266D} major / E\u{266D} minor",
        "C\u{266D} major / A\u{266D} minor",
    ];
    if k >= 0 {
        SHARP[k.min(7) as usize].to_string()
    } else {
        FLAT[(-k).min(7) as usize].to_string()
    }
}

fn bend_name(q: u8) -> String {
    match q {
        0 => "None".into(),
        1 => "\u{00BC} tone".into(),
        2 => "\u{00BD} tone".into(),
        4 => "Full".into(),
        6 => "1\u{00BD} tones".into(),
        8 => "2 tones".into(),
        q => format!("{} tones", q as f32 / 4.0),
    }
}

impl App {
    /// Opens a popup with a copy of the current bar, beat and note to edit.
    pub(super) fn open_popup(&mut self, kind: Popup) {
        let beat = self.cur_beat().clone();
        let note = if self.std_cursor.is_some() {
            self.note_at_std()
        } else {
            beat.note_on(self.string as u8).cloned()
        }
        .or_else(|| beat.notes.first().cloned());
        if kind.needs_note() && note.is_none() {
            self.status = "Put a note on this beat first.".into();
            return;
        }
        let mut note = note.unwrap_or_default();
        match kind {
            Popup::Bend if note.bend == 0 => note.bend = 4,
            Popup::Trill if note.trill == 0 => note.trill = (note.fret + 2).min(30),
            Popup::Grace if note.grace.is_none() => note.grace = Some(note.fret.saturating_sub(2)),
            _ => {}
        }
        let mut mb = self.song.master[self.bar].clone();
        if kind == Popup::Tempo && self.bar == 0 && mb.tempo.is_none() {
            mb.tempo = Some(self.song.tempo);
        }
        if kind == Popup::Repeat && mb.repeat_close == 0 {
            mb.repeat_close = 2;
        }
        self.pop_text = match kind {
            Popup::Section => mb.marker.clone().unwrap_or_else(|| {
                let k = self
                    .song
                    .master
                    .iter()
                    .filter(|m| m.marker.is_some())
                    .count();
                format!("Section {}", (b'A' + (k % 26) as u8) as char)
            }),
            Popup::Text => beat.text.clone().unwrap_or_default(),
            _ => String::new(),
        };
        self.pop_bar = mb;
        self.pop_beat = beat;
        self.pop_note = note;
        self.pop_until_next = true;
        self.popup = Some(kind);
    }

    pub(super) fn popup_window(&mut self, ctx: &egui::Context) {
        let Some(kind) = self.popup else { return };
        let mut ok = false;
        let mut cancel = false;
        let mut remove = false;
        let bar_no = self.bar + 1;
        egui::Window::new(kind.title())
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_TOP, Vec2::new(0.0, 140.0))
            .show(ctx, |ui| {
                ui.set_min_width(300.0);
                match kind {
                    Popup::TimeSig => {
                        ui.label(RichText::new(format!("Bar {bar_no}")).weak());
                        ui.horizontal(|ui| {
                            let r =
                                ui.add(egui::DragValue::new(&mut self.pop_bar.num).range(1..=32));
                            if ui.memory(|m| m.focused().is_none()) {
                                r.request_focus();
                            }
                            ui.label(RichText::new("/").size(18.0));
                            egui::ComboBox::from_id_salt("pop_den")
                                .width(50.0)
                                .selected_text(format!("{}", self.pop_bar.den))
                                .show_ui(ui, |ui| {
                                    for d in [1u8, 2, 4, 8, 16, 32] {
                                        ui.selectable_value(
                                            &mut self.pop_bar.den,
                                            d,
                                            format!("{d}"),
                                        );
                                    }
                                });
                        });
                        ui.horizontal_wrapped(|ui| {
                            for (n, d) in [
                                (2u8, 4u8),
                                (3, 4),
                                (4, 4),
                                (5, 4),
                                (6, 8),
                                (7, 8),
                                (9, 8),
                                (12, 8),
                            ] {
                                if ui
                                    .selectable_label(
                                        self.pop_bar.num == n && self.pop_bar.den == d,
                                        format!("{n}/{d}"),
                                    )
                                    .clicked()
                                {
                                    self.pop_bar.num = n;
                                    self.pop_bar.den = d;
                                }
                            }
                        });
                        ui.checkbox(&mut self.pop_until_next, "Until the next change");
                    }
                    Popup::KeySig => {
                        ui.label(RichText::new("KEY SIGNATURE").strong().size(16.0));
                        ui.label(RichText::new(format!("Bar {bar_no}")).weak());
                        // The key on a staff, as it will look in the score.
                        let bass = self.song.tracks[self.track].instrument.is_bass();
                        key_staff(ui, self.pop_bar.key, bass, self.music_font);
                        egui::Grid::new("keysig")
                            .num_columns(2)
                            .spacing(Vec2::new(10.0, 8.0))
                            .show(ui, |ui| {
                                ui.label("Mode:");
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 0.0;
                                    for (minor, lab) in [(true, "Minor"), (false, "Major")] {
                                        let on = self.pop_bar.key_minor == minor;
                                        let b =
                                            egui::Button::new(RichText::new(lab).color(if on {
                                                egui::Color32::WHITE
                                            } else {
                                                egui::Color32::from_gray(200)
                                            }))
                                            .fill(if on {
                                                egui::Color32::from_rgb(61, 123, 224)
                                            } else {
                                                egui::Color32::from_rgb(60, 64, 74)
                                            })
                                            .min_size(Vec2::new(54.0, 24.0));
                                        if ui.add(b).clicked() {
                                            self.pop_bar.key_minor = minor;
                                        }
                                    }
                                });
                                ui.end_row();
                                ui.label("Note:");
                                let minor = self.pop_bar.key_minor;
                                egui::ComboBox::from_id_salt("pop_key")
                                    .width(150.0)
                                    .selected_text(key_label(self.pop_bar.key, minor))
                                    .show_ui(ui, |ui| {
                                        for k in -7i8..=7 {
                                            ui.selectable_value(
                                                &mut self.pop_bar.key,
                                                k,
                                                key_label(k, minor),
                                            );
                                        }
                                    });
                                ui.end_row();
                            });
                        ui.label(RichText::new(key_name(self.pop_bar.key)).weak());
                        ui.checkbox(&mut self.pop_until_next, "Until the next change");
                    }
                    Popup::Section => {
                        ui.label(RichText::new(format!("Section starting at bar {bar_no}")).weak());
                        let r = ui.add(
                            egui::TextEdit::singleline(&mut self.pop_text).desired_width(280.0),
                        );
                        r.request_focus();
                        ui.horizontal_wrapped(|ui| {
                            for s in [
                                "Intro",
                                "Verse",
                                "Pre-Chorus",
                                "Chorus",
                                "Bridge",
                                "Solo",
                                "Interlude",
                                "Outro",
                            ] {
                                if ui.small_button(s).clicked() {
                                    self.pop_text = s.to_string();
                                }
                            }
                        });
                        ui.label(
                            RichText::new("Write the name in [brackets] to show it in a frame.")
                                .small()
                                .weak(),
                        );
                    }
                    Popup::Tempo => {
                        if self.bar == 0 {
                            ui.label(RichText::new("Tempo of the song").weak());
                            let mut t = self.pop_bar.tempo.unwrap_or(self.song.tempo);
                            ui.add(egui::DragValue::new(&mut t).range(20..=400).suffix(" bpm"));
                            self.pop_bar.tempo = Some(t);
                        } else {
                            ui.label(RichText::new(format!("Tempo change at bar {bar_no}")).weak());
                            let mut on = self.pop_bar.tempo.is_some();
                            ui.horizontal(|ui| {
                                if ui.checkbox(&mut on, "Change to").changed() {
                                    self.pop_bar.tempo = if on {
                                        Some(self.song.tempo_at(self.bar))
                                    } else {
                                        None
                                    };
                                }
                                if let Some(t) = &mut self.pop_bar.tempo {
                                    ui.add(egui::DragValue::new(t).range(20..=400).suffix(" bpm"));
                                }
                            });
                        }
                    }
                    Popup::Repeat => {
                        ui.label(RichText::new(format!("Repeat closing at bar {bar_no}")).weak());
                        ui.horizontal(|ui| {
                            ui.label("Play");
                            ui.add(
                                egui::DragValue::new(&mut self.pop_bar.repeat_close).range(2..=16),
                            );
                            ui.label("times");
                        });
                        if ui.button("Remove the repeat").clicked() {
                            remove = true;
                        }
                    }
                    Popup::Endings => {
                        ui.label(
                            RichText::new(format!("Bar {bar_no} is played on these passes:"))
                                .weak(),
                        );
                        ui.horizontal_wrapped(|ui| {
                            for k in 0..8u8 {
                                let mut on = self.pop_bar.endings & (1 << k) != 0;
                                if ui.checkbox(&mut on, format!("{}.", k + 1)).changed() {
                                    self.pop_bar.endings ^= 1 << k;
                                }
                            }
                        });
                    }
                    Popup::Text => {
                        let r = ui.add(
                            egui::TextEdit::singleline(&mut self.pop_text)
                                .desired_width(280.0)
                                .hint_text("e.g. rit., a tempo, slowly"),
                        );
                        r.request_focus();
                    }
                    Popup::Bend => {
                        ui.label(
                            RichText::new(format!(
                                "String {}, fret {}",
                                self.pop_note.string + 1,
                                self.pop_note.fret
                            ))
                            .weak(),
                        );
                        egui::Grid::new("bendg").num_columns(2).show(ui, |ui| {
                            ui.label("Type:");
                            ui.horizontal_wrapped(|ui| {
                                for (v, l) in [
                                    (0u8, "Bend"),
                                    (1, "Bend + release"),
                                    (2, "Pre-bend"),
                                    (3, "Pre-bend + release"),
                                ] {
                                    ui.radio_value(&mut self.pop_note.bend_kind, v, l);
                                }
                            });
                            ui.end_row();
                            ui.label("Amount:");
                            ui.horizontal_wrapped(|ui| {
                                for q in [1u8, 2, 4, 6, 8] {
                                    ui.radio_value(&mut self.pop_note.bend, q, bend_name(q));
                                }
                            });
                            ui.end_row();
                        });
                        if ui.button("Remove the bend").clicked() {
                            remove = true;
                        }
                    }
                    Popup::Trill => {
                        ui.label(
                            RichText::new(format!(
                                "String {}, fret {}",
                                self.pop_note.string + 1,
                                self.pop_note.fret
                            ))
                            .weak(),
                        );
                        ui.horizontal(|ui| {
                            ui.label("Trill with fret");
                            ui.add(egui::DragValue::new(&mut self.pop_note.trill).range(1..=30));
                        });
                        if ui.button("Remove the trill").clicked() {
                            remove = true;
                        }
                    }
                    Popup::Grace => {
                        ui.label(
                            RichText::new(format!(
                                "Before string {}, fret {}",
                                self.pop_note.string + 1,
                                self.pop_note.fret
                            ))
                            .weak(),
                        );
                        let mut g = self.pop_note.grace.unwrap_or(0);
                        ui.horizontal(|ui| {
                            ui.label("Grace note on fret");
                            ui.add(egui::DragValue::new(&mut g).range(0..=30));
                        });
                        self.pop_note.grace = Some(g);
                        if ui.button("Remove the grace note").clicked() {
                            remove = true;
                        }
                    }
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button(RichText::new("  OK  ").strong()).clicked() {
                        ok = true;
                    }
                    if ui.button("Cancel").clicked() {
                        cancel = true;
                    }
                });
            });
        if ctx.input(|i| i.key_pressed(Key::Enter)) {
            ok = true;
        }
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            cancel = true;
        }
        if remove {
            self.remove_popup_item(kind);
            self.popup = None;
        } else if ok {
            self.apply_popup(kind);
            self.popup = None;
        } else if cancel {
            self.popup = None;
        }
    }

    fn remove_popup_item(&mut self, kind: Popup) {
        let s = self.pop_note.string;
        match kind {
            Popup::Repeat => self.bar_edit(|m| m.repeat_close = 0),
            Popup::Bend => self.edit(move |so, t, b, j, _| {
                if let Some(n) = so.tracks[t].bars[b].beats[j].note_on_mut(s) {
                    n.bend = 0;
                }
            }),
            Popup::Trill => self.edit(move |so, t, b, j, _| {
                if let Some(n) = so.tracks[t].bars[b].beats[j].note_on_mut(s) {
                    n.trill = 0;
                }
            }),
            Popup::Grace => self.edit(move |so, t, b, j, _| {
                if let Some(n) = so.tracks[t].bars[b].beats[j].note_on_mut(s) {
                    n.grace = None;
                }
            }),
            _ => {}
        }
    }

    fn apply_popup(&mut self, kind: Popup) {
        let mb = self.pop_bar.clone();
        let bar = self.bar;
        let until = self.pop_until_next;
        let text = self.pop_text.trim().to_string();
        let note = self.pop_note.clone();
        match kind {
            Popup::TimeSig => self.edit(move |s, _, _, _, _| {
                let old = (s.master[bar].num, s.master[bar].den);
                let mut i = bar;
                while i < s.master.len()
                    && (i == bar || (until && (s.master[i].num, s.master[i].den) == old))
                {
                    s.master[i].num = mb.num;
                    s.master[i].den = mb.den;
                    i += 1;
                }
            }),
            Popup::KeySig => self.edit(move |s, _, _, _, _| {
                let old = (s.master[bar].key, s.master[bar].key_minor);
                let mut i = bar;
                while i < s.master.len()
                    && (i == bar || (until && (s.master[i].key, s.master[i].key_minor) == old))
                {
                    s.master[i].key = mb.key;
                    s.master[i].key_minor = mb.key_minor;
                    i += 1;
                }
            }),
            Popup::Section => {
                self.bar_edit(move |m| m.marker = if text.is_empty() { None } else { Some(text) })
            }
            Popup::Tempo => {
                if bar == 0 {
                    let t = mb.tempo.unwrap_or(120);
                    self.edit(move |s, _, _, _, _| {
                        s.tempo = t;
                        s.master[0].tempo = None;
                    });
                } else {
                    self.bar_edit(move |m| m.tempo = mb.tempo);
                }
                self.total_secs = self.song_seconds();
            }
            Popup::Repeat => self.bar_edit(move |m| m.repeat_close = mb.repeat_close.max(2)),
            Popup::Endings => self.bar_edit(move |m| m.endings = mb.endings),
            Popup::Text => self.edit(move |s, t, b, j, _| {
                s.tracks[t].bars[b].beats[j].text = if text.is_empty() { None } else { Some(text) }
            }),
            Popup::Bend | Popup::Trill | Popup::Grace => {
                let s = note.string;
                self.edit(move |so, t, b, j, _| {
                    if let Some(n) = so.tracks[t].bars[b].beats[j].note_on_mut(s) {
                        match kind {
                            Popup::Bend => {
                                n.bend = note.bend;
                                n.bend_kind = note.bend_kind;
                            }
                            Popup::Trill => n.trill = note.trill,
                            _ => n.grace = note.grace,
                        }
                    }
                });
            }
        }
    }
}

/// "G (#)", "Bb (bb)", "F#m (######)": the key's tonic and its accidentals, like the original's list.
pub fn key_label(k: i8, minor: bool) -> String {
    const MAJOR: [&str; 15] = [
        "Cb", "Gb", "Db", "Ab", "Eb", "Bb", "F", "C", "G", "D", "A", "E", "B", "F#", "C#",
    ];
    const MINOR: [&str; 15] = [
        "Abm", "Ebm", "Bbm", "Fm", "Cm", "Gm", "Dm", "Am", "Em", "Bm", "F#m", "C#m", "G#m", "D#m",
        "A#m",
    ];
    let i = (k.clamp(-7, 7) + 7) as usize;
    let name = if minor { MINOR[i] } else { MAJOR[i] };
    match k {
        0 => name.to_string(),
        k if k > 0 => format!("{name} ({})", "#".repeat(k as usize)),
        k => format!("{name} ({})", "b".repeat((-k) as usize)),
    }
}

/// A staff with a clef and the sharps or flats of a key, drawn light on the dialog.
fn key_staff(ui: &mut egui::Ui, key: i8, bass: bool, music_font: bool) {
    use egui::{Color32, FontFamily, FontId, Pos2, Sense, Stroke};
    let (r, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width().max(240.0), 78.0),
        Sense::hover(),
    );
    let p = ui.painter();
    let ls = 8.0;
    let top = r.top() + 22.0;
    let ink = Color32::from_gray(235);
    let width = 120.0;
    let left = r.center().x - width * 0.5;
    for i in 0..5 {
        let y = top + i as f32 * ls;
        p.line_segment(
            [Pos2::new(left, y), Pos2::new(left + width, y)],
            Stroke::new(1.0, Color32::from_gray(200)),
        );
    }
    let bottom = top + 4.0 * ls;
    let music = |s: f32| FontId::new(s, FontFamily::Name("music".into()));
    if music_font {
        if bass {
            p.text(
                Pos2::new(left + 16.0, top + ls),
                Align2::CENTER_CENTER,
                "\u{1D122}",
                music(ls * 4.2),
                ink,
            );
        } else {
            p.text(
                Pos2::new(left + 16.0, top + 2.2 * ls),
                Align2::CENTER_CENTER,
                "\u{1D11E}",
                music(ls * 5.6),
                ink,
            );
        }
    } else {
        p.text(
            Pos2::new(left + 16.0, top + 2.0 * ls),
            Align2::CENTER_CENTER,
            if bass { "F" } else { "G" },
            FontId::proportional(ls * 3.5),
            ink,
        );
    }
    let glyph = if key > 0 { "\u{266F}" } else { "\u{266D}" };
    for (i, pos) in crate::score::key_positions(key).iter().enumerate() {
        let pos = if bass { pos - 2 } else { *pos };
        let y = bottom - pos as f32 * ls * 0.5;
        p.text(
            Pos2::new(left + 38.0 + i as f32 * 10.0, y),
            Align2::CENTER_CENTER,
            glyph,
            music(ls * 2.4),
            ink,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_labels() {
        assert_eq!(key_label(1, false), "G (#)");
        assert_eq!(key_label(-2, false), "Bb (bb)");
        assert_eq!(key_label(0, false), "C");
        assert_eq!(key_label(0, true), "Am");
        assert_eq!(key_label(-7, false), "Cb (bbbbbbb)");
    }
}
