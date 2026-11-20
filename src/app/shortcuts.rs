//! The one list of keyboard shortcuts: every action that has a key, its name, where it
//! works, and the keys that run it by default. The keymap on top of it holds the keys
//! the person chose instead (Preferences > Shortcuts). The key handlers look keys up
//! here and run the action (`App::run_act` in `keys.rs`); the menus, tooltips and the
//! Keyboard Shortcuts window show the keys from here, so they always agree.

use egui::Key;

/// What is pressed: a key (with or without modifiers) or a typed symbol. Symbols
/// such as `$` or `@` sit on different keys on different keyboard layouts, so they
/// are matched by the character typed rather than by the key.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Trigger {
    Key(Key),
    Text(char),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Binding {
    pub trigger: Trigger,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

const fn b(k: Key, ctrl: bool, alt: bool, shift: bool) -> Binding {
    Binding { trigger: Trigger::Key(k), ctrl, alt, shift }
}
const fn key(k: Key) -> Binding {
    b(k, false, false, false)
}
const fn ctrl(k: Key) -> Binding {
    b(k, true, false, false)
}
const fn shift(k: Key) -> Binding {
    b(k, false, false, true)
}
const fn alt(k: Key) -> Binding {
    b(k, false, true, false)
}
const fn ctrl_shift(k: Key) -> Binding {
    b(k, true, false, true)
}
const fn ctrl_alt(k: Key) -> Binding {
    b(k, true, true, false)
}
const fn alt_shift(k: Key) -> Binding {
    b(k, false, true, true)
}
const fn text(t: char) -> Binding {
    Binding { trigger: Trigger::Text(t), ctrl: false, alt: false, shift: false }
}

/// Where an action's keys work.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ctx {
    /// Everywhere a document is open (scores and chord charts).
    Global,
    /// Scores only.
    Score,
    /// Chord charts only.
    Chart,
}

/// Headings of the Keyboard Shortcuts window, in its order.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Group {
    File,
    Edit,
    Moving,
    Selecting,
    Notes,
    Effects,
    Bars,
    Voices,
    Tracks,
    Playback,
    View,
    Charts,
}

impl Group {
    pub const ALL: [Group; 12] = [
        Group::File,
        Group::Edit,
        Group::Moving,
        Group::Selecting,
        Group::Notes,
        Group::Effects,
        Group::Bars,
        Group::Voices,
        Group::Tracks,
        Group::Playback,
        Group::View,
        Group::Charts,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Group::File => "Files",
            Group::Edit => "Editing",
            Group::Moving => "Moving around",
            Group::Selecting => "Selecting",
            Group::Notes => "Notes and rhythm",
            Group::Effects => "Effects",
            Group::Bars => "Bars and sections",
            Group::Voices => "Voices",
            Group::Tracks => "Tracks",
            Group::Playback => "Playback",
            Group::View => "Windows and view",
            Group::Charts => "Chord charts",
        }
    }
}

macro_rules! acts {
    ($($v:ident),* $(,)?) => {
        /// Every action that has a keyboard shortcut.
        #[derive(Clone, Copy, PartialEq, Eq, Debug)]
        pub enum Act { $($v),* }
        /// Every `Act`, so the tests can check each one is listed once.
        pub const ALL_ACTS: &[Act] = &[$(Act::$v),*];
    };
}

acts! {
    // Files
    New, NewChart, Open, Browse, Close, CloseAll, Save, SaveAs, Print, Prefs, Quit,
    // Editing
    Undo, Redo, Cut, Copy, Paste, AllTrackCut, AllTrackCopy, SpecialPaste, SelectAll,
    CommandPalette, GoTo, CheckBars, DeleteNote, DeleteBeats, InsertBeat, CopyLastBeat,
    // Moving around
    MoveRight, MoveLeft, StringUp, StringDown, BarStart, BarEnd, NextBar, PrevBar,
    FirstBar, LastBar, ForwardFourBars, BackFourBars, NextStaff, PrevStaff,
    NextSection, PrevSection, StepForward, StepBack, Digits,
    // Selecting
    SelectNextBeat, SelectPrevBeat, SelectToBarStart, SelectToBarEnd, SelectNextBar,
    SelectPrevBar, SelectToStart, SelectToEnd,
    // Notes and rhythm
    Shorter, Longer, Dotted, DoubleDotted, Triplet, Rest, TieNote, TieBeat, GraceBefore,
    GraceOnBeat, Ghost, DeadNote, Accent, HeavyAccent, Staccato, SemitoneUp, SemitoneDown,
    OctaveUp, OctaveDown, NoteStringUp, NoteStringDown, MoveOnStaffUp, MoveOnStaffDown,
    SpellFlats, SpellAuto, SpellSharps, CycleSpelling, Chord, Text, Fermata,
    // Effects
    Hammer, Legato, Slide, Bend, Vibrato, WhammyVibrato, WideWhammyVibrato, TremoloBar,
    Trill, TremoloPicking, PalmMuteNote, PalmMuteBeat, LetRing, NaturalHarmonic,
    ArtificialHarmonic, Tapping, LhTapping, Slap, PickDown, PickUp, BrushDown, BrushUp,
    ArpeggioDown, ArpeggioUp, Barre, Rasgueado, FadeIn, FadeOut, Swell, WahOpen,
    WahClosed, Timer, ScaleDiagram,
    // Bars and sections
    InsertBar, DeleteBar, KeySig, TimeSig, TripletFeel, FreeTime, RepeatOpen, RepeatClose,
    RepeatBar, RepeatTwoBars, ForceBreak, Directions, Section, Automations,
    // Voices
    Voice1, Voice2, Voice3, Voice4, MoveToVoice1, MoveToVoice2, MoveToVoice3,
    MoveToVoice4, MoveVoices,
    // Tracks
    AddTrack, DeleteTrack, NextTrack, PrevTrack,
    // Playback
    PlayPause, PlayFromStart, Stop, Loop,
    // Windows and view
    Help, ShowPalette, ShowGlobalView, SongInspector, TrackInspector, KeyboardView,
    Multitrack, Stylesheet, ZoomIn, ZoomOut, FullScreen, NextTab, PrevTab,
    // Chord charts
    ChartTranspose,
}

pub struct Action {
    pub act: Act,
    pub name: &'static str,
    pub group: Group,
    /// Where its keys work.
    pub ctx: Ctx,
    /// The keys, the main one first (it is the one the menus show).
    pub keys: &'static [Binding],
    /// Keys that do the same on other keyboard layouts (such as = for +): they
    /// work, but aren't listed.
    pub aliases: &'static [Binding],
    /// Typing and moving keys that stay as they are (they can't be changed).
    pub fixed: bool,
    /// How to show the keys when the list of them would be awkward (such as 0–9).
    pub shown: Option<&'static str>,
}

const fn a(act: Act, name: &'static str, group: Group, ctx: Ctx, keys: &'static [Binding]) -> Action {
    Action { act, name, group, ctx, keys, aliases: &[], fixed: false, shown: None }
}

impl Action {
    const fn also(mut self, aliases: &'static [Binding]) -> Action {
        self.aliases = aliases;
        self
    }
    const fn fixed(mut self) -> Action {
        self.fixed = true;
        self
    }
    const fn shown(mut self, s: &'static str) -> Action {
        self.shown = Some(s);
        self
    }
}

use Act::*;
use Ctx::{Chart, Global, Score};
use Group as G;

pub const ACTIONS: &[Action] = &[
    // ----- Files -----
    a(New, "New song", G::File, Global, &[ctrl(Key::N)]),
    a(NewChart, "New chord chart", G::File, Global, &[ctrl_shift(Key::N)]),
    a(Open, "Open a file", G::File, Global, &[ctrl(Key::O)]),
    a(Browse, "Start screen (Browse)", G::File, Score, &[ctrl(Key::B)]),
    a(Close, "Close the file", G::File, Global, &[ctrl(Key::W)]),
    a(CloseAll, "Close all files", G::File, Score, &[ctrl_shift(Key::W)]),
    a(Save, "Save", G::File, Global, &[ctrl(Key::S)]),
    a(SaveAs, "Save as", G::File, Global, &[ctrl_shift(Key::S)]),
    a(Print, "Print", G::File, Global, &[ctrl(Key::P)]),
    a(Prefs, "Preferences", G::File, Global, &[ctrl(Key::Comma)]),
    a(Quit, "Quit", G::File, Global, &[alt(Key::F4)]).fixed(),
    // ----- Editing -----
    a(Undo, "Undo", G::Edit, Global, &[ctrl(Key::Z)]),
    a(Redo, "Redo", G::Edit, Global, &[ctrl(Key::Y)]).also(&[ctrl_shift(Key::Z)]),
    a(Cut, "Cut", G::Edit, Score, &[ctrl(Key::X)]),
    a(Copy, "Copy", G::Edit, Score, &[ctrl(Key::C)]),
    a(Paste, "Paste", G::Edit, Score, &[ctrl(Key::V)]),
    a(AllTrackCut, "Cut in all tracks", G::Edit, Score, &[ctrl_shift(Key::X)]),
    a(AllTrackCopy, "Copy in all tracks", G::Edit, Score, &[ctrl_shift(Key::C)]),
    a(SpecialPaste, "Special paste", G::Edit, Score, &[ctrl_shift(Key::V)]),
    a(SelectAll, "Select all", G::Edit, Score, &[ctrl(Key::A)]),
    a(CommandPalette, "Command palette", G::Edit, Score, &[ctrl(Key::E)]),
    a(GoTo, "Go to a bar", G::Edit, Score, &[ctrl(Key::G)]),
    a(CheckBars, "Check bar duration", G::Edit, Score, &[key(Key::F4)]),
    a(DeleteNote, "Delete the note", G::Edit, Score, &[key(Key::Delete)]).also(&[key(Key::Backspace)]),
    a(DeleteBeats, "Delete the beats", G::Edit, Score, &[shift(Key::Delete)]),
    a(InsertBeat, "Insert a beat", G::Edit, Score, &[key(Key::Insert)]),
    a(CopyLastBeat, "Copy the last beat here", G::Edit, Score, &[key(Key::C)]),
    // ----- Moving around -----
    a(MoveRight, "Next beat", G::Moving, Score, &[key(Key::ArrowRight)]).fixed(),
    a(MoveLeft, "Previous beat", G::Moving, Score, &[key(Key::ArrowLeft)]).fixed(),
    a(StringUp, "String above (or up the staff)", G::Moving, Score, &[key(Key::ArrowUp)]).fixed(),
    a(StringDown, "String below (or down the staff)", G::Moving, Score, &[key(Key::ArrowDown)]).fixed(),
    a(BarStart, "First beat of the bar", G::Moving, Score, &[key(Key::Home)]).fixed(),
    a(BarEnd, "Last beat of the bar", G::Moving, Score, &[key(Key::End)]).fixed(),
    a(NextBar, "Next bar", G::Moving, Score, &[ctrl(Key::ArrowRight)]),
    a(PrevBar, "Previous bar", G::Moving, Score, &[ctrl(Key::ArrowLeft)]),
    a(FirstBar, "First bar", G::Moving, Score, &[ctrl(Key::Home)]),
    a(LastBar, "Last bar", G::Moving, Score, &[ctrl(Key::End)]),
    a(ForwardFourBars, "Four bars on", G::Moving, Score, &[key(Key::PageDown)]).fixed(),
    a(BackFourBars, "Four bars back", G::Moving, Score, &[key(Key::PageUp)]).fixed(),
    a(NextStaff, "Notation \u{2194} tab, then the next track", G::Moving, Score, &[key(Key::Tab)]).fixed(),
    a(PrevStaff, "Back between notation and tab and tracks", G::Moving, Score, &[shift(Key::Tab)]).fixed(),
    a(NextSection, "Next section", G::Moving, Score, &[ctrl_alt(Key::ArrowRight)]),
    a(PrevSection, "Previous section", G::Moving, Score, &[ctrl_alt(Key::ArrowLeft)]),
    a(StepForward, "Step forward and hear the beat", G::Moving, Score, &[alt(Key::ArrowRight)]),
    a(StepBack, "Step backward and hear the beat", G::Moving, Score, &[alt(Key::ArrowLeft)]),
    a(Digits, "Tab: type a fret. Notation: 1\u{2013}9 picks the string, 0 the best one", G::Moving, Score, &[
        key(Key::Num0), key(Key::Num1), key(Key::Num2), key(Key::Num3), key(Key::Num4),
        key(Key::Num5), key(Key::Num6), key(Key::Num7), key(Key::Num8), key(Key::Num9),
    ])
    .fixed()
    .shown("0 \u{2013} 9"),
    // ----- Selecting -----
    a(SelectNextBeat, "Select to the next beat", G::Selecting, Score, &[shift(Key::ArrowRight)]),
    a(SelectPrevBeat, "Select to the previous beat", G::Selecting, Score, &[shift(Key::ArrowLeft)]),
    a(SelectToBarStart, "Select to the start of the bar", G::Selecting, Score, &[shift(Key::Home)]),
    a(SelectToBarEnd, "Select to the end of the bar", G::Selecting, Score, &[shift(Key::End)]),
    a(SelectNextBar, "Select to the next bar", G::Selecting, Score, &[ctrl_shift(Key::ArrowRight)]),
    a(SelectPrevBar, "Select to the previous bar", G::Selecting, Score, &[ctrl_shift(Key::ArrowLeft)]),
    a(SelectToStart, "Select to the first bar", G::Selecting, Score, &[ctrl_shift(Key::Home)]),
    a(SelectToEnd, "Select to the last bar", G::Selecting, Score, &[ctrl_shift(Key::End)]),
    // ----- Notes and rhythm -----
    a(Shorter, "Shorter note", G::Notes, Score, &[key(Key::Plus)]).also(&[key(Key::Equals), shift(Key::Plus), shift(Key::Equals)]),
    a(Longer, "Longer note", G::Notes, Score, &[key(Key::Minus)]).also(&[shift(Key::Minus)]),
    a(Dotted, "Dotted note or rest", G::Notes, Score, &[key(Key::Period)]).also(&[text('*')]),
    a(DoubleDotted, "Double-dotted note or rest", G::Notes, Score, &[ctrl(Key::Period)]),
    a(Triplet, "Triplet", G::Notes, Score, &[text('/')]),
    a(Rest, "Rest", G::Notes, Score, &[key(Key::R)]),
    a(TieNote, "Tie the note", G::Notes, Score, &[key(Key::L)]),
    a(TieBeat, "Tie the beat", G::Notes, Score, &[shift(Key::L)]),
    a(GraceBefore, "Grace note before the beat", G::Notes, Score, &[key(Key::G)]),
    a(GraceOnBeat, "Grace note on the beat", G::Notes, Score, &[ctrl_alt(Key::G)]),
    a(Ghost, "Ghost note", G::Notes, Score, &[key(Key::O)]),
    a(DeadNote, "Dead note", G::Notes, Score, &[key(Key::X)]),
    a(Accent, "Accent", G::Notes, Score, &[text(';')]),
    a(HeavyAccent, "Heavy accent", G::Notes, Score, &[text(':')]),
    a(Staccato, "Staccato", G::Notes, Score, &[text('!')]),
    a(SemitoneUp, "Notes up a semitone", G::Notes, Score, &[alt_shift(Key::ArrowUp)]),
    a(SemitoneDown, "Notes down a semitone", G::Notes, Score, &[alt_shift(Key::ArrowDown)]),
    a(OctaveUp, "Notes up an octave", G::Notes, Score, &[alt_shift(Key::PageUp)]),
    a(OctaveDown, "Notes down an octave", G::Notes, Score, &[alt_shift(Key::PageDown)]),
    a(NoteStringUp, "Move the note to the string above", G::Notes, Score, &[alt(Key::ArrowUp)]),
    a(NoteStringDown, "Move the note to the string below", G::Notes, Score, &[alt(Key::ArrowDown)]),
    a(MoveOnStaffUp, "Move the note up on the staff", G::Notes, Score, &[ctrl_alt(Key::ArrowUp)]),
    a(MoveOnStaffDown, "Move the note down on the staff", G::Notes, Score, &[ctrl_alt(Key::ArrowDown)]),
    a(SpellFlats, "Spell with flats", G::Notes, Score, &[ctrl(Key::Num7)]),
    a(SpellAuto, "Spell automatically", G::Notes, Score, &[ctrl(Key::Num8)]),
    a(SpellSharps, "Spell with sharps", G::Notes, Score, &[ctrl(Key::Num9)]),
    a(CycleSpelling, "Change the spelling", G::Notes, Score, &[ctrl_alt(Key::Num8)]),
    a(Chord, "Chord", G::Notes, Score, &[key(Key::A)]),
    a(Text, "Text", G::Notes, Score, &[key(Key::T)]),
    a(Fermata, "Fermata", G::Notes, Score, &[key(Key::F)]),
    // ----- Effects -----
    a(Hammer, "Hammer-on / pull-off", G::Effects, Score, &[key(Key::H)]),
    a(Legato, "Legato", G::Effects, Score, &[shift(Key::H)]),
    a(Slide, "Slide", G::Effects, Score, &[key(Key::S)]),
    a(Bend, "Bend", G::Effects, Score, &[key(Key::B)]),
    a(Vibrato, "Vibrato", G::Effects, Score, &[key(Key::V)]),
    a(WhammyVibrato, "Vibrato with the tremolo bar (slight)", G::Effects, Score, &[key(Key::W)]),
    a(WideWhammyVibrato, "Vibrato with the tremolo bar (wide)", G::Effects, Score, &[ctrl_alt(Key::W)]),
    a(TremoloBar, "Tremolo bar", G::Effects, Score, &[ctrl_alt(Key::V)]),
    a(Trill, "Trill", G::Effects, Score, &[key(Key::N)]),
    a(TremoloPicking, "Tremolo picking", G::Effects, Score, &[text('"')]),
    a(PalmMuteNote, "Palm mute on the note", G::Effects, Score, &[key(Key::P)]),
    a(PalmMuteBeat, "Palm mute on the beat", G::Effects, Score, &[shift(Key::P)]),
    a(LetRing, "Let ring", G::Effects, Score, &[key(Key::I)]),
    a(NaturalHarmonic, "Natural harmonic", G::Effects, Score, &[key(Key::Y)]),
    a(ArtificialHarmonic, "Artificial harmonic", G::Effects, Score, &[ctrl_alt(Key::Y)]),
    a(Tapping, "Tapping", G::Effects, Score, &[text(')')]),
    a(LhTapping, "Left-hand tapping", G::Effects, Score, &[text('(')]),
    a(Slap, "Slap", G::Effects, Score, &[text('$')]),
    a(PickDown, "Pickstroke down", G::Effects, Score, &[shift(Key::D)]),
    a(PickUp, "Pickstroke up", G::Effects, Score, &[shift(Key::U)]),
    a(BrushDown, "Brush down", G::Effects, Score, &[ctrl(Key::D)]),
    a(BrushUp, "Brush up", G::Effects, Score, &[ctrl(Key::U)]),
    a(ArpeggioDown, "Arpeggio down", G::Effects, Score, &[ctrl_shift(Key::D)]),
    a(ArpeggioUp, "Arpeggio up", G::Effects, Score, &[ctrl_shift(Key::U)]),
    a(Barre, "Barre", G::Effects, Score, &[shift(Key::I)]),
    a(Rasgueado, "Rasgueado", G::Effects, Score, &[shift(Key::R)]),
    a(FadeIn, "Fade in", G::Effects, Score, &[text('<')]),
    a(FadeOut, "Fade out", G::Effects, Score, &[text('>')]),
    a(Swell, "Volume swell", G::Effects, Score, &[alt_shift(Key::Period)]).also(&[alt_shift(Key::Comma)]),
    a(WahOpen, "Wah open", G::Effects, Score, &[ctrl_alt(Key::O)]),
    a(WahClosed, "Wah closed", G::Effects, Score, &[ctrl_alt(Key::C)]),
    a(Timer, "Timer", G::Effects, Score, &[text('@')]),
    a(ScaleDiagram, "Scale diagram (Chords & Scales)", G::Effects, Score, &[shift(Key::S)]),
    // ----- Bars and sections -----
    a(InsertBar, "Insert a bar", G::Bars, Score, &[ctrl(Key::Insert)]),
    a(DeleteBar, "Delete the bar", G::Bars, Score, &[ctrl(Key::Delete)]),
    a(KeySig, "Key signature", G::Bars, Score, &[ctrl(Key::K)]).also(&[key(Key::K)]),
    a(TimeSig, "Time signature", G::Bars, Score, &[ctrl(Key::T)]),
    a(TripletFeel, "Triplet feel", G::Bars, Score, &[ctrl(Key::Slash)]),
    a(FreeTime, "Free time", G::Bars, Score, &[alt_shift(Key::L)]),
    a(RepeatOpen, "Repeat open", G::Bars, Score, &[text('[')]),
    a(RepeatClose, "Repeat close", G::Bars, Score, &[text(']')]),
    a(RepeatBar, "Repeat the previous bar", G::Bars, Score, &[text('%')]),
    a(RepeatTwoBars, "Repeat the two previous bars", G::Bars, Score, &[ctrl_shift(Key::Num5)]),
    a(ForceBreak, "New line after this bar", G::Bars, Score, &[ctrl(Key::Enter)]),
    a(Directions, "Directions (Segno, Coda, D.C., D.S.)", G::Bars, Score, &[key(Key::D)]),
    a(Section, "Section", G::Bars, Score, &[shift(Key::Insert)]),
    a(Automations, "Tempo and automations", G::Bars, Global, &[key(Key::F10)]),
    // ----- Voices -----
    a(Voice1, "Edit voice 1", G::Voices, Score, &[ctrl(Key::Num1)]),
    a(Voice2, "Edit voice 2", G::Voices, Score, &[ctrl(Key::Num2)]),
    a(Voice3, "Edit voice 3", G::Voices, Score, &[ctrl(Key::Num3)]),
    a(Voice4, "Edit voice 4", G::Voices, Score, &[ctrl(Key::Num4)]),
    a(MoveToVoice1, "Move the notes to voice 1", G::Voices, Score, &[alt(Key::Num1)]),
    a(MoveToVoice2, "Move the notes to voice 2", G::Voices, Score, &[alt(Key::Num2)]),
    a(MoveToVoice3, "Move the notes to voice 3", G::Voices, Score, &[alt(Key::Num3)]),
    a(MoveToVoice4, "Move the notes to voice 4", G::Voices, Score, &[alt(Key::Num4)]),
    a(MoveVoices, "Move, copy or swap voices", G::Voices, Score, &[ctrl_shift(Key::M)]),
    // ----- Tracks -----
    a(AddTrack, "Add a track", G::Tracks, Score, &[ctrl_shift(Key::Insert)]),
    a(DeleteTrack, "Delete the track", G::Tracks, Score, &[ctrl_shift(Key::Delete)]),
    a(NextTrack, "Next track", G::Tracks, Score, &[ctrl(Key::ArrowDown)]),
    a(PrevTrack, "Previous track", G::Tracks, Score, &[ctrl(Key::ArrowUp)]),
    // ----- Playback -----
    a(PlayPause, "Play / pause", G::Playback, Score, &[key(Key::Space)]),
    a(PlayFromStart, "Play from the beginning", G::Playback, Score, &[ctrl(Key::Space)]),
    a(Stop, "Stop (and clear the selection)", G::Playback, Score, &[key(Key::Escape)]),
    a(Loop, "Loop", G::Playback, Score, &[key(Key::F9)]),
    // ----- Windows and view -----
    a(Help, "Help", G::View, Global, &[key(Key::F1)]),
    a(ShowPalette, "Editing palette", G::View, Score, &[key(Key::F2)]),
    a(ShowGlobalView, "Tracks (global view)", G::View, Score, &[key(Key::F8)]),
    a(SongInspector, "Song settings", G::View, Score, &[key(Key::F5)]),
    a(TrackInspector, "Track settings", G::View, Score, &[key(Key::F6)]),
    a(KeyboardView, "Keyboard view", G::View, Score, &[ctrl(Key::F6)]),
    a(Multitrack, "Multitrack", G::View, Score, &[key(Key::F3)]),
    a(Stylesheet, "Stylesheet", G::View, Global, &[key(Key::F7)]),
    a(ZoomIn, "Zoom in", G::View, Global, &[ctrl(Key::Plus)]).also(&[ctrl(Key::Equals)]),
    a(ZoomOut, "Zoom out", G::View, Global, &[ctrl(Key::Minus)]),
    a(FullScreen, "Full screen (Esc leaves it)", G::View, Score, &[key(Key::F11)]),
    a(NextTab, "Next tab", G::View, Global, &[ctrl(Key::Tab)]),
    a(PrevTab, "Previous tab", G::View, Global, &[ctrl_shift(Key::Tab)]),
    // ----- Chord charts -----
    a(ChartTranspose, "Transpose the chart", G::Charts, Chart, &[ctrl(Key::T)]),
];

/// The listing of an action.
pub fn action(act: Act) -> &'static Action {
    ACTIONS.iter().find(|x| x.act == act).expect("every action is listed (see the tests)")
}

/// The menu names of keys that differ from egui's own names.
const KEY_NAMES: [(Key, &str); 17] = [
    (Key::Insert, "Ins"),
    (Key::Delete, "Del"),
    (Key::Backspace, "Backspace"),
    (Key::Enter, "Return"),
    (Key::Escape, "Esc"),
    (Key::ArrowUp, "Up"),
    (Key::ArrowDown, "Down"),
    (Key::ArrowLeft, "Left"),
    (Key::ArrowRight, "Right"),
    (Key::PageUp, "PgUp"),
    (Key::PageDown, "PgDown"),
    (Key::Plus, "+"),
    (Key::Equals, "="),
    (Key::Minus, "-"),
    (Key::Comma, ","),
    (Key::Period, "."),
    (Key::Slash, "/"),
];

/// A key's name as the menus write it.
fn key_name(k: Key) -> &'static str {
    KEY_NAMES.iter().find(|(x, _)| *x == k).map(|(_, n)| *n).unwrap_or_else(|| k.name())
}

impl Binding {
    /// As the menus write it: "Ctrl+Shift+N", "Alt+Shift+Down", "$".
    pub fn label(&self) -> String {
        let mut s = String::new();
        if self.ctrl {
            s.push_str("Ctrl+");
        }
        if self.alt {
            s.push_str("Alt+");
        }
        if self.shift {
            s.push_str("Shift+");
        }
        match self.trigger {
            Trigger::Key(k) => s.push_str(key_name(k)),
            Trigger::Text(t) => s.push(t),
        }
        s
    }
}

// ----- the keymap: the default keys and the ones the person chose -----

impl Binding {
    /// The binding of a key pressed with these modifiers (Ctrl and Cmd count the same).
    pub fn of(k: Key, m: egui::Modifiers) -> Binding {
        b(k, m.command || m.ctrl, m.alt, m.shift)
    }

    /// A typed symbol (the modifiers that made it don't count).
    pub fn typed(c: char) -> Binding {
        text(c)
    }

    /// Whether this key may run its action while a text field has the keyboard:
    /// only keys with Ctrl or Alt, and function keys, so typing is never taken.
    pub fn works_while_typing(&self) -> bool {
        match self.trigger {
            Trigger::Text(_) => false,
            Trigger::Key(k) => self.ctrl || self.alt || is_function_key(k),
        }
    }

    /// As saved in the preferences: the menu name, with typed symbols in quotes
    /// ("'/'") so they aren't taken for the key of the same name.
    pub fn save_str(&self) -> String {
        match self.trigger {
            Trigger::Text(c) => format!("'{c}'"),
            Trigger::Key(_) => self.label(),
        }
    }

    /// Reads `save_str`'s text back (None for anything it doesn't know).
    pub fn parse(s: &str) -> Option<Binding> {
        let chars: Vec<char> = s.chars().collect();
        if chars.len() == 3 && chars[0] == '\'' && chars[2] == '\'' {
            return Some(text(chars[1]));
        }
        let (mut ctrl, mut alt, mut shift) = (false, false, false);
        let mut rest = s;
        loop {
            // "Ctrl++" is Ctrl with the + key: only take off a prefix that leaves a key.
            if let Some(r) = rest.strip_prefix("Ctrl+").filter(|r| !r.is_empty()) {
                ctrl = true;
                rest = r;
            } else if let Some(r) = rest.strip_prefix("Alt+").filter(|r| !r.is_empty()) {
                alt = true;
                rest = r;
            } else if let Some(r) = rest.strip_prefix("Shift+").filter(|r| !r.is_empty()) {
                shift = true;
                rest = r;
            } else {
                break;
            }
        }
        let k = KEY_NAMES.iter().find(|(_, n)| *n == rest).map(|(k, _)| *k).or_else(|| Key::from_name(rest))?;
        Some(b(k, ctrl, alt, shift))
    }
}

fn is_function_key(k: Key) -> bool {
    let n = k.name();
    n.len() > 1 && n.starts_with('F') && n[1..].chars().all(|c| c.is_ascii_digit())
}

impl Ctx {
    /// Whether keys of an action that works in `self` also work in `other`.
    fn overlaps(self, other: Ctx) -> bool {
        self == other || self == Ctx::Global || other == Ctx::Global
    }
}

/// Where a key is pressed, for looking it up.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Place {
    Score,
    Chart,
    /// The start screen: only actions that work everywhere.
    Home,
}

impl Place {
    fn has(self, c: Ctx) -> bool {
        match self {
            Place::Score => c != Ctx::Chart,
            Place::Chart => c != Ctx::Score,
            Place::Home => c == Ctx::Global,
        }
    }
}

/// The keys in use: the defaults, except for the actions the person changed.
#[derive(Clone, Default, PartialEq, Debug)]
pub struct Keymap {
    custom: Vec<(Act, Vec<Binding>)>,
}

impl Keymap {
    /// The keys shown for an action (the main one first).
    pub fn keys(&self, act: Act) -> Vec<Binding> {
        match self.custom.iter().find(|(a, _)| *a == act) {
            Some((_, k)) => k.clone(),
            None => action(act).keys.to_vec(),
        }
    }

    /// Every key that runs an action: its keys, and its aliases while it has its default keys.
    pub fn all_keys_of(&self, act: Act) -> Vec<Binding> {
        match self.custom.iter().find(|(a, _)| *a == act) {
            Some((_, k)) => k.clone(),
            None => action(act).keys.iter().chain(action(act).aliases).copied().collect(),
        }
    }

    pub fn is_custom(&self, act: Act) -> bool {
        self.custom.iter().any(|(a, _)| *a == act)
    }

    /// Gives an action these keys (its default keys make it default again).
    /// The keys of fixed actions can't be changed.
    pub fn set(&mut self, act: Act, keys: Vec<Binding>) {
        if action(act).fixed {
            return;
        }
        self.custom.retain(|(a, _)| *a != act);
        if keys != action(act).keys {
            self.custom.push((act, keys));
        }
    }

    pub fn reset(&mut self, act: Act) {
        self.custom.retain(|(a, _)| *a != act);
    }

    /// The action a key runs here, if any.
    pub fn find(&self, place: Place, k: &Binding) -> Option<Act> {
        ACTIONS.iter().filter(|x| place.has(x.ctx)).find(|x| self.all_keys_of(x.act).contains(k)).map(|x| x.act)
    }

    /// Another action that already uses this key where `act` works.
    pub fn clash(&self, act: Act, k: &Binding) -> Option<Act> {
        let ctx = action(act).ctx;
        ACTIONS
            .iter()
            .filter(|x| x.act != act && x.ctx.overlaps(ctx))
            .find(|x| self.all_keys_of(x.act).contains(k))
            .map(|x| x.act)
    }

    /// For the preferences file: only the changed actions, by name.
    pub fn to_saved(&self) -> std::collections::BTreeMap<String, Vec<String>> {
        self.custom.iter().map(|(a, k)| (format!("{a:?}"), k.iter().map(Binding::save_str).collect())).collect()
    }

    /// From the preferences file. Unknown actions and keys are left out, so an
    /// older or hand-edited file still loads.
    pub fn from_saved(saved: &std::collections::BTreeMap<String, Vec<String>>) -> Keymap {
        let mut m = Keymap::default();
        for (name, keys) in saved {
            if let Some(&act) = ALL_ACTS.iter().find(|a| format!("{a:?}") == *name) {
                m.set(act, keys.iter().filter_map(|k| Binding::parse(k)).collect());
            }
        }
        m
    }
}

thread_local! {
    /// The keymap in use. The window runs on one thread; each test has its own.
    static KEYMAP: std::cell::RefCell<Keymap> = std::cell::RefCell::new(Keymap::default());
}

pub fn keymap() -> Keymap {
    KEYMAP.with(|k| k.borrow().clone())
}

pub fn set_keymap(m: Keymap) {
    KEYMAP.with(|k| *k.borrow_mut() = m);
}

/// The action a key runs here, with the keymap in use.
pub fn find(place: Place, k: &Binding) -> Option<Act> {
    KEYMAP.with(|m| m.borrow().find(place, k))
}

/// The keys of an action in the keymap in use.
pub fn keys_of(act: Act) -> Vec<Binding> {
    KEYMAP.with(|m| m.borrow().keys(act))
}

/// The main shortcut of an action, as the menus show it ("" if it has none).
pub fn sc(act: Act) -> String {
    keys_of(act).first().map(Binding::label).unwrap_or_default()
}

/// Every shortcut of an action, for the Keyboard Shortcuts window: "Ctrl+Y / Ctrl+Shift+Z".
pub fn all_keys(act: Act) -> String {
    if let Some(s) = action(act).shown {
        return s.to_string();
    }
    let keys = keys_of(act);
    if keys.is_empty() {
        return "\u{2014}".into();
    }
    keys.iter().map(Binding::label).collect::<Vec<_>>().join("  /  ")
}

/// Text with `{Act}` written for shortcuts (the action's name, such as `{Bend}`), as
/// the person's keys: "bend ({Bend})" becomes "bend (B)". When the action has no key,
/// " ({Act})" is left out, and a bare `{Act}` says "no shortcut". An unknown name is
/// left as it is, so the tests can find it.
pub fn fill_keys(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find('{') {
        let Some(len) = rest[start..].find('}') else { break };
        let (before, name, after) = (&rest[..start], &rest[start + 1..start + len], &rest[start + len + 1..]);
        let Some(&act) = ALL_ACTS.iter().find(|a| format!("{a:?}") == name) else {
            out.push_str(&rest[..start + len + 1]);
            rest = after;
            continue;
        };
        let key = sc(act);
        if key.is_empty() && before.ends_with('(') && after.starts_with(')') {
            let b = &before[..before.len() - 1];
            out.push_str(b.strip_suffix(' ').unwrap_or(b));
            rest = &after[1..];
        } else {
            out.push_str(before);
            out.push_str(if key.is_empty() { "no shortcut" } else { &key });
            rest = after;
        }
    }
    out.push_str(rest);
    out
}

/// A tooltip with the shortcut after it: "Undo (Ctrl+Z)" (just the text if it has none).
pub fn tip(text: &str, act: Act) -> String {
    let k = sc(act);
    if k.is_empty() { text.to_string() } else { format!("{text} ({k})") }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_action_is_listed_once() {
        for &act in ALL_ACTS {
            let n = ACTIONS.iter().filter(|x| x.act == act).count();
            assert_eq!(n, 1, "{act:?} is listed {n} times");
        }
        assert_eq!(ACTIONS.len(), ALL_ACTS.len());
    }

    #[test]
    fn every_action_has_a_key() {
        for x in ACTIONS {
            assert!(!x.keys.is_empty(), "{:?} has no key", x.act);
        }
    }

    #[test]
    fn no_key_does_two_things_in_the_same_place() {
        let overlaps = |a: Ctx, b: Ctx| a == b || a == Ctx::Global || b == Ctx::Global;
        let mut seen: Vec<(Binding, Ctx, Act)> = vec![];
        for x in ACTIONS {
            for k in x.keys.iter().chain(x.aliases) {
                if let Some((_, _, other)) = seen.iter().find(|(kb, c, _)| kb == k && overlaps(*c, x.ctx)) {
                    panic!("{} is used by both {:?} and {:?}", k.label(), other, x.act);
                }
                seen.push((*k, x.ctx, x.act));
            }
        }
    }

    #[test]
    fn keys_are_named_as_the_menus_name_them() {
        assert_eq!(sc(Act::NewChart), "Ctrl+Shift+N");
        assert_eq!(sc(Act::SemitoneDown), "Alt+Shift+Down");
        assert_eq!(sc(Act::OctaveUp), "Alt+Shift+PgUp");
        assert_eq!(sc(Act::PrevSection), "Ctrl+Alt+Left");
        assert_eq!(sc(Act::InsertBar), "Ctrl+Ins");
        assert_eq!(sc(Act::DeleteBeats), "Shift+Del");
        assert_eq!(sc(Act::ForceBreak), "Ctrl+Return");
        assert_eq!(sc(Act::ZoomIn), "Ctrl++");
        assert_eq!(sc(Act::ZoomOut), "Ctrl+-");
        assert_eq!(sc(Act::Prefs), "Ctrl+,");
        assert_eq!(sc(Act::TripletFeel), "Ctrl+/");
        assert_eq!(sc(Act::Stop), "Esc");
        assert_eq!(sc(Act::PlayFromStart), "Ctrl+Space");
        assert_eq!(sc(Act::Quit), "Alt+F4");
        assert_eq!(sc(Act::Slap), "$");
        assert_eq!(sc(Act::Voice3), "Ctrl+3");
        assert_eq!(all_keys(Act::Redo), "Ctrl+Y");
        assert_eq!(all_keys(Act::Digits), "0 \u{2013} 9");
    }

    #[test]
    fn saved_keys_read_back_the_same() {
        for x in ACTIONS {
            for k in x.keys.iter().chain(x.aliases) {
                assert_eq!(Binding::parse(&k.save_str()), Some(*k), "{}", k.save_str());
            }
        }
        assert_eq!(Binding::parse("Ctrl++"), Some(ctrl(Key::Plus)));
        assert_eq!(Binding::parse("'/'"), Some(text('/')));
        assert_eq!(Binding::parse("/"), Some(key(Key::Slash)));
        assert_eq!(Binding::parse("Ctrl+Nonsense"), None);
    }

    #[test]
    fn a_changed_key_replaces_the_default_one() {
        let mut m = Keymap::default();
        assert_eq!(m.find(Place::Score, &key(Key::R)), Some(Act::Rest));
        m.set(Act::Rest, vec![key(Key::Q)]);
        assert_eq!(m.find(Place::Score, &key(Key::Q)), Some(Act::Rest));
        assert_eq!(m.find(Place::Score, &key(Key::R)), None, "the old key is free");
        let back = Keymap::from_saved(&m.to_saved());
        assert_eq!(back, m, "saved and loaded again");
        m.set(Act::Rest, vec![key(Key::R)]);
        assert!(!m.is_custom(Act::Rest), "the default keys make it default again");
    }

    #[test]
    fn aliases_go_with_the_default_keys() {
        let mut m = Keymap::default();
        assert_eq!(m.find(Place::Score, &key(Key::Equals)), Some(Act::Shorter));
        m.set(Act::Shorter, vec![key(Key::Q)]);
        assert_eq!(m.find(Place::Score, &key(Key::Equals)), None);
    }

    #[test]
    fn places_and_clashes() {
        let m = Keymap::default();
        assert_eq!(m.find(Place::Chart, &ctrl(Key::T)), Some(Act::ChartTranspose));
        assert_eq!(m.find(Place::Score, &ctrl(Key::T)), Some(Act::TimeSig));
        assert_eq!(m.find(Place::Home, &ctrl(Key::T)), None);
        assert_eq!(m.find(Place::Home, &ctrl(Key::N)), Some(Act::New));
        assert_eq!(m.clash(Act::Rest, &key(Key::B)), Some(Act::Bend));
        assert_eq!(m.clash(Act::ChartTranspose, &ctrl(Key::K)), None, "Ctrl+K is for scores only");
        assert_eq!(m.clash(Act::Save, &ctrl(Key::K)), Some(Act::KeySig), "Save works in scores too");
    }

    #[test]
    fn fixed_keys_stay() {
        let mut m = Keymap::default();
        m.set(Act::MoveRight, vec![key(Key::Q)]);
        assert_eq!(m.find(Place::Score, &key(Key::ArrowRight)), Some(Act::MoveRight));
    }

    #[test]
    fn guide_text_shows_the_keys_in_use() {
        assert_eq!(fill_keys("bend ({Bend}), slide ({Slide})."), "bend (B), slide (S).");
        let mut m = Keymap::default();
        m.set(Act::Bend, vec![ctrl_alt(Key::B)]);
        m.set(Act::Slide, vec![]);
        set_keymap(m);
        assert_eq!(fill_keys("bend ({Bend}), slide ({Slide})."), "bend (Ctrl+Alt+B), slide.");
        assert_eq!(fill_keys("Slide is {Slide}."), "Slide is no shortcut.");
        assert_eq!(fill_keys("a {Nonsense} b"), "a {Nonsense} b", "unknown names stay for the tests");
        set_keymap(Keymap::default());
    }

    #[test]
    fn typing_keeps_plain_keys() {
        assert!(!key(Key::R).works_while_typing());
        assert!(!text('$').works_while_typing());
        assert!(ctrl(Key::S).works_while_typing());
        assert!(key(Key::F7).works_while_typing());
        assert!(!key(Key::F).works_while_typing());
    }
}
