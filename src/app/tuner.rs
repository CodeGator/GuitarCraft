//! The Tuner window (toolbar tuning fork, Tools > Tuner): a polyphonic section that tunes
//! every string of the current track from one strum, and a single-string (monophonic)
//! section. It listens only while the window is open.

use super::App;
use super::panels::BLUE;
use crate::model::pitch_name;
use crate::tuner::{self, Tuner, cents, note_hz};
use egui::{Align2, Color32, FontId, Pos2, Rect, RichText, Sense, Stroke, Vec2};

/// Within this many cents a string counts as in tune (shown green).
const IN_TUNE: f32 = 3.0;
const GREEN: Color32 = Color32::from_rgb(70, 190, 90);

/// The Tuner window's state.
#[derive(Default)]
pub struct TunerDlg {
    pub open: bool,
    /// The chosen input (None: the system's default).
    pub device: Option<String>,
    pub engine: Option<Tuner>,
    pub error: Option<String>,
    /// A made-up reading shown instead of listening (screenshots).
    pub sample: Option<tuner::Readout>,
}

/// A cents bar from -50 to +50: the offset's marker, green when in tune.
fn cents_bar(ui: &mut egui::Ui, c: Option<f32>, width: f32, height: f32) {
    let (r, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    let p = ui.painter();
    p.rect_filled(r, 3.0, Color32::from_rgb(24, 26, 30));
    // Tick marks every 10 cents, the centre stronger.
    for k in -5..=5 {
        let x = r.center().x + r.width() * 0.5 * k as f32 / 5.0;
        let (h, col) = if k == 0 { (r.height(), 140) } else { (r.height() * 0.35, 70) };
        p.line_segment(
            [Pos2::new(x, r.center().y - h * 0.5), Pos2::new(x, r.center().y + h * 0.5)],
            Stroke::new(1.0, Color32::from_gray(col)),
        );
    }
    let Some(c) = c else { return };
    let in_tune = c.abs() <= IN_TUNE;
    let col = if in_tune {
        GREEN
    } else if c.abs() <= 15.0 {
        Color32::from_rgb(230, 190, 60)
    } else {
        Color32::from_rgb(220, 80, 70)
    };
    let x = r.center().x + r.width() * 0.5 * (c / 50.0).clamp(-1.0, 1.0);
    // A bar from the centre to the reading, then the marker.
    let bar = Rect::from_x_y_ranges(
        x.min(r.center().x)..=x.max(r.center().x),
        (r.center().y - 3.0)..=(r.center().y + 3.0),
    );
    p.rect_filled(bar, 1.0, col.gamma_multiply(0.6));
    p.rect_filled(
        Rect::from_center_size(Pos2::new(x, r.center().y), Vec2::new(4.0, r.height() - 2.0)),
        1.0,
        col,
    );
    if in_tune {
        p.rect_stroke(r, 3.0, Stroke::new(1.5, GREEN), egui::StrokeKind::Inside);
    }
}

fn cents_text(c: Option<f32>) -> RichText {
    match c {
        Some(c) if c.abs() <= IN_TUNE => RichText::new("in tune").color(GREEN),
        Some(c) if c.abs() >= 59.0 => RichText::new(if c > 0.0 { "far sharp" } else { "far flat" }),
        Some(c) => RichText::new(format!("{c:+.0} cents")),
        None => RichText::new("—").weak(),
    }
}

impl App {
    pub(super) fn open_tuner(&mut self) {
        self.tuner.open = true;
        self.tuner.error = None;
    }

    pub(super) fn tuner_window(&mut self, ctx: &egui::Context) {
        if !self.tuner.open {
            // Closing the window stops listening.
            self.tuner.engine = None;
            return;
        }
        if self.tuner.engine.is_none() && self.tuner.error.is_none() && self.tuner.sample.is_none() {
            match Tuner::start(self.tuner.device.as_deref()) {
                Ok(t) => self.tuner.engine = Some(t),
                Err(e) => self.tuner.error = Some(e),
            }
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(100));

        // The current track's open strings (with the capo), string 1 first.
        let track = self.song.tracks.get(self.track);
        let strings: Vec<u8> = track
            .filter(|t| t.has_strings())
            .map(|t| {
                (0..t.tuning.len())
                    .map(|s| t.pitch(&crate::model::Note::new(s as u8, 0)))
                    .collect()
            })
            .unwrap_or_default();
        let flat = track.map(|t| t.tuning_flat).unwrap_or(false);
        let track_name = track.map(|t| t.name.clone()).unwrap_or_default();
        let readout = self.tuner.engine.as_ref().map(|e| {
            if let Ok(mut tg) = e.targets.lock() {
                *tg = strings.iter().map(|&m| note_hz(m as f32)).collect();
            }
            e.readout.lock().map(|r| r.clone()).unwrap_or_default()
        });
        let readout = readout.or_else(|| self.tuner.sample.clone());

        let mut open = true;
        let mut restart = false;
        egui::Window::new("Tuner")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(420.0)
            .show(ctx, |ui| {
                // Input device and level.
                ui.horizontal(|ui| {
                    ui.label("Input:");
                    let current = self.tuner.device.clone().unwrap_or_else(|| {
                        match &self.tuner.engine {
                            Some(e) => format!("Default input ({})", e.device),
                            None => "Default input".into(),
                        }
                    });
                    egui::ComboBox::from_id_salt("tuner-input")
                        .selected_text(current)
                        .width(240.0)
                        .show_ui(ui, |ui| {
                            if ui.selectable_label(self.tuner.device.is_none(), "Default input").clicked() {
                                self.tuner.device = None;
                                restart = true;
                            }
                            for name in tuner::input_device_names() {
                                let on = self.tuner.device.as_deref() == Some(name.as_str());
                                if ui.selectable_label(on, &name).clicked() {
                                    self.tuner.device = Some(name);
                                    restart = true;
                                }
                            }
                        });
                    let level = readout.as_ref().map(|r| r.level).unwrap_or(0.0);
                    let (lr, _) = ui.allocate_exact_size(Vec2::new(60.0, 8.0), Sense::hover());
                    let db = (20.0 * level.max(1e-5).log10()).clamp(-60.0, 0.0);
                    ui.painter().rect_filled(lr, 2.0, Color32::from_rgb(24, 26, 30));
                    ui.painter().rect_filled(
                        Rect::from_min_size(lr.min, Vec2::new(lr.width() * (db + 60.0) / 60.0, lr.height())),
                        2.0,
                        if level > 0.5 { Color32::from_rgb(220, 80, 70) } else { BLUE },
                    );
                });
                if let Some(e) = &self.tuner.error {
                    ui.colored_label(Color32::from_rgb(230, 120, 100), format!("Can't listen: {e}"));
                    if ui.button("Try again").clicked() {
                        restart = true;
                    }
                    return;
                }
                ui.separator();

                // Polyphonic: one bar per string.
                ui.label(RichText::new("All strings").strong());
                if strings.is_empty() {
                    ui.label(RichText::new("This track has no strings: use the single-string tuner below.").weak());
                } else {
                    ui.label(
                        RichText::new(format!("Strum every open string of {track_name} once and let them ring."))
                            .weak(),
                    );
                    egui::Grid::new("tuner-strings").spacing(Vec2::new(10.0, 4.0)).show(ui, |ui| {
                        for (s, &m) in strings.iter().enumerate() {
                            let c = readout.as_ref().and_then(|r| r.strings.get(s).copied().flatten());
                            ui.label(RichText::new(format!("{}", s + 1)).weak());
                            ui.label(
                                RichText::new(super::tuning::named(m, flat))
                                    .monospace()
                                    .color(if c.is_some_and(|c| c.abs() <= IN_TUNE) {
                                        GREEN
                                    } else {
                                        Color32::from_gray(220)
                                    }),
                            );
                            cents_bar(ui, c, 240.0, 16.0);
                            ui.label(cents_text(c));
                            ui.end_row();
                        }
                    });
                }
                ui.separator();

                // Monophonic: the single note heard.
                ui.label(RichText::new("Single string").strong());
                let mono = readout.as_ref().and_then(|r| r.mono);
                // Against the nearest string of the track, or the nearest note.
                let (name, target_cents, hint) = match mono {
                    Some(f) => {
                        let near_string = strings
                            .iter()
                            .enumerate()
                            .map(|(s, &m)| (s, m, cents(f, note_hz(m as f32))))
                            .filter(|(_, _, c)| c.abs() < 100.0)
                            .min_by(|a, b| a.2.abs().total_cmp(&b.2.abs()));
                        match near_string {
                            Some((s, m, c)) => (
                                super::tuning::named(m, flat),
                                Some(c),
                                format!("String {} · {f:.1} Hz", s + 1),
                            ),
                            None => {
                                let midi = (69.0 + 12.0 * (f / 440.0).log2()).round();
                                let c = cents(f, note_hz(midi));
                                let m = midi.clamp(0.0, 127.0) as u8;
                                (
                                    format!("{}{}", pitch_name(m, flat), m as i32 / 12 - 1),
                                    Some(c),
                                    format!("{f:.1} Hz"),
                                )
                            }
                        }
                    }
                    None => ("—".into(), None, "Play one string".into()),
                };
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(Vec2::new(70.0, 40.0), Sense::hover());
                    ui.painter().text(
                        r.center(),
                        Align2::CENTER_CENTER,
                        &name,
                        FontId::proportional(26.0),
                        if target_cents.is_some_and(|c| c.abs() <= IN_TUNE) {
                            GREEN
                        } else {
                            Color32::from_gray(225)
                        },
                    );
                    ui.vertical(|ui| {
                        cents_bar(ui, target_cents, 290.0, 18.0);
                        ui.horizontal(|ui| {
                            ui.label(cents_text(target_cents));
                            ui.label(RichText::new(hint).weak());
                        });
                    });
                });
            });
        if restart {
            self.tuner.engine = None;
            self.tuner.error = None;
        }
        if !open {
            self.tuner.open = false;
            self.tuner.engine = None;
        }
    }
}
