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
}
