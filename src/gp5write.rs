//! Writes songs as Guitar Pro 5 files (.gp5, version 5.10), following the
//! community-documented layout of the format (the same one the reader uses).
//! Guitar Pro 5 keeps two voices per bar and up to seven strings per track.

use crate::model::*;

struct W {
    b: Vec<u8>,
}

impl W {
    fn u8(&mut self, v: u8) {
        self.b.push(v);
    }
    fn i8(&mut self, v: i8) {
        self.b.push(v as u8);
    }
    fn i16(&mut self, v: i16) {
        self.b.extend_from_slice(&v.to_le_bytes());
    }
    fn i32(&mut self, v: i32) {
        self.b.extend_from_slice(&v.to_le_bytes());
    }
    fn zeros(&mut self, n: usize) {
        self.b.extend(std::iter::repeat_n(0u8, n));
    }
    /// Text as single bytes (Latin-1); other characters become '?'.
    fn latin(s: &str) -> Vec<u8> {
        s.chars()
            .map(|c| {
                if (c as u32) < 256 {
                    c as u32 as u8
                } else {
                    b'?'
                }
            })
            .collect()
    }
    /// Byte length, then a block of fixed size.
    fn byte_str(&mut self, s: &str, size: usize) {
        let mut t = W::latin(s);
        t.truncate(size.min(255));
        self.u8(t.len() as u8);
        let n = t.len();
        self.b.extend_from_slice(&t);
        self.zeros(size - n);
    }
    /// Int total size, byte length, text.
    fn int_byte_str(&mut self, s: &str) {
        let mut t = W::latin(s);
        t.truncate(255);
        self.i32(t.len() as i32 + 1);
        self.u8(t.len() as u8);
        self.b.extend_from_slice(&t);
    }
    /// Int length, text.
    fn int_str(&mut self, s: &str) {
        let t = W::latin(s);
        self.i32(t.len() as i32);
        self.b.extend_from_slice(&t);
    }
}

/// MIDI channel (0..63, over four ports) for each track: drums on channel 10,
/// the others in pairs (main and effect channel).
fn channels(song: &Song) -> Vec<(usize, usize)> {
    let mut free = (0..64usize).filter(|c| c % 16 != 9);
    song.tracks
        .iter()
        .map(|t| {
            if t.instrument == Instrument::Drums {
                (9, 9)
            } else {
                let a = free.next().unwrap_or(0);
                let b = free.next().unwrap_or(a);
                (a, b)
            }
        })
        .collect()
}

pub fn export_gp5(song: &Song) -> Vec<u8> {
    let mut w = W {
        b: Vec::with_capacity(64 * 1024),
    };
    w.byte_str("FICHIER GUITAR PRO v5.10", 30);
    // Song information.
    for s in [
        &song.title,
        &song.subtitle,
        &song.artist,
        &song.album,
        &song.words,
        &song.music_by,
        &song.copyright,
        &song.tabbed_by,
        &song.instructions,
    ] {
        w.int_byte_str(s);
    }
    // The notice, line by line.
    let notice: Vec<&str> = if song.notice.is_empty() {
        vec![]
    } else {
        song.notice.lines().collect()
    };
    w.i32(notice.len() as i32);
    for l in notice {
        w.int_byte_str(l);
    }
    // Lyrics: the first track that has some.
    let lyr = song.tracks.iter().position(|t| !t.lyrics.trim().is_empty());
    w.i32(lyr.map(|k| k as i32 + 1).unwrap_or(0));
    for line in 0..5 {
        match (line, lyr) {
            (0, Some(k)) => {
                w.i32(song.tracks[k].lyrics_bar as i32 + 1);
                w.int_str(&song.tracks[k].lyrics);
            }
            _ => {
                w.i32(1);
                w.int_str("");
            }
        }
    }
    // Master effect: volume, reverb, equalizer (10 bands and gain).
    w.i32(100);
    w.i32(0);
    w.zeros(11);
    // Page setup: size, margins, proportion, header/footer flags, then the header texts.
    for v in [210, 297, 10, 10, 15, 10, 100] {
        w.i32(v);
    }
    w.i16(0x01FF);
    for s in [
        "%TITLE%",
        "%SUBTITLE%",
        "%ARTIST%",
        "%ALBUM%",
        "Words by %WORDS%",
        "Music by %MUSIC%",
        "Words & Music by %WORDSMUSIC%",
        "Copyright %COPYRIGHT%",
        "All Rights Reserved - International Copyright Secured",
        "Page %N%/%P%",
    ] {
        w.int_byte_str(s);
    }
    w.int_byte_str("Moderate");
    w.i32(song.tempo.clamp(20, 400) as i32);
    w.u8(0); // tempo shown
    w.i8(song.master.first().map(|m| m.key).unwrap_or(0));
    w.i32(0);
    // The 64 MIDI channels.
    let chans = channels(song);
    let mut table = vec![(25i32, 13u8, 8u8); 64];
    for (k, t) in song.tracks.iter().enumerate() {
        let vol = (t.volume.clamp(0.0, 1.0) * 16.0).round() as u8;
        let bal = ((t.pan.clamp(-1.0, 1.0) * 8.0) + 8.0)
            .round()
            .clamp(0.0, 16.0) as u8;
        let prog = if t.instrument == Instrument::Drums {
            0
        } else {
            t.instrument.midi_program() as i32
        };
        let (a, b) = chans[k];
        table[a] = (prog, vol, bal);
        table[b] = (prog, vol, bal);
    }
    for (prog, vol, bal) in table {
        w.i32(prog);
        w.u8(vol);
        w.u8(bal);
        w.zeros(4); // chorus, reverb, phaser, tremolo
        w.zeros(2);
    }
    // Directions: the bar (counted from 1) of each sign, or -1.
    let mut dirs = [-1i16; 19];
    for (i, mb) in song.master.iter().enumerate() {
        for slot in [
            gp5_direction_slot(mb.direction_target, 0),
            gp5_direction_slot(0, mb.direction_jump),
        ]
        .into_iter()
        .flatten()
        {
            if dirs[slot] < 0 {
                dirs[slot] = i as i16 + 1;
            }
        }
    }
    for d in dirs {
        w.i16(d);
    }
    w.i32(0);
    let nbars = song.bar_count();
    w.i32(nbars as i32);
    w.i32(song.tracks.len() as i32);

    // Bar headers.
    for (i, mb) in song.master.iter().enumerate() {
        let prev = if i > 0 {
            Some(&song.master[i - 1])
        } else {
            None
        };
        let mut flags = 0u8;
        if prev.map(|p| p.num != mb.num).unwrap_or(true) {
            flags |= 0x01;
        }
        if prev.map(|p| p.den != mb.den).unwrap_or(true) {
            flags |= 0x02;
        }
        if mb.repeat_open {
            flags |= 0x04;
        }
        if mb.repeat_close > 0 {
            flags |= 0x08;
        }
        if mb.endings != 0 {
            flags |= 0x10;
        }
        if mb.marker.is_some() {
            flags |= 0x20;
        }
        if prev
            .map(|p| p.key != mb.key || p.key_minor != mb.key_minor)
            .unwrap_or(true)
        {
            flags |= 0x40;
        }
        if mb.double_bar {
            flags |= 0x80;
        }
        if i > 0 {
            w.u8(0);
        }
        w.u8(flags);
        if flags & 0x01 != 0 {
            w.i8(mb.num as i8);
        }
        if flags & 0x02 != 0 {
            w.i8(mb.den as i8);
        }
        if flags & 0x08 != 0 {
            w.i8(mb.repeat_close.max(2) as i8);
        }
        if let Some(m) = &mb.marker {
            w.int_byte_str(m);
            w.b.extend_from_slice(&[255, 0, 0, 0]);
        }
        if flags & 0x40 != 0 {
            w.i8(mb.key);
            w.i8(if mb.key_minor { 1 } else { 0 });
        }
        if flags & 0x10 != 0 {
            w.u8(mb.endings);
        }
        if flags & 0x03 != 0 {
            // Eighth notes per beam group.
            let beams = if mb.den == 8 && mb.num % 3 == 0 {
                [3, 3, 3, 3]
            } else {
                [2, 2, 2, 2]
            };
            w.b.extend_from_slice(&beams);
        }
        if flags & 0x10 == 0 {
            w.u8(0);
        }
        w.u8(if mb.triplet_feel { 1 } else { 0 });
    }

    // Tracks.
    for (k, t) in song.tracks.iter().enumerate() {
        if k == 0 {
            w.u8(0);
        }
        let drum = t.instrument == Instrument::Drums;
        let mut f1 = 0u8;
        if drum {
            f1 |= 0x01;
        }
        if t.instrument == Instrument::Banjo {
            f1 |= 0x04;
        }
        if t.visible {
            f1 |= 0x08;
        }
        if t.solo {
            f1 |= 0x10;
        }
        if t.mute {
            f1 |= 0x20;
        }
        w.u8(f1);
        w.byte_str(&t.name, 40);
        let n = t.strings().clamp(1, 7);
        w.i32(n as i32);
        for s in 0..7 {
            w.i32(if s < n {
                t.tuning.get(s).copied().unwrap_or(0) as i32
            } else {
                0
            });
        }
        let (a, b) = chans[k];
        w.i32((a / 16) as i32 + 1);
        w.i32((a % 16) as i32 + 1);
        w.i32((b % 16) as i32 + 1);
        w.i32(t.frets.max(12) as i32);
        w.i32(t.capo as i32);
        w.b.extend_from_slice(&[t.color[0], t.color[1], t.color[2], 0]);
        let mut f2: i16 = 0;
        if t.has_tab() {
            f2 |= 0x0001;
        }
        if t.show_std || !t.has_tab() {
            f2 |= 0x0002;
        }
        if t.auto_let_ring {
            f2 |= 0x0200;
        }
        w.i16(f2);
        w.u8((t.accentuation.clamp(0.0, 1.0) * 5.0).round() as u8);
        w.u8(0); // bank
        // Sound engine settings: humanize, unused values, instrument, equalizer, effect.
        w.u8(0);
        w.zeros(12);
        w.zeros(12);
        for _ in 0..4 {
            w.i32(-1);
        }
        w.zeros(4);
        w.int_byte_str("");
        w.int_byte_str("");
    }
    w.u8(0);

    // Bars: two voices per track.
    for bi in 0..nbars {
        for (ti, t) in song.tracks.iter().enumerate() {
            let bar = &t.bars[bi];
            for v in 0..2 {
                let beats = bar.voice(v);
                let used = v == 0 || bar.voice_used(v);
                if !used || beats.is_empty() {
                    // An empty voice: one blank beat.
                    w.i32(1);
                    w.u8(0x40);
                    w.u8(0x00);
                    w.i8(0);
                    w.u8(0);
                    w.i16(0);
                    continue;
                }
                w.i32(beats.len() as i32);
                for (j, beat) in beats.iter().enumerate() {
                    let tempo = if ti == 0 && v == 0 && j == 0 && bi > 0 {
                        song.master[bi].tempo
                    } else {
                        None
                    };
                    write_beat(&mut w, t, beat, tempo);
                }
            }
            w.u8(if song.master[bi].line_break { 1 } else { 0 });
        }
    }
    w.b
}

fn write_beat(w: &mut W, t: &Track, beat: &Beat, tempo: Option<u32>) {
    let mut flags = 0u8;
    if beat.dotted || beat.double_dot {
        flags |= 0x01;
    }
    if beat.chord.is_some() {
        flags |= 0x02;
    }
    if beat.text.is_some() {
        flags |= 0x04;
    }
    let effects =
        beat.fade_in || beat.technique != 0 || beat.stroke != Stroke::None || beat.pick != 0;
    if effects {
        flags |= 0x08;
    }
    if tempo.is_some() {
        flags |= 0x10;
    }
    if beat.tuplet > 0 {
        flags |= 0x20;
    }
    if beat.is_rest() {
        flags |= 0x40;
    }
    w.u8(flags);
    if beat.is_rest() {
        w.u8(0x02);
    }
    w.i8(beat.dur.index() as i8 - 2);
    if beat.tuplet > 0 {
        w.i32(beat.tuplet as i32);
    }
    if let Some(name) = &beat.chord {
        // A chord name, with its diagram when shown.
        w.u8(0);
        w.int_byte_str(name);
        if beat.chord_diagram && t.has_strings() && t.strings() <= 6 {
            let first = beat
                .notes
                .iter()
                .map(|n| n.fret)
                .filter(|&f| f > 0)
                .min()
                .unwrap_or(1)
                .max(1);
            w.i32(first as i32);
            for s in 0..6u8 {
                w.i32(beat.note_on(s).map(|n| n.fret as i32).unwrap_or(-1));
            }
        } else {
            w.i32(0);
        }
    }
    if let Some(text) = &beat.text {
        w.int_byte_str(text);
    }
    if effects {
        let mut f1 = 0u8;
        let mut f2 = 0u8;
        if beat.fade_in {
            f1 |= 0x10;
        }
        if beat.technique != 0 {
            f1 |= 0x20;
        }
        if beat.stroke != Stroke::None {
            f1 |= 0x40;
        }
        if beat.pick != 0 {
            f2 |= 0x02;
        }
        w.u8(f1);
        w.u8(f2);
        if beat.technique != 0 {
            w.u8(beat.technique);
        }
        if beat.stroke != Stroke::None {
            let (down, up) = if beat.stroke == Stroke::Down {
                (3, 0)
            } else {
                (0, 3)
            };
            w.u8(down);
            w.u8(up);
        }
        if beat.pick != 0 {
            w.u8(if beat.pick == 1 { 2 } else { 1 });
        }
    }
    if let Some(bpm) = tempo {
        // A mix table change carrying only the tempo.
        w.i8(-1);
        for _ in 0..4 {
            w.i32(-1);
        }
        for _ in 0..6 {
            w.i8(-1);
        }
        w.int_byte_str("");
        w.i32(bpm.clamp(20, 400) as i32);
        w.u8(0); // reached at once
        w.u8(0); // shown
        w.u8(0); // applies to this track's settings only
        w.i8(-1); // no wah
        w.int_byte_str("");
        w.int_byte_str("");
    }
    // Notes, at most seven, on the strings they use (instruments without strings
    // can sit on any of the seven).
    let mut placed: Vec<(usize, &Note)> = vec![];
    if t.has_strings() {
        for n in &beat.notes {
            if (n.string as usize) < 7 && !placed.iter().any(|p| p.0 == n.string as usize) {
                placed.push((n.string as usize, n));
            }
        }
    } else {
        for (k, n) in beat
            .notes
            .iter()
            .take(7.min(t.strings().max(1)))
            .enumerate()
        {
            placed.push((k, n));
        }
    }
    placed.sort_by_key(|p| p.0);
    let mut sflags = 0u8;
    for (s, _) in &placed {
        sflags |= 1 << (6 - s);
    }
    w.u8(sflags);
    for (_, n) in &placed {
        write_note(w, beat, n);
    }
    w.i16(0);
}

fn write_note(w: &mut W, beat: &Beat, n: &Note) {
    let fx = n.bend > 0
        || n.hammer
        || n.let_ring
        || n.grace.is_some()
        || n.staccato
        || n.palm_mute
        || beat.tremolo
        || n.slide
        || n.slide_in
        || n.slide_out
        || n.harmonic
        || n.pinch_harmonic
        || n.trill > 0
        || n.vibrato
        || n.wide_vibrato;
    let mut flags = 0x20 | 0x10;
    if n.heavy_accent {
        flags |= 0x02;
    }
    if n.ghost {
        flags |= 0x04;
    }
    if fx {
        flags |= 0x08;
    }
    if n.accent {
        flags |= 0x40;
    }
    w.u8(flags);
    w.u8(if n.tie {
        2
    } else if n.dead {
        3
    } else {
        1
    });
    let dynv = Dynamic::ALL
        .iter()
        .position(|d| *d == beat.dynamic)
        .unwrap_or(4) as i8
        + 1;
    w.i8(dynv);
    w.i8(n.fret.min(127) as i8);
    w.u8(0);
    if !fx {
        return;
    }
    let mut f1 = 0u8;
    let mut f2 = 0u8;
    if n.bend > 0 {
        f1 |= 0x01;
    }
    if n.hammer {
        f1 |= 0x02;
    }
    if n.let_ring {
        f1 |= 0x08;
    }
    if n.grace.is_some() {
        f1 |= 0x10;
    }
    if n.staccato {
        f2 |= 0x01;
    }
    if n.palm_mute {
        f2 |= 0x02;
    }
    if beat.tremolo {
        f2 |= 0x04;
    }
    if n.slide || n.slide_in || n.slide_out {
        f2 |= 0x08;
    }
    if n.harmonic || n.pinch_harmonic {
        f2 |= 0x10;
    }
    if n.trill > 0 {
        f2 |= 0x20;
    }
    if n.vibrato || n.wide_vibrato {
        f2 |= 0x40;
    }
    w.u8(f1);
    w.u8(f2);
    if n.bend > 0 {
        // Bend points over 0..60, values in hundredths of a tone / 4.
        let v = n.bend as i32 * 25;
        let (kind, pts): (u8, Vec<(i32, i32)>) = match n.bend_kind {
            1 => (2, vec![(0, 0), (15, v), (30, v), (45, 0), (60, 0)]),
            2 => (4, vec![(0, v), (60, v)]),
            3 => (5, vec![(0, v), (30, v), (60, 0)]),
            _ => (1, vec![(0, 0), (15, v), (60, v)]),
        };
        w.u8(kind);
        w.i32(v);
        w.i32(pts.len() as i32);
        for (pos, val) in pts {
            w.i32(pos);
            w.i32(val);
            w.u8(0);
        }
    }
    if let Some(g) = n.grace {
        w.u8(g);
        w.u8(5);
        w.u8(0);
        w.u8(1);
        w.u8(0);
    }
    if beat.tremolo {
        w.u8(3);
    }
    if f2 & 0x08 != 0 {
        let mut st = 0i8;
        if n.slide {
            st |= 0x01;
        }
        if n.slide_out {
            st |= 0x04;
        }
        if n.slide_in {
            st |= 0x10;
        }
        w.i8(st);
    }
    if f2 & 0x10 != 0 {
        w.i8(if n.pinch_harmonic { 4 } else { 1 });
    }
    if n.trill > 0 {
        w.u8(n.trill);
        w.u8(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_round_trips_through_gp5() {
        let song = Song::demo();
        let data = export_gp5(&song);
        let back = crate::gpimport::import(&data).expect("reads back");
        assert_eq!(back.title, song.title);
        assert_eq!(back.tempo, song.tempo);
        assert_eq!(back.bar_count(), song.bar_count());
        assert_eq!(back.tracks.len(), song.tracks.len());
        for (a, b) in song.tracks.iter().zip(back.tracks.iter()) {
            assert_eq!(a.name, b.name);
            assert_eq!(a.tuning, b.tuning);
            for (x, y) in a.bars.iter().zip(b.bars.iter()) {
                assert_eq!(x.beats.len(), y.beats.len());
                for (p, q) in x.beats.iter().zip(y.beats.iter()) {
                    assert_eq!(p.dur, q.dur);
                    let mut pa: Vec<u8> = p.notes.iter().map(|n| a.pitch(n)).collect();
                    let mut pb: Vec<u8> = q.notes.iter().map(|n| b.pitch(n)).collect();
                    pa.sort();
                    pb.sort();
                    assert_eq!(pa, pb);
                }
            }
        }
        let mut song2 = song.clone();
        song2.master[2].direction_target = DIR_SEGNO;
        song2.master[5].direction_jump = 7;
        song2.master[9].direction_target = DIR_CODA;
        let back2 = crate::gpimport::import(&export_gp5(&song2)).expect("reads back");
        assert_eq!(back2.master[2].direction_target, DIR_SEGNO);
        assert_eq!(back2.master[5].direction_jump, 7);
        assert_eq!(back2.master[9].direction_target, DIR_CODA);
        for (x, y) in song.master.iter().zip(back.master.iter()) {
            assert_eq!(
                (
                    x.num,
                    x.den,
                    x.key,
                    x.repeat_open,
                    x.repeat_close,
                    &x.marker
                ),
                (
                    y.num,
                    y.den,
                    y.key,
                    y.repeat_open,
                    y.repeat_close,
                    &y.marker
                )
            );
        }
    }

    #[test]
    fn piano_drums_and_lyrics_round_trip() {
        let mut song = Song::new_blank();
        let mut piano = Track::of_instrument(Instrument::Piano, "Piano");
        let mut drums = Track::of_instrument(Instrument::Drums, "Drums");
        let mut voc = Track::of_instrument(Instrument::LeadVocals, "Voice");
        voc.lyrics = "la la la".into();
        let chord = |ps: &[u8]| Beat {
            notes: ps
                .iter()
                .enumerate()
                .map(|(k, &p)| Note::new(k as u8, p))
                .collect(),
            ..Beat::rest(Dur::Whole)
        };
        piano.bars = (0..4)
            .map(|_| Bar::new(vec![chord(&[48, 60, 64, 67])]))
            .collect();
        drums.bars = (0..4).map(|_| Bar::new(vec![chord(&[36, 42])])).collect();
        voc.bars = (0..4).map(|_| Bar::new(vec![chord(&[72])])).collect();
        song.tracks = vec![voc, piano, drums];
        let back = crate::gpimport::import(&export_gp5(&song)).expect("reads back");
        assert_eq!(back.tracks[0].instrument, Instrument::LeadVocals);
        assert_eq!(back.tracks[0].lyrics, "la la la");
        assert_eq!(back.tracks[1].instrument, Instrument::Piano);
        assert!(back.tracks[1].grand());
        assert_eq!(back.tracks[2].instrument, Instrument::Drums);
        let p: Vec<u8> = back.tracks[1].bars[0].beats[0]
            .notes
            .iter()
            .map(|n| back.tracks[1].pitch(n))
            .collect();
        assert_eq!(p, vec![48, 60, 64, 67]);
        let d: Vec<u8> = back.tracks[2].bars[0].beats[0]
            .notes
            .iter()
            .map(|n| back.tracks[2].pitch(n))
            .collect();
        assert_eq!(d, vec![36, 42]);
        assert!(!back.tracks[2].has_tab());
    }
}
