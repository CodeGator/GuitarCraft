# Handoff

Written 2026-11-23, at the end of the session that shipped v0.9.0. Read this first, then
`docs/gp8-feature-gaps.md` (what is left to build) and `docs/ai-design.md` (where the AI work is
going).

## Where things stand

- **Released:** v0.9.0, published 2026-11-23 (https://github.com/CodeGator/GuitarCraft/releases/tag/v0.9.0).
  MSI and portable zip, unsigned. The MSI was never installed by hand; only the checksums were
  checked.
- **Working branch:** `version-0.10.0`, made from `main` after the release, with the version
  already bumped to 0.10.0 (one commit, pushed). Nothing else is on it yet.
- **Branches on GitHub:** `main`, `release`, `version-0.10.0`, and an old merged
  `version-0.8.0` that could be deleted.

## What 0.9.0 added (this session and the one before)

- Dockable and floating panels, and Preferences > AI (Ollama and Claude settings, key in the
  system credential store).
- **Numbered (jianpu) notation** (`draw_num_beat`, `jianpu_degree` in `src/score/draw.rs`; the "2"
  button in `side.rs`; `Track::show_num`, saved in `.gc` only). Checked against Guitar Pro 8 by
  eye; it uses the sounding pitch, which is what GP8 does.
- **Action list and Expression text** as `@` and `>` modes of the command palette (`tools.rs`,
  `expression.rs`, two new shortcuts). Expression text lists chord roots first and names the
  highlighted entry under the list, as GP8 does. The palette list is taller (egui's own
  minimum was two rows).
- **AI Assistant panel** (`src/app/aipanel.rs`, `ai::ask` in `src/app/ai.rs`): a docked
  conversation with Ollama or Claude. Return sends (Shift+Return for a new line), there is no
  Send button, and the typing box uses 15 pt text (`INPUT_TEXT_SIZE`). The "What will be sent"
  preview was removed in 0.10.0 at the owner's request. It answers in words and, since 0.10.0, proposes score changes as cards (see "AI plans" below).
- **Layout changes made to match GP8:** files open at 100% zoom (was 110%), mouse-wheel scrolling
  is twice as fast (`line_scroll_speed` 80 in `setup_style`), and note columns are narrower
  (`col_width` in `src/score.rs`) so Morning Walk's first three bars share a line.

## Known problems and things not checked

- **Three `dock` tests fail** (`the_layout_survives_saving_and_loading`,
  `odd_layouts_are_repaired_or_replaced`, `panels_start_with_sensible_proportions`) with a
  panic at `epaint-0.36.2/src/textures.rs:337`, headless. They fail the same way without any of
  this session's changes. The release workflow doesn't run tests, so they never blocked it.
- **The AI panel has been tried with Ollama only** (one live question, kept as the ignored test
  `ollama_answers_live`). Claude: a real key was saved and "Test connection" worked (2026-11-23), but answering needs prepaid API credits, which the owner chose not to buy, so no question has been answered by Claude (`claude_answers_live` is the ignored test for it). The first Ollama answer
  can take a minute while the model loads.
- **The bar spacing was tuned from one example** (Morning Walk). Check a busy score with
  16th notes or two-digit frets; if it looks cramped, raise the `11.0` in `col_width`.
- **Jianpu gaps:** ties, tuplet numbers, grace notes, bar lines through the lane, voices 2 to 4,
  chords of more than two notes, drums, and `.gp`/`.gpx`/`.gp5` files don't carry it.
- **MIDI:** input has never been tried with a real keyboard; one slide is silent because it
  shares a channel with another note (see `docs/midi-design.md`); real-time recording isn't built.
- **README screenshots** in `docs/readme/` were taken before the 100% zoom and tighter spacing,
  and don't show the AI panel or the numbered lane.
- The GP8 guide doesn't state the Windows shortcuts for the Action list and Expression text; we
  use Ctrl+Alt+E and Ctrl+Shift+E by analogy with its Mac ones.

## What to do next

1. **AI, step one of the build order** in `docs/ai-design.md`: lyrics syllable splitting, then
   chord suggestions for chord charts. These need a preview-and-apply step (one undoable
   edit) that the panel doesn't have yet. The open questions at the end of that document are
   still open: default provider, a spending limit, saving answers,
   and key storage outside Windows and macOS.
2. **Still missing from Guitar Pro 8** (see the gap list): Audio Note Settings (Shift+F),
   instrument change automation, Drum view, Design mode, and importing TablEdit and Power Tab.
   File password protection was decided against.
3. Claude questions are untested: the key works, but answering needs prepaid API credits (the owner won't pay for them). Do this only if that changes; run `cargo test claude_answers_live -- --ignored --nocapture`. Meanwhile Ollama is the path to tune for.
4. **Split score view** (built, not yet checked by eye): View > Score > Split Score View shows
   the score in two views one above the other, each with its own scroll, in the two vertical
   layouts only (Page - Vertical and Screen - Vertical), where both share one layout. Off by
   default, saved in prefs (`split_score`). `score()` in `src/app/bottom.rs` now draws one or two
   `score_pane`s; the one last clicked follows the cursor and playback; the bar between them is
   dragged (`split_frac`, not saved). Left to do: a command-palette entry, a mark showing which
   view is active, and a left/right split for the Horizontal layouts if wanted.
5. **More AI plan steps** (the cards are built, see "AI plans" below): the toolbox so far is copy,
   paste and delete bars. Natural next steps, each a small addition to `aiplan.rs` (a `Step`
   variant, its run code, its line on the card, its test, a line in `PLAN_HELP`): transpose a
   range, set the tempo, rename or add a section, add or remove a track, set a repeat. Also still
   to do: a natural-language mode in the command palette, chord suggestions for charts, and
   "fix messy tab".

## How we work here

- **Never commit to `main`.** Work on the version branch (`version-0.10.0` now). When it is
  ready: pull request into `main` with a merge commit, then
  `git push origin origin/main:release` (check it fast-forwards), wait about 3 minutes for the
  workflow to draft the release, check the checksums, publish it. Then delete the merged branch
  locally and on GitHub, make the next `version-X.Y.0` branch from `main` and bump it with
  `cargo xtask version set`. Details in `docs/releasing.md`.
- **Don't commit unless asked**; the person says "commit that" when they want it.
- **The app locks its own exe on Windows** while it is open: `cargo run --release` and
  `cargo build --release` fail with "Access is denied" until it is closed. To check a change
  while the app is open, build into another target directory
  (`CARGO_TARGET_DIR=<scratch> cargo build --release`) and take a picture with
  `--screenshot`; `--show ai` or `--show fretboard,keyboard` opens panels in it.
- **Edit files with small, exact edits.** Long Python heredocs that contain `\u{2026}` style
  escapes break (Python reads them as its own escapes); write such scripts to a file with a raw
  string, or use the edit tool.
- **Sample to test with:** `samples/Morning Walk.gp`.
- The person has old eyes: keep text large and readable in new windows.
- Git warns "LF will be replaced by CRLF" on every commit; it is harmless here.

## AI plans (the assistant changing the score)

The assistant proposes a change as a fenced block tagged `guitarcraft-plan` holding a JSON list of
steps. `src/app/aiplan.rs` is the whole core and has no UI: `parse` reads the list, `run` checks
every step against the real song and carries it out (on a copy first, to show the card), and
`PLAN_HELP` is what the model is told. The steps are `copy_bars`, `paste_bars` (insert or
replace, at a bar or "end") and `delete_bars`; tracks and sections are named, and `find_track`
also takes a track number, as the outline lists them. `outline` (tracks, sections with their
bars) goes with every question except an order, which gets only the outline (see
`ai_context`), so the model can name them. There are no checkboxes: the program decides what goes with a question. `src/app/aipanel.rs` splits each answer into prose and
blocks (`split_blocks`) and draws a card for a plan (`plan_card`): the steps in words, the bar
count before and after, notes (bars of a different length), and Apply / Dismiss. Apply runs the
plan again on a copy of the song as it is now, and only then takes one undo snapshot and swaps it
in (`ai_apply_plan`), so a plan that no longer fits changes nothing and one Undo takes it back.
Nothing happens without Apply, and a chord chart refuses a plan.

Checked live with `qwen3:8b` on the Morning Walk sample (`cargo test plan_live -- --ignored
--nocapture`, set `PLAN_QUESTION`): "copy the intro and paste it past the last bar", "copy the
guitar track for Verse 1 and paste it into measure 12" and a three-step delete-and-copy all gave
valid cards (the model resolved "guitar track" and "Verse 1" to the real names). It chose insert
for "paste into measure 12"; the card says so and the person can ask for replace. An answer takes
about 100 seconds on the 8 GB card (the 8B model at 16K context spills to the CPU). Not tried:
Claude, the card on screen, a large song.
