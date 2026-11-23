//! File > Preferences (Ctrl+,): the person's settings, kept between runs, laid out in
//! the five tabs of Guitar Pro 8's Preferences window (General, Interface, Notation
//! Errors, Misc (was User Information), Audio & MIDI), plus AI and Shortcuts. As in the original, a change takes effect
//! straight away: there is no OK or Cancel. The settings are saved in
//! `preferences.json` in GuitarCraft's settings folder.
//!
//! Options GuitarCraft can't act on yet are shown greyed out with the reason, so the
//! window has the original's shape and it is clear what is missing.

use super::home::{TEMPLATES, app_dir};
use super::stylesheet::{BODY_BG, heading, row_w, switch, tab_row};
use super::App;
use super::shortcuts::{ACTIONS, Act, Binding, Group, Keymap, action, keymap, set_keymap};
use crate::audio::Audio;
use crate::model::Song;
use crate::score::PlayCursor;
use egui::{Align2, Color32, FontId, RichText, Ui, Vec2};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const TABS: [&str; 7] = ["General", "Interface", "Notation Errors", "Misc", "Audio & MIDI", "AI", "Shortcuts"];

/// How often to look for a newer version without being asked.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug, Serialize, Deserialize)]
pub enum UpdateCheck {
    /// Only Help > Check for Updates looks.
    #[default]
    Never,
    Daily,
    Weekly,
    Monthly,
}

impl UpdateCheck {
    const ALL: [(UpdateCheck, &'static str); 4] = [
        (UpdateCheck::Never, "Only when I ask"),
        (UpdateCheck::Daily, "Every day"),
        (UpdateCheck::Weekly, "Every week"),
        (UpdateCheck::Monthly, "Every month"),
    ];

    fn label(self) -> &'static str {
        Self::ALL.iter().find(|(v, _)| *v == self).map(|(_, l)| *l).unwrap_or("")
    }

    /// Seconds between two checks (None: never by itself).
    fn every(self) -> Option<u64> {
        const DAY: u64 = 24 * 60 * 60;
        match self {
            UpdateCheck::Never => None,
            UpdateCheck::Daily => Some(DAY),
            UpdateCheck::Weekly => Some(7 * DAY),
            UpdateCheck::Monthly => Some(30 * DAY),
        }
    }
}

const CURSORS: [(PlayCursor, &str, &str); 4] = [
    (PlayCursor::None, "None", "No line: only the notes being heard are highlighted"),
    (PlayCursor::Smooth, "Smooth", "The line glides from note to note"),
    (PlayCursor::EachNote, "Each Note", "The line jumps to each note as it is played"),
    (PlayCursor::Metronome, "Metronome", "The line jumps on each beat of the bar"),
];

/// Buffer sizes offered, in samples (None: the sound device chooses).
const BUFFERS: [Option<u32>; 6] = [None, Some(128), Some(256), Some(512), Some(1024), Some(2048)];

/// The page layouts, as in View (the order of `App::view_mode`).
pub const FORCED_STYLES: [&str; 4] = ["Standard", "Rock", "Classic", "Jazz"];
pub const FORCED_NOTATIONS: [&str; 3] = ["Standard notation and tablature", "Standard notation only", "Tablature only"];
pub const LAYOUTS: [&str; 5] = ["Page - Vertical", "Page - Horizontal", "Page - Grid", "Screen - Vertical", "Screen - Horizontal"];

impl Prefs {
    /// The checks that are on, as `model::ERR_*` bits.
    pub fn error_bits(&self) -> u8 {
        use crate::model::*;
        [(self.err_bar_duration, ERR_LENGTH), (self.err_out_of_range, ERR_RANGE), (self.err_tuplets, ERR_TUPLET), (self.err_hopo, ERR_HOPO), (self.err_unreachable, ERR_UNREACHABLE)]
            .iter()
            .filter(|c| c.0)
            .fold(0, |a, c| a | c.1)
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Prefs {
    // ----- General -----
    /// The template File > New starts from (an index into `TEMPLATES`; 0 is empty).
    pub default_template: usize,
    /// Open every song at this zoom, whatever it was.
    pub force_zoom: bool,
    pub zoom_pct: u32,
    /// Open every song in this page layout.
    pub force_layout: bool,
    /// Every song opened or created gets this stylesheet: 0 off, then `FORCED_STYLES`.
    pub force_style: u8,
    /// Every song opened or created shows this notation: 0 off, then `FORCED_NOTATIONS`.
    pub force_notation: u8,
    /// Copying beats or bars keeps their chord diagrams.
    pub copy_diagrams: bool,
    pub layout: u8,
    /// Show the score in two views one above the other (only in the vertical layouts).
    pub split_score: bool,
    /// Reopen the files that were open when GuitarCraft was last closed.
    pub reopen_last: bool,
    pub update_check: UpdateCheck,
    /// When GuitarCraft last looked for a newer version by itself (seconds since 1970).
    pub last_update_check: u64,
    // ----- Interface -----
    pub play_cursor: PlayCursor,
    pub highlight_played_bar: bool,
    /// An option of earlier versions: + and − the other way round. It is read once and
    /// turned into Shortcuts (Shorter note on −, Longer note on +), then never saved.
    #[serde(rename = "swap_plus_minus", skip_serializing)]
    legacy_swap: bool,
    // ----- Notation errors -----
    pub err_bar_duration: bool,
    pub err_out_of_range: bool,
    pub err_tuplets: bool,
    pub err_hopo: bool,
    pub err_unreachable: bool,
    // ----- User information (filled into every new song) -----
    pub artist: String,
    pub words: String,
    pub music: String,
    pub copyright: String,
    pub transcriber: String,
    pub instructions: String,
    // ----- Audio -----
    /// The sound output device by name (None: the system's default).
    pub output_device: Option<String>,
    /// The audio buffer in samples (None: the device's own).
    pub buffer: Option<u32>,
    /// The MIDI output port by name (None: no MIDI output).
    pub midi_output: Option<String>,
    /// The MIDI input port by name (None: no MIDI input).
    pub midi_input: Option<String>,
    /// The built-in sound keeps playing while a MIDI output is open.
    pub midi_builtin_too: bool,
    // ----- AI -----
    /// Ollama is switched on as an AI provider.
    pub ollama_on: bool,
    /// Where Ollama listens (empty: `ai::OLLAMA_URL`).
    pub ollama_url: String,
    /// The Ollama model to use (empty: none chosen yet).
    pub ollama_model: String,
    /// The context size asked of Ollama (`num_ctx`), in tokens: how much text the model reads
    /// at once. Larger sizes need more memory and run slower.
    pub ollama_context: u32,
    /// Load the Ollama model when the assistant opens and keep it in memory between questions,
    /// so the first answer doesn't wait for it to load.
    pub ollama_warm: bool,
    /// Claude is switched on as an AI provider.
    pub claude_on: bool,
    /// The Claude model to use.
    pub claude_model: String,
    // ----- Shortcuts -----
    /// The keys the person changed, by action name (see `shortcuts::Keymap::to_saved`).
    pub shortcuts: std::collections::BTreeMap<String, Vec<String>>,
}

impl Default for Prefs {
    fn default() -> Self {
        Prefs {
            default_template: 0,
            force_zoom: false,
            zoom_pct: 100,
            force_layout: false,
            force_style: 0,
            force_notation: 0,
            copy_diagrams: true,
            layout: 0,
            split_score: false,
            reopen_last: false,
            update_check: UpdateCheck::Never,
            last_update_check: 0,
            play_cursor: PlayCursor::Smooth,
            highlight_played_bar: true,
            legacy_swap: false,
            err_bar_duration: true,
            err_out_of_range: true,
            err_tuplets: true,
            err_hopo: true,
            err_unreachable: true,
            artist: String::new(),
            words: String::new(),
            music: String::new(),
            copyright: String::new(),
            transcriber: String::new(),
            instructions: String::new(),
            output_device: None,
            buffer: None,
            midi_output: None,
            midi_input: None,
            midi_builtin_too: false,
            ollama_on: false,
            ollama_url: super::ai::OLLAMA_URL.into(),
            ollama_model: String::new(),
            ollama_context: super::ai::OLLAMA_CONTEXT,
            ollama_warm: true,
            claude_on: false,
            claude_model: super::ai::CLAUDE_MODEL.into(),
            shortcuts: Default::default(),
        }
    }
}

fn prefs_file() -> Option<PathBuf> {
    Some(app_dir()?.join("preferences.json"))
}

fn session_file() -> Option<PathBuf> {
    Some(app_dir()?.join("session.json"))
}

pub(super) fn write_json<T: Serialize + ?Sized>(f: Option<PathBuf>, v: &T) {
    let Some(f) = f else { return };
    if let Some(dir) = f.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(s) = serde_json::to_string_pretty(v) {
        let _ = std::fs::write(f, s);
    }
}

impl Prefs {
    pub fn load() -> Prefs {
        prefs_file()
            .and_then(|f| std::fs::read(f).ok())
            .and_then(|d| serde_json::from_slice::<Prefs>(&d).ok())
            .map(Prefs::fixed)
            .unwrap_or_default()
    }

    pub fn save(&self) {
        write_json(prefs_file(), self);
    }

    /// Values from a hand-edited or older file brought back into range.
    fn fixed(mut self) -> Prefs {
        if self.default_template >= TEMPLATES.len() {
            self.default_template = 0;
        }
        self.zoom_pct = self.zoom_pct.clamp(25, 400);
        self.ollama_context = self.ollama_context.clamp(*super::ai::OLLAMA_CONTEXT_RANGE.start(), *super::ai::OLLAMA_CONTEXT_RANGE.end());
        self.layout = self.layout.min(LAYOUTS.len() as u8 - 1);
        // The old plus/minus swap becomes the same two shortcuts (unless they were set).
        if std::mem::take(&mut self.legacy_swap) && !self.shortcuts.contains_key("Shorter") && !self.shortcuts.contains_key("Longer") {
            self.shortcuts.insert("Shorter".into(), vec!["-".into()]);
            self.shortcuts.insert("Longer".into(), vec!["+".into()]);
        }
        self
    }

    /// Fills the user information into a new song's empty fields.
    pub fn fill_song(&self, s: &mut Song) {
        for (field, v) in [
            (&mut s.artist, &self.artist),
            (&mut s.words, &self.words),
            (&mut s.music_by, &self.music),
            (&mut s.copyright, &self.copyright),
            (&mut s.tabbed_by, &self.transcriber),
            (&mut s.instructions, &self.instructions),
        ] {
            if field.trim().is_empty() && !v.trim().is_empty() {
                *field = v.trim().to_string();
            }
        }
    }

    /// Whether it is time to look for a newer version, `now` in seconds since 1970.
    pub fn update_due(&self, now: u64) -> bool {
        self.update_check.every().is_some_and(|every| now.saturating_sub(self.last_update_check) >= every)
    }
}

/// The files to reopen at start (only those that still exist).
pub fn load_session() -> Vec<PathBuf> {
    session_file()
        .and_then(|f| std::fs::read(f).ok())
        .and_then(|d| serde_json::from_slice::<Vec<PathBuf>>(&d).ok())
        .unwrap_or_default()
        .into_iter()
        .filter(|p| p.is_file())
        .collect()
}

pub fn save_session(list: &[PathBuf]) {
    write_json(session_file(), list);
}

pub fn now_secs() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// The window's state while it is open.
#[derive(Default)]
pub struct PrefsDlg {
    pub tab: usize,
    /// The sound devices found when the window opened (or Refresh was pressed).
    devices: Vec<String>,
    /// The MIDI output and input ports found when the window opened (or Refresh was pressed).
    midi_ports: (Vec<String>, Vec<String>),
    /// Shortcuts: the filter typed in the search box.
    search: String,
    /// Shortcuts: waiting for a key for this action (true: added to its keys).
    pub(super) recording: Option<(Act, bool)>,
    /// Shortcuts: the key just pressed is used by another action; ask first.
    pub(super) clash: Option<Clash>,
    /// Shortcuts: what happened last.
    key_note: String,
    /// AI: the last "Test connection" of each provider.
    ollama_test: super::ai::Probe,
    claude_test: super::ai::Probe,
    /// AI: the Claude key being typed (never kept once saved), whether one is saved, and what
    /// happened to it last.
    key_input: String,
    key_saved: bool,
    ai_key_note: Option<(bool, String)>,
}

/// A key the person chose that another action already uses.
pub(super) struct Clash {
    act: Act,
    add: bool,
    key: Binding,
    other: Act,
}

const LABEL_W: f32 = 210.0;

fn note(ui: &mut Ui, t: &str) {
    ui.label(RichText::new(t).size(12.0).color(Color32::from_gray(150)));
}

/// A choice of MIDI port: "None" and the ports found, with a button to look again.
fn midi_row(
    ui: &mut Ui,
    label: &str,
    id: &str,
    chosen: &mut Option<String>,
    ports: &mut Vec<String>,
    scan: fn() -> Vec<String>,
    changed: &mut bool,
) {
    row_w(ui, LABEL_W, label, |ui| {
        let shown = chosen.clone().unwrap_or_else(|| "None".into());
        egui::ComboBox::from_id_salt(id).width(300.0).selected_text(shown).show_ui(ui, |ui| {
            if ui.selectable_label(chosen.is_none(), "None").clicked() && chosen.is_some() {
                *chosen = None;
                *changed = true;
            }
            for p in ports.iter() {
                let on = chosen.as_deref() == Some(p.as_str());
                if ui.selectable_label(on, p.as_str()).clicked() && !on {
                    *chosen = Some(p.clone());
                    *changed = true;
                }
            }
        });
        if ui.button("\u{27F3}").on_hover_text("Look for devices again").clicked() {
            *ports = scan();
        }
    });
    if let Some(d) = chosen
        && !ports.contains(d)
    {
        note(ui, "This device isn't connected now.");
    }
}

fn text_row(ui: &mut Ui, label: &str, v: &mut String) -> bool {
    let mut changed = false;
    row_w(ui, LABEL_W, label, |ui| {
        changed = ui.add(egui::TextEdit::singleline(v).desired_width(320.0)).changed();
    });
    changed
}

/// A model name to type, with the models the last test found to pick from.
fn model_row(ui: &mut Ui, id: &str, v: &mut String, probe: &super::ai::Probe) {
    row_w(ui, LABEL_W, "Model:", |ui| {
        ui.add(egui::TextEdit::singleline(v).desired_width(240.0));
        if let Some(super::ai::Test::Ok(found)) = &probe.result
            && !found.is_empty()
        {
            egui::ComboBox::from_id_salt(id).width(70.0).selected_text("Pick").show_ui(ui, |ui| {
                for m in found {
                    if ui.selectable_label(v == m, m.as_str()).clicked() {
                        *v = m.clone();
                    }
                }
            });
        }
    });
}

/// The "Test connection" button and what the last test found. True when it was pressed.
fn test_row(ui: &mut Ui, probe: &super::ai::Probe) -> bool {
    let mut pressed = false;
    row_w(ui, LABEL_W, "", |ui| {
        pressed = ui.add_enabled(!probe.busy(), egui::Button::new("Test connection")).clicked();
        match &probe.result {
            _ if probe.busy() => note(ui, "Connecting..."),
            Some(super::ai::Test::Ok(m)) => note(ui, &format!("Connected. {} model{} available.", m.len(), if m.len() == 1 { "" } else { "s" })),
            Some(super::ai::Test::Failed(e)) => {
                ui.label(RichText::new(e.as_str()).size(12.0).color(Color32::from_rgb(210, 90, 80)));
            }
            None => {}
        }
    });
    pressed
}

impl App {
    /// File > Preferences... (Ctrl+,).
    pub(super) fn open_prefs(&mut self) {
        if self.prefs_dlg.is_none() {
            self.prefs_dlg = Some(PrefsDlg { tab: 0, key_saved: super::ai::key::exists(), devices: Audio::output_device_names(), midi_ports: (crate::midi::output_port_names(), crate::midi::input_port_names()), ..PrefsDlg::default() });
        }
    }

    /// Opens Preferences on the AI tab.
    pub(super) fn open_prefs_ai(&mut self) {
        self.open_prefs();
        if let Some(d) = &mut self.prefs_dlg {
            d.tab = 5;
        }
    }

    /// Applies the forced zoom and page layout to a song just opened or created.
    pub(super) fn apply_forced_view(&mut self) {
        if self.prefs.force_zoom {
            self.zoom = self.prefs.zoom_pct as f32 / 100.0;
        }
        if self.prefs.force_layout {
            self.view_mode = self.prefs.layout;
        }
        // The forced stylesheet and notation change the song itself, so they count as a change
        // only when something differs.
        let style = match self.prefs.force_style {
            1 => Some(crate::style::Stylesheet::standard()),
            2 => Some(crate::style::Stylesheet::rock()),
            3 => Some(crate::style::Stylesheet::classic()),
            4 => Some(crate::style::Stylesheet::jazz()),
            _ => None,
        };
        if let Some(s) = style {
            self.song.style = s;
        }
        let (std, tab) = match self.prefs.force_notation {
            1 => (true, true),
            2 => (true, false),
            3 => (false, true),
            _ => return,
        };
        for t in self.song.tracks.iter_mut().filter(|t| t.has_strings()) {
            t.show_std = std;
            t.show_tab = tab;
        }
    }

    /// Takes the chord diagrams out of a copy when Preferences says not to copy them.
    pub(super) fn strip_copied_diagrams(&self, beats: &mut [crate::model::Beat]) {
        if !self.prefs.copy_diagrams {
            for b in beats {
                b.chord_diagram = false;
            }
        }
    }

    /// The files open in every tab, to reopen next time.
    pub(super) fn open_paths(&self) -> Vec<PathBuf> {
        let mut v = vec![];
        for (i, d) in self.tabs.iter().enumerate() {
            let p = match d {
                Some(d) => d.path.clone(),
                None if i == self.active_tab => self.path.clone(),
                None => None,
            };
            if let Some(p) = p
                && !v.contains(&p)
            {
                v.push(p);
            }
        }
        v
    }

    /// At start: reopen last session's files and look for updates if it is time.
    pub(super) fn prefs_at_start(&mut self, ctx: &egui::Context) {
        if self.prefs.reopen_last {
            let files = load_session();
            for p in files {
                self.open_path(p);
            }
        }
        let now = now_secs();
        if self.prefs.update_due(now) {
            self.prefs.last_update_check = now;
            self.prefs.save();
            self.check_for_updates_quietly(ctx);
        }
    }

    /// Shorter note (+) and Longer note (−).
    pub(super) fn shorter_longer(&mut self, shorter: bool) {
        let d = self.cur_beat().dur;
        self.set_duration(if shorter { d.shorter() } else { d.longer() });
    }

    pub(super) fn prefs_window(&mut self, ctx: &egui::Context) {
        let Some(mut dlg) = self.prefs_dlg.take() else {
            return;
        };
        let before = self.prefs.clone();
        let mut open = true;
        let mut close = false;
        let mut restart_audio = false;
        let mut test_sound = false;
        let mut midi_changed = false;
        // A key being recorded for a shortcut is taken before anything else sees it.
        let recorded = if dlg.recording.is_some() { take_recorded_key(ctx) } else { None };
        match recorded {
            Some(None) => {
                dlg.recording = None;
                dlg.key_note = "Nothing changed.".into();
            }
            Some(Some(k)) => {
                if let Some((act, add)) = dlg.recording.take() {
                    self.choose_key(&mut dlg, act, add, k);
                }
            }
            None => {}
        }
        egui::Window::new("Preferences")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .pivot(Align2::CENTER_CENTER)
            .default_pos(ctx.content_rect().center())
            .fixed_size(Vec2::new(720.0, 560.0))
            .constrain(true)
            .frame(egui::Frame::window(&ctx.global_style()).fill(BODY_BG).inner_margin(0.0))
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = 6.0;
                egui::Panel::top("pref-tabs").frame(egui::Frame::NONE).resizable(false).show(ui, |ui| {
                    tab_row(ui, &TABS, "pref-tab", &mut dlg.tab, |p, i, c, col| {
                        let glyph = ["\u{2699}", "\u{25A3}", "\u{26A0}", "\u{263A}", "\u{266B}", "\u{2728}", "\u{2328}"][i];
                        p.text(c, Align2::CENTER_CENTER, glyph, FontId::proportional(20.0), col);
                    });
                });
                egui::Panel::bottom("pref-bar")
                    .frame(egui::Frame::new().fill(BODY_BG).inner_margin(egui::Margin::symmetric(14, 10)))
                    .resizable(false)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            if ui.button("Restore defaults").on_hover_text("Every setting on this tab back to how GuitarCraft starts").clicked() {
                                self.restore_tab(dlg.tab);
                                dlg.recording = None;
                                dlg.clash = None;
                                if dlg.tab == 4 {
                                    restart_audio = true;
                                }
                            }
                            note(ui, "Changes take effect straight away.");
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button("Close").clicked() {
                                    close = true;
                                }
                            });
                        });
                    });
                egui::CentralPanel::default().frame(egui::Frame::new().inner_margin(egui::Margin::symmetric(18, 4))).show(ui, |ui| {
                    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| match dlg.tab {
                        0 => self.general_tab(ui),
                        1 => self.interface_tab(ui),
                        2 => self.errors_tab(ui),
                        3 => self.user_tab(ui),
                        4 => self.audio_tab(ui, &mut dlg.devices, &mut dlg.midi_ports, &mut restart_audio, &mut test_sound, &mut midi_changed),
                        5 => self.ai_tab(ui, &mut dlg),
                        _ => self.shortcuts_tab(ui, &mut dlg),
                    });
                });
            });
        if self.prefs != before {
            self.prefs.save();
        }
        if restart_audio {
            self.restart_audio();
        }
        if test_sound {
            self.test_sound();
        }
        if midi_changed {
            self.connect_midi();
        }
        // Esc closes the window, except the Esc that cancels recording a key.
        let esc = recorded.is_none() && dlg.recording.is_none() && ctx.input(|i| i.key_pressed(egui::Key::Escape));
        if open && !close && !esc {
            self.prefs_dlg = Some(dlg);
        }
    }

    fn restore_tab(&mut self, tab: usize) {
        let d = Prefs::default();
        let p = &mut self.prefs;
        match tab {
            0 => {
                p.default_template = d.default_template;
                p.force_zoom = d.force_zoom;
                p.zoom_pct = d.zoom_pct;
                p.force_layout = d.force_layout;
                p.layout = d.layout;
                p.force_style = d.force_style;
                p.force_notation = d.force_notation;
                p.reopen_last = d.reopen_last;
                p.update_check = d.update_check;
            }
            1 => {
                p.play_cursor = d.play_cursor;
                p.highlight_played_bar = d.highlight_played_bar;
                p.copy_diagrams = d.copy_diagrams;
            }
            2 => {
                p.err_bar_duration = d.err_bar_duration;
                p.err_out_of_range = d.err_out_of_range;
                p.err_tuplets = d.err_tuplets;
                p.err_hopo = d.err_hopo;
                p.err_unreachable = d.err_unreachable;
            }
            3 => {
                p.artist.clear();
                p.words.clear();
                p.music.clear();
                p.copyright.clear();
                p.transcriber.clear();
                p.instructions.clear();
            }
            4 => {
                p.output_device = d.output_device;
                p.buffer = d.buffer;
                p.midi_output = d.midi_output;
                p.midi_input = d.midi_input;
                p.midi_builtin_too = d.midi_builtin_too;
            }
            5 => {
                p.ollama_on = d.ollama_on;
                p.ollama_url = d.ollama_url;
                p.ollama_model = d.ollama_model;
                p.ollama_context = d.ollama_context;
                p.ollama_warm = d.ollama_warm;
                p.claude_on = d.claude_on;
                p.claude_model = d.claude_model;
            }
            _ => {
                p.shortcuts.clear();
                set_keymap(Keymap::default());
            }
        }
    }

    fn general_tab(&mut self, ui: &mut Ui) {
        let p = &mut self.prefs;
        heading(ui, "Documents");
        row_w(ui, LABEL_W, "Default template:", |ui| {
            egui::ComboBox::from_id_salt("pref-template")
                .width(260.0)
                .selected_text(TEMPLATES[p.default_template])
                .show_ui(ui, |ui| {
                    for (i, t) in TEMPLATES.iter().enumerate() {
                        ui.selectable_value(&mut p.default_template, i, *t);
                    }
                });
        });
        note(ui, "File > New starts from this template.");
        ui.add_space(6.0);
        row_w(ui, LABEL_W, "Force zoom:", |ui| {
            switch(ui, &mut p.force_zoom);
            ui.add_enabled(p.force_zoom, egui::DragValue::new(&mut p.zoom_pct).range(25..=400).suffix(" %").speed(1.0));
        });
        row_w(ui, LABEL_W, "Force page layout:", |ui| {
            switch(ui, &mut p.force_layout);
            ui.add_enabled_ui(p.force_layout, |ui| {
                egui::ComboBox::from_id_salt("pref-layout")
                    .width(180.0)
                    .selected_text(LAYOUTS[p.layout as usize])
                    .show_ui(ui, |ui| {
                        for (i, l) in LAYOUTS.iter().enumerate() {
                            ui.selectable_value(&mut p.layout, i as u8, *l);
                        }
                    });
            });
        });
        note(ui, "Forced settings are used for every song opened or created, whatever the file says.");
        row_w(ui, LABEL_W, "Force stylesheet:", |ui| {
            let mut on = p.force_style > 0;
            if switch(ui, &mut on) {
                p.force_style = if on { 1 } else { 0 };
            }
            ui.add_enabled_ui(on, |ui| {
                egui::ComboBox::from_id_salt("pref-style")
                    .width(180.0)
                    .selected_text(FORCED_STYLES[(p.force_style.max(1) - 1) as usize])
                    .show_ui(ui, |ui| {
                        for (i, l) in FORCED_STYLES.iter().enumerate() {
                            ui.selectable_value(&mut p.force_style, i as u8 + 1, *l);
                        }
                    });
            });
        });
        row_w(ui, LABEL_W, "Force notation:", |ui| {
            let mut on = p.force_notation > 0;
            if switch(ui, &mut on) {
                p.force_notation = if on { 1 } else { 0 };
            }
            ui.add_enabled_ui(on, |ui| {
                egui::ComboBox::from_id_salt("pref-notation")
                    .width(260.0)
                    .selected_text(FORCED_NOTATIONS[(p.force_notation.max(1) - 1) as usize])
                    .show_ui(ui, |ui| {
                        for (i, l) in FORCED_NOTATIONS.iter().enumerate() {
                            ui.selectable_value(&mut p.force_notation, i as u8 + 1, *l);
                        }
                    });
            });
        });
        ui.add_space(6.0);
        row_w(ui, LABEL_W, "Open last session's documents:", |ui| {
            switch(ui, &mut p.reopen_last);
        });
        heading(ui, "Updates");
        row_w(ui, LABEL_W, "Look for a new version:", |ui| {
            egui::ComboBox::from_id_salt("pref-update")
                .width(180.0)
                .selected_text(p.update_check.label())
                .show_ui(ui, |ui| {
                    for (v, l) in UpdateCheck::ALL {
                        ui.selectable_value(&mut p.update_check, v, l);
                    }
                });
        });
        note(ui, "The check asks GitHub for the newest version number when GuitarCraft starts; it sends nothing about you or your files.");
    }

    fn interface_tab(&mut self, ui: &mut Ui) {
        let p = &mut self.prefs;
        heading(ui, "Score");
        row_w(ui, LABEL_W, "Playing cursor style:", |ui| {
            let cur = CURSORS.iter().find(|c| c.0 == p.play_cursor).map(|c| c.1).unwrap_or("");
            egui::ComboBox::from_id_salt("pref-cursor").width(160.0).selected_text(cur).show_ui(ui, |ui| {
                for (v, l, tip) in CURSORS {
                    ui.selectable_value(&mut p.play_cursor, v, l).on_hover_text(tip);
                }
            });
        });
        row_w(ui, LABEL_W, "Highlight the bar being played:", |ui| {
            switch(ui, &mut p.highlight_played_bar);
        });
        row_w(ui, LABEL_W, "Copy chord and scale diagrams:", |ui| {
            switch(ui, &mut p.copy_diagrams);
            note(ui, "Off: copying leaves the diagrams behind (Special Paste can still choose)");
        });
    }

    fn errors_tab(&mut self, ui: &mut Ui) {
        let p = &mut self.prefs;
        heading(ui, "Notation errors");
        note(ui, "Like a spelling checker: mistakes in the score are shown in red as you write. They are never printed.");
        ui.add_space(6.0);
        row_w(ui, LABEL_W, "Wrong bar length:", |ui| {
            switch(ui, &mut p.err_bar_duration);
            note(ui, "Bars with too many or too few beats for their time signature");
        });
        row_w(ui, LABEL_W, "Note out of range:", |ui| {
            switch(ui, &mut p.err_out_of_range);
            note(ui, "A fret beyond the end of the neck");
        });
        row_w(ui, LABEL_W, "Invalid tuplets:", |ui| {
            switch(ui, &mut p.err_tuplets);
            note(ui, "A tuplet group that isn't complete");
        });
        row_w(ui, LABEL_W, "Invalid hammer-on / pull-off:", |ui| {
            switch(ui, &mut p.err_hopo);
            note(ui, "Nothing on that string after it to slur to");
        });
        row_w(ui, LABEL_W, "Bar never reached in playback:", |ui| {
            switch(ui, &mut p.err_unreachable);
            note(ui, "A repeat or jump means playback skips it");
        });
    }

    fn ai_tab(&mut self, ui: &mut Ui, dlg: &mut PrefsDlg) {
        let ctx = ui.ctx().clone();
        dlg.ollama_test.poll();
        dlg.claude_test.poll();
        if dlg.ollama_test.busy() || dlg.claude_test.busy() {
            ctx.request_repaint_after(std::time::Duration::from_millis(200));
        }
        let p = &mut self.prefs;
        heading(ui, "Ollama");
        note(ui, "A model running on your computer, or on one you run yourself. Nothing leaves your network.");
        ui.add_space(6.0);
        row_w(ui, LABEL_W, "Use Ollama:", |ui| {
            switch(ui, &mut p.ollama_on);
            if !p.ollama_on {
                note(ui, "Switch on to use Ollama in the AI Assistant.");
            }
        });
        text_row(ui, "Server address:", &mut p.ollama_url);
        model_row(ui, "ollama-model", &mut p.ollama_model, &dlg.ollama_test);
        row_w(ui, LABEL_W, "Context size:", |ui| {
            ui.add(egui::DragValue::new(&mut p.ollama_context).range(super::ai::OLLAMA_CONTEXT_RANGE).speed(256));
            note(ui, "tokens");
            for size in [8192, 16_384, 32_768] {
                if ui.small_button(format!("{}k", size / 1024)).clicked() {
                    p.ollama_context = size;
                }
            }
        });
        note(ui, "How much text the model reads at once. Larger sizes use more memory and run slower.");
        row_w(ui, LABEL_W, "Keep model loaded:", |ui| {
            switch(ui, &mut p.ollama_warm);
        });
        note(ui, "Loads the model when the assistant opens and keeps it in memory for 30 minutes, so the first answer is faster. Uses memory while it is loaded.");
        if test_row(ui, &dlg.ollama_test) {
            dlg.ollama_test.test_ollama(&ctx, &p.ollama_url);
        }
        ui.add_space(14.0);
        heading(ui, "Claude");
        note(ui, "Anthropic's AI service. Needs an API key from console.anthropic.com. What you ask is sent over the internet to Anthropic.");
        ui.add_space(6.0);
        row_w(ui, LABEL_W, "Use Claude:", |ui| {
            switch(ui, &mut p.claude_on);
            if !p.claude_on {
                note(ui, "Switch on to use Claude in the AI Assistant.");
            }
        });
        let (mut save, mut remove) = (false, false);
        row_w(ui, LABEL_W, "API key:", |ui| {
            let hint = if dlg.key_saved { "saved - paste a new key to replace it" } else { "paste your key here" };
            ui.add(egui::TextEdit::singleline(&mut dlg.key_input).password(true).hint_text(hint).desired_width(240.0));
            save = ui.add_enabled(!dlg.key_input.trim().is_empty(), egui::Button::new("Save")).clicked();
            remove = ui.add_enabled(dlg.key_saved, egui::Button::new("Remove")).clicked();
        });
        if save {
            dlg.ai_key_note = Some(match super::ai::key::set(&dlg.key_input) {
                Ok(()) => {
                    dlg.key_input.clear();
                    dlg.key_saved = true;
                    dlg.claude_test.result = None;
                    (true, "Key saved.".to_string())
                }
                Err(e) => (false, format!("Couldn't save the key: {e}")),
            });
        }
        if remove {
            dlg.ai_key_note = Some(match super::ai::key::remove() {
                Ok(()) => {
                    dlg.key_saved = false;
                    dlg.claude_test.result = None;
                    (true, "Key removed.".to_string())
                }
                Err(e) => (false, format!("Couldn't remove the key: {e}")),
            });
        }
        if let Some((ok, text)) = &dlg.ai_key_note {
            row_w(ui, LABEL_W, "", |ui| {
                if *ok {
                    note(ui, text);
                } else {
                    ui.label(RichText::new(text.as_str()).size(12.0).color(Color32::from_rgb(210, 90, 80)));
                }
            });
        }
        model_row(ui, "claude-model", &mut p.claude_model, &dlg.claude_test);
        if test_row(ui, &dlg.claude_test) {
            dlg.claude_test.test_claude(&ctx, &super::ai::key::get().unwrap_or_default());
        }
        ui.add_space(8.0);
        note(ui, "The key is kept in the Windows Credential Manager, never in GuitarCraft's preferences file.");
    }

    fn user_tab(&mut self, ui: &mut Ui) {
        let p = &mut self.prefs;
        heading(ui, "Your details");
        note(ui, "Filled into every new song (you can still change them in the song's information).");
        ui.add_space(6.0);
        text_row(ui, "Artist:", &mut p.artist);
        text_row(ui, "Words:", &mut p.words);
        text_row(ui, "Music:", &mut p.music);
        text_row(ui, "Copyright:", &mut p.copyright);
        text_row(ui, "Transcriber:", &mut p.transcriber);
        text_row(ui, "Instructions:", &mut p.instructions);
    }

    fn audio_tab(&mut self, ui: &mut Ui, devices: &mut Vec<String>, midi: &mut (Vec<String>, Vec<String>), restart: &mut bool, test: &mut bool, midi_changed: &mut bool) {
        let p = &mut self.prefs;
        heading(ui, "Audio");
        row_w(ui, LABEL_W, "Output device:", |ui| {
            let shown = p.output_device.clone().unwrap_or_else(|| "System default".into());
            egui::ComboBox::from_id_salt("pref-device").width(300.0).selected_text(shown).show_ui(ui, |ui| {
                if ui.selectable_label(p.output_device.is_none(), "System default").clicked() && p.output_device.is_some() {
                    p.output_device = None;
                    *restart = true;
                }
                for d in devices.iter() {
                    let on = p.output_device.as_deref() == Some(d.as_str());
                    if ui.selectable_label(on, d.as_str()).clicked() && !on {
                        p.output_device = Some(d.clone());
                        *restart = true;
                    }
                }
            });
            if ui.button("\u{27F3}").on_hover_text("Look for devices again").clicked() {
                *devices = Audio::output_device_names();
            }
            if ui.button("Check").on_hover_text("Play a note").clicked() {
                *test = true;
            }
        });
        if let Some(d) = &p.output_device
            && !devices.contains(d)
        {
            note(ui, "This device isn't connected now: the system's default device is used.");
        }
        row_w(ui, LABEL_W, "Buffer size:", |ui| {
            let label = |b: Option<u32>| b.map(|n| format!("{n} samples")).unwrap_or_else(|| "Automatic".into());
            egui::ComboBox::from_id_salt("pref-buffer").width(160.0).selected_text(label(p.buffer)).show_ui(ui, |ui| {
                for b in BUFFERS {
                    if ui.selectable_label(p.buffer == b, label(b)).clicked() && p.buffer != b {
                        p.buffer = b;
                        *restart = true;
                    }
                }
            });
        });
        note(ui, "Smaller buffers answer faster but may crackle; larger ones are steadier but lag a little.");
        heading(ui, "MIDI");
        midi_row(ui, "MIDI output:", "pref-midi-out", &mut p.midi_output, &mut midi.0, crate::midi::output_port_names, midi_changed);
        midi_row(ui, "MIDI input:", "pref-midi-in", &mut p.midi_input, &mut midi.1, crate::midi::input_port_names, midi_changed);
        if let Some(e) = &self.midi_error {
            note(ui, e);
        }
        row_w(ui, LABEL_W, "Built-in sound with MIDI output:", |ui| {
            switch(ui, &mut p.midi_builtin_too);
            note(ui, "Off: only the MIDI device plays while an output is chosen");
        });
        if self.midi.input_open() {
            note(ui, &format!("MIDI input connected: {} message(s) received. Play a key to check.", self.midi.activity()));
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(200));
        }
        if self.midi.output_open() {
            note(ui, "MIDI output connected: notes and playback go there. The setting applies when playback starts.");
        }
    }

    /// Enters the notes played on the MIDI input at the cursor: notes pressed together make one
    /// chord in one beat, then the cursor moves on like the right arrow. Called every frame.
    /// Nothing is entered while the song plays or when no score shows.
    pub(super) fn handle_midi_input(&mut self, ctx: &egui::Context) {
        if !self.midi.input_open() {
            self.midi_pending.clear();
            return;
        }
        self.midi_pending.extend(self.midi.take_played());
        if self.playing || self.show_home || self.chart.is_some() || self.prefs_dlg.is_some() {
            self.midi_pending.clear();
        }
        let now = std::time::Instant::now();
        while let Some(chord) = crate::midi::take_ready_chord(&mut self.midi_pending, now, crate::midi::CHORD_WINDOW) {
            self.enter_midi_chord(&chord);
        }
        // Nothing wakes the screen when a key is pressed, so look again soon (sooner while a
        // chord is still being played).
        let again = if self.midi_pending.is_empty() { 50 } else { 10 };
        ctx.request_repaint_after(std::time::Duration::from_millis(again));
    }

    /// Puts a chord (pitches, lowest first) into the beat at the cursor and moves on.
    fn enter_midi_chord(&mut self, pitches: &[u8]) {
        let mut used: Vec<u8> = self.cur_beat().notes.iter().map(|n| n.string).collect();
        let mut skipped = 0;
        for &pitch in pitches {
            // A single note starts from the cursor's string; a chord takes the lowest frets.
            let prefer = if pitches.len() == 1 { self.string } else { usize::MAX };
            match crate::midi::string_and_fret(self.tr(), pitch, prefer, &used) {
                Some((string, fret)) => {
                    used.push(string as u8);
                    self.set_fret(string, fret);
                }
                None => skipped += 1,
            }
        }
        if skipped < pitches.len() {
            self.move_right(false);
        }
        if skipped > 0 {
            self.refuse(format!("{skipped} note(s) out of range for this track were left out."));
        }
    }

    /// The built-in sound stays quiet: a MIDI output is open and Preferences doesn't ask for both.
    pub(super) fn mute_builtin(&self) -> bool {
        self.midi.output_open() && !self.prefs.midi_builtin_too
    }

    /// Opens the MIDI ports Preferences chose; a port that fails says why in Preferences.
    pub(super) fn connect_midi(&mut self) {
        let out = self.midi.set_output(self.prefs.midi_output.as_deref());
        let inp = self.midi.set_input(self.prefs.midi_input.as_deref());
        let errs: Vec<String> = [out, inp].into_iter().filter_map(|r| r.err()).collect();
        self.midi_error = if errs.is_empty() { None } else { Some(errs.join("  ")) };
    }

    /// Starts the sound again with the chosen device and buffer.
    fn restart_audio(&mut self) {
        if self.playing {
            self.stop();
        }
        if self.audio.take().is_some() {
            self.ensure_audio();
            self.status = match self.audio.as_ref().and_then(|a| a.error.clone()) {
                Some(e) => super::notice::failed(format!("Sound unavailable: {e}")),
                None => "Sound output changed.".into(),
            };
        }
    }

    /// Audio > Check: an open A string, through the chosen device.
    fn test_sound(&mut self) {
        if self.playing || !self.sound_allowed {
            return;
        }
        self.ensure_audio();
        let Some(a) = &self.audio else { return };
        if let Some(e) = &a.error {
            self.status = super::notice::failed(format!("Sound unavailable: {e}"));
            return;
        }
        let track = crate::model::Track { tuning: vec![57], ..crate::model::Track::default() };
        let ev = crate::audio::note_preview(&track, 0, &crate::model::Note::new(0, 0), a.sample_rate());
        if let Ok(mut st) = a.state.lock() {
            st.preview.push(ev);
        }
    }

    // ----- Shortcuts tab -----

    /// Changes the keymap in use and keeps the preferences in step.
    fn change_keys(&mut self, f: impl FnOnce(&mut Keymap)) {
        let mut km = keymap();
        f(&mut km);
        self.prefs.shortcuts = km.to_saved();
        set_keymap(km);
    }

    /// The key pressed for `act`: used at once, refused, or asked about if it is taken.
    fn choose_key(&mut self, dlg: &mut PrefsDlg, act: Act, add: bool, k: Binding) {
        let km = keymap();
        let name = action(act).name;
        if km.keys(act).contains(&k) {
            dlg.key_note = format!("{} is already a shortcut of \u{201C}{name}\u{201D}.", k.label());
            return;
        }
        if let Some(other) = km.clash(act, &k) {
            if action(other).fixed {
                dlg.key_note =
                    format!("{} is kept for \u{201C}{}\u{201D}, which can't be changed. Try another key.", k.label(), action(other).name);
            } else {
                dlg.clash = Some(Clash { act, add, key: k, other });
                dlg.key_note.clear();
            }
            return;
        }
        self.give_key(act, add, k);
        dlg.key_note = format!("\u{201C}{name}\u{201D} is now {}.", super::shortcuts::all_keys(act));
    }

    fn give_key(&mut self, act: Act, add: bool, k: Binding) {
        self.change_keys(|km| {
            let mut keys = if add { km.keys(act) } else { vec![] };
            keys.push(k);
            km.set(act, keys);
        });
    }

    /// Takes a key from another action (keeping its other keys, and its aliases).
    fn take_key_from(&mut self, other: Act, k: Binding) {
        self.change_keys(|km| {
            let keys: Vec<Binding> = km.all_keys_of(other).into_iter().filter(|x| *x != k).collect();
            km.set(other, keys);
        });
    }

    fn shortcuts_tab(&mut self, ui: &mut Ui, dlg: &mut PrefsDlg) {
        heading(ui, "Keyboard shortcuts");
        note(ui, "Click Change and press the new key (Esc cancels); + adds a second key. Typing and moving keys (digits, arrows, Tab...) can't be changed.");
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label("Find:");
            ui.add(egui::TextEdit::singleline(&mut dlg.search).hint_text("action or key").desired_width(260.0));
            if !dlg.search.is_empty() && ui.small_button("\u{2716}").clicked() {
                dlg.search.clear();
            }
        });
        // What is going on: waiting for a key, a key that is taken, or the last change.
        if let Some((act, _)) = dlg.recording {
            ui.label(
                RichText::new(format!("Press the new key for \u{201C}{}\u{201D}\u{2026}  (Esc cancels)", action(act).name))
                    .color(Color32::from_rgb(240, 190, 90)),
            );
        } else if let Some(c) = &dlg.clash {
            let (act, add, key, other) = (c.act, c.add, c.key, c.other);
            let mut done = false;
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(format!(
                        "{} is used by \u{201C}{}\u{201D}. Use it for \u{201C}{}\u{201D} instead?",
                        key.label(),
                        action(other).name,
                        action(act).name
                    ))
                    .color(Color32::from_rgb(240, 190, 90)),
                );
                if ui.button("Use it here").clicked() {
                    self.take_key_from(other, key);
                    self.give_key(act, add, key);
                    dlg.key_note = format!(
                        "\u{201C}{}\u{201D} is now {}; \u{201C}{}\u{201D} is {}.",
                        action(act).name,
                        super::shortcuts::all_keys(act),
                        action(other).name,
                        super::shortcuts::all_keys(other)
                    );
                    done = true;
                }
                if ui.button("Cancel").clicked() {
                    dlg.key_note = "Nothing changed.".into();
                    done = true;
                }
            });
            if done {
                dlg.clash = None;
            }
        } else if !dlg.key_note.is_empty() {
            note(ui, &dlg.key_note);
        }
        let busy = dlg.recording.is_some() || dlg.clash.is_some();
        let km = keymap();
        let q = dlg.search.trim().to_lowercase();
        for g in Group::ALL {
            let rows: Vec<_> = ACTIONS
                .iter()
                .filter(|x| x.group == g)
                .filter(|x| {
                    q.is_empty()
                        || x.name.to_lowercase().contains(&q)
                        || super::shortcuts::all_keys(x.act).to_lowercase().contains(&q)
                })
                .collect();
            if rows.is_empty() {
                continue;
            }
            ui.add_space(6.0);
            ui.label(RichText::new(g.title()).strong().size(14.0).color(Color32::from_gray(235)));
            egui::Grid::new(("pref-keys", g.title())).striped(true).min_col_width(10.0).show(ui, |ui| {
                for x in rows {
                    let custom = km.is_custom(x.act);
                    left_cell(ui, 300.0, egui::Label::new(x.name).truncate());
                    let keys = RichText::new(super::shortcuts::all_keys(x.act)).strong();
                    let keys = if custom { keys.color(Color32::from_rgb(110, 170, 250)) } else { keys };
                    left_cell(ui, 170.0, egui::Label::new(keys).truncate())
                        .on_hover_text(if custom { "Changed by you" } else { "GuitarCraft's key" });
                    ui.horizontal(|ui| {
                        if x.fixed {
                            ui.label(RichText::new("can't be changed").size(12.0).color(Color32::from_gray(120)));
                            return;
                        }
                        ui.add_enabled_ui(!busy, |ui| {
                            if ui.small_button("Change").on_hover_text("Press a new key for this").clicked() {
                                dlg.recording = Some((x.act, false));
                                dlg.key_note.clear();
                            }
                            if ui.small_button("+").on_hover_text("Add another key").clicked() {
                                dlg.recording = Some((x.act, true));
                                dlg.key_note.clear();
                            }
                            if !km.keys(x.act).is_empty() && ui.small_button("\u{2716}").on_hover_text("No shortcut").clicked() {
                                self.change_keys(|km| km.set(x.act, vec![]));
                                dlg.key_note = format!("\u{201C}{}\u{201D} has no shortcut now.", x.name);
                            }
                            if custom && ui.small_button("\u{21BA}").on_hover_text("Back to GuitarCraft's key").clicked() {
                                self.change_keys(|km| km.reset(x.act));
                                dlg.key_note = format!("\u{201C}{}\u{201D} is {} again.", x.name, super::shortcuts::all_keys(x.act));
                            }
                        });
                    });
                    ui.end_row();
                }
            });
        }
    }
}

/// A widget in a left-aligned cell of a fixed width.
fn left_cell(ui: &mut Ui, w: f32, widget: impl egui::Widget) -> egui::Response {
    ui.allocate_ui_with_layout(Vec2::new(w, 20.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
        ui.set_min_width(w);
        ui.add(widget)
    })
    .inner
}

/// While a shortcut is being recorded: the key pressed this frame, taken away from
/// everything else. None: nothing yet; Some(None): Esc (cancel); Some(Some(key)).
/// A symbol typed with Shift (such as $ or %) is kept as that symbol, so it works
/// on any keyboard layout, as the standard symbol shortcuts do.
fn take_recorded_key(ctx: &egui::Context) -> Option<Option<Binding>> {
    let events = ctx.input_mut(|i| std::mem::take(&mut i.events));
    let shift = ctx.input(|i| i.modifiers.shift);
    let clip = |k: egui::Key| Binding::of(k, egui::Modifiers { ctrl: true, command: true, shift, ..egui::Modifiers::NONE });
    let (mut key, mut typed) = (None, None);
    for ev in events {
        match ev {
            egui::Event::Key { key: egui::Key::Escape, pressed: true, modifiers, .. } if !modifiers.any() => return Some(None),
            egui::Event::Key { key: k, pressed: true, modifiers, .. } if key.is_none() => key = Some(Binding::of(k, modifiers)),
            egui::Event::Copy if key.is_none() => key = Some(clip(egui::Key::C)),
            egui::Event::Cut if key.is_none() => key = Some(clip(egui::Key::X)),
            egui::Event::Paste(_) if key.is_none() => key = Some(clip(egui::Key::V)),
            egui::Event::Text(t) => {
                if let Some(c) = t.chars().next()
                    && !c.is_alphanumeric()
                    && !c.is_whitespace()
                {
                    typed = Some(Binding::typed(c));
                }
            }
            _ => {}
        }
    }
    match (key, typed) {
        // Ctrl and Alt keys are kept as keys; a plain or shifted symbol as the symbol.
        (Some(k), _) if k.ctrl || k.alt => Some(Some(k)),
        (_, Some(t)) => Some(Some(t)),
        (Some(k), None) => Some(Some(k)),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_information_fills_only_empty_fields() {
        let p = Prefs { artist: "Me".into(), copyright: "(c) 2026".into(), ..Prefs::default() };
        let mut s = Song::new_blank();
        s.artist.clear();
        s.copyright = "Theirs".into();
        p.fill_song(&mut s);
        assert_eq!(s.artist, "Me");
        assert_eq!(s.copyright, "Theirs");
    }

    #[test]
    fn the_midi_choices_survive_saving_and_loading() {
        let p = Prefs { midi_output: Some("Microsoft GS Wavetable Synth".into()), midi_input: Some("Keys".into()), ..Prefs::default() };
        let json = serde_json::to_string_pretty(&p).unwrap();
        let back = serde_json::from_slice::<Prefs>(json.as_bytes()).unwrap().fixed();
        assert_eq!(back.midi_output.as_deref(), Some("Microsoft GS Wavetable Synth"));
        assert_eq!(back.midi_input.as_deref(), Some("Keys"));
    }

    #[test]
    fn update_checks_come_when_due() {
        let mut p = Prefs::default();
        assert!(!p.update_due(10_000_000), "never by default");
        p.update_check = UpdateCheck::Weekly;
        p.last_update_check = 1_000_000;
        assert!(!p.update_due(1_000_000 + 6 * 86_400));
        assert!(p.update_due(1_000_000 + 7 * 86_400));
    }

    #[test]
    fn the_old_plus_minus_swap_becomes_two_shortcuts() {
        use super::super::shortcuts::{Act, Keymap, Place};
        let p: Prefs = serde_json::from_str(r#"{"swap_plus_minus": true}"#).unwrap();
        let p = p.fixed();
        let km = Keymap::from_saved(&p.shortcuts);
        let plus = super::super::shortcuts::Binding::of(egui::Key::Plus, egui::Modifiers::NONE);
        let minus = super::super::shortcuts::Binding::of(egui::Key::Minus, egui::Modifiers::NONE);
        assert_eq!(km.find(Place::Score, &plus), Some(Act::Longer));
        assert_eq!(km.find(Place::Score, &minus), Some(Act::Shorter));
        assert!(!serde_json::to_string(&p).unwrap().contains("swap_plus_minus"), "not saved again");
        // Without the old option nothing changes.
        let q: Prefs = serde_json::from_str("{}").unwrap();
        assert!(q.fixed().shortcuts.is_empty());
    }

    #[test]
    fn odd_saved_values_are_brought_into_range() {
        let p: Prefs = serde_json::from_str(r#"{"default_template": 999, "zoom_pct": 5, "layout": 9, "ollama_context": 10}"#).unwrap();
        let p = p.fixed();
        assert_eq!(p.default_template, 0);
        assert_eq!(p.zoom_pct, 25);
        assert_eq!(p.layout, 4);
        assert_eq!(p.ollama_context, 2048);
        assert!(p.highlight_played_bar, "missing fields keep their defaults");
    }

    #[test]
    fn the_split_view_is_off_unless_chosen_and_is_remembered() {
        assert!(!Prefs::default().split_score, "off by default");
        let old: Prefs = serde_json::from_str(r#"{"zoom_pct": 100}"#).unwrap();
        assert!(!old.split_score, "a preferences file from before has it off");
        let on = Prefs { split_score: true, ..Prefs::default() };
        let back: Prefs = serde_json::from_str(&serde_json::to_string(&on).unwrap()).unwrap();
        assert!(back.split_score);
    }
}
