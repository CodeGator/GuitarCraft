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

/// The handwritten font standing in for Guitar Pro's own jazz font.
pub const JAZZ_FONT: &str = "Ink Free";

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
fn arial(size: f32) -> FontSpec {
    FontSpec::new("Arial", size, false, false)
}

/// Header items of the first page, in their usual order (Guitar Pro 8's starting values).
fn first_header() -> Vec<HeaderItem> {
    vec![
        hi("Title", true, "%TITLE%", 1, times(25.0)),
        hi("Subtitle", true, "%SUBTITLE%", 1, times(12.0)),
        hi("Artist", true, "%ARTIST%", 1, times_b(12.0)),
        hi("Album", true, "%ALBUM%", 1, times(12.0)),
        hi("Words", true, "Words by %WORDS%", 0, times(10.0)),
        hi("Music", true, "Music by %MUSIC%", 2, times(10.0)),
        hi("Words & Music", true, "Words & Music by %MUSIC%", 2, times(10.0)),
        hi("Transcriber", false, "Tabbed by %TABBER%", 2, times(10.0)),
    ]
}

fn footer() -> Vec<HeaderItem> {
    vec![
        hi("Copyright", true, "%COPYRIGHT%", 1, times(7.0)),
        hi("Copyright (second line)", true, "All Rights Reserved - International Copyright Secured", 1, times(7.0)),
        hi("Page number", true, "%PAGE%/%PAGES%", 2, times(8.0)),
    ]
}

/// The following pages: a small "title by artist" line on top.
fn other_header() -> Vec<HeaderItem> {
    let mut v = first_header();
    for it in &mut v {
        it.on = false;
    }
    v[0].on = true;
    v[0].text = "%TITLE% by %ARTIST%".into();
    v[0].font = times(7.0);
    v[0].align = 1;
    v
}

fn symbols(family: &str) -> Vec<SymbolLabel> {
    let s = |name: &str, text: &str, f: FontSpec| SymbolLabel { name: name.into(), text: text.into(), font: f };
    let f = |size: f32, bold: bool, italic: bool| FontSpec::new(family, size, bold, italic);
    vec![
        s("Let Ring", "let ring", f(8.0, false, true)),
        s("Palm Mute", "P.M.", f(8.0, true, false)),
        s("Slide", "sl.", f(6.0, false, true)),
        s("Hammer-On", "H", f(6.0, false, false)),
        s("Pull-Off", "P", f(6.0, false, false)),
        s("Slap", "S", f(6.3, false, false)),
        s("Pop", "P", f(6.3, false, false)),
        s("Pick Scrape", "P.S.", f(8.0, false, false)),
        s("Rasgueado", "rasg.", f(8.0, false, true)),
        s("Whammy Bar Indication", "w/bar", f(8.0, false, false)),
    ]
}

impl Stylesheet {
    /// The look new songs start with: Guitar Pro 8's own starting values.
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
            tuning_mode: 2,
            tuning_cols: 0,
            tuning_framed: false,
            chord_style: 2,
            chords_top: true,
            chords_top_size: 10.0,
            chords_top_spacing: 3.0,
            chords_in_score: false,
            chords_in_score_size: 9.7,
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
            name_single_policy: 0,
            name_multi: true,
            name_multi_policy: 1,
            name_first_mode: 1,
            name_first_orient: 0,
            name_other_mode: 1,
            first_header: first_header(),
            first_footer: footer(),
            other_header: other_header(),
            other_footer: footer(),
            font_fret: arial(7.0),
            font_grace: arial(5.0),
            font_text: arial(7.0),
            font_chord: times_b(10.0),
            font_tuning: times(8.0),
            font_section: times_b(10.0),
            font_tempo: times_b(10.0),
            tempo_decimal: false,
            font_timer: FontSpec::new("Arial", 8.0, false, false),
            font_capo: times(8.0),
            font_barre: times_b(8.0),
            font_track_h: times_b(10.0),
            font_track_v: times(8.0),
            font_lyrics: times(6.0),
            lyrics_pos: 0,
            font_bar_number: times(6.0),
            bar_numbers: 0,
            symbol_style: 0,
            symbols: symbols("Times New Roman"),
            lines_over_rests: true,
            rhythm_hide_with_std: true,
            rhythm_extend: true,
            beams_on_rests: false,
            circle_half_whole: true,
            horizontal_beams: false,
            horizontal_beams_drums: true,
            rhythm_voice: [1, 1, 2, 2],
            harmonic_string: true,
            harmonic_dot: false,
            common_time_c: false,
            bend_real_pitch: true,
            arpeggio_arrow: true,
            trill_second: true,
            tied_in_tab: false,
            coda_space_on: true,
            coda_space: 6.0,
            hopo_letters: true,
            hopo_avoid: true,
            hopo_outer: true,
            hopo_single: true,
            slide_sl: true,
            color_voices: false,
            hide_useless_rests: true,
            dash_quarter_rest: false,
            hide_dynamics: false,
            slash_fret_string: false,
            tuplet_brackets: true,
            scale_grid: 2.0,
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

    // The ready-made styles, as the user guide describes them: the chord
    // diagram notation (Classic, Jazz or Rock) and the musical symbols (Classic
    // or Jazz). Everything else stays as Guitar Pro 8 starts a new file.

    /// Rock: Rock chord diagrams with Classic symbols (Guitar Pro 8's starting look).
    pub fn rock() -> Stylesheet {
        let mut s = Stylesheet::standard();
        s.chord_style = 2;
        s.symbol_style = 0;
        s
    }

    /// Classic, as Guitar Pro 8 sets it (the person's pictures 43-46 of the
    /// Stylesheet window after Options > Apply Classic style).
    pub fn classic() -> Stylesheet {
        let mut s = Stylesheet::standard();
        s.chord_style = 0;
        s.symbol_style = 0;
        // Systems & Staves.
        s.indent_on = true;
        s.indent = 10.0;
        s.brackets = 2;
        s.separator = true;
        // Header: bigger title lines; the words line is just the name; transcriber shown.
        let sizes = [36.0, 21.0, 24.0, 15.0, 15.0, 12.0, 11.0, 11.0];
        for (it, size) in s.first_header.iter_mut().zip(sizes) {
            it.font = times(size);
        }
        s.first_header[4].text = "%WORDS%".into();
        s.first_header[4].align = 1;
        s.first_header[7].on = true;
        // Texts & Styles.
        s.font_fret = arial(7.0);
        s.font_grace = arial(5.0);
        s.font_text = arial(8.0);
        s.font_chord = times(9.0);
        s.font_tuning = times(9.0);
        s.font_section = times_b(14.0);
        s.font_tempo = times(11.0);
        s.font_timer = arial(9.0);
        s.font_capo = times(9.0);
        s.font_barre = times(9.0);
        s.font_track_h = times(12.0);
        s.font_track_v = times(9.0);
        s.font_lyrics = times(9.0);
        s.font_bar_number = times(7.0);
        s.lines_over_rests = true;
        // Notation.
        s.rhythm_hide_with_std = true;
        s.rhythm_extend = true;
        s.beams_on_rests = false;
        s.circle_half_whole = false;
        s.horizontal_beams = true;
        s.horizontal_beams_drums = true;
        s.rhythm_voice = [1, 0, 2, 2];
        s.harmonic_string = false;
        s.harmonic_dot = false;
        s.common_time_c = true;
        s.bend_real_pitch = false;
        s.arpeggio_arrow = true;
        s.trill_second = true;
        s.tied_in_tab = false;
        s.coda_space_on = true;
        s.coda_space = 6.0;
        s.hopo_letters = false;
        s.hopo_avoid = true;
        s.hopo_outer = true;
        s.hopo_single = true;
        s.slide_sl = false;
        s.color_voices = false;
        s.hide_useless_rests = true;
        s.dash_quarter_rest = false;
        s.hide_dynamics = false;
        s.slash_fret_string = false;
        s.tuplet_brackets = true;
        s.scale_grid = 2.0;
        s.lh_type = 1;
        s.lh_scale = 1.0;
        s.lh_staff = 1;
        s.lh_tab = 0;
        s.rh_type = 0;
        s.rh_scale = 1.0;
        s.rh_staff = 0;
        s.rh_tab = 1;
        s
    }

    /// Jazz, as Guitar Pro 8 sets it (the person's pictures 47-50 after
    /// Options > Apply Jazz style). Guitar Pro uses its own handwritten font
    /// there, which isn't on this computer; Windows' "Ink Free" stands in.
    pub fn jazz() -> Stylesheet {
        let mut s = Stylesheet::standard();
        let hand = |size: f32| FontSpec::new(JAZZ_FONT, size, false, false);
        s.chord_style = 1;
        s.symbol_style = 1;
        // Systems & Staves.
        s.indent_on = false;
        s.clef_each_system = false;
        s.capo_affects_std = false;
        s.brackets = 0;
        s.separator = false;
        s.name_first_mode = 1;
        s.name_first_orient = 1;
        s.name_other_mode = 1;
        // Header & footer, all in the handwritten font.
        let sizes = [36.0, 16.0, 16.0, 14.0, 11.0, 11.0, 11.0, 11.0];
        for (it, size) in s.first_header.iter_mut().zip(sizes) {
            it.font = hand(size);
        }
        for (it, size) in s.first_footer.iter_mut().zip([6.0, 6.0, 7.0]) {
            it.font = hand(size);
        }
        for it in s.other_header.iter_mut().chain(s.other_footer.iter_mut()) {
            it.font.family = JAZZ_FONT.into();
            it.font.bold = false;
            it.font.italic = false;
        }
        // Texts & Styles.
        s.font_fret = hand(7.5);
        s.font_grace = hand(6.0);
        s.font_text = hand(10.0);
        s.font_chord = hand(14.0);
        s.font_tuning = hand(10.0);
        s.font_section = FontSpec::new(JAZZ_FONT, 14.0, true, false);
        s.font_tempo = hand(13.0);
        s.font_timer = hand(10.0);
        s.font_capo = hand(12.0);
        s.font_barre = hand(12.0);
        s.font_track_h = hand(8.0);
        s.font_track_v = hand(8.0);
        s.font_lyrics = hand(7.0);
        s.font_bar_number = hand(7.0);
        s.symbols = symbols(JAZZ_FONT);
        s.lines_over_rests = true;
        // Notation.
        s.rhythm_hide_with_std = true;
        s.rhythm_extend = true;
        s.beams_on_rests = false;
        s.circle_half_whole = false;
        s.horizontal_beams = false;
        s.horizontal_beams_drums = true;
        s.rhythm_voice = [1, 1, 2, 2];
        s.harmonic_string = true;
        s.harmonic_dot = false;
        s.common_time_c = true;
        s.bend_real_pitch = false;
        s.arpeggio_arrow = false;
        s.trill_second = true;
        s.tied_in_tab = false;
        s.coda_space_on = true;
        s.coda_space = 6.0;
        s.hopo_letters = false;
        s.hopo_avoid = true;
        s.hopo_outer = true;
        s.hopo_single = true;
        s.slide_sl = false;
        s.color_voices = false;
        s.hide_useless_rests = true;
        s.dash_quarter_rest = false;
        s.hide_dynamics = true;
        s.slash_fret_string = false;
        s.tuplet_brackets = true;
        s.scale_grid = 2.0;
        s.lh_type = 0;
        s.lh_scale = 1.0;
        s.lh_staff = 1;
        s.lh_tab = 1;
        s.rh_type = 0;
        s.rh_scale = 1.0;
        s.rh_staff = 1;
        s.rh_tab = 0;
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
        let r = Stylesheet::jazz();
        // Rock is Guitar Pro 8's starting look; Classic and Jazz differ from it.
        assert_eq!(Stylesheet::rock(), Stylesheet::standard());
        assert_ne!(Stylesheet::classic(), Stylesheet::standard());
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
        // Only the files Guitar Pro itself writes are in the archive.
        let zip = zip::ZipArchive::new(std::io::Cursor::new(&data[..])).unwrap();
        let known = ["VERSION", "Content/score.gpif", "Content/PartConfiguration", "Content/BinaryStylesheet", "Content/LayoutConfiguration"];
        assert!(zip.file_names().all(|n| known.contains(&n) || n.starts_with("Content/ScoreViews/")));
        // The score stays well-formed XML with the stylesheet comment in it.
        let xml = crate::gpifwrite::gpif_xml(&song);
        assert!(roxmltree::Document::parse(&xml).is_ok());
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
