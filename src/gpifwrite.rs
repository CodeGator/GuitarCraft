//! Writes songs as Guitar Pro files of the newer kind (.gp): a zip archive
//! holding the score as XML ("Content/score.gpif"), following the
//! community-documented layout that the reader in gpif.rs also uses.
//! Everything is stored the standard way, including each track's volume, pan,
//! mute and solo, so Guitar Pro and this app read the same file.

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
        Piano => ("acousticPiano", "Acoustic Piano"),
        ElectricPiano => ("electricPiano", "Electric Piano"),
        Organ => ("organ", "Organ"),
        Synth => ("leadSynthesizer", "Lead Synthesizer"),
        LeadVocals => ("leadVoice", "Lead Voice"),
        Choir => ("voiceChoir", "Choir"),
        Violin => ("violin", "Violin"),
        Cello => ("cello", "Cello"),
        Strings => ("stringEnsemble", "String Ensemble"),
        Trumpet => ("trumpet", "Trumpet"),
        Saxophone => ("altoSaxophone", "Alto Saxophone"),
        Flute => ("flute", "Flute"),
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

/// Drum kit elements written for drum tracks: (name, midi numbers).
const DRUMS: [(&str, &[u8]); 12] = [
    ("Kick Drum", &[35, 36]),
    ("Snare", &[37, 38, 39, 40]),
    ("Hi-Hat", &[42, 44, 46]),
    ("Low Floor Tom", &[41]),
    ("High Floor Tom", &[43]),
    ("Low Tom", &[45]),
    ("Mid Tom", &[47]),
    ("High Tom", &[48, 50]),
    ("Crash", &[49, 57, 55]),
    ("Ride", &[51, 53, 59]),
    ("China", &[52]),
    ("Percussion", &[54, 56]),
];

/// Index of a drum sound among all the drum kit's articulations, in writing order.
fn drum_articulation(p: u8) -> usize {
    let mut k = 0;
    for (_, ms) in DRUMS {
        for &m in ms {
            if m == p {
                return k;
            }
            k += 1;
        }
    }
    k - 1
}

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

thread_local! {
    static OPTS: std::cell::Cell<GpOptions> = const { std::cell::Cell::new(GpOptions { chords: false, views: false }) };
}

fn opts() -> GpOptions {
    OPTS.with(|o| o.get())
}

/// The score as Guitar Pro XML.
#[cfg(test)]
pub fn gpif_xml(song: &Song) -> String {
    gpif_xml_with(song, GpOptions::safe(song))
}

pub fn gpif_xml_with(song: &Song, o: GpOptions) -> String {
    OPTS.with(|c| c.set(o));
    let mut x = String::with_capacity(256 * 1024);
    x.push_str("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<GPIF>\n<GPVersion>7</GPVersion>\n<GPRevision required=\"12021\" recommended=\"12023\">12025</GPRevision>\n<Encoding><EncodingDescription>GP7</EncodingDescription></Encoding>\n");
    // Song information.
    let _ = writeln!(
        x,
        "<Score><Title>{}</Title><SubTitle>{}</SubTitle><Artist>{}</Artist><Album>{}</Album><Words>{}</Words><Music>{}</Music><WordsAndMusic>{}</WordsAndMusic><Copyright>{}</Copyright><Tabber>{}</Tabber><Instructions>{}</Instructions><Notices>{}</Notices><FirstPageHeader>{}</FirstPageHeader><FirstPageFooter>{}</FirstPageFooter><PageHeader>{}</PageHeader><PageFooter>{}</PageFooter><ScoreSystemsDefaultLayout>4</ScoreSystemsDefaultLayout><ScoreSystemsLayout>4</ScoreSystemsLayout><ScoreZoomPolicy>Value</ScoreZoomPolicy><ScoreZoom>1</ScoreZoom><MultiVoice>0</MultiVoice></Score>",
        cdata(&song.title),
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
        cdata(""),
        cdata(""),
        cdata(""),
        cdata(""),
    );
    // Tempo changes.
    let ids: Vec<String> = (0..song.tracks.len()).map(|k| k.to_string()).collect();
    let _ = write!(
        x,
        "<MasterTrack><Tracks>{}</Tracks><Automations>",
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
    x.push_str("</Automations></MasterTrack>\n<Tracks>\n");

    // Tracks.
    let mut chan = 0usize;
    for (k, t) in song.tracks.iter().enumerate() {
        let (ty, label) = inst_type(t.instrument);
        let drum = t.instrument == Instrument::Drums;
        let _ = write!(
            x,
            "<Track id=\"{k}\"><Name>{}</Name><ShortName>{}</ShortName><Color>{} {} {}</Color><SystemsDefautLayout>3</SystemsDefautLayout><SystemsLayout>1</SystemsLayout><PalmMute>{}</PalmMute><AutoAccentuation>{}</AutoAccentuation>{}{}<PlayingStyle>{}</PlayingStyle><IconId>{}</IconId>",
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
        let _ = write!(
            x,
            "<InstrumentSet><Name>{label}</Name><Type>{ty}</Type><LineCount>5</LineCount><Elements>"
        );
        if drum {
            for (name, ms) in DRUMS {
                let _ = write!(
                    x,
                    "<Element><Name>{name}</Name><Type>{}</Type><SoundbankName/><Articulations>",
                    name.to_lowercase().replace(' ', "")
                );
                for &m in ms {
                    let (line, cross) = drum_staff_pos(m);
                    let head = if cross {
                        "noteheadXBlack noteheadXHalf noteheadXWhole"
                    } else {
                        "noteheadBlack noteheadHalf noteheadWhole"
                    };
                    let _ = write!(
                        x,
                        "<Articulation><Name>{}</Name><StaffLine>{}</StaffLine><Noteheads>{head}</Noteheads><TechniquePlacement>outside</TechniquePlacement><TechniqueSymbol></TechniqueSymbol><InputMidiNumbers>{m}</InputMidiNumbers><OutputRSESound/><OutputMidiNumber>{m}</OutputMidiNumber></Articulation>",
                        drum_name(m),
                        8 - line
                    );
                }
                x.push_str("</Articulations></Element>");
            }
        } else {
            x.push_str("<Element><Name>Pitched</Name><Type>pitched</Type><SoundbankName/><Articulations><Articulation><Name/><StaffLine>0</StaffLine><Noteheads>noteheadBlack noteheadHalf noteheadWhole</Noteheads><TechniquePlacement>outside</TechniquePlacement><TechniqueSymbol></TechniqueSymbol><InputMidiNumbers/><OutputRSESound/><OutputMidiNumber>0</OutputMidiNumber></Articulation></Articulations></Element>");
        }
        x.push_str("</Elements></InstrumentSet>");
        let _ = write!(
            x,
            "<Transpose><Chromatic>0</Chromatic><Octave>{}</Octave></Transpose>",
            if t.clef().shift() != 0 { -1 } else { 0 }
        );
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
        let _ = write!(
            x,
            "<Sounds><Sound><Name>{label}</Name><Label>{label}</Label><Path>Midi/{label}</Path><Role>Factory</Role><MIDI><LSB>0</LSB><MSB>{}</MSB><Program>{prog}</Program></MIDI></Sound></Sounds><MidiConnection><Port>0</Port><PrimaryChannel>{primary}</PrimaryChannel><SecondaryChannel>{secondary}</SecondaryChannel><ForeOneChannelPerString>false</ForeOneChannelPerString></MidiConnection>",
            if drum { 128 } else { 0 }
        );
        x.push_str("<ForcedSound>-1</ForcedSound>");
        let state = if t.solo {
            "Solo"
        } else if t.mute {
            "Mute"
        } else {
            "Default"
        };
        let _ = write!(
            x,
            "<PlaybackState>{state}</PlaybackState><AudioEngineState>MIDI</AudioEngineState>"
        );
        let mut strip = [0.5f32; 16];
        strip[9] = 0.0;
        strip[11] = (t.pan.clamp(-1.0, 1.0) + 1.0) * 0.5;
        strip[12] = t.volume.clamp(0.0, 1.0);
        let params: Vec<String> = strip.iter().map(|v| format!("{v:.6}")).collect();
        let _ = write!(
            x,
            "<RSE><ChannelStrip version=\"E56\"><Parameters>{}</Parameters></ChannelStrip></RSE>",
            params.join(" ")
        );
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
        for _ in 0..staves {
            x.push_str("<Staff><Properties>");
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
            let mut names: Vec<&String> = if opts().chords {
                t.bars
                    .iter()
                    .flat_map(|b| b.beats.iter().filter_map(|x| x.chord.as_ref()))
                    .collect()
            } else {
                vec![]
            };
            names.sort();
            names.dedup();
            x.push_str("<Property name=\"ChordCollection\"><Items/></Property><Property name=\"ChordWorkingSet\"><Items/></Property><Property name=\"DiagramCollection\"><Items>");
            for (i, n) in names.iter().enumerate() {
                let _ = write!(
                    x,
                    "<Item id=\"{i}\" name=\"{}\"/>",
                    n.replace('&', "&amp;")
                        .replace('"', "&quot;")
                        .replace('<', "&lt;")
                );
            }
            x.push_str(
                "</Items></Property><Property name=\"DiagramWorkingSet\"><Items/></Property>",
            );
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
        let _ = writeln!(
            x,
            "</Staves><Automations><Automation><Type>Sound</Type><Linear>false</Linear><Bar>0</Bar><Position>0</Position><Visible>true</Visible><Value>{}</Value></Automation></Automations></Track>",
            cdata(&format!("Midi/{label};{label};Factory"))
        );
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
    };
    let mut master = String::from("<MasterBars>\n");
    for (bi, mb) in song.master.iter().enumerate() {
        let mut bar_ids = vec![];
        for t in &song.tracks {
            let key = mb.key;
            let chords: Vec<&String> = {
                let mut v: Vec<&String> = t
                    .bars
                    .iter()
                    .flat_map(|b| b.beats.iter().filter_map(|x| x.chord.as_ref()))
                    .collect();
                v.sort();
                v.dedup();
                v
            };
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
            for (low, clef) in staves {
                let bar = &t.bars[bi];
                let mut vids = vec![];
                for v in 0..VOICES {
                    if v > 0 && !bar.voice_used(v) {
                        vids.push("-1".to_string());
                        continue;
                    }
                    let mut beat_ids = vec![];
                    for beat in bar.voice(v) {
                        // On a grand staff each staff keeps the notes on its side of middle C.
                        let notes: Vec<&Note> = beat
                            .notes
                            .iter()
                            .filter(|n| match low {
                                None => true,
                                Some(l) => (t.pitch(n) < 60) == l,
                            })
                            .collect();
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
                                &mut ids, t, &gb, &refs, &chords, key, false, true,
                            ));
                        }
                        beat_ids.push(write_beat(
                            &mut ids,
                            t,
                            beat,
                            &notes,
                            &chords,
                            key,
                            v == 0 && low != Some(true),
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
                let id = ids.nbars;
                ids.nbars += 1;
                let _ = writeln!(
                    ids.bars,
                    "<Bar id=\"{id}\"><Clef>{clef}</Clef><Voices>{}</Voices></Bar>",
                    vids.join(" ")
                );
                bar_ids.push(id.to_string());
            }
        }
        // The bar's settings shared by all tracks.
        let _ = write!(
            master,
            "<MasterBar><Key><AccidentalCount>{}</AccidentalCount><Mode>{}</Mode><TransposeAs>{}</TransposeAs></Key><Time>{}/{}</Time>",
            mb.key,
            if mb.key_minor { "Minor" } else { "Major" },
            if mb.key < 0 { "Flats" } else { "Sharps" },
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
    if opts().views {
        x.push_str("<ScoreViews>");
        for k in 0..=song.tracks.len() {
            let _ = write!(x, "<ScoreView id=\"{k}\"/>");
        }
        x.push_str("</ScoreViews>\n");
    }
    x.push_str("</GPIF>\n");
    x
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

fn write_beat(
    ids: &mut Ids,
    t: &Track,
    beat: &Beat,
    notes: &[&Note],
    chords: &[&String],
    key: i8,
    marks: bool,
) -> usize {
    write_beat_ex(ids, t, beat, notes, chords, key, marks, false)
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
            && opts().chords
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
    if !notes.is_empty() {
        let mut nids = vec![];
        for n in notes {
            nids.push(write_note(ids, t, beat, n, key).to_string());
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
    x.push_str("</Beat>\n");
    ids.beats.push_str(&x);
    id
}

fn write_note(ids: &mut Ids, t: &Track, beat: &Beat, n: &Note, key: i8) -> usize {
    let id = ids.nnotes;
    ids.nnotes += 1;
    let pitch = t.pitch(n) as i32;
    let mut x = format!("<Note id=\"{id}\">");
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
            drum_articulation(pitch as u8)
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
    let written = pitch + t.clef().shift();
    let (step, accd, oct) = pitch_parts(pitch);
    let (wstep, waccd, woct) = pitch_parts(written);
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
    if t.has_strings() {
        let nstr = t.strings();
        let _ = write!(
            x,
            "<Property name=\"String\"><String>{}</String></Property><Property name=\"Fret\"><Fret>{}</Fret></Property>",
            nstr - 1 - (n.string as usize).min(nstr - 1),
            n.fret
        );
    }
    if n.dead {
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
    if n.slide || n.slide_in || n.slide_out {
        let flags = (if n.slide { 2 } else { 0 })
            | (if n.slide_out { 4 } else { 0 })
            | (if n.slide_in { 16 } else { 0 });
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
fn part_configuration(song: &Song) -> Vec<u8> {
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
    for f in &flags {
        b.push(0);
        b.extend_from_slice(&1i32.to_be_bytes());
        b.push(*f);
    }
    b.extend_from_slice(&0i32.to_be_bytes());
    b
}

/// Where the stylesheet is kept inside a .gp file.
pub const STYLE_FILE: &str = "Content/GuitarCraftStylesheet.json";

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
        z.start_file("Content/PartConfiguration", opt).map_err(e)?;
        z.write_all(&part_configuration(song))
            .map_err(|e| e.to_string())?;
        // This app's stylesheet, kept in its own file next to the score (other
        // programs skip files they don't know). Left out when it's the standard one.
        if song.style != crate::style::Stylesheet::standard() {
            z.start_file(STYLE_FILE, opt).map_err(e)?;
            let json = serde_json::to_string_pretty(&song.style).map_err(|e| e.to_string())?;
            z.write_all(json.as_bytes()).map_err(|e| e.to_string())?;
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
        if !o.views {
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
    fn opens_sample_saved_by_guitar_pro() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("samples")
            .join("Empty.gp");
        let Ok(data) = std::fs::read(&p) else { return };
        let song = crate::gpif::import_gp(&data).expect("opens the sample");
        assert_eq!(song.tracks.len(), 2);
        assert_eq!(song.tracks[0].instrument, Instrument::Bass);
        assert_eq!(song.tracks[0].tuning, STANDARD_BASS.to_vec());
        assert!((song.tracks[0].volume - 0.795).abs() < 0.001);
        assert_eq!(song.tracks[0].short_name, "el.bs.");
        assert!((song.tracks[0].palm_mute - 0.3).abs() < 0.001);
        assert!(!song.tracks[1].auto_let_ring);
        // The bass track is hidden in the sample (its eye is off); the other is shown.
        assert!(!song.tracks[0].visible);
        assert!(song.tracks[1].visible);
        assert_eq!(song.bar_count(), 10);
        // Our own part settings have the same layout as the sample's.
        let ours = part_configuration(&song);
        let mut z = zip::ZipArchive::new(std::io::Cursor::new(&data[..])).unwrap();
        let mut theirs = vec![];
        std::io::Read::read_to_end(
            &mut z.by_name("Content/PartConfiguration").unwrap(),
            &mut theirs,
        )
        .unwrap();
        assert_eq!(ours.len(), theirs.len());
    }
}
