# GuitarCraft codebase audit: simplification plan

15 Nov 2026 · Branch: `audit` · Status: proposed (nothing changed yet)

## Summary

The code is in good shape. The build gives no warnings, clippy reports only 11 minor style lints, and all 73 tests pass (1 ignored). The problems are copies of the same small helpers in different files, plus a few comments, constants and test hooks that were true once but are now out of date. Doing everything below removes about 250–300 lines. Only three items change behaviour, and each is noted where it appears (A2, B2, B6).

## A. Assumptions that no longer hold

### A1. A comment saying two symbols aren't drawn, when they are

- **Evidence:** `src/style.rs:202-205` marks `SYM_RASG` and `SYM_WHAMMY` with `#[allow(dead_code)]` and the comment "no drawing code reads it yet". But `src/score/draw.rs:2002` and `src/score/draw.rs:2012` both use them.
- **Change:** delete the two `#[allow]` lines and their comments.
- **Behaviour:** none.

### A2. The home-screen preview assumes every page is A4 portrait

- **Evidence:** `src/app/home.rs:534` sets the preview zoom from the fixed `score::PAGE_W` and `PAGE_H` (820×1160). Page size is now set by the stylesheet (`style.rs:711 page_mm()`), and the layout uses that (`score.rs:693`).
- **Problem:** a Letter or landscape template is scaled as if it were A4, so its preview can be too large or too small.
- **Change:** work out the zoom from `song.style.page_mm()` multiplied by `style::MM`. After that, `score::PAGE_W` and `PAGE_H` (`score.rs:24-25`) aren't used anywhere and can be deleted.
- **Behaviour:** no change for A4 portrait; other page sizes now fit the preview correctly.

### A3. The `--gp-tests` mode was for an experiment that's finished (needs a decision)

- **Evidence:** `src/main.rs:108-124` writes "Check 2/3/4" files to test whether Guitar Pro 8 opens files that contain chord names. `GpOptions::safe` (`gpifwrite.rs:150`) already records the answer: "chord names (Guitar Pro 8 opens them)". The `chords: false` option is used only by this mode.
- **Change:** delete `--gp-tests` and the `chords` field. `views` can then be worked out from the song inside `gpifwrite`, so `GpOptions` goes away. `export_gp_with` merges into `export_gp`, and `gpif_xml_with` into `gpif_xml`.
- **Saving:** about 40 lines and one public type.
- **Behaviour:** normal saving is unchanged.
- **Decision:** `samples/README.txt` still mentions `--gp-tests`. Keep it if you still use it to check Guitar Pro.

### A4. Upgrade code for chart files that may never have existed outside development (needs a decision)

- **Evidence:** `src/chart.rs:538-559` upgrades "format 1" charts and charts that kept `columns` and `show_diagrams` outside the stylesheet. Two tests cover this (`chart.rs:1355`, `chart.rs:1439`). But chord charts arrived on main all at once (PR #4), and the sample file is already format 2.
- **Change:** if no old-format chart files need to be kept, delete `FORMAT_1`, the upgrade code and those two tests (about 30 lines).
- **Decision:** are there any format-1 or pre-stylesheet chart files worth keeping?

## B. Duplicated logic

### B1. The blue highlight colour is defined 10 times

- **Evidence:** `Color32::from_rgb(61, 123, 224)` appears as a named constant in `chartui.rs:14`, `icons.rs:7`, `panels.rs:12`, `stylesheet.rs:15` and `tuning.rs:11`. It's also written out in full in `clipmenu.rs:115`, `popups.rs:342`, `popups.rs:522`, `printing.rs:236` and `printing.rs:307`.
- **Change:** keep `panels::BLUE`, which is already public, and point everything else at it.
- **Behaviour:** none.

### B2. Three copies of the On/Off toggle switch

- **Evidence:** `side.rs:510 toggle_switch`, `stylesheet.rs:125 switch` and `tuning.rs:669 switch` are the same widget. The only differences are its size (30×16 or 32×17) and one quirk: the tuning copy draws before it reacts to the click, so the new state shows one frame late.
- **Change:** keep `stylesheet::switch` with a size argument and delete the other two (about 45 lines).
- **Behaviour:** the same, except the tuning switch no longer shows the new state a frame late.

### B3. The settings-folder lookup is repeated five times

- **Evidence:** "use the APPDATA folder, otherwise HOME, then add GuitarCraft" is written out in `home.rs:10`, `actions.rs:226`, `stylesheet.rs:40`, `stylesheet.rs:45`, `chartstyle.rs:25` and `chartstyle.rs:30`.
- **Evidence:** "list a folder's files with one extension, sorted" is copied three times: `stylesheet.rs:63`, `chartstyle.rs:55` and `actions.rs:231`.
- **Extra:** `home::config_dir(app)` takes an app name, but its only call passes `"GuitarCraft"`.
- **Change:** add `app_dir()` and `files_with_ext(dir, ext)` to `home.rs` and use them in all of these places (about 40 lines).
- **Behaviour:** the same folders and files.

### B4. Two near-identical tab bars in the stylesheet windows

- **Evidence:** `stylesheet.rs:465` and `chartstyle.rs:163` share the same layout, clicking, highlight and caption code. Only the icons in the `match i` differ.
- **Change:** one `tab_strip(ui, names, salt, &mut tab, draw_icon)` that each window passes its own icon drawing to (about 30 lines).
- **Behaviour:** none.

### B5. Note names are listed three times

- **Evidence:** the sharp names appear in `model.rs:960`, `tuning.rs:31` and `chart.rs:626`; the flat names in `tuning.rs:34` and `chart.rs:627`. `score.rs:548 sharp_to_flat` changes sharp spellings to flats by hand.
- **Change:** put public `SHARPS` and `FLATS` tables in `model.rs` and use them in all four places.
- **Behaviour:** none.

### B6. Small helpers written twice

| Duplicate | Where | Keep |
|---|---|---|
| Roman numerals 1–24 | `palette.rs:919 roman`, `draw.rs:2085 roman_numeral` | the `&'static str` version |
| Time as m:ss | `widgets.rs:23 fmt_time`, written out again at `draw.rs:2031` | `fmt_time` (move it next to the model) |
| Ticks before a beat | `draw.rs:2075 beat_start` and `model.rs:437 Bar::beat_start` | `vbar.beat_start(0, bj)` |
| Seconds into the song | `draw.rs:2080 time_at` (f32), `gpifwrite.rs:900 seconds_at` (f64, rounded down) | `Song::seconds_at`; the score timer rounds the f64 value down (it may differ by a second in rare rounding cases) |
| Success/failure exit | `main.rs:54`, `76` and `101` | one `exit_with(r)` |
| Enabled menu item | `menus.rs:13 item_if`, `clipmenu.rs:19 item` | `item_if` |

### B7. A wrapper with a single caller

- **Evidence:** `gpifwrite.rs:743 write_beat` only passes `grace = false` on to `write_beat_ex`, and it's called once (`gpifwrite.rs:529`).
- **Change:** call `write_beat_ex(..., false)` there and delete the wrapper.
- **Behaviour:** none.

## C. Deliberately left alone

- **Headings:** the `heading()` copies in `chartui.rs`, `side.rs` and `stylesheet.rs` use different sizes (14, 15 and 17), so they look different on purpose and merging them would change the look.
- **Reading/writing pairs:** `gp5write` W / `gpimport` R, and `fileio::vlq` / `textmidi::read_vlq`, mirror each other. They aren't copies.
- **Panel greys:** `chartui.rs:13` uses (44,47,54), `panels.rs:10` uses (42,45,53) and `main.rs:263` uses (44,47,53). They're almost the same. Making them identical is a design choice, so it's left to you.

## Plan

1. A1, B1, B6, B7 and A2 first: the smallest and safest.
2. Then B2, B3, B4 and B5.
3. A3 and A4 once they're decided.

After each step, run `cargo test` (baseline: 73 passed, 1 ignored) and take a quick look at the app.
