# Handoff

Written 2026-11-23, after releasing 0.10.0 and starting `version-0.11.0`. Read this first, then
`docs/gp8-feature-gaps.md` (what is left to build) and
`docs/ai-design.md` (the AI rules and ideas).

## Where things stand

- **Released:** v0.10.0, published 2026-11-23 (https://github.com/CodeGator/GuitarCraft/releases/tag/v0.10.0),
  from PR #20. MSI and portable zip, unsigned. The MSI was never installed by hand; only the
  checksums were checked (they match). v0.9.0 is the one before it.
- **Working branch:** `version-0.11.0`, made from `main` after the release, version bumped to
  0.11.0 (one commit, not pushed yet). Nothing else is on it.
- **Branches on GitHub:** `main`, `release`, and an old merged `version-0.8.0` that could be
  deleted.
- **Tests:** `cargo test --release` passes in full (235 passed, 6 ignored live tests that need a
  model or a key). The three `dock` tests the last handoff called failing passed in this
  session's runs; I don't know why they used to fail, so don't trust it blindly.

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
- **AI assistant**, all in `src/app/ai*.rs` and `aipanel.rs` (details under "The AI assistant"
  below): the assistant can now change the score through cards; it answers questions about
  GuitarCraft from the user guide; every question carries an outline and a summary of the track.

## The AI assistant

**Panel.** The top of the panel shows only a "New chat" button. By the owner's decision it shows
nothing about the provider or model ("I care about the outcome"; that is in Preferences > AI), and
it has no checkboxes: the program decides what goes with a question. If no provider is switched
on, it shows a warning with "Switch on Ollama" (when a model is chosen) and "AI Preferences". The
provider is the first one switched on, Ollama before Claude; there is no choice in the panel.

**What is sent with a question** (`ai_send`, `ai_context` in `aipanel.rs`):
- An *order* (`aiplan::is_change_request`: "Copy the intro to the end", "please delete bars 3-4";
  not "How do I copy bars?"): the song outline only (`aiplan::outline`: track names, section names
  with bar ranges). No guide text, because its how-to steps once talked the model into explaining
  instead of writing a plan.
- Anything else: the matching sections of the user guide (`aihelp.rs`, plain word matching, at
  most 4, none if nothing matches), the outline, a summary of the whole track
  (`aifacts::track_facts`: key signature, every note counted, the scales that fit best with the
  share of notes inside each), and the selected bars (or the bar at the cursor) as ASCII tab with
  the notes, scales and chords found in them (`aifacts::passage_facts`).
- A one-line reminder follows the question (`aiplan::REMINDER`), and Ollama answers at
  temperature 0.3 (`OLLAMA_TEMPERATURE`), both to keep it to the plan format.
- The system prompt is `ai::system_prompt()` = `SYSTEM` + `aiplan::PLAN_HELP`.

**Plans** (`src/app/aiplan.rs`, no UI; the card is `plan_card` in `aipanel.rs`). The assistant
proposes a change as a fenced block tagged `guitarcraft-plan` holding a JSON list of steps.
`parse` reads it, `run` checks each step against the real song and carries it out (first on a copy,
to fill the card). The steps are `copy_bars`, `paste_bars` (insert or replace, at a bar or
`"end"`) and `delete_bars`; tracks and sections are named (a track may also be given by its number
in the outline). The card lists the steps in words, the bar count before and after, and notes
(bars of a different length), with Apply and Dismiss. Apply runs the plan again on a copy of the
song as it is now and only then takes one undo snapshot and swaps it in (`ai_apply_plan`), so a
stale plan changes nothing and one Undo takes it back. A chord chart refuses a plan. An order that
got an explanation instead of a plan shows "No change was proposed." with **Ask again**.

**Ollama.** "Keep model loaded" (Preferences > AI, on by default, `ollama_warm`): the model is
loaded in the background when the panel shows it, with the same context size as questions (a
different one makes Ollama load it twice), and `keep_alive` is 30 minutes.

**Checked live** with `qwen3:8b` on `samples/Morning Walk.gp` (ignored tests; set the variables
shown): `plan_live` (`PLAN_QUESTION`, `PLAN_CTX`, `OLLAMA_MODEL`, `PLAN_OLD=1` for the old prompt),
`help_answers_live`, `summary_live`, `ollama_answers_live`, `claude_answers_live`. "Copy the intro
to the end of the song", "copy the guitar track for Verse 1 and paste it into measure 12" and a
three-step delete-and-copy all gave valid cards; the model resolved "guitar track" and "Verse 1"
to the real names, and chose insert for "paste into measure 12" (the card says so). The owner once
got manual instructions instead of a plan, with the guide ticked; I could not reproduce it (about
ten runs), so the guide skip, the reminder, the lower temperature and Ask again are protections
for an occasional failure, not a proven fix. An answer takes about 100 seconds on the owner's
8 GB card (`qwen3:8b` does not fit it at 16K context: 20% on the CPU); the owner chose an 8K
context.

**Claude** was never asked a question: the key saved and "Test connection" worked, but answering
needs prepaid API credits, which the owner will not buy. Do not spend time here unless that
changes. Ollama is the path to tune.

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
- **README screenshots** in `docs/readme/` were taken before the 100% zoom and tighter spacing,
  and don't show the AI panel, the numbered lane, the layout dropdown or the split view.
- The GP8 guide doesn't state the Windows shortcuts for the Action list and Expression text; we
  use Ctrl+Alt+E and Ctrl+Shift+E by analogy with its Mac ones.

## What to do next

1. **More plan steps.** Each is a small addition to `aiplan.rs`: a `Step` variant, its run code,
   its line on the card, its test, and a line in `PLAN_HELP`. Candidates: transpose a range, set
   the tempo, rename or add a section, add or remove a track, set a repeat.
2. **Other AI ideas** (`docs/ai-design.md`): a natural-language mode in the command palette (a
   fixed list of commands, confirmed first), chord suggestions for chord charts, "fix messy tab"
   (the model tidies pasted tab, the existing importer reads it). Lyrics syllable splitting was
   set aside by the owner (a model can't know where a song breaks its syllables; it would need
   the melody's note count).
3. **Still missing from Guitar Pro 8** (see the gap list): Audio Note Settings (Shift+F),
   instrument change automation, Drum view, Design mode, and importing TablEdit and Power Tab.
   File password protection was decided against.
4. **Release 0.11.0** when the owner says: pull request `version-0.11.0` into `main` (see "How we
   work here"). Update the README screenshots first if wanted.

## How we work here

- **Never commit or push to `main`.** Work on the version branch (`version-0.11.0` now). When it is
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
  `cargo build --release` fail with "Access is denied" until it is closed. Close it
  (`taskkill //IM guitarcraft.exe //F`), build, start it again. To check a change while it is open,
  build into another target directory (`CARGO_TARGET_DIR=<scratch> cargo build --release`) and
  take a picture with `--screenshot`; `--show ai` or `--show fretboard,keyboard` opens panels.
- **Edit files with small, exact edits.** `sed` with `\n` or `{}` in the pattern often silently
  does nothing; check the result, or use the edit tool. Python isn't installed here.
- **`cargo test` takes one name filter** (`cargo test aiplan`, not two).
- **Sample to test with:** `samples/Morning Walk.gp` (12 bars; sections Intro 1-4, Verse 5-8,
  Outro 9-12; tracks "Steel Guitar" and "Electric Bass").
- The person has old eyes: keep text large and readable in new windows.
- The owner's machine: RTX 3060 Ti with 8 GB; Ollama models installed include `qwen3:8b`
  (the one in use), `gemma3:4b` and `qwen2.5-coder`.
- Git warns "LF will be replaced by CRLF" on every commit; it is harmless here.
