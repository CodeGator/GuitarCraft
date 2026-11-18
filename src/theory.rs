//! Music theory for the Chords & Scales tool: the lists of scales and chord
//! types, how notes and intervals are spelled, the chords built on a scale,
//! the scales that fit a chord, and naming a chord from the notes played.
//! Chord types are written as the name after the root ("m7", "sus4", "add9")
//! and read by `chordshape::formula`, so the tool and the chord charts agree.

use crate::chordshape::{self, Formula};
use std::sync::OnceLock;

/// The twelve notes, as sharp and flat names (the same where they don't differ).
pub const ROOTS: [(&str, &str); 12] = [
    ("C", "C"),
    ("C\u{266F}", "D\u{266D}"),
    ("D", "D"),
    ("D\u{266F}", "E\u{266D}"),
    ("E", "E"),
    ("F", "F"),
    ("F\u{266F}", "G\u{266D}"),
    ("G", "G"),
    ("G\u{266F}", "A\u{266D}"),
    ("A", "A"),
    ("A\u{266F}", "B\u{266D}"),
    ("B", "B"),
];

/// A list row for a root: "D", or "D♯ · E♭".
pub fn root_label(pc: u8) -> String {
    let (s, f) = ROOTS[pc as usize % 12];
    if s == f { s.to_string() } else { format!("{s} \u{00B7} {f}") }
}

/// Whether music in this key is written with flats (F, B♭, E♭, A♭, D♭).
pub fn flats_for(root: u8) -> bool {
    matches!(root % 12, 1 | 3 | 5 | 8 | 10)
}

/// A note's name, with ♯ or ♭.
pub fn note_name(pc: u8, flats: bool) -> String {
    let (s, f) = ROOTS[pc as usize % 12];
    if flats { f.to_string() } else { s.to_string() }
}

/// The same with plain # and b, as chord names are written in charts and scores.
pub fn ascii(name: &str) -> String {
    name.replace('\u{266F}', "#").replace('\u{266D}', "b")
}

// ----- scales

pub struct Scale {
    pub group: &'static str,
    pub name: &'static str,
    pub iv: &'static [u8],
}

const fn sc(group: &'static str, name: &'static str, iv: &'static [u8]) -> Scale {
    Scale { group, name, iv }
}

pub const SCALES: [Scale; 47] = [
    sc("Common", "Major", &[0, 2, 4, 5, 7, 9, 11]),
    sc("Common", "Natural Minor", &[0, 2, 3, 5, 7, 8, 10]),
    sc("Common", "Harmonic Minor", &[0, 2, 3, 5, 7, 8, 11]),
    sc("Common", "Melodic Minor (Ascending)", &[0, 2, 3, 5, 7, 9, 11]),
    sc("Common", "Major Pentatonic", &[0, 2, 4, 7, 9]),
    sc("Common", "Minor Pentatonic", &[0, 3, 5, 7, 10]),
    sc("Common", "Major Blues", &[0, 2, 3, 4, 7, 9]),
    sc("Common", "Minor Blues", &[0, 3, 5, 6, 7, 10]),
    sc("Church Modes", "Ionian", &[0, 2, 4, 5, 7, 9, 11]),
    sc("Church Modes", "Dorian", &[0, 2, 3, 5, 7, 9, 10]),
    sc("Church Modes", "Phrygian", &[0, 1, 3, 5, 7, 8, 10]),
    sc("Church Modes", "Lydian", &[0, 2, 4, 6, 7, 9, 11]),
    sc("Church Modes", "Mixolydian", &[0, 2, 4, 5, 7, 9, 10]),
    sc("Church Modes", "Aeolian", &[0, 2, 3, 5, 7, 8, 10]),
    sc("Church Modes", "Locrian", &[0, 1, 3, 5, 6, 8, 10]),
    sc("Church Modes", "Dorian \u{266F}4", &[0, 2, 3, 6, 7, 9, 10]),
    sc("Church Modes", "Dorian \u{266D}2", &[0, 1, 3, 5, 7, 9, 10]),
    sc("Church Modes", "Lydian Augmented", &[0, 2, 4, 6, 8, 9, 11]),
    sc("Church Modes", "Lydian Dominant", &[0, 2, 4, 6, 7, 9, 10]),
    sc("Church Modes", "Mixolydian \u{266D}6", &[0, 2, 4, 5, 7, 8, 10]),
    sc("Church Modes", "Locrian \u{266E}2", &[0, 2, 3, 5, 6, 8, 10]),
    sc("Church Modes", "Super Locrian (Altered)", &[0, 1, 3, 4, 6, 8, 10]),
    sc("Pentatonic", "Egyptian (Suspended)", &[0, 2, 5, 7, 10]),
    sc("Pentatonic", "Hirajoshi", &[0, 2, 3, 7, 8]),
    sc("Pentatonic", "In-Sen", &[0, 1, 5, 7, 10]),
    sc("Pentatonic", "Iwato", &[0, 1, 5, 6, 10]),
    sc("Pentatonic", "Kumoi", &[0, 2, 3, 7, 9]),
    sc("Symmetrical", "Whole Tone", &[0, 2, 4, 6, 8, 10]),
    sc("Symmetrical", "Diminished (Whole-Half)", &[0, 2, 3, 5, 6, 8, 9, 11]),
    sc("Symmetrical", "Diminished (Half-Whole)", &[0, 1, 3, 4, 6, 7, 9, 10]),
    sc("Symmetrical", "Augmented", &[0, 3, 4, 7, 8, 11]),
    sc("Symmetrical", "Chromatic", &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]),
    sc("Modal & Classical", "Harmonic Major", &[0, 2, 4, 5, 7, 8, 11]),
    sc("Modal & Classical", "Phrygian Dominant", &[0, 1, 4, 5, 7, 8, 10]),
    sc("Modal & Classical", "Neapolitan Minor", &[0, 1, 3, 5, 7, 8, 11]),
    sc("Modal & Classical", "Neapolitan Major", &[0, 1, 3, 5, 7, 9, 11]),
    sc("Modal & Classical", "Bebop Dominant", &[0, 2, 4, 5, 7, 9, 10, 11]),
    sc("Modal & Classical", "Bebop Major", &[0, 2, 4, 5, 7, 8, 9, 11]),
    sc("World", "Hungarian Minor", &[0, 2, 3, 6, 7, 8, 11]),
    sc("World", "Double Harmonic (Byzantine)", &[0, 1, 4, 5, 7, 8, 11]),
    sc("World", "Persian", &[0, 1, 4, 5, 6, 8, 11]),
    sc("World", "Gypsy", &[0, 2, 3, 6, 7, 8, 10]),
    sc("World", "Enigmatic", &[0, 1, 4, 6, 8, 10, 11]),
    sc("World", "Prometheus", &[0, 2, 4, 6, 9, 10]),
    sc("World", "Altered Dominant Pentatonic", &[0, 4, 6, 8, 10]),
    sc("World", "Blues (Hexatonic)", &[0, 3, 5, 6, 7, 10]),
    sc("World", "Spanish Gypsy", &[0, 1, 4, 5, 7, 8, 10]),
];

/// The letter names, for spelling seven-note scales one letter per degree.
const LETTERS: [(char, u8); 7] = [('C', 0), ('D', 2), ('E', 4), ('F', 5), ('G', 7), ('A', 9), ('B', 11)];

/// The note's number in a major scale (1 to 7) that a semitone above the root
/// is spelled from: a minor third is a flattened 3rd, a fifth is a 5th ...
const SEMITONE_DEGREE: [usize; 12] = [1, 2, 2, 3, 3, 4, 5, 5, 6, 6, 7, 7];

/// The notes of a pattern (a scale or an arpeggio) from a root, spelled with one
/// letter for each degree (D major: D E F♯ G A B C♯; C minor pentatonic:
/// C E♭ F G B♭). Where that would need a double sharp or flat, the sharps or
/// flats that suit the key are used instead.
pub fn spell_pattern(root: u8, iv: &[u8]) -> Vec<String> {
    let flats = flats_for(root);
    let plain = || iv.iter().map(|i| note_name((root + i) % 12, flats)).collect::<Vec<_>>();
    let by_degree = iv.len() == 7 && iv.windows(2).all(|w| w[0] < w[1]);
    let root_name = note_name(root, flats);
    let Some(li) = LETTERS.iter().position(|l| l.0 == root_name.chars().next().unwrap_or('C')) else { return plain() };
    let mut out = vec![];
    for (d, i) in iv.iter().enumerate() {
        // A seven-note scale has each letter once; anything else follows the interval.
        let degree = if by_degree { d + 1 } else { SEMITONE_DEGREE[*i as usize % 12] };
        let (letter, natural) = LETTERS[(li + degree - 1) % 7];
        let pc = (root + i) % 12;
        let acc = (pc + 12 - natural) % 12;
        out.push(match acc {
            0 => letter.to_string(),
            1 => format!("{letter}\u{266F}"),
            11 => format!("{letter}\u{266D}"),
            _ => return plain(),
        });
    }
    out
}

/// What each semitone above the root is called in a scale.
const SEMITONE_LABELS: [&str; 12] = ["R", "\u{266D}2", "2", "\u{266D}3", "3", "4", "\u{266D}5", "5", "\u{266F}5", "6", "\u{266D}7", "7"];

/// The interval names of a scale: R 2 3 4 ... (♭3, ♭7 and so on where they differ
/// from the major scale).
pub fn scale_intervals(iv: &[u8]) -> Vec<String> {
    if iv.len() == 7 {
        const MAJOR: [i32; 7] = [0, 2, 4, 5, 7, 9, 11];
        return iv
            .iter()
            .enumerate()
            .map(|(d, &s)| {
                let diff = s as i32 - MAJOR[d];
                let acc = match diff {
                    0 => "",
                    -1 => "\u{266D}",
                    1 => "\u{266F}",
                    _ => return SEMITONE_LABELS[s as usize % 12].to_string(),
                };
                if d == 0 { "R".to_string() } else { format!("{acc}{}", d + 1) }
            })
            .collect();
    }
    iv.iter().map(|&s| SEMITONE_LABELS[s as usize % 12].to_string()).collect()
}

// ----- chord types

pub struct ChordKind {
    pub group: &'static str,
    /// The name after the root, as written in a chart: "m7", "sus4", "add9".
    pub suffix: &'static str,
    /// What the list shows.
    pub label: &'static str,
}

const fn ck(group: &'static str, suffix: &'static str, label: &'static str) -> ChordKind {
    ChordKind { group, suffix, label }
}

pub const CHORDS: [ChordKind; 48] = [
    ck("Common", "", "major"),
    ck("Common", "m", "minor"),
    ck("Common", "dim", "dim"),
    ck("Common", "aug", "aug"),
    ck("Power", "5", "5 (power chord)"),
    ck("Sixths", "6", "6"),
    ck("Sixths", "m6", "m6"),
    ck("Sixths", "6/9", "6/9"),
    ck("7th Chords", "7", "7"),
    ck("7th Chords", "maj7", "maj7"),
    ck("7th Chords", "m7", "m7"),
    ck("7th Chords", "m(maj7)", "m(maj7)"),
    ck("7th Chords", "dim7", "dim7"),
    ck("7th Chords", "m7b5", "m7\u{266D}5"),
    ck("7th Chords", "7b5", "7\u{266D}5"),
    ck("7th Chords", "7#5", "7\u{266F}5 (aug7)"),
    ck("7th Chords", "maj7#5", "maj7\u{266F}5"),
    ck("Suspended", "sus2", "sus2"),
    ck("Suspended", "sus4", "sus4"),
    ck("Suspended", "sus2sus4", "sus2sus4"),
    ck("Suspended", "7sus4", "7sus4"),
    ck("Suspended", "9sus4", "9sus4"),
    ck("Added Tone", "add9", "add9"),
    ck("Added Tone", "madd9", "madd9"),
    ck("Added Tone", "add11", "add11"),
    ck("Added Tone", "5add9", "5add9"),
    ck("Extended (9, 11, 13)", "9", "9"),
    ck("Extended (9, 11, 13)", "maj9", "maj9"),
    ck("Extended (9, 11, 13)", "m9", "m9"),
    ck("Extended (9, 11, 13)", "11", "11"),
    ck("Extended (9, 11, 13)", "m11", "m11"),
    ck("Extended (9, 11, 13)", "maj11", "maj11"),
    ck("Extended (9, 11, 13)", "13", "13"),
    ck("Extended (9, 11, 13)", "maj13", "maj13"),
    ck("Extended (9, 11, 13)", "m13", "m13"),
    ck("Altered", "7b9", "7\u{266D}9"),
    ck("Altered", "7#9", "7\u{266F}9"),
    ck("Altered", "7#11", "7\u{266F}11"),
    ck("Altered", "7b13", "7\u{266D}13"),
    ck("Altered", "9b5", "9\u{266D}5"),
    ck("Altered", "9#5", "9\u{266F}5"),
    ck("Altered", "7b5b9", "7\u{266D}5\u{266D}9"),
    ck("Altered", "7#5#9", "7\u{266F}5\u{266F}9"),
    ck("Altered", "alt", "alt"),
    ck("Altered", "m7b9", "m7\u{266D}9"),
    ck("Altered", "m7#5", "m7\u{266F}5"),
    ck("Altered", "dim9", "dim(9)"),
    ck("Altered", "maj7#11", "maj7\u{266F}11"),
];

fn formulas() -> &'static Vec<Formula> {
    static F: OnceLock<Vec<Formula>> = OnceLock::new();
    F.get_or_init(|| {
        CHORDS
            .iter()
            .map(|k| chordshape::formula(k.suffix).unwrap_or(Formula { tones: vec![(0, true)], power: false }))
            .collect()
    })
}

/// A chord type's notes above the root (in semitones), with their names as a
/// musician says them in that chord: R 3 5 ♭7 9 (9 rather than 2 when there's
/// a third, ♭5 or ♯11, 13 rather than 6 over a seventh ...), lowest first.
pub fn chord_notes(kind: usize) -> Vec<(u8, String)> {
    let iv: Vec<u8> = formulas()[kind.min(CHORDS.len() - 1)].tones.iter().map(|t| t.0 as u8).collect();
    let has = |x: u8| iv.contains(&x);
    let third = has(3) || has(4);
    let seventh = has(10) || has(11);
    let mut out: Vec<(u8, String, u8)> = iv
        .iter()
        .map(|&s| {
            // (name, where it sits above the root, for putting them in order)
            let (name, place) = match s {
                0 => ("R", 0),
                1 => if third { ("\u{266D}9", 13) } else { ("\u{266D}2", 1) },
                2 => if third { ("9", 14) } else { ("2", 2) },
                3 => if has(4) { ("\u{266F}9", 15) } else { ("\u{266D}3", 3) },
                4 => ("3", 4),
                5 => if third { ("11", 17) } else { ("4", 5) },
                6 => if has(7) { ("\u{266F}11", 18) } else { ("\u{266D}5", 6) },
                7 => ("5", 7),
                8 => if has(7) { ("\u{266D}13", 20) } else { ("\u{266F}5", 8) },
                9 => if has(6) && has(3) && !has(10) { ("\u{266D}\u{266D}7", 9) } else if seventh { ("13", 21) } else { ("6", 9) },
                10 => ("\u{266D}7", 10),
                _ => ("7", 11),
            };
            (s, name.to_string(), place)
        })
        .collect();
    out.sort_by_key(|o| o.2);
    out.into_iter().map(|o| (o.0, o.1)).collect()
}

/// A chord type's notes above the root, in the order of `chord_notes`
/// (an arpeggio's intervals).
pub fn chord_pattern(kind: usize) -> Vec<u8> {
    chord_notes(kind).into_iter().map(|n| n.0).collect()
}

/// Whether a chord name is written with flats ("Eb", "Bb", "Db") or sharps
/// ("C#m", "F#", "G#m") for this root and chord type.
pub fn chord_flats(root: u8, kind: usize) -> bool {
    let suffix = CHORDS[kind].suffix;
    let minorish = (suffix.starts_with('m') && !suffix.starts_with("maj")) || suffix.starts_with("dim");
    match root % 12 {
        3 | 10 | 5 => true,
        1 => matches!(suffix, "" | "5" | "6" | "6/9" | "maj7" | "maj9" | "add9"),
        8 => !minorish,
        _ => false,
    }
}

/// A chord's name as written in a chart: "Bsus2", "F#m7", "D/F#".
pub fn chord_name(root: u8, kind: usize, bass: Option<u8>) -> String {
    let flats = chord_flats(root, kind);
    let mut s = format!("{}{}", ascii(&note_name(root % 12, flats)), CHORDS[kind].suffix);
    if let Some(b) = bass
        && b % 12 != root % 12
    {
        s.push('/');
        s.push_str(&ascii(&note_name(b % 12, flats)));
    }
    s
}

/// The same, as the list shows it: "B sus2", "F♯ m7♭5".
pub fn chord_display(root: u8, kind: usize, bass: Option<u8>) -> String {
    let flats = chord_flats(root, kind);
    let mut s = format!("{} {}", note_name(root % 12, flats), if CHORDS[kind].suffix.is_empty() { "major" } else { CHORDS[kind].label });
    if let Some(b) = bass
        && b % 12 != root % 12
    {
        s.push_str(&format!(" / {}", note_name(b % 12, flats)));
    }
    s
}

// ----- finding and matching

/// A chord found from notes: its root, type and bass note.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Found {
    pub root: u8,
    pub kind: usize,
    pub bass: u8,
}

/// The chords that can be named from these notes (pitch classes 0 to 11), best
/// first: with the lowest note as the root, the fewest left-out notes, the
/// simplest names. `bass` is the lowest note, which a slash names when it isn't the root.
pub fn identify(pcs: &[u8], bass: u8) -> Vec<Found> {
    let mut set: Vec<u8> = pcs.iter().map(|p| p % 12).collect();
    set.sort();
    set.dedup();
    if set.len() < 2 {
        return vec![];
    }
    let mut out: Vec<(f32, Found)> = vec![];
    for root in 0..12u8 {
        for (kind, f) in formulas().iter().enumerate() {
            let rel: Vec<i32> = set.iter().map(|p| (*p as i32 - root as i32).rem_euclid(12)).collect();
            if f.tones.iter().any(|(pc, need)| *need && !rel.contains(pc)) {
                continue;
            }
            if rel.iter().any(|r| !f.tones.iter().any(|(pc, _)| pc == r)) {
                continue;
            }
            let missing = f.tones.iter().filter(|(pc, _)| !rel.contains(pc)).count() as f32;
            let score = if root == bass % 12 { 0.0 } else { 20.0 } + missing * 4.0 + f.tones.len() as f32 + kind as f32 * 0.01;
            out.push((score, Found { root, kind, bass: bass % 12 }));
        }
    }
    out.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    out.into_iter().map(|o| o.1).take(14).collect()
}

/// Scales (root, scale) that contain all of the given notes, those starting on
/// `prefer` first, then the smaller scales.
pub fn scales_containing(pcs: &[u8], prefer: u8) -> Vec<(u8, usize)> {
    if pcs.is_empty() {
        return vec![];
    }
    let mut out: Vec<(u8, usize)> = vec![];
    for root in 0..12u8 {
        for (k, s) in SCALES.iter().enumerate() {
            // The chromatic scale holds everything; it says nothing.
            if s.iv.len() >= 12 {
                continue;
            }
            if pcs.iter().all(|p| s.iv.iter().any(|i| (root + i) % 12 == p % 12)) {
                out.push((root, k));
            }
        }
    }
    out.sort_by_key(|&(root, k)| (root != prefer % 12, SCALES[k].iv.len(), k, root));
    out
}

/// Scales that fit a chord (all its needed notes), those on the chord's root first.
pub fn scales_for_chord(root: u8, kind: usize) -> Vec<(u8, usize)> {
    let f = &formulas()[kind];
    let need: Vec<u8> = f.tones.iter().filter(|t| t.1).map(|t| ((root as i32 + t.0).rem_euclid(12)) as u8).collect();
    scales_containing(&need, root)
}

/// Chords built on the notes of a scale: (root, chord type, the note's number
/// in the scale), in scale order, each note with its simplest chords first.
pub fn chords_for_scale(root: u8, scale: usize) -> Vec<(u8, usize, usize)> {
    let iv = SCALES[scale].iv;
    if iv.len() >= 12 {
        return vec![];
    }
    let in_scale = |pc: i32| iv.iter().any(|i| (root as i32 + *i as i32).rem_euclid(12) == pc.rem_euclid(12));
    let mut out = vec![];
    for (d, i) in iv.iter().enumerate() {
        let r = (root + i) % 12;
        for (k, f) in formulas().iter().enumerate() {
            // Chords with an altered or added colour tone are left for the Chords tab.
            if !matches!(CHORDS[k].group, "Common" | "Sixths" | "7th Chords" | "Suspended" | "Added Tone" | "Extended (9, 11, 13)") {
                continue;
            }
            if f.tones.iter().filter(|t| t.1).all(|t| in_scale(r as i32 + t.0)) {
                out.push((r, k, d + 1));
            }
        }
    }
    out
}

/// The part of the neck a scale or arpeggio is shown in: start fret (0 = open).
pub const POSITIONS: [u8; 12] = [0, 1, 3, 5, 7, 9, 10, 12, 14, 15, 17, 19];
/// How many frets a position spans.
pub const POSITION_SPAN: u8 = 5;

/// A position's name: 0 for the nut, then I, III, V ...
pub fn position_name(start: u8) -> String {
    if start == 0 { "0".into() } else { crate::score::roman(start).to_string() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scales_are_spelled_one_letter_per_degree() {
        let names = |root: u8, k: usize| spell_pattern(root, SCALES[k].iv).join(" ");
        assert_eq!(names(2, 0), "D E F\u{266F} G A B C\u{266F}");
        assert_eq!(names(10, 0), "B\u{266D} C D E\u{266D} F G A");
        assert_eq!(names(9, 1), "A B C D E F G");
        assert_eq!(names(2, 2), "D E F G A B\u{266D} C\u{266F}");
        // Not seven notes: the sharps or flats that suit the key.
        assert_eq!(names(0, 5), "C E\u{266D} F G B\u{266D}");
        assert_eq!(names(2, 4), "D E F\u{266F} A B");
    }

    #[test]
    fn interval_names() {
        assert_eq!(scale_intervals(SCALES[0].iv).join(" "), "R 2 3 4 5 6 7");
        assert_eq!(scale_intervals(SCALES[9].iv).join(" "), "R 2 \u{266D}3 4 5 6 \u{266D}7");
        assert_eq!(scale_intervals(SCALES[5].iv).join(" "), "R \u{266D}3 4 5 \u{266D}7");
        let c = |suffix: &str| {
            let k = CHORDS.iter().position(|k| k.suffix == suffix).unwrap();
            chord_notes(k).into_iter().map(|n| n.1).collect::<Vec<_>>().join(" ")
        };
        assert_eq!(c(""), "R 3 5");
        assert_eq!(c("m7"), "R \u{266D}3 5 \u{266D}7");
        assert_eq!(c("sus2"), "R 2 5");
        assert_eq!(c("add9"), "R 3 5 9");
        assert_eq!(c("9"), "R 3 5 \u{266D}7 9");
        assert_eq!(c("7#9"), "R 3 5 \u{266D}7 \u{266F}9");
        assert_eq!(c("13"), "R 3 5 \u{266D}7 9 13");
    }

    #[test]
    fn chords_are_named_from_notes() {
        let kind = |s: &str| CHORDS.iter().position(|k| k.suffix == s).unwrap();
        // The person's example: B, C#, F# with B lowest is B sus2, also B5add9, F#sus4/B ...
        let found = identify(&[11, 1, 6], 11);
        assert_eq!(found[0], Found { root: 11, kind: kind("sus2"), bass: 11 });
        let names: Vec<String> = found.iter().map(|f| chord_name(f.root, f.kind, Some(f.bass))).collect();
        assert!(names.contains(&"B5add9".to_string()), "{names:?}");
        assert!(names.contains(&"F#sus4/B".to_string()), "{names:?}");
        assert!(names.contains(&"C#7sus4/B".to_string()), "{names:?}");
        // C E G: C major first; with E lowest it's C/E.
        assert_eq!(chord_name(identify(&[0, 4, 7], 0)[0].root, identify(&[0, 4, 7], 0)[0].kind, Some(0)), "C");
        let first = identify(&[0, 4, 7], 4)[0];
        assert_eq!(chord_name(first.root, first.kind, Some(first.bass)), "C/E");
        assert!(identify(&[3], 3).is_empty());
    }

    #[test]
    fn scales_and_chords_match_each_other() {
        // The notes of C major fit the C major scale, which also holds Am7 and G7.
        let scales = scales_containing(&[0, 2, 4, 5, 7, 9, 11], 0);
        assert_eq!(scales[0], (0, 0));
        assert!(!scales.iter().any(|s| SCALES[s.1].iv.len() >= 12));
        let g7 = CHORDS.iter().position(|k| k.suffix == "7").unwrap();
        let fits = scales_for_chord(7, g7);
        assert_eq!(fits[0].0, 7, "scales on the chord's root first");
        assert!(fits.contains(&(0, 0)), "G7 fits C major");
        assert!(fits.iter().any(|&(r, k)| r == 7 && SCALES[k].name == "Mixolydian"));
        let chords = chords_for_scale(0, 0);
        let m7 = CHORDS.iter().position(|k| k.suffix == "m7").unwrap();
        assert!(chords.iter().any(|&(r, k, d)| r == 9 && k == m7 && d == 6), "A m7 on the 6th note");
        assert!(!chords.iter().any(|&(r, k, _)| r == 0 && CHORDS[k].suffix == "m"), "C minor isn't in C major");
    }

    #[test]
    fn every_chord_type_has_notes_and_a_diagram() {
        for (k, kind) in CHORDS.iter().enumerate() {
            assert!(chord_pattern(k).len() >= 2, "{}", kind.suffix);
            assert!(chord_pattern(k).contains(&0), "{}", kind.suffix);
            let v = chordshape::voicings(&[40, 45, 50, 55, 59, 64], 0, kind.suffix, None, 17);
            assert!(!v.is_empty(), "no fingering for C{}", kind.suffix);
        }
    }
}
