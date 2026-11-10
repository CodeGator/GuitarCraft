//! Reads Guitar Pro 3, 4 and 5 song files (.gp3/.gp4/.gp5), following the
//! community-documented layout of these formats. Drum tracks are skipped.

use crate::model::*;

struct R<'a> {
    d: &'a [u8],
    p: usize,
}

type Res<T> = Result<T, String>;

impl<'a> R<'a> {
    fn need(&self, n: usize) -> Res<()> {
        if self.p + n > self.d.len() {
            Err("The file ended early; it may be damaged or a format this app can't read.".into())
        } else {
            Ok(())
        }
    }
    fn u8(&mut self) -> Res<u8> {
        self.need(1)?;
        self.p += 1;
        Ok(self.d[self.p - 1])
    }
    fn i8(&mut self) -> Res<i8> {
        Ok(self.u8()? as i8)
    }
    fn i16(&mut self) -> Res<i16> {
        self.need(2)?;
        let v = i16::from_le_bytes([self.d[self.p], self.d[self.p + 1]]);
        self.p += 2;
        Ok(v)
    }
    fn i32(&mut self) -> Res<i32> {
        self.need(4)?;
        let v = i32::from_le_bytes([
            self.d[self.p],
            self.d[self.p + 1],
            self.d[self.p + 2],
            self.d[self.p + 3],
        ]);
        self.p += 4;
        Ok(v)
    }
    fn skip(&mut self, n: usize) -> Res<()> {
        self.need(n)?;
        self.p += n;
        Ok(())
    }
    fn chars(&mut self, n: usize, keep: usize) -> Res<String> {
        self.need(n)?;
        let s: String = self.d[self.p..self.p + keep.min(n)]
            .iter()
            .map(|&b| b as char)
            .collect();
        self.p += n;
        Ok(s)
    }
    /// Byte length followed by a fixed-size block.
    fn byte_str(&mut self, size: usize) -> Res<String> {
        let n = self.u8()? as usize;
        self.chars(size, n)
    }
    /// Int total size, then byte length, then text.
    fn int_byte_str(&mut self) -> Res<String> {
        let d = self.i32()?;
        let n = self.u8()? as usize;
        let size = (d - 1).max(0) as usize;
        self.chars(size, n)
    }
    fn int_str(&mut self) -> Res<String> {
        let n = self.i32()?.max(0) as usize;
        self.chars(n, n)
    }
}

#[derive(Clone, Copy, PartialEq, PartialOrd)]
enum V {
    Gp3,
    Gp4,
    Gp50,
    Gp51,
}

pub fn import(data: &[u8]) -> Result<Song, String> {
    let mut r = R { d: data, p: 0 };
    let ver = r.byte_str(30)?;
    let v = if ver.contains("v3.") {
        V::Gp3
    } else if ver.contains("4.") {
        V::Gp4
    } else if ver.contains("5.00") {
        V::Gp50
    } else if ver.contains("5.") {
        V::Gp51
    } else {
        return Err(format!(
            "Unrecognised file version \"{ver}\". Only Guitar Pro 3, 4 and 5 files can be opened."
        ));
    };
    let mut song = Song::new_blank();
    song.title = r.int_byte_str()?;
    song.subtitle = r.int_byte_str()?;
    song.artist = r.int_byte_str()?;
    song.album = r.int_byte_str()?;
    let words = r.int_byte_str()?;
    if v >= V::Gp50 {
        song.words = words;
        song.music_by = r.int_byte_str()?;
    } else {
        song.music_by = words;
    }
    song.copyright = r.int_byte_str()?;
    song.tabbed_by = r.int_byte_str()?;
    song.instructions = r.int_byte_str()?;
    let notice = r.i32()?;
    let mut lines = vec![];
    for _ in 0..notice.clamp(0, 256) {
        lines.push(r.int_byte_str()?);
    }
    song.notice = lines.join("\n");
    if v <= V::Gp4 {
        r.u8()?; // triplet feel
    }
    let mut lyrics: Option<(usize, usize, String)> = None;
    if v >= V::Gp4 {
        let lt = r.i32()?;
        for k in 0..5 {
            let from = r.i32()?;
            let text = r.int_str()?;
            if k == 0 && lt > 0 && !text.trim().is_empty() {
                lyrics = Some((lt as usize - 1, (from - 1).max(0) as usize, text));
            }
        }
    }
    if v >= V::Gp50 {
        if v == V::Gp51 {
            r.skip(4 + 4 + 11)?; // master effect
        }
        r.skip(7 * 4 + 2)?; // page setup
        for _ in 0..10 {
            r.int_byte_str()?;
        }
        r.int_byte_str()?; // tempo name
    }
    song.tempo = r.i32()?.clamp(20, 400) as u32;
    if v == V::Gp51 {
        r.u8()?;
    }
    if v >= V::Gp50 {
        r.i8()?;
        r.i32()?;
    } else if v == V::Gp4 {
        r.i32()?;
        r.i8()?;
    } else {
        r.i32()?;
    }
    // MIDI channels.
    let mut channels = vec![];
    for _ in 0..64 {
        let inst = r.i32()?;
        let vol = r.u8()?;
        let bal = r.u8()?;
        r.skip(4 + 2)?;
        channels.push((inst, vol, bal));
    }
    let mut dirs = [-1i16; 19];
    if v >= V::Gp50 {
        for d in dirs.iter_mut() {
            *d = r.i16()?;
        }
        r.i32()?; // master reverb
    }
    let measures = r.i32()?;
    let ntracks = r.i32()?;
    if !(0..=4096).contains(&measures) || !(1..=128).contains(&ntracks) {
        return Err("This file has an unexpected layout.".into());
    }
    let measures = measures as usize;
    let ntracks = ntracks as usize;

    // Measure headers.
    let mut master = vec![];
    let mut num = 4u8;
    let mut den = 4u8;
    for i in 0..measures {
        if v >= V::Gp50 && i > 0 {
            r.skip(1)?;
        }
        let flags = r.u8()?;
        let mut mb = MasterBar::default();
        if flags & 0x01 != 0 {
            num = r.i8()?.max(1) as u8;
        }
        if flags & 0x02 != 0 {
            den = r.i8()?.max(1) as u8;
        }
        mb.num = num;
        mb.den = den;
        mb.repeat_open = flags & 0x04 != 0;
        if v >= V::Gp50 {
            if flags & 0x08 != 0 {
                mb.repeat_close = r.i8()?.max(2) as u8;
            }
            if flags & 0x20 != 0 {
                mb.marker = Some(r.int_byte_str()?);
                r.skip(4)?;
            }
            if flags & 0x40 != 0 {
                mb.key = r.i8()?.clamp(-7, 7);
                mb.key_minor = r.i8()? == 1;
            } else if let Some(p) = master.last() {
                let p: &MasterBar = p;
                mb.key = p.key;
                mb.key_minor = p.key_minor;
            }
            if flags & 0x10 != 0 {
                mb.endings = r.u8()?;
            }
            if flags & 0x03 != 0 {
                r.skip(4)?;
            }
            if flags & 0x10 == 0 {
                r.skip(1)?;
            }
            mb.triplet_feel = r.u8()? == 1;
            mb.double_bar = flags & 0x80 != 0;
        } else {
            if flags & 0x08 != 0 {
                mb.repeat_close = (r.i8()?.max(1) as u8).saturating_add(1);
            }
            if flags & 0x10 != 0 {
                // GP3/4 store the ending number; turn it into a bit.
                let e = r.u8()?;
                if (1..=8).contains(&e) {
                    mb.endings = 1 << (e - 1);
                }
            }
            if flags & 0x20 != 0 {
                mb.marker = Some(r.int_byte_str()?);
                r.skip(4)?;
            }
            if flags & 0x40 != 0 {
                mb.key = r.i8()?.clamp(-7, 7);
                r.skip(1)?;
            }
            mb.double_bar = flags & 0x80 != 0;
        }
        master.push(mb);
    }

    // Directions (Segno, Coda, Fine and the jumps to them).
    for (slot, &bar) in dirs.iter().enumerate() {
        if bar >= 1 && (bar as usize) <= master.len() {
            let mb = &mut master[bar as usize - 1];
            let (t, j) = gp5_direction_of_slot(slot);
            if t != 0 && mb.direction_target == 0 {
                mb.direction_target = t;
            }
            if j != 0 && mb.direction_jump == 0 {
                mb.direction_jump = j;
            }
        }
    }

    // Tracks.
    let mut tracks = vec![];
    let mut drums = vec![];
    for i in 0..ntracks {
        if v >= V::Gp50 && (i == 0 || v == V::Gp50) {
            r.skip(1)?;
        }
        let flags = r.u8()?;
        let name = r.byte_str(40)?;
        let nstr = r.i32()?.clamp(1, 7) as usize;
        let mut tuning = vec![];
        for k in 0..7 {
            let t = r.i32()?;
            if k < nstr {
                tuning.push(t.clamp(0, 127) as u8);
            }
        }
        let port = r.i32()?;
        let ch = r.i32()?;
        let _eff = r.i32()?;
        let frets = r.i32()?;
        let capo = r.i32()?;
        let col = [r.u8()?, r.u8()?, r.u8()?];
        r.u8()?;
        let mut shown = (true, true, false);
        if v >= V::Gp50 {
            let f2 = r.i16()?;
            shown = (f2 & 0x0002 != 0, f2 & 0x0001 != 0, f2 & 0x0200 != 0);
            r.u8()?;
            r.u8()?;
            // RSE settings.
            r.u8()?;
            r.skip(12 + 12)?;
            r.skip(16)?;
            if v == V::Gp51 {
                r.skip(4)?;
                r.int_byte_str()?;
                r.int_byte_str()?;
            }
        }
        let idx = (((port - 1).max(0) * 16 + (ch - 1).max(0)) as usize).min(63);
        let (inst, vol, bal) = channels[idx];
        let is_drum = flags & 0x01 != 0 || ch == 10;
        drums.push(is_drum);
        let instrument = Instrument::from_midi_program(inst.clamp(0, 127) as u8);
        let mut t = Track {
            name: if name.trim().is_empty() {
                format!("Track {}", i + 1)
            } else {
                name
            },
            instrument,
            tuning,
            capo: capo.clamp(0, 24) as u8,
            frets: frets.clamp(12, 36) as u8,
            color: col,
            volume: (vol as f32 / 16.0).clamp(0.0, 1.0),
            pan: ((bal as f32 - 8.0) / 8.0).clamp(-1.0, 1.0),
            mute: false,
            solo: false,
            visible: true,
            bars: vec![],
            ..Track::default()
        };
        if t.tuning.len() == 4 && t.tuning.iter().all(|&x| x < 50) {
            t.instrument = Instrument::Bass;
        }
        // Which staves the track shows, and auto let ring.
        t.show_std = shown.0 || !shown.1;
        t.show_tab = shown.1 || !shown.0;
        t.auto_let_ring = shown.2;
        if !t.has_strings() && t.tuning.iter().all(|&x| x == 0) {
            // Saved by this app: the fret is the pitch.
            t.frets = 127;
        }
        if is_drum {
            // Drum tracks keep their notes as General MIDI drum numbers on the frets.
            t.instrument = Instrument::Drums;
            t.tuning = vec![0; t.tuning.len().max(1)];
            t.frets = 127;
            t.capo = 0;
        } else if !t.has_strings() {
            t.grand_staff = matches!(
                t.instrument,
                Instrument::Piano | Instrument::ElectricPiano | Instrument::Organ
            );
        }
        tracks.push(t);
    }
    if v >= V::Gp50 {
        r.skip(if v == V::Gp50 { 2 } else { 1 })?;
    }

    // Measures.
    let mut last_fret = vec![[0u8; 8]; ntracks];
    for mb in master.iter_mut() {
        for ti in 0..ntracks {
            let voices = if v >= V::Gp50 { 2 } else { 1 };
            let mut bar = Bar::new(vec![]);
            for voice in 0..voices {
                let nb = r.i32()?;
                if !(0..=512).contains(&nb) {
                    return Err("This file has an unexpected layout.".into());
                }
                for _ in 0..nb {
                    let beat = read_beat(&mut r, v, &tracks[ti], &mut last_fret[ti], mb)?;
                    if voice == 0 {
                        bar.beats.push(beat);
                    } else {
                        if bar.voices.is_empty() {
                            bar.voices.push(vec![]);
                        }
                        bar.voices[0].push(beat);
                    }
                }
            }
            if v >= V::Gp50 {
                r.u8()?;
            }
            if bar.beats.is_empty() {
                bar.beats = vec![Beat::rest(Dur::Whole)];
            }
            // A second voice with nothing in it is left out.
            if bar
                .voices
                .first()
                .map(|v| v.iter().all(|b| b.is_rest()))
                .unwrap_or(false)
            {
                bar.voices.clear();
            }
            tracks[ti].bars.push(bar);
        }
    }
    song.master = master;
    let _ = drums;
    if let Some((k, from, text)) = lyrics {
        if let Some(t) = tracks.get_mut(k) {
            t.lyrics = text;
            t.lyrics_bar = from;
        }
    }
    song.tracks = tracks;
    if song.tracks.is_empty() {
        return Err("This file only has drum tracks, which this app can't show yet.".into());
    }
    song.normalize();
    Ok(song)
}

fn read_bend(r: &mut R) -> Res<u8> {
    r.u8()?;
    r.i32()?;
    let n = r.i32()?.clamp(0, 64);
    let mut maxv = 0;
    for _ in 0..n {
        r.i32()?;
        let val = r.i32()?;
        r.u8()?;
        maxv = maxv.max(val);
    }
    Ok(((maxv as f32 / 25.0).round().clamp(0.0, 12.0)) as u8)
}

fn read_chord(r: &mut R, v: V) -> Res<Option<String>> {
    let header = r.u8()?;
    if header & 0x01 == 0 {
        let name = r.int_byte_str()?;
        let first = r.i32()?;
        if first != 0 {
            r.skip(6 * 4)?;
        }
        return Ok(Some(name));
    }
    if v == V::Gp3 {
        r.skip(25)?;
        let name = r.byte_str(34)?;
        r.i32()?;
        r.skip(6 * 4)?;
        r.skip(36)?;
        Ok(Some(name))
    } else {
        r.skip(16)?;
        let name = r.byte_str(21)?;
        r.skip(4)?;
        r.i32()?;
        r.skip(7 * 4)?;
        r.skip(32)?;
        Ok(Some(name))
    }
}

fn read_beat(r: &mut R, v: V, track: &Track, last: &mut [u8; 8], mb: &mut MasterBar) -> Res<Beat> {
    let flags = r.u8()?;
    let mut beat = Beat::rest(Dur::Quarter);
    if flags & 0x40 != 0 {
        r.u8()?;
    }
    let d = r.i8()?;
    beat.dur = Dur::from_index((d as i32 + 2).clamp(0, 6) as usize);
    beat.dotted = flags & 0x01 != 0;
    if flags & 0x20 != 0 {
        let t = r.i32()?;
        beat.tuplet = match t {
            3 | 5 | 6 | 7 | 9 => t as u8,
            _ => 0,
        };
    }
    if flags & 0x02 != 0 {
        beat.chord = read_chord(r, v)?.filter(|s| !s.trim().is_empty());
    }
    if flags & 0x04 != 0 {
        let t = r.int_byte_str()?;
        if !t.trim().is_empty() {
            beat.text = Some(t);
        }
    }
    let mut beat_vibrato = false;
    let mut beat_harm = false;
    if flags & 0x08 != 0 {
        if v == V::Gp3 {
            let f1 = r.u8()?;
            beat_vibrato = f1 & 0x03 != 0;
            beat_harm = f1 & 0x04 != 0;
            if f1 & 0x20 != 0 {
                r.u8()?;
                r.i32()?;
            }
            if f1 & 0x40 != 0 {
                let down = r.u8()?;
                let up = r.u8()?;
                beat.stroke = if down > 0 {
                    Stroke::Down
                } else if up > 0 {
                    Stroke::Up
                } else {
                    Stroke::None
                };
            }
        } else {
            let f1 = r.u8()?;
            let f2 = r.u8()?;
            beat_vibrato = f1 & 0x03 != 0;
            beat.fade_in = f1 & 0x10 != 0;
            if f1 & 0x20 != 0 {
                let t = r.u8()?;
                if (1..=3).contains(&t) {
                    beat.technique = t;
                }
            }
            if f2 & 0x04 != 0 {
                read_bend(r)?;
            }
            if f1 & 0x40 != 0 {
                let down = r.u8()?;
                let up = r.u8()?;
                beat.stroke = if down > 0 {
                    Stroke::Down
                } else if up > 0 {
                    Stroke::Up
                } else {
                    Stroke::None
                };
            }
            if f2 & 0x02 != 0 {
                let ps = r.u8()?;
                beat.pick = match ps {
                    1 => 2,
                    2 => 1,
                    _ => 0,
                };
            }
        }
    }
    if flags & 0x10 != 0 {
        // Mix table change.
        r.i8()?;
        if v >= V::Gp50 {
            r.skip(16)?;
        }
        let mut vals = [0i8; 6];
        for x in vals.iter_mut() {
            *x = r.i8()?;
        }
        if v >= V::Gp50 {
            r.int_byte_str()?;
        }
        let tempo = r.i32()?;
        for x in vals {
            if x >= 0 {
                r.u8()?;
            }
        }
        if tempo >= 0 {
            r.u8()?;
            if v == V::Gp51 {
                r.u8()?;
            }
            mb.tempo = Some(tempo.clamp(20, 400) as u32);
        }
        if v >= V::Gp4 {
            r.u8()?;
        }
        if v >= V::Gp50 {
            r.i8()?;
        }
        if v == V::Gp51 {
            r.int_byte_str()?;
            r.int_byte_str()?;
        }
    }
    let sflags = r.u8()?;
    let n = track.tuning.len();
    for s in 0..7usize {
        if sflags & (1 << (6 - s)) != 0 {
            let note = read_note(r, v, s, last)?;
            if let Some((mut note, dynv)) = note {
                if beat_vibrato {
                    note.vibrato = true;
                }
                if beat_harm {
                    note.harmonic = true;
                }
                if s < n {
                    if let Some(d) = dynv {
                        beat.dynamic = d;
                    }
                    beat.notes.push(note);
                }
            }
        }
    }
    if v >= V::Gp50 {
        let f2 = r.i16()?;
        if f2 & 0x0800 != 0 {
            r.u8()?;
        }
    }
    Ok(beat)
}

fn read_note(
    r: &mut R,
    v: V,
    s: usize,
    last: &mut [u8; 8],
) -> Res<Option<(Note, Option<Dynamic>)>> {
    let mut dynv = None;
    let flags = r.u8()?;
    let mut n = Note::new(s as u8, 0);
    n.ghost = flags & 0x04 != 0;
    n.accent = flags & 0x40 != 0 || flags & 0x02 != 0;
    let mut kind = 1;
    if flags & 0x20 != 0 {
        kind = r.u8()?;
    }
    if v <= V::Gp4 && flags & 0x01 != 0 {
        r.skip(2)?;
    }
    if flags & 0x10 != 0 {
        let x = r.i8()?;
        dynv = Some(Dynamic::ALL[(x.clamp(1, 8) - 1) as usize]);
    }
    if flags & 0x20 != 0 {
        let f = r.i8()?;
        n.fret = f.clamp(0, 127) as u8;
    }
    if flags & 0x80 != 0 {
        r.skip(2)?;
    }
    if v >= V::Gp50 {
        if flags & 0x01 != 0 {
            r.skip(8)?;
        }
        r.u8()?;
    }
    match kind {
        2 => {
            n.tie = true;
            n.fret = last[s];
        }
        3 => n.dead = true,
        _ => {}
    }
    last[s] = n.fret;
    if flags & 0x08 != 0 {
        if v == V::Gp3 {
            let f1 = r.u8()?;
            if f1 & 0x01 != 0 {
                n.bend = read_bend(r)?;
            }
            n.hammer = f1 & 0x02 != 0;
            n.slide = f1 & 0x04 != 0;
            n.let_ring = f1 & 0x08 != 0;
            if f1 & 0x10 != 0 {
                let g = r.u8()?;
                n.grace = Some(g.min(30));
                r.skip(3)?;
            }
        } else {
            let f1 = r.u8()?;
            let f2 = r.u8()?;
            if f1 & 0x01 != 0 {
                n.bend = read_bend(r)?;
            }
            n.hammer = f1 & 0x02 != 0;
            n.let_ring = f1 & 0x08 != 0;
            if f1 & 0x10 != 0 {
                let g = r.u8()?;
                n.grace = Some(g.min(30));
                r.skip(if v >= V::Gp50 { 4 } else { 3 })?;
            }
            n.staccato = f2 & 0x01 != 0;
            n.palm_mute = f2 & 0x02 != 0;
            if f2 & 0x04 != 0 {
                r.u8()?;
            }
            if f2 & 0x08 != 0 {
                let st = r.i8()?;
                n.slide = if v >= V::Gp50 {
                    st & 0x03 != 0
                } else {
                    st == 1 || st == 2
                };
                n.slide_out = if v >= V::Gp50 {
                    st & 0x0C != 0
                } else {
                    st == 3 || st == 4
                };
                n.slide_in = if v >= V::Gp50 { st & 0x30 != 0 } else { st < 0 };
            }
            if f2 & 0x10 != 0 {
                let h = r.i8()?;
                if h == 4 || (v < V::Gp50 && h == 15) {
                    n.pinch_harmonic = true;
                } else {
                    n.harmonic = true;
                }
                if v >= V::Gp50 {
                    if h == 2 {
                        r.skip(3)?;
                    } else if h == 3 {
                        r.skip(1)?;
                    }
                }
            }
            if f2 & 0x20 != 0 {
                let tf = r.u8()?;
                r.u8()?;
                n.trill = tf.min(30);
            }
            n.vibrato = f2 & 0x40 != 0;
        }
    }
    Ok(Some((n, dynv)))
}
