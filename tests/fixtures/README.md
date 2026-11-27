# Test fixtures

Files the tests read, through `crate::fixture("gp8/<name>")`. A test fails if its file is
missing. Only files you made or own go here; nothing from the Guitar Pro program folder.

For example songs to open in the app, see `samples/`.

## `gp8/`: files saved by Guitar Pro 8

Each file was saved by Guitar Pro 8 so GuitarCraft can be matched against the real format.
Almost all of them are the demo song ("Morning Walk", File > New in GuitarCraft) with one
thing changed in Guitar Pro 8. Comparing a file with `Morning Walk Gp8.gp` shows how Guitar
Pro stores that change.

| File | What was done in Guitar Pro 8 | Read by |
|---|---|---|
| `Morning Walk Gp8.gp` | Nothing: the demo song as Guitar Pro 8 saves it. The baseline. | `gpifwrite::opens_the_demo_saved_by_guitar_pro`, `gpstyle::reads_guitar_pro_saved_stylesheet`, `gpsv::matches_views_saved_by_guitar_pro` |
| `Morning Walk - legato.gp` | Legato | `gpifwrite::reads_legato_from_guitar_pro_file` |
| `Morning Walk - wah.gp` | Wah open, then closed, on the bass's first two notes | `gpifwrite::wah_written_like_guitar_pro` |
| `Morning Walk - Arpeggio.gp` | Arpeggio down on the first bass note, up on the second | `gpifwrite::arpeggio_written_like_guitar_pro`, `gpifwrite::arpeggio_settings_saved` |
| `Morning Walk - golpe.gp` | Golpe (finger and thumb) | `gpifwrite::golpe_written_like_guitar_pro` |
| `Morning Walk - pick scrape.gp` | Pick scrape up and down | `gpifwrite::pick_scrape_written_like_guitar_pro` |
| `Morning Walk - fingering.gp` | Left- and right-hand fingering | `gpifwrite::fingering_written_like_guitar_pro` |
| `Morning Walk - audio note settings.gp` | Audio Note Settings on the first five notes of the guitar: duration 50%; offset 5 ticks; velocity +6 dB; -6 dB; duration 200% with +3 dB | `gpifwrite::audio_note_settings_read_and_written_like_guitar_pro` |
| `Morning Walk - timer.gp` | A timer at the start of the song | `gpifwrite::timer_written_like_guitar_pro` |
| `Morning Walk - bar repeats.gp` | A bar repeated on its own (repeat signs with a count) | `gpifwrite::single_bar_repeat_like_guitar_pro` |
| `Morning Walk - track eq default.gp` | Track EQ on, every knob at its default | `gpifwrite::track_equalizer_read_and_written_like_guitar_pro` |
| `Morning Walk - track eq max.gp` | Track EQ on, every knob fully right | same |
| `Morning Walk - track eq min.gp` | Track EQ on, every knob fully left | same |
| `Morning Walk - track eq code A.gp` | Each knob at left, default or right; with code B, each knob gets a different pair, which shows which number is which knob | same |
| `Morning Walk - track eq code B.gp` | See code A | same |
| `Drum check.gp` | A drum test file made by GuitarCraft (every drum sound, one- and two-bar repeats), saved by Guitar Pro 8 with the track on its own sound engine (RSE) | `gpkeep` |
| `Instruments.gp` | Made in Guitar Pro 8: one track each of piano, voice, violin, synth, electric piano, organ, choir, cello, strings, trumpet, saxophone and flute, with a few notes | `gpkeep`, `gpifwrite::reads_the_instruments_guitar_pro_saved` |
| `Morning Walk - GP8.gpx` | The demo song saved as `.gpx`: the model for GuitarCraft's `.gpx` export (file layout and file list) | `guitarcraft --gpx-tests` |

## What these files showed

### Palette marks

Confirmed from a Guitar Pro 8 file (GuitarCraft writes and reads them the same way):
legato, wah open/closed, arpeggio up/down (with its duration and start time), golpe
finger/thumb, pick scrape up/down, left- and right-hand fingering, timer, a bar repeated on
its own (repeat signs with a count).

Written from public descriptions of the format, not yet checked with a file: shift slide,
slide out upwards, slide in from above. Timer: the file only had one at the start of the
song (time 0), so it isn't known yet whether Guitar Pro counts seconds or something finer.

Set aside for later (15 Nov 2026). These couldn't be added in Guitar Pro 8, so they wait
until a fixture is made. Nothing is lost: GuitarCraft keeps them in its own hidden note in
the file, which Guitar Pro 8 ignores, so only Guitar Pro won't show them yet:

- Barre ("BV", Notation section of the palette)
- Slash notation (Notation section)
- Stem direction and beaming (the row of note icons under the Notation section)
- Free time (the dotted bar line, Bars section, next to the triplet feel)
- Repeat the previous bar / two bars (the slash-and-dots signs, Bars section)
- Ornaments (mordents, turns), artificial harmonic types (A.H., T.H., S.H.), left-hand
  tapping, sustain pedal, fade out, volume swell, whammy bar, bridge vibrato, rasgueado,
  dead slap, string number

Tips for making those files (from the user guide and the palette pictures):

- Barre, slash, stems and beaming are greyed out in Guitar Pro 8 until a note is selected,
  and stems and beaming only work while standard notation is shown (Track > Notation).
  Click a note in the notation staff first, then the tool.
- Free time and the bar repeat signs act on the whole bar: click inside the bar (an empty
  bar works), then the tool.
- Barre opens a small window to choose the fret and full or half barre.
- If a tool still won't turn on, the same command is usually in the menus (Bar menu for
  free time and repeats, Note or Effects menu for the rest).

### `.gpx` and MusicXML (17-18 Nov 2026)

`guitarcraft --gpx-tests` writes three files to `shots/gpx-checks/`: our export; Guitar Pro
8's own files in our packing; our score with Guitar Pro 8's other files. All three open in
Guitar Pro 8 (confirmed 18 Nov). Guitar Pro 8 imports GuitarCraft's MusicXML (confirmed
18 Nov). `guitarcraft --gp-tests` writes `.gp` check files to `shots/gp-checks/`.

### Score views

The demo's part views were never opened in Guitar Pro 8, so they hold only the track's
name. The full score view starts with the bar list GuitarCraft writes, then has layout that
Guitar Pro 8 adds and GuitarCraft doesn't write (`16+1+0.5+0+1…`). An older fixture
(`Empty.gp`, now lost) checked a full layout byte for byte; that check needs a new file.

### Track equalizer (20 Nov 2026)

The track's `<RSE><ChannelStrip><Parameters>` holds 16 numbers from 0 to 1. The first ten
are Guitar Pro 8's 5-band track EQ, stored right to left compared with the panel:

| # | Knob | # | Knob |
|---|---|---|---|
| 0 | High dB | 5 | Medium Q |
| 1 | High Medium dB | 6 | Low Medium dB |
| 2 | High Medium Hz | 7 | Low Medium Hz |
| 3 | Medium dB | 8 | Low dB |
| 4 | Medium Hz | 9 | On (1) / Off (0) |

Then 11 is pan and 12 is volume. Every knob's default is 0.5. dB knobs run from -15 dB (0)
to +15 dB (1), so 0.5 is 0 dB.

Ranges from Guitar Pro 8's tooltips (Hz and Q knobs are logarithmic; the default is the
geometric middle):

- Low Medium: 45 Hz to 3000 Hz, default 367 Hz
- Medium: 45 Hz to 18 kHz, default 900 Hz
- High Medium: 500 Hz to 18 kHz, default 3000 Hz (500 worked out from the default)
- Medium Q: 0.5 to 3.0, default 1.2 (the middle is 1.22)

Not known yet: the frequencies of the Low and High shelves (GuitarCraft guesses 100 Hz and
8 kHz). Only the sound in GuitarCraft depends on these. Files store knob positions, so they
open the same in Guitar Pro either way.

## Files made by GuitarCraft, for comparing with Guitar Pro 8

Not read by any test. Open each in both programs and compare.

| File | What it holds | Made with |
|---|---|---|
| `Morning Walk.gp` | The demo song, as GuitarCraft saves it. | `guitarcraft --convert demo "tests/fixtures/Morning Walk.gp"` |
| `Morning Walk - numbered check.gp` | The demo with bar 2 of the Steel Guitar changed: beat 2 tied to beat 1, a grace note on beat 3, a three-note chord on beat 4, a triplet on beats 5 to 7 (and one more eighth to keep the bar full). Switch the numbered lane on for the Steel Guitar in both programs (the file doesn't carry it). | `guitarcraft --convert demo:numbers "tests/fixtures/Morning Walk - numbered check.gp"` |
