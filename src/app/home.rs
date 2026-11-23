//! The start screen: new/open, recent files, templates and examples with a preview.

use super::{App, Pending};
use crate::fileio;
use crate::model::*;
use crate::score::{self, Notation};
use egui::{Align2, Color32, FontId, Pos2, Rect, RichText, Sense, Stroke as PStroke, Ui, Vec2};
use std::path::PathBuf;

/// GuitarCraft's settings folder: in APPDATA, otherwise in HOME.
pub(super) fn app_dir() -> Option<PathBuf> {
    let base = std::env::var_os("APPDATA").or_else(|| std::env::var_os("HOME"))?;
    Some(PathBuf::from(base).join("GuitarCraft"))
}

/// The files in `dir` ending in `.ext`, sorted (none if there is no folder).
pub(super) fn files_with_ext(dir: Option<PathBuf>, ext: &str) -> Vec<PathBuf> {
    let Some(d) = dir else { return vec![] };
    let mut v: Vec<PathBuf> = std::fs::read_dir(d)
        .map(|r| r.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.extension().map(|e| e == ext).unwrap_or(false)).collect())
        .unwrap_or_default();
    v.sort();
    v
}

fn config_file() -> Option<PathBuf> {
    Some(app_dir()?.join("recent.json"))
}

/// The recent files that are still there. A file that has been moved, deleted or renamed (a
/// chord chart saved under an older extension, say) isn't offered any more.
pub fn only_existing(list: Vec<PathBuf>) -> Vec<PathBuf> {
    list.into_iter().filter(|p| p.exists()).collect()
}

pub fn load_recent() -> Vec<PathBuf> {
    let Some(f) = config_file() else {
        return vec![];
    };
    let list = std::fs::read(&f)
        .ok()
        .and_then(|d| serde_json::from_slice::<Vec<PathBuf>>(&d).ok())
        .unwrap_or_default();
    only_existing(list)
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

/// The standard templates, in the order Guitar Pro 8 lists them. The first is an
/// empty document, which asks for its first track.
pub const TEMPLATES: [&str; 24] = [
    "Empty (default)",
    "Acoustic Bass",
    "Acoustic Piano",
    "Blues Power Trio",
    "Californian Punk Band",
    "Classical Guitar",
    "Drumkit",
    "Electric Bass",
    "Electric Guitar Clean",
    "Electric Guitar Distortion",
    "Electric Guitar Overdrive",
    "Funk Orchestra",
    "Guitar Vocals",
    "Jazz Guitar",
    "Jazz Quintet",
    "Jazz Trio",
    "Metal Band",
    "Modern Rock Band",
    "Nylon Guitar",
    "Progressive Rock Band",
    "Rock Experience Band",
    "Steel Guitar",
    "Stoner Band",
    "Thrash Band",
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
    let mut s = Song::new_blank();
    s.tracks = vec![
        inst("Lead Vocals", Instrument::LeadVocals, 5),
        inst("Piano", Instrument::Piano, 4),
        tr("Guitar", Instrument::SteelGuitar, &STANDARD_GUITAR, 0),
        tr("Bass", Instrument::Bass, &STANDARD_BASS, 1),
        inst("Drums", Instrument::Drums, 6),
    ];
    s.normalize();
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

/// The tracks of a standard template.
fn template_tracks(k: usize) -> Vec<Track> {
    const STD: &[u8] = &STANDARD_GUITAR;
    const DROP_D: [u8; 6] = [64, 59, 55, 50, 45, 38];
    const D_STD: [u8; 6] = [62, 57, 53, 48, 43, 38];
    const BASS: &[u8] = &STANDARD_BASS;
    const DROP_D_BASS: [u8; 4] = [43, 38, 33, 26];
    let guitar = |name: &str, i: Instrument, tuning: &[u8]| tr(name, i, tuning, 0);
    let bass = |name: &str| tr(name, Instrument::Bass, BASS, 1);
    let drums = || inst("Drums", Instrument::Drums, 6);
    let vocals = || inst("Lead Vocals", Instrument::LeadVocals, 5);
    match k {
        1 => vec![bass("Acoustic Bass")],
        2 => vec![inst("Acoustic Piano", Instrument::Piano, 4)],
        3 => vec![guitar("Guitar", Instrument::OverdriveGuitar, STD), bass("Bass"), drums()],
        4 => vec![
            guitar("Guitar 1", Instrument::DistortionGuitar, STD),
            guitar("Guitar 2", Instrument::DistortionGuitar, STD),
            bass("Bass"),
            drums(),
            vocals(),
        ],
        5 => vec![guitar("Classical Guitar", Instrument::NylonGuitar, STD)],
        6 => vec![drums()],
        7 => vec![bass("Electric Bass")],
        8 => vec![guitar("Electric Guitar Clean", Instrument::CleanGuitar, STD)],
        9 => vec![guitar("Electric Guitar Distortion", Instrument::DistortionGuitar, STD)],
        10 => vec![guitar("Electric Guitar Overdrive", Instrument::OverdriveGuitar, STD)],
        11 => vec![
            guitar("Rhythm Guitar", Instrument::CleanGuitar, STD),
            bass("Bass"),
            inst("Electric Piano", Instrument::ElectricPiano, 4),
            inst("Trumpet", Instrument::Trumpet, 5),
            inst("Saxophone", Instrument::Saxophone, 3),
            drums(),
        ],
        12 => vec![vocals(), guitar("Steel Guitar", Instrument::SteelGuitar, STD)],
        13 => vec![guitar("Jazz Guitar", Instrument::CleanGuitar, STD)],
        14 => vec![
            inst("Saxophone", Instrument::Saxophone, 3),
            inst("Piano", Instrument::Piano, 4),
            guitar("Jazz Guitar", Instrument::CleanGuitar, STD),
            bass("Bass"),
            drums(),
        ],
        15 => vec![inst("Piano", Instrument::Piano, 4), bass("Bass"), drums()],
        16 => vec![
            guitar("Rhythm Guitar", Instrument::DistortionGuitar, &DROP_D),
            guitar("Lead Guitar", Instrument::DistortionGuitar, &DROP_D),
            tr("Bass", Instrument::Bass, &DROP_D_BASS, 1),
            drums(),
            vocals(),
        ],
        17 => vec![
            vocals(),
            guitar("Rhythm Guitar", Instrument::OverdriveGuitar, STD),
            guitar("Lead Guitar", Instrument::CleanGuitar, STD),
            bass("Bass"),
            drums(),
        ],
        18 => vec![guitar("Nylon Guitar", Instrument::NylonGuitar, STD)],
        19 => vec![
            vocals(),
            guitar("Guitar", Instrument::OverdriveGuitar, STD),
            inst("Keyboards", Instrument::Synth, 4),
            bass("Bass"),
            drums(),
        ],
        20 => vec![vocals(), guitar("Guitar", Instrument::OverdriveGuitar, STD), bass("Bass"), drums()],
        21 => vec![guitar("Steel Guitar", Instrument::SteelGuitar, STD)],
        22 => vec![
            guitar("Guitar 1", Instrument::DistortionGuitar, &D_STD),
            guitar("Guitar 2", Instrument::DistortionGuitar, &D_STD),
            tr("Bass", Instrument::Bass, &[41, 36, 31, 24], 1),
            drums(),
        ],
        23 => vec![
            guitar("Rhythm Guitar", Instrument::DistortionGuitar, STD),
            guitar("Lead Guitar", Instrument::DistortionGuitar, STD),
            bass("Bass"),
            drums(),
            vocals(),
        ],
        // The empty document keeps a plain guitar until its first track is chosen.
        _ => vec![guitar("Steel Guitar", Instrument::SteelGuitar, STD)],
    }
}

/// A standard template as a song: its tracks, each in its own colour, with empty bars.
pub fn template(k: usize) -> Song {
    let mut s = Song::new_blank();
    s.title = "Untitled".into();
    s.tracks = template_tracks(k);
    for (i, t) in s.tracks.iter_mut().enumerate() {
        t.color = TRACK_COLORS[i % TRACK_COLORS.len()];
    }
    s.normalize();
    s
}

/// The names to list recent files under: the file name without its extension, as before, but
/// where two files would read the same, the extension is added, and where they still would, the
/// folder they are in, so each can be told from the other.
pub(super) fn recent_labels(files: &[PathBuf]) -> Vec<String> {
    let lower = |s: String| s.to_lowercase();
    let stem = |p: &PathBuf| p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let full = |p: &PathBuf| p.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let stems: Vec<String> = files.iter().map(stem).collect();
    let fulls: Vec<String> = files.iter().map(full).collect();
    files
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let same = |names: &[String]| names.iter().filter(|n| lower((*n).clone()) == lower(names[i].clone())).count() > 1;
            if !same(&stems) {
                return stems[i].clone();
            }
            if !same(&fulls) {
                return fulls[i].clone();
            }
            let folder = p.parent().and_then(|d| d.file_name()).map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            if folder.is_empty() { fulls[i].clone() } else { format!("{}  ({folder})", fulls[i]) }
        })
        .collect()
}

enum Item {
    Recent(PathBuf),
    Template(usize),
    /// One of your own templates (File > Save As Template).
    UserTemplate(PathBuf),
    Example,
}

impl App {
    fn home_items(&self) -> Vec<(String, Item)> {
        let q = self.home_search.to_lowercase();
        let mut v: Vec<(String, Item)> = match self.home_tab {
            0 => recent_labels(&self.recent).into_iter().zip(self.recent.iter()).map(|(name, p)| (name, Item::Recent(p.clone()))).collect(),
            1 => TEMPLATES
                .iter()
                .enumerate()
                .map(|(i, n)| (n.to_string(), Item::Template(i)))
                .chain(Self::user_templates().into_iter().map(|p| {
                    let name = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                    (format!("{name}  (my template)"), Item::UserTemplate(p))
                }))
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
                self.open_path(p);
            }
            Item::Template(k) => self.request(Pending::Template(*k)),
            Item::UserTemplate(p) => self.open_template_file(p.clone()),
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
        let mut open_chart = false;
        let r_new = Rect::from_center_size(Pos2::new(full.center().x, y), Vec2::new(220.0, 26.0));
        let r_open =
            Rect::from_center_size(Pos2::new(full.center().x, y + 36.0), Vec2::new(220.0, 26.0));
        let r_chart =
            Rect::from_center_size(Pos2::new(full.center().x, y + 72.0), Vec2::new(220.0, 26.0));
        let (k_new, k_open) = (format!("({})", super::shortcuts::sc(super::shortcuts::Act::New)), format!("({})", super::shortcuts::sc(super::shortcuts::Act::Open)));
        for (r, label, key, which) in [
            (r_new, "New File", k_new.as_str(), 0),
            (r_open, "Open File…", k_open.as_str(), 1),
            (r_chart, "New Chord Chart", "", 2),
        ] {
            let resp = ui.interact(r, ui.id().with(("home_btn", which)), Sense::click());
            let p = ui.painter();
            if resp.hovered() {
                p.rect_filled(r, 4.0, Color32::from_gray(60));
            }
            let ic =
                Rect::from_center_size(Pos2::new(r.left() + 30.0, r.center().y), Vec2::splat(14.0));
            if which == 2 {
                // A little page of lines.
                let c = Color32::from_gray(220);
                p.rect_stroke(ic, 1.0, egui::Stroke::new(1.2, c), egui::StrokeKind::Inside);
                for k in 0..3 {
                    let yy = ic.top() + 4.0 + k as f32 * 3.0;
                    p.line_segment([Pos2::new(ic.left() + 3.0, yy), Pos2::new(ic.right() - 3.0, yy)], egui::Stroke::new(1.0, c));
                }
            } else if which == 0 {
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
                match which {
                    0 => open_new = true,
                    1 => open_file = true,
                    _ => open_chart = true,
                }
            }
        }
        y += 112.0;
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
                Item::UserTemplate(p) => format!("my-template-{}", p.display()),
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
                    Item::UserTemplate(p) => fileio::load_song(p).ok(),
                    Item::Example => Some(Song::demo()),
                };
                self.preview = song.map(|s| (key.clone(), s));
                if self.preview.is_none() {
                    self.preview = Some((key, Song::new_blank()));
                }
            }
            if let Some((_, song)) = &self.preview {
                let avail = prev_r.shrink(14.0);
                let (pw, ph) = song.style.page_mm();
                let zoom = (avail.height() / (ph * crate::style::MM)).min(avail.width() / (pw * crate::style::MM));
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
                    printing: false,
                    transparent: false,
                    play_cursor: crate::score::PlayCursor::Smooth,
                    play_bar: true,
                    bar_errors: 0,
 unreached: &[],
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
        if let Some(i) = open_idx
            && let Some((_, it)) = items.get(i)
        {
            self.open_item(it);
        }
        if open_new {
            self.request(Pending::New);
        }
        if open_file {
            self.request(Pending::Open);
        }
        if open_chart {
            self.request(Pending::NewChart);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(list: &[&str]) -> Vec<PathBuf> {
        list.iter().map(PathBuf::from).collect()
    }

    #[test]
    fn names_that_differ_are_listed_as_before() {
        assert_eq!(recent_labels(&paths(&["a/One.gp", "b/Two.gp"])), vec!["One", "Two"]);
    }

    #[test]
    fn the_same_name_in_two_formats_gets_its_extension() {
        let l = recent_labels(&paths(&["s/Morning Walk.gp", "s/Morning Walk.musicxml", "s/Other.gp"]));
        assert_eq!(l, vec!["Morning Walk.gp", "Morning Walk.musicxml", "Other"]);
    }

    #[test]
    fn the_same_file_name_in_two_folders_gets_its_folder() {
        let l = recent_labels(&paths(&["samples/Morning Walk.gp", "Desktop/Morning Walk.gp", "samples/Morning Walk.musicxml"]));
        assert_eq!(l, vec!["Morning Walk.gp  (samples)", "Morning Walk.gp  (Desktop)", "Morning Walk.musicxml"]);
    }

    #[test]
    fn capitals_do_not_make_names_different() {
        let l = recent_labels(&paths(&["a/song.gp", "b/Song.gp"]));
        assert_eq!(l, vec!["song.gp  (a)", "Song.gp  (b)"]);
    }

    #[test]
    fn files_that_are_gone_are_not_offered_as_recent() {
        let dir = std::env::temp_dir().join(format!("gc-recent-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let there = dir.join("Here.gp");
        std::fs::write(&there, b"x").unwrap();
        let gone = dir.join("Gone.gp");
        assert_eq!(only_existing(vec![gone.clone(), there.clone()]), vec![there.clone()]);
        assert!(only_existing(vec![gone]).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
