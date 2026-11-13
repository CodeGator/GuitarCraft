//! Reads newer Guitar Pro files: .gpx (version 6) and .gp (version 7 and later).
//! Both hold an XML score ("score.gpif"); .gp is a zip archive and .gpx a
//! compressed container. Built from the community-documented file layout.
//! Tracks without string tunings (drums, vocals, keyboards) keep their notes by pitch.

use crate::model::*;
use roxmltree::{Document, Node};
use std::collections::HashMap;
use std::io::Read;

pub fn import_gp(data: &[u8]) -> Result<Song, String> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(data))
        .map_err(|_| "This .gp file isn't a readable archive.".to_string())?;
    let mut xml = String::new();
    let name = (0..zip.len())
        .filter_map(|i| zip.by_index(i).ok().map(|f| f.name().to_string()))
        .find(|n| n.ends_with("score.gpif"))
        .ok_or_else(|| "No score was found inside this .gp file.".to_string())?;
    zip.by_name(&name)
        .map_err(|e| e.to_string())?
        .read_to_string(&mut xml)
        .map_err(|e| e.to_string())?;
    let mut song = parse_gpif(&xml)?;
    // Which tracks the full score shows (the others are hidden with the eye icon).
    let mut view = vec![];
    if let Ok(mut f) = zip.by_name("Content/ScoreViews/0.gpsv") {
        let _ = f.read_to_end(&mut view);
    }
    if let Some(shown) = full_score_tracks(&view)
        && !shown.is_empty()
        && shown.iter().all(|&k| k < song.tracks.len())
    {
        for (k, t) in song.tracks.iter_mut().enumerate() {
            t.visible = shown.contains(&k);
        }
    }
    // The settings of Guitar Pro's stylesheet that this app has, then this
    // app's full stylesheet when the score carries one.
    let mut gp_style = vec![];
    if let Ok(mut f) = zip.by_name("Content/BinaryStylesheet") {
        let _ = f.read_to_end(&mut gp_style);
    }
    if let Some(map) = crate::gpstyle::read(&gp_style) {
        crate::gpstyle::apply(&map, &mut song.style);
    }
    if let Some(st) = crate::gpifwrite::style_from_xml(&xml) {
        song.style = st;
    }
    if let Some(x) = crate::extras::from_xml(&xml) {
        crate::extras::apply(&mut song, &x);
    }
    // Which staves each track shows (notation, tablature).
    let mut part = vec![];
    if let Ok(mut f) = zip.by_name("Content/PartConfiguration") {
        let _ = f.read_to_end(&mut part);
    }
    if part.len() >= 9 {
        let n = i32::from_be_bytes([part[5], part[6], part[7], part[8]]).max(0) as usize;
        for (k, t) in song.tracks.iter_mut().enumerate().take(n) {
            if let Some(&f) = part.get(9 + k)
                && t.has_strings()
                && f & 3 != 0
            {
                t.show_std = f & 1 != 0;
                t.show_tab = f & 2 != 0;
            }
        }
    }
    Ok(song)
}

pub fn import_gpx(data: &[u8]) -> Result<Song, String> {
    let fs = if data.starts_with(b"BCFZ") {
        decompress_bcfz(&data[4..])?
    } else if data.starts_with(b"BCFS") {
        data[4..].to_vec()
    } else {
        return Err("This .gpx file has an unknown layout.".into());
    };
    let file = read_bcfs(&fs, "score.gpif")
        .ok_or_else(|| "No score was found inside this .gpx file.".to_string())?;
    let xml = String::from_utf8_lossy(&file).to_string();
    parse_gpif(&xml)
}

type PbField<'a> = (u64, u8, u64, &'a [u8]);

/// Fields of a protocol-buffer message: (field number, wire type, number value, bytes).
fn pb_fields(b: &[u8]) -> Option<Vec<PbField<'_>>> {
    fn varint(b: &[u8], i: &mut usize) -> Option<u64> {
        let mut r = 0u64;
        let mut s = 0;
        loop {
            let c = *b.get(*i)?;
            *i += 1;
            r |= ((c & 0x7f) as u64) << s;
            s += 7;
            if c < 0x80 {
                return Some(r);
            }
            if s > 63 {
                return None;
            }
        }
    }
    let mut out = vec![];
    let mut i = 0;
    while i < b.len() {
        let key = varint(b, &mut i)?;
        let (f, wt) = (key >> 3, (key & 7) as u8);
        match wt {
            0 => out.push((f, 0, varint(b, &mut i)?, &b[0..0])),
            2 => {
                let n = varint(b, &mut i)? as usize;
                let d = b.get(i..i + n)?;
                i += n;
                out.push((f, 2, 0, d));
            }
            5 => {
                i += 4;
                out.push((f, 5, 0, &b[0..0]));
            }
            1 => {
                i += 8;
                out.push((f, 1, 0, &b[0..0]));
            }
            _ => return None,
        }
    }
    Some(out)
}

/// The tracks laid out in the "full score" view: each line of music lists its
/// staves (after a header entry), each naming its track (missing means track 0).
fn full_score_tracks(view: &[u8]) -> Option<Vec<usize>> {
    let top = pb_fields(view)?;
    let system = top.iter().find(|f| f.0 == 4 && f.1 == 2)?.3;
    let mut shown = vec![];
    for (_, wt, _, d) in pb_fields(system)?.into_iter().filter(|f| f.0 == 13).skip(1) {
        if wt != 2 {
            continue;
        }
        let Some(key) = pb_fields(d)?.into_iter().find(|g| g.0 == 9 && g.1 == 2) else {
            continue;
        };
        let mut track = 0usize;
        for (h, hw, v, _) in pb_fields(key.3)? {
            if h == 1 && hw == 0 {
                track = v as usize;
            }
        }
        if !shown.contains(&track) {
            shown.push(track);
        }
    }
    Some(shown)
}

struct Bits<'a> {
    d: &'a [u8],
    pos: usize,
    bit: u32,
}

impl Bits<'_> {
    fn bit(&mut self) -> Option<u32> {
        let byte = *self.d.get(self.pos)?;
        let v = (byte >> (7 - self.bit)) & 1;
        self.bit += 1;
        if self.bit == 8 {
            self.bit = 0;
            self.pos += 1;
        }
        Some(v as u32)
    }
    fn bits(&mut self, n: u32) -> Option<u32> {
        let mut w = 0;
        for i in 0..n {
            w |= self.bit()? << (n - i - 1);
        }
        Some(w)
    }
    fn bits_rev(&mut self, n: u32) -> Option<u32> {
        let mut w = 0;
        for i in 0..n {
            w |= self.bit()? << i;
        }
        Some(w)
    }
}

fn decompress_bcfz(d: &[u8]) -> Result<Vec<u8>, String> {
    if d.len() < 4 {
        return Err("This .gpx file is too short.".into());
    }
    let expected = i32::from_le_bytes([d[0], d[1], d[2], d[3]]).max(0) as usize;
    let mut out: Vec<u8> = Vec::with_capacity(expected);
    let mut b = Bits {
        d: &d[4..],
        pos: 0,
        bit: 0,
    };
    while out.len() < expected {
        let Some(flag) = b.bit() else { break };
        if flag == 1 {
            let (Some(ws), _) = (b.bits(4), ()) else {
                break;
            };
            let (Some(offset), Some(size)) = (b.bits_rev(ws), b.bits_rev(ws)) else {
                break;
            };
            let offset = offset as usize;
            if offset == 0 || offset > out.len() {
                return Err("This .gpx file is damaged.".into());
            }
            let src = out.len() - offset;
            let n = offset.min(size as usize);
            for k in 0..n {
                out.push(out[src + k]);
            }
        } else {
            let Some(size) = b.bits_rev(2) else { break };
            for _ in 0..size {
                let Some(x) = b.bits(8) else { break };
                out.push(x as u8);
            }
        }
    }
    if out.starts_with(b"BCFS") {
        out.drain(..4);
    }
    Ok(out)
}

fn le32(d: &[u8], at: usize) -> Option<usize> {
    let s = d.get(at..at + 4)?;
    Some(i32::from_le_bytes([s[0], s[1], s[2], s[3]]).max(0) as usize)
}

/// Finds a file in the sector-based container of a .gpx file.
fn read_bcfs(d: &[u8], want: &str) -> Option<Vec<u8>> {
    const SECTOR: usize = 0x1000;
    let mut offset = SECTOR;
    while offset + 3 < d.len() {
        if le32(d, offset) == Some(2) {
            let raw = d.get(offset + 4..offset + 4 + 127)?;
            let end = raw.iter().position(|&c| c == 0).unwrap_or(raw.len());
            let name = String::from_utf8_lossy(&raw[..end]).to_string();
            let size = le32(d, offset + 0x8C)?;
            let mut data = vec![];
            let mut k = 0;
            loop {
                let sector = le32(d, offset + 0x94 + 4 * k)?;
                k += 1;
                if sector == 0 || k > 100_000 {
                    break;
                }
                let at = sector * SECTOR;
                data.extend_from_slice(d.get(at..(at + SECTOR).min(d.len()))?);
            }
            if name == want {
                data.truncate(size);
                return Some(data);
            }
        }
        offset += SECTOR;
    }
    None
}

fn child<'a, 'i>(n: Node<'a, 'i>, name: &str) -> Option<Node<'a, 'i>> {
    n.children()
        .find(|c| c.is_element() && c.tag_name().name() == name)
}

fn text(n: Option<Node>) -> String {
    n.and_then(|n| n.text()).unwrap_or("").trim().to_string()
}

fn path<'a, 'i>(n: Node<'a, 'i>, p: &[&str]) -> Option<Node<'a, 'i>> {
    let mut cur = n;
    for s in p {
        cur = child(cur, s)?;
    }
    Some(cur)
}

/// The `<Property name="...">` children of a `<Properties>` element.
fn props<'a, 'i>(n: Node<'a, 'i>) -> HashMap<String, Node<'a, 'i>> {
    let mut m = HashMap::new();
    if let Some(ps) = child(n, "Properties") {
        for p in ps.children().filter(|c| c.has_tag_name("Property")) {
            if let Some(name) = p.attribute("name") {
                m.insert(name.to_string(), p);
            }
        }
    }
    m
}

fn index<'a, 'i>(root: Node<'a, 'i>, coll: &str, item: &str) -> HashMap<String, Node<'a, 'i>> {
    let mut m = HashMap::new();
    if let Some(c) = child(root, coll) {
        for n in c.children().filter(|x| x.has_tag_name(item)) {
            if let Some(id) = n.attribute("id") {
                m.insert(id.to_string(), n);
            }
        }
    }
    m
}

struct TrackInfo {
    keep: bool,
    staff_offset: usize,
    chords: HashMap<String, String>,
}

pub fn parse_gpif(xml: &str) -> Result<Song, String> {
    let doc = Document::parse(xml)
        .map_err(|e| format!("The score inside the file couldn't be read ({e})."))?;
    let root = doc.root_element();
    let mut song = Song::new_blank();
    if let Some(sc) = child(root, "Score") {
        song.title = text(child(sc, "Title"));
        song.subtitle = text(child(sc, "SubTitle"));
        song.artist = text(child(sc, "Artist"));
        song.album = text(child(sc, "Album"));
        song.music_by = text(child(sc, "Music"));
        song.tabbed_by = text(child(sc, "Tabber"));
        song.words = text(child(sc, "Words"));
        song.copyright = text(child(sc, "Copyright"));
        song.notice = text(child(sc, "Notices"));
        song.instructions = text(child(sc, "Instructions"));
    }
    if song.title.is_empty() {
        song.title = "Untitled".into();
    }
    // Tempo changes.
    let mut tempos: Vec<(usize, u32)> = vec![];
    let mut ramps: Vec<usize> = vec![];
    if let Some(autos) = path(root, &["MasterTrack", "Automations"]) {
        for a in autos.children().filter(|c| c.has_tag_name("Automation")) {
            if text(child(a, "Type")) == "Tempo" {
                let bar = text(child(a, "Bar")).parse::<usize>().unwrap_or(0);
                let v = text(child(a, "Value"))
                    .split_whitespace()
                    .next()
                    .and_then(|x| x.parse::<f32>().ok())
                    .unwrap_or(120.0);
                tempos.push((bar, v.round().clamp(20.0, 400.0) as u32));
                if text(child(a, "Linear")) == "true" {
                    ramps.push(bar);
                }
            }
        }
    }
    // Tracks.
    let mut infos = vec![];
    let mut tracks = vec![];
    let mut staff_offset = 0;
    if let Some(ts) = child(root, "Tracks") {
        for (k, t) in ts
            .children()
            .filter(|c| c.has_tag_name("Track"))
            .enumerate()
        {
            let staves = child(t, "Staves")
                .map(|s| s.children().filter(|c| c.has_tag_name("Staff")).count())
                .unwrap_or(1)
                .max(1);
            let mut tuning: Vec<u8> = vec![];
            let mut capo = 0u8;
            let mut tuning_flat = true;
            let mut partial = 0u8;
            let mut partial_bits = String::new();
            let mut tuning_label = String::new();
            let mut tuning_label_visible = true;
            let mut chords = HashMap::new();
            for p in t.descendants().filter(|d| d.has_tag_name("Property")) {
                match p.attribute("name") {
                    Some("Tuning") if tuning.is_empty() => {
                        tuning_flat = child(p, "Flat").is_some();
                        tuning_label = text(child(p, "Label"));
                        tuning_label_visible = text(child(p, "LabelVisible")) != "false";
                        let mut v: Vec<u8> = text(child(p, "Pitches"))
                            .split_whitespace()
                            .filter_map(|x| x.parse::<i32>().ok())
                            .map(|x| x.clamp(0, 127) as u8)
                            .collect();
                        v.reverse();
                        tuning = v;
                    }
                    Some("CapoFret") => {
                        capo = text(child(p, "Fret"))
                            .parse::<i32>()
                            .unwrap_or(0)
                            .clamp(0, 24) as u8
                    }
                    Some("PartialCapoFret") => {
                        partial = text(child(p, "Fret"))
                            .parse::<i32>()
                            .unwrap_or(0)
                            .clamp(0, 24) as u8
                    }
                    Some("PartialCapoStringFlags") => partial_bits = text(child(p, "Bitset")),
                    Some("DiagramCollection") | Some("DiagramWorkingSet") => {
                        for it in p.descendants().filter(|d| d.has_tag_name("Item")) {
                            if let (Some(id), Some(name)) =
                                (it.attribute("id"), it.attribute("name"))
                            {
                                chords.insert(id.to_string(), name.to_string());
                            }
                        }
                    }
                    _ => {}
                }
            }
            let name = text(child(t, "Name"));
            let inst_type = t
                .descendants()
                .find(|d| {
                    d.has_tag_name("Type")
                        && d.parent()
                            .map(|p| p.has_tag_name("InstrumentSet"))
                            .unwrap_or(false)
                })
                .map(|n| text(Some(n)).to_lowercase())
                .unwrap_or_default();
            let program = t
                .descendants()
                .find(|d| d.has_tag_name("Program"))
                .map(|n| text(Some(n)))
                .and_then(|x| x.parse::<i32>().ok())
                .unwrap_or(25);
            let drum = inst_type.contains("drum") || inst_type.contains("percussion");
            let keep = tuning.len() <= 8;
            let mut tr = Track {
                name: if name.is_empty() {
                    format!("Track {}", k + 1)
                } else {
                    name.clone()
                },
                instrument: Instrument::from_midi_program(program.clamp(0, 127) as u8),
                tuning: tuning.clone(),
                capo,
                color: TRACK_COLORS[k % TRACK_COLORS.len()],
                ..Track::default()
            };
            tr.short_name = text(child(t, "ShortName"));
            tr.icon = text(child(t, "IconId")).parse::<u8>().unwrap_or(0);
            tr.tuning_flat = tuning_flat;
            tr.partial_capo = partial;
            // One digit per string, the lowest string first.
            let n = partial_bits.len();
            tr.partial_capo_strings = partial_bits
                .chars()
                .enumerate()
                .filter(|(_, c)| *c == '1')
                .map(|(k, _)| n - 1 - k)
                .filter(|&s| s < 8)
                .fold(0u8, |a, s| a | (1 << s));
            tr.tuning_label = tuning_label.clone();
            tr.tuning_label_visible = tuning_label_visible;
            tr.palm_mute = text(child(t, "PalmMute"))
                .parse::<f32>()
                .unwrap_or(0.0)
                .clamp(0.0, 1.0);
            tr.accentuation = text(child(t, "AutoAccentuation"))
                .parse::<f32>()
                .unwrap_or(0.0)
                .clamp(0.0, 1.0);
            // Flags: an empty element when on (older files may say true/false).
            tr.auto_let_ring = child(t, "LetRingThroughout")
                .map(|n| text(Some(n)) != "false")
                .unwrap_or(false);
            tr.auto_brush = child(t, "AutoBrush")
                .map(|n| text(Some(n)) != "false")
                .unwrap_or(false);
            let style = text(child(t, "PlayingStyle"));
            tr.playing_style = PLAYING_STYLES
                .iter()
                .position(|p| p.1 == style)
                .unwrap_or(0) as u8;
            // Mixer: the channel strip holds pan (11) and volume (12), from 0 to 1.
            if let Some(ps) = t.descendants().find(|d| {
                d.has_tag_name("Parameters")
                    && d.parent()
                        .map(|p| p.has_tag_name("ChannelStrip"))
                        .unwrap_or(false)
            }) {
                let v: Vec<f32> = text(Some(ps))
                    .split_whitespace()
                    .filter_map(|x| x.parse::<f32>().ok())
                    .collect();
                if v.len() > 12 {
                    tr.pan = (v[11] * 2.0 - 1.0).clamp(-1.0, 1.0);
                    tr.volume = v[12].clamp(0.0, 1.0);
                }
            }
            match text(child(t, "PlaybackState")).as_str() {
                "Solo" => tr.solo = true,
                "Mute" => tr.mute = true,
                _ => {}
            }
            if let Some(ly) = child(t, "Lyrics")
                && let Some(line) = ly.children().find(|c| c.has_tag_name("Line"))
            {
                tr.lyrics = text(child(line, "Text"));
                tr.lyrics_bar = text(child(line, "Offset")).parse::<usize>().unwrap_or(0);
            }
            let col: Vec<u8> = text(child(t, "Color"))
                .split_whitespace()
                .filter_map(|x| x.parse::<u8>().ok())
                .collect();
            if col.len() >= 3 {
                tr.color = [col[0], col[1], col[2]];
            }
            if !tuning.is_empty()
                && (inst_type.contains("bass")
                    || name.to_lowercase().contains("bass")
                    || tuning.iter().min().map(|&m| m < 36).unwrap_or(false) && tuning.len() <= 5)
            {
                tr.instrument = Instrument::Bass;
            }
            // The instrument family named in the file (e.g. ukulele, banjo, cello).
            if let Some(i) = Instrument::ALL
                .iter()
                .copied()
                .find(|&i| crate::gpifwrite::inst_type(i).0.to_lowercase() == inst_type)
                && crate::gpifwrite::inst_type(tr.instrument).0 != crate::gpifwrite::inst_type(i).0
                && i.has_strings_kind() == !tuning.is_empty()
            {
                tr.instrument = i;
            }
            if drum {
                tr.instrument = Instrument::Drums;
            } else if !tuning.is_empty() && !tr.has_strings() {
                // Tuned tracks are played on strings, whatever their sound.
                tr.instrument = if inst_type.contains("bass") {
                    Instrument::Bass
                } else {
                    Instrument::CleanGuitar
                };
            } else if tuning.is_empty() {
                let low = inst_type.as_str();
                tr.instrument =
                    if low.contains("voice") || low.contains("vocal") || low.contains("choir") {
                        Instrument::LeadVocals
                    } else if tr.has_strings() {
                        Instrument::Piano
                    } else {
                        tr.instrument
                    };
            }
            if drum || tuning.is_empty() {
                tr.tuning = tr.instrument.default_tuning();
                tr.frets = 127;
                tr.capo = 0;
                tr.grand_staff = staves > 1 && tr.kind() == InstrumentType::Keyboard;
            }
            infos.push(TrackInfo {
                keep,
                staff_offset,
                chords,
            });
            tracks.push(tr);
            staff_offset += staves;
        }
    }
    if !infos.iter().any(|i| i.keep) {
        return Err("This file has no tracks that this app can show.".into());
    }
    let bars_idx = index(root, "Bars", "Bar");
    let voices_idx = index(root, "Voices", "Voice");
    let beats_idx = index(root, "Beats", "Beat");
    let notes_idx = index(root, "Notes", "Note");
    let rhythms_idx = index(root, "Rhythms", "Rhythm");

    let mut master = vec![];
    let mut num = 4u8;
    let mut den = 4u8;
    let mbs: Vec<Node> = child(root, "MasterBars")
        .map(|m| {
            m.children()
                .filter(|c| c.has_tag_name("MasterBar"))
                .collect()
        })
        .unwrap_or_default();
    for (mi, m) in mbs.iter().enumerate() {
        let time = text(child(*m, "Time"));
        if let Some((a, b)) = time.split_once('/') {
            num = a.trim().parse::<u8>().unwrap_or(num).max(1);
            den = b.trim().parse::<u8>().unwrap_or(den).max(1);
        }
        let mut mb = MasterBar {
            num,
            den,
            ..MasterBar::default()
        };
        if let Some(r) = child(*m, "Repeat") {
            mb.repeat_open = r.attribute("start") == Some("true");
            if r.attribute("end") == Some("true") {
                mb.repeat_close = r
                    .attribute("count")
                    .and_then(|c| c.parse::<u8>().ok())
                    .unwrap_or(2)
                    .max(2);
            }
        }
        if let Some(k) = child(*m, "Key") {
            mb.key = text(child(k, "AccidentalCount"))
                .parse::<i8>()
                .unwrap_or(0)
                .clamp(-7, 7);
            mb.key_minor = text(child(k, "Mode")) == "Minor";
        }
        mb.tempo_ramp = ramps.contains(&mi);
        if let Some(d) = child(*m, "Directions") {
            let tn = text(child(d, "Target"));
            let jn = text(child(d, "Jump"));
            mb.direction_target = (1..=5u8)
                .find(|&c| crate::gpifwrite::direction_target_name(c) == Some(tn.as_str()))
                .unwrap_or(0);
            mb.direction_jump = (1..=10u8)
                .find(|&c| crate::gpifwrite::direction_jump_name(c) == Some(jn.as_str()))
                .unwrap_or(0);
        }
        // Fermatas: offsets as a fraction of a whole note.
        let fermata_ticks: Vec<u32> = child(*m, "Fermatas")
            .map(|f| {
                f.children()
                    .filter(|c| c.has_tag_name("Fermata"))
                    .filter_map(|c| {
                        let o = text(child(c, "Offset"));
                        let (a, b) = o.split_once('/')?;
                        let (a, b) = (
                            a.trim().parse::<u64>().ok()?,
                            b.trim().parse::<u64>().ok()?.max(1),
                        );
                        Some((a * QUARTER as u64 * 4 / b) as u32)
                    })
                    .collect()
            })
            .unwrap_or_default();
        mb.double_bar = child(*m, "DoubleBar").is_some();
        mb.endings = text(child(*m, "AlternateEndings"))
            .split_whitespace()
            .filter_map(|x| x.parse::<u8>().ok())
            .filter(|x| (1..=8).contains(x))
            .fold(0u8, |a, x| a | (1 << (x - 1)));
        let feel = text(child(*m, "TripletFeel"));
        mb.triplet_feel = feel.contains("Triplet");
        if let Some(s) = child(*m, "Section") {
            let t = text(child(s, "Text"));
            let l = text(child(s, "Letter"));
            let name = if !t.is_empty() { t } else { l };
            if !name.is_empty() {
                mb.marker = Some(name);
            }
        }
        if let Some(&(_, t)) = tempos.iter().find(|(b, _)| *b == mi && mi > 0) {
            mb.tempo = Some(t);
        }
        let bar_ids: Vec<String> = text(child(*m, "Bars"))
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();
        for (ti, info) in infos.iter().enumerate() {
            let mut bar = Bar::new(vec![]);
            if info.keep {
                let nstr = tracks[ti].tuning.len();
                let virt = !tracks[ti].has_strings();
                if let Some(bn) = bar_ids
                    .get(info.staff_offset)
                    .and_then(|id| bars_idx.get(id))
                {
                    let voice_ids: Vec<String> = text(child(*bn, "Voices"))
                        .split_whitespace()
                        .take(VOICES)
                        .map(|s| s.to_string())
                        .collect();
                    for (vk, vid) in voice_ids.iter().enumerate() {
                        let Some(vn) = voices_idx.get(vid) else {
                            continue;
                        };
                        let mut beats = vec![];
                        // Grace notes come as short beats just before the note they lead into.
                        let mut grace: Vec<Note> = vec![];
                        for bid in text(child(*vn, "Beats")).split_whitespace() {
                            if let Some(bt) = beats_idx.get(bid) {
                                if child(*bt, "GraceNotes").is_some() {
                                    grace = read_beat(
                                        *bt,
                                        &rhythms_idx,
                                        &notes_idx,
                                        nstr,
                                        &info.chords,
                                        virt,
                                    )
                                    .notes;
                                    continue;
                                }
                                let mut b = read_beat(
                                    *bt,
                                    &rhythms_idx,
                                    &notes_idx,
                                    nstr,
                                    &info.chords,
                                    virt,
                                );
                                for g in grace.drain(..) {
                                    let target = if virt {
                                        b.notes.first_mut()
                                    } else {
                                        b.note_on_mut(g.string)
                                    };
                                    if let Some(n) = target {
                                        n.grace = Some(g.fret);
                                    }
                                }
                                beats.push(b);
                            }
                        }
                        if vk == 0 {
                            bar.beats = beats;
                        } else if !beats.is_empty() {
                            bar.pad_voices();
                            bar.voices[vk - 1] = beats;
                        }
                    }
                }
                if tracks[ti].grand()
                    && let Some(bn) = bar_ids
                        .get(info.staff_offset + 1)
                        .and_then(|id| bars_idx.get(id))
                {
                    let vid = text(child(*bn, "Voices"))
                        .split_whitespace()
                        .next()
                        .map(|x| x.to_string());
                    if let Some(vn) = vid.and_then(|v| voices_idx.get(&v).copied()) {
                        let beats: Vec<Beat> = text(child(vn, "Beats"))
                            .split_whitespace()
                            .filter_map(|bid| beats_idx.get(bid))
                            .filter(|bt| child(**bt, "GraceNotes").is_none())
                            .map(|bt| {
                                read_beat(*bt, &rhythms_idx, &notes_idx, nstr, &info.chords, virt)
                            })
                            .collect();
                        let same = beats.len() == bar.beats.len()
                            && beats
                                .iter()
                                .zip(bar.beats.iter())
                                .all(|(a, b)| a.ticks() == b.ticks());
                        if same {
                            for (low, up) in beats.into_iter().zip(bar.beats.iter_mut()) {
                                for mut n in low.notes {
                                    if up.notes.len() < nstr {
                                        n.string = up.notes.len() as u8;
                                        up.notes.push(n);
                                    }
                                }
                            }
                        } else if beats.iter().any(|b| !b.is_rest()) {
                            bar.pad_voices();
                            if let Some(slot) = (0..bar.voices.len())
                                .find(|&k| bar.voices[k].iter().all(|b| b.is_rest()))
                            {
                                bar.voices[slot] = beats;
                            }
                        }
                    }
                }
            }
            if bar.beats.is_empty() {
                bar.beats = vec![Beat::rest(Dur::Whole)];
            }
            if ti == 0 {
                let mut tick = 0;
                for b in bar.beats.iter_mut() {
                    if fermata_ticks.contains(&tick) {
                        b.fermata = true;
                    }
                    tick += b.ticks();
                }
            }
            tracks[ti].bars.push(bar);
        }
        master.push(mb);
    }
    if let Some(&(_, t)) = tempos.iter().find(|(b, _)| *b == 0) {
        song.tempo = t;
    } else if let Some(&(_, t)) = tempos.first() {
        song.tempo = t;
    }
    song.master = master;
    song.tracks = tracks
        .into_iter()
        .zip(infos.iter())
        .filter(|(_, i)| i.keep)
        .map(|(t, _)| t)
        .collect();
    song.normalize();
    Ok(song)
}

fn read_beat(
    bt: Node,
    rhythms: &HashMap<String, Node>,
    notes: &HashMap<String, Node>,
    nstr: usize,
    chords: &HashMap<String, String>,
    virt: bool,
) -> Beat {
    let mut beat = Beat::rest(Dur::Quarter);
    if let Some(r) = child(bt, "Rhythm")
        .and_then(|r| r.attribute("ref"))
        .and_then(|id| rhythms.get(id))
    {
        beat.dur = match text(child(*r, "NoteValue")).as_str() {
            "Whole" => Dur::Whole,
            "Half" => Dur::Half,
            "Eighth" => Dur::Eighth,
            "16th" => Dur::Sixteenth,
            "32nd" => Dur::ThirtySecond,
            "64th" | "128th" | "256th" => Dur::SixtyFourth,
            _ => Dur::Quarter,
        };
        if let Some(d) = child(*r, "AugmentationDot") {
            match d.attribute("count").unwrap_or("1") {
                "0" => {}
                "2" => beat.double_dot = true,
                _ => beat.dotted = true,
            }
        }
        if let Some(t) = child(*r, "PrimaryTuplet") {
            beat.tuplet = t
                .attribute("num")
                .and_then(|x| x.parse::<u8>().ok())
                .filter(|n| matches!(n, 3 | 5 | 6 | 7 | 9))
                .unwrap_or(0);
        }
    }
    beat.dynamic = match text(child(bt, "Dynamic")).as_str() {
        "PPP" => Dynamic::Ppp,
        "PP" => Dynamic::Pp,
        "P" => Dynamic::P,
        "MP" => Dynamic::Mp,
        "F" => Dynamic::F,
        "FF" => Dynamic::Ff,
        "FFF" => Dynamic::Fff,
        _ => Dynamic::Mf,
    };
    beat.hairpin = match text(child(bt, "Hairpin")).as_str() {
        "Crescendo" => 1,
        "Decrescendo" | "Diminuendo" => 2,
        _ => 0,
    };
    if let Some(l) = child(bt, "Lyrics").and_then(|l| child(l, "Line")) {
        let t = text(Some(l));
        if !t.trim().is_empty() {
            beat.lyric = Some(t);
        }
    }
    let ft = text(child(bt, "FreeText"));
    if !ft.is_empty() {
        beat.text = Some(ft);
    }
    let ch = text(child(bt, "Chord"));
    if let Some(name) = chords.get(&ch)
        && !name.trim().is_empty()
    {
        beat.chord = Some(name.clone());
    }
    let bp = props(bt);
    if let Some(b) = bp.get("Brush") {
        beat.stroke = match text(child(*b, "Direction")).as_str() {
            "Up" => Stroke::Up,
            "Down" => Stroke::Down,
            _ => Stroke::None,
        };
    }
    let beat_vib = child(bt, "Vibrato").is_some();
    beat.ottava = match text(child(bt, "Ottavia")).as_str() {
        "8va" => 1,
        "15ma" => 2,
        "8vb" => -1,
        "15mb" => -2,
        _ => 0,
    };
    beat.tremolo = child(bt, "Tremolo").is_some();
    beat.fade_in = text(child(bt, "Fadding")) == "FadeIn";
    if bp.contains_key("Tapped") {
        beat.technique = 1;
    } else if bp.contains_key("Slapped") {
        beat.technique = 2;
    } else if bp.contains_key("Popped") {
        beat.technique = 3;
    }
    if let Some(ps) = bp.get("PickStroke") {
        beat.pick = match text(child(*ps, "Direction")).as_str() {
            "Down" => 1,
            "Up" => 2,
            _ => 0,
        };
    }
    for nid in text(child(bt, "Notes")).split_whitespace() {
        let Some(nn) = notes.get(nid) else { continue };
        let p = props(*nn);
        let mut n = if virt {
            // Instruments without strings: the note's pitch, on the next free virtual string.
            let pitch = p
                .get("Midi")
                .map(|x| text(child(*x, "Number")))
                .and_then(|x| x.parse::<i32>().ok())
                .or_else(|| {
                    let cp = p.get("ConcertPitch").or_else(|| p.get("TransposedPitch"))?;
                    let pt = child(*cp, "Pitch")?;
                    let step = match text(child(pt, "Step")).as_str() {
                        "C" => 0,
                        "D" => 2,
                        "E" => 4,
                        "F" => 5,
                        "G" => 7,
                        "A" => 9,
                        "B" => 11,
                        _ => return None,
                    };
                    let acc = match text(child(pt, "Accidental")).as_str() {
                        "#" => 1,
                        "b" => -1,
                        "##" | "x" => 2,
                        "bb" => -2,
                        _ => 0,
                    };
                    let oct = text(child(pt, "Octave")).parse::<i32>().ok()?;
                    Some((oct + 1) * 12 + step + acc)
                });
            let Some(pitch) = pitch else { continue };
            if beat.notes.len() >= nstr {
                continue;
            }
            Note::new(beat.notes.len() as u8, pitch.clamp(0, 127) as u8)
        } else {
            let Some(s) = p
                .get("String")
                .and_then(|x| text(child(*x, "String")).parse::<usize>().ok())
            else {
                continue;
            };
            let Some(f) = p
                .get("Fret")
                .and_then(|x| text(child(*x, "Fret")).parse::<i32>().ok())
            else {
                continue;
            };
            if s >= nstr {
                continue;
            }
            Note::new((nstr - 1 - s) as u8, f.clamp(0, 36) as u8)
        };
        n.dead = p.contains_key("Muted");
        n.palm_mute = p.contains_key("PalmMuted");
        n.hammer = p.contains_key("HopoOrigin");
        let htype = p
            .get("HarmonicType")
            .map(|x| text(child(*x, "HType")))
            .unwrap_or_default();
        if htype == "Pinch" || htype == "Artificial" {
            n.pinch_harmonic = true;
        } else {
            n.harmonic = p.contains_key("HarmonicType") || p.contains_key("HarmonicFret");
        }
        if let Some(sl) = p.get("Slide") {
            let flags = text(child(*sl, "Flags")).parse::<i32>().unwrap_or(0);
            n.slide = flags & 3 != 0;
            n.slide_out = flags & 12 != 0;
            n.slide_in = flags & 48 != 0;
        }
        if p.contains_key("Bended") {
            let mut v = 0.0f32;
            for k in ["BendDestinationValue", "BendMiddleValue"] {
                if let Some(x) = p
                    .get(k)
                    .and_then(|x| text(child(*x, "Float")).parse::<f32>().ok())
                {
                    v = v.max(x);
                }
            }
            n.bend = (v / 25.0).round().clamp(0.0, 12.0) as u8;
        }
        if let Some(t) = child(*nn, "Tie") {
            n.tie = t.attribute("destination") == Some("true");
        }
        n.vibrato = child(*nn, "Vibrato").is_some() || beat_vib;
        n.wide_vibrato = text(child(*nn, "Vibrato")) == "Wide";
        n.heavy_accent = text(child(*nn, "Accent")).parse::<i32>().unwrap_or(0) & 4 != 0;
        n.let_ring = child(*nn, "LetRing").is_some();
        n.ghost = child(*nn, "AntiAccent").is_some();
        let acc = text(child(*nn, "Accent")).parse::<i32>().unwrap_or(0);
        n.staccato = acc & 1 != 0;
        n.accent = acc & (4 | 8) != 0;
        if beat.note_on(n.string).is_none() {
            beat.notes.push(n);
        }
    }
    beat.notes.sort_by_key(|n| n.string);
    beat
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<GPIF>
 <Score><Title><![CDATA[Test Song]]></Title><Artist><![CDATA[Someone]]></Artist></Score>
 <MasterTrack><Tracks>0</Tracks><Automations><Automation><Type>Tempo</Type><Bar>0</Bar><Value>100 2</Value></Automation></Automations></MasterTrack>
 <Tracks>
  <Track id="0"><Name><![CDATA[Lead]]></Name><Color>255 0 0</Color>
   <Staves><Staff><Properties><Property name="Tuning"><Pitches>40 45 50 55 59 64</Pitches></Property></Properties></Staff></Staves>
  </Track>
  <Track id="1"><Name><![CDATA[Drums]]></Name><InstrumentSet><Type>drumKit</Type></InstrumentSet><Staves><Staff/></Staves></Track>
 </Tracks>
 <MasterBars>
  <MasterBar><Time>4/4</Time><Repeat start="true" end="true" count="2"/><Section><Text><![CDATA[Intro]]></Text></Section><Bars>0 1</Bars></MasterBar>
 </MasterBars>
 <Bars><Bar id="0"><Voices>0 -1 -1 -1</Voices></Bar><Bar id="1"><Voices>-1 -1 -1 -1</Voices></Bar></Bars>
 <Voices><Voice id="0"><Beats>0 1</Beats></Voice></Voices>
 <Beats>
  <Beat id="0"><Rhythm ref="0"/><Notes>0 1</Notes><Dynamic>F</Dynamic></Beat>
  <Beat id="1"><Rhythm ref="0"/></Beat>
 </Beats>
 <Notes>
  <Note id="0"><Properties><Property name="String"><String>0</String></Property><Property name="Fret"><Fret>3</Fret></Property><Property name="PalmMuted"><Enable/></Property></Properties></Note>
  <Note id="1"><LetRing/><Properties><Property name="String"><String>5</String></Property><Property name="Fret"><Fret>12</Fret></Property><Property name="Bended"><Enable/></Property><Property name="BendDestinationValue"><Float>100</Float></Property></Properties></Note>
 </Notes>
 <Rhythms><Rhythm id="0"><NoteValue>Half</NoteValue></Rhythm></Rhythms>
</GPIF>"#;

    #[test]
    fn reads_gpif() {
        let s = parse_gpif(SAMPLE).unwrap();
        assert_eq!(s.title, "Test Song");
        assert_eq!(s.tempo, 100);
        // The drum track is kept, as a drum kit on the percussion staff.
        assert_eq!(s.tracks.len(), 2);
        assert_eq!(s.tracks[1].instrument, Instrument::Drums);
        assert!(!s.tracks[1].has_tab());
        assert_eq!(s.tracks[0].tuning, STANDARD_GUITAR.to_vec());
        assert_eq!(s.master[0].marker.as_deref(), Some("Intro"));
        assert_eq!(s.master[0].repeat_close, 2);
        let b = &s.tracks[0].bars[0].beats;
        assert_eq!(b.len(), 2);
        assert_eq!(b[0].dur, Dur::Half);
        assert_eq!(b[0].dynamic, Dynamic::F);
        let low = b[0].note_on(5).unwrap();
        assert_eq!(low.fret, 3);
        assert!(low.palm_mute);
        let high = b[0].note_on(0).unwrap();
        assert_eq!(high.fret, 12);
        assert_eq!(high.bend, 4);
        assert!(high.let_ring);
        assert!(b[1].is_rest());
    }

    #[test]
    fn bcfz_literal_and_copy() {
        // Hand-built stream: literals "ab" then a back-reference copying 2 bytes.
        let mut bits: Vec<u8> = vec![];
        let push = |bits: &mut Vec<u8>, v: u32, n: u32, rev: bool| {
            for i in 0..n {
                let b = if rev {
                    (v >> i) & 1
                } else {
                    (v >> (n - 1 - i)) & 1
                };
                bits.push(b as u8);
            }
        };
        push(&mut bits, 0, 1, false);
        push(&mut bits, 2, 2, true);
        push(&mut bits, b'a' as u32, 8, false);
        push(&mut bits, b'b' as u32, 8, false);
        push(&mut bits, 1, 1, false);
        push(&mut bits, 2, 4, false);
        push(&mut bits, 2, 2, true);
        push(&mut bits, 2, 2, true);
        let mut bytes = vec![];
        for ch in bits.chunks(8) {
            let mut b = 0u8;
            for (i, &x) in ch.iter().enumerate() {
                b |= x << (7 - i);
            }
            bytes.push(b);
        }
        let mut data = 4i32.to_le_bytes().to_vec();
        data.extend(bytes);
        assert_eq!(decompress_bcfz(&data).unwrap(), b"abab");
    }
}
