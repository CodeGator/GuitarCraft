//! The main window: state, editing commands, playback and keyboard input.
//! The panels are drawn in `panels.rs`.

mod about;
mod actions;
mod addtrack;
mod ai;
mod aicode;
mod aifacts;
mod aihelp;
mod aipanel;
mod aitext;
mod batch;
#[cfg(test)]
mod flows;
mod automation;
mod bottom;
mod chartstyle;
mod chartsuggest;
mod chartui;
mod chartxpose;
mod chordscales;
mod dock;
mod expression;
mod dockui;
mod float;
mod chordedit;
mod chordlib;
mod clipmenu;
mod cursor;
mod drumview;
mod home;
mod icons;
mod keys;
mod menus;
mod notice;
mod palette;
mod panels;
mod popups;
mod prefs;
mod shortcuts;
mod printing;
mod side;
mod stylesheet;
mod templates;
mod tabs;
mod tools;
mod update;
pub mod trackicons;
mod tuner;
mod tuning;
mod voices;
mod widgets;

use crate::audio::{self, Audio, ScheduleOptions, TimelineBar};
use crate::fileio;
use crate::model::*;
use crate::score::Notation;
use egui::Key;
use std::path::PathBuf;
use widgets::sanitize;

/// Choices from the command line. Most are for taking screenshots of a given view.
#[derive(Debug, Default)]
pub struct LaunchOptions {
    /// Save a picture of the window here, then quit.
    pub screenshot: Option<PathBuf>,
    /// A song file to open.
    pub open: Option<PathBuf>,
    /// Bar to put the cursor on, counting from 1.
    pub bar: Option<usize>,
    /// Track to select, counting from 1.
    pub track: Option<usize>,
    pub zoom: Option<f32>,
    /// Start on the home screen.
    pub home: bool,
    /// Extra views to show, such as "keyboard" or "fretboard".
    pub show: Vec<String>,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Pending {
    New,
    Open,
    Demo,
    Template(usize),
    Close,
    Quit,
    /// A new chord chart.
    NewChart,
    /// Close every open file (File > Close All, Ctrl+Shift+W).
    CloseAll,
}

#[derive(Clone, Copy, PartialEq)]
enum InspectorTab {
    Song,
    Track,
}

pub struct App {
    song: Song,
    path: Option<PathBuf>,
    dirty: bool,
    track: usize,
    bar: usize,
    beat: usize,
    string: usize,
    sel_anchor: Option<(usize, usize)>,
    undo: Vec<Song>,
    redo: Vec<Song>,
    clipboard: Vec<Beat>,
    cur_dur: Dur,
    zoom: f32,
    notation: Notation,
    audio: Option<Audio>,
    sound_allowed: bool,
    playing: bool,
    timeline: Vec<TimelineBar>,
    start_offset: u64,
    play_start_bar: usize,
    play_beat: Option<(usize, usize)>,
    /// Where playback is: the bar and the exact tick within it.
    play_tick: Option<(usize, f32)>,
    play_secs: f32,
    total_secs: f32,
    metronome: bool,
    count_in: bool,
    looping: bool,
    speed: f32,
    master_volume: f32,
    /// The track whose equalizer window is open.
    eq_track: Option<usize>,
    tuner: tuner::TunerDlg,
    /// The mixer's Focus knob, -1..1: above 0 the selected track stands out (the others
    /// are turned down), below 0 it is pushed back.
    focus: f32,
    /// The equalizers and track gains last sent to the player.
    mix_sent: (Vec<crate::model::Equalizer>, Vec<f32>),
    status: String,
    last_digit: Option<(u8, f64, usize, usize, usize)>,
    /// Where each panel is: docked, tabbed or floating in the window (see `dock.rs`).
    dock: egui_dock::DockState<dock::Pane>,
    /// The panels popped out into windows of their own (see `float.rs`).
    popped: Vec<dock::Pane>,
    /// While the dock is being drawn it is out of `dock`; panel changes asked for then wait here
    /// (the panel, and whether to show it).
    dock_busy: bool,
    dock_cmds: Vec<(dock::Pane, bool)>,
    /// The layout as last saved, to save again only when it changed; `None` for pictures and
    /// tests, which never touch the person's file.
    layout_saved: Option<String>,
    /// The notes sounding now, for the fretboard and keyboard.
    sounding: Vec<(usize, u8, u8)>,
    /// Where the score was drawn last frame (toasts sit in it).
    score_rect: egui::Rect,
    inspector: InspectorTab,
    follow: bool,
    scroll_to_cursor: bool,
    music_font: bool,
    screenshot: Option<PathBuf>,
    frame: u64,
    shot_requested: bool,
    pending: Option<Pending>,
    show_help: bool,
    show_about: bool,
    /// Pictures for checks: the File menu open on Import (false) or Export (true).
    file_menu_demo: Option<bool>,
    /// The Batch Converter window, when open.
    batch: Option<batch::BatchDlg>,
    /// The About window's tab and the Open Source list.
    about: about::AboutState,
    /// Help > Check for Updates.
    update: update::UpdateState,
    title_cache: String,
    show_home: bool,
    single_track: bool,
    toast: (String, f64),
    recent: Vec<PathBuf>,
    home_tab: usize,
    home_sel: usize,
    home_search: String,
    preview: Option<(String, home::Preview)>,
    kb_octave: i32,
    play_while_editing: bool,
    /// 0 page vertical, 1 page horizontal, 2 page grid, 3 screen vertical, 4 screen horizontal.
    view_mode: u8,
    /// The split view: which of the two score views (0 top, 1 bottom) was last clicked, and how
    /// much of the height the top one takes.
    score_active: usize,
    split_frac: f32,
    dir_open: bool,
    /// The Add Track window.
    at: addtrack::AddTrack,
    /// Where to suggest saving a Guitar Pro file opened in another format.
    save_hint: Option<PathBuf>,
    /// The song tab shows every information field (More) or only title and artist (Less).
    song_more: bool,
    /// The icon list beside the track name is open (for checking pictures).
    icon_menu_open: bool,
    tuning_dlg: Option<tuning::TuningDlg>,
    chord_open: bool,
    chord_ed: chordedit::ChordEd,
    /// The Chords & Scales tool.
    cs: chordscales::ChordScales,
    lyrics_open: bool,
    lyrics_snap: bool,
    cmd_open: bool,
    cmd_text: String,
    cmd_sel: usize,
    /// The AI Assistant panel.
    ai: aipanel::AiPanel,
    trainer_open: bool,
    trainer_on: bool,
    trainer_start: f32,
    trainer_end: f32,
    trainer_step: f32,
    guide_open: bool,
    last_copy_all: bool,
    keep_pitch: bool,
    transpose_sel_only: bool,
    transpose_key: bool,
    transpose_diatonic: bool,
    last_play_pos: u64,
    metro_flash: bool,
    /// Staff position of the cursor when it is in the notation staff (None = in the tab).
    std_cursor: Option<i32>,
    /// The voice being edited (0 = voice 1).
    voice: usize,
    voices_open: bool,
    auto_open: bool,
    auto_sel: usize,
    auto_snap: bool,
    auto_drag: bool,
    vaction: voices::VoiceAction,
    vfrom: usize,
    vto: usize,
    vrange: voices::VoiceRange,
    popup: Option<popups::Popup>,
    pop_bar: MasterBar,
    pop_beat: Beat,
    pop_note: Note,
    pop_text: String,
    pop_until_next: bool,
    goto_open: bool,
    goto_text: String,
    transpose_open: bool,
    transpose_amt: i32,
    transpose_all: bool,
    bar_clipboard: Vec<(MasterBar, Vec<Bar>)>,
    fullscreen: bool,
    fullscreen_changed: bool,
    focus_request: Option<String>,
    /// The Stylesheet window, while open.
    style_dlg: Option<stylesheet::StyleDlg>,
    /// The chord diagram library window (Chords button of the palette).
    chord_lib_open: bool,
    /// The PDF / PNG export window, while open.
    export_dlg: Option<printing::ExportDlg>,
    /// The print preview window, while open.
    print_preview: Option<printing::PrintPreview>,
    /// The Special Paste window, while open.
    special_paste: Option<clipmenu::SpecialPaste>,
    /// Pictures for checks: the score's right-click menu shown by the cursor.
    ctx_menu_demo: bool,
    /// Pictures for checks: a bar number shown as if the mouse were over it.
    bar_hover_demo: Option<usize>,
    /// During playback: the line of music the view last followed.
    follow_sys: Option<usize>,
    /// The chord chart being edited, when one is open (instead of a score).
    chart: Option<chartui::ChartState>,
    /// The open files, one per tab. The showing tab's data is in the fields above
    /// (its entry here is None); the others are kept here.
    tabs: Vec<Option<tabs::Doc>>,
    active_tab: usize,
    /// The window is closing for good (every question about unsaved changes is answered).
    quitting: bool,
    /// Whether Insert and Delete were held at the last frame (Windows hides some of their shortcuts).
    ins_down: bool,
    del_down: bool,
    /// File > Preferences: the person's settings, kept between runs.
    prefs: prefs::Prefs,
    /// The MIDI output and input ports Preferences chose.
    midi: crate::midi::Midi,
    /// Why a chosen MIDI port could not be opened.
    midi_error: Option<String>,
    /// Notes played on the MIDI input that are waiting for the rest of their chord.
    midi_pending: Vec<(u8, std::time::Instant)>,
    /// The Preferences window, while open.
    prefs_dlg: Option<prefs::PrefsDlg>,
    /// Something was copied this frame: mark the system clipboard so pasting is offered.
    clip_mark: bool,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, launch: LaunchOptions) -> App {
        App::with_ctx(&cc.egui_ctx, launch)
    }

    /// The app, made from an egui context (tests make theirs without a window).
    pub fn with_ctx(ctx: &egui::Context, launch: LaunchOptions) -> App {
        let LaunchOptions {
            screenshot,
            open,
            bar,
            track,
            zoom,
            home,
            show,
        } = launch;
        // Pictures and tests always start from the standard settings, not the person's.
        let own_settings = screenshot.is_none() && !cfg!(test);
        let no_file_asked = open.is_none();
        let music_font = crate::setup_fonts(ctx);
        crate::setup_style(ctx);
        // Open on the start screen (File > Browse), as Guitar Pro 8 does. Screenshot runs
        // keep starting in the score; they ask for the start screen with --home.
        let start_on_home = screenshot.is_none();
        let mut app = App {
            song: Song::demo(),
            path: None,
            dirty: false,
            track: 0,
            bar: 0,
            beat: 0,
            string: 0,
            sel_anchor: None,
            undo: vec![],
            redo: vec![],
            clipboard: vec![],
            cur_dur: Dur::Eighth,
            zoom: 1.0,
            notation: Notation::Both,
            audio: None,
            sound_allowed: screenshot.is_none(),
            playing: false,
            timeline: vec![],
            start_offset: 0,
            play_start_bar: 0,
            play_beat: None,
            play_tick: None,
            play_secs: 0.0,
            total_secs: 0.0,
            metronome: false,
            count_in: false,
            looping: false,
            speed: 100.0,
            master_volume: 0.9,
            eq_track: None,
            tuner: tuner::TunerDlg::default(),
            focus: 0.0,
            mix_sent: (vec![], vec![]),
            status: "Ready. Click the score or use the arrow keys, then type fret numbers.".into(),
            last_digit: None,
            dock: dock::default_layout(),
            popped: vec![],
            dock_busy: false,
            dock_cmds: vec![],
            layout_saved: None,
            sounding: vec![],
            score_rect: egui::Rect::NOTHING,
            inspector: InspectorTab::Song,
            follow: true,
            scroll_to_cursor: false,
            music_font,
            screenshot,
            frame: 0,
            shot_requested: false,
            pending: None,
            show_help: false,
            show_about: false,
            file_menu_demo: None,
            batch: None,
            about: Default::default(),
            update: Default::default(),
            title_cache: String::new(),
            show_home: start_on_home,
            single_track: false,
            toast: (String::new(), 0.0),
            recent: vec![],
            home_tab: 0,
            home_sel: 0,
            home_search: String::new(),
            preview: None,
            kb_octave: 1,
            play_while_editing: true,
            view_mode: 0,
            score_active: 0,
            split_frac: 0.5,
            dir_open: false,
            at: Default::default(),
            save_hint: None,
            song_more: false,
            icon_menu_open: false,
            tuning_dlg: None,
            chord_open: false,
            chord_ed: Default::default(),
            cs: Default::default(),
            lyrics_open: false,
            lyrics_snap: false,
            cmd_open: false,
            cmd_text: String::new(),
            cmd_sel: 0,
            ai: Default::default(),
            trainer_open: false,
            trainer_on: false,
            trainer_start: 60.0,
            trainer_end: 100.0,
            trainer_step: 5.0,
            guide_open: false,
            last_copy_all: false,
            keep_pitch: true,
            transpose_sel_only: false,
            transpose_key: false,
            transpose_diatonic: false,
            last_play_pos: 0,
            metro_flash: false,
            std_cursor: None,
            voice: 0,
            voices_open: false,
            auto_open: false,
            auto_sel: 0,
            auto_snap: false,
            auto_drag: false,
            vaction: voices::VoiceAction::Move,
            vfrom: 0,
            vto: 1,
            vrange: voices::VoiceRange::Bar,
            popup: None,
            pop_bar: MasterBar::default(),
            pop_beat: Beat::default(),
            pop_note: Note::default(),
            pop_text: String::new(),
            pop_until_next: true,
            goto_open: false,
            goto_text: String::new(),
            transpose_open: false,
            transpose_amt: 2,
            transpose_all: true,
            bar_clipboard: vec![],
            fullscreen: false,
            fullscreen_changed: false,
            focus_request: None,
            style_dlg: None,
            chord_lib_open: false,
            export_dlg: None,
            print_preview: None,
            special_paste: None,
            ctx_menu_demo: false,
            bar_hover_demo: None,
            follow_sys: None,
            chart: None,
            tabs: vec![],
            active_tab: 0,
            quitting: false,
            ins_down: false,
            del_down: false,
            prefs: if own_settings { prefs::Prefs::load() } else { prefs::Prefs::default() },
            prefs_dlg: None,
            midi: Default::default(),
            midi_error: None,
            midi_pending: vec![],
            clip_mark: false,
        };
        if own_settings {
            let layout = dock::load();
            app.dock = layout.dock;
            app.popped = layout.popped;
            app.layout_saved = Some(dock::to_json(&app.dock, &app.popped));
        }
        // The keys the person chose (the defaults for pictures and tests).
        shortcuts::set_keymap(shortcuts::Keymap::from_saved(&app.prefs.shortcuts));
        if own_settings {
            app.connect_midi();
        }
        if app.screenshot.is_none() {
            app.recent = home::load_recent();
        }
        if app.recent.is_empty() {
            app.home_tab = 2;
        }
        if let Some(p) = open {
            app.open_path(p);
        }
        if let Some(b) = bar {
            app.bar = b.saturating_sub(1);
            app.clamp_cursor();
            app.scroll_to_cursor = true;
        }
        // Extra views for checking pictures: --show keyboard,fretboard,track,screen
        for part in &show {
            match part.as_str() {
                "keyboard" => app.show_panel(dock::Pane::Keyboard),
                "ai" => app.show_panel(dock::Pane::Ai),
                "fretboard" => app.show_panel(dock::Pane::Fretboard),
                // The drumkit view, on the song's first drum track.
                "drums" => {
                    if let Some(t) = app.song.tracks.iter().position(|t| t.kind() == InstrumentType::Drums) {
                        app.track = t;
                        app.clamp_cursor();
                    }
                    app.show_panel(dock::Pane::Drums);
                }
                // A panel in a window of its own, for checking pop-out: --show popout.
                "popout" => app.pop_out(dock::Pane::Palette),
                "track" => app.inspector = InspectorTab::Track,
                "bar" => app.open_popup(popups::Popup::TimeSig),
                "key" => app.open_popup(popups::Popup::KeySig),
                "single" => app.single_track = true,
                // Preferences, on a given tab: prefs, prefs1 ... prefs4.
                p if p.starts_with("prefs") => {
                    app.open_prefs();
                    if let Some(d) = app.prefs_dlg.as_mut() {
                        d.tab = p[5..].parse().unwrap_or(0).min(prefs::TABS.len() - 1);
                    }
                }
                "more" => app.song_more = true,
                "playing" => {
                    // A picture of playback part way through bar 1 (no sound).
                    app.playing = true;
                    app.play_beat = Some((0, 3));
                    app.play_tick = Some((0, 1.6 * crate::model::QUARTER as f32));
                }
                "numbers" => {
                    // The numbered lane on every track, with a tie, a grace note, a three-note chord
                    // and a triplet in bar 2 of the first track, for checking.
                    app.song.numbers_check();
                }
                "short" => {
                    // Bar 1 one beat short and bar 2 half a beat long, for checking.
                    let t = &mut app.song.tracks[0];
                    t.bars[0].beats.pop();
                    t.bars[0].beats.pop();
                    let extra = t.bars[1].beats[0].clone();
                    t.bars[1].beats.push(extra);
                }
                "icons" => {
                    app.inspector = InspectorTab::Track;
                    app.icon_menu_open = true;
                }
                "tuning" => {
                    app.inspector = InspectorTab::Track;
                    app.open_tuning();
                }
                "eq" => {
                    // The first track's equalizer turned, and the Focus knob, for checking.
                    let e = &mut app.song.tracks[0].equalizer;
                    e.on = true;
                    e.knobs = [0.5, 0.622, 0.403, 0.5, 0.5, 0.684, 0.5, 0.324, 0.5];
                    app.eq_track = Some(0);
                    app.focus = 0.5;
                }
                "tuner" => {
                    // The tuner with a made-up strum (no microphone), for checking.
                    app.open_tuner();
                    app.tuner.sample = Some(crate::tuner::Readout {
                        level: 0.08,
                        strings: vec![Some(1.0), Some(12.0), None, Some(-24.0), Some(-2.0), Some(-60.0)],
                        mono: Some(110.0 * (7.0f32 / 1200.0).exp2()),
                    });
                }
                "partial" => {
                    app.inspector = InspectorTab::Track;
                    app.open_tuning();
                    if let Some(d) = app.tuning_dlg.as_mut() {
                        d.capo = 2;
                        d.partial = 4;
                        d.partial_strings = 0b001110;
                    }
                }
                "addtrack2" => {
                    app.open_add_track(false);
                    app.at.choose(Instrument::Piano);
                }
                "band" => {
                    // A band with vocals, piano, guitar, bass and drums, for checking.
                    app.song = home::band_song();
                    app.track = 0;
                    app.bar = 0;
                    app.beat = 0;
                    app.total_secs = app.song_seconds();
                }
                "ring" => {
                    // Select bars 3-4 and turn let ring on, for checking.
                    app.bar = 2;
                    app.beat = 0;
                    app.sel_anchor = Some((2, 0));
                    app.bar = 3;
                    app.beat = app.song.tracks[0].bars[3].beats.len() - 1;
                    app.toggle_note_flag("Let ring", |n| &mut n.let_ring);
                }
                "beat" => app.open_popup(popups::Popup::Text),
                "section" => app.open_popup(popups::Popup::Section),
                "bend" => app.open_popup(popups::Popup::Bend),
                "screen" => app.view_mode = 3,
                "grid" => app.view_mode = 2,
                "guide" => app.guide_open = true,
                "palette-cmd" => app.cmd_open = true,
                "directions" => app.dir_open = true,
                "chord" => app.open_chord_tool(),
                "chordlib" => app.chord_lib_open = true,
                "exportpdf" => app.open_export(printing::ExportKind::Pdf),
                "printpreview" => app.print_song(),
                "ctxmenu" => app.ctx_menu_demo = true,
                "chart3" => {
                    let mut st = chartui::ChartState::new(crate::chart::ChordChart::sample());
                    st.doc.look.columns = 3;
                    st.tab = 0;
                    app.chart = Some(st);
                    app.show_home = false;
                    app.status =
                        "Three columns: lines flow down each column, then on to the next.".into();
                }
                "chartsuggest" => {
                    // Suggest Chords on the first verse with a made-up answer (nothing is asked).
                    let mut st = chartui::ChartState::new(crate::chart::ChordChart::sample());
                    st.sel = 1;
                    app.prefs.claude_code_on = true;
                    app.chart = Some(st);
                    app.show_home = false;
                    app.open_suggest_chords();
                    if let Some(st) = &mut app.chart {
                        let words = st.doc.sections.get(st.sel).map(|s| chartsuggest::words_of(&s.text)).unwrap_or_default();
                        if let Some(d) = st.suggest.as_mut() {
                            d.style = "simple folk".into();
                            d.reply = Some(words.lines().map(|l| format!("[A]{l}")).collect::<Vec<_>>().join("\n"));
                        }
                    }
                }
                "chartdiagrams" => {
                    let mut st = chartui::ChartState::new(crate::chart::ChordChart::sample());
                    st.tab = 0;
                    st.doc.sections[1]
                        .text
                        .push_str("\n[Badd9]Weird [F#m7b5]chords [G6/9]too [D/F#]");
                    st.diagrams_open = true;
                    app.chart = Some(st);
                    app.show_home = false;
                    app.status = "Chord diagrams are worked out from the chord names.".into();
                }
                "charttime" | "chartkey" => {
                    let mut st = chartui::ChartState::new(crate::chart::ChordChart::sample());
                    st.tab = 0;
                    app.chart = Some(st);
                    app.show_home = false;
                    app.open_chart_sig(part == "charttime");
                    app.status = "The chart's key and time use the score's editors.".into();
                }
                "charttempo" => {
                    let mut st = chartui::ChartState::new(crate::chart::ChordChart::sample());
                    // A slower bridge that picks up again into the last chorus.
                    st.doc
                        .set_tempo_points(&[(0, 63, false), (10, 58, true), (11, 66, false)]);
                    st.tempo_open = true;
                    st.tempo_ed.sel = 10;
                    app.chart = Some(st);
                    app.show_home = false;
                    app.status = "The chart's BPM opens the same tempo editor as a score's.".into();
                }
                "about" | "about2" | "about3" => {
                    app.show_about = true;
                    app.about.tab = match part.as_str() {
                        "about2" => 1,
                        "about3" => 2,
                        _ => 0,
                    };
                }
                "update" | "update2" | "update3" => {
                    app.show_update_outcome(match part.as_str() {
                        "update2" => update::Outcome::UpToDate { latest: env!("CARGO_PKG_VERSION").into() },
                        "update3" => update::Outcome::Newer {
                            latest: "9.9.9".into(),
                            url: "https://github.com/CodeGator/GuitarCraft/releases".into(),
                            notes: "A sample of what a release's notes look like.".into(),
                        },
                        _ => update::Outcome::NoReleases,
                    });
                }
                "accidentals" => {
                    // The bass line with a sharp on its first note and a flat on its second.
                    app.track = 1;
                    app.bar = 0;
                    app.beat = 0;
                    app.set_accidental(1);
                    app.beat = 1;
                    app.set_accidental(-1);
                    app.beat = 0;
                }
                "batch" => app.open_batch(),
                "tabs" | "tabshome" | "tabschart" | "quitprompt" => {
                    // Several files open at once: the demo song, a chord chart and a new song.
                    app.begin_new_doc();
                    app.song = Song::demo();
                    app.reset_after_load();
                    app.show_home = false;
                    app.new_chart();
                    if let Some(c) = app.chart.as_mut() {
                        c.doc = crate::chart::ChordChart::sample();
                    }
                    app.request(Pending::Template(0));
                    app.song.title = "Waltz in Three".into();
                    // Show the first tab, or the chart, or the start screen.
                    match part.as_str() {
                        "tabschart" => app.switch_to(1),
                        "tabshome" => app.switch_to(2),
                        _ => app.switch_to(0),
                    }
                    if part == "tabshome" {
                        app.show_home = true;
                    }
                    if part == "quitprompt" {
                        // Closing the window with two files that have unsaved changes.
                        app.dirty = true;
                        if let Some(Some(d)) = app.tabs.get_mut(2) {
                            d.dirty = true;
                        }
                        app.pending = Some(Pending::Quit);
                    }
                }
                "importmenu" | "exportmenu" => app.file_menu_demo = Some(part == "exportmenu"),
                "chartmoved" => {
                    let mut st = chartui::ChartState::new(crate::chart::ChordChart::sample());
                    // Verse 2 one place earlier, as with the Sections list's up button.
                    st.sel = st.doc.move_section(2, -1);
                    app.chart = Some(st);
                    app.show_home = false;
                    app.status =
                        "Verse 2 moved before Verse 1: the map and the chart follow.".into();
                }
                "transpose" => {
                    app.chart = Some(chartui::ChartState::new(crate::chart::ChordChart::sample()));
                    app.show_home = false;
                    app.open_transpose(None);
                    app.status = "Transpose: by half steps or to a key; the preview shows each chord's new name.".into();
                }
                "chartstyle" | "chartstyle2" | "chartstyle3" | "chartstage" => {
                    let mut st = chartui::ChartState::new(crate::chart::ChordChart::sample());
                    if part == "chartstage" {
                        st.doc.look = crate::chart::ChartLook::stage();
                    }
                    app.chart = Some(st);
                    app.show_home = false;
                    match part.as_str() {
                        "chartstyle" => app.open_chart_style(0),
                        "chartstyle2" => app.open_chart_style(2),
                        "chartstyle3" => app.open_chart_style(3),
                        _ => {}
                    }
                    app.status =
                        "The chart's stylesheet sets its fonts, colours and columns (F7).".into();
                }
                "chart" => {
                    app.chart = Some(chartui::ChartState::new(crate::chart::ChordChart::sample()));
                    app.show_home = false;
                    app.status =
                        "Click a section on the page to edit it; rearrange the song in the map."
                            .into();
                }
                "barhover" => app.bar_hover_demo = Some(2),
                // Playback part way through bar 9 (no sound), for checking that the view follows.
                "playlate" => {
                    app.playing = true;
                    app.play_beat = Some((8, 1));
                    app.play_tick = Some((8, 0.6 * crate::model::QUARTER as f32));
                }
                // A muted and a soloed track, for checking the header buttons.
                "mutesolo" => {
                    app.song.tracks[0].mute = true;
                    if let Some(t) = app.song.tracks.get_mut(1) {
                        t.solo = true;
                    }
                }
                "specialpaste" => {
                    app.copy();
                    app.open_special_paste();
                }
                "exportpng" => app.open_export(printing::ExportKind::Png),
                "arpup" => app.open_popup(popups::Popup::ArpeggioUp),
                "arpdown" => app.open_popup(popups::Popup::ArpeggioDown),
                // A sample of the palette's newer marks on the demo, for checking pictures.
                "newmarks" => {
                    let t = &mut app.song.tracks[0];
                    t.bars[0].beats[0].fx.timer = true;
                    t.bars[0].beats[1].fx.wah = 1;
                    t.bars[0].beats[2]
                        .notes
                        .iter_mut()
                        .for_each(|n| n.fx.ornament = 1);
                    t.bars[0].beats[3].fx.stem = 2;
                    t.bars[0].beats[4].fx.barre = 5;
                    t.bars[0].beats[5].fx.slash = true;
                    t.bars[0].beats[6].fx.pedal = 1;
                    t.bars[0].beats[6]
                        .notes
                        .iter_mut()
                        .for_each(|n| n.fx.lh_finger = 3);
                    t.bars[0].beats[7].notes.iter_mut().for_each(|n| {
                        n.fx.string_number = true;
                        n.fx.legato = true;
                    });
                    t.bars[1].simile = 1;
                    app.song.tracks[1].bars[1].simile = 1;
                    app.song.master[2].free_time = true;
                }
                "scales" => app.open_chord_scales(chordscales::Mode::Scales),
                "arpeggios" => {
                    app.open_chord_scales(chordscales::Mode::Arpeggios);
                    app.cs.root = 2;
                    app.cs.arp = 1;
                }
                "chords" | "chordscales" => {
                    app.open_chord_scales(chordscales::Mode::Chords);
                    app.cs.root = 4;
                    app.cs.chord = 18;
                    app.cs.label = 2;
                }
                "identify" => {
                    app.open_chord_scales(chordscales::Mode::Chords);
                    app.cs.identify = true;
                    app.cs.picks = [None, None, Some(4), Some(4), Some(2), None];
                }
                "scalesrelated" => {
                    app.open_chord_scales(chordscales::Mode::Scales);
                    app.cs.root = 2;
                    app.cs.scale = 0;
                    app.cs.related = true;
                }
                "lyrics" => app.lyrics_open = true,
                "addtrack" => app.open_add_track(false),
                "newdoc" => app.open_add_track(true),
                "std" => {
                    // Cursor on the lowest note of the beat, in the notation.
                    if let Some(n) = app
                        .cur_beat()
                        .notes
                        .iter()
                        .max_by_key(|n| n.string)
                        .cloned()
                    {
                        app.string = n.string as usize;
                    }
                    app.enter_std();
                }
                "tab1" => app.string = 1,
                "voicetool" => app.voices_open = true,
                "tempo" => {
                    app.song.master[4].tempo = Some(112);
                    app.song.master[8].tempo_ramp = false;
                    app.song.master[4].tempo_ramp = true;
                    app.song.master[8].tempo = Some(84);
                    app.open_automations(4);
                }
                "move2" => {
                    // Move the cursor's note (bar 1, low string) to voice 2, for checking.
                    app.string = 5;
                    app.move_to_voice(1);
                }
                "voice2" => {
                    // A second voice in bar 1 (bass in half notes) for checking pictures.
                    let b = &mut app.song.tracks[0].bars[0];
                    b.pad_voices();
                    b.voices[0] = vec![
                        Beat {
                            notes: vec![Note::new(5, 3)],
                            ..Beat::rest(Dur::Half)
                        },
                        Beat {
                            notes: vec![Note::new(4, 2)],
                            ..Beat::rest(Dur::Half)
                        },
                    ];
                    app.voice = 1;
                    app.string = 5;
                }
                "help" => app.show_help = true,
                // The Stylesheet window on one of its tabs, for checking pictures.
                "stylesheet" | "stylesheet1" | "stylesheet2" | "stylesheet3" | "stylesheet4"
                | "stylesheet5" => {
                    app.open_stylesheet();
                    if let Some(d) = app.style_dlg.as_mut() {
                        d.tab = part
                            .trim()
                            .trim_start_matches("stylesheet")
                            .parse::<usize>()
                            .unwrap_or(1)
                            .clamp(1, 5)
                            - 1;
                    }
                }
                "style-rock" => app.song.style = crate::style::Stylesheet::rock(),
                "style-classic" => app.song.style = crate::style::Stylesheet::classic(),
                "style-jazz" => app.song.style = crate::style::Stylesheet::jazz(),
                _ => {}
            }
        }
        if home {
            app.show_home = true;
        }
        if let Some(z) = zoom {
            app.zoom = z.clamp(0.3, 3.0);
        }
        if let Some(t) = track {
            app.track = t.saturating_sub(1);
            app.clamp_cursor();
        }
        app.total_secs = app.song_seconds();
        if own_settings && no_file_asked {
            app.prefs_at_start(ctx);
        }
        app
    }

    // ----- helpers -----

    fn tr(&self) -> &Track {
        &self.song.tracks[self.track]
    }
    fn cur_beat(&self) -> &Beat {
        &self.song.tracks[self.track].bars[self.bar].voice(self.voice)[self.beat]
    }
    fn clamp_cursor(&mut self) {
        self.track = self.track.min(self.song.tracks.len() - 1);
        self.bar = self.bar.min(self.song.bar_count() - 1);
        let nb = self.song.tracks[self.track].bars[self.bar]
            .voice(self.voice)
            .len();
        self.beat = self.beat.min(nb.saturating_sub(1));
        self.string = self.string.min(self.tr().strings() - 1);
    }
    fn selection(&self) -> Option<((usize, usize), (usize, usize))> {
        let a = self.sel_anchor?;
        let c = (self.bar, self.beat);
        if a == c {
            return None;
        }
        Some((a.min(c), a.max(c)))
    }

    fn snapshot(&mut self) {
        self.undo.push(self.song.clone());
        if self.undo.len() > 200 {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.dirty = true;
    }

    fn edit(&mut self, f: impl FnOnce(&mut Song, usize, usize, usize, usize)) {
        self.snapshot();
        let (t, b, j, s) = (self.track, self.bar, self.beat, self.string);
        // Editing voice 2-4: put that voice where voice 1 normally is for the change.
        self.song.swap_in(t, self.voice);
        f(&mut self.song, t, b, j, s);
        self.song.swap_out_all();
        self.song.normalize();
        self.clamp_cursor();
        self.total_secs = self.song_seconds();
    }

    fn do_undo(&mut self) {
        if let Some(s) = self.undo.pop() {
            self.redo.push(std::mem::replace(&mut self.song, s));
            self.song.normalize();
            self.clamp_cursor();
            self.dirty = true;
            self.status = "Undone.".into();
        }
    }
    fn do_redo(&mut self) {
        if let Some(s) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.song, s));
            self.song.normalize();
            self.clamp_cursor();
            self.dirty = true;
            self.status = "Redone.".into();
        }
    }

    fn song_seconds(&self) -> f32 {
        let mut t = 0.0;
        for bi in self.song.play_order() {
            t += self.song.master[bi].ticks() as f32 / QUARTER as f32 * 60.0
                / self.song.tempo_at(bi) as f32;
        }
        t / (self.speed / 100.0)
    }

    fn preview(&mut self, n: &Note) {
        if self.playing || !self.sound_allowed || !self.play_while_editing {
            return;
        }
        // A MIDI output, when one is chosen, hears the note as well.
        if self.midi.output_open() {
            let t = &self.song.tracks[self.track];
            let drums = t.kind() == crate::model::InstrumentType::Drums;
            self.midi.preview_note(
                crate::midi::channel_for(self.track, drums),
                t.instrument.midi_program(),
                t.sound(n),
                (0.8 * t.volume.max(0.3) * 127.0) as u8,
                600,
            );
        }
        self.ensure_audio();
        if self.mute_builtin() {
            return;
        }
        if let Some(a) = &self.audio
            && a.error.is_none()
        {
            let sr = a.sample_rate();
            let ev = audio::note_preview(&self.song.tracks[self.track], self.track, n, sr);
            if let Ok(mut st) = a.state.lock() {
                st.preview.push(ev);
            }
        }
    }

    fn ensure_audio(&mut self) {
        if self.audio.is_none() {
            self.audio = Some(if self.sound_allowed {
                Audio::with_output(self.prefs.output_device.as_deref(), self.prefs.buffer)
            } else {
                Audio::silent()
            });
            if let Some(e) = self.audio.as_ref().and_then(|a| a.error.clone()) {
                self.status = notice::failed(format!("Sound unavailable: {e}"));
            }
            // A new player starts with a plain mix: send the equalizers and Focus again.
            self.mix_sent = (vec![], vec![]);
            self.sync_track_mix();
        }
    }

    // ----- cursor movement -----

    fn bar_full(&self, bar: usize) -> bool {
        let b = &self.song.tracks[self.track].bars[bar];
        let v = b.voice(self.voice);
        v.iter().map(|x| x.ticks()).sum::<u32>() >= self.song.master[bar].ticks()
            || (v.len() == 1 && v[0].is_rest() && v[0].dur == Dur::Whole)
    }

    fn move_right(&mut self, extend: bool) {
        self.update_anchor(extend);
        let nb = self.song.tracks[self.track].bars[self.bar]
            .voice(self.voice)
            .len();
        if self.beat + 1 < nb {
            self.beat += 1;
        } else if !extend && !self.bar_full(self.bar) {
            let d = self.cur_beat().dur;
            let dotted = self.cur_beat().dotted;
            let tup = self.cur_beat().tuplet;
            self.edit(|s, t, b, _, _| {
                let mut nbt = Beat::rest(d);
                nbt.dotted = dotted;
                nbt.tuplet = tup;
                s.tracks[t].bars[b].beats.push(nbt)
            });
            self.beat = nb;
        } else if self.bar + 1 < self.song.bar_count() {
            self.bar += 1;
            self.beat = 0;
        } else if !extend {
            self.edit(|s, _, _, _, _| {
                let mut mb = s.master.last().cloned().unwrap_or_default();
                mb.marker = None;
                mb.repeat_open = false;
                mb.repeat_close = 0;
                mb.tempo = None;
                s.master.push(mb);
            });
            self.bar += 1;
            self.beat = 0;
            self.status = "Added a new bar at the end.".into();
        }
        self.scroll_to_cursor = true;
    }

    fn move_left(&mut self, extend: bool) {
        self.update_anchor(extend);
        if self.beat > 0 {
            self.beat -= 1;
        } else if self.bar > 0 {
            self.bar -= 1;
            self.beat = self.song.tracks[self.track].bars[self.bar]
                .voice(self.voice)
                .len()
                - 1;
        }
        self.scroll_to_cursor = true;
    }

    fn update_anchor(&mut self, extend: bool) {
        if extend {
            if self.sel_anchor.is_none() {
                self.sel_anchor = Some((self.bar, self.beat));
            }
        } else {
            self.sel_anchor = None;
        }
    }

    /// Chooses the voice to edit (0 = voice 1).
    fn set_voice(&mut self, v: usize) {
        self.voice = v.min(VOICES - 1);
        self.sel_anchor = None;
        self.clamp_cursor();
        self.status = format!("Editing voice {}.", self.voice + 1);
    }

    fn goto_bar(&mut self, bar: usize) {
        self.bar = bar.min(self.song.bar_count() - 1);
        self.beat = 0;
        self.sel_anchor = None;
        self.scroll_to_cursor = true;
    }

    // ----- editing -----

    fn set_fret(&mut self, string: usize, fret: u8) {
        let cur_dur = self.cur_dur;
        self.edit(|s, t, b, j, _| {
            let bar = &mut s.tracks[t].bars[b];
            let placeholder =
                bar.beats.len() == 1 && bar.beats[0].is_rest() && bar.beats[0].dur == Dur::Whole;
            let beat = &mut bar.beats[j];
            if placeholder {
                beat.dur = cur_dur;
            }
            if let Some(n) = beat.note_on_mut(string as u8) {
                n.fret = fret;
                n.tie = false;
                n.dead = false;
            } else {
                beat.notes.push(Note::new(string as u8, fret));
                beat.notes.sort_by_key(|n| n.string);
            }
        });
        let n = Note::new(string as u8, fret);
        if self.std_cursor.is_some()
            && let Some(placed) = self.cur_beat().note_on(string as u8).cloned()
        {
            self.std_cursor = Some(self.staff_pos_of(&placed));
        }
        self.preview(&n);
        let pitch = self.tr().pitch(&n);
        self.status = if self.on_drum_track() {
            format!("{} (MIDI {pitch})", drum_name(pitch))
        } else {
            format!("String {}, fret {} ({})", string + 1, fret, note_name_oct(pitch))
        };
    }

    fn type_digit(&mut self, d: u8, now: f64) {
        let pos = (self.bar, self.beat, self.string);
        let max = self.tr().frets;
        let mut fret = d;
        if let Some((prev, t, b, j, s)) = self.last_digit
            && (b, j, s) == pos
            && now - t < 0.8
        {
            let combined = prev as u32 * 10 + d as u32;
            if combined <= max as u32 {
                fret = combined as u8;
                // Replace the one-digit entry instead of adding an undo step.
                self.undo.pop();
            }
        }
        self.set_fret(self.string, fret);
        self.last_digit = Some((fret, now, pos.0, pos.1, pos.2));
    }

    fn delete_at_cursor(&mut self) {
        if let Some((a, b)) = self.selection() {
            self.edit(|s, t, _, _, _| {
                for bi in (a.0..=b.0).rev() {
                    let beats = &mut s.tracks[t].bars[bi].beats;
                    let from = if bi == a.0 { a.1 } else { 0 };
                    let to = if bi == b.0 { b.1 } else { beats.len() - 1 };
                    for j in (from..=to.min(beats.len() - 1)).rev() {
                        beats.remove(j);
                    }
                    if beats.is_empty() {
                        beats.push(Beat::rest(Dur::Whole));
                    }
                }
            });
            self.bar = a.0;
            self.beat = a.1;
            self.sel_anchor = None;
            self.clamp_cursor();
            return;
        }
        let has_note = self.cur_beat().note_on(self.string as u8).is_some();
        if has_note {
            self.edit(|s, t, b, j, st| {
                s.tracks[t].bars[b].beats[j]
                    .notes
                    .retain(|n| n.string != st as u8)
            });
        } else if self.cur_beat().is_rest()
            && self.song.tracks[self.track].bars[self.bar]
                .voice(self.voice)
                .len()
                > 1
        {
            self.edit(|s, t, b, j, _| {
                s.tracks[t].bars[b].beats.remove(j);
            });
        } else if !self.cur_beat().is_rest() {
            self.edit(|s, t, b, j, _| s.tracks[t].bars[b].beats[j].notes.clear());
        }
    }

    fn insert_beat(&mut self) {
        let d = self.cur_dur;
        self.edit(|s, t, b, j, _| s.tracks[t].bars[b].beats.insert(j, Beat::rest(d)));
    }

    fn set_duration(&mut self, d: Dur) {
        self.cur_dur = d;
        self.edit(|s, t, b, j, _| s.tracks[t].bars[b].beats[j].dur = d);
        self.status = format!("{} selected.", d.name());
    }

    fn toggle_note_flag(&mut self, name: &str, f: fn(&mut Note) -> &mut bool) {
        // With a selection, the effect goes on (or off) for every selected note.
        if let Some((sa, sb)) = self.selection() {
            let mut all_on = true;
            let mut any = false;
            for bi in sa.0..=sb.0 {
                let beats = self.song.tracks[self.track].bars[bi].voice(self.voice);
                let from = if bi == sa.0 { sa.1 } else { 0 };
                let to = if bi == sb.0 {
                    sb.1
                } else {
                    beats.len().saturating_sub(1)
                };
                for beat in beats.iter().take(to + 1).skip(from) {
                    for n in &beat.notes {
                        any = true;
                        let mut c = n.clone();
                        all_on &= *f(&mut c);
                    }
                }
            }
            if !any {
                self.refuse("The selection has no notes.");
                return;
            }
            let on = !all_on;
            self.edit(|s, t, _, _, _| {
                for bi in sa.0..=sb.0 {
                    let beats = &mut s.tracks[t].bars[bi].beats;
                    let from = if bi == sa.0 { sa.1 } else { 0 };
                    let to = if bi == sb.0 {
                        sb.1
                    } else {
                        beats.len().saturating_sub(1)
                    };
                    for beat in beats.iter_mut().take(to + 1).skip(from) {
                        for n in &mut beat.notes {
                            *f(n) = on;
                        }
                    }
                }
            });
            self.status = format!(
                "{name} {} for the selected notes.",
                if on { "on" } else { "off" }
            );
            return;
        }
        if self.cur_beat().is_rest() {
            self.refuse("Put a note on this beat first.");
            return;
        }
        let on_string = self.cur_beat().note_on(self.string as u8).is_some()
            && (self.std_cursor.is_none() || self.note_at_std().is_some());
        self.edit(|s, t, b, j, st| {
            let beat = &mut s.tracks[t].bars[b].beats[j];
            if on_string {
                if let Some(n) = beat.note_on_mut(st as u8) {
                    let v = f(n);
                    *v = !*v;
                }
            } else {
                let newv = !beat.notes.iter_mut().all(|n| *f(n));
                for n in &mut beat.notes {
                    *f(n) = newv;
                }
            }
        });
        let on = self
            .cur_beat()
            .note_on(self.string as u8)
            .or(self.cur_beat().notes.first())
            .map(|n| {
                let mut c = n.clone();
                *f(&mut c)
            })
            .unwrap_or(false);
        self.status = format!("{name}: {}.", if on { "on" } else { "off" });
    }

    fn cycle_bend(&mut self) {
        if self.cur_beat().is_rest() {
            self.refuse("Put a note on this beat first.");
            return;
        }
        self.edit(|s, t, b, j, st| {
            let beat = &mut s.tracks[t].bars[b].beats[j];
            let idx = beat
                .notes
                .iter()
                .position(|n| n.string == st as u8)
                .unwrap_or(0);
            let n = &mut beat.notes[idx];
            n.bend = match n.bend {
                0 => 4,
                4 => 2,
                2 => 6,
                6 => 8,
                _ => 0,
            };
        });
    }

    /// Bar > Multirest (Ctrl+R): the current track shows its runs of empty bars as one bar with
    /// their number, or goes back to showing them one by one.
    fn toggle_multirest(&mut self) {
        let ti = self.track;
        let on = !self.song.tracks[ti].multirest;
        self.edit(move |s, _, _, _, _| s.tracks[ti].multirest = on);
        self.status = if on {
            "Multirest on: empty bars in a row show as one (Ctrl+R to turn it off). It shows when this track is on its own.".into()
        } else {
            "Multirest off.".into()
        };
    }

    fn insert_bar(&mut self, at: usize) {
        self.edit(|s, _, _, _, _| {
            let mut mb = s.master[at.min(s.master.len() - 1)].clone();
            mb.marker = None;
            mb.repeat_open = false;
            mb.repeat_close = 0;
            mb.tempo = None;
            s.master.insert(at, mb);
            for t in &mut s.tracks {
                t.bars.insert(at, Bar::empty());
            }
        });
        self.goto_bar(at);
        self.status = format!("Inserted bar {}.", at + 1);
    }

    fn delete_bar(&mut self, at: usize) {
        if self.song.bar_count() <= 1 {
            return;
        }
        self.edit(|s, _, _, _, _| {
            s.master.remove(at);
            for t in &mut s.tracks {
                t.bars.remove(at);
            }
        });
        self.goto_bar(at.min(self.song.bar_count() - 1));
        self.status = format!("Deleted bar {}.", at + 1);
    }

    fn copy(&mut self) {
        let mut out = vec![];
        if let Some((a, b)) = self.selection() {
            for bi in a.0..=b.0 {
                let beats = &self.song.tracks[self.track].bars[bi].voice(self.voice);
                let from = if bi == a.0 { a.1 } else { 0 };
                let to = if bi == b.0 { b.1 } else { beats.len() - 1 };
                out.extend_from_slice(&beats[from..=to.min(beats.len() - 1)]);
            }
        } else {
            out.push(self.cur_beat().clone());
        }
        let n = out.len();
        self.strip_copied_diagrams(&mut out);
        self.clipboard = out;
        self.status = format!("Copied {n} beat(s).");
    }

    fn paste(&mut self) {
        if self.clipboard.is_empty() {
            return;
        }
        let clip = self.clipboard.clone();
        let n = clip.len();
        self.edit(|s, t, b, j, _| {
            let bar = &mut s.tracks[t].bars[b];
            let placeholder =
                bar.beats.len() == 1 && bar.beats[0].is_rest() && bar.beats[0].dur == Dur::Whole;
            if placeholder {
                bar.beats.clear();
                bar.beats.extend(clip);
            } else {
                for (k, bt) in clip.into_iter().enumerate() {
                    bar.beats.insert(j + k, bt);
                }
            }
        });
        self.status = format!("Pasted {n} beat(s).");
    }

    fn transpose(&mut self, delta: i32) {
        let sel = self.selection();
        let (a, b) = sel.unwrap_or(((self.bar, self.beat), (self.bar, self.beat)));
        let maxf = self.tr().frets as i32;
        self.edit(|s, t, _, _, _| {
            for bi in a.0..=b.0 {
                let beats = &mut s.tracks[t].bars[bi].beats;
                let from = if bi == a.0 { a.1 } else { 0 };
                let to = if bi == b.0 { b.1 } else { beats.len() - 1 };
                let to = to.min(beats.len() - 1);
                for beat in &mut beats[from..=to] {
                    for n in &mut beat.notes {
                        n.fret = (n.fret as i32 + delta).clamp(0, maxf) as u8;
                    }
                }
            }
        });
        self.status = format!(
            "Moved notes {} a half step.",
            if delta > 0 { "up" } else { "down" }
        );
    }

    fn remove_track(&mut self) {
        if self.song.tracks.len() <= 1 {
            self.refuse("A song needs at least one track.");
            return;
        }
        let ti = self.track;
        self.edit(|s, _, _, _, _| {
            s.tracks.remove(ti);
        });
        self.track = self.track.min(self.song.tracks.len() - 1);
        self.clamp_cursor();
    }

    // ----- files -----

    fn new_song(&mut self) {
        // Preferences > General can start new songs from a template.
        let k = self.prefs.default_template;
        if k > 0 && k < home::TEMPLATES.len() {
            self.new_from_template(k);
            return;
        }
        self.begin_new_doc();
        self.song = Song::new_blank();
        self.song.style = stylesheet::default_style();
        self.prefs.fill_song(&mut self.song);
        self.path = None;
        self.reset_after_load();
        self.show_home = false;
        self.status = "New song. Click a string and type fret numbers.".into();
        // The first thing to do with an empty document is to add a track.
        self.open_add_track(true);
    }

    /// A new song from one of the standard templates.
    fn new_from_template(&mut self, k: usize) {
        self.begin_new_doc();
        self.song = home::template(k);
        self.prefs.fill_song(&mut self.song);
        self.path = None;
        self.reset_after_load();
        self.show_home = false;
    }

    fn reset_after_load(&mut self) {
        // A score is open now, not a chord chart.
        self.chart = None;
        self.dirty = false;
        self.undo.clear();
        self.redo.clear();
        // Start on the first track the score shows (hidden tracks stay out of view).
        self.track = self.song.tracks.iter().position(|t| t.visible).unwrap_or(0);
        self.bar = 0;
        self.beat = 0;
        self.string = 0;
        self.sel_anchor = None;
        self.total_secs = self.song_seconds();
        self.inspector = InspectorTab::Song;
        self.apply_forced_view();
    }

    /// The folder file dialogs start in: the open song's folder, otherwise the
    /// folder of the file opened or saved last.
    fn dialog_dir(&self) -> Option<PathBuf> {
        self.path
            .iter()
            .chain(self.save_hint.iter())
            .chain(self.recent.iter())
            .filter_map(|p| full_path(p).parent().map(|d| d.to_path_buf()))
            .find(|d| d.is_dir())
    }

    /// A file dialog starting in `dialog_dir`.
    fn file_dialog(&self) -> rfd::FileDialog {
        let d = rfd::FileDialog::new();
        match self.dialog_dir() {
            Some(dir) => d.set_directory(dir),
            None => d,
        }
    }

    fn open_dialog(&mut self) {
        if self.screenshot.is_some() {
            return;
        }
        let f = self
            .file_dialog()
            .add_filter(
                "Guitar Pro files and chord charts",
                &[
                    "gp",
                    "gpx",
                    "gp3",
                    "gp4",
                    "gp5",
                    crate::chart::EXT,
                ],
            )
            .add_filter("Guitar Pro files", &["gp", "gpx", "gp3", "gp4", "gp5"])
            .add_filter(
                "GuitarCraft chord charts",
                &[crate::chart::EXT],
            )
            .pick_file();
        if let Some(p) = f {
            self.open_path(p);
        }
    }

    fn open_path(&mut self, p: PathBuf) {
        // Kept as a full location, so later dialogs can start in its folder.
        let p = full_path(&p);
        if let Some(i) = self.find_open(&p) {
            self.switch_to(i);
            self.refuse("That file is already open.");
            return;
        }
        // Gone since the list was made (deleted or moved while GuitarCraft was open): say so, and
        // stop offering it.
        if !p.exists() {
            self.recent.retain(|x| full_path(x) != p);
            home::save_recent(&self.recent);
            self.status = notice::failed("That file is no longer there, so it was removed from the recent files.".to_string());
            return;
        }
        if crate::chart::is_chart_path(&p) {
            self.open_chart_path(p);
            return;
        }
        match fileio::load_song(&p) {
            Ok(song) => {
                self.begin_new_doc();
                self.song = song;
                // Files saved straight back: Guitar Pro 5 and this app's own. Other
                // Guitar Pro files are saved next to the original as .gp5.
                let ext = p
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_ascii_lowercase();
                let native = [fileio::GP_EXT, fileio::GP5_EXT].contains(&ext.as_str());
                self.path = if native { Some(p.clone()) } else { None };
                self.save_hint = if native {
                    None
                } else {
                    Some(p.with_extension(fileio::GP_EXT))
                };
                self.reset_after_load();
                self.add_recent(&p);
                self.show_home = false;
                let name = p
                    .file_name()
                    .map(|f| f.to_string_lossy().to_string())
                    .unwrap_or_default();
                self.status = format!("Opened {name}.");
            }
            Err(e) => self.status = notice::failed(format!("Couldn't open the file: {e}")),
        }
    }

    fn save(&mut self, save_as: bool) {
        // A chord chart is only ever saved as a chord chart (Guitar Pro has no
        // place for one).
        if self.chart.is_some() {
            self.save_chart(save_as);
            return;
        }
        if self.screenshot.is_some() {
            return;
        }
        let path = if save_as || self.path.is_none() {
            // Starts in the song's own folder, or the folder used last.
            let mut d = self
                .file_dialog()
                .add_filter("Guitar Pro file", &[fileio::GP_EXT])
                .add_filter("Guitar Pro 5 file (older)", &[fileio::GP5_EXT]);
            match (&self.save_hint, &self.path) {
                (_, Some(p)) | (Some(p), None) => {
                    let stem = p
                        .file_stem()
                        .map(|x| x.to_string_lossy().to_string())
                        .unwrap_or_else(|| sanitize(&self.song.title));
                    d = d.set_file_name(format!("{stem}.{}", fileio::GP_EXT));
                }
                _ => {
                    d = d.set_file_name(format!(
                        "{}.{}",
                        sanitize(&self.song.title),
                        fileio::GP_EXT
                    ))
                }
            }
            d.save_file()
        } else {
            self.path.clone()
        };
        if let Some(p) = path {
            match fileio::save_song(&self.song, &p) {
                Ok(()) => {
                    self.dirty = false;
                    let gp = p
                        .extension()
                        .map(|e| e.eq_ignore_ascii_case(fileio::GP5_EXT))
                        .unwrap_or(false);
                    let lost = if gp {
                        fileio::gp5_losses(&self.song)
                    } else {
                        vec![]
                    };
                    self.status = format!("Saved {}.", p.display());
                    if !lost.is_empty() {
                        self.refuse(format!("Saved {}, but Guitar Pro 5 can't keep: {}.", p.display(), lost.join(", ")));
                    }
                    self.save_hint = None;
                    let p = full_path(&p);
                    self.add_recent(&p);
                    self.path = Some(p);
                }
                Err(e) => self.status = notice::failed(format!("Couldn't save: {e}")),
            }
        }
    }

    /// Asks where to export, suggesting the song's title with this extension.
    /// Never asks while taking a screenshot.
    fn export_path(&self, kind: &str, ext: &str) -> Option<PathBuf> {
        if self.screenshot.is_some() {
            return None;
        }
        self.file_dialog()
            .add_filter(kind, &[ext])
            .set_file_name(format!("{}.{ext}", sanitize(&self.song.title)))
            .save_file()
    }

    /// Writes an exported file and says how it went.
    fn write_export(&mut self, p: &std::path::Path, data: impl AsRef<[u8]>) {
        self.status = match std::fs::write(p, data) {
            Ok(()) => format!("Exported {}.", p.display()),
            Err(e) => notice::failed(format!("Couldn't export: {e}")),
        };
    }

    fn export_midi(&mut self) {
        if let Some(p) = self.export_path("MIDI file", "mid") {
            self.write_export(&p, fileio::export_midi(&self.song));
        }
    }

    fn export_ascii(&mut self) {
        if let Some(p) = self.export_path("Text file", "txt") {
            self.write_export(&p, fileio::export_ascii(&self.song, self.track));
        }
    }

    /// File > Export > GPX: a Guitar Pro 6 file.
    fn export_gpx(&mut self) {
        if let Some(p) = self.export_path("Guitar Pro 6 file", "gpx") {
            self.write_export(&p, crate::gpx::export_gpx(&self.song));
        }
    }

    /// File > Export > GP5: a Guitar Pro 5 file, saying what that older format can't keep.
    fn export_gp5(&mut self) {
        if let Some(p) = self.export_path("Guitar Pro 5 file", "gp5") {
            self.write_export(&p, crate::gp5write::export_gp5(&self.song));
            let lost = fileio::gp5_losses(&self.song);
            if !lost.is_empty() && self.status.starts_with("Exported") {
                self.refuse(format!("{} Guitar Pro 5 can't keep: {}.", self.status, lost.join(", ")));
            }
        }
    }

    /// File > Export > MusicXML.
    fn export_musicxml(&mut self) {
        if let Some(p) = self.export_path("MusicXML file", "musicxml") {
            self.write_export(&p, crate::musicxml::export_musicxml(&self.song));
        }
    }

    fn add_recent(&mut self, p: &std::path::Path) {
        if self.screenshot.is_some() {
            return;
        }
        let p = p.to_path_buf();
        self.recent.retain(|x| x != &p);
        self.recent.insert(0, p);
        self.recent.truncate(20);
        home::save_recent(&self.recent);
    }

    /// Tracks shown in the score.
    fn shown_tracks(&self) -> Vec<usize> {
        if self.single_track {
            return vec![self.track];
        }
        (0..self.song.tracks.len())
            .filter(|&i| self.song.tracks[i].visible || i == self.track)
            .collect()
    }

    fn request(&mut self, what: Pending) {
        // Opening and creating add a tab, so only closing a tab or quitting can lose changes.
        let ask = match what {
            Pending::Close => self.dirty,
            Pending::Quit | Pending::CloseAll => self.any_dirty(),
            _ => false,
        };
        if ask {
            self.pending = Some(what);
        } else {
            self.run_pending(what);
        }
    }

    fn run_pending(&mut self, what: Pending) {
        match what {
            Pending::New => self.new_song(),
            Pending::Open => self.open_dialog(),
            Pending::Demo => {
                self.begin_new_doc();
                self.song = Song::demo();
                self.path = None;
                self.reset_after_load();
                self.show_home = false;
            }
            // The empty template is an empty document: it asks for its first track.
            Pending::Template(0) => self.new_song(),
            Pending::Template(k) => self.new_from_template(k),
            Pending::Close => self.close_song(),
            Pending::CloseAll => self.close_all(),
            Pending::NewChart => self.new_chart(),
            Pending::Quit => {
                self.dirty = false;
                self.pending = None;
                self.quitting = true;
            }
        }
    }

    // ----- playback -----

    fn toggle_play(&mut self) {
        if self.playing {
            self.pause();
        } else {
            if self.trainer_on {
                self.speed = self.trainer_start;
                self.looping = true;
            }
            self.play();
        }
    }

    fn play(&mut self) {
        self.ensure_audio();
        let Some(a) = &self.audio else { return };
        if let Some(e) = &a.error {
            self.status = notice::failed(format!("Sound unavailable: {e}"));
            return;
        }
        let sr = a.sample_rate();
        let sel = if self.looping { self.selection() } else { None };
        // Looping without a selection repeats the whole song, starting from the cursor's bar.
        let whole_loop = self.looping && sel.is_none();
        let opt = ScheduleOptions {
            from_bar: if whole_loop { 0 } else { self.bar },
            metronome: self.metronome,
            count_in: self.count_in && !(whole_loop && self.bar > 0),
            speed: self.speed / 100.0,
            only_bars: sel.map(|(x, y)| (x.0, y.0)),
        };
        let mut sch = audio::build_schedule(&self.song, sr, &opt);
        let at_tick =
            |t: &audio::TimelineBar, tick: u32| t.start + (tick as f64 * t.samples_per_tick) as u64;
        let song_end = sch
            .timeline
            .last()
            .map(|t| at_tick(t, t.ticks))
            .unwrap_or(sch.end);
        let mut start_pos = 0u64;
        let loop_range = match (sel, self.looping) {
            (Some((a0, b0)), true) => {
                // Exactly the highlighted beats: from the first one's start to the last one's end.
                let bars = &self.song.tracks[self.track].bars;
                let start_tick = bars[a0.0].beat_start(self.voice, a0.1);
                let end_tick = bars[b0.0].beat_start(self.voice, b0.1)
                    + bars[b0.0]
                        .voice(self.voice)
                        .get(b0.1)
                        .map(|x| x.ticks())
                        .unwrap_or(0);
                let first = sch.timeline.first().cloned();
                let last = sch.timeline.last().cloned();
                match (first, last) {
                    (Some(f), Some(l)) => {
                        let ls = at_tick(&f, start_tick);
                        let le = at_tick(&l, end_tick.min(l.ticks)).max(ls + 1);
                        // Notes of the first bar before the selection aren't played.
                        let off = sch.start_offset;
                        sch.events
                            .retain(|e| e.start >= ls || e.start < off || e.track == usize::MAX);
                        if off == 0 {
                            start_pos = ls;
                        }
                        Some((ls, le))
                    }
                    _ => None,
                }
            }
            (None, true) => {
                let from = sch
                    .timeline
                    .iter()
                    .find(|t| t.bar == self.bar)
                    .map(|t| t.start)
                    .unwrap_or(sch.start_offset);
                if from > sch.start_offset {
                    start_pos = from;
                }
                Some((sch.start_offset, song_end))
            }
            _ => None,
        };
        // A chosen MIDI output plays along (and, unless Preferences says otherwise, instead).
        self.midi.setup_tracks(&self.song.tracks);
        let link = audio::MidiLink { tx: self.midi.sender(), mute_builtin: self.mute_builtin() };
        if let Ok(mut st) = a.state.lock() {
            st.midi = link;
            st.stop_all();
            st.next_ev = sch.events.partition_point(|e| e.start < start_pos);
            st.events = sch.events;
            st.pos = start_pos;
            st.end = sch.end;
            st.master = self.master_volume;
            st.loop_range = loop_range;
            st.playing = true;
        }
        self.timeline = sch.timeline;
        self.start_offset = sch.start_offset;
        self.play_start_bar = self.bar;
        self.playing = true;
        self.last_play_pos = 0;
        self.status = "Playing…".into();
    }

    /// Each track's gain from the Focus knob: the selected track against the others.
    fn focus_gains(&self) -> Vec<f32> {
        let f = self.focus.clamp(-1.0, 1.0);
        (0..self.song.tracks.len())
            .map(|k| match (k == self.track, f >= 0.0) {
                (true, true) => 1.0 + 0.3 * f,
                (true, false) => 1.0 + 0.85 * f,
                (false, true) => 1.0 - 0.85 * f,
                (false, false) => 1.0,
            })
            .collect()
    }

    /// Sends the tracks' equalizers and Focus gains to the player when they have changed
    /// (a knob turned, another track selected, another tab shown). Called every frame.
    pub(super) fn sync_track_mix(&mut self) {
        let eqs: Vec<_> = self.song.tracks.iter().map(|t| t.equalizer).collect();
        let gains = self.focus_gains();
        if self.mix_sent.0 == eqs && self.mix_sent.1 == gains {
            return;
        }
        if let Some(a) = &self.audio
            && let Ok(mut st) = a.state.lock()
        {
            st.set_track_mix(&eqs, &gains);
        }
        self.mix_sent = (eqs, gains);
    }

    fn pause(&mut self) {
        if let Some(a) = &self.audio
            && let Ok(mut st) = a.state.lock()
        {
            st.stop_all();
        }
        if let (Some((b, j)), None) = (self.play_beat, self.selection()) {
            self.bar = b;
            self.beat = j;
            self.clamp_cursor();
        }
        self.playing = false;
        self.play_beat = None;
        self.play_tick = None;
        self.status = "Paused.".into();
    }

    fn stop(&mut self) {
        if let Some(a) = &self.audio
            && let Ok(mut st) = a.state.lock()
        {
            st.stop_all();
        }
        if self.playing {
            self.bar = self.play_start_bar.min(self.song.bar_count() - 1);
            self.beat = 0;
        }
        self.playing = false;
        self.play_beat = None;
        self.play_tick = None;
        self.play_secs = 0.0;
    }

    fn poll_playback(&mut self) -> Vec<(usize, u8, u8)> {
        let mut sounding = vec![];
        if !self.playing {
            if let Some(a) = &self.audio
                && let Ok(st) = a.state.lock()
            {
                sounding = st.sounding.clone();
            }
            return sounding;
        }
        let Some(a) = &self.audio else {
            return sounding;
        };
        let (pos, still, sr) = match a.state.lock() {
            Ok(st) => {
                sounding = st.sounding.clone();
                (st.pos, st.playing, st.sample_rate)
            }
            Err(_) => return sounding,
        };
        if !still {
            self.playing = false;
            self.play_beat = None;
            self.play_tick = None;
            self.status = "Finished.".into();
            return sounding;
        }
        // The loop came round again: the speed trainer raises the speed.
        if pos < self.last_play_pos
            && self.looping
            && self.trainer_on
            && self.speed < self.trainer_end
        {
            self.speed = (self.speed + self.trainer_step).min(self.trainer_end);
            self.last_play_pos = 0;
            let keep = (self.bar, self.beat);
            if let Some((b, _)) = self.timeline.first().map(|t| (t.bar, 0)) {
                self.bar = b;
                self.beat = 0;
            }
            self.play();
            self.bar = keep.0;
            self.beat = keep.1;
            self.status = format!("Speed trainer: {}%", self.speed.round());
            return sounding;
        }
        self.last_play_pos = pos;
        self.play_secs = pos.saturating_sub(self.start_offset) as f32 / sr;
        let idx = self.timeline.partition_point(|t| t.start <= pos);
        if idx > 0 {
            let tb = &self.timeline[idx - 1];
            let exact = (pos - tb.start) as f64 / tb.samples_per_tick;
            self.play_tick = Some((tb.bar, exact as f32));
            let tick = exact as u32;
            let beat_len = QUARTER * 4 / self.song.master[tb.bar].den.max(1) as u32;
            self.metro_flash = tick % beat_len < beat_len / 5;
            let beats = &self.song.tracks[self.track].bars[tb.bar].voice(self.voice);
            let mut acc = 0;
            let mut j = 0;
            for (k, b) in beats.iter().enumerate() {
                if acc <= tick {
                    j = k;
                }
                acc += b.ticks();
            }
            let new = Some((tb.bar, j));
            if new != self.play_beat && self.follow {
                self.scroll_to_cursor = true;
            }
            self.play_beat = new;
        }
        sounding
    }

    // ----- input -----

    /// Shortcuts that work on the start screen too.
    fn handle_global_keys(&mut self, ctx: &egui::Context) {
        if self.pending.is_some() || self.prefs_dlg.is_some() {
            return;
        }
        let events = ctx.input(|i| i.events.clone());
        for ev in events {
            if let egui::Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } = ev
            {
                // Esc leaves the start screen; other keys come from the shortcut list.
                if key == Key::Escape && !modifiers.any() {
                    self.show_home = false;
                    continue;
                }
                let b = shortcuts::Binding::of(key, modifiers);
                match shortcuts::find(shortcuts::Place::Home, &b) {
                    Some(shortcuts::Act::New) => self.request(Pending::New),
                    Some(shortcuts::Act::NewChart) => self.request(Pending::NewChart),
                    Some(shortcuts::Act::Open) => self.request(Pending::Open),
                    Some(shortcuts::Act::Prefs) => self.open_prefs(),
                    Some(shortcuts::Act::Help) => self.show_help = true,
                    _ => {}
                }
            }
        }
    }

    fn handle_screenshot(&mut self, ctx: &egui::Context) {
        let Some(path) = self.screenshot.clone() else {
            return;
        };
        self.frame += 1;
        ctx.request_repaint();
        if self.frame >= 120 && !self.shot_requested {
            self.shot_requested = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
        let shot = ctx.input(|i| {
            i.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(img) = shot {
            let [w, h] = img.size;
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            match image::RgbaImage::from_raw(w as u32, h as u32, img.as_raw().to_vec()) {
                Some(buf) => {
                    if let Err(e) = buf.save(&path) {
                        eprintln!("Couldn't save the screenshot: {e}");
                    }
                }
                None => eprintln!("Screenshot had an unexpected size"),
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if self.frame > 600 {
            eprintln!("Screenshot timed out");
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

/// The full location of a file (a path typed or passed in short form is made
/// complete), without Windows' "\\?\" prefix that dialogs don't accept.
fn full_path(p: &std::path::Path) -> PathBuf {
    let full = std::fs::canonicalize(p).unwrap_or_else(|_| {
        std::env::current_dir()
            .map(|d| d.join(p))
            .unwrap_or_else(|_| p.to_path_buf())
    });
    let text = full.to_string_lossy();
    match text.strip_prefix(r"\\?\") {
        Some(rest) if !rest.starts_with("UNC") => PathBuf::from(rest),
        _ => full,
    }
}

#[cfg(test)]
mod path_tests {
    use super::full_path;

    #[test]
    fn short_paths_become_full_ones() {
        let p = full_path(std::path::Path::new("Cargo.toml"));
        assert!(p.is_absolute());
        assert!(p.parent().map(|d| d.is_dir()).unwrap_or(false));
        assert!(!p.to_string_lossy().starts_with(r"\\?\"));
    }
}
