//! The Tuning window, opened from the gear button beside a track's tuning:
//! choose a tuning from the library or set each string, the capo, and how the
//! tuning is labelled on the score. "Adjust the Fingering" keeps every note's
//! pitch (frets move); "Keep the Fingering" keeps the frets (pitches move).

use super::App;
use crate::audio;
use crate::model::*;
use egui::{Align2, Color32, FontId, Pos2, Rect, RichText, Sense, Stroke, Vec2};

const BLUE: Color32 = Color32::from_rgb(61, 123, 224);
const BOX: Color32 = Color32::from_rgb(28, 30, 36);

/// The settings being changed, applied only when a fingering button is pressed.
#[derive(Clone)]
pub struct TuningDlg {
    pub track: usize,
    pub tuning: Vec<u8>,
    pub capo: u8,
    pub partial: u8,
    pub partial_strings: u8,
    pub flat: bool,
    pub label: String,
    pub label_visible: bool,
    pub detail: bool,
}

/// A note's name with its octave, using flats or sharps.
pub fn named(midi: u8, flat: bool) -> String {
    const S: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    const F: [&str; 12] = [
        "C", "Db", "D", "Eb", "E", "F", "Gb", "G", "Ab", "A", "Bb", "B",
    ];
    let n = if flat { F } else { S }[(midi % 12) as usize];
    format!("{}{}", n, midi as i32 / 12 - 1)
}

/// The instrument families of the tuning library.
const FAMILIES: [&str; 3] = ["Guitar", "Bass", "Ukulele"];

fn family_of(name: &str) -> usize {
    if name.contains("Ukulele") {
        2
    } else if name.contains("ass") {
        1
    } else {
        0
    }
}

fn presets_for(strings: usize, family: usize) -> Vec<(&'static str, Vec<u8>)> {
    tuning_presets(strings)
        .into_iter()
        .filter(|(n, _)| family_of(n) == family)
        .collect()
}

impl App {
    pub(super) fn open_tuning(&mut self) {
        let t = &self.song.tracks[self.track];
        self.tuning_dlg = Some(TuningDlg {
            track: self.track,
            tuning: t.tuning.clone(),
            capo: t.capo,
            partial: t.partial_capo,
            partial_strings: t.partial_capo_strings,
            flat: t.tuning_flat,
            label: t.tuning_label.clone(),
            label_visible: t.tuning_label_visible,
            detail: true,
        });
    }

    /// Plays open strings of the tuning being edited (all, from the lowest, or one).
    fn play_strings(&mut self, tuning: &[u8], capo: (u8, u8, u8), only: Option<usize>) {
        if !self.sound_allowed {
            return;
        }
        self.ensure_audio();
        let Some(a) = &self.audio else { return };
        if a.error.is_some() {
            return;
        }
        let sr = a.sample_rate();
        let mut tr = self.song.tracks[self.track].clone();
        tr.tuning = tuning.to_vec();
        tr.capo = capo.0;
        tr.partial_capo = capo.1;
        tr.partial_capo_strings = capo.2;
        if let Ok(mut st) = a.state.lock() {
            let strings: Vec<usize> = match only {
                Some(s) => vec![s],
                None => (0..tr.strings()).rev().collect(),
            };
            for (k, s) in strings.into_iter().enumerate() {
                let mut ev = audio::note_preview(&tr, self.track, &Note::new(s as u8, 0), sr);
                ev.start = (k as f32 * sr * 0.35) as u64;
                ev.len = (sr * 0.9) as u64;
                st.preview.push(ev);
            }
        }
    }

    pub(super) fn tuning_window(&mut self, ctx: &egui::Context) {
        let Some(mut d) = self.tuning_dlg.clone() else {
            return;
        };
        if d.track >= self.song.tracks.len() {
            self.tuning_dlg = None;
            return;
        }
        let mut open = true;
        let mut apply: Option<bool> = None;
        let mut cancel = false;
        let mut play: Option<Option<usize>> = None;
        egui::Window::new("Tuning")
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .fixed_size(Vec2::new(540.0, 0.0))
            .show(ctx, |ui| {
                let flat = d.flat;
                ui.label(RichText::new("TUNING").strong().size(17.0));
                ui.add_space(4.0);
                // Library: number of strings, instrument family, named tuning.
                let n = d.tuning.len();
                let current = tuning_name(&d.tuning);
                let mut family = family_of(&current);
                if !current.contains('(') && !tuning_presets(n).iter().any(|(nm, _)| *nm == current)
                {
                    family = if n <= 5 && d.tuning.iter().all(|&x| x < 50) {
                        1
                    } else {
                        0
                    };
                }
                ui.horizontal(|ui| {
                    ui.add_sized([62.0, 20.0], egui::Label::new("Library:"));
                    let mut k = n;
                    egui::ComboBox::from_id_salt("tun_n")
                        .width(86.0)
                        .selected_text(format!("{n} strings"))
                        .show_ui(ui, |ui| {
                            for c in [4usize, 5, 6, 7] {
                                ui.selectable_value(&mut k, c, format!("{c} strings"));
                            }
                        });
                    if k != n {
                        let fam = if k == 6 || k == 7 {
                            0
                        } else if family == 2 && k == 4 {
                            2
                        } else {
                            1
                        };
                        if let Some((_, t)) = presets_for(k, fam)
                            .into_iter()
                            .next()
                            .or_else(|| tuning_presets(k).into_iter().next())
                        {
                            d.tuning = t;
                        }
                    }
                    let mut fam = family;
                    egui::ComboBox::from_id_salt("tun_fam")
                        .width(110.0)
                        .selected_text(FAMILIES[family])
                        .show_ui(ui, |ui| {
                            for (i, f) in FAMILIES.iter().enumerate() {
                                ui.selectable_value(&mut fam, i, *f);
                            }
                        });
                    if fam != family {
                        let count = match fam {
                            0 => 6,
                            _ => 4,
                        };
                        if let Some((_, t)) = presets_for(count, fam).into_iter().next() {
                            d.tuning = t;
                        }
                    }
                    let shown = if current.contains('(')
                        || tuning_presets(d.tuning.len())
                            .iter()
                            .any(|(nm, t)| *nm == current && *t == d.tuning)
                    {
                        current.split(" (").next().unwrap_or("").to_string()
                    } else {
                        "Custom".to_string()
                    };
                    egui::ComboBox::from_id_salt("tun_name")
                        .width(ui.available_width())
                        .selected_text(shown)
                        .show_ui(ui, |ui| {
                            for (name, t) in presets_for(d.tuning.len(), fam) {
                                if ui
                                    .selectable_label(
                                        t == d.tuning,
                                        name.split(" (").next().unwrap_or(name),
                                    )
                                    .clicked()
                                {
                                    d.tuning = t;
                                }
                            }
                        });
                });
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.add_sized([62.0, 20.0], egui::Label::new("Strings:"));
                    let names: Vec<String> =
                        d.tuning.iter().rev().map(|&p| named(p, flat)).collect();
                    ui.label(
                        RichText::new(names.join(" "))
                            .strong()
                            .color(Color32::WHITE),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button("\u{25B6}")
                            .on_hover_text("Play the open strings")
                            .clicked()
                        {
                            play = Some(None);
                        }
                        if ui
                            .button(if d.detail {
                                "Detail \u{25B4}"
                            } else {
                                "Detail \u{25BE}"
                            })
                            .clicked()
                        {
                            d.detail = !d.detail;
                        }
                    });
                });
                if d.detail {
                    // Each string: number, note (with up/down), the string itself, and a play button.
                    ui.add_space(2.0);
                    egui::Frame::new()
                        .fill(BOX)
                        .inner_margin(egui::Margin::same(8))
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing.y = 3.0;
                            ui.horizontal(|ui| {
                                ui.add_space(64.0);
                                if ui
                                    .small_button("\u{2228}")
                                    .on_hover_text("Tune every string down a semitone")
                                    .clicked()
                                {
                                    for p in d.tuning.iter_mut() {
                                        *p = p.saturating_sub(1).max(16);
                                    }
                                }
                                if ui
                                    .small_button("\u{2227}")
                                    .on_hover_text("Tune every string up a semitone")
                                    .clicked()
                                {
                                    for p in d.tuning.iter_mut() {
                                        *p = (*p + 1).min(96);
                                    }
                                }
                            });
                            for s in 0..d.tuning.len() {
                                ui.horizontal(|ui| {
                                    let (r, _) =
                                        ui.allocate_exact_size(Vec2::splat(18.0), Sense::hover());
                                    ui.painter().circle_filled(
                                        r.center(),
                                        9.0,
                                        Color32::from_gray(80),
                                    );
                                    ui.painter().text(
                                        r.center(),
                                        Align2::CENTER_CENTER,
                                        format!("{}", s + 1),
                                        FontId::proportional(11.0),
                                        Color32::WHITE,
                                    );
                                    let text = named(d.tuning[s], flat);
                                    spin(ui, &mut d.tuning[s], 16, 96, 34.0, &text);
                                    let w = ui.available_width() - 30.0;
                                    let (lr, _) = ui.allocate_exact_size(
                                        Vec2::new(w.max(40.0), 22.0),
                                        Sense::hover(),
                                    );
                                    let thick = 1.0 + s as f32 * 0.7;
                                    ui.painter().line_segment(
                                        [
                                            Pos2::new(lr.left() + 6.0, lr.center().y),
                                            Pos2::new(lr.right() - 4.0, lr.center().y),
                                        ],
                                        Stroke::new(thick, Color32::from_gray(150)),
                                    );
                                    if ui
                                        .button("\u{25B6}")
                                        .on_hover_text(format!("Play string {}", s + 1))
                                        .clicked()
                                    {
                                        play = Some(Some(s));
                                    }
                                });
                            }
                        });
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.add_sized([62.0, 20.0], egui::Label::new("Alteration:"));
                    ui.spacing_mut().item_spacing.x = 0.0;
                    for (f, lab) in [(true, "\u{266D}"), (false, "\u{266F}")] {
                        let on = d.flat == f;
                        let b =
                            egui::Button::new(RichText::new(lab).size(15.0).color(Color32::WHITE))
                                .fill(if on {
                                    BLUE
                                } else {
                                    Color32::from_rgb(60, 64, 74)
                                })
                                .min_size(Vec2::new(38.0, 24.0));
                        if ui.add(b).clicked() {
                            d.flat = f;
                        }
                    }
                });

                ui.add_space(10.0);
                ui.label(RichText::new("CAPO").strong().size(17.0));
                ui.horizontal(|ui| {
                    ui.add_sized([62.0, 20.0], egui::Label::new("Capo:"));
                    let text = if d.capo == 0 {
                        "None".to_string()
                    } else {
                        d.capo.to_string()
                    };
                    spin(ui, &mut d.capo, 0, 12, 44.0, &text);
                });
                ui.horizontal(|ui| {
                    ui.add_sized([62.0, 20.0], egui::Label::new("Partial Capo:"));
                    let text = if d.partial == 0 {
                        "None".to_string()
                    } else {
                        d.partial.to_string()
                    };
                    spin(ui, &mut d.partial, 0, 12, 44.0, &text);
                    let on: Vec<String> = (0..d.tuning.len())
                        .filter(|&k| d.partial_strings & (1 << k) != 0)
                        .map(|k| (k + 1).to_string())
                        .collect();
                    let label = match on.len() {
                        0 => "No strings capoed".to_string(),
                        1 => format!("String {} capoed", on[0]),
                        _ => format!("Strings {} capoed", on.join(", ")),
                    };
                    let b = ui.add(
                        egui::Button::new(format!("{label}   \u{2195}"))
                            .min_size(Vec2::new(240.0, 0.0)),
                    );
                    egui::Popup::menu(&b)
                        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                        .show(|ui| {
                            for k in 0..d.tuning.len().min(8) {
                                ui.horizontal(|ui| {
                                    let (r, _) =
                                        ui.allocate_exact_size(Vec2::splat(18.0), Sense::hover());
                                    ui.painter().circle_filled(
                                        r.center(),
                                        9.0,
                                        Color32::from_gray(80),
                                    );
                                    ui.painter().text(
                                        r.center(),
                                        Align2::CENTER_CENTER,
                                        format!("{}", k + 1),
                                        FontId::proportional(11.0),
                                        Color32::WHITE,
                                    );
                                    let mut v = d.partial_strings & (1 << k) != 0;
                                    if switch(ui, &mut v) {
                                        d.partial_strings ^= 1 << k;
                                    }
                                });
                            }
                        });
                });
                // The neck, with the capo across it.
                let (r, _) = ui.allocate_exact_size(Vec2::new(350.0, 40.0), Sense::hover());
                let p = ui.painter();
                let neck =
                    Rect::from_min_max(r.min + Vec2::new(4.0, 4.0), r.max - Vec2::new(4.0, 4.0));
                p.rect_filled(neck, 1.0, Color32::from_gray(52));
                let frets = 15;
                let fx = |f: usize| neck.left() + neck.width() * f as f32 / frets as f32;
                p.line_segment(
                    [
                        Pos2::new(neck.left(), neck.top()),
                        Pos2::new(neck.left(), neck.bottom()),
                    ],
                    Stroke::new(3.0, Color32::from_gray(190)),
                );
                for f in 1..=frets {
                    p.line_segment(
                        [
                            Pos2::new(fx(f), neck.top()),
                            Pos2::new(fx(f), neck.bottom()),
                        ],
                        Stroke::new(1.0, Color32::from_gray(110)),
                    );
                }
                for f in [3usize, 5, 7, 9, 15] {
                    p.circle_filled(
                        Pos2::new((fx(f - 1) + fx(f)) * 0.5, neck.center().y),
                        1.6,
                        Color32::from_gray(170),
                    );
                }
                p.circle_filled(
                    Pos2::new((fx(11) + fx(12)) * 0.5, neck.top() + neck.height() * 0.3),
                    1.6,
                    Color32::from_gray(170),
                );
                p.circle_filled(
                    Pos2::new((fx(11) + fx(12)) * 0.5, neck.top() + neck.height() * 0.7),
                    1.6,
                    Color32::from_gray(170),
                );
                let n = d.tuning.len().max(1);
                for s in 0..n {
                    let y = neck.top() + neck.height() * (s as f32 + 0.5) / n as f32;
                    p.line_segment(
                        [Pos2::new(neck.left(), y), Pos2::new(neck.right(), y)],
                        Stroke::new(0.6 + s as f32 * 0.2, Color32::from_gray(150)),
                    );
                }
                if d.capo > 0 {
                    let x = fx(d.capo as usize) - neck.width() / frets as f32 * 0.3;
                    p.rect_filled(
                        Rect::from_center_size(
                            Pos2::new(x, neck.center().y),
                            Vec2::new(5.0, neck.height() + 4.0),
                        ),
                        2.0,
                        BLUE,
                    );
                }
                if d.partial > 0 {
                    let x = fx(d.partial as usize) - neck.width() / frets as f32 * 0.3;
                    for s in 0..n {
                        if d.partial_strings & (1 << s) != 0 {
                            let y = neck.top() + neck.height() * (s as f32 + 0.5) / n as f32;
                            p.rect_filled(
                                Rect::from_center_size(
                                    Pos2::new(x, y),
                                    Vec2::new(5.0, neck.height() / n as f32 + 1.0),
                                ),
                                1.5,
                                Color32::from_rgb(240, 160, 60),
                            );
                        }
                    }
                }

                ui.add_space(10.0);
                ui.label(RichText::new("DISPLAY").strong().size(17.0));
                ui.horizontal(|ui| {
                    ui.add_sized([62.0, 20.0], egui::Label::new("Label:"));
                    let hint = tuning_name(&d.tuning);
                    let hint = if hint.contains('(') {
                        hint.split(" (").next().unwrap_or("").to_string()
                    } else {
                        hint
                    };
                    ui.add(
                        egui::TextEdit::singleline(&mut d.label)
                            .hint_text(hint)
                            .desired_width(ui.available_width() - 80.0),
                    );
                    ui.checkbox(&mut d.label_visible, "Visible");
                });
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Cancel").clicked() {
                            cancel = true;
                        }
                        if ui
                            .button("Keep the Fingering")
                            .on_hover_text(
                                "Keep the frets; the notes' pitch follows the new tuning",
                            )
                            .clicked()
                        {
                            apply = Some(false);
                        }
                        let adj = egui::Button::new(
                            RichText::new("Adjust the Fingering").color(Color32::WHITE),
                        )
                        .fill(BLUE);
                        if ui
                            .add(adj)
                            .on_hover_text(
                                "Keep the notes' pitch; frets move to suit the new tuning",
                            )
                            .clicked()
                        {
                            apply = Some(true);
                        }
                    });
                });
            });
        if let Some(only) = play {
            let (t, c) = (d.tuning.clone(), (d.capo, d.partial, d.partial_strings));
            self.play_strings(&t, c, only);
        }
        if let Some(adjust) = apply {
            let dd = d.clone();
            self.edit(move |s, _, _, _, _| {
                let t = &mut s.tracks[dd.track];
                let old = t.clone();
                t.tuning = dd.tuning.clone();
                t.capo = dd.capo;
                t.partial_capo = dd.partial;
                t.partial_capo_strings = dd.partial_strings;
                t.tuning_flat = dd.flat;
                t.tuning_label = dd.label.clone();
                t.tuning_label_visible = dd.label_visible;
                if adjust {
                    refit_keeping_pitch(t, &old);
                } else {
                    // Same frets: drop notes on strings that no longer exist.
                    let n = t.tuning.len() as u8;
                    for bar in &mut t.bars {
                        for beat in bar.all_beats_mut() {
                            beat.notes.retain(|x| x.string < n);
                        }
                    }
                }
            });
            self.song.normalize();
            self.clamp_cursor();
            self.total_secs = self.song_seconds();
            self.status = if adjust {
                "Tuning changed; the notes keep their pitch.".into()
            } else {
                "Tuning changed; the notes keep their frets.".into()
            };
            self.tuning_dlg = None;
            return;
        }
        self.tuning_dlg = if open && !cancel { Some(d) } else { None };
    }
}

/// Moves every note to a string and fret that plays the pitch it had before
/// (with the old tuning and capos).
fn refit_keeping_pitch(t: &mut Track, old: &Track) {
    let maxf = t.frets as i32;
    let n = t.tuning.len();
    let new = t.clone();
    for (bi, bar) in t.bars.iter_mut().enumerate() {
        let _ = bi;
        for beat in bar.all_beats_mut() {
            let mut out: Vec<Note> = vec![];
            for note in beat.notes.drain(..) {
                if note.string as usize >= old.tuning.len() {
                    continue;
                }
                let pitch = old.pitch(&note) as i32;
                let mut order: Vec<usize> = (0..n).collect();
                order.sort_by_key(|&s| (s as i32 - note.string as i32).abs());
                for s in order {
                    let f = pitch - new.tuning[s] as i32 - new.capo_at(s) as i32;
                    if (0..=maxf).contains(&f) && !out.iter().any(|o| o.string as usize == s) {
                        let mut m = note.clone();
                        m.string = s as u8;
                        m.fret = f as u8;
                        out.push(m);
                        break;
                    }
                }
            }
            out.sort_by_key(|x| x.string);
            beat.notes = out;
        }
    }
}

/// A value box with small up/down arrows beside it (like the original's spin boxes).
fn spin(ui: &mut egui::Ui, v: &mut u8, min: u8, max: u8, width: f32, text: &str) -> bool {
    let (r, _) = ui.allocate_exact_size(Vec2::new(width + 14.0, 22.0), Sense::hover());
    let field = Rect::from_min_size(r.min, Vec2::new(width, r.height()));
    let p = ui.painter();
    p.rect_filled(r, 4.0, Color32::from_gray(18));
    p.rect_stroke(
        r,
        4.0,
        Stroke::new(1.0, Color32::from_gray(95)),
        egui::StrokeKind::Inside,
    );
    p.text(
        field.left_center() + Vec2::new(6.0, 0.0),
        Align2::LEFT_CENTER,
        text,
        FontId::proportional(12.5),
        Color32::WHITE,
    );
    let up = Rect::from_min_max(
        Pos2::new(field.right(), r.top()),
        Pos2::new(r.right(), r.center().y),
    );
    let down = Rect::from_min_max(Pos2::new(field.right(), r.center().y), r.max);
    let ru = ui.interact(
        up,
        ui.id().with((r.min.x as i32, r.min.y as i32, "up")),
        Sense::click(),
    );
    let rd = ui.interact(
        down,
        ui.id().with((r.min.x as i32, r.min.y as i32, "down")),
        Sense::click(),
    );
    let p = ui.painter();
    for (rect, resp, dir) in [(up, &ru, -1.0f32), (down, &rd, 1.0f32)] {
        let c = if resp.hovered() {
            Color32::WHITE
        } else {
            Color32::from_gray(200)
        };
        let m = rect.center();
        p.add(egui::Shape::convex_polygon(
            vec![
                Pos2::new(m.x - 3.5, m.y - dir * 1.8),
                Pos2::new(m.x + 3.5, m.y - dir * 1.8),
                Pos2::new(m.x, m.y + dir * 2.2),
            ],
            c,
            Stroke::NONE,
        ));
    }
    let mut changed = false;
    if ru.clicked() && *v < max {
        *v += 1;
        changed = true;
    }
    if rd.clicked() && *v > min {
        *v -= 1;
        changed = true;
    }
    // The mouse wheel over the box changes it too.
    if ui.rect_contains_pointer(r) {
        let dy = ui.input(|i| i.smooth_scroll_delta.y);
        if dy > 0.0 && *v < max {
            *v += 1;
            changed = true;
        } else if dy < 0.0 && *v > min {
            *v -= 1;
            changed = true;
        }
    }
    changed
}

/// An on/off switch with its state written beside it.
fn switch(ui: &mut egui::Ui, on: &mut bool) -> bool {
    let (r, resp) = ui.allocate_exact_size(Vec2::new(30.0, 16.0), Sense::click());
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
    if resp.clicked() {
        *on = !*on;
        return true;
    }
    false
}
