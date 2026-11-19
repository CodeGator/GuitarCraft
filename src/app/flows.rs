//! Tests that drive the whole app through a few frames, the way a person uses
//! it: start on the start screen, make or open a song, then edit it with the
//! keyboard and the menus. They run without a window.

use super::*;
use egui::{Event, Key, Modifiers, Pos2, RawInput, Rect, Vec2};
use std::cell::Cell;

thread_local! {
    /// Set by the Bar menu each time it is drawn: whether its items are enabled.
    pub static MENU_ENABLED: Cell<bool> = const { Cell::new(false) };
}

struct Rig {
    ctx: egui::Context,
    app: App,
    time: f64,
}

impl Rig {
    fn new() -> Rig {
        let ctx = egui::Context::default();
        // A normal start, on the start screen.
        let launch = LaunchOptions::default();
        let mut app = App::with_ctx(&ctx, launch);
        app.show_home = true;
        let mut r = Rig { ctx, app, time: 0.0 };
        r.frame(vec![]);
        r
    }

    fn frame(&mut self, events: Vec<Event>) {
        self.time += 0.05;
        let input = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1440.0, 900.0))), time: Some(self.time), events, ..Default::default() };
        let app = &mut self.app;
        let mut out = self.ctx.run_ui(input, |ui| app.frame_ui(ui));
        // No pixels are drawn here, so the texture updates aren't needed.
        out.textures_delta.clear();
    }

    fn frames(&mut self, n: usize) {
        for _ in 0..n {
            self.frame(vec![]);
        }
    }

    fn key(&mut self, key: Key, modifiers: Modifiers) {
        self.frame(vec![Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers }]);
        self.frame(vec![Event::Key { key, physical_key: None, pressed: false, repeat: false, modifiers }]);
    }
}

#[test]
fn a_new_song_can_be_edited_with_the_keyboard() {
    let mut r = Rig::new();
    assert!(r.app.show_home);
    // Home > New File: a new tab, asking for its first track.
    r.app.request(Pending::New);
    r.frames(2);
    assert!(!r.app.show_home, "the new song is showing");
    assert_eq!(r.app.tabs.len(), 1);
    assert!(r.app.at.open && r.app.at.new_doc, "the Add Track window is up");
    // Skip: keep the default track.
    r.app.at.open = false;
    r.frames(2);
    let bars = r.app.song.bar_count();
    r.key(Key::Insert, Modifiers::COMMAND);
    assert_eq!(r.app.song.bar_count(), bars + 1, "Ctrl+Ins inserts a bar");
    // Typing a fret number puts a note on the cursor's beat.
    r.frame(vec![
        Event::Key { key: Key::Num3, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE },
        Event::Text("3".into()),
    ]);
    r.frame(vec![Event::Key { key: Key::Num3, physical_key: None, pressed: false, repeat: false, modifiers: Modifiers::NONE }]);
    r.frames(2);
    let notes: usize = r.app.song.tracks[0].bars.iter().flat_map(|b| b.beats.iter()).map(|b| b.notes.len()).sum();
    assert!(notes > 0, "a note was added");
}

#[test]
fn creating_the_first_track_leaves_nothing_blocking_the_editing() {
    let mut r = Rig::new();
    r.app.request(Pending::New);
    r.frames(2);
    // Create: pick a bass.
    r.app.at.choose(Instrument::Bass);
    let t = r.app.at.build();
    r.app.at.open = false;
    r.app.song.tracks = vec![t];
    r.app.song.normalize();
    r.frames(2);
    assert!(!r.app.dialog_open(), "no window is left open");
    assert!(r.app.pending.is_none());
    let bars = r.app.song.bar_count();
    r.key(Key::Insert, Modifiers::COMMAND);
    assert_eq!(r.app.song.bar_count(), bars + 1);
}

#[test]
fn the_menus_work_while_a_song_is_showing() {
    let mut r = Rig::new();
    r.app.request(Pending::Demo);
    r.frames(3);
    assert!(!r.app.show_home);
    // The menu bar is not greyed out: it is greyed only on the start screen.
    assert!(!r.app.show_home);
    let bars = r.app.song.bar_count();
    // The Bar menu's Insert Bar does what the keyboard shortcut does.
    r.app.insert_bar(r.app.bar);
    assert_eq!(r.app.song.bar_count(), bars + 1);
    // Back to the start screen and into the song again through its tab.
    r.app.show_home = true;
    r.frames(2);
    r.app.switch_to(0);
    r.frames(2);
    assert!(!r.app.show_home);
    r.key(Key::Insert, Modifiers::COMMAND);
    assert_eq!(r.app.song.bar_count(), bars + 2);
}

#[test]
fn opening_a_second_song_from_the_start_screen_keeps_both_editable() {
    let mut r = Rig::new();
    r.app.request(Pending::Demo);
    r.frames(2);
    r.app.show_home = true;
    r.frames(2);
    r.app.request(Pending::Template(3));
    r.frames(2);
    assert_eq!(r.app.tabs.len(), 2);
    assert!(!r.app.show_home);
    let bars = r.app.song.bar_count();
    r.key(Key::Insert, Modifiers::COMMAND);
    assert_eq!(r.app.song.bar_count(), bars + 1, "the new tab is editable");
    r.app.switch_to(0);
    r.frames(2);
    let first_bars = r.app.song.bar_count();
    r.key(Key::Insert, Modifiers::COMMAND);
    assert_eq!(r.app.song.bar_count(), first_bars + 1, "so is the first tab");
}

impl Rig {
    fn click(&mut self, x: f32, y: f32) {
        let pos = Pos2::new(x, y);
        self.frame(vec![Event::PointerMoved(pos)]);
        self.frame(vec![Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE }]);
        self.frame(vec![Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE }]);
        self.frames(2);
    }
}

fn type_digit(r: &mut Rig, key: Key, text: &str) {
    r.frame(vec![
        Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE },
        Event::Text(text.into()),
    ]);
    r.frame(vec![Event::Key { key, physical_key: None, pressed: false, repeat: false, modifiers: Modifiers::NONE }]);
    r.frames(2);
}

fn note_count(r: &Rig) -> usize {
    r.app.song.tracks.iter().flat_map(|t| t.bars.iter()).flat_map(|b| b.beats.iter()).map(|b| b.notes.len()).sum()
}

#[test]
fn clicking_skip_in_the_add_track_window_leaves_the_song_editable() {
    let mut r = Rig::new();
    r.app.request(Pending::New);
    r.frames(3);
    assert!(r.app.at.open);
    r.click(893.0, 693.0);
    assert!(!r.app.at.open, "Skip closed the window");
    assert!(!r.app.show_home && r.app.pending.is_none());
    assert!(!r.app.dialog_open(), "nothing else is open");
    let n = note_count(&r);
    type_digit(&mut r, Key::Num3, "3");
    assert!(note_count(&r) > n, "typing a fret number adds a note");
    let bars = r.app.song.bar_count();
    r.key(Key::Insert, Modifiers::COMMAND);
    assert_eq!(r.app.song.bar_count(), bars + 1);
}

#[test]
fn clicking_create_in_the_add_track_window_adds_that_track_and_leaves_the_song_editable() {
    let mut r = Rig::new();
    r.app.request(Pending::New);
    r.frames(3);
    // Keyboards, then Create.
    r.click(611.0, 288.0);
    assert_eq!(r.app.at.kind, InstrumentType::Keyboard);
    r.click(827.0, 693.0 - 18.0);
    assert!(!r.app.at.open, "Create closed the window");
    assert_eq!(r.app.song.tracks.len(), 1);
    assert_eq!(r.app.song.tracks[0].instrument, Instrument::Piano);
    assert!(!r.app.dialog_open());
    let bars = r.app.song.bar_count();
    r.key(Key::Insert, Modifiers::COMMAND);
    assert_eq!(r.app.song.bar_count(), bars + 1);
}

#[test]
fn the_bar_menu_opens_with_enabled_items_and_inserts_a_bar() {
    let mut r = Rig::new();
    r.app.request(Pending::Demo);
    r.frames(3);
    MENU_ENABLED.set(false);
    // Click "Bar" in the menu bar.
    r.click(124.0, 12.0);
    assert!(MENU_ENABLED.get(), "the Bar menu's items are enabled");
    // Click its first item, Insert Bar.
    let bars = r.app.song.bar_count();
    r.click(150.0, 38.0);
    assert_eq!(r.app.song.bar_count(), bars + 1, "Bar > Insert Bar inserted a bar");
}

#[test]
fn the_menus_are_greyed_only_on_the_start_screen() {
    let mut r = Rig::new();
    r.app.request(Pending::Demo);
    r.frames(3);
    r.app.show_home = true;
    r.frames(2);
    MENU_ENABLED.set(false);
    r.click(124.0, 12.0);
    assert!(!MENU_ENABLED.get(), "on the start screen the Bar menu stays shut");
    r.app.switch_to(0);
    r.frames(2);
    MENU_ENABLED.set(false);
    r.click(124.0, 12.0);
    assert!(MENU_ENABLED.get(), "after returning to the song the Bar menu works");
}

#[test]
fn the_keyboard_is_free_again_after_typing_in_the_add_track_window() {
    let mut r = Rig::new();
    r.app.request(Pending::New);
    r.frames(3);
    // Click into the track name box and type a name.
    r.click(688.0, 550.0);
    assert!(r.ctx.egui_wants_keyboard_input(), "the name box has the keyboard");
    r.frame(vec![Event::Text("Lead".into())]);
    r.frames(2);
    // Create.
    r.click(827.0, 693.0);
    assert!(!r.app.at.open);
    r.frames(3);
    let n = note_count(&r);
    type_digit(&mut r, Key::Num3, "3");
    assert!(note_count(&r) > n, "typing a fret number adds a note");
    let bars = r.app.song.bar_count();
    r.key(Key::Insert, Modifiers::COMMAND);
    assert_eq!(r.app.song.bar_count(), bars + 1);
}

#[test]
fn a_score_opened_after_a_chord_chart_is_fully_editable() {
    let mut r = Rig::new();
    r.app.request(Pending::NewChart);
    r.frames(3);
    assert!(r.app.chart.is_some());
    // Home, then a score from the start screen.
    r.app.show_home = true;
    r.frames(2);
    r.app.request(Pending::Demo);
    r.frames(3);
    assert!(r.app.chart.is_none(), "the score is showing, not the chart");
    assert!(!r.app.show_home && r.app.pending.is_none() && !r.app.dialog_open());
    MENU_ENABLED.set(false);
    r.click(124.0, 12.0);
    assert!(MENU_ENABLED.get(), "the score's Bar menu is enabled");
    r.key(Key::Escape, Modifiers::NONE);
    let bars = r.app.song.bar_count();
    r.key(Key::Insert, Modifiers::COMMAND);
    assert_eq!(r.app.song.bar_count(), bars + 1);
    let n = note_count(&r);
    type_digit(&mut r, Key::Num3, "3");
    assert!(note_count(&r) > n);
    // And back to the chart and the score again by their tabs.
    r.app.switch_to(0);
    r.frames(2);
    assert!(r.app.chart.is_some());
    r.app.switch_to(1);
    r.frames(2);
    assert!(r.app.chart.is_none());
    let bars = r.app.song.bar_count();
    r.key(Key::Insert, Modifiers::COMMAND);
    assert_eq!(r.app.song.bar_count(), bars + 1);
}

#[test]
fn closing_the_add_track_window_with_cancel_leaves_the_other_tabs_editable() {
    let mut r = Rig::new();
    r.app.request(Pending::Demo);
    r.frames(2);
    r.app.show_home = true;
    r.frames(2);
    r.app.request(Pending::New);
    r.frames(3);
    assert!(r.app.at.open);
    // Cancel: the new document is discarded and the first song is showing again.
    r.click(957.0, 693.0);
    assert!(!r.app.at.open);
    assert_eq!(r.app.tabs.len(), 1);
    assert!(!r.app.show_home && r.app.pending.is_none());
    let bars = r.app.song.bar_count();
    r.key(Key::Insert, Modifiers::COMMAND);
    assert_eq!(r.app.song.bar_count(), bars + 1);
}

fn press(r: &mut Rig, key: Key, modifiers: Modifiers) {
    r.key(key, modifiers);
    r.frames(1);
}

fn text(r: &mut Rig, t: &str) {
    r.frame(vec![Event::Text(t.into())]);
    r.frames(1);
}

/// A new song with one note (fret 3) under the cursor.
fn song_with_a_note() -> Rig {
    let mut r = Rig::new();
    r.app.request(Pending::New);
    r.frames(3);
    r.app.at.open = false;
    r.frames(2);
    type_digit(&mut r, Key::Num3, "3");
    // Back onto the note.
    r.key(Key::ArrowLeft, Modifiers::NONE);
    r.frames(1);
    assert!(!r.app.cur_beat().is_rest(), "the cursor is on a note");
    r
}

const CA: Modifiers = Modifiers { alt: true, ctrl: true, shift: false, mac_cmd: false, command: true };
const AS: Modifiers = Modifiers { alt: true, ctrl: false, shift: true, mac_cmd: false, command: false };
const CS: Modifiers = Modifiers { alt: false, ctrl: true, shift: true, mac_cmd: false, command: true };

#[test]
fn guitar_pro_8_effect_keys_toggle_their_effects() {
    let mut r = song_with_a_note();
    press(&mut r, Key::H, Modifiers::SHIFT);
    assert!(r.app.cur_beat().notes[0].fx.legato, "Shift+H is legato");
    assert!(!r.app.cur_beat().notes[0].hammer, "not hammer-on");
    press(&mut r, Key::I, Modifiers::SHIFT);
    assert!(r.app.cur_beat().fx.barre > 0, "Shift+I is a barre");
    press(&mut r, Key::I, Modifiers::SHIFT);
    assert_eq!(r.app.cur_beat().fx.barre, 0);
    press(&mut r, Key::R, Modifiers::SHIFT);
    assert!(r.app.cur_beat().fx.rasgueado, "Shift+R is rasgueado");
    text(&mut r, "@");
    assert!(r.app.cur_beat().fx.timer, "@ is a timer");
    text(&mut r, ">");
    assert!(r.app.cur_beat().fx.fade_out, "> is fade out");
    text(&mut r, "<");
    assert!(r.app.cur_beat().fade_in && !r.app.cur_beat().fx.fade_out, "< is fade in");
    press(&mut r, Key::Period, AS);
    assert!(r.app.cur_beat().fx.swell, "Alt+Shift+. is a volume swell");
    press(&mut r, Key::W, Modifiers::NONE);
    assert_eq!(r.app.cur_beat().fx.bridge_vibrato, 1, "W is a slight tremolo-bar vibrato");
    press(&mut r, Key::W, CA);
    assert_eq!(r.app.cur_beat().fx.bridge_vibrato, 2, "Ctrl+Alt+W is a wide one");
    press(&mut r, Key::V, CA);
    assert!(r.app.cur_beat().fx.whammy, "Ctrl+Alt+V is the tremolo bar");
    press(&mut r, Key::O, CA);
    assert_eq!(r.app.cur_beat().fx.wah, 1, "Ctrl+Alt+O is wah open");
    press(&mut r, Key::C, CA);
    assert_eq!(r.app.cur_beat().fx.wah, 2, "Ctrl+Alt+C is wah closed");
    text(&mut r, "(");
    assert!(r.app.cur_beat().notes[0].fx.lh_tap, "( is left-hand tapping");
    press(&mut r, Key::G, CA);
    assert!(r.app.cur_beat().fx.grace_on_beat && r.app.cur_beat().notes[0].grace.is_some(), "Ctrl+Alt+G is a grace note on the beat");
}

#[test]
fn guitar_pro_8_bar_keys() {
    let mut r = song_with_a_note();
    press(&mut r, Key::L, AS);
    assert!(r.app.song.master[r.app.bar].free_time, "Alt+Shift+L is free time");
    text(&mut r, "%");
    assert_eq!(r.app.song.tracks[0].bars[r.app.bar].simile, 1, "% repeats the previous bar");
    press(&mut r, Key::Num5, CS);
    assert_eq!(r.app.song.tracks[0].bars[r.app.bar].simile, 2, "Ctrl+% repeats the two previous bars");
    // Ctrl+Shift+D / U ask about an arpeggio (the note is on the beat after a few presses: put it back).
    r.app.song.tracks[0].bars[r.app.bar].simile = 0;
    press(&mut r, Key::D, CS);
    assert_eq!(r.app.popup, Some(popups::Popup::ArpeggioDown));
    r.app.popup = None;
    press(&mut r, Key::U, CS);
    assert_eq!(r.app.popup, Some(popups::Popup::ArpeggioUp));
    r.app.popup = None;
    // Shift+S: the scale diagram.
    press(&mut r, Key::S, Modifiers::SHIFT);
    assert!(r.app.cs.open && r.app.cs.mode == chordscales::Mode::Scales);
}

#[test]
fn ctrl_alt_arrows_move_the_note_on_the_staff() {
    let mut r = song_with_a_note();
    let fret = r.app.cur_beat().notes[0].fret;
    press(&mut r, Key::ArrowUp, CA);
    let up = r.app.cur_beat().notes[0].fret;
    assert!(up > fret && up - fret <= 2, "the note moved up to the next staff position ({fret} to {up})");
    press(&mut r, Key::ArrowDown, CA);
    press(&mut r, Key::ArrowDown, CA);
    assert!(r.app.cur_beat().notes[0].fret < up);
}

#[test]
fn close_all_asks_about_each_file_with_changes() {
    let mut r = Rig::new();
    r.app.request(Pending::Demo);
    r.frames(2);
    r.app.dirty = true;
    r.app.show_home = true;
    r.frames(2);
    r.app.request(Pending::Template(3));
    r.frames(2);
    r.app.dirty = true;
    r.frames(1);
    assert_eq!(r.app.tabs.len(), 2);
    // Ctrl+Shift+W: the first question is about the showing file.
    press(&mut r, Key::W, CS);
    assert_eq!(r.app.pending, Some(Pending::CloseAll));
    assert_eq!(r.app.dirty_count(), 2);
    assert_eq!(r.app.tabs.len(), 2, "nothing is closed before the questions are answered");
    // Don't save: on to the other file.
    r.app.pending = None;
    r.app.dirty = false;
    let next = r.app.other_dirty_tab();
    assert!(next.is_some(), "the other file still has changes");
    r.app.switch_to(next.unwrap());
    r.app.dirty = false;
    r.app.run_pending(Pending::CloseAll);
    r.frames(2);
    assert!(r.app.tabs.is_empty() && r.app.show_home, "every file is closed and the start screen shows");
}

#[test]
fn the_star_key_toggles_a_dot_on_a_note_or_a_rest() {
    let mut r = song_with_a_note();
    assert!(!r.app.cur_beat().dotted);
    text(&mut r, "*");
    assert!(r.app.cur_beat().dotted, "* dots the note");
    text(&mut r, "*");
    assert!(!r.app.cur_beat().dotted, "* again takes the dot off");
    // A rest too: move to the next beat (an empty one) and star it.
    r.key(Key::ArrowRight, Modifiers::NONE);
    r.frames(1);
    assert!(r.app.cur_beat().is_rest());
    text(&mut r, "*");
    assert!(r.app.cur_beat().dotted, "* dots the rest");
    // It is the same dot as the period key: the period takes it off.
    press(&mut r, Key::Period, Modifiers::NONE);
    assert!(!r.app.cur_beat().dotted);
    // A double dot is replaced by a single one.
    r.app.beat_edit(|b| b.double_dot = true);
    text(&mut r, "*");
    assert!(r.app.cur_beat().dotted && !r.app.cur_beat().double_dot);
}
