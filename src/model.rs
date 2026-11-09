//! Song data: tracks, bars, beats and notes.

use serde::{Deserialize, Serialize};

/// Ticks in a quarter note.
pub const QUARTER: u32 = 960;

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum Dur {
    Whole,
    Half,
    Quarter,
    Eighth,
    Sixteenth,
    ThirtySecond,
    SixtyFourth,
}

impl Dur {
    pub const ALL: [Dur; 7] = [
        Dur::Whole,
        Dur::Half,
        Dur::Quarter,
        Dur::Eighth,
        Dur::Sixteenth,
        Dur::ThirtySecond,
        Dur::SixtyFourth,
    ];
    pub fn index(self) -> usize {
        self as usize
    }
    pub fn from_index(i: usize) -> Dur {
        Dur::ALL[i.min(6)]
    }
    pub fn ticks(self) -> u32 {
        (QUARTER * 4) >> self.index()
    }
    pub fn longer(self) -> Dur {
        Dur::from_index(self.index().saturating_sub(1))
    }
    pub fn shorter(self) -> Dur {
        Dur::from_index(self.index() + 1)
    }
    pub fn name(self) -> &'static str {
        match self {
            Dur::Whole => "Whole note",
            Dur::Half => "Half note",
            Dur::Quarter => "Quarter note",
            Dur::Eighth => "Eighth note",
            Dur::Sixteenth => "16th note",
            Dur::ThirtySecond => "32nd note",
            Dur::SixtyFourth => "64th note",
        }
    }
    /// Number of flags/beams (0 for quarter and longer).
    pub fn flags(self) -> usize {
        self.index().saturating_sub(2)
    }
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Debug, Default)]
pub enum Dynamic {
    Ppp,
    Pp,
    P,
    Mp,
    #[default]
    Mf,
    F,
    Ff,
    Fff,
}

impl Dynamic {
    pub const ALL: [Dynamic; 8] = [
        Dynamic::Ppp,
        Dynamic::Pp,
        Dynamic::P,
        Dynamic::Mp,
        Dynamic::Mf,
        Dynamic::F,
        Dynamic::Ff,
        Dynamic::Fff,
    ];
    pub fn label(self) -> &'static str {
        match self {
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
    pub fn velocity(self) -> u8 {
        match self {
            Dynamic::Ppp => 20,
            Dynamic::Pp => 35,
            Dynamic::P => 50,
            Dynamic::Mp => 65,
            Dynamic::Mf => 80,
            Dynamic::F => 95,
            Dynamic::Ff => 110,
            Dynamic::Fff => 125,
        }
    }
    #[allow(dead_code)]
    pub fn from_velocity(v: u8) -> Dynamic {
        let mut best = Dynamic::Mf;
        let mut d = 255i32;
        for x in Dynamic::ALL {
            let dd = (x.velocity() as i32 - v as i32).abs();
            if dd < d {
                d = dd;
                best = x;
            }
        }
        best
    }
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Debug, Default)]
pub enum Stroke {
    #[default]
    None,
    Down,
    Up,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default, PartialEq)]
#[serde(default)]
pub struct Note {
    /// 0 = highest-pitched string.
    pub string: u8,
    pub fret: u8,
    pub tie: bool,
    pub hammer: bool,
    pub slide: bool,
    /// Bend amount in quarter tones (4 = full).
    pub bend: u8,
    pub vibrato: bool,
    pub palm_mute: bool,
    pub let_ring: bool,
    pub dead: bool,
    pub ghost: bool,
    pub harmonic: bool,
    pub accent: bool,
    pub staccato: bool,
    pub heavy_accent: bool,
    /// Artificial (pinch) harmonic.
    pub pinch_harmonic: bool,
    /// 0 bend, 1 bend and release, 2 pre-bend, 3 pre-bend and release.
    pub bend_kind: u8,
    pub slide_in: bool,
    pub slide_out: bool,
    pub wide_vibrato: bool,
    /// Trill with this fret (0 = no trill).
    pub trill: u8,
    /// Spelling in notation: 0 automatic, 1 sharp, 2 flat.
    pub spell: u8,
    /// A grace note played just before, on this fret.
    pub grace: Option<u8>,
}

impl Note {
    pub fn new(string: u8, fret: u8) -> Note {
        Note {
            string,
            fret,
            ..Default::default()
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
#[serde(default)]
pub struct Beat {
    pub dur: Dur,
    pub dotted: bool,
    /// 0 = none, otherwise e.g. 3 for triplets.
    pub tuplet: u8,
    pub notes: Vec<Note>,
    pub text: Option<String>,
    pub chord: Option<String>,
    pub stroke: Stroke,
    pub dynamic: Dynamic,
    pub double_dot: bool,
    pub fermata: bool,
    /// 0 none, 1 crescendo, 2 decrescendo.
    pub hairpin: u8,
    pub lyric: Option<String>,
    /// Show a chord diagram (made from this beat's notes) above the staff.
    pub chord_diagram: bool,
    /// Pick stroke: 0 none, 1 down, 2 up.
    pub pick: u8,
    pub tremolo: bool,
    /// 0 none, 1 tap, 2 slap, 3 pop.
    pub technique: u8,
    pub fade_in: bool,
    /// Written an octave (or two) higher/lower: -2, -1, 0, 1, 2.
    pub ottava: i8,
}

impl Default for Beat {
    fn default() -> Self {
        Beat::rest(Dur::Quarter)
    }
}

impl Beat {
    pub fn rest(dur: Dur) -> Beat {
        Beat {
            dur,
            dotted: false,
            tuplet: 0,
            notes: vec![],
            text: None,
            chord: None,
            stroke: Stroke::None,
            dynamic: Dynamic::Mf,
            double_dot: false,
            fermata: false,
            hairpin: 0,
            lyric: None,
            chord_diagram: false,
            pick: 0,
            tremolo: false,
            technique: 0,
            fade_in: false,
            ottava: 0,
        }
    }
    pub fn is_rest(&self) -> bool {
        self.notes.is_empty()
    }
    pub fn ticks(&self) -> u32 {
        let mut t = self.dur.ticks();
        if self.double_dot {
            t = t * 7 / 4;
        } else if self.dotted {
            t = t * 3 / 2;
        }
        match self.tuplet {
            3 => t * 2 / 3,
            5 => t * 4 / 5,
            6 => t * 4 / 6,
            7 => t * 4 / 7,
            9 => t * 8 / 9,
            _ => t,
        }
    }
    pub fn note_on(&self, string: u8) -> Option<&Note> {
        self.notes.iter().find(|n| n.string == string)
    }
    pub fn note_on_mut(&mut self, string: u8) -> Option<&mut Note> {
        self.notes.iter_mut().find(|n| n.string == string)
    }
}

/// Number of voices a bar can hold.
pub const VOICES: usize = 4;

static NO_BEATS: Vec<Beat> = Vec::new();

fn voices_unused(v: &[Vec<Beat>]) -> bool {
    v.iter().all(|x| x.iter().all(|b| b.is_rest()))
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct Bar {
    /// Voice 1.
    pub beats: Vec<Beat>,
    /// Voices 2, 3 and 4 (a single whole rest when unused).
    #[serde(default, skip_serializing_if = "voices_unused")]
    pub voices: Vec<Vec<Beat>>,
}

impl Bar {
    pub fn new(beats: Vec<Beat>) -> Bar {
        Bar { beats, voices: vec![] }
    }
    pub fn empty() -> Bar {
        Bar::new(vec![Beat::rest(Dur::Whole)])
    }
    pub fn ticks(&self) -> u32 {
        self.beats.iter().map(|b| b.ticks()).sum()
    }
    /// True when no voice has any notes.
    pub fn is_empty(&self) -> bool {
        (0..VOICES).all(|v| self.voice(v).iter().all(|b| b.is_rest()))
    }
    /// The beats of a voice (0 = voice 1).
    pub fn voice(&self, v: usize) -> &Vec<Beat> {
        if v == 0 {
            &self.beats
        } else {
            self.voices.get(v - 1).unwrap_or(&NO_BEATS)
        }
    }
    pub fn voice_ticks(&self, v: usize) -> u32 {
        self.voice(v).iter().map(|b| b.ticks()).sum()
    }
    /// A voice is used when it has notes.
    pub fn voice_used(&self, v: usize) -> bool {
        self.voice(v).iter().any(|b| !b.is_rest())
    }
    /// A voice that is filled but doesn't add up to the bar's length.
    pub fn wrong_length(&self, need: u32) -> bool {
        (0..VOICES).any(|v| {
            let b = self.voice(v);
            let placeholder = b.len() == 1 && b[0].is_rest();
            !placeholder && !b.is_empty() && self.voice_ticks(v) != need && (v == 0 || self.voice_used(v))
        })
    }
    /// Voices whose length doesn't match the bar: (voice, ticks too many (+) or too few (-)).
    pub fn length_errors(&self, need: u32) -> Vec<(usize, i64)> {
        (0..VOICES)
            .filter(|&v| {
                let b = self.voice(v);
                let placeholder = b.len() == 1 && b[0].is_rest();
                !placeholder && !b.is_empty() && self.voice_ticks(v) != need && (v == 0 || self.voice_used(v))
            })
            .map(|v| (v, self.voice_ticks(v) as i64 - need as i64))
            .collect()
    }
    pub fn pad_voices(&mut self) {
        while self.voices.len() < VOICES - 1 {
            self.voices.push(vec![]);
        }
        for v in &mut self.voices {
            if v.is_empty() {
                v.push(Beat::rest(Dur::Whole));
            }
        }
    }
    /// The beats of a voice, for changing (voices are padded first).
    pub fn voice_mut(&mut self, v: usize) -> &mut Vec<Beat> {
        if v == 0 {
            &mut self.beats
        } else {
            self.pad_voices();
            &mut self.voices[v - 1]
        }
    }

    /// Tick where beat `j` of voice `v` starts.
    pub fn beat_start(&self, v: usize, j: usize) -> u32 {
        self.voice(v).iter().take(j).map(|b| b.ticks()).sum()
    }

    /// Puts notes into voice `v` at tick `t`, fitting them into that voice's rhythm:
    /// into a beat starting there, or by splitting a rest. `src` gives the note's
    /// rhythm and beat settings. Returns false if a note is already playing there.
    pub fn put_notes(&mut self, v: usize, t: u32, src: &Beat, notes: &[Note], bar_len: u32) -> bool {
        let list = self.voice_mut(v);
        // An unused voice is one long rest filling the bar.
        if list.iter().all(|b| b.is_rest()) {
            *list = rests_for(bar_len);
        }
        let mut start = 0u32;
        let mut k = 0usize;
        while k < list.len() && start + list[k].ticks() <= t {
            start += list[k].ticks();
            k += 1;
        }
        if k == list.len() {
            // Past the end: pad with rests, then the note.
            if t > start {
                list.extend(rests_for(t - start));
            }
            let mut b = src.clone();
            b.notes = notes.to_vec();
            list.push(b);
            return true;
        }
        if start == t && !list[k].is_rest() {
            // Join an existing beat if the strings are free.
            if notes.iter().any(|n| list[k].note_on(n.string).is_some()) {
                return false;
            }
            list[k].notes.extend(notes.iter().cloned());
            list[k].notes.sort_by_key(|n| n.string);
            return true;
        }
        if !list[k].is_rest() {
            return false;
        }
        // Split the rest: [start, t) rest, the note, then the remaining rest.
        let rest_len = list[k].ticks();
        let space = start + rest_len - t;
        let mut new_beats = rests_for(t - start);
        let mut b = src.clone();
        b.notes = notes.to_vec();
        let used = if b.ticks() <= space {
            b.ticks()
        } else {
            let (d, dot) = split_ticks(space).first().copied().unwrap_or((Dur::SixtyFourth, false));
            b.dur = d;
            b.dotted = dot;
            b.double_dot = false;
            b.tuplet = 0;
            b.ticks()
        };
        new_beats.push(b);
        new_beats.extend(rests_for(space - used));
        list.splice(k..k + 1, new_beats);
        true
    }

    /// Every beat of every voice.
    pub fn all_beats_mut(&mut self) -> impl Iterator<Item = &mut Beat> {
        self.beats.iter_mut().chain(self.voices.iter_mut().flatten())
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
#[serde(default)]
pub struct MasterBar {
    pub num: u8,
    pub den: u8,
    pub repeat_open: bool,
    /// Number of times to play when closing a repeat (0 = no repeat sign).
    pub repeat_close: u8,
    pub marker: Option<String>,
    pub tempo: Option<u32>,
    /// The tempo changes gradually from this bar's tempo point to the next one.
    pub tempo_ramp: bool,
    /// Key signature: number of sharps (positive) or flats (negative).
    pub key: i8,
    /// The key is minor (only changes its name).
    pub key_minor: bool,
    pub triplet_feel: bool,
    pub double_bar: bool,
    /// Alternate endings as bits: 1 = first time, 2 = second time, ...
    pub endings: u8,
    /// Start a new line of music after this bar.
    pub line_break: bool,
    /// Direction target at the start of this bar (see DIR_* constants).
    pub direction_target: u8,
    /// Direction jump at the end of this bar (see JUMP_NAMES).
    pub direction_jump: u8,
}

impl Default for MasterBar {
    fn default() -> Self {
        MasterBar {
            num: 4,
            den: 4,
            repeat_open: false,
            repeat_close: 0,
            marker: None,
            tempo: None,
            tempo_ramp: false,
            key: 0,
            key_minor: false,
            triplet_feel: false,
            double_bar: false,
            endings: 0,
            line_break: false,
            direction_target: 0,
            direction_jump: 0,
        }
    }
}

impl MasterBar {
    pub fn ticks(&self) -> u32 {
        self.num as u32 * (QUARTER * 4 / self.den.max(1) as u32)
    }
}

/// The family of a track's instrument. It decides which staves the track can
/// show: only stringed instruments have tablature, keyboards can use a grand
/// staff, and drums use the percussion staff.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InstrumentType {
    Stringed,
    Keyboard,
    Vocal,
    Orchestra,
    Drums,
}

impl InstrumentType {
    pub const ALL: [InstrumentType; 5] = [InstrumentType::Stringed, InstrumentType::Keyboard, InstrumentType::Vocal, InstrumentType::Orchestra, InstrumentType::Drums];
    pub fn name(self) -> &'static str {
        match self {
            InstrumentType::Stringed => "Stringed",
            InstrumentType::Keyboard => "Keyboards",
            InstrumentType::Vocal => "Vocals",
            InstrumentType::Orchestra => "Orchestra",
            InstrumentType::Drums => "Drums",
        }
    }
    pub fn instruments(self) -> Vec<Instrument> {
        Instrument::ALL.iter().copied().filter(|i| i.kind() == self).collect()
    }
}

/// The clef of a staff. Guitars and basses are written an octave above the sound.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Clef {
    Treble8,
    Bass8,
    Treble,
    Bass,
    Percussion,
}

impl Clef {
    /// Semitones added to the sounding pitch to get the written one.
    pub fn shift(self) -> i32 {
        match self {
            Clef::Treble8 | Clef::Bass8 => 12,
            _ => 0,
        }
    }
    /// Diatonic step number of the bottom line (E4 for treble, G2 for bass).
    pub fn bottom(self) -> i32 {
        match self {
            Clef::Bass | Clef::Bass8 => 2 * 7 + 4,
            _ => 4 * 7 + 2,
        }
    }
    pub fn is_bass(self) -> bool {
        matches!(self, Clef::Bass | Clef::Bass8)
    }
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum Instrument {
    SteelGuitar,
    NylonGuitar,
    CleanGuitar,
    OverdriveGuitar,
    DistortionGuitar,
    Bass,
    Ukulele,
    Banjo,
    Piano,
    ElectricPiano,
    Organ,
    Synth,
    LeadVocals,
    Choir,
    Violin,
    Cello,
    Strings,
    Trumpet,
    Saxophone,
    Flute,
    Drums,
}

impl Instrument {
    pub const ALL: [Instrument; 21] = [
        Instrument::SteelGuitar,
        Instrument::NylonGuitar,
        Instrument::CleanGuitar,
        Instrument::OverdriveGuitar,
        Instrument::DistortionGuitar,
        Instrument::Bass,
        Instrument::Ukulele,
        Instrument::Banjo,
        Instrument::Piano,
        Instrument::ElectricPiano,
        Instrument::Organ,
        Instrument::Synth,
        Instrument::LeadVocals,
        Instrument::Choir,
        Instrument::Violin,
        Instrument::Cello,
        Instrument::Strings,
        Instrument::Trumpet,
        Instrument::Saxophone,
        Instrument::Flute,
        Instrument::Drums,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Instrument::SteelGuitar => "Acoustic Steel Guitar",
            Instrument::NylonGuitar => "Classical Nylon Guitar",
            Instrument::CleanGuitar => "Electric Guitar (clean)",
            Instrument::OverdriveGuitar => "Electric Guitar (overdrive)",
            Instrument::DistortionGuitar => "Electric Guitar (distortion)",
            Instrument::Bass => "Electric Bass",
            Instrument::Ukulele => "Ukulele",
            Instrument::Banjo => "Banjo",
            Instrument::Piano => "Grand Piano",
            Instrument::ElectricPiano => "Electric Piano",
            Instrument::Organ => "Organ",
            Instrument::Synth => "Synth Lead",
            Instrument::LeadVocals => "Lead Vocals",
            Instrument::Choir => "Choir",
            Instrument::Violin => "Violin",
            Instrument::Cello => "Cello",
            Instrument::Strings => "String Ensemble",
            Instrument::Trumpet => "Trumpet",
            Instrument::Saxophone => "Saxophone",
            Instrument::Flute => "Flute",
            Instrument::Drums => "Drum Kit",
        }
    }
    pub fn kind(self) -> InstrumentType {
        use Instrument::*;
        match self {
            SteelGuitar | NylonGuitar | CleanGuitar | OverdriveGuitar | DistortionGuitar | Bass | Ukulele | Banjo => InstrumentType::Stringed,
            Piano | ElectricPiano | Organ | Synth => InstrumentType::Keyboard,
            LeadVocals | Choir => InstrumentType::Vocal,
            Violin | Cello | Strings | Trumpet | Saxophone | Flute => InstrumentType::Orchestra,
            Drums => InstrumentType::Drums,
        }
    }
    pub fn midi_program(self) -> u8 {
        use Instrument::*;
        match self {
            SteelGuitar => 25,
            NylonGuitar => 24,
            CleanGuitar => 27,
            OverdriveGuitar => 29,
            DistortionGuitar => 30,
            Bass => 33,
            Ukulele => 24,
            Banjo => 105,
            Piano => 0,
            ElectricPiano => 4,
            Organ => 16,
            Synth => 80,
            LeadVocals => 53,
            Choir => 52,
            Violin => 40,
            Cello => 42,
            Strings => 48,
            Trumpet => 56,
            Saxophone => 65,
            Flute => 73,
            Drums => 0,
        }
    }
    pub fn from_midi_program(p: u8) -> Instrument {
        use Instrument::*;
        match p {
            0..=3 | 6..=15 => Piano,
            4 | 5 => ElectricPiano,
            16..=23 => Organ,
            24 => NylonGuitar,
            25 => SteelGuitar,
            26..=28 => CleanGuitar,
            29 => OverdriveGuitar,
            30 | 31 => DistortionGuitar,
            32..=39 => Bass,
            40 | 41 => Violin,
            42 | 43 => Cello,
            44..=51 => Strings,
            52 => Choir,
            53 | 54 | 85 => LeadVocals,
            56..=63 => Trumpet,
            64..=71 => Saxophone,
            72..=79 => Flute,
            80..=84 | 86..=103 => Synth,
            105 | 106 => Banjo,
            _ => SteelGuitar,
        }
    }
    pub fn is_bass(self) -> bool {
        matches!(self, Instrument::Bass)
    }
    pub fn has_strings_kind(self) -> bool {
        self.kind() == InstrumentType::Stringed
    }
    /// Clef of the (first) notation staff.
    pub fn clef(self) -> Clef {
        use Instrument::*;
        match self {
            Bass => Clef::Bass8,
            SteelGuitar | NylonGuitar | CleanGuitar | OverdriveGuitar | DistortionGuitar | Banjo => Clef::Treble8,
            Cello => Clef::Bass,
            Drums => Clef::Percussion,
            _ => Clef::Treble,
        }
    }
    /// Instruments without strings keep their notes on "virtual strings" that all
    /// start at pitch 0, so the fret number is the pitch. This many notes can sound together.
    pub fn virtual_strings(self) -> usize {
        match self.kind() {
            InstrumentType::Stringed => 0,
            InstrumentType::Keyboard => 10,
            InstrumentType::Vocal => if self == Instrument::Choir { 4 } else { 2 },
            InstrumentType::Orchestra => if self == Instrument::Strings { 6 } else { 2 },
            InstrumentType::Drums => 8,
        }
    }
    /// Default tuning for a new track of this instrument.
    pub fn default_tuning(self) -> Vec<u8> {
        match self {
            Instrument::Bass => tuning_presets(4)[0].1.clone(),
            Instrument::Ukulele => tuning_presets(4)[2].1.clone(),
            Instrument::Banjo => vec![62, 59, 55, 50, 67],
            _ if self.kind() != InstrumentType::Stringed => vec![0; self.virtual_strings()],
            _ => tuning_presets(6)[0].1.clone(),
        }
    }
    /// Highest "fret" for a new track (for instruments without strings, the highest pitch).
    pub fn default_frets(self) -> u8 {
        if self.kind() == InstrumentType::Stringed { 24 } else { 127 }
    }
}

/// Playing styles: the name shown and the name stored in .gp files.
pub const PLAYING_STYLES: [(&str, &str); 5] = [("Default", "Default"), ("Pick", "StringedPick"), ("Fingers", "StringedFinger"), ("Picking", "StringedPicking"), ("Bass Slap", "BassSlap")];

/// Name of a General MIDI drum sound, from the manual's drum map.
pub fn drum_name(p: u8) -> &'static str {
    match p {
        35 | 36 => "Bass Drum",
        37 => "Side Stick",
        38 | 40 => "Snare",
        39 => "Hand Clap",
        41 | 43 => "Floor Tom",
        42 => "Hi-hat (closed)",
        44 => "Hi-hat (pedal)",
        45 | 47 => "Mid Tom",
        46 => "Hi-hat (open)",
        48 | 50 => "High Tom",
        49 | 57 => "Crash",
        51 | 59 => "Ride",
        52 => "China",
        53 => "Ride Bell",
        54 => "Tambourine",
        55 => "Splash",
        56 => "Cowbell",
        _ => "Percussion",
    }
}

/// Where a drum sound sits on the percussion staff (0 = bottom line) and whether it has a cross head.
pub fn drum_staff_pos(p: u8) -> (i32, bool) {
    match p {
        35 | 36 => (1, false),
        37 => (5, true),
        38 | 40 => (5, false),
        39 => (5, true),
        41 => (2, false),
        43 => (3, false),
        42 | 46 => (9, true),
        44 => (-1, true),
        45 => (4, false),
        47 => (6, false),
        48 => (7, false),
        50 => (8, false),
        49 | 57 => (10, true),
        51 | 59 => (8, true),
        52 => (11, true),
        53 => (8, true),
        54 => (9, true),
        55 => (11, true),
        56 => (10, false),
        _ => (4, true),
    }
}

/// The drum sound for a line or space of the percussion staff.
pub fn drum_at_pos(pos: i32) -> u8 {
    match pos {
        ..=-1 => 44,
        0 | 1 => 36,
        2 => 41,
        3 => 43,
        4 => 45,
        5 => 38,
        6 => 47,
        7 => 48,
        8 => 51,
        9 => 42,
        10 => 49,
        _ => 52,
    }
}

/// Common tunings: (name, notes from highest string to lowest).
pub fn tuning_presets(strings: usize) -> Vec<(&'static str, Vec<u8>)> {
    match strings {
        4 => vec![
            ("Bass standard (E A D G)", vec![43, 38, 33, 28]),
            ("Bass drop D", vec![43, 38, 33, 26]),
            ("Ukulele (G C E A)", vec![69, 64, 60, 67]),
        ],
        5 => vec![("5-string bass (B E A D G)", vec![43, 38, 33, 28, 23])],
        7 => vec![("7-string standard", vec![64, 59, 55, 50, 45, 40, 35])],
        _ => vec![
            ("Standard (E A D G B E)", vec![64, 59, 55, 50, 45, 40]),
            ("Drop D", vec![64, 59, 55, 50, 45, 38]),
            ("Half step down", vec![63, 58, 54, 49, 44, 39]),
            ("Open G", vec![62, 59, 55, 50, 43, 38]),
            ("Open D", vec![62, 57, 54, 50, 45, 38]),
            ("DADGAD", vec![62, 57, 55, 50, 45, 38]),
        ],
    }
}

pub fn tuning_name(t: &[u8]) -> String {
    for (name, notes) in tuning_presets(t.len()) {
        if notes == t {
            return name.to_string();
        }
    }
    t.iter()
        .rev()
        .map(|&n| note_name(n))
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn note_name(midi: u8) -> String {
    const N: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
    N[(midi % 12) as usize].to_string()
}

pub fn note_name_oct(midi: u8) -> String {
    format!("{}{}", note_name(midi), midi as i32 / 12 - 1)
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
#[serde(default)]
pub struct Track {
    pub name: String,
    pub instrument: Instrument,
    pub tuning: Vec<u8>,
    pub capo: u8,
    pub frets: u8,
    pub color: [u8; 3],
    pub volume: f32,
    pub pan: f32,
    pub mute: bool,
    pub solo: bool,
    /// Shown in the score.
    pub visible: bool,
    /// Short name shown beside the staves (empty = made from the name).
    pub short_name: String,
    /// Let every note ring on.
    pub auto_let_ring: bool,
    /// Strum chords slightly, string after string, without a brush sign.
    pub auto_brush: bool,
    /// How the strings are played: see PLAYING_STYLES.
    pub playing_style: u8,
    /// The track's icon (0 = the instrument's usual one).
    pub icon: u8,
    /// Amount of palm muting on every note, 0..1.
    pub palm_mute: f32,
    /// Extra accent on the first beat of each bar, 0..1.
    pub accentuation: f32,
    /// Lyrics text; syllables are spread over the beats with notes.
    pub lyrics: String,
    /// Bar (0-based) where the lyrics start.
    pub lyrics_bar: usize,
    pub bars: Vec<Bar>,
    /// Show this track's standard notation staff.
    pub show_std: bool,
    /// Show this track's tablature (stringed instruments only).
    pub show_tab: bool,
    /// Two staves joined by a brace (keyboards only).
    pub grand_staff: bool,
    /// A capo over only some strings: its fret (0 = none) and the strings it
    /// covers (bit 0 = string 1, the highest).
    pub partial_capo: u8,
    pub partial_capo_strings: u8,
    /// Tuning notes are named with flats (otherwise sharps).
    pub tuning_flat: bool,
    /// The tuning's name on the score (empty: the usual name).
    pub tuning_label: String,
    /// The tuning is named on the score.
    pub tuning_label_visible: bool,
    /// While editing another voice, that voice is swapped into `beats` (not saved).
    #[serde(skip)]
    pub swapped: usize,
}

impl Default for Track {
    fn default() -> Self {
        Track {
            name: "Guitar".into(),
            instrument: Instrument::SteelGuitar,
            tuning: vec![64, 59, 55, 50, 45, 40],
            capo: 0,
            frets: 24,
            color: TRACK_COLORS[0],
            volume: 0.8,
            pan: 0.0,
            mute: false,
            solo: false,
            visible: true,
            short_name: String::new(),
            auto_let_ring: false,
            palm_mute: 0.0,
            accentuation: 0.0,
            lyrics: String::new(),
            lyrics_bar: 0,
            bars: vec![],
            show_std: true,
            show_tab: true,
            grand_staff: false,
            partial_capo: 0,
            partial_capo_strings: 0,
            auto_brush: false,
            playing_style: 0,
            icon: 0,
            tuning_flat: true,
            tuning_label: String::new(),
            tuning_label_visible: true,
            swapped: 0,
        }
    }
}

impl Track {
    pub fn pitch(&self, n: &Note) -> u8 {
        let base = self.tuning.get(n.string as usize).copied().unwrap_or(40);
        (base as u32 + n.fret as u32 + self.capo_at(n.string as usize) as u32).min(127) as u8
    }
    /// The capo on a string: the full capo, or the partial capo where it is higher and covers the string.
    pub fn capo_at(&self, s: usize) -> u8 {
        if self.partial_capo > self.capo && s < 8 && self.partial_capo_strings & (1 << s) != 0 {
            self.partial_capo
        } else {
            self.capo
        }
    }
    pub fn strings(&self) -> usize {
        self.tuning.len()
    }
    pub fn kind(&self) -> InstrumentType {
        self.instrument.kind()
    }
    /// Only stringed instruments have strings (and so tablature).
    pub fn has_strings(&self) -> bool {
        self.kind() == InstrumentType::Stringed
    }
    /// The tablature is shown for this track (when the view shows tablature).
    pub fn has_tab(&self) -> bool {
        self.has_strings() && self.show_tab
    }
    /// A grand staff: keyboards only, never vocals or drums.
    pub fn grand(&self) -> bool {
        self.grand_staff && self.kind() == InstrumentType::Keyboard
    }
    pub fn clef(&self) -> Clef {
        self.instrument.clef()
    }
    /// A new track of an instrument, with its usual tuning and staves.
    pub fn of_instrument(i: Instrument, name: &str) -> Track {
        Track {
            name: name.to_string(),
            instrument: i,
            tuning: i.default_tuning(),
            frets: i.default_frets(),
            grand_staff: matches!(i, Instrument::Piano | Instrument::ElectricPiano | Instrument::Organ),
            ..Track::default()
        }
    }
    /// Changes the instrument. Moving between stringed and other instruments
    /// changes the strings, keeping every note's pitch.
    pub fn set_instrument(&mut self, i: Instrument) {
        let was = self.has_strings();
        let old_tuning = self.tuning.clone();
        let old_capo = self.capo;
        let old_kind = self.kind();
        self.instrument = i;
        let now = self.has_strings();
        if was != now || (!now && self.tuning.len() != i.virtual_strings()) {
            self.tuning = i.default_tuning();
            self.frets = i.default_frets();
            self.capo = 0;
            refit_notes(self, &old_tuning, old_capo);
        }
        if i.kind() != old_kind {
            self.grand_staff = matches!(i, Instrument::Piano | Instrument::ElectricPiano | Instrument::Organ);
        }
    }
}

/// Puts every note back on a string that can play its pitch after the tuning changed.
pub fn refit_notes(t: &mut Track, old_tuning: &[u8], old_capo: u8) {
    let maxf = t.frets as i32;
    let n = t.tuning.len();
    let capos: Vec<u8> = (0..n).map(|s| t.capo_at(s)).collect();
    for bar in &mut t.bars {
        for beat in bar.all_beats_mut() {
            let mut out: Vec<Note> = vec![];
            for note in beat.notes.drain(..) {
                let Some(&open) = old_tuning.get(note.string as usize) else { continue };
                let pitch = open as i32 + note.fret as i32 + old_capo as i32;
                let mut order: Vec<usize> = (0..n).collect();
                order.sort_by_key(|&s| (s as i32 - note.string as i32).abs());
                for s in order {
                    let f = pitch - t.tuning[s] as i32 - capos[s] as i32;
                    if (0..=maxf).contains(&f) && !out.iter().any(|o| o.string as usize == s) {
                        let mut m = note.clone();
                        m.string = s as u8;
                        m.fret = f as u8;
                        out.push(m);
                        break;
                    }
                }
            }
            out.sort_by_key(|x| x.string);
            beat.notes = out;
        }
    }
}

pub const TRACK_COLORS: [[u8; 3]; 8] = [
    [236, 110, 110],
    [232, 196, 92],
    [200, 214, 112],
    [240, 186, 112],
    [84, 192, 222],
    [170, 140, 226],
    [120, 206, 150],
    [222, 130, 190],
];

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
#[serde(default)]
pub struct Song {
    pub title: String,
    pub subtitle: String,
    pub artist: String,
    pub album: String,
    pub music_by: String,
    pub tabbed_by: String,
    pub words: String,
    pub copyright: String,
    pub notice: String,
    pub instructions: String,
    pub tempo: u32,
    pub master: Vec<MasterBar>,
    pub tracks: Vec<Track>,
}

impl Default for Song {
    fn default() -> Self {
        Song::new_blank()
    }
}

impl Song {
    pub fn new_blank() -> Song {
        let bars = 4;
        let mut t = Track::default();
        t.name = "Steel Guitar".into();
        t.bars = (0..bars).map(|_| Bar::empty()).collect();
        Song {
            title: "Untitled".into(),
            subtitle: String::new(),
            artist: String::new(),
            album: String::new(),
            music_by: String::new(),
            tabbed_by: String::new(),
            words: String::new(),
            copyright: String::new(),
            notice: String::new(),
            instructions: String::new(),
            tempo: 120,
            master: (0..bars).map(|_| MasterBar::default()).collect(),
            tracks: vec![t],
        }
    }

    pub fn bar_count(&self) -> usize {
        self.master.len()
    }

    /// Makes sure every track has as many bars as the master list.
    pub fn normalize(&mut self) {
        if self.master.is_empty() {
            self.master.push(MasterBar::default());
        }
        if self.tracks.is_empty() {
            self.tracks.push(Track::default());
        }
        let n = self.master.len();
        for t in &mut self.tracks {
            if t.tuning.is_empty() {
                t.tuning = vec![64, 59, 55, 50, 45, 40];
            }
            t.bars.resize_with(n, Bar::empty);
            for b in &mut t.bars {
                if b.beats.is_empty() {
                    b.beats.push(Beat::rest(Dur::Whole));
                }
                b.pad_voices();
                let s = t.tuning.len() as u8;
                for beat in b.all_beats_mut() {
                    beat.notes.retain(|nn| nn.string < s);
                }
            }
        }
    }

    /// Puts voice `v` of a track where voice 1 normally is, so edits apply to it.
    pub fn swap_in(&mut self, ti: usize, v: usize) {
        if v == 0 || ti >= self.tracks.len() || self.tracks[ti].swapped != 0 {
            return;
        }
        let t = &mut self.tracks[ti];
        for b in &mut t.bars {
            b.pad_voices();
            std::mem::swap(&mut b.beats, &mut b.voices[v - 1]);
        }
        t.swapped = v;
    }

    /// Puts every swapped voice back in its place.
    pub fn swap_out_all(&mut self) {
        for t in &mut self.tracks {
            let v = t.swapped;
            if v == 0 {
                continue;
            }
            for b in &mut t.bars {
                b.pad_voices();
                std::mem::swap(&mut b.beats, &mut b.voices[v - 1]);
            }
            t.swapped = 0;
        }
    }

    /// Tempo points: (bar, bpm, gradual change to the next point). Bar 0 is always there.
    pub fn tempo_points(&self) -> Vec<(usize, u32, bool)> {
        let mut v = vec![(0, self.master.first().and_then(|m| m.tempo).unwrap_or(self.tempo), self.master.first().map(|m| m.tempo_ramp).unwrap_or(false))];
        for (i, m) in self.master.iter().enumerate().skip(1) {
            if let Some(t) = m.tempo {
                v.push((i, t, m.tempo_ramp));
            }
        }
        v
    }

    /// Tempo of a bar, following gradual changes between tempo points.
    pub fn tempo_at(&self, bar: usize) -> u32 {
        let pts = self.tempo_points();
        let k = pts.iter().rposition(|p| p.0 <= bar).unwrap_or(0);
        let (b0, t0, ramp) = pts[k];
        let t = match pts.get(k + 1) {
            Some(&(b1, t1, _)) if ramp && b1 > b0 => {
                let f = (bar - b0) as f32 / (b1 - b0) as f32;
                (t0 as f32 + (t1 as f32 - t0 as f32) * f).round() as u32
            }
            _ => t0,
        };
        t.max(10)
    }

    /// The bar order when playing, following repeat signs.
    pub fn play_order(&self) -> Vec<usize> {
        let n = self.master.len();
        let mut out = vec![];
        let mut i = 0;
        let mut repeat_start = 0;
        let mut counts: std::collections::HashMap<usize, u8> = Default::default();
        let mut guard = 0;
        let mut pass: u8 = 1;
        // Musical directions.
        let mut jumped = false;
        let mut al_fine = false;
        let mut al_coda = 0u8; // 0 none, 3 coda, 4 double coda
        let mut done_jumps: std::collections::HashSet<usize> = Default::default();
        let has_al_coda = self.master.iter().any(|m| matches!(m.direction_jump, 3 | 4 | 7));
        let find_target = |t: u8| self.master.iter().position(|m| m.direction_target == t);
        while i < n && guard < 20000 {
            guard += 1;
            let m = &self.master[i];
            if m.repeat_open && i != repeat_start {
                repeat_start = i;
                pass = 1;
            } else if m.repeat_open {
                repeat_start = i;
            }
            // Skip alternate endings that aren't for this time through.
            if m.endings != 0 {
                let skip = if jumped {
                    // After a jump, repeats aren't taken again: play only the last ending.
                    let mut group_max = 0u8;
                    let mut k = i;
                    while k < n && self.master[k].endings != 0 {
                        group_max |= self.master[k].endings;
                        k += 1;
                    }
                    let top = |e: u8| 7 - e.leading_zeros().min(7) as u8;
                    top(m.endings) < top(group_max)
                } else {
                    pass <= 8 && m.endings & (1 << (pass - 1)) == 0
                };
                if skip {
                    i += 1;
                    continue;
                }
            }
            out.push(i);
            if m.repeat_close > 0 && !jumped {
                let c = counts.entry(i).or_insert(0);
                *c += 1;
                if *c < m.repeat_close.max(2) {
                    i = repeat_start;
                    pass = c.saturating_add(1);
                    continue;
                } else {
                    counts.remove(&i);
                    pass = 1;
                }
            }
            // Fine ends the piece once a jump has been made.
            if m.direction_target == DIR_FINE && jumped && (al_fine || !self.master.iter().any(|x| matches!(x.direction_jump, 2 | 6))) {
                break;
            }
            // To Coda / To Double Coda.
            if matches!(m.direction_jump, 9 | 10) && jumped {
                let want = if m.direction_jump == 9 { DIR_CODA } else { DIR_DOUBLE_CODA };
                if !has_al_coda || al_coda == want {
                    if let Some(t) = find_target(want) {
                        if t > i {
                            i = t;
                            continue;
                        }
                    }
                }
            }
            // D.C. / D.S. jumps, each taken once.
            if (1..=8).contains(&m.direction_jump) && !done_jumps.contains(&i) {
                done_jumps.insert(i);
                let dest = match m.direction_jump {
                    1..=4 => Some(0),
                    8 => find_target(DIR_SEGNO_SEGNO),
                    _ => find_target(DIR_SEGNO),
                };
                if let Some(d) = dest {
                    jumped = true;
                    al_fine = matches!(m.direction_jump, 2 | 6);
                    al_coda = match m.direction_jump {
                        3 | 7 => DIR_CODA,
                        4 => DIR_DOUBLE_CODA,
                        _ => 0,
                    };
                    i = d;
                    repeat_start = d;
                    continue;
                }
            }
            i += 1;
        }
        out
    }

    /// Lyrics of a track split into syllables and placed on the beats that have notes.
    pub fn lyric_map(&self, ti: usize) -> std::collections::HashMap<(usize, usize), String> {
        let mut map = std::collections::HashMap::new();
        let t = &self.tracks[ti];
        if t.lyrics.trim().is_empty() {
            return map;
        }
        let syl = split_lyrics(&t.lyrics);
        let mut k = 0;
        'outer: for bi in t.lyrics_bar.min(t.bars.len())..t.bars.len() {
            for (bj, b) in t.bars[bi].beats.iter().enumerate() {
                if b.is_rest() || b.notes.iter().all(|n| n.tie) {
                    continue;
                }
                if k >= syl.len() {
                    break 'outer;
                }
                if !syl[k].is_empty() {
                    map.insert((bi, bj), syl[k].clone());
                }
                k += 1;
            }
        }
        map
    }

    /// A short original demo piece so the app opens with something to look at and hear.
    pub fn demo() -> Song {
        let mut song = Song::new_blank();
        song.title = "Morning Walk".into();
        song.subtitle = "Demo piece".into();
        song.artist = "GuitarCraft".into();
        song.music_by = "GuitarCraft demo".into();
        song.tempo = 96;
        let nbars = 12;
        song.master = (0..nbars).map(|_| MasterBar::default()).collect();
        song.master[0].marker = Some("Intro".into());
        song.master[0].repeat_open = true;
        song.master[3].repeat_close = 2;
        song.master[4].marker = Some("Verse".into());
        song.master[8].marker = Some("Outro".into());
        for m in song.master.iter_mut() {
            m.key = 1;
        }
        song.master[7].double_bar = true;

        // Lead guitar: arpeggios over G - D - Em - C, then a melody with techniques.
        let mut g = Track::default();
        g.name = "Steel Guitar".into();
        g.color = TRACK_COLORS[0];
        let n = |s: u8, f: u8| Note::new(s, f);
        let e8 = |notes: Vec<Note>| Beat {
            dur: Dur::Eighth,
            notes,
            ..Beat::rest(Dur::Eighth)
        };
        let q = |notes: Vec<Note>| Beat {
            dur: Dur::Quarter,
            notes,
            ..Beat::rest(Dur::Quarter)
        };
        let arp = |shape: [(u8, u8); 4]| -> Bar {
            let mut beats = vec![];
            let order = [0usize, 1, 2, 3, 2, 1, 2, 3];
            for (i, &o) in order.iter().enumerate() {
                let (s, f) = shape[o];
                let mut nn = n(s, f);
                if i == 0 {
                    nn.let_ring = true;
                }
                beats.push(e8(vec![nn]));
            }
            Bar::new(beats)
        };
        let mut bars = vec![
            arp([(5, 3), (3, 0), (2, 0), (1, 3)]),
            arp([(4, 0), (2, 2), (1, 3), (0, 2)]),
            arp([(5, 0), (3, 2), (2, 0), (1, 0)]),
            arp([(4, 3), (3, 2), (2, 0), (1, 1)]),
        ];
        // Verse melody
        let mut b5 = vec![
            q(vec![n(5, 3), n(1, 3)]),
            e8(vec![n(1, 0)]),
            e8(vec![n(2, 2)]),
            q(vec![{
                let mut x = n(2, 2);
                x.hammer = true;
                x
            }]),
            q(vec![n(2, 4)]),
        ];
        b5[0].chord = Some("G".into());
        b5[0].dynamic = Dynamic::Mf;
        for b in b5.iter_mut().take(3) {
            b.hairpin = 1;
        }
        let mut b6 = vec![
            e8(vec![n(4, 0), n(1, 3)]),
            e8(vec![n(1, 3)]),
            e8(vec![n(0, 2)]),
            e8(vec![n(0, 3)]),
            Beat {
                dur: Dur::Half,
                notes: vec![{
                    let mut x = n(0, 2);
                    x.vibrato = true;
                    x
                }],
                ..Beat::rest(Dur::Half)
            },
        ];
        b6[0].chord = Some("D".into());
        let mut b7 = vec![
            q(vec![n(5, 0), {
                let mut x = n(1, 5);
                x.slide = true;
                x
            }]),
            q(vec![n(1, 7)]),
            e8(vec![n(1, 5)]),
            e8(vec![n(1, 3)]),
            q(vec![{
                let mut x = n(2, 4);
                x.bend = 4;
                x
            }]),
        ];
        b7[0].chord = Some("Em".into());
        let mut b8 = vec![
            Beat {
                dur: Dur::Quarter,
                dotted: true,
                notes: vec![n(4, 3), n(2, 0), n(1, 1)],
                stroke: Stroke::Down,
                ..Beat::rest(Dur::Quarter)
            },
            e8(vec![n(1, 1)]),
            q(vec![n(1, 0)]),
            Beat::rest(Dur::Quarter),
        ];
        b8[0].chord = Some("C".into());
        bars.push(Bar::new(b5));
        bars.push(Bar::new(b6));
        bars.push(Bar::new(b7));
        bars.push(Bar::new(b8));
        // Outro: palm-muted rhythm and final chord.
        let pm = |s: u8, f: u8| {
            let mut x = n(s, f);
            x.palm_mute = true;
            x
        };
        for (root_s, root_f) in [(5u8, 3u8), (5, 0), (4, 3)] {
            let mut beats = vec![];
            for i in 0..8 {
                let mut bt = e8(vec![pm(root_s, root_f), pm(root_s - 1, root_f + 2)]);
                if i == 4 {
                    bt.notes[0].accent = true;
                }
                beats.push(bt);
            }
            bars.push(Bar::new(beats));
        }
        let mut last = Beat {
            dur: Dur::Whole,
            notes: vec![n(5, 3), n(4, 2), n(3, 0), n(2, 0), n(1, 0), n(0, 3)],
            stroke: Stroke::Down,
            ..Beat::rest(Dur::Whole)
        };
        for x in &mut last.notes {
            x.let_ring = true;
        }
        last.chord = Some("G".into());
        last.text = Some("rit.".into());
        last.fermata = true;
        bars.push(Bar::new(vec![last]));
        g.bars = bars;

        // Bass track.
        let mut b = Track::default();
        b.name = "Electric Bass".into();
        b.instrument = Instrument::Bass;
        b.tuning = vec![43, 38, 33, 28];
        b.color = TRACK_COLORS[1];
        b.volume = 0.75;
        let roots: [(u8, u8); 12] = [
            (3, 3),
            (2, 5),
            (3, 0),
            (2, 3),
            (3, 3),
            (2, 5),
            (3, 0),
            (2, 3),
            (3, 3),
            (3, 0),
            (2, 3),
            (3, 3),
        ];
        b.bars = roots
            .iter()
            .enumerate()
            .map(|(i, &(s, f))| {
                if i == 11 {
                    Bar {
                        voices: vec![],
                        beats: vec![Beat {
                            dur: Dur::Whole,
                            notes: vec![n(s, f)],
                            ..Beat::rest(Dur::Whole)
                        }],
                    }
                } else if i < 4 {
                    Bar {
                        voices: vec![],
                        beats: vec![
                            Beat {
                                dur: Dur::Half,
                                notes: vec![n(s, f)],
                                ..Beat::rest(Dur::Half)
                            },
                            Beat {
                                dur: Dur::Half,
                                notes: vec![n(s, f)],
                                ..Beat::rest(Dur::Half)
                            },
                        ],
                    }
                } else {
                    Bar {
                        voices: vec![],
                        beats: vec![
                            q(vec![n(s, f)]),
                            e8(vec![n(s, f)]),
                            e8(vec![n(s, f)]),
                            q(vec![n(s, f)]),
                            q(vec![n(s - 1, f + 2)]),
                        ],
                    }
                }
            })
            .collect();
        song.tracks = vec![g, b];
        song.normalize();
        song
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alternate_endings_play_order() {
        let mut s = Song::new_blank();
        s.master = (0..4).map(|_| MasterBar::default()).collect();
        s.master[0].repeat_open = true;
        s.master[1].endings = 1;
        s.master[1].repeat_close = 2;
        s.master[2].endings = 2;
        s.normalize();
        assert_eq!(s.play_order(), vec![0, 1, 0, 2, 3]);
    }

    #[test]
    fn double_dot_length() {
        let mut b = Beat::rest(Dur::Quarter);
        b.double_dot = true;
        assert_eq!(b.ticks(), QUARTER * 7 / 4);
    }
}

pub const DIR_SEGNO: u8 = 1;
pub const DIR_SEGNO_SEGNO: u8 = 2;
pub const DIR_CODA: u8 = 3;
pub const DIR_DOUBLE_CODA: u8 = 4;
pub const DIR_FINE: u8 = 5;

/// Names of direction targets, by code.
pub const TARGET_NAMES: [&str; 6] = ["None", "Segno", "Segno Segno", "Coda", "Double Coda", "Fine"];

/// Names of direction jumps, by code.
pub const JUMP_NAMES: [&str; 11] = [
    "None",
    "Da Capo",
    "Da Capo al Fine",
    "Da Capo al Coda",
    "Da Capo al Double Coda",
    "Dal Segno",
    "Dal Segno al Fine",
    "Dal Segno al Coda",
    "Dal Segno Segno",
    "Da Coda",
    "Da Double Coda",
];

/// Slot of each direction in a Guitar Pro 5 file's list of 19: targets
/// (Coda, Double Coda, Segno, Segno Segno, Fine) then jumps.
pub fn gp5_direction_slot(target: u8, jump: u8) -> Option<usize> {
    if target != 0 {
        return match target {
            DIR_CODA => Some(0),
            DIR_DOUBLE_CODA => Some(1),
            DIR_SEGNO => Some(2),
            DIR_SEGNO_SEGNO => Some(3),
            DIR_FINE => Some(4),
            _ => None,
        };
    }
    match jump {
        1 => Some(5),
        3 => Some(6),
        4 => Some(7),
        2 => Some(8),
        5 => Some(9),
        7 => Some(10),
        6 => Some(12),
        8 => Some(13),
        9 => Some(17),
        10 => Some(18),
        _ => None,
    }
}

/// The direction (target, jump) for a Guitar Pro 5 slot.
pub fn gp5_direction_of_slot(slot: usize) -> (u8, u8) {
    match slot {
        0 => (DIR_CODA, 0),
        1 => (DIR_DOUBLE_CODA, 0),
        2 => (DIR_SEGNO, 0),
        3 => (DIR_SEGNO_SEGNO, 0),
        4 => (DIR_FINE, 0),
        5 => (0, 1),
        6 => (0, 3),
        7 => (0, 4),
        8 => (0, 2),
        9 => (0, 5),
        10 | 11 => (0, 7),
        12 => (0, 6),
        13..=16 => (0, 8),
        17 => (0, 9),
        18 => (0, 10),
        _ => (0, 0),
    }
}

/// Short form shown on the score.
pub fn jump_label(code: u8) -> &'static str {
    match code {
        1 => "D.C.",
        2 => "D.C. al Fine",
        3 => "D.C. al Coda",
        4 => "D.C. al Double Coda",
        5 => "D.S.",
        6 => "D.S. al Fine",
        7 => "D.S. al Coda",
        8 => "D.S.S.",
        9 => "To Coda",
        10 => "To Double Coda",
        _ => "",
    }
}

/// Splits lyrics into syllables: a space or hyphen ends a syllable, extra
/// spaces or hyphens leave a beat empty, "+" joins words, [text] is ignored.
pub fn split_lyrics(text: &str) -> Vec<String> {
    let mut clean = String::new();
    let mut depth = 0;
    for ch in text.chars() {
        match ch {
            '[' => depth += 1,
            ']' if depth > 0 => depth -= 1,
            _ if depth > 0 => {}
            '\r' => {}
            _ => clean.push(ch),
        }
    }
    // Line breaks count as one space, however many there are.
    let mut flat = String::new();
    let mut prev_nl = false;
    for ch in clean.chars() {
        if ch == '\n' {
            if !prev_nl {
                flat.push(' ');
            }
            prev_nl = true;
        } else {
            flat.push(ch);
            prev_nl = false;
        }
    }
    let mut out = vec![];
    let mut cur = String::new();
    let mut after_sep = true;
    for ch in flat.trim().chars() {
        if ch == ' ' || ch == '-' {
            if !cur.is_empty() {
                if ch == '-' {
                    cur.push('-');
                }
                out.push(std::mem::take(&mut cur).replace('+', " "));
                after_sep = true;
            } else if after_sep && !out.is_empty() {
                out.push(String::new());
            }
        } else {
            cur.push(ch);
            after_sep = false;
        }
    }
    if !cur.is_empty() {
        out.push(cur.replace('+', " "));
    }
    out
}

#[cfg(test)]
mod direction_tests {
    use super::*;

    fn song(n: usize) -> Song {
        let mut s = Song::new_blank();
        s.master = (0..n).map(|_| MasterBar::default()).collect();
        s.normalize();
        s
    }

    #[test]
    fn da_capo_al_fine() {
        let mut s = song(4);
        s.master[1].direction_target = DIR_FINE;
        s.master[3].direction_jump = 2;
        assert_eq!(s.play_order(), vec![0, 1, 2, 3, 0, 1]);
    }

    #[test]
    fn dal_segno_al_coda() {
        let mut s = song(5);
        s.master[1].direction_target = DIR_SEGNO;
        s.master[2].direction_jump = 9;
        s.master[3].direction_jump = 7;
        s.master[4].direction_target = DIR_CODA;
        assert_eq!(s.play_order(), vec![0, 1, 2, 3, 1, 2, 4]);
    }

    #[test]
    fn lyrics_split() {
        assert_eq!(split_lyrics("Hel-lo my+friend  la"), vec!["Hel-", "lo", "my friend", "", "la"]);
        assert_eq!(split_lyrics("[Verse] one\n\ntwo"), vec!["one", "two"]);
    }
}

#[cfg(test)]
mod length_words_tests {
    use super::*;

    #[test]
    fn says_how_much_to_add_or_remove() {
        assert_eq!(length_advice(0, -(QUARTER as i64), 4), "Voice 1 is too short: add 1 beat (a quarter note)");
        assert_eq!(length_advice(0, QUARTER as i64 / 2, 4), "Voice 1 is too long: remove \u{BD} beat (an eighth note)");
        assert_eq!(length_advice(1, -(QUARTER as i64 * 3 / 2), 4), "Voice 2 is too short: add 1\u{BD} beats (a dotted quarter note)");
        assert_eq!(beats_words(QUARTER * 2, 4), "2 beats");
        assert_eq!(beats_words(QUARTER / 2, 8), "1 beat");
    }
}

#[cfg(test)]
mod voice_tests {
    use super::*;

    #[test]
    fn swap_in_and_out() {
        let mut s = Song::new_blank();
        s.normalize();
        s.swap_in(0, 1);
        s.tracks[0].bars[0].beats = vec![Beat { notes: vec![Note::new(0, 5)], ..Beat::rest(Dur::Whole) }];
        s.swap_out_all();
        assert!(s.tracks[0].bars[0].voice_used(1));
        assert!(!s.tracks[0].bars[0].voice_used(0));
        assert_eq!(s.tracks[0].bars[0].voice(1)[0].notes[0].fret, 5);
        assert_eq!(s.tracks[0].swapped, 0);
        let j = serde_json::to_string(&s).unwrap();
        let back: Song = serde_json::from_str(&j).unwrap();
        assert!(back.tracks[0].bars[0].voice_used(1));
    }
}

/// Splits a length in ticks into standard (possibly dotted) note values, longest first.
pub fn split_ticks(mut t: u32) -> Vec<(Dur, bool)> {
    let mut cand: Vec<(u32, Dur, bool)> = vec![];
    for d in Dur::ALL {
        cand.push((d.ticks(), d, false));
        if d != Dur::SixtyFourth {
            cand.push((d.ticks() * 3 / 2, d, true));
        }
    }
    cand.sort_by(|a, b| b.0.cmp(&a.0));
    let mut out = vec![];
    while t >= Dur::SixtyFourth.ticks() {
        match cand.iter().find(|c| c.0 <= t) {
            Some(&(len, d, dot)) => {
                out.push((d, dot));
                t -= len;
            }
            None => break,
        }
    }
    out
}

/// A length in beats of the bar's time signature, in words: "1 beat", "½ beat", "2½ beats".
pub fn beats_words(ticks: u32, den: u8) -> String {
    let unit = QUARTER * 4 / den.max(1) as u32;
    let whole = ticks / unit;
    let rest = ticks % unit;
    let frac = match (rest * 4 + unit / 2) / unit {
        0 if rest == 0 => "",
        1 => "\u{BC}",
        2 => "\u{BD}",
        3 => "\u{BE}",
        _ => "",
    };
    if rest != 0 && frac.is_empty() || (rest != 0 && (rest * 4) % unit != 0) {
        let b = ticks as f32 / unit as f32;
        return format!("{b:.2} beats");
    }
    let n = if whole == 0 && !frac.is_empty() { frac.to_string() } else { format!("{whole}{frac}") };
    let plural = !(whole == 1 && frac.is_empty()) && !(whole == 0 && !frac.is_empty());
    format!("{n} beat{}", if plural { "s" } else { "" })
}

/// A length as note values, e.g. "a quarter note" or "a half note + an eighth note".
pub fn note_values_words(ticks: u32) -> String {
    let parts = split_ticks(ticks);
    let covered: u32 = parts.iter().map(|(d, dot)| if *dot { d.ticks() * 3 / 2 } else { d.ticks() }).sum();
    if parts.is_empty() || covered != ticks || parts.len() > 3 {
        return String::new();
    }
    parts
        .iter()
        .map(|(d, dot)| {
            let name = d.name().to_lowercase();
            if *dot {
                format!("a dotted {name}")
            } else if name.starts_with('e') {
                format!("an {name}")
            } else {
                format!("a {name}")
            }
        })
        .collect::<Vec<_>>()
        .join(" + ")
}

/// What to do about a voice that is too short or too long, in words.
pub fn length_advice(voice: usize, diff: i64, den: u8) -> String {
    let t = diff.unsigned_abs() as u32;
    let values = note_values_words(t);
    let values = if values.is_empty() { String::new() } else { format!(" ({values})") };
    if diff < 0 {
        format!("Voice {} is too short: add {}{values}", voice + 1, beats_words(t, den))
    } else {
        format!("Voice {} is too long: remove {}{values}", voice + 1, beats_words(t, den))
    }
}

/// Rests filling a length in ticks.
pub fn rests_for(t: u32) -> Vec<Beat> {
    split_ticks(t)
        .into_iter()
        .map(|(d, dot)| {
            let mut r = Beat::rest(d);
            r.dotted = dot;
            r
        })
        .collect()
}

#[cfg(test)]
mod move_voice_tests {
    use super::*;

    fn q(f: u8, s: u8) -> Beat {
        Beat { notes: vec![Note::new(s, f)], ..Beat::rest(Dur::Quarter) }
    }

    #[test]
    fn into_empty_voice() {
        let mut b = Bar::new(vec![q(0, 0), q(2, 0), q(3, 0), q(5, 0)]);
        b.pad_voices();
        let src = b.beats[2].clone();
        assert!(b.put_notes(1, 1920, &src, &[Note::new(5, 3)], 3840));
        // Rest (half) then the quarter note then a quarter rest.
        let v = b.voice(1);
        assert_eq!(v.iter().map(|x| x.ticks()).sum::<u32>(), 3840);
        assert_eq!(b.beat_start(1, v.iter().position(|x| !x.is_rest()).unwrap()), 1920);
        assert!(b.voice_used(1));
    }

    #[test]
    fn join_existing_beat_and_block() {
        let mut b = Bar::new(vec![q(0, 0), q(2, 0), q(3, 0), q(5, 0)]);
        b.pad_voices();
        let src = b.beats[0].clone();
        assert!(b.put_notes(0, 0, &src, &[Note::new(4, 2)], 3840));
        assert_eq!(b.beats[0].notes.len(), 2);
        // Same string already used.
        assert!(!b.put_notes(0, 0, &src, &[Note::new(4, 5)], 3840));
        // Inside a note: not possible.
        let mut e = Bar::new(vec![Beat { notes: vec![Note::new(0, 1)], ..Beat::rest(Dur::Whole) }]);
        assert!(!e.put_notes(0, 960, &src, &[Note::new(2, 2)], 3840));
    }
}

#[cfg(test)]
mod tempo_tests {
    use super::*;

    #[test]
    fn gradual_tempo() {
        let mut s = Song::new_blank();
        s.master = (0..5).map(|_| MasterBar::default()).collect();
        s.tempo = 100;
        s.master[0].tempo_ramp = true;
        s.master[4].tempo = Some(140);
        assert_eq!(s.tempo_at(0), 100);
        assert_eq!(s.tempo_at(2), 120);
        assert_eq!(s.tempo_at(4), 140);
        s.master[0].tempo_ramp = false;
        assert_eq!(s.tempo_at(3), 100);
    }
}
