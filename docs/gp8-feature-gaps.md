# Guitar Pro 8 features not yet in GuitarCraft

Compiled 2026-11-20 by comparing the Guitar Pro 8 user guide
(<https://www.guitar-pro.com/docs/gp8/basics>) against the GuitarCraft source.

**How reliable this is:** most of the guide was compared by chapter and section titles; only the
main screen, mixer, tuner, bends and keyboard shortcuts pages were read in full. Features were
matched to the code by keyword search, not by testing the app, so an item may be partly done or
exist under another name. Check before starting work on one, and tick items off as they land.

## To do

- [ ] **Polyphonic tuner**: toolbar tuning-fork button; tunes all strings from one strum (cents
      shown as bars, green when in tune), plus a single-string (monophonic) section. Follows the
      current track's tuning.
- [~] **Preferences window** (Ctrl+,, Edit > Preferences): started 2026-11-20 in
      `src/app/prefs.rs`, saved to `%APPDATA%\GuitarCraft\preferences.json`. Working: default
      template, force zoom, force page layout, reopen last session, update check frequency
      (default "only when I ask"), playing cursor style, highlight played bar, keyboard shortcuts (Shortcuts tab: change, add, remove, reset; clash check),
      wrong-bar-length highlighting, user information for new songs, audio output device, buffer
      size, test sound. Greyed out for now: force stylesheet, force notation, copy chord/scale
      diagrams, language, the other four notation-error checks (out of range, tuplets, HoPo,
      unreachable bars), MIDI output and input.
- [ ] **Audio Note Settings** (Shift+F): playback settings for individual notes.
- [ ] **Design mode** (Ctrl+Alt+D): palette mode for adjusting how the score is laid out.
- [ ] **Action list** (Ctrl+Alt+E) and **Expression text** (Ctrl+Shift+E). The command palette
      (Ctrl+E) already exists.
- [ ] **Drum view** instrument view (fretboard and keyboard views already exist).
- [ ] **MIDI capture**: note entry from a MIDI keyboard, beat by beat; choice of MIDI input and
      output devices.
- [ ] **Per-track equalizer** in the mixer: low / mid / high, with instrument presets.
- [ ] **Focus / Unfocus knob** in the mixer: turns the selected track up and the others down.
- [ ] **Instrument (sound) change automation** partway through a song.
- [ ] **Multi-bar rests** (Ctrl+R).
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
