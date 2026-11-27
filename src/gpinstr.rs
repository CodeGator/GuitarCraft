//! Keyboards, voices and orchestral instruments as Guitar Pro stores them. To this app they have
//! no strings (a note is a pitch); in a Guitar Pro file every staff still has a string tuning and
//! every note sits on a string. Guitar Pro 8 crashed on a file of ours that left this out and
//! used other type names. Read from a file Guitar Pro 8 saved
//! (`tests/fixtures/gp8/Instruments.gp`); it offered no string ensemble, which is a violin with
//! the "String Ensemble 1" sound there.

use crate::model::{Instrument, InstrumentType};

/// What Guitar Pro writes for an instrument.
pub struct GpInstrument {
    /// The instrument set's type and name.
    pub kind: &'static str,
    pub set: &'static str,
    /// The sound: where it is in Guitar Pro's library, its name and its patch.
    pub path: &'static str,
    pub sound: &'static str,
    pub patch: &'static str,
    /// The name of the tuning's instrument, and the open strings, lowest first.
    pub tuned_as: &'static str,
    pub tuning: &'static [u8],
}

const GUITAR: &[u8] = &[40, 45, 50, 55, 59, 64];
const VIOLIN: &[u8] = &[55, 62, 69, 76];
const CELLO: &[u8] = &[36, 43, 50, 57];
/// The lower staff of a grand staff.
pub const LOWER_TUNING: &[u8] = &[23, 28, 33, 38, 43];
/// Frets on every string: the highest a note can sit above its open string.
pub const FRETS: u8 = 24;

/// How Guitar Pro stores a keyboard, voice or orchestral instrument; None for the others.
pub fn pitched(i: Instrument) -> Option<GpInstrument> {
    use Instrument::*;
    let g = |kind, set, path, sound, patch| Some(GpInstrument { kind, set, path, sound, patch, tuned_as: "Guitar", tuning: GUITAR });
    match i {
        Piano => g("acousticPiano", "Acoustic Piano", "Orchestra/Keyboard/Acoustic Piano", "Acoustic Piano", "German-APiano"),
        ElectricPiano => g("electricPiano", "Electric Piano", "Orchestra/Keyboard/Electric Piano", "Electric Piano", "MarkI-EPiano"),
        Organ => g("electricOrgan", "Electric Organ", "Orchestra/Keyboard/Organ", "Organ", "Jazz-Organ"),
        Synth => g("keySynthesizer", "Key Synthesizer", "Orchestra/Synth/Keyboard", "Key", "Harpy-Seq"),
        LeadVocals => g("voice", "Voice", "Orchestra/Other/Singer", "Singer", "Vocal-Pad"),
        Choir => g("choir", "Choir", "Orchestra/Other/Singer", "Choir", "MetaChoir-Pad"),
        Trumpet => g("trumpet", "Trumpet", "Orchestra/Winds/Trumpet", "Trumpet", "Trumpet-Solo"),
        Saxophone => g("saxophone", "Saxophone", "Orchestra/Winds/Saxophone", "Saxophone", "Alto-Sax"),
        Flute => g("flute", "Flute", "Orchestra/Winds/Flute", "Solo", "Flute-Solo"),
        Violin => g("violin", "Violin", "Orchestra/Strings/Violin", "Solo", "Violin-Solo").map(|v| GpInstrument { tuned_as: "Violin", tuning: VIOLIN, ..v }),
        Strings => g("violin", "Violin", "Orchestra/Strings/Violin", "String Ensemble 1", "Violin1-Ensemble").map(|v| GpInstrument { tuned_as: "Violin", tuning: VIOLIN, ..v }),
        Cello => g("cello", "Cello", "Orchestra/Strings/Cello", "Solo", "Cello-Solo").map(|v| GpInstrument { tuned_as: "Cello", tuning: CELLO, ..v }),
        _ => None,
    }
}

/// The instrument of a file's track, when it is one without strings: from the instrument set's
/// type (in lower case), the MIDI program and the sound's path. The older names are those earlier
/// versions of this app wrote.
pub fn from_file(kind: &str, program: u8, path: &str) -> Option<Instrument> {
    use Instrument::*;
    Some(match kind {
        "acousticpiano" => Piano,
        "electricpiano" => ElectricPiano,
        "electricorgan" | "organ" => Organ,
        "keysynthesizer" | "leadsynthesizer" => Synth,
        "voice" | "leadvoice" if program != 52 => LeadVocals,
        "voice" | "choir" | "voicechoir" => Choir,
        "violin" if (44..=51).contains(&program) => Strings,
        "violin" => Violin,
        "cello" => Cello,
        "stringensemble" => Strings,
        "trumpet" => Trumpet,
        "saxophone" | "altosaxophone" => Saxophone,
        "flute" => Flute,
        // Another instrument of Guitar Pro's orchestra: the nearest this app has.
        _ if path.starts_with("Orchestra/") => Some(Instrument::from_midi_program(program)).filter(|i| i.kind() != InstrumentType::Stringed)?,
        _ => return None,
    })
}

/// A string and fret for each of the pitches sounding together, no two on one string, or None if
/// they don't fit. Strings are counted from the lowest; the highest note takes the highest string
/// it fits on.
pub fn place(tuning: &[u8], pitches: &[u8]) -> Option<Vec<(usize, u8)>> {
    let mut order: Vec<usize> = (0..pitches.len()).collect();
    order.sort_by_key(|&i| std::cmp::Reverse(pitches[i]));
    let mut used = vec![false; tuning.len()];
    let mut out = vec![(0, 0); pitches.len()];
    for i in order {
        let p = pitches[i];
        let s = (0..tuning.len()).rev().find(|&s| !used[s] && tuning[s] <= p && p - tuning[s] <= FRETS)?;
        used[s] = true;
        out[i] = (s, p - tuning[s]);
    }
    Some(out)
}

/// The instrument's usual tuning, with strings added where the staff's notes need them: lower or
/// higher ones for notes out of reach, and more for chords with more notes than strings in reach.
pub fn tuning_for(usual: &[u8], chords: &[Vec<u8>]) -> Vec<u8> {
    let mut tuning = usual.to_vec();
    for chord in chords {
        // Each added string takes at least one more note, so this ends.
        for _ in 0..=chord.len() * 2 + 12 {
            if place(&tuning, chord).is_some() {
                break;
            }
            let (Some(&low), Some(&high)) = (chord.iter().min(), chord.iter().max()) else { break };
            let bottom = tuning.first().copied().unwrap_or(0);
            let top = tuning.last().copied().unwrap_or(0);
            let add = if low < bottom {
                low.max(bottom.saturating_sub(12))
            } else if high > top.saturating_add(FRETS) {
                top.saturating_add(12)
            } else {
                low
            };
            let at = tuning.partition_point(|&s| s <= add);
            tuning.insert(at, add);
        }
    }
    tuning
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_chord_goes_on_its_own_strings_highest_note_highest() {
        // As in the file Guitar Pro 8 saved: C6 B5 A5 G5 on the top four strings.
        let placed = place(GUITAR, &[72, 71, 69, 67]).unwrap();
        assert_eq!(placed, vec![(5, 8), (4, 12), (3, 14), (2, 17)]);
        assert_eq!(place(GUITAR, &[30]), None);
    }

    #[test]
    fn the_tuning_gains_strings_for_notes_out_of_reach() {
        assert_eq!(tuning_for(GUITAR, &[vec![60, 64, 67]]), GUITAR);
        for chord in [vec![21], vec![108], vec![41, 42, 43, 44], vec![36, 38, 40, 41, 43, 45, 47, 48, 50, 52]] {
            let tuning = tuning_for(GUITAR, std::slice::from_ref(&chord));
            let placed = place(&tuning, &chord).unwrap_or_else(|| panic!("{chord:?} on {tuning:?}"));
            for (&p, &(s, f)) in chord.iter().zip(&placed) {
                assert_eq!(tuning[s] + f, p);
            }
            assert!(tuning.windows(2).all(|w| w[0] <= w[1]), "{tuning:?}");
        }
    }

    #[test]
    fn every_instrument_without_strings_is_recognised_by_what_we_write() {
        for i in Instrument::ALL {
            let Some(g) = pitched(i) else {
                assert!(i.kind() == InstrumentType::Stringed || i == Instrument::Drums, "{i:?}");
                continue;
            };
            assert_eq!(from_file(&g.kind.to_lowercase(), i.midi_program(), g.path), Some(i), "{i:?}");
        }
        assert_eq!(from_file("electricguitar", 29, "Stringed/Electric Guitars/Overdrive Guitar"), None);
    }
}
