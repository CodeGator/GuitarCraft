//! Keyboard shortcuts, following the original's layout where it makes sense.

use super::popups::Popup;
use super::shortcuts::{Act, Binding, Place, find};
use super::{App, InspectorTab, Pending};
use crate::model::*;
use egui::{Key, Modifiers};

/// Whether a key is physically held down right now. Windows turns Ctrl+Insert
/// into a "copy", Ctrl+Shift+Insert into a "copy" with Shift held, and
/// Ctrl+Shift+Delete into a "cut", and the window library then drops the key
/// itself, so these are the only way to tell them from Ctrl+C and Ctrl+X.
fn native_key_down(key: egui::Key) -> bool {
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_DELETE, VK_INSERT};
        let vk = match key {
            egui::Key::Insert => VK_INSERT,
            egui::Key::Delete => VK_DELETE,
            _ => return false,
        };
        // SAFETY: GetAsyncKeyState only reads the keyboard state.
        (unsafe { GetAsyncKeyState(vk as i32) } as u16 & 0x8000) != 0
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = key;
        false
    }
}

/// Whether Windows (through the window library) swallows this key press into a
/// copy, cut or paste event and never passes the key itself on: Ctrl+Insert
/// (copy), Shift+Insert (paste), Shift+Delete (cut), and their combinations.
/// These are Guitar Pro's Insert Bar, Edit Section, Add Track, Delete the
/// Beats and Delete Track shortcuts.
pub(super) fn swallowed_by_clipboard(key: Key, ctrl: bool, shift: bool) -> bool {
    match key {
        Key::Insert => ctrl || shift,
        Key::Delete => shift,
        _ => false,
    }
}

impl App {
    /// Keeps the Insert and Delete state up to date while keys are ignored.
    fn swallowed_keys_idle(&mut self) {
        self.ins_down = native_key_down(Key::Insert);
        self.del_down = native_key_down(Key::Delete);
    }

    /// Insert or Delete pressed this frame whose key event Windows swallowed:
    /// run its shortcut here, and say that this frame's copy, cut and paste
    /// events were really these keys.
    fn swallowed_keys(&mut self, ctx: &egui::Context, typing: bool) -> bool {
        let (ctrl, shift, mods) = ctx.input(|i| (i.modifiers.ctrl || i.modifiers.command, i.modifiers.shift, i.modifiers));
        let (ins, del) = (native_key_down(Key::Insert), native_key_down(Key::Delete));
        let mut pressed = vec![];
        if ins && !self.ins_down && swallowed_by_clipboard(Key::Insert, ctrl, shift) {
            pressed.push(Key::Insert);
        }
        if del && !self.del_down && swallowed_by_clipboard(Key::Delete, ctrl, shift) {
            pressed.push(Key::Delete);
        }
        self.ins_down = ins;
        self.del_down = del;
        if pressed.is_empty() || typing {
            return false;
        }
        let now = ctx.input(|i| i.time);
        for k in pressed {
            self.key(k, mods, now);
        }
        true
    }

    pub(super) fn handle_keys(&mut self, ctx: &egui::Context) {
        let typing = ctx.egui_wants_keyboard_input();
        // Always look at Insert and Delete, so a press during a question isn't run later.
        let swallowed = if self.pending.is_some() || self.dialog_open() {
            self.swallowed_keys_idle();
            return;
        } else {
            self.swallowed_keys(ctx, typing)
        };
        let events = ctx.input(|i| i.events.clone());
        let now = ctx.input(|i| i.time);
        let shift = ctx.input(|i| i.modifiers.shift);
        for ev in events {
            // Copy, cut and paste events that were really Insert or Delete shortcuts.
            if swallowed && matches!(ev, egui::Event::Copy | egui::Event::Cut | egui::Event::Paste(_)) {
                continue;
            }
            match ev {
                // The window library turns Ctrl+C, X and V into these events (the key
                // itself never arrives): they are looked up as those keys.
                egui::Event::Copy if !typing => self.key(Key::C, clip_mods(shift), now),
                egui::Event::Cut if !typing => self.key(Key::X, clip_mods(shift), now),
                egui::Event::Paste(_) if !typing => self.key(Key::V, clip_mods(shift), now),
                egui::Event::Text(t) if !typing => {
                    for c in t.chars() {
                        if let Some(act) = find(Place::Score, &Binding::typed(c)) {
                            self.run_act(act);
                            self.clamp_cursor();
                        }
                    }
                }
                egui::Event::Key { key, pressed: true, modifiers, .. } => {
                    if typing {
                        // In a text field only Ctrl, Alt and function keys for the
                        // file commands work (typing is never taken).
                        let b = Binding::of(key, modifiers);
                        if let Some(act) = find(Place::Score, &b)
                            && b.works_while_typing()
                            && WHILE_TYPING.contains(&act)
                        {
                            self.run_act(act);
                        }
                        continue;
                    }
                    self.key(key, modifiers, now);
                }
                _ => {}
            }
        }
        // Copying puts a mark on the system clipboard, so pasting later is offered.
        if std::mem::take(&mut self.clip_mark) {
            ctx.copy_text("guitarcraft-beats".into());
        }
    }

    /// A key pressed on the score (not in a text field): its action from the keymap,
    /// or a fret number.
    pub(super) fn key(&mut self, key: Key, m: Modifiers, now: f64) {
        let digit = digit_of(key);
        if digit.is_none() {
            self.last_digit = None;
        }
        // Esc always leaves full screen (and also runs Stop, its shortcut).
        if key == Key::Escape && self.fullscreen {
            self.toggle_fullscreen();
        }
        // 0-9 type fret numbers (or pick the string in notation); they can't be changed.
        if let Some(d) = digit
            && !(m.command || m.ctrl || m.alt || m.shift)
        {
            if self.std_cursor.is_some() {
                self.std_digit(d);
            } else {
                self.type_digit(d, now);
            }
            self.clamp_cursor();
            return;
        }
        if let Some(act) = find(Place::Score, &Binding::of(key, m)) {
            self.run_act(act);
            self.clamp_cursor();
        }
    }

    /// Runs an action of the shortcut list (from a key, or anything else).
    pub(super) fn run_act(&mut self, act: Act) {
        use Act::*;
        match act {
            // ----- Files -----
            New => self.request(Pending::New),
            NewChart => self.request(Pending::NewChart),
            Open => self.request(Pending::Open),
            Browse => self.show_home = true,
            Close => self.request(Pending::Close),
            CloseAll => self.request(Pending::CloseAll),
            Save => self.save(false),
            SaveAs => self.save(true),
            Print => self.print_song(),
            Prefs => self.open_prefs(),
            // Alt+F4 is the window's own: the system closes it.
            Quit => {}
            // ----- Editing -----
            Undo => self.do_undo(),
            Redo => self.do_redo(),
            Cut => {
                self.copy();
                self.last_copy_all = false;
                self.delete_at_cursor();
                self.clip_mark = true;
            }
            Copy => {
                self.copy();
                self.last_copy_all = false;
                self.clip_mark = true;
            }
            Paste => self.paste_any(),
            AllTrackCut => {
                self.all_track_cut();
                self.clip_mark = true;
            }
            AllTrackCopy => {
                self.all_track_copy();
                self.clip_mark = true;
            }
            SpecialPaste => self.open_special_paste(),
            SelectAll => {
                self.sel_anchor = Some((0, 0));
                self.bar = self.song.bar_count() - 1;
                self.beat = self.last_beat();
            }
            CommandPalette => {
                self.cmd_open = !self.cmd_open;
                self.cmd_text.clear();
            }
            GoTo => self.goto_open = true,
            CheckBars => self.check_bar_duration(),
            DeleteNote if self.std_cursor.is_some() => self.std_delete(),
            DeleteNote => self.delete_at_cursor(),
            DeleteBeats => self.delete_beats(),
            InsertBeat => self.insert_beat(),
            CopyLastBeat => self.copy_last_beat(),
            // ----- Moving around -----
            MoveRight => self.move_right(false),
            MoveLeft => self.move_left(false),
            StringUp if self.std_cursor.is_some() => self.std_step(1),
            StringDown if self.std_cursor.is_some() => self.std_step(-1),
            StringUp => self.string = self.string.saturating_sub(1),
            StringDown => self.string = (self.string + 1).min(self.tr().strings() - 1),
            BarStart => self.beat = 0,
            BarEnd => self.beat = self.last_beat(),
            NextBar => self.goto_bar(self.bar + 1),
            PrevBar => self.goto_bar(self.bar.saturating_sub(1)),
            FirstBar => {
                self.stop();
                self.goto_bar(0);
            }
            LastBar => {
                self.stop();
                self.goto_bar(self.song.bar_count() - 1);
            }
            ForwardFourBars => self.goto_bar(self.bar + 4),
            BackFourBars => self.goto_bar(self.bar.saturating_sub(4)),
            NextStaff => self.tab_key(1),
            PrevStaff => self.tab_key(-1),
            NextSection => self.goto_section(1),
            PrevSection => self.goto_section(-1),
            StepForward => self.step(1),
            StepBack => self.step(-1),
            // Typed with the digit itself (see `key`).
            Digits => {}
            // ----- Selecting -----
            SelectNextBeat => self.move_right(true),
            SelectPrevBeat => self.move_left(true),
            SelectToBarStart => {
                self.update_anchor(true);
                self.beat = 0;
            }
            SelectToBarEnd => {
                self.update_anchor(true);
                self.beat = self.last_beat();
            }
            SelectNextBar => {
                self.update_anchor(true);
                self.bar = (self.bar + 1).min(self.song.bar_count() - 1);
                self.beat = self.last_beat();
            }
            SelectPrevBar => {
                self.update_anchor(true);
                self.bar = self.bar.saturating_sub(1);
                self.beat = 0;
            }
            SelectToStart => {
                self.update_anchor(true);
                self.bar = 0;
                self.beat = 0;
            }
            SelectToEnd => {
                self.update_anchor(true);
                self.bar = self.song.bar_count() - 1;
                self.beat = self.last_beat();
            }
            // ----- Notes and rhythm -----
            Shorter => self.shorter_longer(true),
            Longer => self.shorter_longer(false),
            Dotted => self.beat_edit(|b| {
                b.dotted = !b.dotted;
                b.double_dot = false;
            }),
            DoubleDotted => self.beat_edit(|b| {
                b.double_dot = !b.double_dot;
                b.dotted = false;
            }),
            Triplet => self.beat_edit(|b| b.tuplet = if b.tuplet == 3 { 0 } else { 3 }),
            Rest => self.edit(|s, t, b, j, _| s.tracks[t].bars[b].beats[j].notes.clear()),
            TieNote => self.toggle_note_flag("Tie", |n| &mut n.tie),
            TieBeat => self.tie_beat(),
            GraceBefore => self.note_edit(|x| {
                x.grace = if x.grace.is_some() { None } else { Some(x.fret.saturating_sub(2)) }
            }),
            GraceOnBeat => {
                // A grace note on the beat (G alone puts it before the beat).
                self.note_edit(|x| {
                    if x.grace.is_none() {
                        x.grace = Some(x.fret.saturating_sub(2));
                    }
                });
                self.beat_edit(|b| b.fx.grace_on_beat = true);
            }
            Ghost => self.toggle_note_flag("Ghost note", |n| &mut n.ghost),
            DeadNote => self.toggle_note_flag("Dead note", |n| &mut n.dead),
            Accent => self.toggle_note_flag("Accent", |n| &mut n.accent),
            HeavyAccent => self.toggle_note_flag("Heavy accent", |n| &mut n.heavy_accent),
            Staccato => self.toggle_note_flag("Staccato", |n| &mut n.staccato),
            SemitoneUp => self.transpose(1),
            SemitoneDown => self.transpose(-1),
            OctaveUp => self.shift_octave(1),
            OctaveDown => self.shift_octave(-1),
            NoteStringUp => self.move_note_string(-1),
            NoteStringDown => self.move_note_string(1),
            MoveOnStaffUp => self.move_on_staff(1),
            MoveOnStaffDown => self.move_on_staff(-1),
            SpellFlats => self.note_edit(|x| x.spell = 2),
            SpellAuto => self.note_edit(|x| x.spell = 0),
            SpellSharps => self.note_edit(|x| x.spell = 1),
            CycleSpelling => self.note_edit(|x| x.spell = (x.spell + 1) % 3),
            Chord => self.open_chord_tool(),
            Text => self.open_popup(Popup::Text),
            Fermata => self.beat_edit(|b| b.fermata = !b.fermata),
            // ----- Effects -----
            Hammer => self.toggle_note_flag("Hammer-on / pull-off", |n| &mut n.hammer),
            Legato => self.toggle_note_flag("Legato", |n| &mut n.fx.legato),
            Slide => self.toggle_note_flag("Slide", |n| &mut n.slide),
            Bend => self.open_popup(Popup::Bend),
            Vibrato => self.toggle_note_flag("Vibrato", |n| &mut n.vibrato),
            WhammyVibrato => self.beat_edit(|b| b.fx.bridge_vibrato = if b.fx.bridge_vibrato == 1 { 0 } else { 1 }),
            WideWhammyVibrato => self.beat_edit(|b| b.fx.bridge_vibrato = if b.fx.bridge_vibrato == 2 { 0 } else { 2 }),
            TremoloBar => self.beat_edit(|b| b.fx.whammy = !b.fx.whammy),
            Trill => self.open_popup(Popup::Trill),
            TremoloPicking => self.beat_edit(|b| b.tremolo = !b.tremolo),
            PalmMuteNote => self.toggle_note_flag("Palm mute", |n| &mut n.palm_mute),
            PalmMuteBeat => {
                let on = !self.cur_beat().notes.iter().all(|n| n.palm_mute);
                self.edit(|s, t, b, j, _| {
                    for n in &mut s.tracks[t].bars[b].beats[j].notes {
                        n.palm_mute = on;
                    }
                });
            }
            LetRing => self.toggle_note_flag("Let ring", |n| &mut n.let_ring),
            NaturalHarmonic => self.toggle_note_flag("Natural harmonic", |n| &mut n.harmonic),
            ArtificialHarmonic => self.toggle_note_flag("Artificial harmonic", |n| &mut n.pinch_harmonic),
            Tapping => self.beat_edit(|b| b.technique = if b.technique == 1 { 0 } else { 1 }),
            LhTapping => self.toggle_note_flag("Left-hand tapping", |n| &mut n.fx.lh_tap),
            Slap => self.beat_edit(|b| b.technique = if b.technique == 2 { 0 } else { 2 }),
            PickDown => self.beat_edit(|b| b.pick = if b.pick == 1 { 0 } else { 1 }),
            PickUp => self.beat_edit(|b| b.pick = if b.pick == 2 { 0 } else { 2 }),
            BrushDown => self.beat_edit(|b| b.stroke = if b.stroke == Stroke::Down { Stroke::None } else { Stroke::Down }),
            BrushUp => self.beat_edit(|b| b.stroke = if b.stroke == Stroke::Up { Stroke::None } else { Stroke::Up }),
            ArpeggioDown => self.open_popup(Popup::ArpeggioDown),
            ArpeggioUp => self.open_popup(Popup::ArpeggioUp),
            Barre => self.beat_edit(|b| {
                // A barre across the beat's lowest fretted fret, or none.
                b.fx.barre = if b.fx.barre > 0 {
                    0
                } else {
                    b.notes.iter().map(|n| n.fret).filter(|f| *f > 0).min().unwrap_or(1).min(12)
                };
            }),
            Rasgueado => self.beat_edit(|b| b.fx.rasgueado = !b.fx.rasgueado),
            FadeIn => self.beat_edit(|b| {
                b.fade_in = !b.fade_in;
                b.fx.fade_out = false;
                b.fx.swell = false;
            }),
            FadeOut => self.beat_edit(|b| {
                b.fx.fade_out = !b.fx.fade_out;
                b.fade_in = false;
                b.fx.swell = false;
            }),
            Swell => self.beat_edit(|b| {
                b.fx.swell = !b.fx.swell;
                b.fx.fade_out = false;
                b.fade_in = false;
            }),
            WahOpen => self.beat_edit(|b| b.fx.wah = if b.fx.wah == 1 { 0 } else { 1 }),
            WahClosed => self.beat_edit(|b| b.fx.wah = if b.fx.wah == 2 { 0 } else { 2 }),
            Timer => self.beat_edit(|b| b.fx.timer = !b.fx.timer),
            ScaleDiagram => self.open_chord_scales(super::chordscales::Mode::Scales),
            // ----- Bars and sections -----
            InsertBar => self.insert_bar(self.bar),
            DeleteBar => self.delete_bar(self.bar),
            KeySig => self.open_popup(Popup::KeySig),
            TimeSig => self.open_popup(Popup::TimeSig),
            TripletFeel => self.bar_edit(|mb| mb.triplet_feel = !mb.triplet_feel),
            FreeTime => self.bar_edit(|mb| mb.free_time = !mb.free_time),
            RepeatOpen => self.bar_edit(|m| m.repeat_open = !m.repeat_open),
            RepeatClose => self.bar_edit(|m| m.repeat_close = if m.repeat_close > 0 { 0 } else { 2 }),
            RepeatBar => self.edit(|s, t, b, _, _| {
                let bar = &mut s.tracks[t].bars[b];
                bar.simile = if bar.simile == 1 { 0 } else { 1 };
            }),
            RepeatTwoBars => self.edit(|s, t, b, _, _| {
                let bar = &mut s.tracks[t].bars[b];
                bar.simile = if bar.simile == 2 { 0 } else { 2 };
            }),
            ForceBreak => self.bar_edit(|mb| mb.line_break = !mb.line_break),
            Directions => self.dir_open = true,
            Section => self.edit_section(),
            Automations => self.open_automations(self.bar),
            // ----- Voices -----
            Voice1 => self.set_voice(0),
            Voice2 => self.set_voice(1),
            Voice3 => self.set_voice(2),
            Voice4 => self.set_voice(3),
            MoveToVoice1 => self.move_to_voice(0),
            MoveToVoice2 => self.move_to_voice(1),
            MoveToVoice3 => self.move_to_voice(2),
            MoveToVoice4 => self.move_to_voice(3),
            MoveVoices => {
                self.vfrom = self.voice;
                self.vto = if self.voice == 0 { 1 } else { 0 };
                self.vrange = if self.selection().is_some() {
                    super::voices::VoiceRange::Selection
                } else {
                    super::voices::VoiceRange::Bar
                };
                self.voices_open = true;
            }
            // ----- Tracks -----
            AddTrack => self.open_add_track(false),
            DeleteTrack => self.remove_track(),
            NextTrack => self.track = (self.track + 1).min(self.song.tracks.len() - 1),
            PrevTrack => self.track = self.track.saturating_sub(1),
            // ----- Playback -----
            PlayPause => self.toggle_play(),
            PlayFromStart => self.play_from_start(),
            Stop => {
                if self.playing {
                    self.stop();
                }
                self.sel_anchor = None;
            }
            Loop => self.looping = !self.looping,
            // ----- Windows and view -----
            Help => self.guide_open = true,
            ShowPalette => self.show_palette = !self.show_palette,
            ShowGlobalView => self.show_tracks = !self.show_tracks,
            SongInspector => self.toggle_inspector(InspectorTab::Song),
            TrackInspector => self.toggle_inspector(InspectorTab::Track),
            KeyboardView => self.show_keyboard = !self.show_keyboard,
            Multitrack => self.single_track = !self.single_track,
            Stylesheet => self.open_stylesheet(),
            ZoomIn => self.zoom_step(1),
            ZoomOut => self.zoom_step(-1),
            FullScreen => self.toggle_fullscreen(),
            NextTab => self.cycle_tab(1),
            PrevTab => self.cycle_tab(-1),
            // ----- Chord charts (run by the chart's own keys) -----
            ChartTranspose => {}
        }
    }

    /// The last beat of the current bar in the voice being edited.
    fn last_beat(&self) -> usize {
        self.song.tracks[self.track].bars[self.bar].voice(self.voice).len() - 1
    }

    /// F5 / F6: shows the song or track settings, or hides them if they are showing.
    fn toggle_inspector(&mut self, tab: InspectorTab) {
        if self.show_inspector && self.inspector == tab {
            self.show_inspector = false;
        } else {
            self.show_inspector = true;
            self.inspector = tab;
        }
    }
}

/// Actions whose keys work while a text field on the score has the keyboard.
const WHILE_TYPING: [Act; 9] = [Act::Save, Act::SaveAs, Act::Open, Act::Browse, Act::New, Act::NewChart, Act::Close, Act::Prefs, Act::CommandPalette];

/// The modifiers of a copy, cut or paste event: Ctrl, and Shift if it is held.
fn clip_mods(shift: bool) -> Modifiers {
    Modifiers { ctrl: true, command: true, shift, ..Modifiers::NONE }
}

fn digit_of(key: Key) -> Option<u8> {
    Some(match key {
        Key::Num0 => 0,
        Key::Num1 => 1,
        Key::Num2 => 2,
        Key::Num3 => 3,
        Key::Num4 => 4,
        Key::Num5 => 5,
        Key::Num6 => 6,
        Key::Num7 => 7,
        Key::Num8 => 8,
        Key::Num9 => 9,
        _ => return None,
    })
}

#[cfg(test)]
mod swallow_tests {
    use super::*;

    #[test]
    fn windows_hides_these_insert_and_delete_shortcuts_inside_clipboard_events() {
        // Ctrl+Insert (copy), Ctrl+Shift+Insert (copy), Shift+Insert (paste).
        assert!(swallowed_by_clipboard(Key::Insert, true, false));
        assert!(swallowed_by_clipboard(Key::Insert, true, true));
        assert!(swallowed_by_clipboard(Key::Insert, false, true));
        // Shift+Delete and Ctrl+Shift+Delete (cut).
        assert!(swallowed_by_clipboard(Key::Delete, false, true));
        assert!(swallowed_by_clipboard(Key::Delete, true, true));
        // Plain Insert and Delete, and Ctrl+Delete, arrive as keys.
        assert!(!swallowed_by_clipboard(Key::Insert, false, false));
        assert!(!swallowed_by_clipboard(Key::Delete, false, false));
        assert!(!swallowed_by_clipboard(Key::Delete, true, false));
        assert!(!swallowed_by_clipboard(Key::C, true, false));
    }
}
