# Guitar Pro 8 features not yet in GuitarCraft

Compiled 2026-11-20 by comparing the Guitar Pro 8 user guide
(<https://www.guitar-pro.com/docs/gp8/basics>) against the GuitarCraft source.

**How reliable this is:** most of the guide was compared by chapter and section titles; only the
main screen, mixer, tuner, bends and keyboard shortcuts pages were read in full. Features were
matched to the code by keyword search, not by testing the app, so an item may be partly done or
exist under another name. Check before starting work on one, and tick items off as they land.

## To do

- [x] **Polyphonic tuner**: added 2026-11-20. Toolbar tuning fork (and Tools > Tuner):
      one strum shows every string's offset as a cents bar, green when within 3 cents; a
      single-string section below; choice of input device. Follows the current track's
      tuning and capo. Listens only while the window is open. Tested on made-up sounds; not
      yet tried with a real guitar. A string that isn't played can light up when another
      string's harmonic falls on its note.
- [~] **Preferences window** (Ctrl+,, File > Preferences): started 2026-11-20 in
      `src/app/prefs.rs`, saved to `%APPDATA%\GuitarCraft\preferences.json`. Working: default
      template, force zoom, force page layout, reopen last session, update check frequency
      (default "only when I ask"), playing cursor style, highlight played bar, keyboard shortcuts (Shortcuts tab: change, add, remove, reset; clash check),
      notation-error checks (wrong bar length; added 2026-11-22: note out of range, incomplete
      tuplets, hammer-on/pull-off with nothing before it on that string; checked within a bar
      only), user information for new songs, audio output device, buffer
      size, test sound. Added 2026-11-22: force stylesheet (Standard, Rock, Classic, Jazz), force
      notation (standard + tab, standard only, tab only; stringed tracks only) and copy chord
      diagrams (off strips them from beat and all-track copies; scale diagrams don't exist
      yet). Bars playback never reaches are also shown in red (added 2026-11-22, uses `Song::play_order`).
      Not tried by hand in the running app. Language option removed 2026-11-22 (no plans for other languages). Greyed out for now: MIDI output and input.
- [ ] **Audio Note Settings** (Shift+F): playback settings for individual notes.
- [ ] **Design mode** (Ctrl+Alt+D): palette mode for adjusting how the score is laid out.
- [ ] **Action list** (Ctrl+Alt+E) and **Expression text** (Ctrl+Shift+E). The command palette
      (Ctrl+E) already exists.
- [ ] **Drum view** instrument view (fretboard and keyboard views already exist).
- [~] **MIDI capture**: note entry from a MIDI keyboard, beat by beat; choice of MIDI input and
      output devices. Started 2026-11-22, plan in `midi-design.md`: output devices, MIDI
      playback, bends and chords on playback work (tried by hand with Morning Walk.gp); slides
      and vibrato are sent as pitch wheel moves like bends (added 2026-11-22, same curves as
      the built-in sound). Vibrato heard working; the sample's one slide is silent because it
      shares a channel with another note starting with it, and effects are only sent when a
      note sounds alone. Fix if wanted: give such notes their own channel (MPE style); note and chord entry from an input is written but never tried with a real keyboard.
      Not done: real-time recording against the metronome.
- [x] **Per-track equalizer** in the mixer. Added 2026-11-20, laid out like Guitar Pro 8's:
      On switch, presets, and Low, Low Medium (dB, Hz), Medium (Q, dB, Hz), High Medium
      (dB, Hz) and High knobs. Saved in .gc, .gp and .gpx files (matched with Guitar Pro 8
      files, see tests/fixtures/README.md); GP5 keeps only Low, Medium and High. Estimated, not
      confirmed: the Low and High shelf frequencies.
- [x] **Focus / Unfocus knob** in the mixer: turns the selected track up and the others down.
      Added 2026-11-20: the knob on the Master row (EQ column); follows the selected track;
      double-click to reset. Not saved with the song (how Guitar Pro 8 treats it is unchecked).
- [ ] **Instrument (sound) change automation** partway through a song.
- [x] **Multi-bar rests** (Ctrl+R, Bar > Multirest): added 2026-11-22. A setting of each track:
      when that track is shown alone, runs of empty bars show as one bar with a thick rest and
      their number. The guide says little, so the joining rules are ours: bars join while they are
      empty (no notes, chords, text or lyrics) and nothing needs showing between them (repeat,
      ending, section, tempo, key or time signature, double bar, jump, line break). Not shown in
      the multitrack view, as in the guide. Read from and written to .gp files: Guitar Pro 8 keeps
      it as the byte in front of each track's own view in `Content/PartConfiguration`, found by
      comparing `tests/Multi Rest.gp` (on) with files saved with it off. Confirmed 2026-11-22 with
      three-track files saved by Guitar Pro 8 (`tests/Multi Rest.gp` all on, `Multi Rest
      Off.gp` all off, `Multi Rest On For Track 1.gp`): the byte is per track, 1 on, 0 off.
      The record's last four bytes don't follow multi-rest (3 with all on and with all off, 1
      with track 1 only); probably which view was open when saved, a guess. We always write 0,
      which Guitar Pro 8 accepts. Not written to GPX.
      Not tried by eye in the app.
- [ ] **File password protection**: lock a file; lock icon on its tab; unlock prompt.
- [ ] **Numbered (jianpu) notation**: option exists but is greyed out (`src/app/side.rs`).
- [ ] **Import TablEdit and Power Tab**: menu items exist but are greyed out (formats not
      publicly documented).

## Probably never (sound production, per the README)

- Recorded sound banks (RSE), song mastering, signature sounds, amps and effects (pedalboard),
  saved sound presets
- Line-in, audio tracks / backing tracks
- Fretlight guitar support
- mySongBook (Arobas's online score store)

## Already done (for reference)

Bends (including prebends), slides, harmonics, trills, mordents and turns, rasgueado, brush, wah,
sustain pedal, fades and volume swells, free time, page and screen layouts, multitrack, full
screen, count-in and metronome, speed trainer, check bar duration, transpose, command palette,
fretboard and keyboard views, and all import / export / batch converter formats.
