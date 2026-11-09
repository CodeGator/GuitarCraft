# GuitarCraft

A desktop app for writing, editing and playing guitar music: tablature and
standard notation, multiple tracks (guitar, bass, keyboards, vocals, drums and
more), four voices per bar, a built-in synthesizer for playback, a mixer, and
tools such as a tuning window, chord and scale views, lyrics and a speed trainer.

Songs are saved as `.gp` files (and `.gp5` if you prefer the older format).
GuitarCraft also opens `.gpx`, `.gp3`, `.gp4`, MIDI and plain-text tab files.

## Building and running

1. Install Rust from <https://rustup.rs>.
2. In this folder, run:

   ```
   cargo run --release
   ```

   The first build downloads the libraries it needs and takes a few minutes.

To open a song directly: `cargo run --release -- path/to/song.gp`

## Tests

```
cargo test
```

`samples/Empty.gp` is a small song file used by the tests.

## Folders

- `src/` – the program
  - `model.rs` – songs, tracks, bars, notes
  - `score.rs`, `score/` – laying out and drawing the score
  - `audio.rs` – the synthesizer and playback
  - `gpif.rs`, `gpifwrite.rs`, `gpsv.rs` – reading and writing `.gp` / `.gpx` files
  - `gpimport.rs`, `gp5write.rs` – reading `.gp3`–`.gp5` and writing `.gp5`
  - `textmidi.rs` – MIDI and text tab import/export
  - `app.rs`, `app/` – the windows, panels, menus and editing
- `samples/` – song files used by the tests
