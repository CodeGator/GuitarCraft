//! The stylesheet inside a .gp file ("Content/BinaryStylesheet"): a list of
//! named settings, each with a type and a value. Laid out as: a big-endian
//! 32-bit count, then for each setting its name (one length byte, then the
//! text), a type byte (0 yes/no, 1 whole number, 2 decimal number, 3 text,
//! 7 colour) and the value. Built from the public descriptions of the format
//! and a .gp file the person saved; only the settings this app's stylesheet
//! has are written and read back, and Guitar Pro fills in the rest.

use crate::style::{FontSpec, HeaderItem, Stylesheet};
use std::collections::HashMap;

/// Guitar Pro's usual staff space in millimetres.
const GP_SPACE: f32 = 1.5;

#[derive(Clone, Debug, PartialEq)]
pub enum V {
    B(bool),
    I(i32),
    F(f32),
    S(String),
}

/// Guitar Pro's names for the header and footer lines, matched to this app's.
const HEADER: [(&str, &str); 8] = [
    ("Title", "Title"),
    ("Subtitle", "Subtitle"),
    ("Artist", "Artist"),
    ("Album", "Album"),
    ("Words", "Words"),
    ("Music", "Music"),
    ("Words & Music", "WordsAndMusic"),
    ("Transcriber", "Tabber"),
];
const FOOTER: [(&str, &str); 3] = [("Copyright", "Copyright"), ("Copyright (second line)", "Copyright2"), ("Page number", "PageNumber")];

/// Guitar Pro's names for the symbol labels, in the order of `Stylesheet::symbols`.
const SYMBOLS: [&str; 10] = ["LetRing", "PalmMute", "Slide", "HammerOn", "PullOff", "Slap", "Pop", "PickScrape", "Rasgueado", "WhammyBarIndication"];

/// One setting for each part of this app's stylesheet that Guitar Pro has too.
pub fn entries(st: &Stylesheet) -> Vec<(String, V)> {
    let mut v: Vec<(String, V)> = vec![];
    let mut put = |k: &str, x: V| v.push((k.to_string(), x));
    let (w, h) = (st.page_w.min(st.page_h), st.page_w.max(st.page_h));
    put("PageSetup/pagePortraitWidth", V::F(w));
    put("PageSetup/pagePortraitHeight", V::F(h));
    put("PageSetup/pageOrientation", V::I(st.landscape as i32));
    put("PageSetup/pageTopMargin", V::F(st.margin_top));
    put("PageSetup/pageBottomMargin", V::F(st.margin_bottom));
    put("PageSetup/pageLeftMargin", V::F(st.margin_left));
    put("PageSetup/pageRightMargin", V::F(st.margin_right));
    put("PageSetup/pageScale", V::F(1.0));
    // Guitar Pro's usual music size is 1.5 mm; this app's is BASE_PROPORTION.
    put("Global/spaceSizeMM", V::F(st.proportion * GP_SPACE / crate::style::BASE_PROPORTION));
    put("Global/SpacingAffectFontsSize", V::B(st.scale_fonts));
    put("Global/rhythmPower", V::F(st.rhythm_spacing));
    put("Global/DisplayTuning", V::B(st.tuning_show));
    put("Global/TuningPosition", V::I(st.tuning_pos as i32));
    put("Global/TuningDisplayMode", V::I(st.tuning_mode as i32));
    put("Global/tuningBoxed", V::B(st.tuning_framed));
    put("Global/chordNameStyle", V::I(st.chord_style as i32));
    put("Global/DrawChords", V::B(st.chords_top));
    put("Global/ChordSizeMillimeter", V::F(st.chords_top_size));
    put("Global/ChordSpacingMillimeter", V::F(st.chords_top_spacing));
    put("System/drawChordInScore", V::B(st.chords_in_score));
    put("System/inScoreChordSize", V::F(st.chords_in_score_size));
    put("System/SpacingBetweenSystems", V::F(st.system_gap));
    put("Global/firstSystemMarginMillimeter", V::I(if st.indent_on { st.indent.round() as i32 } else { 0 }));
    put("System/ExtendedBarLines", V::B(st.extended_barlines));
    put("Global/showClefAtEachSystemStart", V::B(st.clef_each_system));
    put("Global/ShowTimeSignatureAtEachSystemStart", V::B(st.time_each_system));
    put("Global/ShowTimeSignatureAtEachSection", V::B(st.time_each_section));
    put("Global/useSystemSignSeparator", V::B(st.separator));
    put("System/showTrackNameSingle", V::B(st.name_single));
    put("System/trackNameModeSingle", V::I(st.name_single_policy as i32));
    put("System/showTrackNameMulti", V::B(st.name_multi));
    put("System/trackNameModeMulti", V::I(st.name_multi_policy as i32));
    put("System/shortTrackNameOnFirstSystem", V::B(st.name_first_mode == 1));
    put("System/shortTrackNameOnOtherSystems", V::B(st.name_other_mode == 1));
    put("System/horizontalTrackNameOnFirstSystem", V::B(st.name_first_orient == 1));
    put("System/hideLyrics", V::B(st.lyrics_pos == 2));
    put("System/drawLyricsUpside", V::B(st.lyrics_pos == 1));
    put("System/codaSplit", V::B(st.coda_space_on));
    put("System/codaSplitWidth", V::F(st.coda_space));
    put("Global/drawArpeggioArrow", V::B(st.arpeggio_arrow));
    put("Global/ShowTupletBracket", V::B(st.tuplet_brackets));
    put("Global/ScaleDiagramGridSpacing", V::F(st.scale_grid));
    put("Global/LeftFingeringScaleValue", V::F(st.lh_scale));
    put("Global/RightFingeringScaleValue", V::F(st.rh_scale));
    for k in ["LetRing", "PalmMute", "Rasgueado", "WhammyBarIndication"] {
        put(&format!("Global/{k}/extendOverRests"), V::B(st.lines_over_rests));
    }
    let mut font = |name: &str, f: &FontSpec| {
        put(&format!("Global/Font/{name}/name"), V::S(f.family.clone()));
        put(&format!("Global/Font/{name}/size"), V::F(f.size));
        put(&format!("Global/Font/{name}/bold"), V::B(f.bold));
        put(&format!("Global/Font/{name}/italic"), V::B(f.italic));
    };
    font("Fret", &st.font_fret);
    font("SmallFret", &st.font_grace);
    font("FreeText", &st.font_text);
    font("Chord", &st.font_chord);
    font("Tuning", &st.font_tuning);
    font("Section", &st.font_section);
    font("Tempo", &st.font_tempo);
    font("Timer", &st.font_timer);
    font("Capo", &st.font_capo);
    font("Barre", &st.font_barre);
    font("HorizontalTrackName", &st.font_track_h);
    font("VerticalTrackName", &st.font_track_v);
    font("Lyric", &st.font_lyrics);
    font("BarIndex", &st.font_bar_number);
    for (k, gp) in SYMBOLS.iter().enumerate() {
        if let Some(s) = st.symbols.get(k) {
            font(gp, &s.font);
        }
    }
    for (group, items, names) in [("Header", &st.first_header, &HEADER[..]), ("Footer", &st.first_footer, &FOOTER[..])] {
        for (ours, gp) in names {
            if let Some(it) = items.iter().find(|x| x.name == *ours) {
                font(gp, &it.font);
            }
        }
        let _ = group;
    }
    for (k, gp) in SYMBOLS.iter().enumerate() {
        if let Some(s) = st.symbols.get(k) {
            put(&format!("Global/Font/{gp}/text"), V::S(s.text.clone()));
        }
    }
    for (group, items, names) in [("Header", &st.first_header, &HEADER[..]), ("Footer", &st.first_footer, &FOOTER[..])] {
        for (ours, gp) in names {
            if let Some(it) = items.iter().find(|x| x.name == *ours) {
                put(&format!("{group}/{gp}"), V::S(it.text.clone()));
                put(&format!("{group}/{gp}Alignment"), V::I(it.align as i32));
                put(&format!("{group}/draw{gp}"), V::B(it.on));
            }
        }
    }
    v
}

/// The whole "BinaryStylesheet" file.
pub fn write(st: &Stylesheet) -> Vec<u8> {
    let e = entries(st);
    let mut out = (e.len() as i32).to_be_bytes().to_vec();
    for (k, v) in &e {
        let kb = k.as_bytes();
        out.push(kb.len().min(255) as u8);
        out.extend_from_slice(&kb[..kb.len().min(255)]);
        match v {
            V::B(b) => {
                out.push(0);
                out.push(*b as u8);
            }
            V::I(i) => {
                out.push(1);
                out.extend_from_slice(&i.to_be_bytes());
            }
            V::F(f) => {
                out.push(2);
                out.extend_from_slice(&f.to_be_bytes());
            }
            V::S(s) => {
                let sb = s.as_bytes();
                let n = sb.len().min(i16::MAX as usize);
                out.push(3);
                out.extend_from_slice(&(n as i16).to_be_bytes());
                out.extend_from_slice(&sb[..n]);
            }
        }
    }
    out
}

/// Reads every setting of a "BinaryStylesheet" file (colours and sizes are skipped).
pub fn read(d: &[u8]) -> Option<HashMap<String, V>> {
    let n = i32::from_be_bytes(d.get(..4)?.try_into().ok()?).max(0) as usize;
    let mut i = 4;
    let mut map = HashMap::new();
    let be4 = |i: usize| -> Option<[u8; 4]> { d.get(i..i + 4)?.try_into().ok() };
    for _ in 0..n {
        let l = *d.get(i)? as usize;
        let key = String::from_utf8_lossy(d.get(i + 1..i + 1 + l)?).to_string();
        i += 1 + l;
        let t = *d.get(i)?;
        i += 1;
        let v = match t {
            0 => {
                i += 1;
                Some(V::B(*d.get(i - 1)? != 0))
            }
            1 => {
                i += 4;
                Some(V::I(i32::from_be_bytes(be4(i - 4)?)))
            }
            2 => {
                i += 4;
                Some(V::F(f32::from_be_bytes(be4(i - 4)?)))
            }
            3 => {
                let len = i16::from_be_bytes(d.get(i..i + 2)?.try_into().ok()?).max(0) as usize;
                let s = String::from_utf8_lossy(d.get(i + 2..i + 2 + len)?).to_string();
                i += 2 + len;
                Some(V::S(s))
            }
            4 | 5 => {
                i += 8;
                None
            }
            6 => {
                i += 16;
                None
            }
            7 => {
                i += 4;
                None
            }
            _ => return Some(map),
        };
        if let Some(v) = v {
            map.insert(key, v);
        }
    }
    Some(map)
}

/// Takes the settings this app knows from a Guitar Pro stylesheet.
pub fn apply(map: &HashMap<String, V>, st: &mut Stylesheet) {
    let b = |k: &str| match map.get(k) {
        Some(V::B(x)) => Some(*x),
        _ => None,
    };
    let i = |k: &str| match map.get(k) {
        Some(V::I(x)) => Some(*x),
        _ => None,
    };
    let f = |k: &str| match map.get(k) {
        Some(V::F(x)) if x.is_finite() => Some(*x),
        Some(V::I(x)) => Some(*x as f32),
        Some(V::S(s)) => s.trim().parse::<f32>().ok(),
        _ => None,
    };
    let s = |k: &str| match map.get(k) {
        Some(V::S(x)) => Some(x.clone()),
        _ => None,
    };
    let u8v = |k: &str, max: u8| i(k).map(|x| x.clamp(0, max as i32) as u8);
    if let (Some(w), Some(h)) = (f("PageSetup/pagePortraitWidth"), f("PageSetup/pagePortraitHeight")) {
        st.page_w = w.clamp(50.0, 600.0);
        st.page_h = h.clamp(50.0, 800.0);
        st.page_format = crate::style::PAGE_FORMATS.iter().position(|p| (p.1 - st.page_w).abs() < 0.6 && (p.2 - st.page_h).abs() < 0.6).unwrap_or(crate::style::PAGE_FORMATS.len());
    }
    if let Some(o) = i("PageSetup/pageOrientation") {
        st.landscape = o == 1;
    }
    let set = |v: &mut f32, x: Option<f32>, lo: f32, hi: f32| {
        if let Some(x) = x {
            *v = x.clamp(lo, hi);
        }
    };
    set(&mut st.margin_top, f("PageSetup/pageTopMargin"), 0.0, 80.0);
    set(&mut st.margin_bottom, f("PageSetup/pageBottomMargin"), 0.0, 80.0);
    set(&mut st.margin_left, f("PageSetup/pageLeftMargin"), 0.0, 80.0);
    set(&mut st.margin_right, f("PageSetup/pageRightMargin"), 0.0, 80.0);
    set(&mut st.proportion, f("Global/spaceSizeMM").map(|x| x * crate::style::BASE_PROPORTION / GP_SPACE), 0.8, 4.0);
    set(&mut st.rhythm_spacing, f("Global/rhythmPower"), 0.0, 1.0);
    set(&mut st.chords_top_size, f("Global/ChordSizeMillimeter"), 5.0, 20.0);
    set(&mut st.chords_top_spacing, f("Global/ChordSpacingMillimeter"), 0.0, 20.0);
    set(&mut st.chords_in_score_size, f("System/inScoreChordSize"), 5.0, 20.0);
    set(&mut st.system_gap, f("System/SpacingBetweenSystems"), 0.0, 40.0);
    set(&mut st.coda_space, f("System/codaSplitWidth"), 0.0, 40.0);
    set(&mut st.scale_grid, f("Global/ScaleDiagramGridSpacing"), 0.5, 3.0);
    set(&mut st.lh_scale, f("Global/LeftFingeringScaleValue"), 0.5, 2.0);
    set(&mut st.rh_scale, f("Global/RightFingeringScaleValue"), 0.5, 2.0);
    let flag = |v: &mut bool, x: Option<bool>| {
        if let Some(x) = x {
            *v = x;
        }
    };
    flag(&mut st.scale_fonts, b("Global/SpacingAffectFontsSize"));
    flag(&mut st.tuning_show, b("Global/DisplayTuning"));
    flag(&mut st.tuning_framed, b("Global/tuningBoxed"));
    flag(&mut st.chords_top, b("Global/DrawChords"));
    flag(&mut st.chords_in_score, b("System/drawChordInScore"));
    flag(&mut st.extended_barlines, b("System/ExtendedBarLines"));
    flag(&mut st.clef_each_system, b("Global/showClefAtEachSystemStart"));
    flag(&mut st.time_each_system, b("Global/ShowTimeSignatureAtEachSystemStart"));
    flag(&mut st.time_each_section, b("Global/ShowTimeSignatureAtEachSection"));
    flag(&mut st.separator, b("Global/useSystemSignSeparator"));
    flag(&mut st.name_single, b("System/showTrackNameSingle"));
    flag(&mut st.name_multi, b("System/showTrackNameMulti"));
    flag(&mut st.coda_space_on, b("System/codaSplit"));
    flag(&mut st.arpeggio_arrow, b("Global/drawArpeggioArrow"));
    flag(&mut st.tuplet_brackets, b("Global/ShowTupletBracket"));
    flag(&mut st.lines_over_rests, b("Global/LetRing/extendOverRests"));
    if let Some(x) = u8v("Global/TuningPosition", 1) {
        st.tuning_pos = x;
    }
    if let Some(x) = u8v("Global/TuningDisplayMode", 2) {
        st.tuning_mode = x;
    }
    if let Some(x) = u8v("Global/chordNameStyle", 2) {
        st.chord_style = x;
    }
    if let Some(x) = u8v("System/trackNameModeSingle", 2) {
        st.name_single_policy = x;
    }
    if let Some(x) = u8v("System/trackNameModeMulti", 2) {
        st.name_multi_policy = x;
    }
    if let Some(m) = i("Global/firstSystemMarginMillimeter") {
        st.indent_on = m > 0;
        st.indent = m.clamp(0, 60) as f32;
    }
    if let Some(x) = b("System/shortTrackNameOnFirstSystem") {
        st.name_first_mode = x as u8;
    }
    if let Some(x) = b("System/shortTrackNameOnOtherSystems") {
        st.name_other_mode = x as u8;
    }
    if let Some(x) = b("System/horizontalTrackNameOnFirstSystem") {
        st.name_first_orient = x as u8;
    }
    if b("System/hideLyrics") == Some(true) {
        st.lyrics_pos = 2;
    } else if let Some(up) = b("System/drawLyricsUpside") {
        st.lyrics_pos = if up { 1 } else { 0 };
    }
    let font = |name: &str, fs: &mut FontSpec| {
        if let Some(fam) = s(&format!("Global/Font/{name}/name")) {
            // Guitar Pro's own music text font isn't on this computer: keep ours.
            if !fam.starts_with('@') && !fam.is_empty() {
                fs.family = if crate::style::FONTS.iter().any(|x| x.0 == fam) { fam } else { fs.family.clone() };
            }
        }
        if let Some(sz) = f(&format!("Global/Font/{name}/size")) {
            fs.size = sz.clamp(4.0, 72.0);
        }
        flag(&mut fs.bold, b(&format!("Global/Font/{name}/bold")));
        flag(&mut fs.italic, b(&format!("Global/Font/{name}/italic")));
    };
    font("Fret", &mut st.font_fret);
    font("SmallFret", &mut st.font_grace);
    font("FreeText", &mut st.font_text);
    font("Chord", &mut st.font_chord);
    font("Tuning", &mut st.font_tuning);
    font("Section", &mut st.font_section);
    font("Tempo", &mut st.font_tempo);
    font("Timer", &mut st.font_timer);
    font("Capo", &mut st.font_capo);
    font("Barre", &mut st.font_barre);
    font("HorizontalTrackName", &mut st.font_track_h);
    font("VerticalTrackName", &mut st.font_track_v);
    font("Lyric", &mut st.font_lyrics);
    font("BarIndex", &mut st.font_bar_number);
    for (k, gp) in SYMBOLS.iter().enumerate() {
        if let Some(sy) = st.symbols.get_mut(k) {
            font(gp, &mut sy.font);
            if let Some(t) = s(&format!("Global/Font/{gp}/text")) {
                sy.text = t;
            }
        }
    }
    let lines = |group: &str, items: &mut Vec<HeaderItem>, names: &[(&str, &str)]| {
        for (ours, gp) in names {
            if let Some(it) = items.iter_mut().find(|x| x.name == *ours) {
                font(gp, &mut it.font);
                if let Some(t) = s(&format!("{group}/{gp}")) {
                    it.text = t;
                }
                if let Some(a) = i(&format!("{group}/{gp}Alignment")) {
                    it.align = a.clamp(0, 2) as u8;
                }
                flag(&mut it.on, b(&format!("{group}/draw{gp}")));
            }
        }
    };
    lines("Header", &mut st.first_header, &HEADER);
    lines("Footer", &mut st.first_footer, &FOOTER);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_stylesheet_round_trip() {
        let st = Stylesheet::classic();
        let data = write(&st);
        let map = read(&data).unwrap();
        assert_eq!(map.len(), entries(&st).len());
        let mut back = Stylesheet::standard();
        apply(&map, &mut back);
        assert_eq!(back.margin_top, st.margin_top);
        assert_eq!(back.first_header[0].text, st.first_header[0].text);
        assert_eq!(back.font_track_h, st.font_track_h);
        assert_eq!(back.name_first_orient, st.name_first_orient);
        assert_eq!(back.indent_on, st.indent_on);
    }

    #[test]
    fn reads_guitar_pro_saved_stylesheet() {
        // A Guitar Pro 8 file the person saved, when present.
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("samples").join("Empty.gp");
        let Ok(data) = std::fs::read(p) else { return };
        let mut z = zip::ZipArchive::new(std::io::Cursor::new(&data[..])).unwrap();
        let mut bin = vec![];
        std::io::Read::read_to_end(&mut z.by_name("Content/BinaryStylesheet").unwrap(), &mut bin).unwrap();
        let map = read(&bin).unwrap();
        assert!(map.len() > 600);
        assert_eq!(map.get("PageSetup/pageTopMargin"), Some(&V::F(15.0)));
    }
}
