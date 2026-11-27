//! Writes songs as Guitar Pro files of the newer kind (.gp): a zip archive
//! holding the score as XML ("Content/score.gpif"), following the
//! community-documented layout that the reader in gpif.rs also uses.
//! Everything is stored the standard way, including each track's volume, pan,
//! mute and solo, so Guitar Pro and this app read the same file.

use crate::drums::drum;
use crate::gpkit::{KIT, NOTATION, NOTATION_SOUNDS};
use crate::model::*;
use std::fmt::Write as _;
use std::io::Write as _;

fn cdata(s: &str) -> String {
    format!("<![CDATA[{}]]>", s.replace("]]>", "]]]]><![CDATA[>"))
}

fn note_value(d: Dur) -> &'static str {
    match d {
        Dur::Whole => "Whole",
        Dur::Half => "Half",
        Dur::Quarter => "Quarter",
        Dur::Eighth => "Eighth",
        Dur::Sixteenth => "16th",
        Dur::ThirtySecond => "32nd",
        _ => "64th",
    }
}

fn dyn_name(d: Dynamic) -> &'static str {
    match d {
        Dynamic::Ppp => "PPP",
        Dynamic::Pp => "PP",
        Dynamic::P => "P",
        Dynamic::Mp => "MP",
        Dynamic::Mf => "MF",
        Dynamic::F => "F",
        Dynamic::Ff => "FF",
        Dynamic::Fff => "FFF",
    }
}

pub fn inst_type(i: Instrument) -> (&'static str, &'static str) {
    use Instrument::*;
    match i {
        SteelGuitar => ("steelGuitar", "Steel Guitar"),
        NylonGuitar => ("nylonGuitar", "Classical Guitar"),
        CleanGuitar | OverdriveGuitar | DistortionGuitar => ("electricGuitar", "Electric Guitar"),
        Bass => ("electricBass", "Electric Bass"),
        Ukulele => ("ukulele", "Ukulele"),
        Banjo => ("banjo", "Banjo"),
        // The others are the names in a file Guitar Pro 8 saved (see `gpinstr.rs`).
        Piano | ElectricPiano | Organ | Synth | LeadVocals | Choir | Violin | Cello | Strings | Trumpet | Saxophone | Flute => {
            crate::gpinstr::pitched(i).map(|g| (g.kind, g.set)).unwrap_or(("steelGuitar", "Steel Guitar"))
        }
        Drums => ("drumKit", "Drum Kit"),
    }
}

/// Pitch as step letter, accidental and octave (middle C = C4).
fn pitch_parts(p: i32) -> (&'static str, &'static str, i32) {
    const N: [(&str, &str); 12] = [
        ("C", ""),
        ("C", "#"),
        ("D", ""),
        ("D", "#"),
        ("E", ""),
        ("F", ""),
        ("F", "#"),
        ("G", ""),
        ("G", "#"),
        ("A", ""),
        ("A", "#"),
        ("B", ""),
    ];
    let (s, a) = N[p.rem_euclid(12) as usize];
    (s, a, p.div_euclid(12) - 1)
}

/// Guitar Pro's name for its drum kit sound.
const DRUM_SOUND: &str = "Drums/Drums/Drumkit";

/// Whether a note of this pitch is written on a staff: on a grand staff the lower one takes the
/// notes below middle C. `low` is None for a single staff.
fn on_staff(pitch: u8, low: Option<bool>) -> bool {
    match low {
        None => true,
        Some(l) => (pitch < 60) == l,
    }
}

/// The voice to write in each of a staff's voice places (None: the place is empty). A single
/// staff keeps each voice in its place: the first, and the others that have notes. On a grand
/// staff each staff takes the voices that have notes on it, from the first place on, as Guitar
/// Pro lays out a part for two hands; the upper staff always has the first voice, which carries
/// the chord names and texts.
fn staff_voices(t: &Track, bar: &Bar, low: Option<bool>) -> [Option<usize>; VOICES] {
    let mut out = [None; VOICES];
    if low.is_none() {
        for (v, place) in out.iter_mut().enumerate() {
            *place = (v == 0 || bar.voice_used(v)).then_some(v);
        }
        return out;
    }
    let here = |v: usize| bar.voice(v).iter().any(|b| b.notes.iter().any(|n| on_staff(t.pitch(n), low)));
    let mut used: Vec<usize> = (0..VOICES).filter(|&v| (v == 0 && low == Some(false)) || here(v)).collect();
    if used.is_empty() {
        used.push(0);
    }
    for (place, v) in out.iter_mut().zip(used) {
        *place = Some(v);
    }
    out
}

/// For an instrument that Guitar Pro tunes and this app doesn't, the tuning of each staff, with
/// room for all its notes. Empty for other tracks.
fn staff_tunings(t: &Track) -> Vec<Vec<u8>> {
    let Some(g) = crate::gpinstr::pitched(t.instrument) else { return vec![] };
    let staves: &[(Option<bool>, &[u8])] = if t.grand() {
        &[(Some(false), g.tuning), (Some(true), crate::gpinstr::LOWER_TUNING)]
    } else {
        &[(None, g.tuning)]
    };
    staves
        .iter()
        .enumerate()
        .map(|(staff, &(low, usual))| {
            let usual = t.kept.as_deref().and_then(|k| k.tuning(t, staff)).unwrap_or(usual);
            let mut chords = vec![];
            for beat in t.bars.iter().flat_map(|b| (0..VOICES).flat_map(move |v| b.voice(v).iter())) {
                let here = || beat.notes.iter().filter(move |n| on_staff(t.pitch(n), low));
                chords.push(here().map(|n| t.pitch(n)).collect::<Vec<u8>>());
                chords.push(here().filter_map(|n| Some(t.pitch(&Note::new(n.string, n.grace?)))).collect());
            }
            crate::gpinstr::tuning_for(usual, &chords)
        })
        .collect()
}

/// The channel strip: sixteen numbers from 0 to 1, with the equalizer, pan and volume among
/// them.
pub(crate) fn strip_values(t: &Track) -> [f32; 16] {
    let mut strip = [0.5f32; 16];
    for (k, &gi) in EQ_GPIF_INDEX.iter().enumerate() {
        strip[gi] = t.equalizer.knobs[k].clamp(0.0, 1.0);
    }
    strip[9] = if t.equalizer.on { 1.0 } else { 0.0 };
    strip[11] = (t.pan.clamp(-1.0, 1.0) + 1.0) * 0.5;
    strip[12] = t.volume.clamp(0.0, 1.0);
    strip
}

/// Index of a drum sound among all the drum kit's articulations, in writing order. A number
/// the kit doesn't have becomes the first snare.
fn drum_articulation(p: u8) -> usize {
    KIT.iter().flat_map(|e| e.articulations).position(|a| a.midi == p).unwrap_or(0)
}

/// A drum track's instrument set and notation, as Guitar Pro writes them.
fn write_drum_kit(x: &mut String) {
    x.push_str("<InstrumentSet><Name>Drums</Name><Type>drumKit</Type><LineCount>5</LineCount><Elements>");
    for e in KIT {
        let _ = write!(x, "<Element><Name>{}</Name><Type>{}</Type><SoundbankName>{}</SoundbankName><Articulations>", e.name, e.kind, e.soundbank);
        for a in e.articulations {
            // Every sound of the kit is in the table of sounds.
            let (line, output) = drum(a.midi).map(|d| (8 - d.pos as i32, d.gm)).unwrap_or((3, a.midi));
            let _ = write!(
                x,
                "<Articulation><Name>{}</Name><StaffLine>{line}</StaffLine><Noteheads>{}</Noteheads><TechniquePlacement>{}</TechniquePlacement><TechniqueSymbol>{}</TechniqueSymbol><InputMidiNumbers>{}</InputMidiNumbers><OutputRSESound>{}</OutputRSESound><OutputMidiNumber>{output}</OutputMidiNumber></Articulation>",
                a.name, a.heads, a.place, a.symbol, a.midi, a.rse
            );
        }
        x.push_str("</Articulations></Element>");
    }
    x.push_str("</Elements></InstrumentSet>");
    let _ = write!(x, "<NotationPatch><Name>{NOTATION}</Name><LineCount>5</LineCount><Elements>");
    for (element, sounds) in NOTATION_SOUNDS {
        let _ = write!(x, "<Element><Name>{element}</Name><Articulations>");
        for &m in *sounds {
            if let Some(e) = KIT.iter().find(|e| e.name == *element)
                && let Some(a) = e.articulations.iter().find(|a| a.midi == m)
                && let Some(d) = drum(m)
            {
                let _ = write!(x, "<Articulation><Name>{}</Name><StaffLine>{}</StaffLine></Articulation>", a.name, 8 - d.pos as i32);
            }
        }
        x.push_str("</Articulations></Element>");
    }
    x.push_str("</Elements></NotationPatch>");
}

/// A drum kit's sounds in order: a drum note names its sound by its place in them.
type Kit = Vec<Option<u8>>;

struct Ids {
    beats: String,
    notes: String,
    rhythms: Vec<(Dur, bool, bool, u8)>,
    voices: String,
    bars: String,
    nbeats: usize,
    nnotes: usize,
    nvoices: usize,
    nbars: usize,
    /// The beat about to be written is slurred from the one before it (legato).
    legato_dest: bool,
    /// Seconds from the start of the song to the beat about to be written (for timers).
    beat_secs: u32,
    /// The track being written has a kit kept from its file: its sounds in order.
    kit: Option<Kit>,
    /// Semitones from sounding to written pitch kept from the track's file.
    written_shift: Option<i32>,
}

impl Ids {
    fn rhythm(&mut self, b: &Beat) -> usize {
        let key = (b.dur, b.dotted, b.double_dot, b.tuplet);
        if let Some(i) = self.rhythms.iter().position(|r| *r == key) {
            return i;
        }
        self.rhythms.push(key);
        self.rhythms.len() - 1
    }
}

/// Parts of the file that can be left out: chord names (written without chord
/// diagrams) and the score layout views (needed only to keep tracks hidden).
#[derive(Clone, Copy)]
pub struct GpOptions {
    pub chords: bool,
    pub views: bool,
}

impl GpOptions {
    /// What a normal save writes: chord names (Guitar Pro 8 opens them), and
    /// layout views only when a track is hidden.
    pub fn safe(song: &Song) -> GpOptions {
        GpOptions {
            chords: true,
            views: song.tracks.iter().any(|t| !t.visible),
        }
    }
}

/// The chord diagram of a chord name: the frets of the first beat that uses it
/// (strings counted from the lowest, frets from the diagram's first fret).
fn diagram_xml(t: &Track, name: &str) -> String {
    let n = t.strings().max(1);
    let beat = t
        .bars
        .iter()
        .flat_map(|b| b.beats.iter())
        .find(|b| b.chord.as_deref() == Some(name) && (!b.is_rest() || b.chord_frets.is_some()));
    // Strings counted from the lowest.
    let notes: Vec<(usize, u8)> = beat
        .map(|b| b.diagram_frets(n).iter().enumerate().filter_map(|(s, f)| f.map(|f| (n - 1 - s, f))).collect())
        .unwrap_or_default();
    let fretted: Vec<u8> = notes.iter().map(|x| x.1).filter(|&f| f > 0).collect();
    let max = fretted.iter().copied().max().unwrap_or(0);
    let base = if max <= 5 { 0 } else { fretted.iter().copied().min().unwrap_or(1).saturating_sub(1) };
    let show = beat.map(|b| b.chord_diagram).unwrap_or(false);
    let name_shown = !beat.map(|b| b.chord_name_hidden).unwrap_or(false);
    let mut x = format!(
        "<Diagram stringCount=\"{n}\" fretCount=\"5\" baseFret=\"{base}\" barsStates=\"{}\">",
        vec!["1"; n].join(" ")
    );
    for (s, f) in &notes {
        let _ = write!(x, "<Fret string=\"{s}\" fret=\"{}\"/>", f.saturating_sub(base));
    }
    let _ = write!(
        x,
        "<Property name=\"ShowName\" type=\"bool\" value=\"{name_shown}\"/><Property name=\"ShowDiagram\" type=\"bool\" value=\"{show}\"/><Property name=\"ShowFingering\" type=\"bool\" value=\"false\"/></Diagram>"
    );
    x
}

/// The chord names a track uses, sorted, each once. Their positions are the ids
/// beats refer to. Empty when chord names are left out.
fn chord_names(t: &Track, o: GpOptions) -> Vec<&String> {
    if !o.chords {
        return vec![];
    }
    let mut names: Vec<&String> = t
        .bars
        .iter()
        .flat_map(|b| b.beats.iter().filter_map(|x| x.chord.as_ref()))
        .collect();
    names.sort();
    names.dedup();
    names
}

/// The score as Guitar Pro XML.
#[cfg(test)]
pub fn gpif_xml(song: &Song) -> String {
    gpif_xml_with(song, GpOptions::safe(song))
}

pub fn gpif_xml_with(song: &Song, o: GpOptions) -> String {
    let mut x = String::with_capacity(256 * 1024);
    x.push_str("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<GPIF>\n<GPVersion>7</GPVersion>\n<GPRevision required=\"12021\" recommended=\"12023\">12025</GPRevision>\n<Encoding><EncodingDescription>GP7</EncodingDescription></Encoding>\n");
    // Song information. What was kept from an opened file goes back in its place.
    let kept = song.kept.as_deref();
    let page_text = |tag: &str| match kept.and_then(|k| k.page_text(tag, song)) {
        Some(element) => element.to_string(),
        None => format!("<{tag}>{}</{tag}>", cdata("")),
    };
    let bars_per_line = kept
        .and_then(|k| k.bars_per_line(song))
        .unwrap_or("<ScoreSystemsDefaultLayout>4</ScoreSystemsDefaultLayout><ScoreSystemsLayout>4</ScoreSystemsLayout>");
    let zoom = kept.and_then(|k| k.zoom()).unwrap_or("<ScoreZoomPolicy>Value</ScoreZoomPolicy><ScoreZoom>1</ScoreZoom>");
    let multi_voice = kept.and_then(|k| k.multi_voice()).unwrap_or("<MultiVoice>0</MultiVoice>");
    let _ = writeln!(
        x,
        "<Score><Title>{}</Title><SubTitle>{}</SubTitle><Artist>{}</Artist><Album>{}</Album><Words>{}</Words><Music>{}</Music><WordsAndMusic>{}</WordsAndMusic><Copyright>{}</Copyright><Tabber>{}</Tabber><Instructions>{}</Instructions><Notices>{}</Notices>{}{}{}{}{bars_per_line}{zoom}{multi_voice}</Score>",
        cdata(kept.map_or(song.title.as_str(), |k| k.title(song))),
        cdata(&song.subtitle),
        cdata(&song.artist),
        cdata(&song.album),
        cdata(&song.words),
        cdata(&song.music_by),
        cdata(""),
        cdata(&song.copyright),
        cdata(&song.tabbed_by),
        cdata(&song.instructions),
        cdata(&song.notice),
        page_text("FirstPageHeader"),
        page_text("FirstPageFooter"),
        page_text("PageHeader"),
        page_text("PageFooter"),
    );
    // Tempo changes.
    let ids: Vec<String> = (0..song.tracks.len()).map(|k| k.to_string()).collect();
    let _ = write!(
        x,
        "<MasterTrack>{}<Tracks>{}</Tracks><Automations>",
        if song.master.first().is_some_and(|m| m.pickup) { "<Anacrusis />" } else { "" },
        ids.join(" ")
    );
    let mut points = vec![(
        0usize,
        song.tempo,
        song.master.first().map(|m| m.tempo_ramp).unwrap_or(false),
    )];
    for (bi, mb) in song.master.iter().enumerate().skip(1) {
        if let Some(t) = mb.tempo {
            points.push((bi, t, mb.tempo_ramp));
        }
    }
    for (bi, t, ramp) in points {
        let _ = write!(
            x,
            "<Automation><Type>Tempo</Type><Linear>{ramp}</Linear><Bar>{bi}</Bar><Position>0</Position><Visible>true</Visible><Value>{t} 2</Value></Automation>"
        );
    }
    x.push_str("</Automations>");
    if let Some(effects) = kept.and_then(|k| k.master_effects(song)) {
        x.push_str(&effects);
    }
    x.push_str("</MasterTrack>\n<Tracks>\n");

    // The string tuning of each staff of the instruments without strings.
    let tunings: Vec<Vec<Vec<u8>>> = song.tracks.iter().map(staff_tunings).collect();

    // Tracks.
    let mut chan = 0usize;
    for (k, t) in song.tracks.iter().enumerate() {
        let (ty, label) = inst_type(t.instrument);
        let drum = t.instrument == Instrument::Drums;
        // What was read with the track goes back in its place (see `gpkeep.rs`).
        let kept = t.kept.as_deref();
        let one_channel = kept.and_then(|k| k.one_channel(t)).unwrap_or("");
        let track_lines = kept
            .and_then(|k| k.bars_per_line(t))
            .unwrap_or("<SystemsDefautLayout>3</SystemsDefautLayout><SystemsLayout>1</SystemsLayout>");
        let _ = write!(
            x,
            "<Track id=\"{k}\"><Name>{}</Name><ShortName>{}</ShortName><Color>{} {} {}</Color>{track_lines}<PalmMute>{}</PalmMute><AutoAccentuation>{}</AutoAccentuation>{}{}<PlayingStyle>{}</PlayingStyle>{one_channel}<IconId>{}</IconId>",
            cdata(&t.name),
            cdata(&t.short_name),
            t.color[0],
            t.color[1],
            t.color[2],
            t.palm_mute,
            t.accentuation,
            if t.auto_let_ring {
                "<LetRingThroughout />"
            } else {
                ""
            },
            if t.auto_brush { "<AutoBrush />" } else { "" },
            // Only the style names seen in Guitar Pro's own files; the others are written as Default.
            match t.playing_style {
                1 | 2 => PLAYING_STYLES[t.playing_style as usize].1,
                _ => "Default",
            },
            crate::app::trackicons::icon_of(t)
        );
        // The instrument: its family and how notes are shown.
        if let Some(set) = kept.and_then(|k| k.instrument_set(t)) {
            x.push_str(set);
        } else if drum {
            write_drum_kit(&mut x);
        } else {
            let _ = write!(
                x,
                "<InstrumentSet><Name>{label}</Name><Type>{ty}</Type><LineCount>5</LineCount><Elements>"
            );
            x.push_str("<Element><Name>Pitched</Name><Type>pitched</Type><SoundbankName/><Articulations><Articulation><Name/><StaffLine>0</StaffLine><Noteheads>noteheadBlack noteheadHalf noteheadWhole</Noteheads><TechniquePlacement>outside</TechniquePlacement><TechniqueSymbol></TechniqueSymbol><InputMidiNumbers/><OutputRSESound/><OutputMidiNumber>0</OutputMidiNumber></Articulation></Articulations></Element>");
            x.push_str("</Elements></InstrumentSet>");
        }
        match kept.and_then(|k| k.transpose(t)) {
            Some(element) => x.push_str(element),
            None => {
                let _ = write!(
                    x,
                    "<Transpose><Chromatic>0</Chromatic><Octave>{}</Octave></Transpose>",
                    if t.clef().shift() != 0 { -1 } else { 0 }
                );
            }
        }
        let (prog, primary, secondary) = if drum {
            (0, 9, 9)
        } else {
            // Two channels per track, leaving out channel 10 (drums).
            let mut next = || {
                let c = chan % 15;
                chan += 1;
                if c >= 9 { c + 1 } else { c }
            };
            let (a, b) = (next(), next());
            (t.instrument.midi_program(), a, b)
        };
        match kept.and_then(|k| k.mixer(t)) {
            Some(element) => x.push_str(&element),
            None => {
                let params: Vec<String> = strip_values(t).iter().map(|v| format!("{v:.6}")).collect();
                let _ = write!(
                    x,
                    "<RSE><ChannelStrip version=\"E56\"><Parameters>{}</Parameters></ChannelStrip></RSE>",
                    params.join(" ")
                );
            }
        }
        x.push_str(kept.and_then(|k| k.forced_sound(t)).unwrap_or("<ForcedSound>-1</ForcedSound>"));
        // Keyboards, voices and orchestra take Guitar Pro's sounds too; Guitar Pro 8 finds the
        // right one for a stringed instrument from our name.
        let pitched = crate::gpinstr::pitched(t.instrument);
        let (sound, sound_name, patch) = match &pitched {
            _ if drum => (DRUM_SOUND.to_string(), label, "Drumkit-Master"),
            Some(g) => (g.path.to_string(), g.sound, g.patch),
            None => (format!("Midi/{label}"), label, ""),
        };
        let patch = if patch.is_empty() {
            String::new()
        } else {
            format!("<RSE><SoundbankPatch>{patch}</SoundbankPatch><Pickups><OverloudPosition>0</OverloudPosition><Volumes>1 1</Volumes><Tones>1 1</Tones></Pickups></RSE>")
        };
        match kept.and_then(|k| k.sounds(t)) {
            Some(element) => x.push_str(element),
            None => {
                let _ = write!(
                    x,
                    "<Sounds><Sound><Name>{sound_name}</Name><Label>{sound_name}</Label><Path>{sound}</Path><Role>Factory</Role><MIDI><LSB>0</LSB><MSB>0</MSB><Program>{prog}</Program></MIDI>{patch}</Sound></Sounds>"
                );
            }
        }
        match kept.and_then(|k| k.midi(t)) {
            Some(element) => x.push_str(element),
            None => {
                let _ = write!(
                    x,
                    "<MidiConnection><Port>0</Port><PrimaryChannel>{primary}</PrimaryChannel><SecondaryChannel>{secondary}</SecondaryChannel><ForeOneChannelPerString>false</ForeOneChannelPerString></MidiConnection>"
                );
            }
        }
        let state = if t.solo {
            "Solo"
        } else if t.mute {
            "Mute"
        } else {
            "Default"
        };
        let engine = kept.and_then(|k| k.engine(t)).unwrap_or("<AudioEngineState>MIDI</AudioEngineState>");
        let _ = write!(x, "<PlaybackState>{state}</PlaybackState>{engine}");
        // Lyrics: the first of five lines.
        x.push_str("<Lyrics dispatched=\"true\">");
        for line in 0..5 {
            let (text, from) = if line == 0 {
                (t.lyrics.as_str(), t.lyrics_bar)
            } else {
                ("", 0)
            };
            let _ = write!(
                x,
                "<Line><Text>{}</Text><Offset>{from}</Offset></Line>",
                cdata(text)
            );
        }
        x.push_str("</Lyrics><Staves>");
        let staves = if t.grand() { 2 } else { 1 };
        for staff in 0..staves {
            x.push_str("<Staff><Properties>");
            if let (Some(g), Some(tuning)) = (&pitched, tunings[k].get(staff)) {
                let pitches: Vec<String> = tuning.iter().map(|p| p.to_string()).collect();
                let _ = write!(
                    x,
                    "<Property name=\"CapoFret\"><Fret>0</Fret></Property><Property name=\"FretCount\"><Number>{}</Number></Property><Property name=\"PartialCapoFret\"><Fret>0</Fret></Property><Property name=\"PartialCapoStringFlags\"><Bitset>{}</Bitset></Property><Property name=\"Tuning\"><Pitches>{}</Pitches><Flat></Flat><Instrument>{}</Instrument><Label></Label><LabelVisible>true</LabelVisible></Property>",
                    crate::gpinstr::FRETS,
                    "0".repeat(tuning.len()),
                    pitches.join(" "),
                    if staff == 1 { "Bass" } else { g.tuned_as },
                );
            }
            if t.has_strings() {
                let pitches: Vec<String> = t.tuning.iter().rev().map(|p| p.to_string()).collect();
                let _ = write!(
                    x,
                    "<Property name=\"CapoFret\"><Fret>{}</Fret></Property><Property name=\"FretCount\"><Number>{}</Number></Property><Property name=\"PartialCapoFret\"><Fret>{}</Fret></Property><Property name=\"PartialCapoStringFlags\"><Bitset>{}</Bitset></Property><Property name=\"Tuning\"><Pitches>{}</Pitches>{}<Instrument>{}</Instrument><Label>{}</Label><LabelVisible>{}</LabelVisible></Property>",
                    t.capo,
                    t.frets,
                    t.partial_capo,
                    // One digit per string, the lowest string first.
                    (0..t.strings())
                        .rev()
                        .map(|k| if t.partial_capo_strings & (1 << k) != 0 {
                            '1'
                        } else {
                            '0'
                        })
                        .collect::<String>(),
                    pitches.join(" "),
                    if t.tuning_flat { "<Flat></Flat>" } else { "" },
                    if t.instrument == Instrument::Bass {
                        "Bass"
                    } else {
                        "Guitar"
                    },
                    cdata(&t.tuning_label),
                    t.tuning_label_visible
                );
            }
            // Chord names used by the track.
            let names = chord_names(t, o);
            x.push_str("<Property name=\"ChordCollection\"><Items/></Property><Property name=\"ChordWorkingSet\"><Items/></Property><Property name=\"DiagramCollection\"><Items>");
            for (i, n) in names.iter().enumerate() {
                let _ = write!(
                    x,
                    "<Item id=\"{i}\" name=\"{}\">{}</Item>",
                    n.replace('&', "&amp;")
                        .replace('"', "&quot;")
                        .replace('<', "&lt;"),
                    diagram_xml(t, n)
                );
            }
            x.push_str(
                "</Items></Property><Property name=\"DiagramWorkingSet\"><Items/></Property>",
            );
            if pitched.is_some() {
                let _ = write!(x, "<Property name=\"TuningFlat\"><Enable /></Property><Name>{}</Name>", cdata("Standard"));
            }
            if t.has_strings() {
                if t.tuning_flat {
                    x.push_str("<Property name=\"TuningFlat\"><Enable /></Property>");
                }
                let _ = write!(
                    x,
                    "<Name>{}</Name>",
                    cdata(if tuning_name(&t.tuning).contains('(') {
                        "Standard"
                    } else {
                        ""
                    })
                );
            }
            x.push_str("</Properties></Staff>");
        }
        let automations = match kept.and_then(|k| k.automations(t)) {
            Some(element) => element,
            None if kept.and_then(|k| k.sounds(t)).is_some() => String::new(),
            None => format!(
                "<Automations><Automation><Type>Sound</Type><Linear>false</Linear><Bar>0</Bar><Position>0</Position><Visible>true</Visible><Value>{}</Value></Automation></Automations>",
                cdata(&format!("{sound};{sound_name};Factory"))
            ),
        };
        let _ = writeln!(x, "</Staves>{automations}</Track>");
    }
    x.push_str("</Tracks>\n");

    // Bars, voices, beats and notes.
    let mut ids = Ids {
        beats: String::new(),
        notes: String::new(),
        rhythms: vec![],
        voices: String::new(),
        bars: String::new(),
        nbeats: 0,
        nnotes: 0,
        nvoices: 0,
        nbars: 0,
        legato_dest: false,
        kit: None,
        written_shift: None,
        beat_secs: 0,
    };
    let mut master = String::from("<MasterBars>\n");
    let track_chords: Vec<Vec<&String>> = song.tracks.iter().map(|t| chord_names(t, o)).collect();
    // For each track, the drum kit and the transposition kept from its file.
    let from_file: Vec<(Option<Kit>, Option<i32>)> = song
        .tracks
        .iter()
        .map(|t| {
            let kept = t.kept.as_deref();
            (kept.and_then(|k| k.kit(t)).map(<[_]>::to_vec), kept.and_then(|k| k.written_shift(t)))
        })
        .collect();
    for (bi, mb) in song.master.iter().enumerate() {
        let mut bar_ids = vec![];
        for (ti, ((t, chords), tunings)) in song.tracks.iter().zip(&track_chords).zip(&tunings).enumerate() {
            let key = mb.key;
            let staves: Vec<(Option<bool>, &str)> = if t.grand() {
                vec![(Some(false), "G2"), (Some(true), "F4")]
            } else {
                let clef = match t.clef() {
                    Clef::Percussion => "Neutral",
                    c if c.is_bass() => "F4",
                    _ => "G2",
                };
                vec![(None, clef)]
            };
            (ids.kit, ids.written_shift) = from_file[ti].clone();
            for (low, clef) in staves {
                let strings = tunings.get((low == Some(true)) as usize).map(Vec::as_slice);
                let bar = &t.bars[bi];
                // A repeat sign: two-bar repeats come in pairs, a first and a second.
                let paired = || t.bars[..bi].iter().rev().take_while(|b| b.simile == 2).count() % 2 == 1;
                let simile = match bar.simile {
                    0 => "",
                    1 => "<SimileMark>Simple</SimileMark>",
                    _ if paired() => "<SimileMark>SecondOfDouble</SimileMark>",
                    _ => "<SimileMark>FirstOfDouble</SimileMark>",
                };
                // Guitar Pro writes no voices under a repeat sign. Anything hidden under one
                // here is still written, so that it isn't lost.
                let bare = bar.simile != 0
                    && (0..VOICES).all(|v| bar.voice(v).iter().all(|b| b.is_rest() && b.chord.is_none() && b.text.is_none() && b.lyric.is_none()));
                let mut vids = vec![];
                for v in staff_voices(t, bar, low) {
                    let Some(v) = v.filter(|_| !bare) else {
                        vids.push("-1".to_string());
                        continue;
                    };
                    let mut beat_ids = vec![];
                    for (j, beat) in bar.voice(v).iter().enumerate() {
                        // On a grand staff each staff keeps the notes on its side of middle C.
                        let mut notes: Vec<&Note> = beat
                            .notes
                            .iter()
                            .filter(|n| on_staff(t.pitch(n), low))
                            .collect();
                        // Low string first, as Guitar Pro 8 lists a chord (it shows the first one in the numbered lane).
                        notes.sort_by_key(|n| std::cmp::Reverse(n.string));
                        let graced: Vec<Note> = notes
                            .iter()
                            .filter_map(|n| {
                                let g = n.grace?;
                                let mut m = Note::new(n.string, g);
                                m.tie = false;
                                Some(m)
                            })
                            .collect();
                        if !graced.is_empty() {
                            let gb = Beat {
                                notes: graced.clone(),
                                dynamic: beat.dynamic,
                                ..Beat::rest(Dur::ThirtySecond)
                            };
                            let refs: Vec<&Note> = graced.iter().collect();
                            beat_ids.push(write_beat_ex(
                                &mut ids, t, &gb, &refs, chords, key, false, true, strings,
                            ));
                        }
                        // Legato from the previous beat of this voice (maybe in the previous bar).
                        let prev = if j > 0 {
                            bar.voice(v).get(j - 1)
                        } else if bi > 0 {
                            t.bars[bi - 1].voice(v).last()
                        } else {
                            None
                        };
                        ids.legato_dest = prev.map(|p| p.notes.iter().any(|n| n.fx.legato)).unwrap_or(false);
                        if beat.fx.timer {
                            let before = bar.beat_start(v, j);
                            ids.beat_secs = seconds_at(song, bi, before);
                        }
                        beat_ids.push(write_beat_ex(
                            &mut ids,
                            t,
                            beat,
                            &notes,
                            chords,
                            key,
                            v == 0 && low != Some(true),
                            false,
                            strings,
                        ));
                    }
                    let vid = ids.nvoices;
                    ids.nvoices += 1;
                    let list: Vec<String> = beat_ids.iter().map(|b| b.to_string()).collect();
                    let _ = writeln!(
                        ids.voices,
                        "<Voice id=\"{vid}\"><Beats>{}</Beats></Voice>",
                        list.join(" ")
                    );
                    vids.push(vid.to_string());
                }
                let settings = bar.kept.as_deref().and_then(|k| k.settings((low == Some(true)) as usize)).unwrap_or("");
                let id = ids.nbars;
                ids.nbars += 1;
                let _ = writeln!(
                    ids.bars,
                    "<Bar id=\"{id}\"><Clef>{clef}</Clef>{simile}<Voices>{}</Voices>{settings}</Bar>",
                    vids.join(" ")
                );
                bar_ids.push(id.to_string());
            }
        }
        // The bar's settings shared by all tracks.
        let spelling = mb.kept.as_deref().and_then(|k| k.transpose_as(mb)).map(String::from).unwrap_or_else(|| {
            format!("<TransposeAs>{}</TransposeAs>", if mb.key < 0 { "Flats" } else { "Sharps" })
        });
        let _ = write!(
            master,
            "<MasterBar><Key><AccidentalCount>{}</AccidentalCount><Mode>{}</Mode>{spelling}</Key><Time>{}/{}</Time>",
            mb.key,
            if mb.key_minor { "Minor" } else { "Major" },
            mb.num,
            mb.den
        );
        if mb.double_bar {
            master.push_str("<DoubleBar/>");
        }
        if let Some(m) = &mb.marker {
            let _ = write!(
                master,
                "<Section><Letter>{}</Letter><Text>{}</Text></Section>",
                cdata(""),
                cdata(m)
            );
        }
        if mb.repeat_open || mb.repeat_close > 0 {
            let _ = write!(
                master,
                "<Repeat start=\"{}\" end=\"{}\" count=\"{}\"/>",
                mb.repeat_open,
                mb.repeat_close > 0,
                mb.repeat_close.max(if mb.repeat_close > 0 { 2 } else { 0 })
            );
        }
        if mb.endings != 0 {
            let list: Vec<String> = (0..8)
                .filter(|k| mb.endings & (1 << k) != 0)
                .map(|k| (k + 1).to_string())
                .collect();
            let _ = write!(
                master,
                "<AlternateEndings>{}</AlternateEndings>",
                list.join(" ")
            );
        }
        if mb.direction_target != 0 || mb.direction_jump != 0 {
            master.push_str("<Directions>");
            if let Some(t) = direction_target_name(mb.direction_target) {
                let _ = write!(master, "<Target>{t}</Target>");
            }
            if let Some(j) = direction_jump_name(mb.direction_jump) {
                let _ = write!(master, "<Jump>{j}</Jump>");
            }
            master.push_str("</Directions>");
        }
        if mb.triplet_feel {
            master.push_str("<TripletFeel>Triplet8th</TripletFeel>");
        }
        let _ = write!(master, "<Bars>{}</Bars>", bar_ids.join(" "));
        // Fermatas over the beats of the first track that has them.
        let fermatas: Vec<u32> = song
            .tracks
            .first()
            .map(|t| {
                let mut tick = 0;
                let mut v = vec![];
                for b in &t.bars[bi].beats {
                    if b.fermata {
                        v.push(tick);
                    }
                    tick += b.ticks();
                }
                v
            })
            .unwrap_or_default();
        if !fermatas.is_empty() {
            master.push_str("<Fermatas>");
            for tk in fermatas {
                let _ = write!(
                    master,
                    "<Fermata><Type>Medium</Type><Offset>{}/{}</Offset><Length>0.5</Length></Fermata>",
                    tk,
                    QUARTER * 4
                );
            }
            master.push_str("</Fermatas>");
        }
        master.push_str("</MasterBar>\n");
    }
    master.push_str("</MasterBars>\n");
    x.push_str(&master);
    let _ = write!(
        x,
        "<Bars>\n{}</Bars>\n<Voices>\n{}</Voices>\n<Beats>\n{}</Beats>\n<Notes>\n{}</Notes>\n<Rhythms>\n",
        ids.bars, ids.voices, ids.beats, ids.notes
    );
    for (i, (d, dot, dd, tup)) in ids.rhythms.iter().enumerate() {
        let _ = write!(
            x,
            "<Rhythm id=\"{i}\"><NoteValue>{}</NoteValue>",
            note_value(*d)
        );
        if *dd {
            x.push_str("<AugmentationDot count=\"2\"/>");
        } else if *dot {
            x.push_str("<AugmentationDot count=\"1\"/>");
        }
        if *tup > 0 {
            let den = match tup {
                3 => 2,
                5..=7 => 4,
                _ => 8,
            };
            let _ = write!(x, "<PrimaryTuplet num=\"{tup}\" den=\"{den}\"/>");
        }
        x.push_str("</Rhythm>\n");
    }
    x.push_str("</Rhythms>\n");
    if let Some(element) = song.kept.as_deref().and_then(|k| k.views_element(song)) {
        x.push_str(element);
    } else if o.views {
        x.push_str("<ScoreViews>");
        for k in 0..=song.tracks.len() {
            let _ = write!(x, "<ScoreView id=\"{k}\"/>");
        }
        x.push_str("</ScoreViews>\n");
    }
    // This app's stylesheet rides along as an XML comment inside the score, so
    // the file holds nothing other programs don't expect. Left out when standard.
    if song.style != crate::style::Stylesheet::standard()
        && let Ok(json) = serde_json::to_string(&song.style)
    {
        let _ = writeln!(x, "<!--{STYLE_MARK}{}-->", hex(json.as_bytes()));
    }
    // Marks only this app keeps, also as a hidden note.
    if let Some(c) = crate::extras::comment(song) {
        x.push_str(&c);
    }
    x.push_str("</GPIF>\n");
    x
}

/// Marks the stylesheet comment in score.gpif; the JSON follows in hexadecimal
/// (which can never contain "--", the one thing a comment may not hold).
pub const STYLE_MARK: &str = "GuitarCraftStylesheet:";

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Reads the stylesheet comment back from a score's XML, if there is one.
pub fn style_from_xml(xml: &str) -> Option<crate::style::Stylesheet> {
    let start = xml.find(STYLE_MARK)? + STYLE_MARK.len();
    let end = start + xml[start..].find("-->")?;
    let digits = xml[start..end].trim().as_bytes();
    let bytes: Option<Vec<u8>> = digits
        .chunks(2)
        .map(|c| std::str::from_utf8(c).ok().and_then(|t| u8::from_str_radix(t, 16).ok()))
        .collect();
    let st: crate::style::Stylesheet = serde_json::from_slice(&bytes?).ok()?;
    Some(st.fixed())
}

pub fn direction_target_name(t: u8) -> Option<&'static str> {
    match t {
        DIR_SEGNO => Some("Segno"),
        DIR_SEGNO_SEGNO => Some("SegnoSegno"),
        DIR_CODA => Some("Coda"),
        DIR_DOUBLE_CODA => Some("DoubleCoda"),
        DIR_FINE => Some("Fine"),
        _ => None,
    }
}

pub fn direction_jump_name(j: u8) -> Option<&'static str> {
    match j {
        1 => Some("DaCapo"),
        2 => Some("DaCapoAlFine"),
        3 => Some("DaCapoAlCoda"),
        4 => Some("DaCapoAlDoubleCoda"),
        5 => Some("DaSegno"),
        6 => Some("DaSegnoAlFine"),
        7 => Some("DaSegnoAlCoda"),
        8 => Some("DaSegnoSegno"),
        9 => Some("DaCoda"),
        10 => Some("DaDoubleCoda"),
        _ => None,
    }
}

#[allow(clippy::too_many_arguments)]
fn write_beat_ex(
    ids: &mut Ids,
    t: &Track,
    beat: &Beat,
    notes: &[&Note],
    chords: &[&String],
    key: i8,
    marks: bool,
    grace: bool,
    strings: Option<&[u8]>,
) -> usize {
    let id = ids.nbeats;
    ids.nbeats += 1;
    let r = ids.rhythm(beat);
    let mut x = format!(
        "<Beat id=\"{id}\"><Dynamic>{}</Dynamic><Rhythm ref=\"{r}\"/>",
        dyn_name(beat.dynamic)
    );
    if grace {
        x.push_str("<GraceNotes>BeforeBeat</GraceNotes>");
    }
    if marks {
        if let Some(c) = beat
            .chord
            .as_ref()
            .and_then(|c| chords.iter().position(|n| *n == c))
        {
            let _ = write!(x, "<Chord>{c}</Chord>");
        }
        if let Some(text) = &beat.text {
            let _ = write!(x, "<FreeText>{}</FreeText>", cdata(text));
        }
        if let Some(l) = &beat.lyric {
            let _ = write!(x, "<Lyrics><Line>{}</Line></Lyrics>", cdata(l));
        }
        match beat.hairpin {
            1 => x.push_str("<Hairpin>Crescendo</Hairpin>"),
            2 => x.push_str("<Hairpin>Decrescendo</Hairpin>"),
            _ => {}
        }
    }
    match beat.ottava {
        2 => x.push_str("<Ottavia>15ma</Ottavia>"),
        1 => x.push_str("<Ottavia>8va</Ottavia>"),
        -1 => x.push_str("<Ottavia>8vb</Ottavia>"),
        -2 => x.push_str("<Ottavia>15mb</Ottavia>"),
        _ => {}
    }
    if beat.tremolo {
        x.push_str("<Tremolo>1/8</Tremolo>");
    }
    if beat.fade_in {
        x.push_str("<Fadding>FadeIn</Fadding>");
    }
    // Legato, as Guitar Pro 8 writes it: on the beat the slur starts from
    // (origin) and on the beat it goes to (destination), just before the notes.
    let origin = !grace && notes.iter().any(|n| n.fx.legato);
    let dest = !grace && std::mem::take(&mut ids.legato_dest);
    if origin || dest {
        let _ = write!(x, "<Legato origin=\"{origin}\" destination=\"{dest}\" />");
    }
    // Wah-wah, as Guitar Pro 8 writes it: <Wah>Open</Wah> or <Wah>Closed</Wah>.
    match beat.fx.wah {
        1 => x.push_str("<Wah>Open</Wah>"),
        2 => x.push_str("<Wah>Closed</Wah>"),
        _ => {}
    }
    // Arpeggio, as Guitar Pro 8 writes it: <Arpeggio>Down</Arpeggio> (low
    // string to high) or <Arpeggio>Up</Arpeggio>.
    let arpeggio = if grace { 0 } else { beat.fx.arpeggio };
    match arpeggio {
        1 => x.push_str("<Arpeggio>Down</Arpeggio>"),
        2 => x.push_str("<Arpeggio>Up</Arpeggio>"),
        _ => {}
    }
    // Golpe, as Guitar Pro 8 writes it: <Golpe>Finger</Golpe> or <Golpe>Thumb</Golpe>.
    if !grace {
        match beat.fx.golpe {
            1 => x.push_str("<Golpe>Finger</Golpe>"),
            2 => x.push_str("<Golpe>Thumb</Golpe>"),
            _ => {}
        }
    }
    // Timer, as Guitar Pro 8 writes it: <Timer>time</Timer>. The person's file
    // had one on the first beat (0), so the unit isn't known yet; whole
    // seconds are written, as the timer shows minutes and seconds.
    if !grace && beat.fx.timer {
        let _ = write!(x, "<Timer>{}</Timer>", ids.beat_secs);
    }
    if !notes.is_empty() {
        let mut nids = vec![];
        // Where each note sits, for a staff Guitar Pro tunes.
        let sounding: Vec<u8> = notes.iter().map(|n| t.pitch(n)).collect();
        let places = strings.and_then(|s| crate::gpinstr::place(s, &sounding)).unwrap_or_default();
        for (k, n) in notes.iter().enumerate() {
            nids.push(write_note(ids, t, beat, n, key, places.get(k).copied()).to_string());
        }
        let _ = write!(x, "<Notes>{}</Notes>", nids.join(" "));
    }
    let mut props = String::new();
    match beat.stroke {
        Stroke::Down => {
            props.push_str("<Property name=\"Brush\"><Direction>Down</Direction></Property>")
        }
        Stroke::Up => {
            props.push_str("<Property name=\"Brush\"><Direction>Up</Direction></Property>")
        }
        Stroke::None => {}
    }
    match beat.pick {
        1 => props.push_str("<Property name=\"PickStroke\"><Direction>Down</Direction></Property>"),
        2 => props.push_str("<Property name=\"PickStroke\"><Direction>Up</Direction></Property>"),
        _ => {}
    }
    match beat.technique {
        1 => props.push_str("<Property name=\"Tapped\"><Enable/></Property>"),
        2 => props.push_str("<Property name=\"Slapped\"><Enable/></Property>"),
        3 => props.push_str("<Property name=\"Popped\"><Enable/></Property>"),
        _ => {}
    }
    if !props.is_empty() {
        let _ = write!(x, "<Properties>{props}</Properties>");
    }
    // With an arpeggio Guitar Pro 8 adds two settings after the properties
    // (most likely its speed and start, from the Arpeggio window); these are
    // the values it wrote in the person's file (479 down, 478 up, and 1).
    // The person's pictures of the Arpeggio window show what they are: the
    // duration (480 to the quarter note) and the start time (1 starts on time,
    // 0 falls on time).
    if arpeggio != 0 {
        let len = match beat.fx.arp_len {
            0 => if arpeggio == 1 { 479 } else { 478 },
            n => n,
        };
        let start = (1.0 - beat.fx.arp_falls).clamp(0.0, 1.0);
        let start = format!("{start:.6}").trim_end_matches('0').trim_end_matches('.').to_string();
        let _ = write!(
            x,
            "<XProperties><XProperty id=\"687931393\"><Int>{len}</Int></XProperty><XProperty id=\"687931394\"><Float>{start}</Float></XProperty></XProperties>"
        );
    }
    x.push_str("</Beat>\n");
    ids.beats.push_str(&x);
    id
}

/// Whole seconds from the start of the song (first time through) to `tick`
/// ticks into bar `bi`.
fn seconds_at(song: &Song, bi: usize, tick: u32) -> u32 {
    let secs = |b: usize, ticks: u32| ticks as f64 / QUARTER as f64 * 60.0 / song.tempo_at(b).max(1) as f64;
    let total: f64 = (0..bi).map(|b| secs(b, song.master[b].ticks())).sum::<f64>() + secs(bi, tick);
    total.floor() as u32
}

/// Guitar Pro's letter for a finger (1 thumb ... 5 little finger).
pub fn finger_letter(f: u8) -> Option<&'static str> {
    ["P", "I", "M", "A", "C"].get((f as usize).checked_sub(1)?).copied()
}

/// A finger from Guitar Pro's letter (0 = none).
pub fn finger_from_letter(s: &str) -> u8 {
    match s.trim() {
        "P" | "p" | "T" | "t" => 1,
        "I" | "i" => 2,
        "M" | "m" => 3,
        "A" | "a" => 4,
        "C" | "c" | "E" | "e" | "X" | "x" | "O" | "o" => 5,
        _ => 0,
    }
}

fn write_note(ids: &mut Ids, t: &Track, beat: &Beat, n: &Note, key: i8, place: Option<(usize, u8)>) -> usize {
    let id = ids.nnotes;
    ids.nnotes += 1;
    let pitch = t.pitch(n) as i32;
    let mut x = format!("<Note id=\"{id}\">");
    // Fingerings, as Guitar Pro 8 writes them: first in the note, one letter
    // for both hands (P thumb, I index, M middle, A ring, C little finger).
    if let Some(f) = finger_letter(n.fx.lh_finger) {
        let _ = write!(x, "<LeftFingering>{f}</LeftFingering>");
    }
    if let Some(f) = finger_letter(n.fx.rh_finger) {
        let _ = write!(x, "<RightFingering>{f}</RightFingering>");
    }
    if n.ghost {
        x.push_str("<AntiAccent>Normal</AntiAccent>");
    }
    let acc = (if n.staccato { 1 } else { 0 })
        | (if n.heavy_accent { 4 } else { 0 })
        | (if n.accent { 8 } else { 0 });
    if acc != 0 {
        let _ = write!(x, "<Accent>{acc}</Accent>");
    }
    if t.instrument == Instrument::Drums {
        let _ = write!(
            x,
            "<InstrumentArticulation>{}</InstrumentArticulation>",
            ids.kit.as_ref().and_then(|k| k.iter().position(|&s| s == Some(pitch as u8))).unwrap_or_else(|| drum_articulation(pitch as u8))
        );
    }
    if n.let_ring {
        x.push_str("<LetRing/>");
    }
    if n.vibrato || n.wide_vibrato {
        let _ = write!(
            x,
            "<Vibrato>{}</Vibrato>",
            if n.wide_vibrato { "Wide" } else { "Slight" }
        );
    }
    if n.tie {
        x.push_str("<Tie origin=\"false\" destination=\"true\"/>");
    }
    if n.trill > 0 {
        let _ = write!(
            x,
            "<Trill>{}</Trill>",
            (pitch + n.trill as i32 - n.fret as i32).clamp(0, 127)
        );
    }
    x.push_str("<Properties>");
    let written = pitch + ids.written_shift.unwrap_or_else(|| t.clef().shift());
    let (step, accd, oct) = pitch_parts(pitch);
    let (wstep, waccd, woct) = pitch_parts(written);
    // Guitar Pro numbers the octaves of these instruments one higher.
    let raise = crate::gpinstr::pitched(t.instrument).is_some() as i32;
    let (oct, woct) = (oct + raise, woct + raise);
    let _ = key;
    let _ = write!(
        x,
        "<Property name=\"ConcertPitch\"><Pitch><Step>{step}</Step><Accidental>{accd}</Accidental><Octave>{oct}</Octave></Pitch></Property>"
    );
    let _ = write!(
        x,
        "<Property name=\"TransposedPitch\"><Pitch><Step>{wstep}</Step><Accidental>{waccd}</Accidental><Octave>{woct}</Octave></Pitch></Property>"
    );
    let _ = write!(
        x,
        "<Property name=\"Midi\"><Number>{pitch}</Number></Property>"
    );
    if let Some((string, fret)) = place {
        let _ = write!(x, "<Property name=\"String\"><String>{string}</String></Property><Property name=\"Fret\"><Fret>{fret}</Fret></Property>");
    }
    if t.has_strings() {
        let nstr = t.strings();
        let _ = write!(
            x,
            "<Property name=\"String\"><String>{}</String></Property><Property name=\"Fret\"><Fret>{}</Fret></Property>",
            nstr - 1 - (n.string as usize).min(nstr - 1),
            n.fret
        );
    }
    // A pick scrape is written on each note as a muted note (as Guitar Pro 8 does).
    if n.dead || beat.fx.scrape != 0 {
        x.push_str("<Property name=\"Muted\"><Enable/></Property>");
    }
    if n.palm_mute {
        x.push_str("<Property name=\"PalmMuted\"><Enable/></Property>");
    }
    if n.hammer {
        x.push_str("<Property name=\"HopoOrigin\"><Enable/></Property>");
    }
    if n.harmonic || n.pinch_harmonic {
        let _ = write!(
            x,
            "<Property name=\"HarmonicType\"><HType>{}</HType></Property><Property name=\"HarmonicFret\"><HFret>12</HFret></Property>",
            if n.pinch_harmonic { "Pinch" } else { "Natural" }
        );
    }
    // Slides are bits of one number: 1 shift slide, 2 legato slide, 4 out
    // downwards, 8 out upwards, 16 in from below, 32 in from above, 64 pick
    // scrape downwards, 128 pick scrape upwards. 64 and 128 are confirmed by a
    // file the person saved in Guitar Pro 8; the others follow the public
    // descriptions of the format.
    let flags = (if n.fx.shift_slide { 1 } else { 0 })
        | (if n.slide { 2 } else { 0 })
        | (if n.slide_out { 4 } else { 0 })
        | (if n.fx.slide_out_up { 8 } else { 0 })
        | (if n.slide_in { 16 } else { 0 })
        | (if n.fx.slide_in_above { 32 } else { 0 })
        | match beat.fx.scrape {
            1 => 128,
            2 => 64,
            _ => 0,
        };
    if flags != 0 {
        let _ = write!(
            x,
            "<Property name=\"Slide\"><Flags>{flags}</Flags></Property>"
        );
    }
    if n.bend > 0 {
        let v = n.bend as f32 * 25.0;
        let (origin, middle, dest) = match n.bend_kind {
            1 => (0.0, v, 0.0),
            2 => (v, v, v),
            3 => (v, v, 0.0),
            _ => (0.0, v, v),
        };
        let _ = write!(
            x,
            "<Property name=\"Bended\"><Enable/></Property><Property name=\"BendOriginValue\"><Float>{origin}</Float></Property><Property name=\"BendMiddleValue\"><Float>{middle}</Float></Property><Property name=\"BendDestinationValue\"><Float>{dest}</Float></Property><Property name=\"BendOriginOffset\"><Float>0</Float></Property><Property name=\"BendMiddleOffset1\"><Float>12</Float></Property><Property name=\"BendMiddleOffset2\"><Float>12</Float></Property><Property name=\"BendDestinationOffset\"><Float>100</Float></Property>"
        );
    }
    let _ = beat;
    x.push_str("</Properties></Note>\n");
    ids.notes.push_str(&x);
    id
}

/// Which staves each track shows, for Guitar Pro's part settings: one view of
/// all tracks, then one view per track. Each track: 1 notation, 2 tablature.
pub(crate) fn part_configuration(song: &Song) -> Vec<u8> {
    let flags: Vec<u8> = song
        .tracks
        .iter()
        .map(|t| {
            let (std, tab) = crate::score::staves(t, crate::score::Notation::Both);
            (if std { 1 } else { 0 }) | (if tab { 2 } else { 0 })
        })
        .collect();
    let mut b = vec![];
    b.extend_from_slice(&(1 + flags.len() as i32).to_be_bytes());
    b.push(0);
    b.extend_from_slice(&(flags.len() as i32).to_be_bytes());
    b.extend_from_slice(&flags);
    for (f, t) in flags.iter().zip(&song.tracks) {
        b.push(t.multirest as u8);
        b.extend_from_slice(&1i32.to_be_bytes());
        b.push(*f);
    }
    b.extend_from_slice(&0i32.to_be_bytes());
    b
}

/// The layout settings record, as in a .gp file Guitar Pro 8 saved for the
/// person (its meaning isn't documented; it holds no song content).
pub(crate) const LAYOUT_CONFIGURATION: [u8; 14] = [0, 0, 0, 4, 1, 0, 0, 0xff, 0xff, 0xff, 0, 0, 0, 1];

/// The whole .gp file.
pub fn export_gp(song: &Song) -> Result<Vec<u8>, String> {
    export_gp_with(song, GpOptions::safe(song))
}

pub fn export_gp_with(song: &Song, o: GpOptions) -> Result<Vec<u8>, String> {
    let xml = gpif_xml_with(song, o);
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut z = zip::ZipWriter::new(&mut buf);
        let opt = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        let e = |e: zip::result::ZipError| e.to_string();
        z.start_file("VERSION", opt).map_err(e)?;
        z.write_all(b"7.0").map_err(|e| e.to_string())?;
        z.start_file("Content/score.gpif", opt).map_err(e)?;
        z.write_all(xml.as_bytes()).map_err(|e| e.to_string())?;
        // Views kept from the file that was opened, if they still describe the song.
        let views = song.kept.as_deref().and_then(|k| k.view_files(song));
        let has = |name: &str| views.as_ref().is_some_and(|v| v.iter().any(|f| f.0 == name));
        if !has("Content/PartConfiguration") {
            z.start_file("Content/PartConfiguration", opt).map_err(e)?;
            z.write_all(&part_configuration(song))
                .map_err(|e| e.to_string())?;
        }
        // The stylesheet and the layout settings Guitar Pro's own files carry.
        z.start_file("Content/BinaryStylesheet", opt).map_err(e)?;
        let kept = song.kept.as_deref();
        let stylesheet = kept.and_then(|k| k.stylesheet(song)).unwrap_or_else(|| crate::gpstyle::write(&song.style));
        z.write_all(&stylesheet).map_err(|e| e.to_string())?;
        if !has("Content/LayoutConfiguration") {
            z.start_file("Content/LayoutConfiguration", opt).map_err(e)?;
            z.write_all(&LAYOUT_CONFIGURATION).map_err(|e| e.to_string())?;
        }
        for (name, data) in kept.map(|k| k.files.as_slice()).unwrap_or_default() {
            z.start_file(name.as_str(), opt).map_err(e)?;
            z.write_all(data).map_err(|e| e.to_string())?;
        }
        // Score views: the full score lists only the tracks that are shown.
        let staves_of = |k: usize| -> Vec<(usize, usize)> {
            let (std, tab) = crate::score::staves(&song.tracks[k], crate::score::Notation::Both);
            let mut v = vec![];
            if std {
                v.push((k, 0));
            }
            if tab {
                v.push((k, 1));
            }
            v
        };
        let mut shown: Vec<usize> = (0..song.tracks.len())
            .filter(|&k| song.tracks[k].visible)
            .collect();
        if shown.is_empty() {
            shown = (0..song.tracks.len()).collect();
        }
        let full: Vec<(usize, usize)> = shown.iter().flat_map(|&k| staves_of(k)).collect();
        if let Some(views) = &views {
            for &(name, data) in views {
                z.start_file(name, opt).map_err(e)?;
                z.write_all(data).map_err(|e| e.to_string())?;
            }
        }
        if views.is_some() || !o.views {
            z.finish().map_err(e)?;
            return Ok(buf.into_inner());
        }
        z.start_file("Content/ScoreViews/0.gpsv", opt).map_err(e)?;
        z.write_all(&crate::gpsv::view("Full score", &full, song.bar_count(), 4))
            .map_err(|e| e.to_string())?;
        for k in 0..song.tracks.len() {
            z.start_file(format!("Content/ScoreViews/{}.gpsv", k + 1), opt)
                .map_err(e)?;
            z.write_all(&crate::gpsv::view(
                &song.tracks[k].name,
                &staves_of(k),
                song.bar_count(),
                4,
            ))
            .map_err(|e| e.to_string())?;
        }
        z.finish().map_err(e)?;
    }
    Ok(buf.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pitches(t: &Track, b: &Beat) -> Vec<u8> {
        let mut v: Vec<u8> = b.notes.iter().map(|n| t.pitch(n)).collect();
        v.sort();
        v
    }

    /// What made Guitar Pro 8 open our files (checked by the person on 12 Nov
    /// 2026: the old layout crashed it, this one opens): the stylesheet and
    /// layout parts are present, and every chord name has a diagram.
    #[test]
    fn saves_what_guitar_pro_8_needs() {
        let data = export_gp(&Song::demo()).unwrap();
        let mut z = zip::ZipArchive::new(std::io::Cursor::new(&data[..])).unwrap();
        for name in ["VERSION", "Content/score.gpif", "Content/PartConfiguration", "Content/BinaryStylesheet", "Content/LayoutConfiguration"] {
            assert!(z.by_name(name).is_ok(), "missing {name}");
        }
        let mut xml = String::new();
        std::io::Read::read_to_string(&mut z.by_name("Content/score.gpif").unwrap(), &mut xml).unwrap();
        let doc = roxmltree::Document::parse(&xml).unwrap();
        let items: Vec<_> = doc.descendants().filter(|n| n.has_tag_name("Item") && n.attribute("name").is_some()).collect();
        assert!(!items.is_empty());
        assert!(items.iter().all(|it| it.children().any(|c| c.has_tag_name("Diagram"))));
        // Each track's details in Guitar Pro's order.
        let pos = |t: &str| xml.find(t).unwrap();
        assert!(pos("<ChannelStrip") < pos("<ForcedSound>") && pos("<ForcedSound>") < pos("<Sounds>") && pos("<Sounds>") < pos("<MidiConnection>"));
    }

    /// Legato as Guitar Pro 8 writes it (from a file the person saved in
    /// Guitar Pro 8 with legato on the first two bass notes): the slur's first
    /// beat has origin="true", the next destination="true".
    #[test]
    fn legato_written_like_guitar_pro() {
        let mut song = Song::demo();
        let bass = 1;
        for n in &mut song.tracks[bass].bars[0].beats[0].notes {
            n.fx.legato = true;
        }
        let xml = gpif_xml(&song);
        assert!(xml.contains("<Legato origin=\"true\" destination=\"false\" /><Notes>"));
        assert!(xml.contains("<Legato origin=\"false\" destination=\"true\" /><Notes>"));
        let back = crate::gpif::import_gp(&export_gp(&song).unwrap()).unwrap();
        assert!(back.tracks[bass].bars[0].beats[0].notes.iter().all(|n| n.fx.legato));
        assert!(back.tracks[bass].bars[0].beats[1].notes.iter().all(|n| !n.fx.legato));
    }

    /// Wah as Guitar Pro 8 writes it (from the person's GP8 file with an open
    /// wah on the first bass note and a closed one on the second).
    #[test]
    fn wah_written_like_guitar_pro() {
        let mut song = Song::demo();
        song.tracks[1].bars[0].beats[0].fx.wah = 1;
        song.tracks[1].bars[0].beats[1].fx.wah = 2;
        let xml = gpif_xml(&song);
        assert!(xml.contains("<Wah>Open</Wah><Notes>"));
        assert!(xml.contains("<Wah>Closed</Wah><Notes>"));
        assert!(!xml.contains("GuitarCraftExtras"));
        let back = crate::gpif::import_gp(&export_gp(&song).unwrap()).unwrap();
        assert_eq!(back.tracks[1].bars[0].beats[0].fx.wah, 1);
        assert_eq!(back.tracks[1].bars[0].beats[1].fx.wah, 2);
        // And from the file Guitar Pro 8 saved.
        let data = crate::fixture("gp8/Morning Walk - wah.gp");
        {
            let gp = crate::gpif::import_gp(&data).unwrap();
            let bass = gp.tracks.iter().position(|t| t.instrument == Instrument::Bass).unwrap();
            assert_eq!(gp.tracks[bass].bars[0].beats[0].fx.wah, 1);
            assert_eq!(gp.tracks[bass].bars[0].beats[1].fx.wah, 2);
        }
    }

    /// The track equalizer, checked against Guitar Pro 8 files: "code A" and "code B" set
    /// each knob to a different pair of (left, default, right), which shows which number is
    /// which knob; "default", "max" and "min" have every knob at one place.
    #[test]
    fn track_equalizer_read_and_written_like_guitar_pro() {
        let read = |name: &str| {
            let song = crate::gpif::import_gp(&crate::fixture(&format!("gp8/{name}"))).unwrap();
            song.tracks.iter().find(|t| t.equalizer.on).expect("a track with the EQ on").equalizer
        };
        let close = |a: [f32; 9], b: [f32; 9]| {
            a.iter().zip(b).all(|(x, y)| (x - y).abs() < 0.02)
        };
        let e = read("Morning Walk - track eq code A.gp");
        assert!(close(e.knobs, [0.0, 0.0, 0.0, 0.5, 0.5, 1.0, 1.0, 1.0, 0.5]), "{e:?}");
        let e = read("Morning Walk - track eq code B.gp");
        assert!(close(e.knobs, [0.0, 0.5, 1.0, 0.0, 1.0, 0.0, 0.5, 1.0, 0.5]), "{e:?}");
        // Every knob at its default, fully right, fully left.
        for (name, v) in [("default", 0.5), ("max", 1.0), ("min", 0.0)] {
            let e = read(&format!("Morning Walk - track eq {name}.gp"));
            assert!(close(e.knobs, [v; 9]), "{name}: {e:?}");
        }
        // Tooltip values from Guitar Pro 8: the Hz and Q knobs' ends and defaults.
        assert!((eq_knob_value(2, 0.0) - 45.0).abs() < 0.5);
        assert!((eq_knob_value(2, 0.5) - 367.0).abs() < 1.0);
        assert!((eq_knob_value(2, 1.0) - 3000.0).abs() < 1.0);
        assert!((eq_knob_value(5, 0.5) - 900.0).abs() < 1.0);
        assert!((eq_knob_value(5, 1.0) - 18000.0).abs() < 1.0);
        assert!((eq_knob_value(7, 0.5) - 3000.0).abs() < 1.0);
        assert!((eq_knob_value(3, 0.0) - 0.5).abs() < 0.001);
        assert!((eq_knob_value(3, 0.5) - 1.2).abs() < 0.03);
        assert!((eq_knob_value(3, 1.0) - 3.0).abs() < 0.001);
        assert!((eq_knob_value(0, 1.0) - 15.0).abs() < 0.01);
        // Written back the same way.
        let mut song = Song::demo();
        song.tracks[0].equalizer =
            Equalizer { on: true, knobs: [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9] };
        let xml = gpif_xml(&song);
        assert!(xml.contains(
            "<Parameters>0.900000 0.700000 0.800000 0.500000 0.600000 0.400000 0.200000 0.300000 0.100000 1.000000 "
        ));
        let back = crate::gpif::import_gp(&export_gp(&song).unwrap()).unwrap();
        assert_eq!(back.tracks[0].equalizer, song.tracks[0].equalizer);
        assert!(!back.tracks[1].equalizer.on);
    }

    /// Arpeggio as Guitar Pro 8 writes it (from the person's GP8 file with a
    /// downward arpeggio on the first bass note and an upward one on the second).
    #[test]
    fn arpeggio_written_like_guitar_pro() {
        let mut song = Song::demo();
        song.tracks[1].bars[0].beats[0].fx.arpeggio = 1;
        song.tracks[1].bars[0].beats[1].fx.arpeggio = 2;
        let xml = gpif_xml(&song);
        assert!(xml.contains("<Arpeggio>Down</Arpeggio><Notes>"));
        assert!(xml.contains("<Arpeggio>Up</Arpeggio><Notes>"));
        assert!(xml.contains("<XProperty id=\"687931393\"><Int>479</Int></XProperty>"));
        assert!(!xml.contains("GuitarCraftExtras"));
        assert!(roxmltree::Document::parse(&xml).is_ok());
        let back = crate::gpif::import_gp(&export_gp(&song).unwrap()).unwrap();
        assert_eq!(back.tracks[1].bars[0].beats[0].fx.arpeggio, 1);
        assert_eq!(back.tracks[1].bars[0].beats[1].fx.arpeggio, 2);
        let data = crate::fixture("gp8/Morning Walk - Arpeggio.gp");
        {
            let gp = crate::gpif::import_gp(&data).unwrap();
            let bass = gp.tracks.iter().position(|t| t.instrument == Instrument::Bass).unwrap();
            assert_eq!(gp.tracks[bass].bars[0].beats[0].fx.arpeggio, 1);
            assert_eq!(gp.tracks[bass].bars[0].beats[1].fx.arpeggio, 2);
        }
    }

    /// The arpeggio's duration and start time (from the Arpeggio window) are
    /// kept in the XProperties Guitar Pro writes.
    #[test]
    fn arpeggio_settings_saved() {
        let mut song = Song::demo();
        let b = &mut song.tracks[0].bars[0].beats[0];
        b.fx.arpeggio = 2;
        b.fx.arp_len = 240;
        b.fx.arp_falls = 1.0;
        let xml = gpif_xml(&song);
        assert!(xml.contains("<XProperty id=\"687931393\"><Int>240</Int></XProperty><XProperty id=\"687931394\"><Float>0</Float></XProperty>"));
        let back = crate::gpif::import_gp(&export_gp(&song).unwrap()).unwrap();
        let fx = &back.tracks[0].bars[0].beats[0].fx;
        assert_eq!((fx.arpeggio, fx.arp_len, fx.arp_falls), (2, 240, 1.0));
        // The person's file: a quarter-note arpeggio starting on time.
        let data = crate::fixture("gp8/Morning Walk - Arpeggio.gp");
        {
            let gp = crate::gpif::import_gp(&data).unwrap();
            let bass = gp.tracks.iter().position(|t| t.instrument == Instrument::Bass).unwrap();
            let fx = &gp.tracks[bass].bars[0].beats[0].fx;
            assert_eq!((fx.arp_len, fx.arp_falls), (479, 0.0));
        }
    }

    /// Golpe as Guitar Pro 8 writes it (from the person's GP8 file with a
    /// finger golpe on the first bass note and a thumb golpe on the second).
    #[test]
    fn golpe_written_like_guitar_pro() {
        let mut song = Song::demo();
        song.tracks[1].bars[0].beats[0].fx.golpe = 1;
        song.tracks[1].bars[0].beats[1].fx.golpe = 2;
        let xml = gpif_xml(&song);
        assert!(xml.contains("<Golpe>Finger</Golpe><Notes>"));
        assert!(xml.contains("<Golpe>Thumb</Golpe><Notes>"));
        assert!(!xml.contains("GuitarCraftExtras"));
        let back = crate::gpif::import_gp(&export_gp(&song).unwrap()).unwrap();
        assert_eq!(back.tracks[1].bars[0].beats[0].fx.golpe, 1);
        assert_eq!(back.tracks[1].bars[0].beats[1].fx.golpe, 2);
        let data = crate::fixture("gp8/Morning Walk - golpe.gp");
        {
            let gp = crate::gpif::import_gp(&data).unwrap();
            let bass = gp.tracks.iter().position(|t| t.instrument == Instrument::Bass).unwrap();
            assert_eq!(gp.tracks[bass].bars[0].beats[0].fx.golpe, 1);
            assert_eq!(gp.tracks[bass].bars[0].beats[1].fx.golpe, 2);
        }
    }

    /// Pick scrape as Guitar Pro 8 writes it (from the person's GP8 file with
    /// pick scrapes on the first two bass notes): each note muted, with slide
    /// flag 64 (downwards) or 128 (upwards).
    #[test]
    fn pick_scrape_written_like_guitar_pro() {
        let mut song = Song::demo();
        song.tracks[1].bars[0].beats[0].fx.scrape = 2;
        song.tracks[1].bars[0].beats[1].fx.scrape = 1;
        let xml = gpif_xml(&song);
        assert!(xml.contains("<Property name=\"Slide\"><Flags>64</Flags></Property>"));
        assert!(xml.contains("<Property name=\"Slide\"><Flags>128</Flags></Property>"));
        assert!(!xml.contains("GuitarCraftExtras"));
        let back = crate::gpif::import_gp(&export_gp(&song).unwrap()).unwrap();
        let b = &back.tracks[1].bars[0].beats;
        assert_eq!((b[0].fx.scrape, b[1].fx.scrape), (2, 1));
        assert!(!b[0].notes[0].dead);
        // Other slides round-trip through the same flags.
        let mut song = Song::demo();
        let n = &mut song.tracks[0].bars[0].beats[0].notes[0];
        n.fx.shift_slide = true;
        n.fx.slide_in_above = true;
        n.fx.slide_out_up = true;
        let back = crate::gpif::import_gp(&export_gp(&song).unwrap()).unwrap();
        let m = &back.tracks[0].bars[0].beats[0].notes[0];
        assert!(m.fx.shift_slide && m.fx.slide_in_above && m.fx.slide_out_up && !m.slide);
        let data = crate::fixture("gp8/Morning Walk - pick scrape.gp");
        {
            let gp = crate::gpif::import_gp(&data).unwrap();
            let bass = gp.tracks.iter().position(|t| t.instrument == Instrument::Bass).unwrap();
            let b = &gp.tracks[bass].bars[0].beats;
            assert_eq!((b[0].fx.scrape, b[1].fx.scrape), (2, 1));
        }
    }

    /// Fingerings as Guitar Pro 8 writes them (from the person's GP8 file with
    /// left-hand C, A, M, I, P then right-hand P, I, M, A, C on the bass notes).
    #[test]
    fn fingering_written_like_guitar_pro() {
        let mut song = Song::demo();
        song.tracks[1].bars[0].beats[0].notes[0].fx.lh_finger = 5;
        song.tracks[1].bars[0].beats[1].notes[0].fx.rh_finger = 2;
        let xml = gpif_xml(&song);
        assert!(xml.contains("><LeftFingering>C</LeftFingering>"));
        assert!(xml.contains("><RightFingering>I</RightFingering>"));
        assert!(!xml.contains("GuitarCraftExtras"));
        let back = crate::gpif::import_gp(&export_gp(&song).unwrap()).unwrap();
        assert_eq!(back.tracks[1].bars[0].beats[0].notes[0].fx.lh_finger, 5);
        assert_eq!(back.tracks[1].bars[0].beats[1].notes[0].fx.rh_finger, 2);
        let data = crate::fixture("gp8/Morning Walk - fingering.gp");
        {
            let gp = crate::gpif::import_gp(&data).unwrap();
            let bass = gp.tracks.iter().position(|t| t.instrument == Instrument::Bass).unwrap();
            let f: Vec<(u8, u8)> = gp.tracks[bass].bars[..5]
                .iter()
                .flat_map(|b| b.beats.iter().take(2).map(|x| (x.notes[0].fx.lh_finger, x.notes[0].fx.rh_finger)))
                .collect();
            assert_eq!(f, vec![(5, 0), (4, 0), (3, 0), (2, 0), (1, 0), (0, 1), (0, 2), (0, 3), (0, 4), (0, 5)]);
        }
    }

    /// Timer as Guitar Pro 8 writes it (from the person's GP8 file with a timer
    /// on the first bass note: <Timer>0</Timer>).
    #[test]
    fn timer_written_like_guitar_pro() {
        let mut song = Song::demo();
        song.tracks[1].bars[0].beats[0].fx.timer = true;
        // Bar 3 starts after two bars of 4/4 at 96 bpm: 5 seconds.
        song.tracks[1].bars[2].beats[0].fx.timer = true;
        let xml = gpif_xml(&song);
        assert!(xml.contains("<Timer>0</Timer><Notes>"));
        assert!(xml.contains("<Timer>5</Timer><Notes>"));
        assert!(!xml.contains("GuitarCraftExtras"));
        let back = crate::gpif::import_gp(&export_gp(&song).unwrap()).unwrap();
        assert!(back.tracks[1].bars[0].beats[0].fx.timer && back.tracks[1].bars[2].beats[0].fx.timer);
        let data = crate::fixture("gp8/Morning Walk - timer.gp");
        {
            let gp = crate::gpif::import_gp(&data).unwrap();
            let bass = gp.tracks.iter().position(|t| t.instrument == Instrument::Bass).unwrap();
            assert!(gp.tracks[bass].bars[0].beats[0].fx.timer);
        }
    }

    /// A bar repeated on its own, as Guitar Pro 8 writes it (from the person's
    /// GP8 file with bar 6 played 4 times): repeat start and end on the same bar.
    #[test]
    fn single_bar_repeat_like_guitar_pro() {
        let mut song = Song::demo();
        song.master[5].repeat_open = true;
        song.master[5].repeat_close = 4;
        let xml = gpif_xml(&song);
        assert!(xml.contains("<Repeat start=\"true\" end=\"true\" count=\"4\"/>"));
        let back = crate::gpif::import_gp(&export_gp(&song).unwrap()).unwrap();
        assert!(back.master[5].repeat_open && back.master[5].repeat_close == 4);
        // Played four times in a row.
        let order = back.play_order();
        let start = order.iter().position(|&b| b == 5).unwrap();
        assert_eq!(&order[start..start + 5], &[5, 5, 5, 5, 6]);
        let data = crate::fixture("gp8/Morning Walk - bar repeats.gp");
        {
            let gp = crate::gpif::import_gp(&data).unwrap();
            assert!(gp.master[5].repeat_open && gp.master[5].repeat_close == 4);
        }
    }

    #[test]
    fn reads_legato_from_guitar_pro_file() {
        let data = crate::fixture("gp8/Morning Walk - legato.gp");
        let song = crate::gpif::import_gp(&data).unwrap();
        let bass = song.tracks.iter().position(|t| t.instrument == Instrument::Bass).unwrap();
        let bar = &song.tracks[bass].bars[0];
        assert!(bar.beats[0].notes.iter().all(|n| n.fx.legato));
        assert!(!bar.beats[1].notes.iter().any(|n| n.fx.legato));
    }

    #[test]
    fn score_xml_alone_reads_back() {
        // Read only the score part, as if another program had changed the file.
        let song = Song::demo();
        let back = crate::gpif::parse_gpif(&gpif_xml(&song)).expect("reads back");
        assert_eq!(back.title, song.title);
        assert_eq!(back.tempo, song.tempo);
        assert_eq!(back.tracks.len(), song.tracks.len());
        for (a, b) in song.tracks.iter().zip(back.tracks.iter()) {
            assert_eq!(a.tuning, b.tuning);
            for (x, y) in a.bars.iter().zip(b.bars.iter()) {
                assert_eq!(x.beats.len(), y.beats.len());
                for (p, q) in x.beats.iter().zip(y.beats.iter()) {
                    assert_eq!(p.dur, q.dur);
                    assert_eq!(pitches(a, p), pitches(b, q));
                }
            }
        }
        for (x, y) in song.master.iter().zip(back.master.iter()) {
            assert_eq!(
                (
                    x.num,
                    x.den,
                    x.key,
                    x.repeat_open,
                    x.repeat_close,
                    &x.marker
                ),
                (
                    y.num,
                    y.den,
                    y.key,
                    y.repeat_open,
                    y.repeat_close,
                    &y.marker
                )
            );
        }
    }

    #[test]
    fn whole_file_keeps_everything() {
        let mut song = Song::demo();
        song.tracks[0].volume = 0.37;
        song.tracks[1].pan = -0.5;
        song.tracks[1].solo = true;
        song.tracks[0].mute = true;
        song.tracks[0].bars[1].beats[0].fermata = true;
        song.tracks[0].bars[2].beats[0].notes[0].grace = Some(5);
        song.master[3].direction_jump = 7;
        let data = export_gp(&song).unwrap();
        let back = crate::gpif::import_gp(&data).unwrap();
        assert!((back.tracks[0].volume - 0.37).abs() < 0.001);
        assert!((back.tracks[1].pan + 0.5).abs() < 0.001);
        assert!(back.tracks[1].solo);
        assert!(back.tracks[0].mute);
        assert_eq!(
            back.tracks[0].bars[2].beats.len(),
            song.tracks[0].bars[2].beats.len()
        );
        let s0 = song.tracks[0].bars[2].beats[0].notes[0].string;
        assert_eq!(
            back.tracks[0].bars[2].beats[0]
                .note_on(s0)
                .and_then(|n| n.grace),
            Some(5)
        );
        assert!(back.tracks[0].bars[1].beats[0].fermata);
        assert_eq!(back.master[3].direction_jump, 7);
        // A hidden track stays hidden.
        let mut three = Song::demo();
        three.tracks.push(three.tracks[0].clone());
        three.normalize();
        three.tracks[1].visible = false;
        let back = crate::gpif::import_gp(&export_gp(&three).unwrap()).unwrap();
        assert_eq!(
            back.tracks.iter().map(|t| t.visible).collect::<Vec<_>>(),
            vec![true, false, true]
        );
    }

    #[test]
    fn piano_drums_and_voice_from_score_xml() {
        let mut song = Song::new_blank();
        let mut piano = Track::of_instrument(Instrument::Piano, "Piano");
        let mut drums = Track::of_instrument(Instrument::Drums, "Drums");
        let mut voc = Track::of_instrument(Instrument::LeadVocals, "Voice");
        voc.lyrics = "la la la".into();
        let chord = |ps: &[u8]| Beat {
            notes: ps
                .iter()
                .enumerate()
                .map(|(k, &p)| Note::new(k as u8, p))
                .collect(),
            ..Beat::rest(Dur::Whole)
        };
        piano.bars = (0..4)
            .map(|_| Bar::new(vec![chord(&[48, 60, 64, 67])]))
            .collect();
        drums.bars = (0..4).map(|_| Bar::new(vec![chord(&[36, 42])])).collect();
        voc.bars = (0..4).map(|_| Bar::new(vec![chord(&[72])])).collect();
        song.tracks = vec![voc, piano, drums];
        let back = crate::gpif::parse_gpif(&gpif_xml(&song)).expect("reads back");
        assert_eq!(back.tracks.len(), 3);
        assert_eq!(back.tracks[0].kind(), InstrumentType::Vocal);
        assert_eq!(back.tracks[0].lyrics, "la la la");
        assert!(back.tracks[1].grand());
        assert_eq!(
            pitches(&back.tracks[1], &back.tracks[1].bars[0].beats[0]),
            vec![48, 60, 64, 67]
        );
        assert_eq!(back.tracks[2].instrument, Instrument::Drums);
        assert_eq!(
            pitches(&back.tracks[2], &back.tracks[2].bars[0].beats[0]),
            vec![36, 42]
        );
    }

    #[test]
    fn every_drum_sound_has_its_own_articulation_and_comes_back() {
        let all: Vec<u8> = crate::drums::SOUNDS.iter().map(|d| d.midi).collect();
        let places: std::collections::BTreeSet<usize> = all.iter().map(|&p| drum_articulation(p)).collect();
        assert_eq!(places.len(), all.len(), "no two sounds share an articulation");
        assert_eq!(KIT.iter().map(|e| e.articulations.len()).sum::<usize>(), all.len());
        assert!(NOTATION_SOUNDS.iter().flat_map(|n| n.1).all(|m| crate::drums::STAFF_KIT.contains(m)));
        // Guitar Pro's order: the snare first, then the hi-hat.
        assert_eq!((drum_articulation(38), drum_articulation(42), drum_articulation(36)), (0, 3, 8));
        let mut song = Song::new_blank();
        let mut drums = Track::of_instrument(Instrument::Drums, "Drums");
        drums.bars = all.iter().map(|&p| Bar::new(vec![Beat { notes: vec![Note::new(0, p)], ..Beat::rest(Dur::Whole) }])).collect();
        song.master = vec![MasterBar::default(); all.len()];
        song.tracks = vec![drums];
        let back = crate::gpif::parse_gpif(&gpif_xml(&song)).expect("reads back");
        let read: Vec<u8> = back.tracks[0].bars.iter().map(|b| back.tracks[0].pitch(&b.beats[0].notes[0])).collect();
        assert_eq!(read, all);
    }

    #[test]
    fn a_drum_note_is_read_by_its_place_in_the_kit() {
        let mut song = Song::new_blank();
        let mut drums = Track::of_instrument(Instrument::Drums, "Drums");
        drums.bars = vec![Bar::new(vec![Beat { notes: vec![Note::new(0, 91)], ..Beat::rest(Dur::Whole) }])];
        song.master = vec![MasterBar::default()];
        song.tracks = vec![drums];
        // A file may give the sound it plays (a snare) as the note's MIDI number.
        let xml = gpif_xml(&song).replace("<Number>91</Number>", "<Number>38</Number>");
        let back = crate::gpif::parse_gpif(&xml).expect("reads back");
        assert_eq!(back.tracks[0].pitch(&back.tracks[0].bars[0].beats[0].notes[0]), 91);
    }

    #[test]
    fn bar_repeat_signs_are_kept_the_guitar_pro_way() {
        let mut song = Song::new_blank();
        let mut drums = Track::of_instrument(Instrument::Drums, "Drums");
        let hit = |p: u8| Bar::new(vec![Beat { notes: vec![Note::new(0, p)], ..Beat::rest(Dur::Whole) }]);
        let sign = |n: u8| {
            let mut b = Bar::new(vec![Beat::rest(Dur::Whole)]);
            b.simile = n;
            b
        };
        // Kick, snare, a two-bar repeat of both, then the last bar again.
        drums.bars = vec![hit(36), hit(38), sign(2), sign(2), sign(1)];
        song.master = vec![MasterBar::default(); 5];
        song.tracks = vec![drums];
        let xml = gpif_xml(&song);
        for mark in ["Simple", "FirstOfDouble", "SecondOfDouble"] {
            assert_eq!(xml.matches(&format!("<SimileMark>{mark}</SimileMark><Voices>-1 -1 -1 -1</Voices>")).count(), 1, "{mark}");
        }
        // The score part alone says it all, as in a file Guitar Pro saved.
        let back = crate::gpif::parse_gpif(&xml).expect("reads back");
        let t = &back.tracks[0];
        assert_eq!(t.bars.iter().map(|b| b.simile).collect::<Vec<_>>(), vec![0, 0, 2, 2, 1]);
        let played: Vec<u8> = (0..5).map(|i| t.pitch(&t.played_bar(i).beats[0].notes[0])).collect();
        assert_eq!(played, vec![36, 38, 36, 38, 38]);
    }

    #[test]
    fn a_drum_track_names_guitar_pros_drum_kit_sound() {
        let mut song = Song::new_blank();
        song.tracks = vec![Track::of_instrument(Instrument::Drums, "Drums"), Track::of_instrument(Instrument::Piano, "Piano")];
        song.normalize();
        let xml = gpif_xml(&song);
        let (drums, piano) = xml.split_once("</Track>").expect("two tracks");
        assert!(drums.contains("<Path>Drums/Drums/Drumkit</Path>"));
        assert!(drums.contains("<MSB>0</MSB><Program>0</Program>"));
        assert!(drums.contains("<SoundbankPatch>Drumkit-Master</SoundbankPatch>"));
        // The sound set at bar 1 is the same one.
        assert!(drums.contains("<![CDATA[Drums/Drums/Drumkit;Drum Kit;Factory]]>"));
        assert!(!piano.contains("Drumkit"));
        let back = crate::gpif::parse_gpif(&xml).expect("reads back");
        assert_eq!(back.tracks[0].instrument, Instrument::Drums);
        assert_eq!(back.tracks[1].instrument, Instrument::Piano);
    }

    #[test]
    fn reads_the_instruments_guitar_pro_saved() {
        use Instrument::*;
        let song = crate::gpif::import_gp(&crate::fixture("gp8/Instruments.gp")).unwrap();
        let names: Vec<Instrument> = song.tracks.iter().map(|t| t.instrument).collect();
        assert_eq!(names, vec![Piano, LeadVocals, Violin, Synth, ElectricPiano, Organ, Choir, Cello, Strings, Trumpet, Saxophone, Flute]);
        for i in [0, 3, 4, 5] {
            assert!(song.tracks[i].grand(), "{} has two staves", song.tracks[i].name);
        }
        // The piano's chord, at the pitches the file gives.
        let first = |t: &Track| pitches(t, t.bars[0].beats.iter().find(|b| !b.is_rest()).expect("notes"));
        assert!(first(&song.tracks[0]).contains(&72));
        assert_eq!(first(&song.tracks[7]), vec![59], "cello");
        // Saved and read again: the same instruments and notes.
        let back = crate::gpif::import_gp(&export_gp(&song).unwrap()).unwrap();
        for (a, b) in song.tracks.iter().zip(&back.tracks) {
            assert_eq!((a.instrument, a.grand_staff), (b.instrument, b.grand_staff), "{}", a.name);
            for (x, y) in a.bars.iter().zip(&b.bars) {
                let all = |t: &Track, bar: &Bar| bar.beats.iter().map(|beat| pitches(t, beat)).collect::<Vec<_>>();
                assert_eq!(all(a, x), all(b, y), "{}", a.name);
            }
        }
    }

    #[test]
    fn every_note_of_an_instrument_without_strings_sits_on_a_string() {
        let mut song = Song::new_blank();
        let mut piano = Track::of_instrument(Instrument::Piano, "Piano");
        // A wide chord on both staves, and notes at the ends of the keyboard.
        let chord = |ps: &[u8]| Beat { notes: ps.iter().enumerate().map(|(k, &p)| Note::new(k as u8, p)).collect(), ..Beat::rest(Dur::Whole) };
        piano.bars = vec![Bar::new(vec![chord(&[21, 48, 52, 60, 64, 67, 108])])];
        song.master = vec![MasterBar::default()];
        song.tracks = vec![piano];
        let xml = gpif_xml(&song);
        assert_eq!(xml.matches("<Property name=\"Tuning\">").count(), 2, "both staves");
        assert_eq!(xml.matches("<Property name=\"String\">").count(), 7, "every note");
        assert!(xml.contains("<Type>acousticPiano</Type>") && xml.contains("<Path>Orchestra/Keyboard/Acoustic Piano</Path>"));
        // Without the MIDI numbers, strings and frets say the same.
        let bare = without_midi(&xml);
        for text in [xml, bare] {
            let back = crate::gpif::parse_gpif(&text).expect("reads back");
            assert_eq!(pitches(&back.tracks[0], &back.tracks[0].bars[0].beats[0]), vec![21, 48, 52, 60, 64, 67, 108]);
        }
    }

    /// The XML with every note's MIDI property taken out.
    fn without_midi(xml: &str) -> String {
        let mut out = xml.to_string();
        while let Some(a) = out.find("<Property name=\"Midi\">") {
            let z = a + out[a..].find("</Property>").expect("closed") + "</Property>".len();
            out.replace_range(a..z, "");
        }
        out
    }

    /// A piano bar's voices as (length, pitches), for those with notes.
    fn hands(t: &Track) -> Vec<Vec<(Dur, Vec<u8>)>> {
        let bar = &t.bars[0];
        (0..VOICES)
            .filter(|&v| bar.voice_used(v))
            .map(|v| bar.voice(v).iter().map(|b| (b.dur, pitches(t, b))).collect())
            .collect()
    }

    #[test]
    fn a_left_hand_with_its_own_rhythm_survives_saving_and_opening() {
        let note = |d: Dur, p: u8| Beat { notes: vec![Note::new(0, p)], ..Beat::rest(d) };
        let four = || vec![note(Dur::Quarter, 72), note(Dur::Quarter, 74), note(Dur::Quarter, 76), note(Dur::Quarter, 77)];
        let halves = || vec![note(Dur::Half, 48), note(Dur::Half, 43)];
        let rest = || vec![Beat::rest(Dur::Whole)];
        // Both hands; the left alone; the right alone; a second line in the right hand too.
        let second = || vec![note(Dur::Half, 64), note(Dur::Half, 65)];
        for (first, others) in [
            (four(), vec![halves()]),
            (rest(), vec![halves()]),
            (four(), vec![]),
            (four(), vec![second(), vec![note(Dur::Whole, 36)]]),
        ] {
            let mut song = Song::new_blank();
            let mut piano = Track::of_instrument(Instrument::Piano, "Piano");
            let mut bar = Bar::new(first);
            bar.pad_voices();
            for (k, v) in others.into_iter().enumerate() {
                bar.voices[k] = v;
            }
            piano.bars = vec![bar];
            song.master = vec![MasterBar::default()];
            song.tracks = vec![piano];
            song.normalize();
            let typed = hands(&song.tracks[0]);
            let mut at = song;
            for pass in 1..=3 {
                at = crate::gpif::parse_gpif(&gpif_xml(&at)).expect("reads back");
                assert_eq!(hands(&at.tracks[0]), typed, "pass {pass}");
            }
        }
    }

    #[test]
    fn the_left_hand_is_written_as_the_lower_staffs_first_voice() {
        let note = |d: Dur, p: u8| Beat { notes: vec![Note::new(0, p)], ..Beat::rest(d) };
        let mut song = Song::new_blank();
        let mut piano = Track::of_instrument(Instrument::Piano, "Piano");
        let mut bar = Bar::new(vec![note(Dur::Whole, 72)]);
        bar.pad_voices();
        bar.voices[0] = vec![note(Dur::Half, 48), note(Dur::Half, 43)];
        piano.bars = vec![bar];
        song.master = vec![MasterBar::default()];
        song.tracks = vec![piano];
        song.normalize();
        let xml = gpif_xml(&song);
        // Each staff has one voice, in its first place: no empty second voice of rests.
        assert_eq!(xml.matches("-1 -1 -1</Voices>").count(), 2, "{xml}");
        // As Guitar Pro stores it, with the lower staff's voice first, it opens with both hands.
        let back = crate::gpif::parse_gpif(&xml).unwrap();
        assert_eq!(hands(&back.tracks[0]).len(), 2);
    }

    /// The demo song saved by Guitar Pro 8 opens as the demo song, and GuitarCraft's part
    /// settings are byte for byte the ones Guitar Pro 8 wrote.
    #[test]
    fn opens_the_demo_saved_by_guitar_pro() {
        let data = crate::fixture("gp8/Morning Walk Gp8.gp");
        let song = crate::gpif::import_gp(&data).expect("opens");
        let demo = Song::demo();
        assert_eq!(song.tracks.len(), demo.tracks.len());
        for (a, b) in song.tracks.iter().zip(&demo.tracks) {
            assert_eq!((&a.name, a.instrument, &a.tuning), (&b.name, b.instrument, &b.tuning));
            assert!(a.visible);
        }
        assert_eq!(song.bar_count(), demo.bar_count());
        let mut z = zip::ZipArchive::new(std::io::Cursor::new(&data[..])).unwrap();
        let mut theirs = vec![];
        std::io::Read::read_to_end(
            &mut z.by_name("Content/PartConfiguration").unwrap(),
            &mut theirs,
        )
        .unwrap();
        assert_eq!(part_configuration(&song), theirs);
    }
}
