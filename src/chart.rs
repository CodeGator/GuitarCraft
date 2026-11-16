//! Chord charts: a song written as sections of lyrics with chord names above
//! them (or bars of chords), played in the order of a "map". Saved as this
//! app's own .chart files (Guitar Pro has no such document), printed and
//! exported like scores. No playback.
//!
//! A section's text uses inline chords: `[E]Let my heart be a [B]temple`.
//! A line starting with `|` is a line of bars: `| E /// | B //// |`.

use egui::{Align2, Color32, FontFamily, FontId, Painter, Pos2, Rect, Stroke, Vec2};
use serde::{Deserialize, Serialize};

pub const EXT: &str = "chart";
/// The ending chord charts had before (16 Nov 2026); such files still open.
pub const OLD_EXT: &str = "gcchart";

/// Whether a file is a chord chart (by its ending, old or new).
pub fn is_chart_path(p: &std::path::Path) -> bool {
    p.extension().map(|e| e.eq_ignore_ascii_case(EXT) || e.eq_ignore_ascii_case(OLD_EXT)).unwrap_or(false)
}

/// A chart page in points at zoom 1 (A4, as score pages).
pub const PAGE_W: f32 = 820.0;
pub const PAGE_H: f32 = 1160.0;
pub const PAGE_MM: (f32, f32) = (210.0, 297.0);
/// The most columns a chart can have.
pub const MAX_COLUMNS: u8 = 6;
const MARGIN: f32 = 44.0;
const COL_GAP: f32 = 30.0;
const TOP_OTHER: f32 = 50.0;
const BOTTOM: f32 = PAGE_H - 50.0;

#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq, Eq, Default)]
pub enum Kind {
    Intro,
    #[default]
    Verse,
    PreChorus,
    Chorus,
    Bridge,
    Interlude,
    Instrumental,
    Solo,
    Outro,
    Tag,
    Other,
}

impl Kind {
    pub const ALL: [Kind; 11] = [
        Kind::Intro,
        Kind::Verse,
        Kind::PreChorus,
        Kind::Chorus,
        Kind::Bridge,
        Kind::Interlude,
        Kind::Instrumental,
        Kind::Solo,
        Kind::Outro,
        Kind::Tag,
        Kind::Other,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Kind::Intro => "Intro",
            Kind::Verse => "Verse",
            Kind::PreChorus => "Pre-Chorus",
            Kind::Chorus => "Chorus",
            Kind::Bridge => "Bridge",
            Kind::Interlude => "Interlude",
            Kind::Instrumental => "Instrumental",
            Kind::Solo => "Solo",
            Kind::Outro => "Outro",
            Kind::Tag => "Tag",
            Kind::Other => "Other",
        }
    }
    /// The tint behind the section's lines and its name in the map.
    pub fn tint(self) -> Option<Color32> {
        match self {
            Kind::Verse => Some(Color32::from_rgb(244, 204, 204)),
            Kind::Intro | Kind::Interlude | Kind::Instrumental | Kind::Solo => Some(Color32::from_rgb(255, 242, 204)),
            Kind::Bridge => Some(Color32::from_rgb(208, 224, 227)),
            Kind::PreChorus => Some(Color32::from_rgb(217, 234, 211)),
            Kind::Outro => Some(Color32::from_rgb(252, 229, 205)),
            Kind::Tag => Some(Color32::from_rgb(234, 209, 220)),
            Kind::Chorus | Kind::Other => None,
        }
    }
}

/// A colour as red, green, blue.
pub type Rgb = [u8; 3];

pub fn rgb(c: Color32) -> Rgb {
    [c.r(), c.g(), c.b()]
}

pub fn color(c: Rgb) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

/// The file format written now. Version 1 had no colours of its own: its
/// sections took the colour suggested for their kind.
pub const FORMAT: &str = "GuitarCraft chord chart 2";
const FORMAT_1: &str = "GuitarCraft chord chart 1";

/// How one kind of text on the chart looks: font, size (points at 100 %),
/// bold, italic and colour.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
#[serde(default)]
pub struct TextLook {
    pub family: String,
    pub size: f32,
    pub bold: bool,
    pub italic: bool,
    pub color: Rgb,
}

impl Default for TextLook {
    fn default() -> Self {
        TextLook::new("Arial", 10.5, false, INK)
    }
}

impl TextLook {
    pub fn new(family: &str, size: f32, bold: bool, c: Color32) -> TextLook {
        TextLook { family: family.into(), size, bold, italic: false, color: rgb(c) }
    }
    pub fn font(&self, z: f32) -> FontId {
        let fam = if crate::style::FONTS.iter().any(|f| f.0 == self.family) { self.family.as_str() } else { "Arial" };
        FontId::new((self.size * z).max(1.0), FontFamily::Name(format!("{fam}|{}", self.bold as usize + 2 * self.italic as usize).into()))
    }
    pub fn ink(&self) -> Color32 {
        color(self.color)
    }
    /// The height of one line of this text.
    fn row(&self) -> f32 {
        self.size.max(4.0) * 1.43
    }
    /// The height of a heading line of this text.
    fn head(&self) -> f32 {
        self.size.max(4.0) * 1.62
    }
}

/// The look of the whole chart: each kind of text, and whether sections'
/// lines are shaded with their colours.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
#[serde(default)]
pub struct ChartLook {
    pub title: TextLook,
    /// Artist, capo, tuning, BPM, time and key.
    pub info: TextLook,
    pub map: TextLook,
    /// Section names above each section.
    pub heading: TextLook,
    /// "Repeat Chorus" lines.
    pub repeat: TextLook,
    pub chords: TextLook,
    pub lyrics: TextLook,
    /// Lines of bars: | E /// | B //// |
    pub bars: TextLook,
    /// The copyright line at the foot of each page.
    pub copyright: TextLook,
    pub shade: bool,
    /// Columns of text on a page (1 to MAX_COLUMNS).
    pub columns: u8,
    /// Whether the chord diagrams are printed.
    pub show_diagrams: bool,
    /// Whether the map (the playing order) is printed under the title.
    pub show_map: bool,
    /// The colour a new section of each kind starts with (none: plain).
    pub kind_colors: Vec<(Kind, Option<Rgb>)>,
}

/// The colour suggested for each kind in the standard stylesheet.
fn standard_kind_colors() -> Vec<(Kind, Option<Rgb>)> {
    Kind::ALL.iter().map(|k| (*k, k.tint().map(rgb))).collect()
}

impl Default for ChartLook {
    fn default() -> Self {
        ChartLook::standard()
    }
}

impl ChartLook {
    /// The look of the person's picture.
    pub fn standard() -> ChartLook {
        ChartLook {
            title: TextLook::new("Arial", 22.0, false, INK),
            info: TextLook::new("Arial", 10.5, false, INK),
            map: TextLook::new("Arial", 10.5, true, INK),
            heading: TextLook::new("Arial", 10.5, true, PURPLE),
            repeat: TextLook::new("Arial", 10.5, true, PURPLE),
            chords: TextLook::new("Arial", 10.5, true, INK),
            lyrics: TextLook::new("Arial", 10.5, false, INK),
            bars: TextLook::new("Arial", 10.5, true, INK),
            copyright: TextLook::new("Arial", 7.5, false, Color32::from_gray(90)),
            shade: true,
            columns: 2,
            show_diagrams: false,
            show_map: true,
            kind_colors: standard_kind_colors(),
        }
    }
    /// For a dimly lit stage: large type, chords bigger, bolder and in a
    /// colour of their own so they stand apart from the words.
    pub fn stage() -> ChartLook {
        ChartLook {
            title: TextLook::new("Arial", 24.0, true, INK),
            info: TextLook::new("Arial", 11.0, false, INK),
            map: TextLook::new("Arial", 12.0, true, INK),
            heading: TextLook::new("Arial", 14.0, true, PURPLE),
            repeat: TextLook::new("Arial", 14.0, true, PURPLE),
            chords: TextLook::new("Arial", 16.0, true, Color32::from_rgb(0, 60, 200)),
            lyrics: TextLook::new("Arial", 14.0, false, Color32::BLACK),
            bars: TextLook::new("Arial", 15.0, true, Color32::from_rgb(0, 60, 200)),
            copyright: TextLook::new("Arial", 8.0, false, Color32::from_gray(70)),
            shade: true,
            columns: 1,
            show_diagrams: false,
            show_map: true,
            kind_colors: standard_kind_colors(),
        }
    }
    /// The colour a new section of this kind starts with.
    pub fn kind_color(&self, kind: Kind) -> Option<Rgb> {
        match self.kind_colors.iter().find(|k| k.0 == kind) {
            Some(k) => k.1,
            None => kind.tint().map(rgb),
        }
    }
    /// The same list with every kind present, in order (for the editor).
    pub fn fix_kind_colors(&mut self) {
        let all: Vec<(Kind, Option<Rgb>)> = Kind::ALL.iter().map(|k| (*k, self.kind_color(*k))).collect();
        self.kind_colors = all;
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
#[serde(default)]
pub struct Section {
    pub id: u32,
    pub name: String,
    pub kind: Kind,
    pub text: String,
    /// The colour behind the section's lines and its name in the map (none: plain).
    pub color: Option<Rgb>,
    /// Starts at the top of the next column (a column break before it).
    pub new_column: bool,
}

impl Default for Section {
    fn default() -> Self {
        Section { id: 0, name: "Verse 1".into(), kind: Kind::Verse, text: String::new(), color: None, new_column: false }
    }
}

impl Section {
    /// The short name used in the map: "V1" for "Verse 1", otherwise the name.
    pub fn short(&self) -> String {
        let n = self.name.trim();
        if let Some(rest) = n.strip_prefix("Verse ") {
            if !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()) {
                return format!("V{rest}");
            }
        }
        n.to_string()
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
#[serde(default)]
pub struct MapEntry {
    pub section: u32,
    /// Times played in a row (2 shows "(x2)").
    pub repeat: u8,
    /// Printed instead of the automatic heading ("Repeat Chorus"); empty: automatic.
    pub label: String,
    /// A colour for this entry's heading instead of the chart's.
    pub color: Option<Rgb>,
    /// A tempo change where this entry starts (not on the first entry: that's the chart's BPM).
    pub tempo: Option<u32>,
    /// From this tempo, change gradually to the next one.
    pub tempo_ramp: bool,
}

impl Default for MapEntry {
    fn default() -> Self {
        MapEntry { section: 0, repeat: 1, label: String::new(), color: None, tempo: None, tempo_ramp: false }
    }
}

impl MapEntry {
    pub fn new(section: u32, repeat: u8) -> MapEntry {
        MapEntry { section, repeat, ..Default::default() }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
#[serde(default)]
pub struct ChordChart {
    pub format: String,
    pub title: String,
    pub artist: String,
    pub capo: String,
    /// A note about the tuning, e.g. "-1 whole step" (shown as "(Tuned: ...)").
    pub tuning: String,
    pub bpm: u32,
    /// From the starting tempo, change gradually to the first tempo change.
    pub bpm_ramp: bool,
    pub time: String,
    pub key: String,
    pub sections: Vec<Section>,
    pub map: Vec<MapEntry>,
    /// Chord diagrams shown at the end, by name.
    pub diagrams: Vec<String>,
    /// Printed in small type at the foot of every page.
    pub copyright: String,
    /// The chart's stylesheet: fonts, sizes, colours, columns and what is printed.
    pub look: ChartLook,
}

impl Default for ChordChart {
    fn default() -> Self {
        ChordChart::new_blank()
    }
}

impl ChordChart {
    pub fn new_blank() -> ChordChart {
        let s = |id: u32, name: &str, kind: Kind, text: &str| Section { id, name: name.into(), kind, text: text.into(), color: kind.tint().map(rgb), new_column: false };
        ChordChart {
            format: FORMAT.into(),
            title: "Untitled".into(),
            artist: String::new(),
            capo: "None".into(),
            tuning: String::new(),
            bpm: 90,
            bpm_ramp: false,
            time: "4/4".into(),
            key: String::new(),
            sections: vec![
                s(1, "Intro", Kind::Intro, "| G /// | C /// | G /// | D /// |"),
                s(2, "Verse 1", Kind::Verse, "[G]Write your first line [C]here\n[G]Chords go in [D]brackets"),
                s(3, "Chorus", Kind::Chorus, "[C]This is the [G]chorus\n[C]Sing it [D]loud"),
            ],
            map: vec![MapEntry::new(1, 1), MapEntry::new(2, 1), MapEntry::new(3, 1)],
            diagrams: vec!["G".into(), "C".into(), "D".into()],
            copyright: String::new(),
            look: ChartLook::standard(),
        }
    }

    /// A made-up example in the layout of the person's picture.
    pub fn sample() -> ChordChart {
        let s = |id: u32, name: &str, kind: Kind, text: &str| Section { id, name: name.into(), kind, text: text.into(), color: kind.tint().map(rgb), new_column: false };
        let m = MapEntry::new;
        ChordChart {
            format: FORMAT.into(),
            title: "Morning Walk".into(),
            artist: "GuitarCraft demo".into(),
            capo: "None".into(),
            tuning: "-1 whole step".into(),
            bpm: 63,
            bpm_ramp: false,
            time: "4/4".into(),
            key: "E".into(),
            sections: vec![
                s(1, "Intro", Kind::Intro, "| E /// | B //// | C#m /// | A /// | E /// |"),
                s(2, "Verse 1", Kind::Verse, "[E]Out along the river road\n[B]Where the morning light is slow\n[C#m]Every step a quiet song\n[A]Walking where the [E]day be[B]longs"),
                s(3, "Verse 2", Kind::Verse, "[E]Birds are calling from the trees\n[B]Carried on the open breeze\n[C#m]Nothing here I need to find\n[A]Leave the noise [E]behind"),
                s(4, "Chorus", Kind::Chorus, "And the [A]sun comes up a[E]gain\nOver [A]hills and through the [E]rain\nWe keep [A]walk[B]ing [C#m]on and [E]on [B]"),
                s(5, "Interlude", Kind::Interlude, "| E /// | B //// | C# /// | A /// |"),
                s(6, "Verse 3", Kind::Verse, "[E]When the evening settles in\n[B]And the road comes round again\n[C#m]I will keep this morning near\n[A]Every step still [E]clear"),
                s(7, "Bridge", Kind::Bridge, "[C#m]Step by step and side by [E]side\n[A]Open road and open [E]sky\n[E]Here we go, here we [B]go"),
                s(8, "Verse 4", Kind::Verse, "[C#m]Lanterns hanging in the square\n[B]Music drifting through the air\n[A]Every window, every [E]light"),
                s(9, "Verse 5", Kind::Verse, "Walk with [A]me until the [E]dawn\nSing a[A]long the road we're [E]on\nWe will [A]go where [B]rivers [A]bend and [E]never [B]stop"),
                s(10, "Interlude 2", Kind::Interlude, "| C#m /// //// | A /// //// |"),
            ],
            map: vec![m(1, 1), m(2, 1), m(3, 1), m(4, 1), m(5, 1), m(6, 1), m(8, 1), m(4, 1), m(9, 1), m(10, 1), m(7, 2), m(4, 1), m(9, 1)],
            diagrams: vec!["E5".into(), "A".into(), "C#".into(), "B".into()],
            copyright: "\u{00A9} 2026 GuitarCraft demo. Words and music made up for this example.".into(),
            // As in the person's picture: with chord diagrams.
            look: ChartLook { show_diagrams: true, ..ChartLook::standard() },
        }
    }

    pub fn section(&self, id: u32) -> Option<&Section> {
        self.sections.iter().find(|s| s.id == id)
    }

    /// Sections in the map: how many (they come first in the list once in order).
    pub fn played_count(&self) -> usize {
        self.sections.iter().filter(|s| self.map.iter().any(|e| e.section == s.id)).count()
    }

    /// Puts the sections list in the order the sections are first played in
    /// the map (the order they print); sections not in the map come last.
    pub fn sync_sections(&mut self) {
        let map = &self.map;
        let first = |id: u32| map.iter().position(|e| e.section == id).unwrap_or(usize::MAX);
        let mut keyed: Vec<(usize, Section)> = self.sections.drain(..).map(|s| (first(s.id), s)).collect();
        keyed.sort_by_key(|k| k.0);
        self.sections = keyed.into_iter().map(|k| k.1).collect();
    }

    /// Whether section `i` can move one place earlier (`dir` -1) or later (+1).
    pub fn can_move_section(&self, i: usize, dir: i32) -> bool {
        let played = self.played_count();
        let (lo, hi) = if i < played { (0, played) } else { (played, self.sections.len()) };
        let j = i as i64 + dir as i64;
        j >= lo as i64 && j < hi as i64
    }

    /// Moves section `i` one place earlier (-1) or later (+1) in the song:
    /// its first time in the map changes places with its neighbour's, so the
    /// map and the printed chart follow. Returns the section's new place.
    pub fn move_section(&mut self, i: usize, dir: i32) -> usize {
        self.sync_sections();
        if !self.can_move_section(i, dir) {
            return i;
        }
        let j = (i as i64 + dir as i64) as usize;
        let (a, b) = (self.sections[i].id, self.sections[j].id);
        let pa = self.map.iter().position(|e| e.section == a);
        let pb = self.map.iter().position(|e| e.section == b);
        match (pa, pb) {
            (Some(pa), Some(pb)) => {
                self.map.swap(pa, pb);
                self.sync_sections();
            }
            // Neither is played: only the list changes.
            _ => self.sections.swap(i, j),
        }
        j
    }

    /// The chords that can be shown as diagrams: every chord in the song (in
    /// the order first played), then any chosen earlier that the song no
    /// longer uses. Each with whether the song uses it.
    pub fn diagram_choices(&self) -> Vec<(String, bool)> {
        let used = self.chords_used();
        let mut out: Vec<(String, bool)> = used.iter().map(|u| (u.clone(), true)).collect();
        out.extend(self.diagrams.iter().filter(|d| !used.contains(d)).map(|d| (d.clone(), false)));
        out
    }

    /// Shows or leaves out a chord's diagram. The diagrams print in the order
    /// the song first plays their chords.
    pub fn set_diagram(&mut self, name: &str, on: bool) {
        let used = self.chords_used();
        let mut chosen: Vec<String> = self.diagrams.iter().filter(|d| d.as_str() != name).cloned().collect();
        if on {
            chosen.push(name.to_string());
        }
        let mut out: Vec<String> = used.iter().filter(|u| chosen.contains(u)).cloned().collect();
        out.extend(chosen.into_iter().filter(|c| !used.contains(c)));
        self.diagrams = out;
    }

    /// The tempo points along the map, as the tempo editor shows them:
    /// (map entry, bpm, gradual change to the next point). The first is the chart's BPM.
    pub fn tempo_points(&self) -> Vec<(usize, u32, bool)> {
        let mut v = vec![(0, self.bpm, self.bpm_ramp)];
        for (k, e) in self.map.iter().enumerate().skip(1) {
            if let Some(t) = e.tempo {
                v.push((k, t, e.tempo_ramp));
            }
        }
        v
    }

    /// Takes the tempo editor's points back.
    pub fn set_tempo_points(&mut self, pts: &[(usize, u32, bool)]) {
        if let Some(&(_, t, r)) = pts.iter().find(|p| p.0 == 0) {
            self.bpm = t;
            self.bpm_ramp = r;
        }
        for (k, e) in self.map.iter_mut().enumerate() {
            let p = pts.iter().find(|p| p.0 == k && k > 0);
            e.tempo = p.map(|p| p.1);
            e.tempo_ramp = p.map(|p| p.2).unwrap_or(false);
        }
    }

    /// What a map entry's heading says about the tempo: " — 72 BPM", or
    /// " — 72 → 80 BPM" when it changes gradually from there.
    pub fn tempo_note(&self, entry: usize) -> String {
        let pts = self.tempo_points();
        let Some(k) = pts.iter().position(|p| p.0 == entry) else { return String::new() };
        let (_, t, ramp) = pts[k];
        if entry == 0 && !ramp {
            return String::new();
        }
        match pts.get(k + 1) {
            Some(&(_, next, _)) if ramp => format!(" \u{2014} {t} \u{2192} {next} BPM"),
            _ => format!(" \u{2014} {t} BPM"),
        }
    }

    pub fn next_id(&self) -> u32 {
        self.sections.iter().map(|s| s.id).max().unwrap_or(0) + 1
    }

    /// A name for a new section of a kind: "Verse 3" after "Verse 2", "Chorus 2" ...
    pub fn new_name(&self, kind: Kind) -> String {
        let base = kind.label();
        let count = self.sections.iter().filter(|s| s.kind == kind).count();
        if kind == Kind::Verse {
            return format!("Verse {}", count + 1);
        }
        if count == 0 { base.to_string() } else { format!("{base} {}", count + 1) }
    }

    /// Every chord name used, in order of first use.
    pub fn chords_used(&self) -> Vec<String> {
        let mut out: Vec<String> = vec![];
        for e in &self.map {
            if let Some(s) = self.section(e.section) {
                for line in parse(&s.text) {
                    let names: Vec<String> = match line {
                        Line::Bars(t) => t
                            .split(|c: char| c == '|' || c.is_whitespace())
                            .map(|w| w.trim_matches(|c| c == '[' || c == ']'))
                            .filter(|w| split_chord(w).is_some())
                            .map(|w| w.to_string())
                            .collect(),
                        Line::Lyric { chords, .. } => chords.into_iter().map(|c| c.1).collect(),
                    };
                    for n in names {
                        if !out.contains(&n) {
                            out.push(n);
                        }
                    }
                }
            }
        }
        out
    }

    pub fn load(path: &std::path::Path) -> Result<ChordChart, String> {
        let data = std::fs::read(path).map_err(|e| e.to_string())?;
        let v: serde_json::Value = serde_json::from_slice(&data).map_err(|e| format!("Not a chord chart: {e}"))?;
        let first = v.get("format").and_then(|f| f.as_str()) == Some(FORMAT_1);
        // Charts saved before the stylesheet kept these beside it.
        let columns = v.get("columns").and_then(|c| c.as_u64());
        let diagrams = v.get("show_diagrams").and_then(|c| c.as_bool());
        let in_look = v.get("look").map(|l| l.get("columns").is_some()).unwrap_or(false);
        let mut c: ChordChart = serde_json::from_value(v).map_err(|e| format!("Not a chord chart: {e}"))?;
        if !in_look {
            if let Some(n) = columns {
                c.look.columns = (n as u8).clamp(1, MAX_COLUMNS);
            }
            if let Some(d) = diagrams {
                c.look.show_diagrams = d;
            }
        }
        c.look.fix_kind_colors();
        if first {
            // Charts saved before sections had colours of their own.
            for s in &mut c.sections {
                s.color = s.kind.tint().map(rgb);
            }
            c.format = FORMAT.into();
        }
        Ok(c)
    }

    pub fn save(&self, path: &std::path::Path) -> Result<(), String> {
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(path, json).map_err(|e| e.to_string())
    }
}

/// One line of a section.
#[derive(Clone, Debug, PartialEq)]
pub enum Line {
    /// A lyric line with chords placed at character positions of the lyric.
    Lyric { text: String, chords: Vec<(usize, String)> },
    /// A line of bars, written as typed: `| E /// | B //// |`.
    Bars(String),
}

/// Reads a section's text into lines.
pub fn parse(text: &str) -> Vec<Line> {
    let mut out = vec![];
    for raw in text.lines() {
        let line = raw.trim_end();
        if line.trim_start().starts_with('|') {
            // Chords in bars are written plainly; brackets typed there are dropped.
            out.push(Line::Bars(line.trim().replace(['[', ']'], "")));
            continue;
        }
        let mut lyric = String::new();
        let mut chords = vec![];
        let mut rest = line;
        while let Some(open) = rest.find('[') {
            lyric.push_str(&rest[..open]);
            match rest[open + 1..].find(']') {
                Some(close) => {
                    let name = rest[open + 1..open + 1 + close].trim().trim_matches(|c| c == '[' || c == ']').trim().to_string();
                    if !name.is_empty() {
                        chords.push((lyric.chars().count(), name));
                    }
                    rest = &rest[open + 1 + close + 1..];
                    // "[[E]]": the extra closing bracket isn't a word.
                    rest = rest.trim_start_matches(']');
                }
                None => {
                    lyric.push_str(&rest[open..]);
                    rest = "";
                }
            }
        }
        lyric.push_str(rest);
        out.push(Line::Lyric { text: lyric, chords });
    }
    out
}

/// Common guitar chord shapes: frets from the low E string to the high E
/// string (-1 = not played).
pub fn shape(name: &str) -> Option<[i8; 6]> {
    // Worked out from the name: any chord the name describes gets a diagram.
    let n = name.trim().replace('\u{266F}', "#").replace('\u{266D}', "b");
    let (root, rest, bass) = split_chord(&n)?;
    crate::chordshape::shape_of(&n, root, &rest, bass)
}

// ----- transposing -----

/// How transposed chords are spelled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Spelling {
    /// Sharps or flats as the new key suggests.
    Auto,
    Sharps,
    Flats,
}

/// A note's name at its start ("C#m7" gives (1, 2): C sharp, two characters).
fn note_at(s: &str) -> Option<(i32, usize)> {
    let mut it = s.chars();
    let base = match it.next()? {
        'C' => 0,
        'D' => 2,
        'E' => 4,
        'F' => 5,
        'G' => 7,
        'A' => 9,
        'B' => 11,
        _ => return None,
    };
    match it.next() {
        Some('#') | Some('\u{266F}') => Some(((base + 1) % 12, 1 + s.chars().nth(1).map(|c| c.len_utf8()).unwrap_or(1))),
        Some('b') | Some('\u{266D}') => Some(((base + 11) % 12, 1 + s.chars().nth(1).map(|c| c.len_utf8()).unwrap_or(1))),
        _ => Some((base, 1)),
    }
}

/// A chord name split into root, the rest ("m7", "sus4") and a bass note
/// after a slash ("D/F#"). None when it isn't a chord name.
pub fn split_chord(name: &str) -> Option<(i32, String, Option<i32>)> {
    let n = name.trim();
    let (root, len) = note_at(n)?;
    let rest = &n[len..];
    if let Some(k) = rest.rfind('/')
        && let Some((bass, blen)) = note_at(&rest[k + 1..])
        && k + 1 + blen == rest.len()
    {
        return Some((root, rest[..k].to_string(), Some(bass)));
    }
    // Only letters, digits and the usual signs may follow a root.
    if !rest.chars().all(|c| c.is_ascii_alphanumeric() || "#b+-()/\u{00B0}\u{00F8}\u{0394}\u{266F}\u{266D}".contains(c)) {
        return None;
    }
    Some((root, rest.to_string(), None))
}

fn note_name(pc: i32, flats: bool) -> &'static str {
    crate::model::pitch_name(pc.rem_euclid(12) as u8, flats)
}

/// Whether a key is written with flats (F, Bb, Eb ... and their relative minors).
fn key_uses_flats(root: i32, minor: bool) -> bool {
    let major = if minor { (root + 3).rem_euclid(12) } else { root.rem_euclid(12) };
    // F, Bb, Eb, Ab and Db major; F# rather than Gb, but Ebm rather than D#m.
    matches!(major, 5 | 10 | 3 | 8 | 1) || (minor && major == 6)
}

/// A key name's root and whether it's minor ("C#m", "Am", "Bb major", "E minor").
pub fn split_key(key: &str) -> Option<(i32, bool)> {
    let k = key.trim();
    let (root, len) = note_at(k)?;
    let rest = k[len..].trim().to_ascii_lowercase();
    let minor = rest == "m" || rest.starts_with("min") || rest == "-";
    if !(rest.is_empty() || minor || rest.starts_with("maj")) {
        return None;
    }
    Some((root, minor))
}

/// One chord name moved by `semis` half steps.
pub fn transpose_chord(name: &str, semis: i32, flats: bool) -> String {
    match split_chord(name) {
        Some((root, rest, bass)) => {
            let mut out = format!("{}{}", note_name(root + semis, flats), rest);
            if let Some(b) = bass {
                out.push('/');
                out.push_str(note_name(b + semis, flats));
            }
            out
        }
        None => name.to_string(),
    }
}

/// A section's text with every chord moved: the [chords] above words and the
/// chords in lines of bars. Words are left alone.
pub fn transpose_text(text: &str, semis: i32, flats: bool) -> String {
    let mut out = String::new();
    for (k, line) in text.split('\n').enumerate() {
        if k > 0 {
            out.push('\n');
        }
        if line.trim_start().starts_with('|') {
            // Bar lines: each word that is a chord name.
            let mut word = String::new();
            for c in line.chars().chain(std::iter::once('\0')) {
                if c == '\0' || c.is_whitespace() || c == '|' {
                    if !word.is_empty() {
                        // Keep any brackets around a chord in bars.
                        let inner = word.trim_matches(|c| c == '[' || c == ']');
                        let start = word.find(inner).unwrap_or(0);
                        out.push_str(&word[..start]);
                        out.push_str(&transpose_chord(inner, semis, flats));
                        out.push_str(&word[start + inner.len()..]);
                        word.clear();
                    }
                    if c != '\0' {
                        out.push(c);
                    }
                } else {
                    word.push(c);
                }
            }
            continue;
        }
        let mut rest = line;
        while let Some(open) = rest.find('[') {
            out.push_str(&rest[..=open]);
            match rest[open + 1..].find(']') {
                Some(close) => {
                    out.push_str(&transpose_chord(&rest[open + 1..open + 1 + close], semis, flats));
                    out.push(']');
                    rest = &rest[open + 1 + close + 1..];
                }
                None => {
                    rest = &rest[open + 1..];
                    break;
                }
            }
        }
        out.push_str(rest);
    }
    out
}

impl ChordChart {
    /// Whether the chart writes its chords mostly with flats.
    fn writes_flats(&self) -> bool {
        let used = self.chords_used();
        let count = |c: char| used.iter().filter(|n| n.chars().nth(1) == Some(c)).count();
        count('b') > count('#')
    }

    /// Flats or sharps for a transposition by `semis`.
    pub fn use_flats(&self, semis: i32, spelling: Spelling) -> bool {
        match spelling {
            Spelling::Sharps => false,
            Spelling::Flats => true,
            Spelling::Auto => match split_key(&self.key) {
                Some((root, minor)) => key_uses_flats(root + semis, minor),
                None => self.writes_flats(),
            },
        }
    }

    /// Moves every chord by `semis` half steps: in the sections given (all
    /// when `only` is None), and with all of them also the key and the diagrams.
    pub fn transpose(&mut self, semis: i32, spelling: Spelling, only: Option<usize>) {
        let flats = self.use_flats(semis, spelling);
        for (i, s) in self.sections.iter_mut().enumerate() {
            if only.is_none() || only == Some(i) {
                s.text = transpose_text(&s.text, semis, flats);
            }
        }
        if only.is_none() {
            for d in &mut self.diagrams {
                *d = transpose_chord(d, semis, flats);
            }
            if let Some((root, _)) = split_key(&self.key) {
                let k = self.key.trim();
                let len = note_at(k).map(|n| n.1).unwrap_or(0);
                self.key = format!("{}{}", note_name(root + semis, flats), &k[len..]);
            }
        }
    }
}

/// The 12 major keys, for choosing a key to move to.
pub fn key_names(minor: bool, flats: bool) -> Vec<String> {
    (0..12).map(|k| format!("{}{}", note_name(k, flats), if minor { "m" } else { "" })).collect()
}


/// Measures text: its width in page points for a font at zoom 1.
pub type Measure<'a> = &'a dyn Fn(&str, &FontId) -> f32;

/// Measures with the real fonts, through a painter.
pub fn painter_measure(p: &Painter) -> impl Fn(&str, &FontId) -> f32 + '_ {
    move |t: &str, f: &FontId| p.layout_no_wrap(t.to_string(), f.clone(), Color32::BLACK).size().x
}

/// A rough measure (for tests): half the font size per character.
#[cfg(test)]
pub fn rough_measure(t: &str, f: &FontId) -> f32 {
    t.chars().count() as f32 * f.size * 0.52
}

/// One line of the chart as placed on a page.
#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    /// A section's name above its lines.
    Heading { index: usize, text: String, color: Option<Rgb> },
    /// "Repeat Chorus" for a section printed earlier.
    Repeat { index: usize, text: String, color: Option<Rgb> },
    /// Chord names, each at its distance from the line's start.
    Chords { index: usize, chords: Vec<(f32, String)>, band: f32 },
    Lyric { index: usize, text: String, band: f32 },
    Bars { index: usize, text: String, band: f32 },
    /// A row of chord diagrams.
    Diagrams { names: Vec<String> },
}

impl Item {
    /// The section the line belongs to.
    pub fn section(&self) -> Option<usize> {
        match self {
            Item::Heading { index, .. } | Item::Repeat { index, .. } | Item::Chords { index, .. } | Item::Lyric { index, .. } | Item::Bars { index, .. } => Some(*index),
            Item::Diagrams { .. } => None,
        }
    }
}

/// A line placed on a page: where (page points), in which column, how tall.
#[derive(Clone, Debug)]
pub struct Placed {
    pub item: Item,
    pub pos: Pos2,
    pub col: usize,
    pub h: f32,
}

pub type Page = Vec<Placed>;

const GAP_H: f32 = 22.0;
const DIAGRAM_H: f32 = 92.0;
const DIAGRAM_CELL: f32 = 46.0;

/// Where the header's parts sit on the first page.
struct Header {
    artist_y: f32,
    /// The rule under the title.
    rule1: f32,
    map_y: f32,
    map_line: f32,
}

fn header(chart: &ChordChart) -> Header {
    let l = &chart.look;
    let artist_y = 44.0 + l.title.size * 1.27;
    let left = artist_y + l.info.size * 1.3;
    let right = 44.0 + l.info.size * 1.33 * 3.0;
    let rule1 = left.max(right) + 6.0;
    Header { artist_y, rule1, map_y: rule1 + 8.0, map_line: l.map.head() }
}

/// The map's words: (x, row, text, colour behind, followed by a comma).
fn map_words(chart: &ChordChart, m: Measure) -> (f32, Vec<(f32, usize, String, Option<Rgb>, bool)>) {
    let f = chart.look.map.font(1.0);
    let width = PAGE_W - 2.0 * MARGIN;
    let label_w = m("Map:", &f) + 5.0;
    let comma = m(", ", &f);
    let mut x = MARGIN + label_w;
    let mut row = 0;
    let mut out = vec![];
    let n = chart.map.len();
    for (k, e) in chart.map.iter().enumerate() {
        let Some(s) = chart.section(e.section) else { continue };
        let mut t = s.short();
        if e.repeat > 1 {
            t = format!("{t} (x{})", e.repeat);
        }
        let w = m(&t, &f);
        if x + w > MARGIN + width && x > MARGIN + label_w + 1.0 {
            x = MARGIN + 30.0;
            row += 1;
        }
        out.push((x, row, t, s.color, k + 1 < n));
        x += w + if k + 1 < n { comma } else { 0.0 };
    }
    (label_w, out)
}

fn header_end(chart: &ChordChart, m: Measure) -> f32 {
    let h = header(chart);
    if !chart.look.show_map {
        return h.rule1 + 22.0;
    }
    let rows = map_words(chart, m).1.last().map(|w| w.1 + 1).unwrap_or(1);
    h.map_y + rows as f32 * h.map_line + 4.0 + 22.0
}

pub fn column_width(chart: &ChordChart) -> f32 {
    let cols = chart.look.columns.clamp(1, MAX_COLUMNS) as f32;
    (PAGE_W - 2.0 * MARGIN - COL_GAP * (cols - 1.0)) / cols
}

/// Splits a lyric line (with its chords) into pieces that fit the column,
/// breaking between words as a text editor does.
fn wrap_lyric(text: &str, chords: &[(usize, String)], width: f32, lf: &FontId, m: Measure) -> Vec<(String, Vec<(usize, String)>)> {
    let mut out = vec![];
    let mut chars: Vec<char> = text.chars().collect();
    let mut chords: Vec<(usize, String)> = chords.to_vec();
    loop {
        let whole: String = chars.iter().collect();
        if chars.is_empty() || m(&whole, lf) <= width {
            out.push((whole, chords));
            break;
        }
        // The last space where the words before it still fit.
        let mut cut = None;
        for i in 1..chars.len() {
            if chars[i] == ' ' {
                let head: String = chars[..i].iter().collect();
                if m(&head, lf) <= width {
                    cut = Some(i);
                } else {
                    break;
                }
            }
        }
        // A single word longer than the column: break it where it fills it.
        let cut = cut.unwrap_or_else(|| {
            let mut i = 1;
            while i < chars.len() && m(&chars[..i + 1].iter().collect::<String>(), lf) <= width {
                i += 1;
            }
            i
        });
        let mut skip = cut;
        while skip < chars.len() && chars[skip] == ' ' {
            skip += 1;
        }
        let head: String = chars[..cut].iter().collect();
        let (now, later): (Vec<_>, Vec<_>) = chords.into_iter().partition(|c| c.0 < cut);
        out.push((head, now));
        chords = later.into_iter().map(|(i, n)| (i.saturating_sub(skip), n)).collect();
        chars = chars[skip..].to_vec();
    }
    out
}

/// Chord positions over a lyric: above their syllable, never overlapping.
/// Returns rows (a line of only chords too wide for the column becomes several).
fn place_chords(text: &str, chords: &[(usize, String)], width: f32, look: &ChartLook, m: Measure) -> Vec<(Vec<(f32, String)>, f32)> {
    let lf = look.lyrics.font(1.0);
    let cf = look.chords.font(1.0);
    let gap = look.chords.size * 0.55;
    let mut rows = vec![];
    let mut row: Vec<(f32, String)> = vec![];
    let mut end = 0.0f32;
    for (ci, name) in chords {
        let prefix: String = text.chars().take(*ci).collect();
        let w = m(name, &cf);
        let mut x = m(&prefix, &lf).max(if row.is_empty() { 0.0 } else { end + gap });
        if text.trim().is_empty() && !row.is_empty() && x + w > width {
            rows.push((std::mem::take(&mut row), end));
            x = 0.0;
        }
        row.push((x, name.clone()));
        end = x + w;
    }
    rows.push((row, end));
    rows
}

/// A line of bars, split at bar lines to fit the column.
fn wrap_bars(t: &str, width: f32, f: &FontId, m: Measure) -> Vec<String> {
    if m(t, f) <= width {
        return vec![t.to_string()];
    }
    let cells: Vec<&str> = t.split('|').map(|c| c.trim()).filter(|c| !c.is_empty()).collect();
    let make = |c: &[&str]| format!("| {} |", c.join(" | "));
    let mut out = vec![];
    let mut start = 0;
    while start < cells.len() {
        let mut end = start + 1;
        while end < cells.len() && m(&make(&cells[start..end + 1]), f) <= width {
            end += 1;
        }
        out.push(make(&cells[start..end]));
        start = end;
    }
    out
}

/// A group of lines kept together (a chord line and its words, or a heading
/// with the line after it).
struct Unit {
    items: Vec<(Item, f32)>,
    gap_before: f32,
    keep_with_next: bool,
    new_column: bool,
}

impl Unit {
    fn h(&self) -> f32 {
        self.items.iter().map(|i| i.1).sum()
    }
}

fn units(chart: &ChordChart, m: Measure) -> Vec<Unit> {
    let look = &chart.look;
    let width = column_width(chart);
    let lf = look.lyrics.font(1.0);
    let bf = look.bars.font(1.0);
    let mut out: Vec<Unit> = vec![];
    let mut printed: Vec<u32> = vec![];
    let mut last_repeat = false;
    let unit = |items: Vec<(Item, f32)>| Unit { items, gap_before: 0.0, keep_with_next: false, new_column: false };
    for (entry, e) in chart.map.iter().enumerate() {
        let Some(index) = chart.sections.iter().position(|s| s.id == e.section) else { continue };
        let s = &chart.sections[index];
        // "(x2)" and any tempo change from here.
        let times = format!("{}{}", if e.repeat > 1 { format!(" (x{})", e.repeat) } else { String::new() }, chart.tempo_note(entry));
        let label = e.label.trim();
        let gap = if out.is_empty() { 0.0 } else { GAP_H };
        if printed.contains(&e.section) {
            let text = if label.is_empty() { format!("Repeat {}{times}", s.name) } else { format!("{label}{times}") };
            let mut u = unit(vec![(Item::Repeat { index, text, color: e.color }, look.repeat.head() + 2.0)]);
            u.gap_before = if last_repeat { 0.0 } else { gap };
            out.push(u);
            last_repeat = true;
            continue;
        }
        printed.push(e.section);
        last_repeat = false;
        let text = if label.is_empty() { format!("{}{times}", s.name) } else { format!("{label}{times}") };
        let mut head = unit(vec![(Item::Heading { index, text, color: e.color }, look.heading.head())]);
        head.gap_before = gap;
        head.keep_with_next = true;
        head.new_column = s.new_column;
        out.push(head);
        for line in parse(&s.text) {
            match line {
                Line::Bars(t) => {
                    for piece in wrap_bars(&t, width, &bf, m) {
                        let band = (m(&piece, &bf) + 2.0).min(width);
                        out.push(unit(vec![(Item::Bars { index, text: piece, band }, look.bars.row())]));
                    }
                }
                Line::Lyric { text, chords } => {
                    for (piece, ch) in wrap_lyric(&text, &chords, width, &lf, m) {
                        let lyric_w = m(&piece, &lf);
                        let mut items = vec![];
                        if !ch.is_empty() {
                            for (row, end) in place_chords(&piece, &ch, width, look, m) {
                                let band = (lyric_w.max(end) + 2.0).min(width);
                                items.push((Item::Chords { index, chords: row, band }, look.chords.row()));
                            }
                        }
                        if !piece.trim().is_empty() || ch.is_empty() {
                            let band = if piece.trim().is_empty() { 0.0 } else { (lyric_w + 2.0).min(width) };
                            items.push((Item::Lyric { index, text: piece, band }, look.lyrics.row()));
                        }
                        out.push(unit(items));
                    }
                }
            }
        }
    }
    if chart.look.show_diagrams && !chart.diagrams.is_empty() {
        let per_row = (((width - 10.0) / DIAGRAM_CELL) as usize).max(1);
        for (k, names) in chart.diagrams.chunks(per_row).enumerate() {
            let mut u = unit(vec![(Item::Diagrams { names: names.to_vec() }, DIAGRAM_H)]);
            u.gap_before = if k == 0 { GAP_H } else { 0.0 };
            out.push(u);
        }
    }
    out
}

/// The chart's pages. Lines flow down a column, then into the next column,
/// then onto the next page, as in a text editor; long lines wrap to the
/// column's width.
pub fn layout(chart: &ChordChart, m: Measure) -> Vec<Page> {
    let cols = chart.look.columns.clamp(1, MAX_COLUMNS) as usize;
    let col_w = column_width(chart);
    let first_top = header_end(chart, m);
    let top = |page: usize| if page == 0 { first_top } else { TOP_OTHER };
    let mut pages: Vec<Page> = vec![vec![]];
    let (mut col, mut y) = (0usize, first_top);
    let list = units(chart, m);
    for (k, u) in list.iter().enumerate() {
        let at_top = |y: f32, page: usize| y <= top(page) + 0.5;
        let next_col = |pages: &mut Vec<Page>, col: &mut usize, y: &mut f32| {
            if *col + 1 < cols {
                *col += 1;
            } else {
                pages.push(vec![]);
                *col = 0;
            }
            *y = top(pages.len() - 1);
        };
        if u.new_column && !at_top(y, pages.len() - 1) {
            next_col(&mut pages, &mut col, &mut y);
        }
        let mut gap = if at_top(y, pages.len() - 1) { 0.0 } else { u.gap_before };
        let need = u.h() + if u.keep_with_next { list.get(k + 1).map(|n| n.h()).unwrap_or(0.0) } else { 0.0 };
        if y + gap + need > BOTTOM && !at_top(y, pages.len() - 1) {
            next_col(&mut pages, &mut col, &mut y);
            gap = 0.0;
        }
        y += gap;
        let x = MARGIN + col as f32 * (col_w + COL_GAP);
        for (item, h) in &u.items {
            pages.last_mut().unwrap().push(Placed { item: item.clone(), pos: Pos2::new(x, y), col, h: *h });
            y += h;
        }
    }
    pages
}

const INK: Color32 = Color32::from_rgb(20, 20, 24);
const PURPLE: Color32 = Color32::from_rgb(153, 0, 255);

/// Draws page `page` at `origin`, `z` screen points per page point. With
/// `selected`, that section gets a light frame (on screen only). Returns the
/// screen rectangle of each line with its section, for clicks.
pub fn draw_page(p: &Painter, chart: &ChordChart, pages: &[Page], page: usize, origin: Pos2, z: f32, selected: Option<usize>, printing: bool) -> Vec<(usize, Rect)> {
    let at = |x: f32, y: f32| Pos2::new(origin.x + x * z, origin.y + y * z);
    let page_rect = Rect::from_min_size(origin, Vec2::new(PAGE_W * z, PAGE_H * z));
    if !printing {
        p.rect_filled(page_rect.translate(Vec2::new(2.0, 3.0)), 1.0, Color32::from_black_alpha(30));
    }
    p.rect_filled(page_rect, 0.0, Color32::WHITE);
    let look = &chart.look;
    let rule = Stroke::new((1.0 * z).max(0.6), INK);
    let width = PAGE_W - 2.0 * MARGIN;
    let measure = painter_measure(p);
    // Widths at zoom 1 from fonts at zoom 1.
    let m = |t: &str, f: &FontId| measure(t, f);
    if page == 0 {
        let h = header(chart);
        let info = look.info.font(z);
        let small = look.info.font(z * 0.86);
        let ink = look.info.ink();
        let step = look.info.size * 1.33;
        p.text(at(MARGIN, 44.0), Align2::LEFT_TOP, &chart.title, look.title.font(z), look.title.ink());
        p.text(at(MARGIN, h.artist_y), Align2::LEFT_TOP, &chart.artist, info.clone(), ink);
        let mx = MARGIN + width * 0.5;
        p.text(at(mx, 44.0), Align2::LEFT_TOP, format!("Capo: {}", if chart.capo.trim().is_empty() { "None" } else { chart.capo.trim() }), info.clone(), ink);
        if !chart.tuning.trim().is_empty() {
            p.text(at(mx, 44.0 + step * 2.2), Align2::LEFT_TOP, format!("(Tuned: {})", chart.tuning.trim()), small, ink);
        }
        let rx = MARGIN + width * 0.78;
        p.text(at(rx, 44.0), Align2::LEFT_TOP, format!("BPM: {}", chart.bpm), info.clone(), ink);
        p.text(at(rx, 44.0 + step), Align2::LEFT_TOP, format!("Time: {}", chart.time), info.clone(), ink);
        if !chart.key.trim().is_empty() {
            p.text(at(rx, 44.0 + step * 2.0), Align2::LEFT_TOP, format!("Key: {}", chart.key.trim()), info, ink);
        }
        p.line_segment([at(MARGIN - 6.0, h.rule1), at(MARGIN + width + 6.0, h.rule1)], rule);
        // The map: every section in playing order, in its colour.
        let mf = look.map.font(z);
        let words = if look.show_map { map_words(chart, &m).1 } else { vec![] };
        let mf1 = look.map.font(1.0);
        let mink = look.map.ink();
        if look.show_map {
            p.text(at(MARGIN, h.map_y), Align2::LEFT_TOP, "Map:", mf.clone(), mink);
        }
        let mut last_row = 0;
        for (x, row, t, c, comma) in words {
            let y = h.map_y + row as f32 * h.map_line;
            let w = m(&t, &mf1);
            if let Some(c) = c {
                p.rect_filled(Rect::from_min_size(at(x - 1.0, y - 1.0), Vec2::new((w + 2.0) * z, (look.map.size * 1.43) * z)), 0.0, color(c));
            }
            p.text(at(x, y), Align2::LEFT_TOP, &t, mf.clone(), mink);
            if comma {
                p.text(at(x + w, y), Align2::LEFT_TOP, ", ", mf.clone(), mink);
            }
            last_row = row;
        }
        if look.show_map {
            let y = h.map_y + last_row as f32 * h.map_line;
            p.line_segment([at(MARGIN - 6.0, y + h.map_line + 4.0), at(MARGIN + width + 6.0, y + h.map_line + 4.0)], rule);
        }
    }
    p.line_segment([at(MARGIN - 6.0, BOTTOM + 8.0), at(MARGIN + width + 6.0, BOTTOM + 8.0)], rule);
    if !chart.copyright.trim().is_empty() {
        p.text(at(PAGE_W * 0.5, BOTTOM + 14.0), Align2::CENTER_TOP, chart.copyright.trim(), look.copyright.font(z), look.copyright.ink());
    }
    let col_w = column_width(chart);
    let mut hits = vec![];
    let Some(items) = pages.get(page) else { return hits };
    // The selected section: one frame around its lines in each column.
    if let Some(sel) = selected.filter(|_| !printing) {
        let mut frames: Vec<(usize, Rect)> = vec![];
        for pl in items.iter().filter(|pl| pl.item.section() == Some(sel)) {
            let r = Rect::from_min_size(at(pl.pos.x, pl.pos.y), Vec2::new(col_w * z, pl.h * z));
            match frames.iter_mut().find(|f| f.0 == pl.col) {
                Some(f) => f.1 = f.1.union(r),
                None => frames.push((pl.col, r)),
            }
        }
        for (_, r) in frames {
            p.rect_filled(r.expand(4.0 * z), 3.0, Color32::from_rgba_unmultiplied(80, 140, 240, 28));
            p.rect_stroke(r.expand(4.0 * z), 3.0, Stroke::new(1.2, Color32::from_rgb(80, 140, 240)), egui::StrokeKind::Outside);
        }
    }
    let tint_of = |index: usize| if look.shade { chart.sections.get(index).and_then(|s| s.color).map(color) } else { None };
    let band = |index: usize, pos: Pos2, w: f32, h: f32| {
        if let Some(c) = tint_of(index)
            && w > 0.0
        {
            p.rect_filled(Rect::from_min_max(at(pos.x, pos.y - 0.5), at(pos.x + w, pos.y + h - 1.0)), 0.0, c);
        }
    };
    for pl in items {
        let pos = pl.pos;
        if let Some(i) = pl.item.section() {
            hits.push((i, Rect::from_min_size(at(pos.x, pos.y), Vec2::new(col_w * z, pl.h * z))));
        }
        match &pl.item {
            Item::Heading { text, color: c, .. } => {
                p.text(at(pos.x, pos.y), Align2::LEFT_TOP, text, look.heading.font(z), c.map(color).unwrap_or(look.heading.ink()));
            }
            Item::Repeat { text, color: c, .. } => {
                p.text(at(pos.x, pos.y), Align2::LEFT_TOP, text, look.repeat.font(z), c.map(color).unwrap_or(look.repeat.ink()));
            }
            Item::Chords { index, chords, band: w } => {
                band(*index, pos, *w, pl.h);
                for (x, name) in chords {
                    p.text(at(pos.x + x, pos.y), Align2::LEFT_TOP, name, look.chords.font(z), look.chords.ink());
                }
            }
            Item::Lyric { index, text, band: w } => {
                band(*index, pos, *w, pl.h);
                p.text(at(pos.x, pos.y), Align2::LEFT_TOP, text, look.lyrics.font(z), look.lyrics.ink());
            }
            Item::Bars { index, text, band: w } => {
                band(*index, pos, *w, pl.h);
                p.text(at(pos.x, pos.y), Align2::LEFT_TOP, text, look.bars.font(z), look.bars.ink());
            }
            Item::Diagrams { names } => draw_diagrams(p, names, pos, origin, z),
        }
    }
    hits
}

pub fn draw_diagrams(p: &Painter, names: &[String], pos: Pos2, origin: Pos2, z: f32) {
    let at = |x: f32, y: f32| Pos2::new(origin.x + x * z, origin.y + y * z);
    let cell = DIAGRAM_CELL;
    let n = names.len().max(1);
    let box_r = Rect::from_min_size(at(pos.x, pos.y), Vec2::new((n as f32 * cell + 10.0) * z, (DIAGRAM_H - 12.0) * z));
    p.rect_filled(box_r, 2.0 * z, Color32::from_gray(246));
    let line = Stroke::new((0.8 * z).max(0.5), INK);
    for (k, name) in names.iter().take(n).enumerate() {
        let cx = pos.x + 5.0 + cell * (k as f32 + 0.5);
        p.text(at(cx, pos.y + 4.0), Align2::CENTER_TOP, name, FontId::new(10.0 * z, FontFamily::Name("Times New Roman|1".into())), INK);
        let gap = 5.6;
        let fh = 8.0;
        let left = cx - gap * 2.5;
        let top = pos.y + 26.0;
        let frets = shape(name);
        let base = frets.map(|f| {
            let played: Vec<i8> = f.iter().copied().filter(|&v| v > 0).collect();
            let max = played.iter().copied().max().unwrap_or(0);
            if max <= 4 { 1 } else { played.iter().copied().min().unwrap_or(1) }
        });
        for s in 0..6 {
            let x = left + s as f32 * gap;
            p.line_segment([at(x, top), at(x, top + 4.0 * fh)], line);
        }
        for f in 0..=4 {
            let y = top + f as f32 * fh;
            let st = if f == 0 && base == Some(1) { Stroke::new(2.0 * z, INK) } else { line };
            p.line_segment([at(left, y), at(left + 5.0 * gap, y)], st);
        }
        let Some(frets) = frets else { continue };
        let base = base.unwrap_or(1);
        if base > 1 {
            p.text(at(left + 5.0 * gap + 2.0, top + fh * 0.5), Align2::LEFT_CENTER, format!("{base}"), FontId::new(6.5 * z, FontFamily::Name("Arial|0".into())), INK);
        }
        for (s, &f) in frets.iter().enumerate() {
            let x = left + s as f32 * gap;
            match f {
                -1 => {
                    let c = at(x, top - 4.0);
                    let d = 1.8 * z;
                    p.line_segment([c + Vec2::new(-d, -d), c + Vec2::new(d, d)], line);
                    p.line_segment([c + Vec2::new(-d, d), c + Vec2::new(d, -d)], line);
                }
                0 => {
                    p.circle_stroke(at(x, top - 4.0), 1.8 * z, line);
                }
                f => {
                    let row = f as i32 - base as i32;
                    if (0..4).contains(&row) {
                        p.circle_filled(at(x, top + (row as f32 + 0.5) * fh), 2.4 * z, INK);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_chords_are_read() {
        let lines = parse("[E]Let my heart be a [B]temple\n| E /// | B //// |\n[C#m]");
        assert_eq!(lines[0], Line::Lyric { text: "Let my heart be a temple".into(), chords: vec![(0, "E".into()), (18, "B".into())] });
        assert_eq!(lines[1], Line::Bars("| E /// | B //// |".into()));
        assert_eq!(lines[2], Line::Lyric { text: "".into(), chords: vec![(0, "C#m".into())] });
    }

    #[test]
    fn map_repeats_print_once() {
        let c = ChordChart::sample();
        let pages = layout(&c, &rough_measure);
        let blocks: Vec<&Item> = pages.iter().flatten().map(|p| &p.item).collect();
        let full = blocks.iter().filter(|b| matches!(b, Item::Heading { .. })).count();
        let repeats = blocks.iter().filter(|b| matches!(b, Item::Repeat { .. })).count();
        assert_eq!(full, c.sections.len());
        assert_eq!(full + repeats, c.map.len());
        assert!(blocks.iter().any(|b| matches!(b, Item::Diagrams { .. })));
    }

    #[test]
    fn first_format_gets_kind_colours_and_new_one_keeps_choices() {
        let p = std::env::temp_dir().join(format!("guitarcraft-chart-v1-{}.chart", std::process::id()));
        std::fs::write(&p, r#"{"format":"GuitarCraft chord chart 1","title":"Old","sections":[{"id":1,"name":"Verse 1","kind":"Verse","text":"[E]x"},{"id":2,"name":"Chorus","kind":"Chorus","text":"y"}],"map":[{"section":1,"repeat":1}]}"#).unwrap();
        let c = ChordChart::load(&p).unwrap();
        assert_eq!(c.sections[0].color, Kind::Verse.tint().map(rgb));
        assert_eq!(c.sections[1].color, None);
        assert_eq!(c.look, ChartLook::standard());
        // A chart of the new format keeps a section without colour, and its look.
        let mut d = ChordChart::sample();
        d.sections[1].color = None;
        d.look = ChartLook::stage();
        d.map[6].label = "Chorus again".into();
        d.map[6].color = Some([0, 0, 255]);
        d.save(&p).unwrap();
        assert_eq!(ChordChart::load(&p).unwrap(), d);
        let _ = std::fs::remove_file(&p);
        // Bigger type makes a taller chart.
        let tall = |look: ChartLook| {
            let mut c = ChordChart::sample();
            c.look.columns = 1;
            c.look = look;
            layout(&c, &rough_measure).len()
        };
        assert!(tall(ChartLook::stage()) >= tall(ChartLook::standard()));
    }

    #[test]
    fn lines_flow_into_columns_then_pages() {
        // A long verse in one column flows on to the next page, line by line.
        let mut c = ChordChart::sample();
        let long: Vec<String> = (0..120).map(|k| format!("[E]Line number {k} of a very long verse")).collect();
        c.sections[1].text = long.join("
");
        c.look.columns = 1;
        let one = layout(&c, &rough_measure);
        assert!(one.len() >= 3, "{} pages", one.len());
        // The verse is split: its lines are on several pages.
        let pages_with_v1 = one.iter().filter(|p| p.iter().any(|l| l.item.section() == Some(1))).count();
        assert!(pages_with_v1 >= 2);
        // Every line stays inside the page.
        for p in &one {
            for l in p {
                assert!(l.pos.y + l.h <= BOTTOM + 0.5);
            }
        }
        // Three columns: a page is filled column by column before the next page.
        c.look.columns = 3;
        let three = layout(&c, &rough_measure);
        assert!(three.len() < one.len());
        let cols: Vec<usize> = three[0].iter().map(|l| l.col).collect();
        assert!(cols.windows(2).all(|w| w[0] <= w[1]));
        assert_eq!(*cols.last().unwrap(), 2);
    }

    #[test]
    fn long_lines_wrap_with_their_chords() {
        let f = FontId::new(10.0, FontFamily::Proportional);
        let pieces = wrap_lyric("one two three four five six", &[(0, "E".into()), (14, "B".into())], 60.0, &f, &rough_measure);
        assert!(pieces.len() >= 2);
        assert!(pieces.iter().all(|p| rough_measure(&p.0, &f) <= 60.0));
        let joined: Vec<String> = pieces.iter().map(|p| p.0.clone()).collect();
        assert_eq!(joined.join(" "), "one two three four five six");
        // "B" was over "four": it moves with that word.
        let with_b = pieces.iter().find(|p| p.1.iter().any(|c| c.1 == "B")).unwrap();
        let at = with_b.1.iter().find(|c| c.1 == "B").unwrap().0;
        assert!(with_b.0[at..].starts_with("four"));
        let bars = wrap_bars("| E /// | B //// | C#m /// | A /// | E /// |", 60.0, &f, &rough_measure);
        assert!(bars.len() >= 2 && bars.iter().all(|b| b.starts_with('|') && b.ends_with('|')));
    }

    #[test]
    fn diagrams_are_optional_and_copyright_kept() {
        let mut c = ChordChart::sample();
        c.look.show_diagrams = false;
        assert!(!layout(&c, &rough_measure).iter().flatten().any(|l| matches!(l.item, Item::Diagrams { .. })));
        let mut b = ChordChart::sample();
        b.sections[2].new_column = true;
        let lay = layout(&b, &rough_measure);
        let head = lay[0].iter().find(|l| matches!(&l.item, Item::Heading { index: 2, .. })).unwrap();
        assert_eq!(head.col, 1);
        assert!(!ChordChart::sample().copyright.is_empty());
    }

    #[test]
    fn page_settings_from_before_the_stylesheet_are_kept() {
        let p = std::env::temp_dir().join(format!("guitarcraft-chart-cols-{}.chart", std::process::id()));
        std::fs::write(&p, r#"{"format":"GuitarCraft chord chart 2","title":"T","columns":3,"show_diagrams":false,"look":{"shade":true},"sections":[],"map":[]}"#).unwrap();
        let c = ChordChart::load(&p).unwrap();
        let _ = std::fs::remove_file(&p);
        assert_eq!(c.look.columns, 3);
        assert!(!c.look.show_diagrams);
        assert_eq!(c.look.kind_colors.len(), Kind::ALL.len());
        assert_eq!(c.look.kind_color(Kind::Verse), Kind::Verse.tint().map(rgb));
    }

    #[test]
    fn chords_transpose() {
        assert_eq!(transpose_chord("E", 2, false), "F#");
        assert_eq!(transpose_chord("C#m7", 1, false), "Dm7");
        assert_eq!(transpose_chord("D/F#", -2, false), "C/E");
        assert_eq!(transpose_chord("Bb", 2, true), "C");
        assert_eq!(transpose_chord("A", 1, true), "Bb");
        assert_eq!(transpose_chord("Asus4", 12, false), "Asus4");
        assert_eq!(transpose_chord("N.C.", 3, false), "N.C.");
        assert_eq!(transpose_text("[E]Out along the [B]river", 2, false), "[F#]Out along the [C#]river");
        assert_eq!(transpose_text("| E /// | C#m //// |", -1, true), "| Eb /// | Cm //// |");
        // Words that look like notes are left alone in lyrics.
        assert_eq!(transpose_text("A [G]day", 2, false), "A [A]day");
        let mut c = ChordChart::sample();
        let before = c.clone();
        c.transpose(1, Spelling::Auto, None);
        // E major up a half step is F major: written with flats.
        assert_eq!(c.key, "F");
        let mut up2 = ChordChart::sample();
        up2.transpose(2, Spelling::Auto, None);
        assert_eq!(up2.key, "F#");
        assert!(c.sections[0].text.starts_with("| F /// | C ///"), "{}", c.sections[0].text);
        assert_eq!(c.diagrams[0], "F5");
        c.transpose(-1, Spelling::Sharps, None);
        assert_eq!(c.sections, before.sections);
        assert_eq!(c.key, before.key);
        // One section only: the key and the others stay.
        c.transpose(2, Spelling::Auto, Some(1));
        assert_eq!(c.key, "E");
        assert_eq!(c.sections[0], before.sections[0]);
        assert_ne!(c.sections[1], before.sections[1]);
        // Every transposed chord has a diagram.
        for k in 0..12 {
            for q in ["", "m", "7", "m7", "5", "sus4", "maj7"] {
                let n = format!("{}{q}", note_name(k, k % 2 == 1));
                assert!(shape(&n).is_some(), "{n}");
            }
        }
    }

    #[test]
    fn moving_sections_changes_the_map() {
        let mut c = ChordChart::sample();
        c.sync_sections();
        let names = |c: &ChordChart| c.sections.iter().map(|s| s.name.clone()).collect::<Vec<_>>();
        let map = |c: &ChordChart| c.map.iter().map(|e| c.section(e.section).unwrap().short()).collect::<Vec<_>>().join(",");
        assert_eq!(map(&c), "Intro,V1,V2,Chorus,Interlude,V3,V4,Chorus,V5,Interlude 2,Bridge,Chorus,V5");
        // Verse 2 one place earlier: it is played before Verse 1 now; repeats stay where they were.
        let v2 = c.sections.iter().position(|s| s.name == "Verse 2").unwrap();
        let at = c.move_section(v2, -1);
        assert_eq!(c.sections[at].name, "Verse 2");
        assert_eq!(map(&c), "Intro,V2,V1,Chorus,Interlude,V3,V4,Chorus,V5,Interlude 2,Bridge,Chorus,V5");
        assert_eq!(&names(&c)[..3], &["Intro", "Verse 2", "Verse 1"]);
        // The printed chart follows: Verse 2's heading comes before Verse 1's.
        let lay = layout(&c, &rough_measure);
        let heads: Vec<String> = lay.iter().flatten().filter_map(|l| match &l.item { Item::Heading { text, .. } => Some(text.clone()), _ => None }).collect();
        assert_eq!(&heads[..3], &["Intro", "Verse 2", "Verse 1"]);
        // Chorus later: it changes places with Interlude.
        let ch = c.sections.iter().position(|s| s.name == "Chorus").unwrap();
        c.move_section(ch, 1);
        assert_eq!(map(&c), "Intro,V2,V1,Interlude,Chorus,V3,V4,Chorus,V5,Interlude 2,Bridge,Chorus,V5");
        // Edges don't move; a section not in the map stays among those.
        assert_eq!(c.move_section(0, -1), 0);
        c.sections.push(Section { id: 99, name: "Spare".into(), ..Default::default() });
        let last = c.sections.len() - 1;
        assert!(!c.can_move_section(last, -1));
        // Reordering the map reorders the list.
        c.map.swap(0, 1);
        c.sync_sections();
        assert_eq!(&names(&c)[..2], &["Verse 2", "Intro"]);
        assert_eq!(c.sections.last().unwrap().name, "Spare");
    }

    #[test]
    fn stray_brackets_are_tidied() {
        let mut c = ChordChart::new_blank();
        // Text with stray brackets shows and transposes cleanly.
        assert_eq!(parse("| [E] /// |"), vec![Line::Bars("| E /// |".into())]);
        assert_eq!(parse("[[E]]Out"), vec![Line::Lyric { text: "Out".into(), chords: vec![(0, "E".into())] }]);
        assert_eq!(transpose_text("| [E] /// |", 2, false), "| [F#] /// |");
        c.sections[0].text = "| [E] /// [[B]] |".into();
        assert_eq!(c.chords_used()[..2], ["E".to_string(), "B".to_string()]);
    }

    #[test]
    fn diagrams_are_chosen_from_the_song() {
        let mut c = ChordChart::sample();
        c.diagrams.clear();
        let all: Vec<String> = c.diagram_choices().into_iter().map(|d| d.0).collect();
        assert_eq!(all, c.chords_used());
        // Chosen out of order, they print in the order the song plays them.
        c.set_diagram("C#m", true);
        c.set_diagram("E", true);
        assert_eq!(c.diagrams, ["E", "C#m"]);
        c.set_diagram("E", false);
        assert_eq!(c.diagrams, ["C#m"]);
        // A chord the song no longer uses stays offered (to take it off).
        c.diagrams.push("G".into());
        let choices = c.diagram_choices();
        assert_eq!(choices.last().unwrap(), &("G".to_string(), false));
        c.set_diagram("G", false);
        assert!(!c.diagram_choices().iter().any(|d| d.0 == "G"));
    }

    #[test]
    fn tempo_changes_sit_on_map_entries() {
        let mut c = ChordChart::sample();
        assert_eq!(c.tempo_points(), vec![(0, 63, false)]);
        c.set_tempo_points(&[(0, 63, false), (10, 72, true), (12, 80, false)]);
        assert_eq!(c.map[10].tempo, Some(72));
        assert!(c.map[10].tempo_ramp);
        assert_eq!(c.tempo_points(), vec![(0, 63, false), (10, 72, true), (12, 80, false)]);
        assert_eq!(c.tempo_note(10), " \u{2014} 72 \u{2192} 80 BPM");
        assert_eq!(c.tempo_note(12), " \u{2014} 80 BPM");
        assert_eq!(c.tempo_note(0), "");
        // The heading of the Bridge (entry 10) says so on the page.
        let lay = layout(&c, &rough_measure);
        assert!(lay.iter().flatten().any(|l| matches!(&l.item, Item::Heading { text, .. } if text == "Bridge (x2) \u{2014} 72 \u{2192} 80 BPM")));
        // Taking a point away clears it.
        c.set_tempo_points(&[(0, 70, false)]);
        assert_eq!(c.bpm, 70);
        assert!(c.map.iter().all(|e| e.tempo.is_none()));
    }

    #[test]
    fn diagrams_start_off() {
        // New charts and both ready-made styles print no diagrams until asked.
        assert!(!ChordChart::new_blank().look.show_diagrams);
        assert!(!ChartLook::standard().show_diagrams && !ChartLook::stage().show_diagrams);
        let blank = ChordChart::new_blank();
        assert!(!layout(&blank, &rough_measure).iter().flatten().any(|l| matches!(l.item, Item::Diagrams { .. })));
        // The sample, like the person's picture, has them on.
        assert!(ChordChart::sample().look.show_diagrams);
    }

    #[test]
    fn saves_and_reads_back() {
        let c = ChordChart::sample();
        let p = std::env::temp_dir().join(format!("guitarcraft-chart-{}.chart", std::process::id()));
        c.save(&p).unwrap();
        assert_eq!(ChordChart::load(&p).unwrap(), c);
        let _ = std::fs::remove_file(&p);
        assert_eq!(c.sections[1].short(), "V1");
        assert!(c.chords_used().contains(&"C#m".to_string()));
        assert!(shape("C#m").is_some() && shape("E5").is_some());
    }
}
