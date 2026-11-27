# Keeping a Guitar Pro file's own settings: design

Status: 2026-11-27. The owner agreed the plan (see "Decisions"). **Steps 0 to 4 are built**
(`src/gpkeep.rs`; step 4 was done before step 3). The owner checked steps 1 to 4 in Guitar Pro 8,
which completes the plan. The owner checked steps 1 and 2 in Guitar
Pro 8 (after step 2, saved copies sound the same and differ in layout).

## The problem

GuitarCraft reads a `.gp` file into its own model of a song and writes a new file from that model.
Whatever the model doesn't hold is gone. A song that starts in Guitar Pro, is saved by
GuitarCraft and goes back comes back with its notes, but not as it was.

Measured on 2026-11-27 by saving 19 files through GuitarCraft with no edits: 16 files saved by
Guitar Pro 8 (the fixtures, the two multirest tests), two Guitar Pro 8 copies of a drum test file
of ours, and one drum song of the owner's saved by Guitar Pro 7.

| Lost or rewritten | Seen in | What the person notices in Guitar Pro |
|---|---|---|
| Most of the stylesheet (`Content/BinaryStylesheet`): 398 settings dropped from the GP7 file, 423 of 654 from a GP8 file | 19 of 19 | Dropped settings fall back to the defaults |
| Font names rewritten (such as `@musicalFontText()` to `Times New Roman`) | 8 | Other fonts |
| `Content/Preferences.json`, `meta.json`, view stylesheets (`Content/Stylesheets/`), score views past what we write (`Content/ScoreViews/`) | 18 or 19 | View settings reset |
| Master effects (`MasterTrack/RSE`) | 19 | The song sounds different |
| Track sounds and effects (`Sound/RSE`), and the sound engine, which we set to MIDI | 16 | The song sounds different |
| Bars per line (`ScoreSystemsLayout`, `SystemsLayout`) | GP7 file, drum copies | Another page layout |
| Volume, pan and master effect automations | GP7 file | The changes over time are gone |
| Page header and footer text | GP7 file | Saved as empty |
| Pickup bar (`Anacrusis`), `MultiVoice`, the key's flats choice (`TransposeAs`) | GP7 file | Reset |
| Per-beat pickup volume and tone, stem directions, bar `XProperties` | up to 14 | Small playback and drawing differences |

String and fret values on drum notes are also not written, but Guitar Pro 8 adds them itself.

## Goals

1. Open a `.gp` file, save it without changes, and lose nothing Guitar Pro uses.
2. Edit it, and lose only what the edit makes wrong.
3. Never write a file Guitar Pro can't open. A lost setting is better than a broken file.

Not goals: showing or editing the kept settings in GuitarCraft; keeping them in `.gp5`, MIDI or
MusicXML exports; `.gpx` files (a later step, if wanted).

## Choosing the approach

| Approach | For | Against |
|---|---|---|
| **A. Hold on to the original XML and patch it on save** | Keeps everything | Every edit has to be mapped back onto the XML. Guitar Pro shares identical notes and beats between places, and inserted or deleted bars shift everything after them |
| **B. Add every Guitar Pro setting to our model** | Clean | Very large, and never complete: the next Guitar Pro version adds more |
| **C. Keep what we don't understand on the object that owns it** | Follows edits by itself | Each kept item needs a rule for when it stops being true |

**Proposed: C.** The song, each track, each master bar and each bar get an optional block of
kept settings:

```rust
/// What a Guitar Pro file had that this app doesn't understand, to be written back.
kept: Option<Arc<KeptTrack>>,   // and KeptSong, KeptMasterBar, KeptBar
```

Because it sits on the object, it goes wherever the object goes: undo and redo, moving,
duplicating and deleting tracks, deleting bars, copying all tracks. `Arc` keeps the undo history
small (it holds copies of the whole song). The field is skipped by serialization.

## Rules

Each kept item is written back **until the thing it depends on changes**. To know that, the kept
block also records what our model held when the file was read.

| Kept item | Owner | Written back while | Otherwise |
|---|---|---|---|
| `Preferences.json`, `meta.json`, `MultiVoice`, an empty title | Song | Always (the title while unchanged) | |
| Stylesheet entries | Song | Setting by setting: the original value, unless ours differs from what it was on opening | Our value |
| Header and footer text | Song | Our header and footer settings are unchanged | Ours |
| Master effects | Song | Always; their changes over time are on the master bars | |
| Pickup bar | Master bar (`pickup`) | It is still the first bar | Off |
| Sound engine, sounds, effects, `UseOneChannelPerString`, MIDI channels | Track | The instrument is unchanged | Ours (MIDI) |
| Instrument set, notation, transposition, tuning of an instrument without strings | Track | The instrument is unchanged (a drum kit: while it has every sound used) | Ours |
| Mixer strip numbers | Track | Number by number: volume, pan and equalizer only if changed | Ours |
| Mixer, effect and sound automations | Bar, or master bar for master effects (they name a bar) | They move with their bar and go when it is deleted. A change at the very start of the song (first bar, first position) stays at the start, until a bar's own change comes to that place. A moved mixer control loses its changes; a change to a sound the track doesn't have is left out | Dropped |
| Flats or sharps choice | Master bar | The key is unchanged | Ours |
| Bar and beat settings | Bar, beat | The object exists; stem directions only while its notes are unchanged | Left out |
| Bars per line | Song and track | The number of bars is unchanged | Ours |
| Zoom | Song | Always | |
| Score views, view stylesheets, layout and part configuration | Song | The same number of bars, and the same tracks in the same order with the same names, instruments and staves shown; the score's own newer stylesheet also needs unchanged style settings | Ours |

Special cases:

- **A new bar** made from its neighbour (Insert Bar) starts with nothing kept, as it already starts
  without the neighbour's section name and repeats.
- **A duplicated track** keeps its sound and automations.
- **Bars pasted from another song** keep their bar settings, not automations that name a sound the
  track doesn't have.
- **Hidden marks:** the stylesheet and extra marks GuitarCraft already stores in the file
  (`style_from_xml`, `extras.rs`) stay as they are and win over kept values.

## Steps

Each step is useful on its own, is checked by tests, and ends with a file the owner opens in
Guitar Pro 8.

| Step | What | Size |
|---|---|---|
| 0 | **A round-trip test** that saves each fixture unchanged and lists every difference, with a list of accepted ones (`ACCEPTED` in `gpkeep.rs`). No change in behaviour. The list shrinks with each step. **Built** | Small |
| 1 | **Whole-file settings:** preferences, stylesheet merge, header and footer, master effects, pickup bar, `MultiVoice`, an empty title. **Built** | Medium |
| 2 | **Track settings:** sound engine, sounds, effects, mixer strip, instrument set, transposition. Nothing about the instrument is kept from files earlier versions of GuitarCraft wrote (sounds named `Midi/...`). **Built** | Medium |
| 3 | **Settings tied to bars:** automations, flats choice, bar `XProperties`. **Built** | Medium |
| 4 | **Layout:** bars per line, zoom, score views. **Built** | Medium |
| 5 | **Per-beat settings** (pickup volume and tone, stem directions). **Not wanted** (owner) | Small gain |

Steps 1 and 2 recover most of what a person would notice.

## Testing

- Unit tests for each rule: unchanged keeps it, the change it depends on drops it.
- Edit tests: delete, insert, paste and move bars and tracks, then check the file still reads and
  its automations sit on the right bars.
- Fixtures must be files the owner made. The drum test file saved by Guitar Pro 8 is
  `Drum check.gp`. Still needed, made in Guitar Pro 8 from the demo song: one with volume and pan automation and the
  RSE engine, one with custom bars per line and page header, one with a pickup bar. **No
  copyrighted song goes into the repo.**

## Risks and open points

- **Guitar Pro may reject mixed content.** We write revision 12025; Guitar Pro 8 writes 13008.
  Whether kept Guitar Pro 8 settings need the original revision has to be tried in step 1.
- **Order of XML elements** may matter to Guitar Pro: kept elements go back in their original
  places.
- **Sounds of new tracks.** Guitar Pro 8 swapped our drum sound name for a piano (fixed). We still
  write other instruments as `Midi/<name>`; what Guitar Pro 8 does with each has not been checked.
  Check it with a saved copy of a file holding one track of every instrument, before step 2.
- **Saving more than once** needs nothing special: the song keeps what was read from the file it
  was opened from, and the rules go on comparing with that.

## Decisions (owner, 2026-11-27)

1. A Guitar Pro file keeps its sound engine (RSE) after GuitarCraft saves it (step 2).
2. A kept setting that an edit has made wrong is dropped silently.
3. Step 5 is not wanted.
