# MIDI input and output: design

Status: proposal, 2026-11-22. Step 1 (ports and preferences) built 2026-11-22: `src/midi.rs`,
the two pickers in Preferences > Audio & MIDI, `midi_output` / `midi_input` saved in preferences,
and a live "messages received" count for the input. Step 2 (note previews also go to the MIDI output) built the same day and heard working through the Windows GS Wavetable Synth. Step 3 (playback events carry MIDI pitch, channel and program; `audio::MidiNote`) built the
same day, tested without hardware. Step 4 (playback to the MIDI output, built-in sound off by default while an output is open,
`midi_builtin_too` preference) built the same day, not yet heard by the author. Step 5 (bends as pitch bend: the four bend shapes, sent only when no other note of the channel
sounds with it; slides and vibrato not sent) built the same day, not yet heard. Step 6 (MIDI input: single notes entered at the cursor, then the cursor moves on; string and fret
chosen by `midi::string_and_fret`) built the same day, tested only with unit tests because no
MIDI keyboard was available. Step 7 (chords: notes within 60 ms form one beat, then the cursor moves on once; lowest notes
place first on the lowest frets) built the same day, unit tests only. Step 8 (README, in-app help, notes) done the same day.
Hand-tested afterwards (author, 2026-11-22, Morning Walk.gp): playback to MIDI, bends and chords
work. Slides and vibrato were not sent (the author heard the difference); now `midi::pitch_curve`
adds the slide, slide-in and 5.5 Hz vibrato to the bend curve, with the same overlap rule as bends.
Unit tested. Vibrato heard working. The sample's one slide isn't heard because another note of
the channel starts with it (the overlap rule drops it); giving such notes their own channel would
fix it, and the author chose to leave that for later. MIDI input (steps 6 and 7) is still untried with a real keyboard. Not done: real-time
recording.

This covers the two greyed-out boxes in Preferences > Audio & MIDI (MIDI output, MIDI input)
and the "MIDI capture" item in `gp8-feature-gaps.md`. The two halves are independent: output
can ship without input and the other way round.

## Goals

1. **MIDI output:** playback (and note previews) can go to a chosen MIDI device, such as a
   hardware synth or a software one like Microsoft GS Wavetable or a virtual MIDI cable,
   instead of, or as well as, the built-in sound.
2. **MIDI input (capture):** notes played on a MIDI keyboard are entered into the score at the
   cursor, beat by beat, the way typing fret numbers does today.
3. **Choice of devices** for both, saved in preferences and shown in Preferences > Audio & MIDI.

Not goals for the first version: MIDI clock/sync, MIDI time code, SysEx, receiving program
changes, recording in real time against the metronome, MIDI learn for shortcuts.

## What exists today

- `src/audio.rs`: `build_schedule(song, sr, opts)` turns the song into a time-sorted
  `Vec<NoteEvent>` (sample positions, frequency in Hz, bend, slide, velocity, pan, timbre,
  track, string, fret). `PlayerState::render` (run on the cpal audio thread) starts a `Voice` for
  each event as the playing position passes it.
- `NoteEvent` has **no MIDI pitch or channel**, only `freq`, and effects like bends are
  applied by the built-in synth. `track.pitch(&note)` gives the MIDI pitch (`model.rs`), and
  `Instrument::midi_program()` gives the General MIDI program (`model.rs`).
- Previews: `App::preview` pushes a one-shot `NoteEvent` into `PlayerState.preview`.
- Entering notes: `App::set_fret(string, fret)` (`app.rs`) sets a note at the cursor and goes
  through `App::edit`, so undo works.
- Preferences: `Prefs` is saved to `%APPDATA%\GuitarCraft\preferences.json` with
  `#[serde(default)]`, so new fields are safe to add. The two MIDI boxes are placeholders in
  `audio_tab` in `src/app/prefs.rs`.
- MIDI file import/export already exists (`fileio.rs`). Live MIDI shares nothing with it.

## Decision 1: the MIDI library

Use **`midir`** (cross-platform: WinMM on Windows, CoreMIDI, ALSA). It lists ports, opens an
output connection to send raw bytes, and opens an input connection with a callback on its own
thread. It is small and widely used. Alternative: call WinMM directly, which is Windows-only and
more code for no gain. The README says macOS and Linux are untried, so a cross-platform crate
keeps that door open.

This adds one dependency, which `cargo xtask` licence listing and the About window pick up
from `Cargo.lock` automatically.

Windows note: Windows has no MIDI input by default beyond attached devices, and its built-in
"Microsoft GS Wavetable Synth" is the usual output people will see.

## Decision 2: how playback reaches MIDI

The schedule is in samples and in terms of frequencies, which is the wrong shape for MIDI.
Two options:

- **A. Derive MIDI from `NoteEvent`:** add `midi_note`, `channel` and `program` fields to
  `NoteEvent`, filled in `build_schedule`, and have the player also emit MIDI messages for each
  event. Small change, and the same timeline and loop/speed handling is shared.
- **B. A separate MIDI schedule** built from the song. More code, and it would drift from the
  audio schedule over time.

**Choose A.** The player already walks the events in order; at the same moment it starts a
voice it can also send Note On, and it needs a matching Note Off at `start + len`.

Mapping details:

| Thing | MIDI |
|---|---|
| Note | Note On / Note Off with the pitch from `track.pitch`, velocity from `vel` (0..1 to 1..127) |
| Track | One channel per track (0 to 15, skipping 9). Drum tracks use channel 10 (index 9) |
| Instrument | Program Change at the start of playback, from `Instrument::midi_program()` |
| Volume / pan | CC 7 and CC 10 at the start, and again when the mixer changes them |
| Bend | Pitch Bend messages (see the open question below) |
| Mute / solo | Same rule as the built-in sound: muted tracks send nothing |
| Stop / pause | All Notes Off (CC 123) on every used channel, so nothing hangs |

More than 15 melodic tracks: share channels (rare; note it in the UI).

### Threading

The audio callback must not block. Sending to a MIDI port can block (WinMM calls). So:

1. The audio callback pushes `MidiMsg { when, bytes }` into a lock-free or bounded channel
   (`std::sync::mpsc::sync_channel` with `try_send`: drop on overflow, never wait).
2. A **MIDI thread** owns the `midir` output connection and sends messages when they are due,
   using the sample position converted to time.

Latency between built-in sound and MIDI device will differ by the device's own latency. First
version: no compensation. A later setting, "MIDI output delay (ms)", can delay the built-in
sound to line up (or the reverse).

### Built-in sound alongside MIDI

Add a preference **"Built-in sound: on / off while MIDI output is on"** (default: off, so a
user with a hardware synth does not hear both). With MIDI output set to "None" nothing changes.

### Previews

`App::preview` (clicking a note) also sends a short Note On/Off to MIDI output when it is set.

## Decision 3: MIDI input

`midir` input gives a callback with raw bytes on its own thread. The callback only parses
Note On (velocity above 0) / Note Off and pushes `(pitch, velocity, on)` into a channel. The
UI thread drains the channel once per frame (`App::update`) and acts on it. Nothing in the
callback touches the song.

### Entering a note

For a Note On with pitch `p`, on the current track:

1. Find a string and fret that gives `p`: try the string the cursor is on first, then the
   nearest string that can play it (fret from 0 to `track.frets`), preferring the lowest fret.
   If none can, show "Note out of range for this track" in the status bar and ignore it.
2. Call `set_fret(string, fret)`, so undo and the notation-error checks work as normal.
3. **Chords:** notes that arrive within about 60 ms of each other form one beat (the
   first note enters the beat, the rest add to it on different strings, each avoiding strings
   already used). When the window closes, the cursor advances to the next beat.
4. Keyboards and drum tracks: map the pitch directly (keyboard: the note; drums: the
   pitch to the drum piece, as the existing drum entry does).

Rhythm: the duration used is the one currently selected in the toolbar, so the player picks
the length with the usual shortcuts and plays pitches. This is "step entry", the same as
Guitar Pro 8's MIDI capture, which is what the feature-gap item describes ("beat by beat").

Real-time recording against the metronome is a later extension: the same input channel with
timestamps, quantised to the selected grid.

### Other input

- Sustain pedal (CC 64) and other controllers are ignored in v1.
- While the player is playing, input is ignored unless a later "record" mode exists.

## Preferences and UI

New `Prefs` fields (all with `#[serde(default)]`):

- `midi_output: Option<String>` (port name; `None` means off)
- `midi_input: Option<String>`
- `midi_builtin_with_output: bool` (default false)
- `midi_input_enabled` is implied by `midi_input.is_some()`

Preferences > Audio & MIDI: replace the two greyed-out boxes with a combo box each: "None",
then the ports, with the same refresh button the audio device has. A port that is missing now
shows the same "isn't connected now" note. Reconnect on change, not on every frame.

Status bar: when MIDI input is on, show a small "MIDI in" indicator that flashes on a note.
The toolbar needs no new buttons in v1.

Failure handling: a failed open shows a message in the status bar and leaves the setting as
the person chose it (like the audio device), and a device unplugged during use drops the
connection quietly and sets a "Reconnect" hint.

## Build order

Each step is mergeable and testable on its own.

1. **Ports and preferences.** Add `midir`; list input and output ports; the two combo boxes;
   the new `Prefs` fields; connect and disconnect. Test: ports appear, the choice survives a
   restart.
2. **Output of previews.** `App::preview` sends Note On/Off. Quickest proof that the output
   chain works end to end with a real device or a virtual port.
3. **`NoteEvent` gets MIDI fields** (`midi_note`, `channel`, `program`) filled in
   `build_schedule`. Unit tests on the schedule: pitches, channels, drums on channel 10.
4. **Playback to MIDI:** the MIDI thread and the bounded channel; Program Change, volume and
   pan at the start; All Notes Off on stop; loop and speed trainer checked. The "built-in sound
   with MIDI output" preference.
5. **Bends and slides** as Pitch Bend (decide the semantics from the open question below).
6. **MIDI input, note by note:** the callback and channel, single notes entered at the cursor
   with string/fret choice; status-bar indicator.
7. **Chords from input** (the 60 ms window) and keyboard/drum tracks.
8. **Docs and cleanup:** a section in the README's feature list, update
   `gp8-feature-gaps.md`, a note in the in-app help text.

Steps 1-5 are output, 6-7 are input. If time is short, stop after step 4 for a useful
"MIDI output" and ship input later.

## Testing

- Unit tests with no hardware: the schedule's MIDI fields (step 3), the pitch-to-string-and-fret
  chooser (step 6), the chord grouping window (step 7), the parser for raw MIDI bytes.
- A `MidiSink` trait wrapping "send these bytes", with a recording fake for tests, so the
  playback-to-MIDI logic is tested without `midir`.
- By hand, with a virtual port pair (loopMIDI on Windows) and a MIDI monitor: play a song,
  check notes, channels, program changes and that Stop silences everything. With a real
  keyboard for input.
- The user's own check on real hardware, since the author's setup cannot cover every device.

## Risks and open questions

- **Bends over MIDI.** The built-in synth bends within a note over time. MIDI pitch bend is
  per channel, not per note, so two bending notes on one channel at once (a chord with one bent
  string) conflict. Options: one channel per track and bend the whole channel (wrong for chords),
  or use MPE (one channel per note, widely but not universally supported). **Proposal:** v1 sends
  bends only when one note of the beat is bending, and resets the bend after it; anything else is
  played unbent. Revisit with MPE if people ask.
- **Timing accuracy.** A separate thread with sleeps can jitter by a few milliseconds. Likely
  fine for listening; test with the speed trainer and fast passages before promising more.
- **Drum track mapping.** Checked 2026-11-22: drum tracks keep their notes on virtual strings
  starting at pitch 0, so the fret is the General MIDI drum note (36 kick, 38 snare, 42 hat) and
  `track.pitch` already returns it. Drums go on channel 10 with no Program Change, as the MIDI
  file export already does. Remaining risk: imported files that use a different drum map; not
  heard on a real synth yet.
- **More than 15 tracks.** Channel sharing as above; unlikely to matter.
- **Linux/macOS.** `midir` supports both, but neither platform has been built or tried
  (see README), so treat those as untested.
- **Latency vs the built-in sound** when both play (see above).
- **Permissions/drivers.** Some Windows MIDI devices open for one program at a time; a failed
  open must say so clearly rather than silently doing nothing.
