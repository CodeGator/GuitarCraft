# Developing GuitarCraft

## Supported systems

| System | State |
| --- | --- |
| Windows 10 and 11 | Built and used every day |
| macOS and Linux | The libraries it uses run there, but it hasn't been built or tried yet |

## Building and running

1. Install Rust from <https://rustup.rs> (accept the defaults). On Windows, the installer also offers the Microsoft C++ build tools it needs.
2. Download this repository (**Code → Download ZIP** on GitHub, then unzip it), or clone it with git.
3. Open a terminal in the folder and run:

   ```sh
   cargo run --release
   ```

   The first build downloads the libraries it uses and takes a few minutes; after that it starts straight away. The program itself ends up in `target/release/`.

To open a song directly: `cargo run --release -- "path/to/song.gp"`. There are samples in `samples/`.

## Building and testing

```sh
cargo build --release   # the program, in target/release/
cargo test --workspace  # the tests, including the xtask helper commands
```

The tests check, among other things, that songs survive a round trip through `.gp` and `.gp5` files, and that GuitarCraft reads and writes marks, track settings and other parts of a `.gp` file the way Guitar Pro 8 does (checked against files saved by Guitar Pro 8, in `tests/fixtures/`; its README lists each file and the test that reads it). The tests are silent: they open no sound device or microphone.

## Command-line options

A few options help with checking and converting:

| Option | What it does |
| --- | --- |
| `--convert <from> <to>` | Opens a song (or `demo`) and saves it in the format of `<to>`, e.g. `.gp` or `.gp5` |
| `--export <from> <to.pdf\|to.png> [dpi]` | Prints a song, chord chart, `demo` or `chart` (the sample chart) to a PDF or PNG |
| `--screenshot <file.png>` | Draws the app, saves a picture and exits |
| `--show <names>` | With `--screenshot`, opens panels and windows first, e.g. `chart` or `band,drums,single` (see `src/app.rs`) |
| `--gp-tests <folder>` | Writes the demo song as several `.gp` test files |
| `--gpx-tests <folder>` | Writes `.gpx` test files, made from the demo song and from a `.gpx` saved by Guitar Pro 8 |

On Windows, GuitarCraft is a windowed app, so a terminal doesn't wait for it: its messages appear after the prompt returns. To wait for a run and get its exit code, use `Start-Process -Wait` in PowerShell or `start /wait` in cmd.

The readme's pictures are in `docs/readme/`, made with `--screenshot`: `editing.png` (no `--show`), `drums.png` (`--show band,drums,single`), `chart.png` (`--show chart`), and `scales.png`, `arpeggios.png`, `chords.png` and `identify.png` (`--show` with the same name), cropped to the Chords & Scales window.

## Releasing

The version is kept in one place, `Cargo.toml`, and is changed with `cargo xtask version set X.Y.Z`. `packaging/windows/package.ps1` builds the Windows installer and a portable zip. See [releasing.md](releasing.md).
