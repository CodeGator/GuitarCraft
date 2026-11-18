//! MusicXML, the standard exchange format for sheet music (File > Import and
//! Export > MusicXML). Written as score-partwise MusicXML 4.0: one part per
//! track, with the time and key signatures, clefs, tempo, sections, repeats
//! and endings, chord names, texts, dynamics, lyrics, up to four voices, ties,
//! tuplets and, for stringed instruments, the tuning and each note's string and
//! fret, so programs that show tablature put the notes where they were.
//! Read from .musicxml/.xml files and compressed .mxl files.

use crate::model::*;
use roxmltree::Node;
use std::collections::BTreeMap;
use std::fmt::Write;

const SHARPS: [(&str, i32); 12] = [("C", 0), ("C", 1), ("D", 0), ("D", 1), ("E", 0), ("F", 0), ("F", 1), ("G", 0), ("G", 1), ("A", 0), ("A", 1), ("B", 0)];
const FLATS: [(&str, i32); 12] = [("C", 0), ("D", -1), ("D", 0), ("E", -1), ("E", 0), ("F", 0), ("G", -1), ("G", 0), ("A", -1), ("A", 0), ("B", -1), ("B", 0)];

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// A MIDI note as step, alter and octave.
fn spell(midi: u8, flats: bool) -> (&'static str, i32, i32) {
    let (step, alter) = if flats { FLATS[midi as usize % 12] } else { SHARPS[midi as usize % 12] };
    (step, alter, midi as i32 / 12 - 1)
}

fn type_name(d: Dur) -> &'static str {
    match d {
        Dur::Whole => "whole",
        Dur::Half => "half",
        Dur::Quarter => "quarter",
        Dur::Eighth => "eighth",
        Dur::Sixteenth => "16th",
        Dur::ThirtySecond => "32nd",
        Dur::SixtyFourth => "64th",
    }
}

fn type_from_name(s: &str) -> Option<Dur> {
    Some(match s {
        "breve" | "long" | "maxima" | "whole" => Dur::Whole,
        "half" => Dur::Half,
        "quarter" => Dur::Quarter,
        "eighth" => Dur::Eighth,
        "16th" => Dur::Sixteenth,
        "32nd" => Dur::ThirtySecond,
        "64th" | "128th" | "256th" => Dur::SixtyFourth,
        _ => return None,
    })
}

/// Tuplets as (actual notes, normal notes).
fn tuplet_ratio(t: u8) -> Option<(u8, u8)> {
    match t {
        3 => Some((3, 2)),
        5 => Some((5, 4)),
        6 => Some((6, 4)),
        7 => Some((7, 4)),
        9 => Some((9, 8)),
        _ => None,
    }
}

fn dynamic_name(d: Dynamic) -> &'static str {
    match d {
        Dynamic::Ppp => "ppp",
        Dynamic::Pp => "pp",
        Dynamic::P => "p",
        Dynamic::Mp => "mp",
        Dynamic::Mf => "mf",
        Dynamic::F => "f",
        Dynamic::Ff => "ff",
        Dynamic::Fff => "fff",
    }
}

fn dynamic_from_name(s: &str) -> Option<Dynamic> {
    Some(match s {
        "ppp" | "pppp" => Dynamic::Ppp,
        "pp" => Dynamic::Pp,
        "p" => Dynamic::P,
        "mp" => Dynamic::Mp,
        "mf" => Dynamic::Mf,
        "f" | "sf" | "sfz" | "fp" => Dynamic::F,
        "ff" => Dynamic::Ff,
        "fff" | "ffff" => Dynamic::Fff,
        _ => return None,
    })
}

/// Chord kinds MusicXML names, with the usual way of writing them.
const KINDS: [(&str, &str); 20] = [
    ("major", ""),
    ("minor", "m"),
    ("augmented", "aug"),
    ("diminished", "dim"),
    ("dominant", "7"),
    ("major-seventh", "maj7"),
    ("minor-seventh", "m7"),
    ("diminished-seventh", "dim7"),
    ("augmented-seventh", "aug7"),
    ("half-diminished", "m7b5"),
    ("major-minor", "m(maj7)"),
    ("major-sixth", "6"),
    ("minor-sixth", "m6"),
    ("dominant-ninth", "9"),
    ("major-ninth", "maj9"),
    ("minor-ninth", "m9"),
    ("dominant-11th", "11"),
    ("dominant-13th", "13"),
    ("suspended-second", "sus2"),
    ("suspended-fourth", "sus4"),
];

/// A chord name as MusicXML harmony.
fn harmony_xml(name: &str) -> Option<String> {
    let (root, rest, bass) = crate::chart::split_chord(name)?;
    let flats = name.chars().nth(1) == Some('b');
    let (step, alter, _) = spell(root as u8 + 60, flats);
    let kind = if rest == "5" {
        "power"
    } else {
        KINDS.iter().find(|k| k.1 == rest).map(|k| k.0).unwrap_or("major")
    };
    let mut x = format!("<harmony><root><root-step>{step}</root-step>");
    if alter != 0 {
        let _ = write!(x, "<root-alter>{alter}</root-alter>");
    }
    let _ = write!(x, "</root><kind text=\"{}\">{kind}</kind>", esc(&rest));
    if let Some(b) = bass {
        let bflats = name.rsplit('/').next().map(|s| s.chars().nth(1) == Some('b')).unwrap_or(false);
        let (bs, ba, _) = spell(b as u8 + 60, bflats);
        let _ = write!(x, "<bass><bass-step>{bs}</bass-step>");
        if ba != 0 {
            let _ = write!(x, "<bass-alter>{ba}</bass-alter>");
        }
        x.push_str("</bass>");
    }
    x.push_str("</harmony>");
    Some(x)
}

fn clef_xml(tr: &Track) -> &'static str {
    if tr.instrument == Instrument::Drums {
        return "<clef><sign>percussion</sign></clef>";
    }
    match tr.clef() {
        Clef::Treble8 => "<clef><sign>G</sign><line>2</line><clef-octave-change>-1</clef-octave-change></clef>",
        Clef::Bass8 => "<clef><sign>F</sign><line>4</line><clef-octave-change>-1</clef-octave-change></clef>",
        Clef::Treble => "<clef><sign>G</sign><line>2</line></clef>",
        Clef::Bass => "<clef><sign>F</sign><line>4</line></clef>",
        Clef::Percussion => "<clef><sign>percussion</sign></clef>",
    }
}

/// The beat after beat `j` of voice `v` in bar `bi` (in the next bar when it's the last).
fn next_beat(tr: &Track, bi: usize, v: usize, j: usize) -> Option<&Beat> {
    let bar = tr.bars.get(bi)?;
    match bar.voice(v).get(j + 1) {
        Some(b) => Some(b),
        None => tr.bars.get(bi + 1).and_then(|b| b.voice(v).first()),
    }
}

/// The song as MusicXML.
pub fn export_musicxml(song: &Song) -> String {
    let mut x = String::new();
    x.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\"?>\n");
    x.push_str("<!DOCTYPE score-partwise PUBLIC \"-//Recordare//DTD MusicXML 4.0 Partwise//EN\" \"http://www.musicxml.org/dtds/partwise.dtd\">\n");
    x.push_str("<score-partwise version=\"4.0\">\n");
    let _ = writeln!(x, "  <work><work-title>{}</work-title></work>", esc(&song.title));
    if !song.subtitle.is_empty() {
        let _ = writeln!(x, "  <movement-title>{}</movement-title>", esc(&song.subtitle));
    }
    x.push_str("  <identification>\n");
    for (kind, who) in [("composer", &song.music_by), ("lyricist", &song.words), ("arranger", &song.tabbed_by)] {
        if !who.is_empty() {
            let _ = writeln!(x, "    <creator type=\"{kind}\">{}</creator>", esc(who));
        }
    }
    if !song.artist.is_empty() {
        let _ = writeln!(x, "    <creator type=\"artist\">{}</creator>", esc(&song.artist));
    }
    if !song.copyright.is_empty() {
        let _ = writeln!(x, "    <rights>{}</rights>", esc(&song.copyright));
    }
    x.push_str("    <encoding><software>GuitarCraft</software></encoding>\n  </identification>\n");
    // The parts.
    x.push_str("  <part-list>\n");
    for (k, tr) in song.tracks.iter().enumerate() {
        let id = format!("P{}", k + 1);
        let drums = tr.instrument == Instrument::Drums;
        let ch = if drums { 10 } else if k >= 9 { k + 2 } else { k + 1 }.min(16);
        let _ = writeln!(x, "    <score-part id=\"{id}\"><part-name>{}</part-name>", esc(&tr.name));
        if !tr.short_name.is_empty() {
            let _ = writeln!(x, "      <part-abbreviation>{}</part-abbreviation>", esc(&tr.short_name));
        }
        if drums {
            // One instrument per drum sound, so each note keeps its sound.
            let mut keys: Vec<u8> = tr.bars.iter().flat_map(|b| (0..VOICES).flat_map(move |v| b.voice(v).iter())).flat_map(|b| b.notes.iter().map(|n| tr.pitch(n))).collect();
            keys.sort();
            keys.dedup();
            for p in &keys {
                let _ = writeln!(x, "      <score-instrument id=\"{id}-I{p}\"><instrument-name>Drum {p}</instrument-name></score-instrument>");
            }
            for p in &keys {
                let _ = writeln!(x, "      <midi-instrument id=\"{id}-I{p}\"><midi-channel>10</midi-channel><midi-unpitched>{}</midi-unpitched></midi-instrument>", p + 1);
            }
        } else {
            let _ = writeln!(x, "      <score-instrument id=\"{id}-I1\"><instrument-name>{}</instrument-name></score-instrument>", esc(tr.instrument.name()));
            let _ = writeln!(
                x,
                "      <midi-instrument id=\"{id}-I1\"><midi-channel>{ch}</midi-channel><midi-program>{}</midi-program><volume>{:.0}</volume><pan>{:.0}</pan></midi-instrument>",
                tr.instrument.midi_program() as u32 + 1,
                tr.volume.clamp(0.0, 1.0) * 100.0,
                tr.pan.clamp(-1.0, 1.0) * 90.0
            );
        }
        x.push_str("    </score-part>\n");
    }
    x.push_str("  </part-list>\n");
    for (k, tr) in song.tracks.iter().enumerate() {
        part_xml(&mut x, song, k, tr);
    }
    x.push_str("</score-partwise>\n");
    x
}

fn part_xml(x: &mut String, song: &Song, k: usize, tr: &Track) {
    let id = format!("P{}", k + 1);
    let drums = tr.instrument == Instrument::Drums;
    let _ = writeln!(x, "  <part id=\"{id}\">");
    let mut last_dyn: Option<Dynamic> = None;
    let n = song.bar_count();
    for bi in 0..n {
        let mb = &song.master[bi];
        let prev = if bi > 0 { song.master.get(bi - 1) } else { None };
        let _ = writeln!(x, "    <measure number=\"{}\">", bi + 1);
        // Left bar line: a repeat opening, or the start of an ending.
        let ending_start = mb.endings != 0 && prev.map(|p| p.endings != mb.endings).unwrap_or(true);
        if mb.repeat_open || ending_start {
            x.push_str("      <barline location=\"left\">");
            if mb.repeat_open {
                x.push_str("<bar-style>heavy-light</bar-style>");
            }
            if ending_start {
                let nums: Vec<String> = (0..8).filter(|e| mb.endings & (1 << e) != 0).map(|e| (e + 1).to_string()).collect();
                let _ = write!(x, "<ending number=\"{}\" type=\"start\"/>", nums.join(","));
            }
            if mb.repeat_open {
                x.push_str("<repeat direction=\"forward\"/>");
            }
            x.push_str("</barline>\n");
        }
        // Attributes when they change.
        let sig_changed = prev.map(|p| (p.num, p.den) != (mb.num, mb.den)).unwrap_or(true);
        let key_changed = prev.map(|p| (p.key, p.key_minor) != (mb.key, mb.key_minor)).unwrap_or(true);
        if bi == 0 || sig_changed || key_changed {
            x.push_str("      <attributes>");
            if bi == 0 {
                let _ = write!(x, "<divisions>{QUARTER}</divisions>");
            }
            if key_changed {
                let _ = write!(x, "<key><fifths>{}</fifths><mode>{}</mode></key>", mb.key, if mb.key_minor { "minor" } else { "major" });
            }
            if sig_changed {
                let _ = write!(x, "<time><beats>{}</beats><beat-type>{}</beat-type></time>", mb.num, mb.den);
            }
            if bi == 0 {
                x.push_str(clef_xml(tr));
                if tr.has_strings() {
                    let _ = write!(x, "<staff-details><staff-lines>{}</staff-lines>", tr.strings());
                    for (line, s) in (0..tr.strings()).rev().enumerate() {
                        let (step, alter, oct) = spell(tr.tuning[s], tr.tuning_flat);
                        let _ = write!(x, "<staff-tuning line=\"{}\"><tuning-step>{step}</tuning-step>", line + 1);
                        if alter != 0 {
                            let _ = write!(x, "<tuning-alter>{alter}</tuning-alter>");
                        }
                        let _ = write!(x, "<tuning-octave>{oct}</tuning-octave></staff-tuning>");
                    }
                    if tr.capo > 0 {
                        let _ = write!(x, "<capo>{}</capo>", tr.capo);
                    }
                    x.push_str("</staff-details>");
                }
            }
            x.push_str("</attributes>\n");
        }
        // Tempo and section name (on the first part only).
        let tempo = if bi == 0 { Some(song.tempo_points().first().map(|p| p.1).unwrap_or(song.tempo)) } else { mb.tempo };
        if k == 0 {
            if let Some(m) = &mb.marker {
                let _ = writeln!(x, "      <direction placement=\"above\"><direction-type><rehearsal>{}</rehearsal></direction-type></direction>", esc(m));
            }
            if let Some(t) = tempo {
                let _ = writeln!(
                    x,
                    "      <direction placement=\"above\"><direction-type><metronome><beat-unit>quarter</beat-unit><per-minute>{t}</per-minute></metronome></direction-type><sound tempo=\"{t}\"/></direction>"
                );
            }
        }
        let flats = mb.key < 0;
        let bar = &tr.bars[bi];
        let mut written = 0u32;
        for v in 0..VOICES {
            if v > 0 && !bar.voice_used(v) {
                continue;
            }
            if written > 0 {
                let _ = writeln!(x, "      <backup><duration>{written}</duration></backup>");
            }
            written = 0;
            let beats = bar.voice(v);
            let only_rest = beats.len() == 1 && beats[0].is_rest();
            for (j, beat) in beats.iter().enumerate() {
                if v == 0 {
                    if let Some(h) = beat.chord.as_deref().and_then(harmony_xml) {
                        let _ = writeln!(x, "      {h}");
                    }
                    if let Some(t) = &beat.text {
                        let _ = writeln!(x, "      <direction placement=\"above\"><direction-type><words>{}</words></direction-type></direction>", esc(t));
                    }
                    if !beat.is_rest() && last_dyn != Some(beat.dynamic) {
                        let _ = writeln!(x, "      <direction placement=\"below\"><direction-type><dynamics><{}/></dynamics></direction-type></direction>", dynamic_name(beat.dynamic));
                        last_dyn = Some(beat.dynamic);
                    }
                }
                let mut dur = beat.ticks();
                if beat.is_rest() {
                    if only_rest {
                        dur = mb.ticks();
                        let _ = writeln!(x, "      <note><rest measure=\"yes\"/><duration>{dur}</duration><voice>{}</voice></note>", v + 1);
                    } else {
                        let _ = writeln!(x, "      <note><rest/><duration>{dur}</duration><voice>{}</voice>{}</note>", v + 1, rhythm_xml(beat));
                    }
                    written += dur;
                    continue;
                }
                let next = next_beat(tr, bi, v, j);
                for (ni, note) in beat.notes.iter().enumerate() {
                    let p = tr.pitch(note);
                    let (step, alter, oct) = spell(p, flats);
                    x.push_str("      <note>");
                    if ni > 0 {
                        x.push_str("<chord/>");
                    }
                    if drums {
                        let _ = write!(x, "<unpitched><display-step>{step}</display-step><display-octave>{oct}</display-octave></unpitched>");
                    } else {
                        let _ = write!(x, "<pitch><step>{step}</step>");
                        if alter != 0 {
                            let _ = write!(x, "<alter>{alter}</alter>");
                        }
                        let _ = write!(x, "<octave>{oct}</octave></pitch>");
                    }
                    let _ = write!(x, "<duration>{dur}</duration>");
                    let tie_start = next.and_then(|b| b.note_on(note.string)).map(|n| n.tie).unwrap_or(false);
                    if note.tie {
                        x.push_str("<tie type=\"stop\"/>");
                    }
                    if tie_start {
                        x.push_str("<tie type=\"start\"/>");
                    }
                    if drums {
                        let _ = write!(x, "<instrument id=\"{id}-I{p}\"/>");
                    }
                    let _ = write!(x, "<voice>{}</voice>{}", v + 1, rhythm_xml(beat));
                    if note.dead {
                        x.push_str("<notehead>x</notehead>");
                    }
                    // Ties, string and fret, and the marks most programs read.
                    let mut nt = String::new();
                    if note.tie {
                        nt.push_str("<tied type=\"stop\"/>");
                    }
                    if tie_start {
                        nt.push_str("<tied type=\"start\"/>");
                    }
                    let mut arts = String::new();
                    if note.staccato {
                        arts.push_str("<staccato/>");
                    }
                    if note.accent {
                        arts.push_str("<accent/>");
                    }
                    if note.heavy_accent {
                        arts.push_str("<strong-accent/>");
                    }
                    if !arts.is_empty() {
                        let _ = write!(nt, "<articulations>{arts}</articulations>");
                    }
                    if tr.has_strings() && !drums {
                        nt.push_str("<technical>");
                        if note.hammer {
                            let up = next.and_then(|b| b.note_on(note.string)).map(|n| tr.pitch(n) >= p).unwrap_or(true);
                            nt.push_str(if up { "<hammer-on type=\"start\">H</hammer-on>" } else { "<pull-off type=\"start\">P</pull-off>" });
                        }
                        if note.harmonic {
                            nt.push_str("<harmonic><natural/></harmonic>");
                        }
                        if note.bend > 0 {
                            let _ = write!(nt, "<bend><bend-alter>{}</bend-alter></bend>", note.bend as f32 / 2.0);
                        }
                        let _ = write!(nt, "<string>{}</string><fret>{}</fret></technical>", note.string as usize + 1, note.fret);
                    }
                    if beat.fermata && ni == 0 {
                        nt.push_str("<fermata/>");
                    }
                    if !nt.is_empty() {
                        let _ = write!(x, "<notations>{nt}</notations>");
                    }
                    if ni == 0
                        && let Some(l) = &beat.lyric
                    {
                        let _ = write!(x, "<lyric number=\"1\"><syllabic>single</syllabic><text>{}</text></lyric>", esc(l));
                    }
                    x.push_str("</note>\n");
                }
                written += dur;
            }
        }
        // Right bar line: a repeat closing, the end of an ending, or a double bar.
        let next = song.master.get(bi + 1);
        let ending_stop = mb.endings != 0 && next.map(|nx| nx.endings != mb.endings).unwrap_or(true);
        if mb.repeat_close > 0 || ending_stop || mb.double_bar {
            x.push_str("      <barline location=\"right\">");
            if mb.repeat_close > 0 {
                x.push_str("<bar-style>light-heavy</bar-style>");
            } else if mb.double_bar {
                x.push_str("<bar-style>light-light</bar-style>");
            }
            if ending_stop {
                let nums: Vec<String> = (0..8).filter(|e| mb.endings & (1 << e) != 0).map(|e| (e + 1).to_string()).collect();
                let _ = write!(x, "<ending number=\"{}\" type=\"{}\"/>", nums.join(","), if mb.repeat_close > 0 { "stop" } else { "discontinue" });
            }
            if mb.repeat_close > 0 {
                let _ = write!(x, "<repeat direction=\"backward\" times=\"{}\"/>", mb.repeat_close.max(2));
            }
            x.push_str("</barline>\n");
        }
        x.push_str("    </measure>\n");
    }
    x.push_str("  </part>\n");
}

/// A beat's note value: type, dots and tuplet.
fn rhythm_xml(b: &Beat) -> String {
    let mut s = format!("<type>{}</type>", type_name(b.dur));
    if b.double_dot {
        s.push_str("<dot/><dot/>");
    } else if b.dotted {
        s.push_str("<dot/>");
    }
    if let Some((a, n)) = tuplet_ratio(b.tuplet) {
        let _ = write!(s, "<time-modification><actual-notes>{a}</actual-notes><normal-notes>{n}</normal-notes></time-modification>");
    }
    s
}

// ---------------------------------------------------------------- import

fn child<'a, 'i>(n: Node<'a, 'i>, name: &str) -> Option<Node<'a, 'i>> {
    n.children().find(|c| c.is_element() && c.tag_name().name() == name)
}

fn children<'a, 'i>(n: Node<'a, 'i>, name: &'static str) -> impl Iterator<Item = Node<'a, 'i>> {
    n.children().filter(move |c| c.is_element() && c.tag_name().name() == name)
}

fn text_of(n: Option<Node>) -> String {
    n.and_then(|n| n.text()).unwrap_or("").trim().to_string()
}

fn num<T: std::str::FromStr>(n: Option<Node>) -> Option<T> {
    text_of(n).parse::<f64>().ok().and_then(|v| v.round().to_string().parse::<T>().ok())
}

fn step_pc(s: &str) -> Option<i32> {
    Some(match s.trim() {
        "C" => 0,
        "D" => 2,
        "E" => 4,
        "F" => 5,
        "G" => 7,
        "A" => 9,
        "B" => 11,
        _ => return None,
    })
}

fn midi_of(step: &str, alter: f64, octave: i32) -> Option<u8> {
    let p = (octave + 1) * 12 + step_pc(step)? + alter.round() as i32;
    (0..=127).contains(&p).then_some(p as u8)
}

/// A note being read.
#[derive(Clone, Default)]
struct NoteIn {
    pitch: u8,
    string: Option<u8>,
    fret: Option<u8>,
    tie_stop: bool,
    hammer: bool,
    harmonic: bool,
    dead: bool,
    staccato: bool,
    accent: bool,
    heavy: bool,
    bend: u8,
}

/// A beat being read: when it starts, how long, its note value and notes.
#[derive(Clone, Default)]
struct Ev {
    start: u32,
    ticks: u32,
    value: Option<(Dur, u8, u8)>,
    notes: Vec<NoteIn>,
    chord: Option<String>,
    text: Option<String>,
    lyric: Option<String>,
    dynamic: Option<Dynamic>,
    fermata: bool,
}

/// A harmony element as a chord name ("C#m7", "D/F#").
fn chord_name(h: Node) -> Option<String> {
    let root = child(h, "root")?;
    let step = text_of(child(root, "root-step"));
    let alter: f64 = text_of(child(root, "root-alter")).parse().unwrap_or(0.0);
    let mut name = format!("{step}{}", if alter > 0.5 { "#" } else if alter < -0.5 { "b" } else { "" });
    if let Some(k) = child(h, "kind") {
        let shown = k.attribute("text").map(|t| t.to_string());
        let value = k.text().unwrap_or("").trim();
        let suffix = match shown {
            Some(t) if !t.is_empty() || value == "major" => t,
            _ => KINDS.iter().find(|x| x.0 == value).map(|x| x.1.to_string()).unwrap_or_else(|| if value == "power" { "5".into() } else { String::new() }),
        };
        name.push_str(&suffix);
    }
    if let Some(b) = child(h, "bass") {
        let bs = text_of(child(b, "bass-step"));
        let ba: f64 = text_of(child(b, "bass-alter")).parse().unwrap_or(0.0);
        name.push_str(&format!("/{bs}{}", if ba > 0.5 { "#" } else if ba < -0.5 { "b" } else { "" }));
    }
    (!step.is_empty()).then_some(name)
}

/// The XML of a MusicXML file, or of the score inside a compressed .mxl file.
fn xml_text(data: &[u8]) -> Result<String, String> {
    if !data.starts_with(b"PK") {
        let s = String::from_utf8_lossy(data);
        return Ok(s.trim_start_matches('\u{feff}').to_string());
    }
    use std::io::Read;
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(data)).map_err(|e| format!("Not a MusicXML file: {e}"))?;
    let mut read = |name: &str| -> Option<String> {
        let mut f = z.by_name(name).ok()?;
        let mut s = String::new();
        f.read_to_string(&mut s).ok()?;
        Some(s)
    };
    // The container names the score; otherwise the first .xml/.musicxml file.
    let root = read("META-INF/container.xml").and_then(|c| {
        let doc = roxmltree::Document::parse(&c).ok()?;
        doc.descendants().find(|n| n.has_tag_name("rootfile")).and_then(|n| n.attribute("full-path").map(|s| s.to_string()))
    });
    if let Some(r) = root
        && let Some(s) = read(&r)
    {
        return Ok(s);
    }
    let names: Vec<String> = z.file_names().map(|s| s.to_string()).collect();
    for n in names {
        if !n.starts_with("META-INF") && (n.ends_with(".xml") || n.ends_with(".musicxml"))
            && let Ok(mut f) = z.by_name(&n)
        {
            let mut s = String::new();
            if f.read_to_string(&mut s).is_ok() {
                return Ok(s);
            }
        }
    }
    Err("No score was found inside this .mxl file.".into())
}

/// A part's name, MIDI program and channel, and drum sounds (instrument id to MIDI note).
type PartInfo = (String, Option<u8>, Option<u8>, BTreeMap<String, u8>);

/// Reads a MusicXML file (or compressed .mxl) into a song.
pub fn import_musicxml(data: &[u8], fallback_title: &str) -> Result<Song, String> {
    let xml = xml_text(data)?;
    let opt = roxmltree::ParsingOptions { allow_dtd: true, ..Default::default() };
    let doc = roxmltree::Document::parse_with_options(&xml, opt).map_err(|e| format!("Not a MusicXML file: {e}"))?;
    let root = doc.root_element();
    match root.tag_name().name() {
        "score-partwise" => {}
        "score-timewise" => return Err("This MusicXML file is arranged by time (timewise); only the usual part-by-part layout can be read.".into()),
        _ => return Err("Not a MusicXML file.".into()),
    }
    let mut song = Song::new_blank();
    song.tracks.clear();
    song.master.clear();
    song.title = path_text(root, &["work", "work-title"]);
    if song.title.is_empty() {
        song.title = text_of(child(root, "movement-title"));
    } else {
        song.subtitle = text_of(child(root, "movement-title"));
    }
    if song.title.is_empty() {
        song.title = fallback_title.to_string();
    }
    if let Some(id) = child(root, "identification") {
        for c in children(id, "creator") {
            let who = c.text().unwrap_or("").trim().to_string();
            match c.attribute("type") {
                Some("composer") => song.music_by = who,
                Some("lyricist") | Some("poet") => song.words = who,
                Some("arranger") => song.tabbed_by = who,
                Some("artist") => song.artist = who,
                _ => {}
            }
        }
        song.copyright = text_of(child(id, "rights"));
    }
    // Part names, MIDI programs and drum sounds.
    let mut info: BTreeMap<String, PartInfo> = BTreeMap::new();
    if let Some(pl) = child(root, "part-list") {
        for sp in children(pl, "score-part") {
            let id = sp.attribute("id").unwrap_or("").to_string();
            let name = text_of(child(sp, "part-name"));
            let mut program = None;
            let mut channel = None;
            let mut drums = BTreeMap::new();
            for mi in children(sp, "midi-instrument") {
                if let Some(p) = num::<u32>(child(mi, "midi-program")) {
                    program = Some(p.saturating_sub(1).min(127) as u8);
                }
                if let Some(c) = num::<u8>(child(mi, "midi-channel")) {
                    channel = Some(c);
                }
                if let (Some(iid), Some(u)) = (mi.attribute("id"), num::<u32>(child(mi, "midi-unpitched"))) {
                    drums.insert(iid.to_string(), u.saturating_sub(1).min(127) as u8);
                }
            }
            info.insert(id, (name, program, channel, drums));
        }
    }
    let mut tempo_set = false;
    for (pi, part) in children(root, "part").enumerate() {
        let id = part.attribute("id").unwrap_or("").to_string();
        let (name, program, channel, drum_map) = info.get(&id).cloned().unwrap_or_default();
        let mut tr = make_track(part, &name, program, channel, pi);
        let tab_staff = tab_staff_of(part);
        let mut divisions = 1u32;
        let (mut num_, mut den_) = (4u8, 4u8);
        let (mut key, mut minor) = (0i8, false);
        let mut bars: Vec<Bar> = vec![];
        for (mi, m) in children(part, "measure").enumerate() {
            if song.master.len() <= mi {
                let prev = song.master.last().cloned().unwrap_or_default();
                song.master.push(MasterBar { num: prev.num, den: prev.den, key: prev.key, key_minor: prev.key_minor, ..MasterBar::default() });
            }
            let mut cursor = 0u32;
            let mut last_start = 0u32;
            let mut voices: BTreeMap<(u32, u32), Vec<Ev>> = BTreeMap::new();
            let mut pending = Ev::default();
            let to_ticks = |d: u32, div: u32| (d as u64 * QUARTER as u64 / div.max(1) as u64) as u32;
            for c in m.children().filter(|c| c.is_element()) {
                match c.tag_name().name() {
                    "attributes" => {
                        if let Some(d) = num::<u32>(child(c, "divisions")) {
                            divisions = d.max(1);
                        }
                        if let Some(k) = child(c, "key") {
                            if let Some(f) = num::<i32>(child(k, "fifths")) {
                                key = f.clamp(-7, 7) as i8;
                            }
                            minor = text_of(child(k, "mode")) == "minor";
                        }
                        if let Some(t) = child(c, "time") {
                            let beats: u32 = text_of(child(t, "beats")).split('+').filter_map(|b| b.trim().parse::<u32>().ok()).sum();
                            if beats > 0 {
                                num_ = beats.min(32) as u8;
                            }
                            if let Some(bt) = num::<u8>(child(t, "beat-type")) {
                                den_ = bt.clamp(1, 32);
                            }
                        }
                        let mb = &mut song.master[mi];
                        mb.num = num_;
                        mb.den = den_;
                        mb.key = key;
                        mb.key_minor = minor;
                    }
                    "direction" => {
                        for dt in children(c, "direction-type") {
                            for d in dt.children().filter(|d| d.is_element()) {
                                match d.tag_name().name() {
                                    "rehearsal" => {
                                        let t = d.text().unwrap_or("").trim().to_string();
                                        if !t.is_empty() && song.master[mi].marker.is_none() {
                                            song.master[mi].marker = Some(t);
                                        }
                                    }
                                    "words" => {
                                        let t = d.text().unwrap_or("").trim().to_string();
                                        if !t.is_empty() {
                                            pending.text = Some(match pending.text.take() {
                                                Some(p) => format!("{p} {t}"),
                                                None => t,
                                            });
                                        }
                                    }
                                    "dynamics" => {
                                        if let Some(dn) = d.children().find(|x| x.is_element()) {
                                            pending.dynamic = dynamic_from_name(dn.tag_name().name());
                                        }
                                    }
                                    "metronome" => {
                                        if child(c, "sound").and_then(|s| s.attribute("tempo")).is_none()
                                            && let Some(t) = num::<u32>(child(d, "per-minute"))
                                        {
                                            set_tempo(&mut song, mi, cursor, t, &mut tempo_set);
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        if let Some(t) = child(c, "sound").and_then(|s| s.attribute("tempo")).and_then(|t| t.parse::<f64>().ok()) {
                            set_tempo(&mut song, mi, cursor, t.round() as u32, &mut tempo_set);
                        }
                    }
                    "sound" => {
                        if let Some(t) = c.attribute("tempo").and_then(|t| t.parse::<f64>().ok()) {
                            set_tempo(&mut song, mi, cursor, t.round() as u32, &mut tempo_set);
                        }
                    }
                    "harmony" => pending.chord = chord_name(c),
                    "backup" => cursor = cursor.saturating_sub(to_ticks(num(child(c, "duration")).unwrap_or(0), divisions)),
                    "forward" => cursor += to_ticks(num(child(c, "duration")).unwrap_or(0), divisions),
                    "barline" => {
                        let mb = &mut song.master[mi];
                        if let Some(r) = child(c, "repeat") {
                            if r.attribute("direction") == Some("forward") {
                                mb.repeat_open = true;
                            } else {
                                mb.repeat_close = r.attribute("times").and_then(|t| t.parse::<u8>().ok()).unwrap_or(2).max(2);
                            }
                        }
                        if let Some(e) = child(c, "ending") {
                            for n in e.attribute("number").unwrap_or("").split([',', ' ']) {
                                if let Ok(k) = n.trim().parse::<u8>()
                                    && (1..=8).contains(&k)
                                {
                                    mb.endings |= 1 << (k - 1);
                                }
                            }
                        }
                        if text_of(child(c, "bar-style")) == "light-light" {
                            mb.double_bar = true;
                        }
                    }
                    "note" => {
                        if child(c, "grace").is_some() || child(c, "cue").is_some() {
                            continue;
                        }
                        let is_chord = child(c, "chord").is_some();
                        let ticks = to_ticks(num(child(c, "duration")).unwrap_or(0), divisions);
                        let start = if is_chord { last_start } else { cursor };
                        if !is_chord {
                            cursor += ticks;
                            last_start = start;
                        }
                        let staff: u32 = num(child(c, "staff")).unwrap_or(1);
                        if let Some(t) = tab_staff
                            && staff != t
                        {
                            continue;
                        }
                        let voice: u32 = num(child(c, "voice")).unwrap_or(1);
                        let list = voices.entry((staff, voice)).or_default();
                        let rest = child(c, "rest").is_some();
                        let mut value = None;
                        if let Some(d) = type_from_name(&text_of(child(c, "type"))) {
                            let dots = children(c, "dot").count().min(2) as u8;
                            let tuplet = child(c, "time-modification")
                                .and_then(|tm| {
                                    let a: u8 = num(child(tm, "actual-notes"))?;
                                    let n: u8 = num(child(tm, "normal-notes"))?;
                                    [(3u8, 2u8), (5, 4), (6, 4), (7, 4), (7, 8), (9, 8)].iter().find(|r| r.0 == a && r.1 == n).map(|r| r.0)
                                })
                                .unwrap_or(0);
                            value = Some((d, dots, tuplet));
                        }
                        let ev = match list.last_mut() {
                            Some(e) if is_chord && e.start == start => e,
                            _ => {
                                let mut e = std::mem::take(&mut pending);
                                e.start = start;
                                e.ticks = ticks;
                                e.value = value;
                                list.push(e);
                                list.last_mut().expect("just pushed")
                            }
                        };
                        if let Some(l) = child(c, "lyric").and_then(|l| child(l, "text")) {
                            ev.lyric = Some(l.text().unwrap_or("").to_string());
                        }
                        if rest {
                            continue;
                        }
                        let pitch = if let Some(p) = child(c, "pitch") {
                            let alter: f64 = text_of(child(p, "alter")).parse().unwrap_or(0.0);
                            midi_of(&text_of(child(p, "step")), alter, num(child(p, "octave")).unwrap_or(4))
                        } else if let Some(u) = child(c, "unpitched") {
                            let from_inst = child(c, "instrument").and_then(|i| i.attribute("id")).and_then(|i| drum_map.get(i).copied());
                            from_inst.or_else(|| midi_of(&text_of(child(u, "display-step")), 0.0, num(child(u, "display-octave")).unwrap_or(4)))
                        } else {
                            None
                        };
                        let Some(pitch) = pitch else { continue };
                        let mut n = NoteIn { pitch, ..Default::default() };
                        n.dead = text_of(child(c, "notehead")) == "x";
                        n.tie_stop = children(c, "tie").any(|t| t.attribute("type") == Some("stop"));
                        if let Some(nt) = child(c, "notations") {
                            if children(nt, "tied").any(|t| t.attribute("type") == Some("stop")) {
                                n.tie_stop = true;
                            }
                            if child(nt, "fermata").is_some() {
                                ev.fermata = true;
                            }
                            if let Some(a) = child(nt, "articulations") {
                                n.staccato = child(a, "staccato").is_some();
                                n.accent = child(a, "accent").is_some();
                                n.heavy = child(a, "strong-accent").is_some();
                            }
                            if let Some(t) = child(nt, "technical") {
                                n.string = num(child(t, "string"));
                                n.fret = num(child(t, "fret"));
                                n.hammer = ["hammer-on", "pull-off"].iter().any(|k| child(t, k).and_then(|h| h.attribute("type")) == Some("start"));
                                n.harmonic = child(t, "harmonic").is_some();
                                if let Some(b) = child(t, "bend") {
                                    let alter: f64 = text_of(child(b, "bend-alter")).parse().unwrap_or(0.0);
                                    n.bend = (alter * 2.0).round().clamp(0.0, 12.0) as u8;
                                }
                            }
                        }
                        ev.notes.push(n);
                    }
                    _ => {}
                }
            }
            bars.push(build_bar(&tr, &voices, song.master[mi].ticks()));
        }
        tr.bars = bars;
        song.tracks.push(tr);
    }
    if song.tracks.is_empty() {
        return Err("This MusicXML file has no parts.".into());
    }
    let n = song.master.len().max(1);
    if song.master.is_empty() {
        song.master.push(MasterBar::default());
    }
    for t in &mut song.tracks {
        while t.bars.len() < n {
            t.bars.push(Bar::empty());
        }
    }
    song.normalize();
    Ok(song)
}

fn path_text(n: Node, p: &[&str]) -> String {
    let mut cur = Some(n);
    for s in p {
        cur = cur.and_then(|c| child(c, s));
    }
    text_of(cur)
}

fn set_tempo(song: &mut Song, mi: usize, cursor: u32, t: u32, tempo_set: &mut bool) {
    let t = t.clamp(20, 400);
    if mi == 0 && cursor == 0 && !*tempo_set {
        song.tempo = t;
        *tempo_set = true;
    } else if mi > 0 && song.master[mi].tempo.is_none() && song.tempo_at(mi) != t {
        song.master[mi].tempo = Some(t);
    } else if mi == 0 && !*tempo_set {
        song.tempo = t;
        *tempo_set = true;
    }
}

/// The staff that is tablature when a part has two staves (notes are read from it,
/// where each one has its string and fret).
fn tab_staff_of(part: Node) -> Option<u32> {
    let staves: u32 = part.descendants().find(|n| n.has_tag_name("staves")).and_then(|s| s.text()).and_then(|t| t.trim().parse().ok()).unwrap_or(1);
    if staves < 2 {
        return None;
    }
    part.descendants()
        .filter(|n| n.has_tag_name("clef"))
        .find(|c| text_of(child(*c, "sign")) == "TAB")
        .map(|c| c.attribute("number").and_then(|n| n.parse().ok()).unwrap_or(1))
}

/// The track for a part: drums, a stringed instrument with its tuning, or another instrument.
fn make_track(part: Node, name: &str, program: Option<u8>, channel: Option<u8>, k: usize) -> Track {
    let percussion = channel == Some(10) || part.descendants().any(|n| n.has_tag_name("clef") && text_of(child(n, "sign")) == "percussion");
    // The tuning from the first staff details that have one: line 1 is the lowest string.
    let mut tuning: Vec<(u32, u8)> = vec![];
    let mut capo = 0u8;
    if let Some(sd) = part.descendants().find(|n| n.has_tag_name("staff-details") && children(*n, "staff-tuning").next().is_some()) {
        for st in children(sd, "staff-tuning") {
            let line: u32 = st.attribute("line").and_then(|l| l.parse().ok()).unwrap_or(0);
            let alter: f64 = text_of(child(st, "tuning-alter")).parse().unwrap_or(0.0);
            if let Some(p) = midi_of(&text_of(child(st, "tuning-step")), alter, num(child(st, "tuning-octave")).unwrap_or(3)) {
                tuning.push((line, p));
            }
        }
        capo = num(child(sd, "capo")).unwrap_or(0);
    }
    tuning.sort_by_key(|t| std::cmp::Reverse(t.0));
    let prog = program.unwrap_or(25);
    let mut tr = if percussion {
        Track::of_instrument(Instrument::Drums, "Drums")
    } else if !tuning.is_empty() {
        let low = tuning.iter().map(|t| t.1).min().unwrap_or(40);
        let inst = Instrument::from_midi_program(prog);
        let inst = if inst.has_strings_kind() {
            inst
        } else if low < 36 {
            Instrument::Bass
        } else {
            Instrument::SteelGuitar
        };
        Track { instrument: inst, tuning: tuning.iter().map(|t| t.1).collect(), capo, ..Track::default() }
    } else if (32..=39).contains(&prog) {
        Track { instrument: Instrument::Bass, tuning: STANDARD_BASS.to_vec(), ..Track::default() }
    } else {
        let inst = Instrument::from_midi_program(prog);
        if inst.has_strings_kind() {
            Track { instrument: inst, ..Track::default() }
        } else {
            Track::of_instrument(inst, inst.name())
        }
    };
    tr.color = TRACK_COLORS[k % TRACK_COLORS.len()];
    tr.name = if name.is_empty() { format!("Track {}", k + 1) } else { name.to_string() };
    tr
}

/// One bar of a track from the beats read for each staff and voice.
fn build_bar(tr: &Track, voices: &BTreeMap<(u32, u32), Vec<Ev>>, bar_ticks: u32) -> Bar {
    let mut bar = Bar::empty();
    bar.voices.clear();
    let mut slot = 0;
    for evs in voices.values() {
        if slot >= VOICES {
            break;
        }
        let mut evs: Vec<&Ev> = evs.iter().collect();
        evs.sort_by_key(|e| e.start);
        let mut beats: Vec<Beat> = vec![];
        let mut pos = 0u32;
        let rest = |beats: &mut Vec<Beat>, len: u32| {
            for (d, dot) in split_ticks(len) {
                let mut r = Beat::rest(d);
                r.dotted = dot;
                beats.push(r);
            }
        };
        for e in evs {
            if e.start < pos || e.start >= bar_ticks {
                continue;
            }
            if e.start > pos {
                rest(&mut beats, e.start - pos);
            }
            let ticks = e.ticks.min(bar_ticks - e.start);
            let notes = place_notes(tr, &e.notes);
            // The written note value when it matches the length, otherwise standard values tied together.
            let mut shapes: Vec<(Dur, bool, bool, u8)> = vec![];
            if let Some((d, dots, tup)) = e.value {
                let mut b = Beat::rest(d);
                b.dotted = dots == 1;
                b.double_dot = dots == 2;
                b.tuplet = if tup == 7 { 7 } else { tup };
                if b.ticks().abs_diff(ticks) <= 2 {
                    shapes.push((d, dots == 1, dots == 2, b.tuplet));
                }
            }
            if shapes.is_empty() {
                shapes = split_ticks(ticks).into_iter().map(|(d, dot)| (d, dot, false, 0)).collect();
            }
            for (k, (d, dot, dd, tup)) in shapes.into_iter().enumerate() {
                let mut b = Beat::rest(d);
                b.dotted = dot;
                b.double_dot = dd;
                b.tuplet = tup;
                b.notes = notes.iter().cloned().map(|mut n| {
                    if k > 0 {
                        n.tie = true;
                    }
                    n
                }).collect();
                if k == 0 {
                    b.chord = e.chord.clone();
                    b.text = e.text.clone();
                    b.lyric = e.lyric.clone();
                    b.fermata = e.fermata;
                    if let Some(d) = e.dynamic {
                        b.dynamic = d;
                    }
                }
                beats.push(b);
            }
            pos = e.start + ticks;
        }
        if beats.iter().all(|b| b.is_rest()) && slot > 0 {
            continue;
        }
        if pos < bar_ticks && !beats.is_empty() {
            rest(&mut beats, bar_ticks - pos);
        }
        if beats.is_empty() {
            beats.push(Beat::rest(Dur::Whole));
        }
        if slot == 0 {
            bar.beats = beats;
        } else {
            bar.voices.push(beats);
        }
        slot += 1;
    }
    if !bar.voices.is_empty() {
        bar.pad_voices();
    }
    bar
}

/// The notes on the track's strings: where the file gives string and fret, there;
/// otherwise chosen from the pitches.
fn place_notes(tr: &Track, notes: &[NoteIn]) -> Vec<Note> {
    let given = !notes.is_empty()
        && notes.iter().all(|n| matches!((n.string, n.fret), (Some(s), Some(_)) if s >= 1 && (s as usize) <= tr.strings()))
        && tr.has_strings();
    let mut out: Vec<(Note, &NoteIn)> = vec![];
    if given {
        for n in notes {
            let mut note = Note::new(n.string.unwrap_or(1) - 1, n.fret.unwrap_or(0));
            if out.iter().any(|(o, _)| o.string == note.string) {
                continue;
            }
            note.string = n.string.unwrap_or(1) - 1;
            out.push((note, n));
        }
    } else {
        let pitches: Vec<u8> = notes.iter().map(|n| n.pitch).collect();
        for note in crate::textmidi::place_pitches(tr, &pitches) {
            let p = tr.pitch(&note);
            let src = notes.iter().min_by_key(|n| (n.pitch as i32 - p as i32).abs() % 12 + if n.pitch == p { 0 } else { 100 }).unwrap_or(&notes[0]);
            out.push((note, src));
        }
    }
    out.into_iter()
        .map(|(mut n, s)| {
            n.tie = s.tie_stop;
            n.hammer = s.hammer;
            n.harmonic = s.harmonic;
            n.dead = s.dead;
            n.staccato = s.staccato;
            n.accent = s.accent;
            n.heavy_accent = s.heavy;
            n.bend = s.bend;
            n
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_keeps_the_music() {
        let song = Song::demo();
        let xml = export_musicxml(&song);
        assert!(xml.contains("<score-partwise version=\"4.0\">"));
        let back = import_musicxml(xml.as_bytes(), "x").unwrap();
        assert_eq!(back.title, song.title);
        assert_eq!(back.tracks.len(), song.tracks.len());
        assert_eq!(back.bar_count(), song.bar_count());
        assert_eq!(back.tempo, song.tempo);
        for (a, b) in song.tracks.iter().zip(&back.tracks) {
            assert_eq!(a.tuning, b.tuning, "{}", a.name);
            assert_eq!(a.name, b.name);
            for (ba, bb) in a.bars.iter().zip(&b.bars) {
                let notes = |bar: &Bar| bar.beats.iter().map(|x| (x.dur, x.dotted, x.tuplet, x.notes.iter().map(|n| (n.string, n.fret, n.tie)).collect::<Vec<_>>())).collect::<Vec<_>>();
                assert_eq!(notes(ba), notes(bb), "{}", a.name);
            }
        }
        for (a, b) in song.master.iter().zip(&back.master) {
            assert_eq!((a.num, a.den, a.key, a.key_minor, &a.marker, a.repeat_open, a.repeat_close, a.endings), (b.num, b.den, b.key, b.key_minor, &b.marker, b.repeat_open, b.repeat_close, b.endings));
        }
    }

    #[test]
    fn reads_plain_musicxml_without_tab() {
        let xml = r#"<?xml version="1.0"?>
<score-partwise version="3.1"><work><work-title>Plain</work-title></work>
<part-list><score-part id="P1"><part-name>Guitar</part-name><midi-instrument id="I"><midi-program>26</midi-program></midi-instrument></score-part></part-list>
<part id="P1"><measure number="1">
<attributes><divisions>2</divisions><key><fifths>-1</fifths></key><time><beats>3</beats><beat-type>4</beat-type></time></attributes>
<harmony><root><root-step>F</root-step></root><kind>major</kind></harmony>
<note><pitch><step>F</step><octave>3</octave></pitch><duration>2</duration><voice>1</voice><type>quarter</type></note>
<note><chord/><pitch><step>A</step><octave>3</octave></pitch><duration>2</duration><voice>1</voice><type>quarter</type></note>
<note><rest/><duration>2</duration><voice>1</voice><type>quarter</type></note>
<note><pitch><step>B</step><alter>-1</alter><octave>3</octave></pitch><duration>1</duration><voice>1</voice><type>eighth</type></note>
</measure></part></score-partwise>"#;
        let s = import_musicxml(xml.as_bytes(), "x").unwrap();
        assert_eq!(s.title, "Plain");
        assert_eq!((s.master[0].num, s.master[0].den, s.master[0].key), (3, 4, -1));
        let t = &s.tracks[0];
        let b = &t.bars[0].beats;
        assert_eq!(b[0].chord.as_deref(), Some("F"));
        let mut p: Vec<u8> = b[0].notes.iter().map(|n| t.pitch(n)).collect();
        p.sort();
        assert_eq!(p, vec![53, 57]);
        assert!(b[1].is_rest());
        assert_eq!(t.pitch(&b[2].notes[0]), 58);
        assert_eq!(b[2].dur, Dur::Eighth);
        // The bar is filled up with a rest.
        assert_eq!(t.bars[0].ticks(), s.master[0].ticks());
    }

    #[test]
    fn chord_names_go_both_ways() {
        for n in ["E", "C#m7", "Bbmaj7", "D/F#", "G7", "A5", "Esus4"] {
            let x = harmony_xml(n).unwrap();
            let doc = roxmltree::Document::parse(&x).unwrap();
            assert_eq!(chord_name(doc.root_element()).as_deref(), Some(n), "{x}");
        }
    }
}
