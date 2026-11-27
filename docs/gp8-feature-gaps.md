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
      Tried by hand in the running app (confirmed 2026-11-23). Language option removed 2026-11-22 (no plans for other languages). Greyed out for now: MIDI output and input.
- [ ] **Audio Note Settings** (Shift+F): playback settings for individual notes.
- [ ] **Design mode** (Ctrl+Alt+D): palette mode for adjusting how the score is laid out.
- [x] **Action list** (Ctrl+Alt+E) and **Expression text** (Ctrl+Shift+E): added 2026-11-23, as
      in the Guitar Pro 8 guide both are modes of the command palette (Tools menu, or "@" and
      ">" typed first in the palette; `src/app/tools.rs`, `src/app/expression.rs`). Action list:
      every shortcut-list action (not the chord chart ones), searched by name or group, with its
      keys shown. Expression text: with nothing typed it lists everything (chord roots first, then dynamics, octave signs, key signatures, directions; the highlighted entry's action is named under the list, as in GP8); a typed term becomes a dynamic (ff), a chord name (G maj, Bbm7),
      a key signature (G maj, E min), an octave sign (8va), a direction (Coda, D.S. al Fine) or,
      failing those, text above the beat; chord and key both show for "G maj". The guide gives no
      preset list and its Windows shortcuts aren't stated (we use Ctrl+Alt+E and Ctrl+Shift+E by
      analogy with the Mac ones). Not tried by hand in the running app. Not done: clefs.
- [x] **Drum view** (View > Panels > Show Drumkit View, the drum toolbar button, or Ctrl+F6 on a
      drum track): added 2026-11-27 from the owner's picture of Guitar Pro 8 and its online guide
      (`src/app/drumview.rs`, the kit table in `src/drums.rs`): the staff with 24 sounds, the
      list of all sounds, previous/next beat, number keys 1-9 for the sounds on a line, and the
      gear settings. The score uses the same staff positions and noteheads. Saving writes Guitar
      Pro's own kit definition (`src/gpkit.rs`, read from a Guitar Pro 7 file). Checked with
      Guitar Pro 8 (2026-11-27): it opens, plays and re-saves a drum track of ours unchanged but
      for six bongo names. Not done:
      hand percussion far off the staff (shown mid-staff), the bow and golpe signs, and the
      stick/brush stroke types in the guide.
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
      Verified by eye in the app (confirmed 2026-11-23).
- [~] **Numbered (jianpu) notation**: added 2026-11-23. The "2" button in the track's staff
      switches (Track settings) shows a lane between the notation staff and the tab, with "1=G" at the start of each system
      (moved there 2026-11-23 after the owner compared with GP8; `draw_num_beat` in
      `src/score/draw.rs`, saved in .gc as `show_num`). Degrees count from the key's tonic (a
      minor key uses its relative major), sounding pitch (so a guitar reads an octave lower than its standard staff, as in Guitar Pro 8: checked 2026-11-23 against one note), dots above or
      below for octaves, underlines for eighths and shorter (joined within a beam group),
      dashes for half and whole notes, dots for dotted notes, 0 for rests. Compared with
      Guitar Pro 8 by eye and found matching (confirmed 2026-11-23). Matched to two zoomed GP8
      crops (bars 1-6 of the guitar, 2026-11-23): a chord shows ONE digit, the note on its lowest
      string (the first note GP8 lists; our writer now lists a chord low string first, as GP8
      does); a tie draws nothing; a hammer-on or legato draws an arc over the digit and the next
      note's span; a tuplet is a curved bracket with the number, just above the digits; a grace
      note is a small digit whose underline runs into the beat's own; bar lines run through the
      lane (heavier at a repeat or the end); one lane per voice in use (not compared with GP8).
      Check it with `--show numbers` or `tests/fixtures/Morning Walk - numbered check.gp`. Not
      done: a numbered-only view; drums have none (Guitar Pro 8 has none either); not read from
      or written to .gp/.gpx/.gp5 files (not known how Guitar Pro 8 stores it).
- [ ] **Import TablEdit and Power Tab**: menu items exist but are greyed out (formats not
      publicly documented).

## Not doing

- **File password protection** (decided 2026-11-23).

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
