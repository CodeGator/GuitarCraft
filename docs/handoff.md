# Handoff

Written 2026-11-27, after releasing 0.11.0 and starting `version-0.12.0`. Read this first, then
`docs/gp8-feature-gaps.md` (what is left to build) and
`docs/ai-design.md` (the AI rules and ideas).

## Where things stand

- **Released:** v0.11.0, published 2026-11-27 (https://github.com/CodeGator/GuitarCraft/releases/tag/v0.11.0),
  from PR #21. MSI and portable zip, unsigned. The MSI was never installed by hand; only the
  checksums were checked (they match). v0.10.0 is the one before it. 0.11.0 brought the drumkit
  view, Audio Note Settings, Guitar Pro files keeping more through saving, the **Claude Code** AI
  provider, **Suggest Chords** in chord charts and an AI Assistant that **only answers** (see "The
  AI features" below).
- **Working branch:** `version-0.12.0`, made from `main` after the release, version bumped to
  0.12.0. Not pushed to GitHub yet.
- **Branches on GitHub:** `main` and `release`.
- **Tests:** `cargo test --release` passes in full (296 passed, 7 ignored live tests that need a
  model, a key or a Claude Code sign-in). In a debug build (`cargo test`) three `dock` tests fail
  ("Dropped TexturesDelta with 1 unapplied deltas", from `epaint`); seen 2026-11-27 before any
  change that day.

## Drumkit view (2026-11-27, in 0.11.0)

Built from the owner's picture of Guitar Pro 8 (`src/app/drumview.rs`; names, numbers, staff
positions and noteheads in `src/drums.rs`). The owner called it good for now (2026-11-27). It changed
where many drum notes are written in the score to match Guitar Pro. Drum notes keep Guitar Pro's
sound numbers (91 = rim shot) and play and export to MIDI and GP5 as the sound Guitar Pro gives
them (`Track::sound`). `src/gpkit.rs` is Guitar Pro's kit definition, which `.gp` and `.gpx`
saving now writes; it was read from a drum file of the owner's saved by **Guitar Pro 7** (revision
12019). That file is **copyrighted: never copy it or its music into the repo**.

**Checked with Guitar Pro 8 (2026-11-27):** a file of ours with all 95 sounds and one- and
two-bar repeats opened and played there; the copy GP8 saved (revision 13008) kept every note,
text, section, repeat sign and the kit definition, renaming only the six bongo articulations
("Hand (hit)" became "Bongo high (hit)" and so on), and GuitarCraft reads that copy the same. GP8
writes `String` and `Fret` on drum notes; it doesn't need them from us. The owner heard that the
two don't sound exactly alike but generally play the same: ours are synthesized, and timbales,
agogo, whistles, guiro, cuica, castanets, surdo and the other sounds without a voice in
`synth.rs` share one plain noise. Not seen: a drum track created in GP8 itself (its picture has
"Tinkle Bell", which this kit lacks; it has an orchestral snare, 121, instead). Sounds Guitar Pro
writes far off the staff (much of the hand percussion) show as a cross mid-staff. Picture check:
`--show band,drums`.

**Drum sound fixed (2026-11-27):** we wrote the kit's sound as "Midi/Drum Kit", which GP8 replaced
with an Acoustic Piano when it saved our file. It is now `Drums/Drums/Drumkit` with MSB 0 and the
`Drumkit-Master` patch, as in the GP7 file. Checked: the copy GP8 saved, with the track on its own
sound engine (RSE), kept the drum kit sound.

**Saving a Guitar Pro file keeps its settings (2026-11-27; plan and decisions in
`docs/gp-keep-design.md`).** We rebuild a `.gp` file from our own model, so what the model doesn't
hold used to be lost. The plan is complete (`src/gpkeep.rs`), each step checked by the owner in
GP8. Only per-beat settings are still lost, by the owner's choice; the exact list is `ACCEPTED` in
`gpkeep.rs`, and its test fails on any new loss.

**Keyboards, voices and orchestra (fixed 2026-11-27; the owner opened our new files in GP8 and
GP8's own file in GuitarCraft: "looks good").** GP8
crashed opening a file of ours with voice and orchestra tracks, and we opened GP8's piano, voice,
violin and synth tracks as electric guitars. Guitar Pro tunes every staff, even of these, and puts
each note on a string; `src/gpinstr.rs` now writes and reads them as in `Instruments.gp` (type
names, sounds, tunings with strings added for notes out of reach, concert octaves numbered one
higher). Which part caused the crash was not isolated: guitars, synth, piano (one and two staves)
and drums opened before the fix. GP8 maps our `Midi/<name>` guitar sounds correctly. Still open:
(**Left hand, fixed 2026-11-27:** a lower staff with a rhythm of its own opened correctly but its
notes were lost after one save and reopen here, as only the lower staff's first voice was read.
All its voices are read now, and each staff is written with its own voices from the first place.
Hands are still divided at middle C, and a bar needing more than four voices loses the extra.
Checked: GP8 kept a two-hand file of ours unchanged on saving it, and that copy opens and plays in
GuitarCraft);
GP8's icons for these instruments differ from ours; GP8
offered no string ensemble, so ours is written as a violin with that sound.

**Step 2, track settings (2026-11-27; checked by the owner in GP8: saved copies sound the same,
and differ only in layout, as expected until step 4).** A track keeps what
its file had (`KeptTrack` in `gpkeep.rs`): sound engine, sounds and effects, MIDI channels,
instrument set and notation, transposition, the tuning under a keyboard or orchestral staff, and
the mixer strip, where only a control moved here is changed. All but the mixer go while the
track has another instrument.

**Step 4, layout and views (2026-11-27; the owner checked it in GP8: "looks good").** Bars per line,
zoom, score views and view stylesheets, and the layout and part configuration are kept (the rules
are in the plan; adding a bar or hiding, renaming or moving a track gives our own views).
GuitarCraft doesn't show these: its own line breaks and zoom are unchanged. We still write our
own version number (12025) above content from newer files. Left: step 3 (flats choice, bar settings,
automations tied to bars).

**Step 3, settings tied to bars (2026-11-27; the owner checked it in GP8: "looks good").** Mixer,
sound and master effect changes are kept on the bar they name, so they move, copy and go with it
(those at the very start of the song stay at the start, by the owner's choice);
the flats choice stays with its key and a bar's `XProperties` with the bar. A change of volume or
pan made here drops that control's changes over time. Edge not handled: a master bar pasted from
another song brings its master effect changes, which then apply to the effect in the same place.

**Audio Note Settings** (2026-11-27; checked by the owner, and a changed copy of ours showed the
change in GP8): `NotePlayback` on a note,
read and written as `Duration`, `Offset` (Guitar Pro ticks, 480 to the quarter; ours are 960) and
`RelativeVelocity`; matched to the owner's window picture and a file GP8 saved. The built-in sound
and MIDI export use them; arpeggio notes are left alone, as GP8 says. The window is drawn to
look like GP8's, from the owner's picture of it (its own title bar and number boxes with arrows;
the boxes also take typing, Up and Down, and the mouse wheel); the owner approved it.

**Tab import clean-up** (2026-11-27): the text tab importer now reads tab that is quoted with
">", uses typographic dashes or box lines, has spaces for dashes, or has blank lines between the
strings (`tab_blocks` in `textmidi.rs`). Tested on the demo song's own tab, damaged each way, and
checked by the owner. Also fixed: an ASCII import from the start screen went into a song hidden
behind it; it now opens as a new song. With a song showing it adds a track, as before.

**The name's colour** (2026-11-27, owner): "Craft" is the orange of the logo's pick
(`LOGO_ORANGE` in `panels.rs`), on the start screen and in the About window. It was blue, too much
like Guitar Pro. Use the constant anywhere else the name is drawn in colour.

**Contributors** (2026-11-27, in 0.12.0): Help > About has a Contributors tab, after the one in
storytold's apps but only their "grab bag" of names flowing across the window (the owner's choice,
after FilmCraft's; no statistics, sorting or name switches): `CONTRIBUTORS.md` is built into the
app and parsed in `about.rs` (a test fails if a line isn't read). It credits the owner, the Claude
models named in `Co-Authored-By` lines and Craft Anything. **Add a new model there when one starts
co-authoring**, and a person only with their agreement.

**Bar repeat signs** (2026-11-27): Guitar Pro's `SimileMark` is now read and written
(`Simple`, `FirstOfDouble`, `SecondOfDouble`); before, repeated bars of a Guitar Pro file opened
as rests. `Track::played_bar` gives the notes a sign stands for (playback, MIDI, GP5, MusicXML and
text export). A two-bar repeat shows a sign in each of its bars (Guitar Pro draws one across the
bar line). Notes typed into a bar with a repeat sign stay hidden under it. Guitar Pro 8 keeps all
three marks as we write them.

## What 0.9.0 added (older; 0.10.0 is in the next section)

- Dockable and floating panels, and Preferences > AI (Ollama and Claude settings, key in the
  system credential store).
- **Numbered (jianpu) notation** (`draw_num_beat`, `jianpu_degree` in `src/score/draw.rs`; the "2"
  button in `side.rs`; `Track::show_num`, saved in `.gc` only). Checked against Guitar Pro 8 by
  eye; it uses the sounding pitch, which is what GP8 does.
- **Action list and Expression text** as `@` and `>` modes of the command palette (`tools.rs`,
  `expression.rs`, two new shortcuts).
- **AI Assistant panel** (`src/app/aipanel.rs`, `ai::ask` in `src/app/ai.rs`): a docked
  conversation with Ollama or Claude. Return sends (Shift+Return for a new line); no Send button.
- **Layout changes made to match GP8:** files open at 100% zoom (was 110%), mouse-wheel scrolling
  is twice as fast (`line_scroll_speed` 80 in `setup_style`), and note columns are narrower
  (`col_width` in `src/score.rs`).

## What 0.10.0 added (released)

The owner has checked all of the UI below by eye (2026-11-23). The AI model side was driven by unit
tests and live Ollama calls.

- **Layout dropdown in the toolbar** next to Zoom (`panels.rs`, icons in `icons.rs`,
  `LAYOUT_ICONS`): the closed box shows only the current layout's icon; the list shows each icon
  with its name. It is 48 px wide.
- **View menu regrouped** (`menus.rs`): `Panels` (the Show items, Pop Out Panel, Dock All, Reset),
  `Notation`, `Score` (the five layouts and Split Score View), Follow Playback, Zoom, Full Screen.
- **Split score view** (View > Score > Split Score View): the score in two views one above the
  other, each with its own scroll, only in Page - Vertical and Screen - Vertical (both views
  share one layout there). Off by default, saved in prefs (`split_score`). `score()` in
  `bottom.rs` draws one or two `score_pane`s; the pane last clicked follows the cursor and
  playback; the bar between them drags (`split_frac`, not saved).
  Left: a command-palette entry, a mark for the active view, a left/right split for the Horizontal
  layouts if wanted.
- **Recent files** (`home.rs`): two files with the same name are told apart (`recent_labels`: the
  extension, then the folder); files that no longer exist are dropped when the list loads
  (`only_existing`) and when picked (`open_path`). A lost-drive entry is dropped too.
- **AI assistant**, in `src/app/ai*.rs` and `aipanel.rs`: it could change the score through
  cards (removed since, in 0.11.0); it answers questions about GuitarCraft from the user guide;
  every question carries an outline and a summary of the track.

## The AI features

**The rule (owner, 2026-11-27).** The AI Assistant panel only answers. Anything AI changes is a
button for one task, with a preview and one undo step. Don't add free-text editing, and try a new
AI feature on real examples before building it.

**Panel.** The top of the panel shows only a "New chat" button. By the owner's decision it shows
nothing about the provider or model ("I care about the outcome"; that is in Preferences > AI), and
it has no checkboxes: the program decides what goes with a question. If no provider is switched
on, it shows a warning with "Switch on Ollama" (when a model is chosen) and "AI Preferences". The
provider is the first one switched on (Ollama, Claude Code, Claude with a key); there is no
choice in the panel. The owner's settings on 2026-11-27: Claude Code on, Ollama off.

**What is sent with a question** (`ai_send`, `ai_context` in `aipanel.rs`): the matching sections
of the user guide (`aihelp.rs`, plain word matching, at most 4, none if nothing matches), the
song outline (`aitext::outline`), a summary of the whole track (`aifacts::track_facts`: key
signature, every note counted, the scales that fit best with the share of notes inside each), and
the selected bars (or the bar at the cursor) as ASCII tab with the notes, scales and chords found
in them (`aifacts::passage_facts`). Ollama answers at temperature 0.3. A chord chart sends none of
the song parts. The system prompt is `ai::SYSTEM`.

**Suggest Chords** (`src/app/chartsuggest.rs`; why only this and only Claude is in
`docs/ai-design.md`). Tried live once through its own code (`suggest_chords_live`); the window was
checked in a picture with a made-up answer (`--show chartsuggest`). The owner hasn't used it yet.

**Ollama.** "Keep model loaded" (Preferences > AI, on by default, `ollama_warm`): the model is
loaded in the background when the panel shows it, with the same context size as questions (a
different one makes Ollama load it twice), and `keep_alive` is 30 minutes.

**Checked live** before the change cards were removed, with `qwen3:8b`: `help_answers_live`,
`summary_live` and `ollama_answers_live`. An answer took about 100 seconds on the owner's 8 GB card
(`qwen3:8b` does not fit it at 16K context: 20% on the CPU); the owner chose an 8K context.
`qwen3:8b` is no longer installed (2026-11-27), though Preferences still named it.

**Claude with an API key** was never asked a question: the key saved and "Test connection"
worked, but answering needs prepaid API credits, which the owner will not buy. Do not spend time
there unless that changes.

**Claude Code** (`src/app/aicode.rs`, details and limits in `docs/ai-design.md` > "Claude Code")
is how Claude answers now: the owner's own `claude` program and plan. Live answers were seen
through `claude_code_answers_live` in about 4 seconds for two questions, and the owner asked it a
question in the panel. Before a release, check Anthropic's current terms.

## Known problems and things not checked

- **A settings incident (2026-11-23):** the tests used the real `%APPDATA%\GuitarCraft` folder, and
  a full `cargo test` replaced the owner's `preferences.json` (and the recent files) with defaults.
  Fixed: under test `app_dir()` is a temporary folder (`home.rs`, with a test). The owner re-entered
  their settings; anything else they had set in Preferences was lost and no backup was found. If
  settings ever look reset, check that first.
- **Slow local answers:** about 100 s per answer on an 8 GB card. Options: an 8K context, a smaller
  model, or `think: false` for qwen3 (not tried).
- **The bar spacing was tuned from one example** (Morning Walk). Check a busy score with
  16th notes or two-digit frets; if it looks cramped, raise the `11.0` in `col_width`.
- **Comparison with GP8 (2026-11-23, owner opened `samples/Morning Walk.gp` in both):** layout,
  notation pitches, tab and marks agree. Fixed: the numbered lane now sits between the staff and
  the tab with "1=G"; the default dynamic (mf) is no longer shown at the start of a track (GP8
  stores MF on every beat and shows none). Open: GP8 shows **no chord names** (G, D, ...) though
  the file defines them with ShowName on; no file saved by GP8 has chord names, so we have never
  checked our encoding. Ask the owner to add a chord in GP8, save it as a fixture in
  `tests/fixtures/gp8/`, and compare. Same for the numbered lane (a GP8 file with it on shows
  where GP8 stores it). Also open: the owner's running window showed a `*` (file modified) right
  after opening; a fresh command-line open didn't; the mixer (volume/pan/mute/solo, EQ, track
  visibility) sets `dirty` directly (`bottom.rs`), so a touch there is the likely cause, not
  reproduced. GP8 shows the selected track only (our window shows the whole score), as the file
  carries no ScoreViews.
- **Jianpu, matched to GP8 crops 2026-11-23 (owner's zoomed screenshots of bars 1-6):** chord = one
  digit (lowest string), tie = nothing, hammer-on/legato = arc, tuplet = curved bracket, grace
  underline joined, lane between staff and tab with "1=G". Code: `draw_num_beat` in
  `score/draw.rs`, `num_extent` and the `num_*` fields of `Geom` in `score.rs`. Check with
  `--show numbers` or `tests/fixtures/Morning Walk - numbered check.gp` (make it again with
  `guitarcraft --convert demo:numbers <path>`). Not compared with GP8: extra voices, a busy
  score, dynamics under the staff touching the lane, repeat bars. Still open: drums (GP8 has none
  either), and `.gp`/`.gpx`/`.gp5` files don't carry the setting (needs a GP8 file with the lane
  on, to see where GP8 stores it). No drawing tests yet.
- **Stem direction on a triplet (open, 2026-11-23):** in measure 2 of the numbered check file the
  triplet group (staff positions 6, 3, 6; middle line is 4) has stems DOWN in ours (average above
  the middle, `stem_up_of` in `score/draw.rs`) and UP in GP8. A "farthest note" rule also says
  down, so GP8 uses something else. Everything else in bars 1-6 agrees. Needs a larger GP8 crop
  of the staff, or a second example (other pitches), before changing the rule.
- **MIDI:** input has never been tried with a real keyboard; one slide is silent because it
  shares a channel with another note (see `docs/midi-design.md`); real-time recording isn't built.
- **README** (2026-11-27, the owner's wish, as in Dunbrec's): only a brief, high-level overview.
  Technical details go in `docs/development.md` and how to use things in the user guide (`GUIDE`
  in `src/app/tools.rs`), not in the README. The owner first asked to delete the guide (the app is
  too young), then kept it because the AI Assistant answers from it: **keep it up to date** when a
  feature changes. It was brought up to date for 0.11.0 on 2026-11-27. The README must still say what the AI sends (see
  `docs/ai-design.md`). The owner wants the Chords & Scales pictures in the README: the first rewrite dropped them, and
  they were retaken and put back on 2026-11-27.
- The GP8 guide doesn't state the Windows shortcuts for the Action list and Expression text; we
  use Ctrl+Alt+E and Ctrl+Shift+E by analogy with its Mac ones.

## What to do next

1. **Hear from the owner** about Suggest Chords in real use.
2. **Still missing from Guitar Pro 8** (see the gap list): instrument change automation, Design
   mode, and importing TablEdit and Power Tab. File password protection was decided against.

## How we work here

- **Never commit or push to `main`.** Work on the version branch (`version-0.12.0` now). When it is
  ready: pull request into `main` with a merge commit, then
  `git push origin origin/main:release` (check it fast-forwards), wait about 3 minutes for the
  workflow to draft the release, check the checksums, publish it. Then delete the merged branch
  locally and on GitHub, make the next `version-X.Y.0` branch from `main` and bump it with
  `cargo xtask version set`. Details in `docs/releasing.md`.
- **Don't commit or push unless asked**; the person says "commit that" or "push to github".
- **Keep the AI panel simple.** The owner dislikes controls they have to reason about and anything
  that spends room on setup details. Decide in code; put settings in Preferences.
- **Don't edit files in `%APPDATA%\GuitarCraft` by hand** without the person's say-so (the tool
  refused once); tell them what to change instead.
- **The app locks its own exe on Windows** while it is open: `cargo run --release` and
  `cargo build --release` fail with "Access is denied" until it is closed. Ask the owner to
  quit it (it may have unsaved work, so don't force it), build, then start it again for them. To check a change while it is open,
  build into another target directory (`CARGO_TARGET_DIR=<scratch> cargo build --release`) and
  take a picture with `--screenshot`; `--show ai` or `--show fretboard,keyboard` opens panels.
- **Edit files with small, exact edits.** `sed` with `\n` or `{}` in the pattern often silently
  does nothing; check the result, or use the edit tool.
- **`cargo test` takes one name filter** (`cargo test aitext`, not two).
- **Sample to test with:** `samples/Morning Walk.gp` (12 bars; sections Intro 1-4, Verse 5-8,
  Outro 9-12; tracks "Steel Guitar" and "Electric Bass").
- The person has old eyes: keep text large and readable in new windows.
- The owner's machine: RTX 3060 Ti with 8 GB; on 2026-11-27 Ollama listed `qwen3.5:4b` and
  `qwen2.5-coder:7b`.
- Git warns "LF will be replaced by CRLF" on every commit; it is harmless here.
