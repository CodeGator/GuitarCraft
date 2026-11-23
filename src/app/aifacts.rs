//! Facts about a few bars of music, worked out by the program from the notes, to go along with
//! the tablature in a question to the assistant. A small model is poor at working out which scale
//! fits or what a handful of notes spell, but good at explaining it once it is told, so the
//! program does the sums (with `theory.rs`) and the model does the talking.
//!
//! The facts are possibilities, not verdicts: several scales usually fit, and the chord a few
//! notes spell depends on the music around them. They are worded that way for the model.

use crate::theory::{self, SCALES};

/// The notes sounding on one beat of the bars being sent.
pub struct BeatNotes {
    /// The bar and beat, counted from 1 as the person sees them.
    pub bar: usize,
    pub beat: usize,
    /// The MIDI pitch of each note (capo and tuning already counted).
    pub pitches: Vec<u8>,
}

/// The most scales listed, and the most chord changes.
const MAX_SCALES: usize = 6;
const MAX_CHORDS: usize = 24;

/// A scale's name with its root: "E Minor Pentatonic".
fn scale_name(root: u8, scale: usize) -> String {
    format!("{} {}", theory::ascii(&theory::note_name(root, theory::flats_for(root))), SCALES[scale].name)
}

/// The facts as a few lines of text, or an empty string when there are no notes.
pub fn passage_facts(beats: &[BeatNotes]) -> String {
    let mut count = [0usize; 12];
    for b in beats {
        for p in &b.pitches {
            count[(*p % 12) as usize] += 1;
        }
    }
    if count.iter().all(|c| *c == 0) {
        return String::new();
    }
    // The most used note leads, and decides whether the notes are named with sharps or flats.
    let mut used: Vec<u8> = (0..12u8).filter(|pc| count[*pc as usize] > 0).collect();
    used.sort_by_key(|pc| (std::cmp::Reverse(count[*pc as usize]), *pc));
    let lead = used[0];
    let flats = theory::flats_for(lead);
    let name = |pc: u8| theory::ascii(&theory::note_name(pc, flats));

    let mut out = String::from(
        "Worked out by GuitarCraft from these notes (possibilities to use, not the only answer):\n",
    );
    let notes: Vec<String> = used.iter().map(|pc| format!("{} ({})", name(*pc), count[*pc as usize])).collect();
    out.push_str(&format!("Notes used, most often first: {}.\n", notes.join(", ")));

    let mut pcs = used.clone();
    pcs.sort();
    let scales = theory::scales_containing(&pcs, lead);
    if scales.is_empty() {
        out.push_str("No single scale in GuitarCraft's list contains all of these notes.\n");
    } else {
        let list: Vec<String> = scales.iter().take(MAX_SCALES).map(|(r, k)| scale_name(*r, *k)).collect();
        out.push_str(&format!("Scales that contain all of them: {}.\n", list.join(", ")));
    }

    // The chord the notes sounding together spell on each beat, listing only the changes.
    let mut chords: Vec<String> = vec![];
    let mut last = String::new();
    for b in beats {
        let mut beat_pcs: Vec<u8> = b.pitches.iter().map(|p| p % 12).collect();
        beat_pcs.sort();
        beat_pcs.dedup();
        let Some(bass) = b.pitches.iter().min() else { continue };
        let Some(f) = theory::identify(&beat_pcs, bass % 12).first().copied() else { continue };
        let chord = theory::chord_name(f.root, f.kind, Some(f.bass));
        if chord != last {
            chords.push(format!("bar {} beat {}: {chord}", b.bar, b.beat));
            last = chord;
        }
    }
    if !chords.is_empty() {
        let more = chords.len() > MAX_CHORDS;
        chords.truncate(MAX_CHORDS);
        out.push_str(&format!(
            "Chords the notes sounding together spell (changes only): {}{}.\n",
            chords.join("; "),
            if more { "; and more" } else { "" }
        ));
    }
    out.trim_end().to_string()
}

/// Facts about every note of a track, so a question about a few bars has the rest of the music
/// for context: a handful of bars can use two notes, which fit dozens of scales, while the whole
/// track says which key it is really in. Each scale comes with the share of the track's notes
/// that are inside it, so a scale with the odd note outside it still shows up.
pub fn track_facts(beats: &[BeatNotes], bars: usize) -> String {
    let mut count = [0usize; 12];
    for b in beats {
        for p in &b.pitches {
            count[(*p % 12) as usize] += 1;
        }
    }
    let total: usize = count.iter().sum();
    if total == 0 {
        return String::new();
    }
    let mut used: Vec<u8> = (0..12u8).filter(|pc| count[*pc as usize] > 0).collect();
    used.sort_by_key(|pc| (std::cmp::Reverse(count[*pc as usize]), *pc));
    let lead = used[0];
    let flats = theory::flats_for(lead);
    let name = |pc: u8| theory::ascii(&theory::note_name(pc, flats));
    let notes: Vec<String> = used.iter().map(|pc| format!("{} ({})", name(*pc), count[*pc as usize])).collect();

    // (share inside, size, not on the most used note, scale, root): best first.
    let mut fits: Vec<(usize, usize, bool, usize, u8)> = vec![];
    for root in 0..12u8 {
        for (k, s) in SCALES.iter().enumerate() {
            if s.iv.len() >= 12 {
                continue;
            }
            let inside: usize = (0..12u8)
                .filter(|pc| s.iv.iter().any(|i| (root + i) % 12 == *pc))
                .map(|pc| count[pc as usize])
                .sum();
            fits.push((inside * 100 / total, s.iv.len(), root != lead, k, root));
        }
    }
    fits.sort_by_key(|f| (std::cmp::Reverse(f.0), f.1, f.2, f.3, f.4));
    let best: Vec<String> = fits.iter().take(MAX_SCALES).map(|f| format!("{} ({}%)", scale_name(f.4, f.3), f.0)).collect();

    format!(
        "The whole track ({bars} bars), counting every note in it (a better guide to the key than a few bars):\n\
         Notes used, most often first: {}.\n\
         Scales that fit best, with the share of the notes that are in the scale: {}.",
        notes.join(", "),
        best.join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn beat(bar: usize, beat: usize, pitches: &[u8]) -> BeatNotes {
        BeatNotes { bar, beat, pitches: pitches.to_vec() }
    }

    #[test]
    fn no_notes_give_no_facts() {
        assert_eq!(passage_facts(&[]), "");
        assert_eq!(passage_facts(&[beat(1, 1, &[])]), "");
    }

    #[test]
    fn notes_are_counted_and_a_scale_that_holds_them_is_offered() {
        // E, G, B, D: an E minor seventh.
        let facts = passage_facts(&[beat(5, 1, &[40, 47, 50, 55]), beat(5, 2, &[40])]);
        assert!(facts.contains("Notes used, most often first: E (2), "), "{facts}");
        assert!(facts.contains("Scales that contain all of them: E "), "{facts}");
    }

    #[test]
    fn a_chord_is_named_and_only_changes_are_listed() {
        let facts = passage_facts(&[
            beat(5, 1, &[40, 47, 50, 55]),
            beat(5, 2, &[40, 47, 50, 55]),
            beat(6, 1, &[45, 52, 55, 60]),
        ]);
        assert!(facts.contains("bar 5 beat 1: Em7"), "{facts}");
        assert!(!facts.contains("bar 5 beat 2"), "{facts}");
        assert!(facts.contains("bar 6 beat 1:"), "{facts}");
    }

    #[test]
    fn a_single_line_melody_names_no_chords() {
        let facts = passage_facts(&[beat(1, 1, &[64]), beat(1, 2, &[67]), beat(1, 3, &[69])]);
        assert!(facts.contains("Notes used"), "{facts}");
        assert!(!facts.contains("Chords"), "{facts}");
    }

    #[test]
    fn notes_that_fit_no_scale_say_so() {
        let all: Vec<BeatNotes> = (0..12u8).map(|i| beat(1, i as usize + 1, &[40 + i])).collect();
        let facts = passage_facts(&all);
        assert!(facts.contains("No single scale"), "{facts}");
    }

    #[test]
    fn flat_keys_use_flat_names() {
        // F, A, C, Bb: F major territory, with F the most used.
        let facts = passage_facts(&[beat(1, 1, &[41, 45, 48]), beat(1, 2, &[41, 46])]);
        assert!(facts.contains("B\u{266D}") || facts.contains("Bb"), "{facts}");
        assert!(!facts.contains("A#"), "{facts}");
    }

    #[test]
    fn the_whole_track_shows_the_key_that_a_few_bars_cannot() {
        // G major all through: G A B C D E F#, G most often, over many bars.
        let g_major = [55u8, 57, 59, 60, 62, 64, 66];
        let mut beats = vec![];
        for bar in 1..=8 {
            for (i, p) in g_major.iter().enumerate() {
                beats.push(beat(bar, i + 1, &[*p]));
            }
            beats.push(beat(bar, 8, &[55]));
        }
        let facts = track_facts(&beats, 8);
        assert!(facts.contains("The whole track (8 bars)"), "{facts}");
        assert!(facts.contains("G Major (100%)"), "{facts}");
        assert!(facts.contains("Notes used, most often first: G ("), "{facts}");
    }

    #[test]
    fn when_no_scale_holds_every_note_the_best_fits_are_still_ranked() {
        // All twelve notes, equally often: no scale holds them all, but the track still gets a list.
        let beats: Vec<BeatNotes> = (0..12u8).map(|i| beat(1, i as usize + 1, &[40 + i])).collect();
        let facts = track_facts(&beats, 1);
        assert!(!facts.contains("(100%)"), "{facts}");
        assert!(facts.contains("Scales that fit best"), "{facts}");
        assert!(facts.contains("%)"), "{facts}");
    }

    #[test]
    fn a_track_without_notes_has_no_summary() {
        assert_eq!(track_facts(&[], 0), "");
        assert_eq!(track_facts(&[beat(1, 1, &[])], 1), "");
    }
}
