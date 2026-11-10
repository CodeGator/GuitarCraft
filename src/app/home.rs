//! The start screen: new/open, recent files, templates and examples with a preview.

use super::{App, Pending};
use crate::fileio;
use crate::model::*;
use crate::score::{self, Notation};
use egui::{Align2, Color32, FontId, Pos2, Rect, RichText, Sense, Stroke as PStroke, Ui, Vec2};
use std::path::PathBuf;

fn config_dir(app: &str) -> Option<PathBuf> {
    let base = std::env::var_os("APPDATA").or_else(|| std::env::var_os("HOME"))?;
    Some(PathBuf::from(base).join(app))
}

fn config_file() -> Option<PathBuf> {
    Some(config_dir("GuitarCraft")?.join("recent.json"))
}

pub fn load_recent() -> Vec<PathBuf> {
    let Some(f) = config_file() else {
        return vec![];
    };
    // Fall back to the list saved under the app's earlier name.
    let old = config_dir("TabForge").map(|d| d.join("recent.json"));
    std::fs::read(&f)
        .ok()
        .or_else(|| old.and_then(|o| std::fs::read(o).ok()))
        .and_then(|d| serde_json::from_slice::<Vec<PathBuf>>(&d).ok())
        .unwrap_or_default()
}

pub fn save_recent(list: &[PathBuf]) {
    if let Some(f) = config_file() {
        if let Some(dir) = f.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(s) = serde_json::to_string_pretty(list) {
            let _ = std::fs::write(f, s);
        }
    }
}

pub const TEMPLATES: [&str; 18] = [
    "Empty (default)",
    "Acoustic Bass",
    "Classical Guitar",
    "Electric Bass",
    "Electric Guitar Clean",
    "Electric Guitar Distortion",
    "Electric Guitar Overdrive",
    "Guitar and Bass",
    "Jazz Guitar",
    "Metal Band",
    "Modern Rock Band",
    "Nylon Guitar",
    "Steel Guitar",
    "7-String Guitar",
    "Ukulele",
    "Piano",
    "Vocals and Piano",
    "Pop Band (vocals, piano, guitar, bass, drums)",
];

fn tr(name: &str, inst: Instrument, tuning: &[u8], color: usize) -> Track {
    Track {
        name: name.into(),
        instrument: inst,
        tuning: tuning.to_vec(),
        color: TRACK_COLORS[color % TRACK_COLORS.len()],
        ..Track::default()
    }
}

fn inst(name: &str, i: Instrument, color: usize) -> Track {
    let mut t = Track::of_instrument(i, name);
    t.color = TRACK_COLORS[color % TRACK_COLORS.len()];
    t
}

/// A short song for a band: vocals with lyrics, piano on a grand staff, guitar, bass and drums.
pub fn band_song() -> Song {
    let mut s = template(17);
    s.title = "Band Practice".into();
    s.subtitle = "Five instruments".into();
    s.tempo = 100;
    let beat = |d: Dur, notes: Vec<Note>| Beat {
        notes,
        ..Beat::rest(d)
    };
    // Notes on "virtual strings": the fret is the pitch.
    let keys = |d: Dur, pitches: &[u8]| {
        beat(
            d,
            pitches
                .iter()
                .enumerate()
                .map(|(k, &p)| Note::new(k as u8, p))
                .collect(),
        )
    };
    let chords: [[u8; 3]; 4] = [[60, 64, 67], [59, 62, 67], [57, 60, 64], [57, 60, 65]];
    let roots: [u8; 4] = [48, 43, 45, 41];
    // Vocals.
    let tune: [[u8; 4]; 4] = [
        [72, 74, 76, 79],
        [74, 72, 71, 67],
        [69, 71, 72, 76],
        [74, 72, 69, 0],
    ];
    for (bi, bar) in tune.iter().enumerate() {
        let beats: Vec<Beat> = bar
            .iter()
            .map(|&p| {
                if p == 0 {
                    Beat::rest(Dur::Quarter)
                } else {
                    keys(Dur::Quarter, &[p])
                }
            })
            .collect();
        s.tracks[0].bars[bi] = Bar::new(beats);
    }
    s.tracks[0].lyrics =
        "Sing a-long the mor-ning light, hold the mel-o-dy so bright, keep it near".into();
    // Piano: chords above, roots below middle C.
    for bi in 0..4 {
        let c = chords[bi];
        let r = roots[bi];
        s.tracks[1].bars[bi] = Bar::new(vec![
            keys(Dur::Half, &[r, r + 12, c[0], c[1], c[2]]),
            keys(Dur::Quarter, &[c[0], c[1], c[2]]),
            keys(Dur::Quarter, &[r + 7, c[0], c[1], c[2]]),
        ]);
    }
    // Guitar strums.
    let shapes: [&[(u8, u8)]; 4] = [
        &[(1, 1), (2, 0), (3, 2), (4, 3)],
        &[(0, 3), (1, 0), (2, 0), (3, 0), (4, 2), (5, 3)],
        &[(0, 0), (1, 1), (2, 2), (3, 2), (4, 0)],
        &[(0, 1), (1, 1), (2, 2), (3, 3), (4, 3), (5, 1)],
    ];
    for (bi, shape) in shapes.iter().enumerate() {
        let sh: Vec<Note> = shape.iter().map(|&(st, f)| Note::new(st, f)).collect();
        s.tracks[2].bars[bi] = Bar::new(vec![
            beat(Dur::Quarter, sh.clone()),
            beat(Dur::Eighth, sh.clone()),
            beat(Dur::Eighth, sh.clone()),
            beat(Dur::Half, sh),
        ]);
    }
    // Bass roots.
    let bass: [(u8, u8); 4] = [(1, 3), (3, 3), (3, 5), (3, 1)];
    for (bi, &(st, f)) in bass.iter().enumerate() {
        s.tracks[3].bars[bi] = Bar::new(vec![
            beat(Dur::Half, vec![Note::new(st, f)]),
            beat(Dur::Half, vec![Note::new(st, f)]),
        ]);
    }
    // Drums: hi-hat eighths, bass drum on 1 and 3, snare on 2 and 4.
    for bi in 0..4 {
        let beats: Vec<Beat> = (0..8)
            .map(|k| {
                let mut p = vec![42u8];
                if k == 0 || k == 4 {
                    p.push(36);
                }
                if k == 2 || k == 6 {
                    p.push(38);
                }
                if bi == 0 && k == 0 {
                    p = vec![49, 36];
                }
                keys(Dur::Eighth, &p)
            })
            .collect();
        s.tracks[4].bars[bi] = Bar::new(beats);
    }
    s.normalize();
    s
}

pub fn template(k: usize) -> Song {
    const STD: &[u8] = &STANDARD_GUITAR;
    const DROP_D: [u8; 6] = [64, 59, 55, 50, 45, 38];
    const BASS: &[u8] = &STANDARD_BASS;
    let mut s = Song::new_blank();
    s.title = "Untitled".into();
    let tracks = match k {
        1 => vec![tr("Acoustic Bass", Instrument::Bass, BASS, 1)],
        2 => vec![tr("Classical Guitar", Instrument::NylonGuitar, STD, 0)],
        3 => vec![tr("Electric Bass", Instrument::Bass, BASS, 1)],
        4 => vec![tr("Electric Guitar", Instrument::CleanGuitar, STD, 0)],
        5 => vec![tr(
            "Distortion Guitar",
            Instrument::DistortionGuitar,
            STD,
            0,
        )],
        6 => vec![tr("Overdrive Guitar", Instrument::OverdriveGuitar, STD, 0)],
        7 => vec![
            tr("Steel Guitar", Instrument::SteelGuitar, STD, 0),
            tr("Electric Bass", Instrument::Bass, BASS, 1),
        ],
        8 => vec![tr("Jazz Guitar", Instrument::CleanGuitar, STD, 5)],
        9 => vec![
            tr("Rhythm Guitar", Instrument::DistortionGuitar, &DROP_D, 0),
            tr("Lead Guitar", Instrument::DistortionGuitar, &DROP_D, 3),
            tr("Bass", Instrument::Bass, &[43, 38, 33, 26], 1),
        ],
        10 => vec![
            tr("Rhythm Guitar", Instrument::OverdriveGuitar, STD, 0),
            tr("Lead Guitar", Instrument::CleanGuitar, STD, 3),
            tr("Bass", Instrument::Bass, BASS, 1),
        ],
        11 => vec![tr("Nylon Guitar", Instrument::NylonGuitar, STD, 0)],
        12 => vec![tr("Steel Guitar", Instrument::SteelGuitar, STD, 0)],
        13 => vec![tr(
            "7-String Guitar",
            Instrument::DistortionGuitar,
            &[64, 59, 55, 50, 45, 40, 35],
            0,
        )],
        14 => vec![tr("Ukulele", Instrument::Ukulele, &[69, 64, 60, 67], 2)],
        15 => vec![inst("Piano", Instrument::Piano, 4)],
        16 => vec![
            inst("Lead Vocals", Instrument::LeadVocals, 5),
            inst("Piano", Instrument::Piano, 4),
        ],
        17 => vec![
            inst("Lead Vocals", Instrument::LeadVocals, 5),
            inst("Piano", Instrument::Piano, 4),
            tr("Guitar", Instrument::SteelGuitar, STD, 0),
            tr("Bass", Instrument::Bass, BASS, 1),
            inst("Drums", Instrument::Drums, 6),
        ],
        _ => vec![tr("Steel Guitar", Instrument::SteelGuitar, STD, 0)],
    };
    s.tracks = tracks;
    s.normalize();
    s
}

enum Item {
    Recent(PathBuf),
    Template(usize),
    Example,
}

impl App {
    fn home_items(&self) -> Vec<(String, Item)> {
        let q = self.home_search.to_lowercase();
        let mut v: Vec<(String, Item)> = match self.home_tab {
            0 => self
                .recent
                .iter()
                .map(|p| {
                    (
                        p.file_stem()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_default(),
                        Item::Recent(p.clone()),
                    )
                })
                .collect(),
            1 => TEMPLATES
                .iter()
                .enumerate()
                .map(|(i, n)| (n.to_string(), Item::Template(i)))
                .collect(),
            _ => vec![("Morning Walk (demo song)".to_string(), Item::Example)],
        };
        if !q.is_empty() {
            v.retain(|(n, _)| n.to_lowercase().contains(&q));
        }
        v
    }

    fn open_item(&mut self, it: &Item) {
        match it {
            Item::Recent(p) => {
                let p = p.clone();
                if self.dirty {
                    self.status = "Save or undo your changes first (or use File ▸ Open).".into();
                } else {
                    self.open_path(p);
                }
            }
            Item::Template(k) => self.request(Pending::Template(*k)),
            Item::Example => self.request(Pending::Demo),
        }
    }

    pub(super) fn home_ui(&mut self, ui: &mut Ui) {
        let full = ui.max_rect();
        ui.painter()
            .rect_filled(full, 0.0, Color32::from_rgb(40, 42, 48));
        let w = (full.width() - 160.0).clamp(600.0, 1120.0);
        let left = full.center().x - w * 0.5;
        let mut y = full.top() + 28.0;
        // App name.
        let p = ui.painter();
        let g1 = p.layout_no_wrap(
            "Guitar".into(),
            FontId::new(40.0, egui::FontFamily::Proportional),
            Color32::from_gray(235),
        );
        let g2 = p.layout_no_wrap(
            "Craft".into(),
            FontId::new(40.0, egui::FontFamily::Name("bold".into())),
            Color32::from_rgb(92, 150, 240),
        );
        let tw = g1.size().x + g2.size().x;
        let x0 = full.center().x - tw * 0.5;
        let w1 = g1.size().x;
        p.galley(Pos2::new(x0, y), g1, Color32::WHITE);
        p.galley(Pos2::new(x0 + w1, y), g2, Color32::WHITE);
        y += 92.0;
        let mut open_new = false;
        let mut open_file = false;
        let r_new = Rect::from_center_size(Pos2::new(full.center().x, y), Vec2::new(220.0, 26.0));
        let r_open =
            Rect::from_center_size(Pos2::new(full.center().x, y + 36.0), Vec2::new(220.0, 26.0));
        for (r, label, key, which) in [
            (r_new, "New File", "(Ctrl+N)", 0),
            (r_open, "Open File…", "(Ctrl+O)", 1),
        ] {
            let resp = ui.interact(r, ui.id().with(("home_btn", which)), Sense::click());
            let p = ui.painter();
            if resp.hovered() {
                p.rect_filled(r, 4.0, Color32::from_gray(60));
            }
            let ic =
                Rect::from_center_size(Pos2::new(r.left() + 30.0, r.center().y), Vec2::splat(14.0));
            if which == 0 {
                super::widgets::icon_new(p, ic, Color32::from_gray(220));
            } else {
                super::widgets::icon_open(p, ic, Color32::from_gray(220));
            }
            p.text(
                Pos2::new(r.left() + 50.0, r.center().y),
                Align2::LEFT_CENTER,
                label,
                FontId::proportional(14.0),
                Color32::from_gray(230),
            );
            p.text(
                Pos2::new(r.left() + 150.0, r.center().y),
                Align2::LEFT_CENTER,
                key,
                FontId::proportional(13.0),
                Color32::from_gray(150),
            );
            if resp.clicked() {
                if which == 0 {
                    open_new = true;
                } else {
                    open_file = true;
                }
            }
        }
        y += 76.0;
        // Search.
        let sr = Rect::from_center_size(Pos2::new(full.center().x, y), Vec2::new(320.0, 28.0));
        ui.painter()
            .rect_filled(sr, 4.0, Color32::from_rgb(24, 26, 30));
        ui.painter().rect_stroke(
            sr,
            4.0,
            PStroke::new(1.2, Color32::from_rgb(70, 120, 210)),
            egui::StrokeKind::Inside,
        );
        ui.painter().circle_stroke(
            Pos2::new(sr.left() + 15.0, sr.center().y - 1.0),
            5.0,
            PStroke::new(1.6, Color32::from_gray(220)),
        );
        ui.painter().line_segment(
            [
                Pos2::new(sr.left() + 18.5, sr.center().y + 2.5),
                Pos2::new(sr.left() + 22.0, sr.center().y + 6.0),
            ],
            PStroke::new(1.8, Color32::from_gray(220)),
        );
        let te = egui::TextEdit::singleline(&mut self.home_search)
            .hint_text("Search…")
            .frame(egui::Frame::NONE)
            .desired_width(sr.width() - 40.0);
        ui.put(
            Rect::from_min_max(
                Pos2::new(sr.left() + 30.0, sr.top() + 5.0),
                Pos2::new(sr.right() - 8.0, sr.bottom() - 4.0),
            ),
            te,
        );
        y += 50.0;
        // Tabs.
        let tabs = ["RECENT FILES", "TEMPLATES", "EXAMPLES"];
        let tab_w = w / tabs.len() as f32;
        for (i, t) in tabs.iter().enumerate() {
            let r = Rect::from_min_size(
                Pos2::new(left + i as f32 * tab_w, y - 12.0),
                Vec2::new(tab_w, 26.0),
            );
            let resp = ui.interact(r, ui.id().with(("home_tab", i)), Sense::click());
            let sel = self.home_tab == i;
            ui.painter().text(
                r.center(),
                Align2::CENTER_CENTER,
                *t,
                FontId::new(15.0, egui::FontFamily::Name("bold".into())),
                if sel {
                    Color32::WHITE
                } else if resp.hovered() {
                    Color32::from_gray(190)
                } else {
                    Color32::from_gray(140)
                },
            );
            if resp.clicked() {
                self.home_tab = i;
                self.home_sel = 0;
                self.preview = None;
            }
        }
        y += 24.0;
        let bottom = full.bottom() - 70.0;
        let list_r =
            Rect::from_min_max(Pos2::new(left, y), Pos2::new(left + w * 0.5 - 6.0, bottom));
        let prev_r = Rect::from_min_max(
            Pos2::new(left + w * 0.5 + 6.0, y),
            Pos2::new(left + w, bottom),
        );
        ui.painter()
            .rect_filled(list_r, 4.0, Color32::from_rgb(30, 32, 37));
        ui.painter()
            .rect_filled(prev_r, 4.0, Color32::from_rgb(30, 32, 37));
        let items = self.home_items();
        if self.home_sel >= items.len() {
            self.home_sel = 0;
        }
        let mut open_idx = None;
        let row_h = 28.0;
        for (i, (name, _)) in items.iter().enumerate() {
            let r = Rect::from_min_size(
                Pos2::new(list_r.left() + 4.0, list_r.top() + 4.0 + i as f32 * row_h),
                Vec2::new(list_r.width() - 8.0, row_h - 2.0),
            );
            if r.bottom() > list_r.bottom() {
                break;
            }
            let resp = ui.interact(
                r,
                ui.id().with(("home_item", self.home_tab, i)),
                Sense::click(),
            );
            let sel = i == self.home_sel;
            let p = ui.painter();
            if sel {
                p.rect_filled(r, 4.0, Color32::from_rgb(70, 130, 230));
            } else if resp.hovered() {
                p.rect_filled(r, 4.0, Color32::from_gray(55));
            }
            p.rect_filled(
                Rect::from_center_size(
                    Pos2::new(r.left() + 16.0, r.center().y),
                    Vec2::new(10.0, 13.0),
                ),
                2.0,
                Color32::from_rgb(120, 170, 240),
            );
            p.text(
                Pos2::new(r.left() + 32.0, r.center().y),
                Align2::LEFT_CENTER,
                name,
                FontId::proportional(13.5),
                Color32::from_gray(235),
            );
            if resp.clicked() {
                self.home_sel = i;
                self.preview = None;
            }
            if resp.double_clicked() {
                open_idx = Some(i);
            }
        }
        if items.is_empty() {
            let msg = if self.home_tab == 0 {
                "Songs you open or save will appear here."
            } else {
                "Nothing matches your search."
            };
            ui.painter().text(
                list_r.center(),
                Align2::CENTER_CENTER,
                msg,
                FontId::proportional(13.0),
                Color32::from_gray(140),
            );
        }
        // Preview of the selected song's first page.
        if let Some((_, it)) = items.get(self.home_sel) {
            let key = match it {
                Item::Recent(p) => p.display().to_string(),
                Item::Template(k) => format!("template-{k}"),
                Item::Example => "example".into(),
            };
            if self
                .preview
                .as_ref()
                .map(|(k, _)| k != &key)
                .unwrap_or(true)
            {
                let song = match it {
                    Item::Recent(p) => fileio::load_song(p).ok(),
                    Item::Template(k) => Some(template(*k)),
                    Item::Example => Some(Song::demo()),
                };
                self.preview = song.map(|s| (key.clone(), s));
                if self.preview.is_none() {
                    self.preview = Some((key, Song::new_blank()));
                }
            }
            if let Some((_, song)) = &self.preview {
                let avail = prev_r.shrink(14.0);
                let zoom = (avail.height() / score::PAGE_H).min(avail.width() / score::PAGE_W);
                let shown: Vec<usize> = (0..song.tracks.len()).collect();
                let lay = score::layout(song, &shown, 0, zoom, Notation::Both);
                let o = Pos2::new(
                    avail.center().x - lay.page_w() * 0.5,
                    avail.top() - score::PAGE_GAP,
                );
                let dc = score::DrawCtx {
                    song,
                    ti: 0,
                    cursor: (0, 0, 0),
                    selection: None,
                    play_tick: None,
                    music_font: self.music_font,
                    show_cursor: false,
                    voice: 0,
                    cursor_std: None,
                };
                let painter = ui.painter().with_clip_rect(prev_r);
                score::draw_page(&painter, &lay, &dc, 0, lay.page_origin(0, o.x, o.y));
            }
            let br = Rect::from_center_size(
                Pos2::new(prev_r.center().x, prev_r.bottom() + 22.0),
                Vec2::new(150.0, 28.0),
            );
            let resp = ui.put(
                br,
                egui::Button::new(RichText::new("Open").strong())
                    .fill(Color32::from_rgb(70, 130, 230)),
            );
            if resp.clicked() {
                open_idx = Some(self.home_sel);
            }
        }
        if self.home_tab == 0 && !self.recent.is_empty() {
            let r = Rect::from_min_size(Pos2::new(left, bottom + 8.0), Vec2::new(150.0, 26.0));
            if ui.put(r, egui::Button::new("Clear recent files")).clicked() {
                self.recent.clear();
                save_recent(&self.recent);
            }
        }
        ui.painter().text(
            Pos2::new(full.left() + 20.0, full.bottom() - 18.0),
            Align2::LEFT_CENTER,
            format!("Version {}", env!("CARGO_PKG_VERSION")),
            FontId::proportional(12.0),
            Color32::from_gray(120),
        );
        let hr = Rect::from_min_size(
            Pos2::new(full.right() - 190.0, full.bottom() - 32.0),
            Vec2::new(170.0, 26.0),
        );
        if ui
            .put(hr, egui::Button::new("Keyboard shortcuts…").frame(false))
            .clicked()
        {
            self.show_help = true;
        }
        if let Some(i) = open_idx {
            if let Some((_, it)) = items.get(i) {
                self.open_item(it);
            }
        }
        if open_new {
            self.request(Pending::New);
        }
        if open_file {
            self.request(Pending::Open);
        }
    }
}
