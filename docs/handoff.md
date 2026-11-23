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
  Send button, the typing box uses 18 pt text (`INPUT_TEXT_SIZE`), and "What will be sent" shows
  the exact text. It answers in words and never edits the score.
- **Layout changes made to match GP8:** files open at 100% zoom (was 110%), mouse-wheel scrolling
  is twice as fast (`line_scroll_speed` 80 in `setup_style`), and note columns are narrower
  (`col_width` in `src/score.rs`) so Morning Walk's first three bars share a line.

## Known problems and things not checked

- **Three `dock` tests fail** (`the_layout_survives_saving_and_loading`,
  `odd_layouts_are_repaired_or_replaced`, `panels_start_with_sensible_proportions`) with a
  panic at `epaint-0.36.2/src/textures.rs:337`, headless. They fail the same way without any of
  this session's changes. The release workflow doesn't run tests, so they never blocked it.
- **The AI panel has been tried with Ollama only** (one live question, kept as the ignored test
  `ollama_answers_live`). Claude has never been called with a real key. The first Ollama answer
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
   still open: default provider, where "what is sent" lives, a spending limit, saving answers,
   and key storage outside Windows and macOS.
2. **Still missing from Guitar Pro 8** (see the gap list): Audio Note Settings (Shift+F),
   instrument change automation, Drum view, Design mode, and importing TablEdit and Power Tab.
   File password protection was decided against.
3. Try Claude with a real key and fix whatever shows up.
4. **Split score view** (an idea, not started): a movable splitter in the score view so two
   parts of a large score show at once. Off by default, switched on and off from the View menu
   (and the command palette) and remembered in prefs. `score()` in `src/app/bottom.rs` lays the
   score out once and draws it in one `ScrollArea` (`id_salt("score")`); a split would be two
   scroll areas over that same layout, so the cost is mostly painting. The drawing and
   interaction code inside that closure (drag select, click to place the cursor, hit tests,
   cursor and play position) would move into a helper that takes a pane index. Open choices:
   top/bottom, side by side or switchable; which pane follows the cursor and playback (suggest
   the one last clicked); zoom and view mode shared at first.

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
