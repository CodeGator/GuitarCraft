//! Saving, opening and exporting songs.

use crate::model::*;

/// Files saved by earlier versions of this app (still opened, never written).
pub const OLD_EXTS: [&str; 3] = ["gcraft", "tabf", "json"];

/// Songs are saved as Guitar Pro files: .gp (current) or .gp5 (older).
pub const GP_EXT: &str = "gp";
pub const GP5_EXT: &str = "gp5";

/// The lower-case extension of a path, or "" if it has none.
fn ext_of(path: &std::path::Path) -> String {
    path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
}

/// Saves a song as a Guitar Pro file: .gp5 for the older format, .gp (or no
/// extension) for the current one. Other extensions are refused, so a Guitar Pro
/// file never ends up named like a MIDI or text file.
pub fn save_song(song: &Song, path: &std::path::Path) -> Result<(), String> {
    let data = match ext_of(path).as_str() {
        GP5_EXT => crate::gp5write::export_gp5(song),
        GP_EXT | "" => crate::gpifwrite::export_gp(song)?,
        other => {
            return Err(format!(
                "Can't save as .{other}. Songs are saved as .{GP_EXT} or .{GP5_EXT}; \
                 use Export for MIDI or text tab."
            ));
        }
    };
    std::fs::write(path, data).map_err(|e| e.to_string())
}

/// Writes a song in the format its extension names: Guitar Pro, MIDI (.mid,
/// .midi) or text tab (.txt, .tab). Text tab holds one track, so it takes the first.
pub fn export_song(song: &Song, path: &std::path::Path) -> Result<(), String> {
    let data = match ext_of(path).as_str() {
        "mid" | "midi" => export_midi(song),
        "txt" | "tab" => export_ascii(song, 0).into_bytes(),
        _ => return save_song(song, path),
    };
    std::fs::write(path, data).map_err(|e| e.to_string())
}

/// What a Guitar Pro 5 file can't keep of this song, in plain words (empty if nothing).
/// The newer Guitar Pro format has all of these; Guitar Pro 5 predates them.
pub fn gp5_losses(song: &Song) -> Vec<&'static str> {
    let mut out = vec![];
    let beats = || {
        song.tracks.iter().flat_map(|t| {
            t.bars
                .iter()
                .flat_map(|b| (0..VOICES).flat_map(move |v| b.voice(v).iter()))
        })
    };
    if song
        .tracks
        .iter()
        .any(|t| t.bars.iter().any(|b| (2..VOICES).any(|v| b.voice_used(v))))
    {
        out.push("voices 3 and 4");
    }
    if song
        .tracks
        .iter()
        .any(|t| t.has_strings() && t.strings() > 7)
    {
        out.push("strings beyond the seventh");
    }
    if song.tracks.iter().any(|t| {
        !t.has_strings()
            && t.bars
                .iter()
                .any(|b| b.beats.iter().any(|x| x.notes.len() > 7))
    }) {
        out.push("chords of more than 7 notes");
    }
    if beats().any(|x| x.double_dot) {
        out.push("double dots");
    }
    if beats().any(|x| x.fermata) {
        out.push("fermatas");
    }
    if beats().any(|x| x.hairpin != 0) {
        out.push("crescendo/diminuendo signs");
    }
    if beats().any(|x| x.ottava != 0) {
        out.push("8va/8vb signs");
    }
    if beats().any(|x| x.lyric.is_some()) {
        out.push("lyrics typed on single beats");
    }
    if song.master.iter().any(|m| m.tempo_ramp) {
        out.push("gradual tempo changes");
    }
    if song.tracks.iter().any(|t| !t.short_name.is_empty()) {
        out.push("short track names");
    }
    if song.tracks.iter().any(|t| t.palm_mute > 0.0) {
        out.push("palm mute amount");
    }
    if song.tracks.iter().any(|t| {
        t.grand_staff
            != matches!(
                t.instrument,
                Instrument::Piano | Instrument::ElectricPiano | Instrument::Organ
            )
            && t.kind() == InstrumentType::Keyboard
    }) {
        out.push("single/grand staff choice");
    }
    out
}

pub fn load_song(path: &std::path::Path) -> Result<Song, String> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let data = std::fs::read(path).map_err(|e| e.to_string())?;
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "Untitled".into());
    let mut song = match ext.as_str() {
        "gp3" | "gp4" | "gp5" => crate::gpimport::import(&data)?,
        "gp" => crate::gpif::import_gp(&data)?,
        "gpx" => crate::gpif::import_gpx(&data)?,
        "mid" | "midi" => crate::textmidi::import_midi(&data, &stem)?,
        "txt" | "tab" => song_from_track(
            crate::textmidi::import_ascii(&String::from_utf8_lossy(&data))?,
            &stem,
        ),
        _ => serde_json::from_slice::<Song>(&data).map_err(|e| format!("Not a song file: {e}"))?,
    };
    song.normalize();
    Ok(song)
}

fn vlq(out: &mut Vec<u8>, mut v: u32) {
    let mut buf = [0u8; 5];
    let mut i = 4;
    buf[i] = (v & 0x7f) as u8;
    v >>= 7;
    while v > 0 {
        i -= 1;
        buf[i] = ((v & 0x7f) as u8) | 0x80;
        v >>= 7;
    }
    out.extend_from_slice(&buf[i..]);
}

fn chunk(out: &mut Vec<u8>, events: &mut [(u32, Vec<u8>)]) {
    events.sort_by_key(|e| e.0);
    let mut data = vec![];
    let mut last = 0;
    for (t, ev) in events.iter() {
        vlq(&mut data, t - last);
        last = *t;
        data.extend_from_slice(ev);
    }
    vlq(&mut data, 0);
    data.extend_from_slice(&[0xff, 0x2f, 0x00]);
    out.extend_from_slice(b"MTrk");
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(&data);
}

/// A MIDI meta event (0xFF, kind, length, body).
fn meta(kind: u8, body: &[u8]) -> Vec<u8> {
    let mut ev = vec![0xff, kind];
    vlq(&mut ev, body.len() as u32);
    ev.extend_from_slice(body);
    ev
}

/// Writes the song as a standard MIDI file: a conductor track (title, tempo, time
/// and key signatures, section markers) and one track per instrument. Repeats are
/// played out, since MIDI has no way to mark them.
pub fn export_midi(song: &Song) -> Vec<u8> {
    let order = song.play_order();
    let mut out = vec![];
    out.extend_from_slice(b"MThd");
    out.extend_from_slice(&6u32.to_be_bytes());
    out.extend_from_slice(&1u16.to_be_bytes());
    out.extend_from_slice(&((song.tracks.len() + 1) as u16).to_be_bytes());
    out.extend_from_slice(&(QUARTER as u16).to_be_bytes());

    // Tempo / time signature track.
    let mut tempo_ev = vec![];
    let mut t = 0u32;
    let mut last_tempo = 0;
    let mut last_sig = (0, 0);
    let mut last_key = None;
    for &bi in &order {
        let tempo = song.tempo_at(bi);
        if tempo != last_tempo {
            let us = 60_000_000 / tempo;
            tempo_ev.push((
                t,
                vec![
                    0xff,
                    0x51,
                    0x03,
                    (us >> 16) as u8,
                    (us >> 8) as u8,
                    us as u8,
                ],
            ));
            last_tempo = tempo;
        }
        let mb = &song.master[bi];
        if (mb.num, mb.den) != last_sig {
            let dd = (mb.den as f32).log2() as u8;
            tempo_ev.push((t, vec![0xff, 0x58, 0x04, mb.num, dd, 24, 8]));
            last_sig = (mb.num, mb.den);
        }
        if last_key != Some((mb.key, mb.key_minor)) {
            tempo_ev.push((t, meta(0x59, &[mb.key as u8, mb.key_minor as u8])));
            last_key = Some((mb.key, mb.key_minor));
        }
        if let Some(m) = &mb.marker {
            tempo_ev.push((t, meta(0x06, m.as_bytes())));
        }
        t += mb.ticks();
    }
    tempo_ev.insert(0, (0, meta(0x03, song.title.as_bytes())));
    chunk(&mut out, &mut tempo_ev);

    for (ti, tr) in song.tracks.iter().enumerate() {
        let ch = if tr.instrument == Instrument::Drums {
            9
        } else if ti >= 9 {
            (ti + 1) as u8
        } else {
            ti as u8
        } & 0x0f;
        let mut ev: Vec<(u32, Vec<u8>)> = vec![(0, meta(0x03, tr.name.as_bytes()))];
        ev.push((0, vec![0xC0 | ch, tr.instrument.midi_program()]));
        ev.push((
            0,
            vec![0xB0 | ch, 7, (tr.volume.clamp(0.0, 1.0) * 127.0) as u8],
        ));
        ev.push((
            0,
            vec![
                0xB0 | ch,
                10,
                ((tr.pan.clamp(-1.0, 1.0) + 1.0) * 63.5) as u8,
            ],
        ));
        for v in 0..VOICES {
            // Every beat of this voice in play order, with its start time.
            let mut seq: Vec<(u32, &Beat)> = vec![];
            let mut t = 0u32;
            for &bi in &order {
                let bar = &tr.bars[bi];
                if v == 0 || bar.voice_used(v) {
                    let mut tick = t;
                    for beat in bar.voice(v) {
                        seq.push((tick, beat));
                        tick += beat.ticks();
                    }
                }
                t += song.master[bi].ticks();
            }
            for (i, &(tick, beat)) in seq.iter().enumerate() {
                for n in beat.notes.iter().filter(|n| !n.tie) {
                    // A tied note continues this one, so hold it to the end of the tie.
                    let mut end = tick + beat.ticks();
                    for &(t2, b2) in &seq[i + 1..] {
                        match b2.note_on(n.string) {
                            Some(m) if m.tie && t2 == end => end = t2 + b2.ticks(),
                            _ => break,
                        }
                    }
                    let p = tr.pitch(n);
                    let vel = beat.dynamic.velocity().min(127);
                    ev.push((tick, vec![0x90 | ch, p, vel]));
                    let off = (end - tick).saturating_sub(10).max(1);
                    ev.push((tick + off, vec![0x80 | ch, p, 0]));
                }
            }
        }
        chunk(&mut out, &mut ev);
    }
    out
}

pub use crate::textmidi::export_ascii;

/// Builds a one-track song around an imported track.
pub fn song_from_track(track: Track, title: &str) -> Song {
    let mut s = Song::new_blank();
    s.title = title.to_string();
    s.master = (0..track.bars.len().max(1))
        .map(|_| MasterBar::default())
        .collect();
    s.tracks = vec![track];
    s.normalize();
    s
}

/// A 16-bit stereo WAV file.
pub fn wav_bytes(samples: &[f32], sr: u32) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&sr.to_le_bytes());
    out.extend_from_slice(&(sr * 4).to_le_bytes());
    out.extend_from_slice(&4u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for x in samples {
        out.extend_from_slice(&((x.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh, empty folder for one test's files.
    fn temp_dir(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("guitarcraft-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn save_refuses_formats_it_cannot_write() {
        let d = temp_dir("save-refuses");
        for name in ["song.mid", "song.txt", "song.json"] {
            let p = d.join(name);
            assert!(
                save_song(&Song::demo(), &p).is_err(),
                "{name} should be refused"
            );
            assert!(!p.exists(), "{name} should not be written");
        }
        assert!(save_song(&Song::demo(), &d.join("song")).is_ok());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn export_writes_the_format_the_extension_names() {
        let d = temp_dir("export");
        let song = Song::demo();
        for name in ["song.gp", "song.gp5", "song.mid", "song.txt"] {
            let p = d.join(name);
            export_song(&song, &p).unwrap();
            let back = load_song(&p).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert!(!back.tracks.is_empty(), "{name} came back empty");
        }
        let midi = std::fs::read(d.join("song.mid")).unwrap();
        assert!(midi.starts_with(b"MThd"));
        let _ = std::fs::remove_dir_all(&d);
    }
}
