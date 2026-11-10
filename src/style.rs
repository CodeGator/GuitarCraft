//! The stylesheet: how a score looks on the page and on screen (page size and
//! margins, spacing, staves, header and footer texts, fonts and notation
//! choices). Every song carries one; it is saved with the song. Ready-made
//! styles (Rock, Classic, Jazz) can be applied, and styles can be saved to and
//! loaded from files.

use egui::{FontData, FontDefinitions, FontFamily, FontId};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Page units per millimetre (an A4 page is 820 units wide).
pub const MM: f32 = 820.0 / 210.0;
/// Page units per typographic point.
pub const PT: f32 = MM * 25.4 / 72.0;
/// Staff space (mm) at which the score is drawn at its natural size.
pub const BASE_PROPORTION: f32 = 1.9;

/// Fonts offered in the font lists: the name shown and the files for regular,
/// bold, italic and bold italic. Missing files fall back to the interface font.
pub const FONTS: [(&str, [&str; 4]); 9] = [
    (
        "Times New Roman",
        ["times.ttf", "timesbd.ttf", "timesi.ttf", "timesbi.ttf"],
    ),
    (
        "Arial",
        ["arial.ttf", "arialbd.ttf", "ariali.ttf", "arialbi.ttf"],
    ),
    (
        "Segoe UI",
        [
            "segoeui.ttf",
            "segoeuib.ttf",
            "segoeuii.ttf",
            "segoeuiz.ttf",
        ],
    ),
    (
        "Georgia",
        [
            "georgia.ttf",
            "georgiab.ttf",
            "georgiai.ttf",
            "georgiaz.ttf",
        ],
    ),
    (
        "Verdana",
        [
            "verdana.ttf",
            "verdanab.ttf",
            "verdanai.ttf",
            "verdanaz.ttf",
        ],
    ),
    (
        "Courier New",
        ["cour.ttf", "courbd.ttf", "couri.ttf", "courbi.ttf"],
    ),
    (
        "Comic Sans MS",
        ["comic.ttf", "comicbd.ttf", "comici.ttf", "comicz.ttf"],
    ),
    (
        "Segoe Print",
        ["segoepr.ttf", "segoeprb.ttf", "segoepr.ttf", "segoeprb.ttf"],
    ),
    (
        "Ink Free",
        ["Inkfree.ttf", "Inkfree.ttf", "Inkfree.ttf", "Inkfree.ttf"],
    ),
];

pub const STYLE_NAMES: [&str; 4] = ["Regular", "Bold", "Italic", "Bold Italic"];

/// Adds every font of the list to egui, each style as its own family
/// ("Name|0" .. "Name|3"), falling back to `fallback` for missing glyphs.
pub fn register_fonts(
    fonts: &mut FontDefinitions,
    fallback: &[String],
    read: impl Fn(&str) -> Option<Vec<u8>>,
) {
    for (name, files) in FONTS {
        for (k, file) in files.iter().enumerate() {
            let key = format!("{name}|{k}");
            let mut fam = vec![];
            if let Some(d) = read(file) {
                fonts
                    .font_data
                    .insert(key.clone(), Arc::new(FontData::from_owned(d)));
                fam.push(key.clone());
            }
            fam.extend(fallback.iter().cloned());
            fonts.families.insert(FontFamily::Name(key.into()), fam);
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
#[serde(default)]
pub struct FontSpec {
    pub family: String,
    /// Size in points.
    pub size: f32,
    pub bold: bool,
    pub italic: bool,
}

impl Default for FontSpec {
    fn default() -> Self {
        FontSpec::new("Times New Roman", 10.0, false, false)
    }
}

impl FontSpec {
    pub fn new(family: &str, size: f32, bold: bool, italic: bool) -> FontSpec {
        FontSpec {
            family: family.into(),
            size,
            bold,
            italic,
        }
    }
    pub fn style_index(&self) -> usize {
        self.bold as usize + 2 * self.italic as usize
    }
    pub fn set_style_index(&mut self, k: usize) {
        self.bold = k & 1 != 0;
        self.italic = k & 2 != 0;
    }
    /// The egui font at a zoom (page units to screen pixels).
    pub fn id(&self, zoom: f32) -> FontId {
        let fam = if FONTS.iter().any(|f| f.0 == self.family) {
            self.family.as_str()
        } else {
            "Times New Roman"
        };
        FontId::new(
            (self.size * PT * zoom).max(1.0),
            FontFamily::Name(format!("{fam}|{}", self.style_index()).into()),
        )
    }
}

/// One line of the header or footer: shown or not, its text with tokens such as
/// %TITLE%, where it sits and its font.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
#[serde(default)]
pub struct HeaderItem {
    pub name: String,
    pub on: bool,
    pub text: String,
    /// 0 left, 1 centre, 2 right.
    pub align: u8,
    pub font: FontSpec,
}

impl Default for HeaderItem {
    fn default() -> Self {
        HeaderItem {
            name: String::new(),
            on: false,
            text: String::new(),
            align: 1,
            font: FontSpec::default(),
        }
    }
}

fn hi(name: &str, on: bool, text: &str, align: u8, font: FontSpec) -> HeaderItem {
    HeaderItem {
        name: name.into(),
        on,
        text: text.into(),
        align,
        font,
    }
}

/// A playing-technique label: its words and font.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
#[serde(default)]
#[derive(Default)]
pub struct SymbolLabel {
    pub name: String,
    pub text: String,
    pub font: FontSpec,
}

/// Symbol labels, in the order of `Stylesheet::symbols`.
pub const SYM_LET_RING: usize = 0;
pub const SYM_PALM_MUTE: usize = 1;
pub const SYM_SLIDE: usize = 2;
pub const SYM_HAMMER: usize = 3;
pub const SYM_PULL: usize = 4;
pub const SYM_SLAP: usize = 5;
pub const SYM_POP: usize = 6;
pub const SYM_SCRAPE: usize = 7;
#[allow(dead_code)] // Names a slot in the table; no drawing code reads it yet.
pub const SYM_RASG: usize = 8;
#[allow(dead_code)] // Names a slot in the table; no drawing code reads it yet.
pub const SYM_WHAMMY: usize = 9;

/// Page formats: name and size in millimetres (portrait).
pub const PAGE_FORMATS: [(&str, f32, f32); 12] = [
    ("A4", 210.0, 297.0),
    ("A3", 297.0, 420.0),
    ("A5", 148.0, 210.0),
    ("B4", 250.0, 353.0),
    ("B5", 176.0, 250.0),
    ("Letter", 215.9, 279.4),
    ("Legal", 215.9, 355.6),
    ("Tabloid", 279.4, 431.8),
    ("Executive", 184.2, 266.7),
    ("Statement", 139.7, 215.9),
    ("Folio", 210.0, 330.0),
    ("Quarto", 215.0, 275.0),
];

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
#[serde(default)]
pub struct Stylesheet {
    // ---- Page & score format
    /// Index into PAGE_FORMATS, or PAGE_FORMATS.len() for a custom size.
    pub page_format: usize,
    pub page_w: f32,
    pub page_h: f32,
    pub landscape: bool,
    pub margin_top: f32,
    pub margin_bottom: f32,
    pub margin_left: f32,
    pub margin_right: f32,
    /// Staff space in millimetres: the size of the music on the page.
    pub proportion: f32,
    /// The size of texts and chord diagrams follows the proportion.
    pub scale_fonts: bool,
    /// Extra room given to longer notes (0 = all notes equally spaced, 1 = very proportional).
    pub rhythm_spacing: f32,
    pub tuning_show: bool,
    /// 0 on top of the score, 1 before each track.
    pub tuning_pos: u8,
    /// 0 name only, 1 name and notes, 2 only strings tuned differently from standard.
    pub tuning_mode: u8,
    /// On top of the score: 0 two columns, 1 on one line.
    pub tuning_cols: u8,
    pub tuning_framed: bool,
    /// 0 Classic, 1 Jazz, 2 Rock.
    pub chord_style: u8,
    pub chords_top: bool,
    pub chords_top_size: f32,
    pub chords_top_spacing: f32,
    pub chords_in_score: bool,
    pub chords_in_score_size: f32,

    // ---- Systems & staves
    /// Staff line thickness in staff spaces.
    pub staff_line: f32,
    /// Extra space between systems, in millimetres.
    pub system_gap: f32,
    pub indent_on: bool,
    pub indent: f32,
    pub slash_lines: bool,
    pub slash_barlines: bool,
    pub extended_barlines: bool,
    pub clef_each_system: bool,
    pub time_each_system: bool,
    pub time_each_section: bool,
    pub capo_affects_std: bool,
    /// 0 no brackets, 1 join each track's notation and tablature, 2 group similar instruments.
    pub brackets: u8,
    pub separator: bool,
    pub name_single: bool,
    /// 0 first system, 1 first system of each page, 2 every system.
    pub name_single_policy: u8,
    pub name_multi: bool,
    pub name_multi_policy: u8,
    /// 0 full name, 1 short name.
    pub name_first_mode: u8,
    /// 0 vertical, 1 horizontal.
    pub name_first_orient: u8,
    pub name_other_mode: u8,

    // ---- Header & footer
    pub first_header: Vec<HeaderItem>,
    pub first_footer: Vec<HeaderItem>,
    pub other_header: Vec<HeaderItem>,
    pub other_footer: Vec<HeaderItem>,

    // ---- Texts & styles
    pub font_fret: FontSpec,
    pub font_grace: FontSpec,
    pub font_text: FontSpec,
    pub font_chord: FontSpec,
    pub font_tuning: FontSpec,
    pub font_section: FontSpec,
    pub font_tempo: FontSpec,
    pub tempo_decimal: bool,
    pub font_timer: FontSpec,
    pub font_capo: FontSpec,
    pub font_barre: FontSpec,
    pub font_track_h: FontSpec,
    pub font_track_v: FontSpec,
    pub font_lyrics: FontSpec,
    /// 0 below the staff, 1 above, 2 hidden.
    pub lyrics_pos: u8,
    pub font_bar_number: FontSpec,
    /// 0 every bar, 1 first bar of each system, 2 hidden.
    pub bar_numbers: u8,
    /// 0 Classic, 1 Jazz (handwritten look).
    pub symbol_style: u8,
    pub symbols: Vec<SymbolLabel>,
    pub lines_over_rests: bool,

    // ---- Notation
    pub rhythm_hide_with_std: bool,
    pub rhythm_extend: bool,
    pub beams_on_rests: bool,
    pub circle_half_whole: bool,
    pub horizontal_beams: bool,
    pub horizontal_beams_drums: bool,
    /// Rhythm in the tablature for voices 1-4: 0 above, 1 below, 2 hidden.
    pub rhythm_voice: [u8; 4],
    pub harmonic_string: bool,
    pub harmonic_dot: bool,
    pub common_time_c: bool,
    pub bend_real_pitch: bool,
    pub arpeggio_arrow: bool,
    pub trill_second: bool,
    pub tied_in_tab: bool,
    pub coda_space_on: bool,
    pub coda_space: f32,
    pub hopo_letters: bool,
    pub hopo_avoid: bool,
    pub hopo_outer: bool,
    pub hopo_single: bool,
    pub slide_sl: bool,
    pub color_voices: bool,
    pub hide_useless_rests: bool,
    pub dash_quarter_rest: bool,
    pub hide_dynamics: bool,
    pub slash_fret_string: bool,
    pub tuplet_brackets: bool,
    pub scale_grid: f32,
    /// Fingering marks: 0 "T1234", 1 "P1234" (left hand); 0 pimac, 1 pimax, 2 pimae, 3 timao (right hand).
    pub lh_type: u8,
    pub lh_scale: f32,
    /// In the staff: 0 above, 1 before the note, 2 below. In the tablature: 0 above, 1 below.
    pub lh_staff: u8,
    pub lh_tab: u8,
    pub rh_type: u8,
    pub rh_scale: f32,
    pub rh_staff: u8,
    pub rh_tab: u8,
}

impl Default for Stylesheet {
    fn default() -> Self {
        Stylesheet::standard()
    }
}

fn times(size: f32) -> FontSpec {
    FontSpec::new("Times New Roman", size, false, false)
}
fn times_b(size: f32) -> FontSpec {
    FontSpec::new("Times New Roman", size, true, false)
}
fn ui(size: f32) -> FontSpec {
    FontSpec::new("Segoe UI", size, false, false)
}

/// Header items of the first page, in their usual order.
fn first_header() -> Vec<HeaderItem> {
    vec![
        hi("Title", true, "%TITLE%", 1, times(25.0)),
        hi("Subtitle", true, "%SUBTITLE%", 1, times_b(12.5)),
        hi("Artist", true, "%ARTIST%", 1, times(11.0)),
        hi("Album", true, "%ALBUM%", 1, times(11.0)),
        hi("Words", true, "Words by %WORDS%", 0, times(8.8)),
        hi("Music", true, "Music by %MUSIC%", 1, times(8.8)),
        hi(
            "Words & Music",
            true,
            "Words & Music by %MUSIC%",
            1,
            times(8.8),
        ),
        hi("Transcriber", false, "Tabbed by %TABBER%", 2, times(8.8)),
    ]
}

fn footer(copyright_on: bool) -> Vec<HeaderItem> {
    vec![
        hi("Copyright", copyright_on, "%COPYRIGHT%", 1, times(8.0)),
        hi(
            "Copyright (second line)",
            copyright_on,
            "All Rights Reserved - International Copyright Secured",
            1,
            times(7.0),
        ),
        hi("Page number", true, "%PAGE%/%PAGES%", 2, times(7.3)),
    ]
}

fn other_header() -> Vec<HeaderItem> {
    let mut v = first_header();
    for it in &mut v {
        it.on = false;
    }
    // A small running title on the following pages.
    v[0].on = false;
    v[0].font = times(10.0);
    v[0].align = 0;
    v
}

fn symbols(italic_family: &str) -> Vec<SymbolLabel> {
    let s = |name: &str, text: &str, f: FontSpec| SymbolLabel {
        name: name.into(),
        text: text.into(),
        font: f,
    };
    let it = |size: f32| FontSpec::new(italic_family, size, false, true);
    vec![
        s("Let Ring", "let ring", it(7.3)),
        s("Palm Mute", "P.M.", it(6.9)),
        s("Slide", "sl.", it(6.5)),
        s(
            "Hammer-On",
            "H",
            FontSpec::new(italic_family, 6.5, false, false),
        ),
        s(
            "Pull-Off",
            "P",
            FontSpec::new(italic_family, 6.5, false, false),
        ),
        s("Slap", "S", FontSpec::new(italic_family, 7.6, true, false)),
        s("Pop", "P", FontSpec::new(italic_family, 7.6, true, false)),
        s("Pick Scrape", "P.S.", it(6.9)),
        s("Rasgueado", "rasg.", it(6.9)),
        s("Whammy Bar Indication", "w/bar", it(6.9)),
    ]
}

impl Stylesheet {
    /// The look new songs start with.
    pub fn standard() -> Stylesheet {
        Stylesheet {
            page_format: 0,
            page_w: 210.0,
            page_h: 297.0,
            landscape: false,
            margin_top: 15.0,
            margin_bottom: 10.0,
            margin_left: 10.0,
            margin_right: 10.0,
            proportion: BASE_PROPORTION,
            scale_fonts: true,
            rhythm_spacing: 0.5,
            tuning_show: true,
            tuning_pos: 0,
            tuning_mode: 0,
            tuning_cols: 0,
            tuning_framed: false,
            chord_style: 0,
            chords_top: false,
            chords_top_size: 10.0,
            chords_top_spacing: 3.0,
            chords_in_score: true,
            chords_in_score_size: 9.0,
            staff_line: 0.13,
            system_gap: 4.0,
            indent_on: false,
            indent: 0.0,
            slash_lines: true,
            slash_barlines: true,
            extended_barlines: false,
            clef_each_system: true,
            time_each_system: false,
            time_each_section: false,
            capo_affects_std: true,
            brackets: 0,
            separator: false,
            name_single: true,
            name_single_policy: 2,
            name_multi: true,
            name_multi_policy: 2,
            name_first_mode: 1,
            name_first_orient: 0,
            name_other_mode: 1,
            first_header: first_header(),
            first_footer: footer(true),
            other_header: other_header(),
            other_footer: footer(false),
            font_fret: ui(8.4),
            font_grace: ui(6.2),
            font_text: FontSpec::new("Times New Roman", 8.8, true, true),
            font_chord: times_b(9.5),
            font_tuning: times(8.8),
            font_section: times_b(10.2),
            font_tempo: times(10.2),
            tempo_decimal: false,
            font_timer: FontSpec::new("Arial", 8.0, false, false),
            font_capo: times(8.8),
            font_barre: times_b(8.0),
            font_track_h: times(8.8),
            font_track_v: times(8.0),
            font_lyrics: times(8.8),
            lyrics_pos: 0,
            font_bar_number: ui(6.5),
            bar_numbers: 0,
            symbol_style: 0,
            symbols: symbols("Segoe UI"),
            lines_over_rests: false,
            rhythm_hide_with_std: false,
            rhythm_extend: false,
            beams_on_rests: false,
            circle_half_whole: false,
            horizontal_beams: true,
            horizontal_beams_drums: true,
            rhythm_voice: [1, 1, 2, 2],
            harmonic_string: true,
            harmonic_dot: false,
            common_time_c: false,
            bend_real_pitch: true,
            arpeggio_arrow: true,
            trill_second: true,
            tied_in_tab: true,
            coda_space_on: false,
            coda_space: 6.0,
            hopo_letters: true,
            hopo_avoid: true,
            hopo_outer: true,
            hopo_single: true,
            slide_sl: false,
            color_voices: false,
            hide_useless_rests: true,
            dash_quarter_rest: false,
            hide_dynamics: false,
            slash_fret_string: false,
            tuplet_brackets: false,
            scale_grid: 1.0,
            lh_type: 0,
            lh_scale: 1.0,
            lh_staff: 1,
            lh_tab: 1,
            rh_type: 0,
            rh_scale: 1.0,
            rh_staff: 1,
            rh_tab: 0,
        }
    }

    /// Rock: sans-serif texts, bold chord names, rhythm under the tablature.
    pub fn rock() -> Stylesheet {
        let mut s = Stylesheet::standard();
        let arial = |size: f32, bold: bool| FontSpec::new("Arial", size, bold, false);
        for it in s
            .first_header
            .iter_mut()
            .chain(s.first_footer.iter_mut())
            .chain(s.other_header.iter_mut())
            .chain(s.other_footer.iter_mut())
        {
            it.font.family = "Arial".into();
        }
        s.first_header[0].font = arial(24.0, true);
        s.first_header[1].font = arial(12.0, false);
        s.font_fret = arial(7.5, false);
        s.font_grace = arial(5.5, false);
        s.font_text = FontSpec::new("Arial", 8.0, false, true);
        s.font_chord = arial(9.5, true);
        s.font_tuning = arial(8.0, false);
        s.font_section = arial(10.0, true);
        s.font_tempo = arial(9.5, true);
        s.font_capo = arial(8.0, false);
        s.font_track_h = arial(9.0, true);
        s.font_track_v = arial(7.5, false);
        s.font_lyrics = arial(8.0, false);
        s.font_bar_number = arial(6.0, false);
        s.symbols = symbols("Arial");
        s.chord_style = 2;
        s.chords_top = true;
        s.tuning_mode = 1;
        s.circle_half_whole = true;
        s.rhythm_hide_with_std = false;
        s.rhythm_extend = true;
        s.lines_over_rests = true;
        s.hopo_letters = true;
        s.slide_sl = false;
        s
    }

    /// Classic: serif texts throughout, no rhythm in the tablature when the
    /// notation is shown, bar numbers on the first bar of each system.
    pub fn classic() -> Stylesheet {
        let mut s = Stylesheet::standard();
        let t = |size: f32| FontSpec::new("Times New Roman", size, false, false);
        s.font_fret = t(8.0);
        s.font_grace = t(6.0);
        s.font_text = FontSpec::new("Times New Roman", 9.0, false, true);
        s.font_bar_number = FontSpec::new("Times New Roman", 7.0, false, true);
        s.font_track_v = t(8.0);
        s.font_track_h = t(10.0);
        s.symbols = symbols("Times New Roman");
        s.first_header[4].align = 0;
        s.first_header[5].align = 2;
        s.first_header[6].align = 2;
        s.chord_style = 0;
        s.chords_top = false;
        s.bar_numbers = 1;
        s.rhythm_hide_with_std = true;
        s.circle_half_whole = false;
        s.indent_on = true;
        s.indent = 8.0;
        s.name_first_mode = 0;
        s.name_first_orient = 1;
        s.brackets = 1;
        s.tuplet_brackets = true;
        s.hopo_letters = false;
        s.slide_sl = false;
        s
    }

    /// Jazz: a handwritten look for texts and symbols, and jazz chord diagrams.
    pub fn jazz() -> Stylesheet {
        let mut s = Stylesheet::standard();
        let hand = |size: f32, bold: bool| FontSpec::new("Ink Free", size, bold, false);
        for it in s
            .first_header
            .iter_mut()
            .chain(s.first_footer.iter_mut())
            .chain(s.other_header.iter_mut())
            .chain(s.other_footer.iter_mut())
        {
            it.font.family = "Ink Free".into();
            it.font.italic = false;
        }
        s.first_header[0].font = hand(28.0, false);
        s.font_fret = hand(8.5, false);
        s.font_grace = hand(6.5, false);
        s.font_text = hand(9.0, false);
        s.font_chord = hand(11.0, true);
        s.font_tuning = hand(9.0, false);
        s.font_section = FontSpec::new("Ink Free", 11.0, true, false);
        s.font_tempo = hand(10.5, false);
        s.font_capo = hand(9.0, false);
        s.font_track_h = hand(10.0, false);
        s.font_track_v = hand(8.5, false);
        s.font_lyrics = hand(9.0, false);
        s.font_bar_number = hand(6.5, false);
        s.symbols = symbols("Ink Free");
        for sy in &mut s.symbols {
            sy.font.italic = false;
        }
        s.symbol_style = 1;
        s.chord_style = 1;
        s.chords_top = true;
        s.rhythm_hide_with_std = true;
        s.common_time_c = true;
        s.lines_over_rests = true;
        s
    }

    /// Page size in millimetres, turned for landscape.
    pub fn page_mm(&self) -> (f32, f32) {
        let (w, h) = (self.page_w.max(50.0), self.page_h.max(50.0));
        if self.landscape {
            (w.max(h), w.min(h))
        } else {
            (w.min(h), w.max(h))
        }
    }

    /// How much bigger than its natural size the music is drawn.
    pub fn music_scale(&self) -> f32 {
        (self.proportion / BASE_PROPORTION).clamp(0.4, 3.0)
    }

    /// Makes a stylesheet read from a file safe to use (lists of the right length, sane numbers).
    pub fn fixed(mut self) -> Stylesheet {
        let d = Stylesheet::standard();
        // Items a style saved earlier doesn't have yet are added in their usual place.
        let fix = |v: &mut Vec<HeaderItem>, d: &Vec<HeaderItem>| {
            for (k, it) in d.iter().enumerate() {
                if !v.iter().any(|x| x.name == it.name) {
                    v.insert(k.min(v.len()), it.clone());
                }
            }
            v.retain(|x| d.iter().any(|it| it.name == x.name));
        };
        fix(&mut self.first_header, &d.first_header);
        fix(&mut self.first_footer, &d.first_footer);
        fix(&mut self.other_header, &d.other_header);
        fix(&mut self.other_footer, &d.other_footer);
        if self.symbols.len() != d.symbols.len() {
            self.symbols = d.symbols.clone();
        }
        if !self.proportion.is_finite() || self.proportion <= 0.0 {
            self.proportion = d.proportion;
        }
        self.page_format = self.page_format.min(PAGE_FORMATS.len());
        self
    }
}

/// Song details the header and footer tokens stand for.
pub struct Tokens<'a> {
    pub title: &'a str,
    pub subtitle: &'a str,
    pub artist: &'a str,
    pub album: &'a str,
    pub words: &'a str,
    pub music: &'a str,
    pub tabber: &'a str,
    pub copyright: &'a str,
    pub page: usize,
    pub pages: usize,
}

pub const TOKENS: [&str; 10] = [
    "%TITLE%",
    "%SUBTITLE%",
    "%ARTIST%",
    "%ALBUM%",
    "%WORDS%",
    "%MUSIC%",
    "%TABBER%",
    "%COPYRIGHT%",
    "%PAGE%",
    "%PAGES%",
];

impl Tokens<'_> {
    fn value(&self, tok: &str) -> String {
        match tok {
            "%TITLE%" => self.title.into(),
            "%SUBTITLE%" => self.subtitle.into(),
            "%ARTIST%" => self.artist.into(),
            "%ALBUM%" => self.album.into(),
            "%WORDS%" => self.words.into(),
            "%MUSIC%" => self.music.into(),
            "%TABBER%" => self.tabber.into(),
            "%COPYRIGHT%" => self.copyright.into(),
            "%PAGE%" => self.page.to_string(),
            "%PAGES%" => self.pages.to_string(),
            _ => String::new(),
        }
    }
    /// The text with its tokens filled in; None when a token it uses is empty
    /// (so "Music by" isn't printed without a name).
    pub fn fill(&self, text: &str) -> Option<String> {
        let mut out = text.to_string();
        for tok in TOKENS {
            if out.contains(tok) {
                let v = self.value(tok);
                if v.trim().is_empty() {
                    return None;
                }
                out = out.replace(tok, v.trim());
            }
        }
        (!out.trim().is_empty()).then_some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toks<'a>(music: &'a str) -> Tokens<'a> {
        Tokens {
            title: "Song",
            subtitle: "",
            artist: "Band",
            album: "",
            words: "",
            music,
            tabber: "",
            copyright: "",
            page: 2,
            pages: 5,
        }
    }

    #[test]
    fn tokens_fill_and_skip_empty() {
        let t = toks("Ann");
        assert_eq!(t.fill("Music by %MUSIC%").as_deref(), Some("Music by Ann"));
        assert_eq!(t.fill("%PAGE%/%PAGES%").as_deref(), Some("2/5"));
        assert_eq!(t.fill("Words by %WORDS%"), None);
    }

    #[test]
    fn presets_differ_and_survive_json() {
        let r = Stylesheet::rock();
        assert_ne!(r, Stylesheet::standard());
        assert_ne!(Stylesheet::classic(), Stylesheet::jazz());
        let back: Stylesheet = serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
        assert_eq!(back, r);
        // Older files without a stylesheet get the standard one.
        let empty: Stylesheet = serde_json::from_str("{}").unwrap();
        assert_eq!(empty, Stylesheet::standard());
    }

    #[test]
    fn style_is_saved_in_gp_files() {
        let mut song = crate::model::Song::demo();
        song.style = Stylesheet::jazz();
        song.style.first_header[0].text = "Song: %TITLE%".into();
        let data = crate::gpifwrite::export_gp(&song).unwrap();
        let back = crate::gpif::import_gp(&data).unwrap();
        assert_eq!(back.style, song.style);
        // A song with the standard style doesn't need the extra file.
        let plain = crate::gpif::import_gp(
            &crate::gpifwrite::export_gp(&crate::model::Song::demo()).unwrap(),
        )
        .unwrap();
        assert_eq!(plain.style, Stylesheet::standard());
    }

    #[test]
    fn older_styles_gain_new_footer_lines() {
        let mut old = Stylesheet::standard();
        old.first_footer
            .retain(|x| x.name != "Copyright (second line)");
        old.first_footer[0].text = "(c) %COPYRIGHT%".into();
        let fixed = old.fixed();
        assert_eq!(fixed.first_footer.len(), 3);
        assert_eq!(fixed.first_footer[1].name, "Copyright (second line)");
        assert_eq!(fixed.first_footer[0].text, "(c) %COPYRIGHT%");
    }

    #[test]
    fn landscape_turns_the_page() {
        let mut s = Stylesheet::standard();
        s.landscape = true;
        assert_eq!(s.page_mm(), (297.0, 210.0));
    }
}
