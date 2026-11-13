//! Marks this app has that it doesn't yet write in Guitar Pro's own way (the
//! palette's newer tools: further note and beat marks, bar repeat signs, free
//! time). They travel with the song inside the score file as a hidden note
//! that Guitar Pro skips, so files stay readable by Guitar Pro 8.

use crate::model::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Debug, PartialEq)]
#[serde(default)]
pub struct Extras {
    /// Track, bar, voice, beat, string and the note's marks.
    pub notes: Vec<(usize, usize, usize, usize, u8, NoteFx)>,
    /// Track, bar, voice, beat and the beat's marks.
    pub beats: Vec<(usize, usize, usize, usize, BeatFx)>,
    /// Track, bar and its repeat sign.
    pub similes: Vec<(usize, usize, u8)>,
    /// Bars in free time.
    pub free_time: Vec<usize>,
}

impl Extras {
    pub fn is_empty(&self) -> bool {
        self.notes.is_empty() && self.beats.is_empty() && self.similes.is_empty() && self.free_time.is_empty()
    }
}

/// Everything of the song that only this app keeps.
pub fn collect(song: &Song) -> Extras {
    let mut x = Extras::default();
    for (ti, t) in song.tracks.iter().enumerate() {
        for (bi, bar) in t.bars.iter().enumerate() {
            if bar.simile != 0 {
                x.similes.push((ti, bi, bar.simile));
            }
            for v in 0..VOICES {
                for (j, b) in bar.voice(v).iter().enumerate() {
                    if b.fx != BeatFx::default() {
                        x.beats.push((ti, bi, v, j, b.fx.clone()));
                    }
                    for n in &b.notes {
                        if n.fx != NoteFx::default() {
                            x.notes.push((ti, bi, v, j, n.string, n.fx.clone()));
                        }
                    }
                }
            }
        }
    }
    x.free_time = song.master.iter().enumerate().filter(|(_, m)| m.free_time).map(|(i, _)| i).collect();
    x
}

/// Puts the marks back on a song read from a file.
pub fn apply(song: &mut Song, x: &Extras) {
    for &(ti, bi, s) in &x.similes {
        if let Some(bar) = song.tracks.get_mut(ti).and_then(|t| t.bars.get_mut(bi)) {
            bar.simile = s.min(2);
        }
    }
    for (ti, bi, v, j, fx) in &x.beats {
        if let Some(b) = beat_mut(song, *ti, *bi, *v, *j) {
            b.fx = fx.clone();
        }
    }
    for (ti, bi, v, j, s, fx) in &x.notes {
        if let Some(n) = beat_mut(song, *ti, *bi, *v, *j).and_then(|b| b.note_on_mut(*s)) {
            n.fx = fx.clone();
        }
    }
    for &bi in &x.free_time {
        if let Some(m) = song.master.get_mut(bi) {
            m.free_time = true;
        }
    }
}

fn beat_mut(song: &mut Song, ti: usize, bi: usize, v: usize, j: usize) -> Option<&mut Beat> {
    let bar = song.tracks.get_mut(ti)?.bars.get_mut(bi)?;
    if v >= VOICES {
        return None;
    }
    bar.pad_voices();
    bar.voice_mut(v).get_mut(j)
}

/// Marks the hidden note in score.gpif; the JSON follows in hexadecimal.
pub const MARK: &str = "GuitarCraftExtras:";

/// The hidden note for the score, or nothing when the song has no such marks.
pub fn comment(song: &Song) -> Option<String> {
    let x = collect(song);
    if x.is_empty() {
        return None;
    }
    let json = serde_json::to_string(&x).ok()?;
    let hex: String = json.as_bytes().iter().map(|b| format!("{b:02x}")).collect();
    Some(format!("<!--{MARK}{hex}-->\n"))
}

/// Reads the hidden note back from a score's XML.
pub fn from_xml(xml: &str) -> Option<Extras> {
    let start = xml.find(MARK)? + MARK.len();
    let end = start + xml[start..].find("-->")?;
    let digits = xml[start..end].trim().as_bytes();
    let bytes: Option<Vec<u8>> = digits
        .chunks(2)
        .map(|c| std::str::from_utf8(c).ok().and_then(|t| u8::from_str_radix(t, 16).ok()))
        .collect();
    serde_json::from_slice(&bytes?).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_marks_survive_saving() {
        let mut song = Song::demo();
        song.tracks[0].bars[0].beats[0].fx.arpeggio = 2;
        song.tracks[0].bars[0].beats[0].fx.timer = true;
        let s = song.tracks[0].bars[0].beats[0].notes[0].string;
        song.tracks[0].bars[0].beats[0].notes[0].fx.legato = true;
        song.tracks[0].bars[0].beats[0].notes[0].fx.lh_finger = 3;
        song.tracks[1].bars[2].simile = 1;
        song.master[3].free_time = true;
        let data = crate::gpifwrite::export_gp(&song).unwrap();
        let back = crate::gpif::import_gp(&data).unwrap();
        assert_eq!(back.tracks[0].bars[0].beats[0].fx.arpeggio, 2);
        assert!(back.tracks[0].bars[0].beats[0].fx.timer);
        let n = back.tracks[0].bars[0].beats[0].note_on(s).unwrap();
        assert!(n.fx.legato);
        assert_eq!(n.fx.lh_finger, 3);
        assert_eq!(back.tracks[1].bars[2].simile, 1);
        assert!(back.master[3].free_time);
        // The score stays well-formed XML.
        assert!(roxmltree::Document::parse(&crate::gpifwrite::gpif_xml(&song)).is_ok());
    }
}
