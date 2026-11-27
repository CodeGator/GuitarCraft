//! The drum kit: every percussion sound of a drum track, with its name, where it is written on
//! the percussion staff, its notehead and the General MIDI sound it plays.
//!
//! A drum note keeps the sound's number as its pitch. The numbers are Guitar Pro's: General
//! MIDI's, plus its own for the other ways of playing an instrument (91 is a snare rim shot, 92
//! a half-open hi-hat). The names are those of a picture of Guitar Pro 8's drumkit view. Staff
//! positions, noteheads and output sounds are those of the kit definition in a file Guitar Pro
//! saved (see `gpkit.rs`).

/// The shape of a drum note's head.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DrumHead {
    Normal,
    Cross,
    Diamond,
    CircleSlash,
    CircleCross,
    HeavyCross,
    China,
    Triangle,
    Slashed,
}

/// A sign drawn with a drum note's head.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DrumSign {
    /// A cymbal struck on its edge: a small mark above.
    Edge,
    /// A cymbal choked: a dot above.
    Choke,
    /// A muted drum: the head in brackets.
    Muted,
}

/// One percussion sound.
#[derive(Clone, Copy, Debug)]
pub struct DrumSound {
    pub midi: u8,
    pub name: &'static str,
    /// Where Guitar Pro writes it, in half-spaces above the bottom line (-1 is just below it,
    /// 8 the top line). Many hand percussion sounds are far off the staff.
    pub pos: i8,
    pub head: DrumHead,
    /// The MIDI sound Guitar Pro plays it as.
    pub gm: u8,
}

const fn s(midi: u8, name: &'static str, pos: i8, head: DrumHead, gm: u8) -> DrumSound {
    DrumSound { midi, name, pos, head, gm }
}

use DrumHead::{China, CircleCross, CircleSlash, Cross, Diamond, HeavyCross, Normal, Slashed, Triangle};

/// Every sound, by number.
pub const SOUNDS: [DrumSound; 95] = [
    s(29, "Ride (choke)", 6, Cross, 59),
    s(30, "Reverse Cymbal (hit)", 11, Cross, 49),
    s(31, "Snare (side stick)", 5, Slashed, 40),
    s(33, "Metronome (hit)", 5, Cross, 37),
    s(34, "Metronome (bell)", 5, Normal, 38),
    s(35, "Kick (hit)", 0, Normal, 35),
    s(36, "Kick (hit)", 1, Normal, 36),
    s(37, "Snare (side stick)", 5, Cross, 37),
    s(38, "Snare (hit)", 5, Normal, 38),
    s(39, "Hand Clap (hit)", 5, Normal, 39),
    s(40, "Electric Snare (hit)", 5, Normal, 40),
    s(41, "Low Floor Tom (hit)", 3, Normal, 41),
    s(42, "Hi-Hat (closed)", 9, Cross, 42),
    s(43, "Very Low Tom (hit)", 2, Normal, 43),
    s(44, "Pedal Hi-Hat (hit)", -1, Cross, 44),
    s(45, "Low Tom (hit)", 3, Normal, 45),
    s(46, "Hi-Hat (open)", 9, CircleCross, 46),
    s(47, "Mid Tom (hit)", 4, Normal, 47),
    s(48, "High Tom (hit)", 6, Normal, 48),
    s(49, "Crash high (hit)", 10, HeavyCross, 49),
    s(50, "High Floor Tom (hit)", 7, Normal, 50),
    s(51, "Ride (middle)", 8, Cross, 51),
    s(52, "China (hit)", 11, China, 52),
    s(53, "Ride (bell)", 8, Diamond, 53),
    s(54, "Tambourine (hit)", 5, Triangle, 54),
    s(55, "Splash (hit)", 10, Cross, 55),
    s(56, "Cowbell medium (hit)", 8, Triangle, 56),
    s(57, "Crash medium (hit)", 9, HeavyCross, 57),
    s(58, "Vibraslap (hit)", -20, Normal, 58),
    s(59, "Ride (edge)", 6, Cross, 59),
    s(60, "Bongo High (hit)", 12, Normal, 60),
    s(61, "Bongo Low (hit)", 15, Normal, 61),
    s(62, "Conga high (mute)", -11, Normal, 62),
    s(63, "Conga high (hit)", -6, Normal, 63),
    s(64, "Conga low (hit)", -9, Normal, 64),
    s(65, "Timbale high (hit)", -1, Normal, 65),
    s(66, "Timbale low (hit)", -2, Normal, 66),
    s(67, "Agogo high (hit)", -3, Normal, 67),
    s(68, "Agogo low (hit)", -4, Normal, 68),
    s(69, "Cabasa (hit)", -15, Normal, 69),
    s(70, "Left Maraca (hit)", 20, Normal, 70),
    s(71, "Whistle high (hit)", 25, Normal, 71),
    s(72, "Whistle low (hit)", 19, Normal, 72),
    s(73, "Guiro (hit)", -30, Normal, 73),
    s(74, "Guiro (scrap-return)", -29, Normal, 74),
    s(75, "Claves (hit)", -12, Normal, 75),
    s(76, "Woodblock high (hit)", 18, Triangle, 76),
    s(77, "Woodblock low (hit)", 17, Triangle, 77),
    s(78, "Cuica (mute)", -21, Cross, 78),
    s(79, "Cuica (open)", -22, Normal, 79),
    s(80, "Triangle (mute)", -18, Cross, 80),
    s(81, "Triangle (hit)", -19, Normal, 81),
    s(82, "Shaker (hit)", 31, Normal, 54),
    s(83, "Jingle Bell (hit)", 28, Normal, 53),
    s(84, "Bell Tree (hit)", 26, Normal, 53),
    s(85, "Castanets (hit)", -13, Normal, 85),
    s(86, "Surdo (hit)", -28, Normal, 86),
    s(87, "Surdo (mute)", -27, Cross, 87),
    s(91, "Snare (rim shot)", 5, Diamond, 38),
    s(92, "Hi-Hat (half)", 9, CircleSlash, 46),
    s(93, "Ride (edge)", 8, Cross, 51),
    s(94, "Ride (choke)", 8, Cross, 51),
    s(95, "Splash (choke)", 10, Cross, 55),
    s(96, "China (choke)", 11, China, 52),
    s(97, "Crash high (choke)", 10, HeavyCross, 49),
    s(98, "Crash medium (choke)", 9, HeavyCross, 57),
    s(99, "Cowbell low (hit)", 7, Triangle, 56),
    s(100, "Cowbell low (tip)", 7, Cross, 56),
    s(101, "Cowbell medium (tip)", 8, Cross, 56),
    s(102, "Cowbell high (hit)", 9, Triangle, 56),
    s(103, "Cowbell high (tip)", 9, Cross, 56),
    s(104, "Bongo High (mute)", 13, Normal, 60),
    s(105, "Bongo High (slap)", 14, Cross, 60),
    s(106, "Bongo Low (mute)", 16, Normal, 61),
    s(107, "Bongo Low (slap)", 24, Cross, 61),
    s(108, "Conga low (slap)", -8, Cross, 64),
    s(109, "Conga low (mute)", -7, Normal, 64),
    s(110, "Conga high (slap)", -5, Cross, 63),
    s(111, "Tambourine (return)", 6, Triangle, 54),
    s(112, "Tambourine (roll)", 7, Triangle, 54),
    s(113, "Tambourine (hand)", 15, Cross, 54),
    s(114, "Grancassa (hit)", -17, Normal, 43),
    s(115, "Piatti (hit)", -10, Normal, 49),
    s(116, "Piatti (hand)", -16, Cross, 49),
    s(117, "Cabasa (return)", -14, Normal, 69),
    s(118, "Left Maraca (return)", 21, Normal, 70),
    s(119, "Right Maraca (hit)", 22, Normal, 70),
    s(120, "Right Maraca (return)", 23, Normal, 70),
    s(121, "Snare (hit)", 5, Normal, 38),
    s(122, "Shaker (return)", 32, Normal, 54),
    s(123, "Bell Tree (return)", 27, Normal, 53),
    s(124, "Golpe (thumb)", 29, Cross, 62),
    s(125, "Golpe (finger)", 30, Cross, 62),
    s(126, "Ride (middle)", 6, Cross, 59),
    s(127, "Ride (bell)", 6, Diamond, 59),
];

/// The sounds of the drumkit view's staff, from left to right.
pub const STAFF_KIT: [u8; 24] = [38, 37, 91, 42, 92, 46, 44, 35, 36, 50, 48, 47, 45, 43, 93, 51, 53, 55, 52, 49, 57, 99, 56, 102];

/// The numbers General MIDI itself gives to drum sounds.
pub const GENERAL_MIDI: std::ops::RangeInclusive<u8> = 35..=81;

pub fn drum(p: u8) -> Option<&'static DrumSound> {
    SOUNDS.binary_search_by_key(&p, |d| d.midi).ok().map(|i| &SOUNDS[i])
}

/// Name of a drum sound.
pub fn drum_name(p: u8) -> &'static str {
    drum(p).map(|d| d.name).unwrap_or("Percussion")
}

/// The positions a score can show: those the notation cursor reaches.
const SHOWN: std::ops::RangeInclusive<i32> = -8..=16;

/// Where a drum sound is shown on the percussion staff (0 = bottom line) and its notehead.
/// A sound Guitar Pro writes further off the staff than the score can show is a cross in the
/// middle of it.
pub fn drum_staff_pos(p: u8) -> (i32, DrumHead) {
    drum(p).map(|d| (d.pos as i32, d.head)).filter(|(pos, _)| SHOWN.contains(pos)).unwrap_or((4, Cross))
}

/// The sign drawn with a drum note, if any.
pub fn drum_sign(p: u8) -> Option<DrumSign> {
    match p {
        59 | 93 => Some(DrumSign::Edge),
        29 | 94 | 95 | 97 | 98 => Some(DrumSign::Choke),
        62 | 80 | 87 | 104 | 106 | 109 => Some(DrumSign::Muted),
        _ => None,
    }
}

/// The General MIDI sound a drum note plays as.
pub fn drum_sound(p: u8) -> u8 {
    drum(p).map(|d| d.gm).unwrap_or(p)
}

/// The staff sounds written on a line or space, in the order of their number keys (1, 2, ...).
pub fn drums_on_line(pos: i32) -> Vec<u8> {
    STAFF_KIT.iter().copied().filter(|&p| drum_staff_pos(p).0 == pos).collect()
}

/// The drum sound a line or space of the percussion staff stands for: the first one on it.
pub fn drum_at_pos(pos: i32) -> u8 {
    let (low, high) = STAFF_KIT.iter().fold((i32::MAX, i32::MIN), |(l, h), &p| {
        let at = drum_staff_pos(p).0;
        (l.min(at), h.max(at))
    });
    drums_on_line(pos.clamp(low, high)).first().copied().unwrap_or(38)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_is_in_order_and_plays_sounds_a_midi_device_knows() {
        assert!(SOUNDS.windows(2).all(|w| w[0].midi < w[1].midi));
        for d in &SOUNDS {
            assert!((27..=87).contains(&d.gm), "{} plays as {}", d.midi, d.gm);
        }
        assert_eq!(drum_name(91), "Snare (rim shot)");
        assert_eq!(drum_sound(91), 38);
        assert_eq!((drum_name(0), drum_sound(0)), ("Percussion", 0));
    }

    #[test]
    fn number_keys_follow_the_original() {
        // The boxed numbers 1 to 4 in the picture of the original: all on the top line.
        assert_eq!(drums_on_line(8), vec![93, 51, 53, 56]);
        assert_eq!(drums_on_line(9), vec![42, 92, 46, 57, 102]);
        assert_eq!(drums_on_line(5), vec![38, 37, 91]);
    }

    #[test]
    fn sounds_far_off_the_staff_are_shown_in_the_middle() {
        assert_eq!(drum(73).map(|d| d.pos), Some(-30), "where Guitar Pro writes the guiro");
        assert_eq!(drum_staff_pos(73), (4, Cross));
        assert_eq!(drum_staff_pos(60), (12, Normal));
    }

    #[test]
    fn every_line_of_the_staff_has_a_sound() {
        for pos in -1..=11 {
            let p = drum_at_pos(pos);
            assert_eq!(drum_staff_pos(p).0, pos, "line {pos}");
        }
        assert_eq!(drum_at_pos(-8), 44);
        assert_eq!(drum_at_pos(16), 52);
    }
}
