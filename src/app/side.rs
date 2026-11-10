//! The edition palette (left) and the settings panel (right).

use super::panels::BLUE;
use super::widgets::*;
use super::{App, InspectorTab};
use crate::model::*;
use egui::{
    Align2, Color32, FontFamily, FontId, Pos2, Rect, RichText, Sense, Stroke as PStroke, Ui, Vec2,
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

fn divider(ui: &mut Ui) {
    ui.add_space(5.0);
    let r = ui.available_rect_before_wrap();
    ui.painter().line_segment(
        [Pos2::new(r.left(), r.top()), Pos2::new(r.right(), r.top())],
        PStroke::new(1.0, Color32::from_gray(70)),
    );
    ui.add_space(6.0);
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

    pub(super) fn palette(&mut self, ui: &mut Ui) {
        let cur = self.cur_beat().clone();
        let mb = self.song.master[self.bar].clone();
        let n = cur
            .note_on(self.string as u8)
            .or(cur.notes.first())
            .cloned()
            .unwrap_or_default();
        let has = !cur.is_rest();
        ui.spacing_mut().item_spacing = Vec2::new(2.0, 3.0);
        let sym = |s: f32| FontId::new(s, FontFamily::Name("music".into()));
        let bolds = |s: f32| FontId::new(s, FontFamily::Name("bold".into()));
        let ital = |s: f32| FontId::new(s, FontFamily::Name("serif-bi".into()));
        let serif = |s: f32| FontId::new(s, FontFamily::Name("serif".into()));

        // Voices 1-4.
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 3.0;
            for v in 0..VOICES {
                let used = self.song.tracks[self.track].bars[self.bar].voice_used(v);
                let on = self.voice == v;
                let (r, resp) = ui.allocate_exact_size(Vec2::new(24.0, 24.0), Sense::click());
                let fill = if on {
                    Color32::from_rgb(240, 160, 50)
                } else if resp.hovered() {
                    Color32::from_gray(80)
                } else {
                    Color32::from_gray(62)
                };
                ui.painter().rect_filled(r, 4.0, fill);
                ui.painter().text(
                    r.center(),
                    Align2::CENTER_CENTER,
                    format!("{}", v + 1),
                    FontId::new(13.0, FontFamily::Name("bold".into())),
                    if on {
                        Color32::BLACK
                    } else {
                        Color32::from_gray(230)
                    },
                );
                if used && !on {
                    ui.painter().circle_filled(
                        Pos2::new(r.right() - 4.0, r.top() + 4.0),
                        2.5,
                        VOICE_COLORS[v],
                    );
                }
                if resp
                    .on_hover_text(format!("Edit voice {} (Ctrl+{})", v + 1, v + 1))
                    .clicked()
                {
                    self.set_voice(v);
                }
            }
        });
        ui.add_space(2.0);
        // Lyrics, chords and text.
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            for (k, lab, tip) in [
                (0, "Lyrics", "Lyrics editor"),
                (1, "Chords", "Chord name and diagram (A)"),
                (2, "Text", "Text above this beat (T)"),
            ] {
                if ui
                    .add(
                        egui::Button::new(RichText::new(lab).size(12.0))
                            .min_size(Vec2::new(64.0, 22.0)),
                    )
                    .on_hover_text(tip)
                    .clicked()
                {
                    match k {
                        0 => self.lyrics_open = true,
                        1 => self.open_chord_tool(),
                        _ => self.open_popup(super::popups::Popup::Text),
                    }
                }
            }
        });
        divider(ui);

        // Bar and score symbols.
        ui.horizontal_wrapped(|ui| {
            let ks = if mb.key > 0 {
                format!("{}\u{266F}", mb.key)
            } else if mb.key < 0 {
                format!("{}\u{266D}", -mb.key)
            } else {
                "\u{266F}\u{266D}".to_string()
            };
            if glyph_button(
                ui,
                mb.key != 0,
                &ks,
                sym(13.0),
                "Key signature\u{2026} (Ctrl+K)",
            )
            .clicked()
            {
                self.open_popup(super::popups::Popup::KeySig);
            }
            let ts = format!("{}\n{}", mb.num, mb.den);
            if glyph_button(
                ui,
                false,
                &ts,
                bolds(10.0),
                "Time signature\u{2026} (Ctrl+T)",
            )
            .clicked()
            {
                self.open_popup(super::popups::Popup::TimeSig);
            }
            if glyph_button(
                ui,
                mb.triplet_feel,
                "\u{266B}\u{00B3}",
                sym(14.0),
                "Triplet feel (swing eighths) from this bar",
            )
            .clicked()
            {
                self.bar_edit(|m| m.triplet_feel = !m.triplet_feel);
            }
            if glyph_button(
                ui,
                mb.double_bar,
                "\u{2016}",
                FontId::proportional(18.0),
                "Double bar line",
            )
            .clicked()
            {
                self.bar_edit(|m| m.double_bar = !m.double_bar);
            }
            if glyph_button(ui, mb.repeat_open, "\u{1D106}", sym(20.0), "Start repeat").clicked() {
                self.bar_edit(|m| m.repeat_open = !m.repeat_open);
            }
            if glyph_button(
                ui,
                mb.repeat_close > 0,
                "\u{1D107}",
                sym(20.0),
                "End repeat (plays twice)",
            )
            .clicked()
            {
                self.bar_edit(|m| m.repeat_close = if m.repeat_close > 0 { 0 } else { 2 });
            }
            let el = match mb.endings {
                0 => "1.".to_string(),
                e => (0..8)
                    .filter(|k| e & (1 << k) != 0)
                    .map(|k| format!("{}.", k + 1))
                    .collect::<Vec<_>>()
                    .join(""),
            };
            if glyph_button(
                ui,
                mb.endings != 0,
                &el,
                serif(12.0),
                "Alternate ending: click to cycle 1st, 2nd, 3rd, 4th time, off",
            )
            .clicked()
            {
                self.bar_edit(|m| {
                    m.endings = match m.endings {
                        0 => 1,
                        1 => 2,
                        2 => 4,
                        4 => 8,
                        _ => 0,
                    }
                });
            }
            if glyph_button(
                ui,
                mb.marker.is_some(),
                "A",
                bolds(13.0),
                "Section\u{2026} (Shift+Ins)",
            )
            .clicked()
            {
                self.open_popup(super::popups::Popup::Section);
            }
            if glyph_button(
                ui,
                mb.tempo.is_some(),
                "\u{2669}=",
                sym(13.0),
                "Tempo\u{2026}",
            )
            .clicked()
            {
                self.open_automations(self.bar);
            }
            for (v, lab, tip) in [
                (1i8, "8va", "Written an octave higher (8va)"),
                (-1, "8vb", "Written an octave lower (8vb)"),
                (2, "15ma", "Two octaves higher (15ma)"),
                (-2, "15mb", "Two octaves lower (15mb)"),
            ] {
                if glyph_button(ui, cur.ottava == v, lab, ital(10.5), tip).clicked() {
                    self.beat_edit(|b| b.ottava = if b.ottava == v { 0 } else { v });
                }
            }
            if glyph_button(ui, cur.fermata, "\u{1D110}", sym(18.0), "Fermata (hold)").clicked() {
                self.beat_edit(|b| b.fermata = !b.fermata);
            }
            if glyph_button(
                ui,
                false,
                "+",
                FontId::proportional(18.0),
                "Insert a bar after this one",
            )
            .clicked()
            {
                self.insert_bar(self.bar + 1);
            }
            if glyph_button(
                ui,
                false,
                "\u{2212}",
                FontId::proportional(18.0),
                "Delete this bar",
            )
            .clicked()
            {
                self.delete_bar(self.bar);
            }
        });
        divider(ui);

        // Note values, rest, dots, tuplets, tie, accidentals, grace notes.
        ui.horizontal_wrapped(|ui| {
            for d in Dur::ALL {
                if note_icon_button(ui, d, cur.dur == d)
                    .on_hover_text(format!("{} (+ / − change it)", d.name()))
                    .clicked()
                {
                    self.set_duration(d);
                }
            }
            if glyph_button(ui, cur.is_rest(), "\u{1D13D}", sym(20.0), "Rest (R)").clicked()
                && !cur.is_rest()
            {
                self.key(egui::Key::R, egui::Modifiers::NONE, 0.0);
            }
        });
        ui.horizontal_wrapped(|ui| {
            if glyph_button(
                ui,
                cur.dotted && !cur.double_dot,
                "\u{2022}",
                FontId::proportional(13.0),
                "Dotted note (.)",
            )
            .clicked()
            {
                self.beat_edit(|b| {
                    b.dotted = !(b.dotted && !b.double_dot);
                    b.double_dot = false;
                });
            }
            if glyph_button(
                ui,
                cur.double_dot,
                "\u{2022}\u{2022}",
                FontId::proportional(13.0),
                "Double-dotted note",
            )
            .clicked()
            {
                self.beat_edit(|b| {
                    b.double_dot = !b.double_dot;
                    b.dotted = false;
                });
            }
            for (k, tip) in [
                (3u8, "Triplet (T)"),
                (5, "Quintuplet"),
                (6, "Sextuplet"),
                (7, "Septuplet"),
                (9, "Nonuplet"),
            ] {
                if glyph_button(
                    ui,
                    cur.tuplet == k,
                    &format!("-{k}-"),
                    FontId::proportional(10.5),
                    tip,
                )
                .clicked()
                {
                    self.beat_edit(|b| b.tuplet = if b.tuplet == k { 0 } else { k });
                }
            }
            if glyph_button(
                ui,
                n.tie,
                "\u{203F}",
                FontId::proportional(18.0),
                "Tie to the previous note (I)",
            )
            .clicked()
            {
                self.toggle_note_flag("Tie", |n| &mut n.tie);
            }
        });
        ui.add_enabled_ui(has, |ui| {
            ui.horizontal_wrapped(|ui| {
                for (v, lab, tip) in [
                    (0u8, "\u{266E}", "Spell automatically from the key"),
                    (1, "\u{266F}", "Spell with sharps"),
                    (2, "\u{266D}", "Spell with flats"),
                ] {
                    if glyph_button(ui, has && n.spell == v, lab, sym(15.0), tip).clicked() {
                        self.note_edit(|x| x.spell = v);
                    }
                }
                if glyph_button(
                    ui,
                    n.grace.is_some(),
                    "\u{266A}",
                    sym(12.0),
                    "Grace note before this note (two frets lower; change it in the Beat tab)",
                )
                .clicked()
                {
                    self.note_edit(|x| {
                        x.grace = if x.grace.is_some() {
                            None
                        } else {
                            Some(x.fret.saturating_sub(2))
                        }
                    });
                }
            });
        });
        divider(ui);

        // Dynamics and hairpins.
        ui.horizontal_wrapped(|ui| {
            for d in Dynamic::ALL {
                if glyph_button(
                    ui,
                    cur.dynamic == d && has,
                    d.label(),
                    ital(14.0),
                    "Loudness of this beat",
                )
                .clicked()
                {
                    self.beat_edit(|b| b.dynamic = d);
                }
            }
            for (v, lab, tip) in [
                (1u8, "<", "Crescendo (getting louder)"),
                (2, ">", "Decrescendo (getting softer)"),
            ] {
                if glyph_button(ui, cur.hairpin == v, lab, FontId::proportional(18.0), tip)
                    .clicked()
                {
                    self.beat_edit(|b| b.hairpin = if b.hairpin == v { 0 } else { v });
                }
            }
        });
        divider(ui);

        // Note effects.
        ui.add_enabled_ui(has, |ui| {
            ui.horizontal_wrapped(|ui| {
                let items: [(&str, FontId, &str, bool, fn(&mut Note) -> &mut bool); 14] = [
                    (
                        "( )",
                        FontId::proportional(13.0),
                        "Ghost note (G)",
                        n.ghost,
                        |n| &mut n.ghost,
                    ),
                    (">", bolds(16.0), "Accent (A)", n.accent, |n| &mut n.accent),
                    ("^", bolds(16.0), "Heavy accent", n.heavy_accent, |n| {
                        &mut n.heavy_accent
                    }),
                    (
                        "\u{2022}",
                        FontId::proportional(9.0),
                        "Staccato",
                        n.staccato,
                        |n| &mut n.staccato,
                    ),
                    (
                        "let\nring",
                        FontId::new(8.5, FontFamily::Name("italic".into())),
                        "Let ring (L)",
                        n.let_ring,
                        |n| &mut n.let_ring,
                    ),
                    (
                        "P.M.",
                        FontId::new(10.0, FontFamily::Name("italic".into())),
                        "Palm mute (P)",
                        n.palm_mute,
                        |n| &mut n.palm_mute,
                    ),
                    ("X", bolds(16.0), "Dead note (X)", n.dead, |n| &mut n.dead),
                    (
                        "\u{25C7}",
                        sym(15.0),
                        "Natural harmonic (O)",
                        n.harmonic,
                        |n| &mut n.harmonic,
                    ),
                    (
                        "P.H.",
                        FontId::new(9.5, FontFamily::Name("italic".into())),
                        "Pinch (artificial) harmonic",
                        n.pinch_harmonic,
                        |n| &mut n.pinch_harmonic,
                    ),
                    (
                        "H",
                        bolds(14.0),
                        "Hammer-on / pull-off (H)",
                        n.hammer,
                        |n| &mut n.hammer,
                    ),
                    (
                        "\u{27CB}",
                        sym(16.0),
                        "Slide to the next note (S)",
                        n.slide,
                        |n| &mut n.slide,
                    ),
                    (
                        "\u{2197}",
                        FontId::proportional(15.0),
                        "Slide in from below",
                        n.slide_in,
                        |n| &mut n.slide_in,
                    ),
                    (
                        "\u{2198}",
                        FontId::proportional(15.0),
                        "Slide out downwards",
                        n.slide_out,
                        |n| &mut n.slide_out,
                    ),
                    ("\u{223F}", sym(18.0), "Vibrato (V)", n.vibrato, |n| {
                        &mut n.vibrato
                    }),
                ];
                for (lab, font, tip, on, f) in items.iter() {
                    if glyph_button(ui, has && *on, lab, font.clone(), tip).clicked() {
                        self.toggle_note_flag(tip, *f);
                    }
                }
                if glyph_button(
                    ui,
                    has && n.wide_vibrato,
                    "\u{224B}",
                    sym(18.0),
                    "Wide vibrato",
                )
                .clicked()
                {
                    self.toggle_note_flag("Wide vibrato", |n| &mut n.wide_vibrato);
                }
                for (k, lab, tip) in [
                    (0u8, "\u{2934}", "Bend (B cycles the amount)"),
                    (1, "\u{2934}\u{2935}", "Bend and release"),
                    (2, "\u{2191}", "Pre-bend"),
                    (3, "\u{2191}\u{2935}", "Pre-bend and release"),
                ] {
                    let on = has && n.bend > 0 && n.bend_kind == k;
                    if glyph_button(ui, on, lab, sym(if k % 2 == 1 { 11.0 } else { 15.0 }), tip)
                        .clicked()
                    {
                        self.note_edit(|x| {
                            if x.bend > 0 && x.bend_kind == k {
                                x.bend = 0;
                            } else {
                                x.bend_kind = k;
                                if x.bend == 0 {
                                    x.bend = 4;
                                }
                            }
                        });
                    }
                }
                if glyph_button(
                    ui,
                    has && n.trill > 0,
                    "tr",
                    ital(14.0),
                    "Trill with the fret two above (change it in the Beat tab)",
                )
                .clicked()
                {
                    self.note_edit(|x| {
                        x.trill = if x.trill > 0 { 0 } else { (x.fret + 2).min(30) }
                    });
                }
            });
        });
        // Beat techniques.
        ui.horizontal_wrapped(|ui| {
            if glyph_button(ui, cur.tremolo, "\u{2AFD}", sym(15.0), "Tremolo picking").clicked() {
                self.beat_edit(|b| b.tremolo = !b.tremolo);
            }
            for (v, lab, tip) in [
                (1u8, "tap", "Tapping"),
                (2, "slap", "Slap"),
                (3, "pop", "Pop"),
            ] {
                if glyph_button(ui, cur.technique == v, lab, ital(11.0), tip).clicked() {
                    self.beat_edit(|b| b.technique = if b.technique == v { 0 } else { v });
                }
            }
            for (s, lab, tip) in [
                (Stroke::Down, "\u{2193}", "Strum down"),
                (Stroke::Up, "\u{2191}", "Strum up"),
            ] {
                if glyph_button(ui, cur.stroke == s, lab, FontId::proportional(16.0), tip).clicked()
                {
                    let v = if cur.stroke == s { Stroke::None } else { s };
                    self.beat_edit(|b| b.stroke = v);
                }
            }
            for (v, lab, tip) in [
                (1u8, "\u{2293}", "Pick downstroke"),
                (2, "V", "Pick upstroke"),
            ] {
                if glyph_button(
                    ui,
                    cur.pick == v,
                    lab,
                    if v == 1 {
                        sym(16.0)
                    } else {
                        FontId::proportional(14.0)
                    },
                    tip,
                )
                .clicked()
                {
                    self.beat_edit(|b| b.pick = if b.pick == v { 0 } else { v });
                }
            }
            if glyph_button(
                ui,
                cur.fade_in,
                "fade",
                ital(10.5),
                "Fade in (volume swell)",
            )
            .clicked()
            {
                self.beat_edit(|b| b.fade_in = !b.fade_in);
            }
        });
        divider(ui);
        ui.label(RichText::new("Click a string on the score and type fret numbers. Arrow keys move, F1 lists every shortcut.").size(11.0).color(Color32::from_gray(140)));
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
                                                Vec2::new(10.0, 15.0),
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
pub const VOICE_COLORS: [Color32; 4] = [
    Color32::from_rgb(240, 200, 80),
    Color32::from_rgb(110, 200, 120),
    Color32::from_rgb(230, 120, 180),
    Color32::from_rgb(160, 120, 230),
];
