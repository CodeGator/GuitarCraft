//! Chord diagrams worked out from chord names, not looked up in a list: the
//! name is read into the notes of the chord ("Badd9" is B, D#, F#, C#), and
//! the guitar's neck is searched for a fingering a hand can play (at most
//! four fingers, with a barre, within four frets) that has every needed note,
//! the right bass note, and as few muted strings as possible, as low on the
//! neck as it can be. Any six-string tuning; the Chords & Scales tool lists the
//! best fingering in each part of the neck.

use std::collections::HashMap;
use std::sync::Mutex;

/// The open strings, low E to high E, as MIDI notes.
const TUNING: [i32; 6] = [40, 45, 50, 55, 59, 64];

/// A chord's notes above its root (0 to 11), each with whether a diagram
/// must have it (a plain fifth, or a ninth under an eleventh, may be left out).
#[derive(Clone, Debug, PartialEq)]
pub struct Formula {
    pub tones: Vec<(i32, bool)>,
    /// A power chord ("E5"): two or three strings suit it best.
    pub power: bool,
}

fn eat(s: &mut &str, p: &str) -> bool {
    if s.starts_with(p) {
        *s = &s[p.len()..];
        true
    } else {
        false
    }
}

/// Reads the part of a chord name after its root ("m7b5", "maj9", "sus4",
/// "add9", "6/9", "7(#9)", "dim7" ...). None when it can't be read.
pub fn formula(quality: &str) -> Option<Formula> {
    let mut q = quality.trim().replace(['(', ')', ',', ' '], "");
    for (a, b) in [("\u{266F}", "#"), ("\u{266D}", "b"), ("\u{2212}", "-"), ("\u{00F8}7", "m7b5"), ("\u{00F8}", "m7b5"), ("\u{00B0}7", "dim7"), ("\u{00B0}", "dim"), ("\u{0394}", "maj")] {
        q = q.replace(a, b);
    }
    let mut s = q.as_str();
    let mut third: Option<i32> = Some(4);
    let mut fifth: Option<i32> = Some(7);
    let mut fifth_altered = false;
    let mut seventh: Option<i32> = None;
    let mut ext: Vec<(i32, bool)> = vec![];
    let mut maj = false;
    let mut dim = false;
    let mut power = false;
    // What kind of chord.
    let ma_digit = s.starts_with("ma") && s[2..].starts_with(|c: char| c.is_ascii_digit());
    if eat(&mut s, "maj") || eat(&mut s, "Maj") || eat(&mut s, "MA") || (ma_digit && eat(&mut s, "ma")) {
        maj = true;
    } else if s.starts_with('M') && !s.starts_with("Mi") {
        s = &s[1..];
        maj = true;
    } else if eat(&mut s, "min") || eat(&mut s, "mi") || eat(&mut s, "m") || eat(&mut s, "-") {
        third = Some(3);
        // m(maj7), mMaj7, mM7
        if eat(&mut s, "Maj") || eat(&mut s, "maj") || eat(&mut s, "M") {
            maj = true;
        }
    } else if eat(&mut s, "dim") {
        third = Some(3);
        fifth = Some(6);
        fifth_altered = true;
        dim = true;
    } else if eat(&mut s, "aug") || eat(&mut s, "+") {
        fifth = Some(8);
        fifth_altered = true;
    }
    let seventh_of = |maj: bool, dim: bool| if dim { 9 } else if maj { 11 } else { 10 };
    // Numbers, suspensions, additions and alterations, in any order.
    let mut first_number = true;
    while !s.is_empty() {
        if eat(&mut s, "69") || eat(&mut s, "6/9") {
            ext.push((9, true));
            ext.push((2, true));
        } else if eat(&mut s, "sus2") {
            third = None;
            ext.push((2, true));
        } else if eat(&mut s, "sus4") || eat(&mut s, "sus") {
            third = None;
            ext.push((5, true));
        } else if eat(&mut s, "add9") || eat(&mut s, "add2") {
            ext.push((2, true));
        } else if eat(&mut s, "add#11") {
            ext.push((6, true));
        } else if eat(&mut s, "add11") || eat(&mut s, "add4") {
            ext.push((5, true));
        } else if eat(&mut s, "add13") || eat(&mut s, "add6") {
            ext.push((9, true));
        } else if eat(&mut s, "no3") || eat(&mut s, "omit3") {
            third = None;
        } else if eat(&mut s, "no5") || eat(&mut s, "omit5") {
            fifth = None;
        } else if eat(&mut s, "alt") {
            seventh = Some(10);
            ext.push((1, true));
            ext.push((8, true));
        } else if eat(&mut s, "maj") || eat(&mut s, "Maj") || eat(&mut s, "M") {
            maj = true;
        } else if eat(&mut s, "b5") || eat(&mut s, "-5") {
            fifth = Some(6);
            fifth_altered = true;
        } else if eat(&mut s, "#5") || eat(&mut s, "+5") {
            fifth = Some(8);
            fifth_altered = true;
        } else if eat(&mut s, "b9") || eat(&mut s, "-9") {
            ext.retain(|e| e.0 != 2);
            ext.push((1, true));
            seventh = seventh.or(Some(10));
        } else if eat(&mut s, "#9") || eat(&mut s, "+9") {
            ext.retain(|e| e.0 != 2);
            ext.push((3, true));
            seventh = seventh.or(Some(10));
        } else if eat(&mut s, "#11") || eat(&mut s, "+11") {
            ext.push((6, true));
        } else if eat(&mut s, "b13") || eat(&mut s, "-13") || eat(&mut s, "b6") {
            ext.push((8, true));
        } else if eat(&mut s, "13") {
            seventh = Some(seventh_of(maj, dim));
            ext.push((2, false));
            ext.push((9, true));
        } else if eat(&mut s, "11") {
            seventh = Some(seventh_of(maj, dim));
            ext.push((2, false));
            ext.push((5, true));
        } else if eat(&mut s, "9") {
            seventh = Some(seventh_of(maj, dim));
            ext.push((2, true));
        } else if eat(&mut s, "7") {
            seventh = Some(seventh_of(maj, dim));
        } else if eat(&mut s, "6") {
            ext.push((9, true));
        } else if eat(&mut s, "5") {
            if first_number {
                third = None;
                power = true;
            }
        } else if eat(&mut s, "4") {
            third = None;
            ext.push((5, true));
        } else if eat(&mut s, "2") {
            third = None;
            ext.push((2, true));
        } else {
            return None;
        }
        first_number = false;
    }
    // "Cmaj" alone is just C major.
    let mut tones: Vec<(i32, bool)> = vec![(0, true)];
    if let Some(t) = third {
        tones.push((t, true));
    }
    if let Some(f) = fifth {
        tones.push((f, fifth_altered || power));
    }
    if let Some(v) = seventh {
        tones.push((v, true));
    }
    tones.extend(ext);
    // One entry per note; needed if any mention needs it.
    let mut out: Vec<(i32, bool)> = vec![];
    for (pc, need) in tones {
        let pc = pc.rem_euclid(12);
        match out.iter_mut().find(|t| t.0 == pc) {
            Some(t) => t.1 |= need,
            None => out.push((pc, need)),
        }
    }
    Some(Formula { tones: out, power })
}

/// How hard a fingering is (lower is better), or None when a hand can't play it.
fn cost(tuning: &[i32; 6], frets: &[i8; 6], f: &Formula, root: i32, bass: i32) -> Option<f32> {
    let played: Vec<usize> = (0..6).filter(|&s| frets[s] >= 0).collect();
    if played.len() < 2 {
        return None;
    }
    let notes: Vec<i32> = played.iter().map(|&s| tuning[s] + frets[s] as i32).collect();
    let pcs: Vec<i32> = notes.iter().map(|n| (n - root).rem_euclid(12)).collect();
    // Every needed note, and the bass note lowest.
    if f.tones.iter().any(|(pc, need)| *need && !pcs.contains(pc)) {
        return None;
    }
    let low = notes.iter().copied().min()?;
    if low.rem_euclid(12) != bass.rem_euclid(12) {
        return None;
    }
    // Two notes at one fret with higher notes between them need a barre at
    // that fret; with an open or muted string in the way, fingers would cross.
    for &a in &played {
        for &b in &played {
            if b > a + 1 && frets[a] > 0 && frets[a] == frets[b] {
                let between = a + 1..b;
                let higher = between.clone().any(|s| frets[s] > frets[a]);
                let gap = between.clone().any(|s| frets[s] <= 0);
                if higher && gap {
                    return None;
                }
            }
        }
    }
    // Fingers: one per fretted note, unless a barre at the lowest fret takes several.
    let fretted: Vec<usize> = played.iter().copied().filter(|&s| frets[s] > 0).collect();
    let mut fingers = fretted.len();
    if fingers > 4 {
        let min = fretted.iter().map(|&s| frets[s]).min()?;
        let at_min: Vec<usize> = fretted.iter().copied().filter(|&s| frets[s] == min).collect();
        let (a, b) = (*at_min.first()?, *at_min.last()?);
        // Under a barre every string sounds: none open or muted between its ends.
        if (a..=b).all(|s| frets[s] >= min) {
            fingers = 1 + fretted.iter().filter(|&&s| frets[s] > min).count();
        }
    }
    if fingers > 4 {
        return None;
    }
    let lo = fretted.iter().map(|&s| frets[s]).min().unwrap_or(0);
    let hi = fretted.iter().map(|&s| frets[s]).max().unwrap_or(0);
    let first = *played.first()?;
    let last = *played.last()?;
    let inner_mutes = (first..=last).filter(|&s| frets[s] < 0).count();
    let opens = played.iter().filter(|&&s| frets[s] == 0).count();
    let missing = f.tones.iter().filter(|(pc, need)| !*need && !pcs.contains(pc)).count();
    let strings = if f.power { (played.len() as f32 - 3.0).abs() } else { 6.0 - played.len() as f32 };
    // Strings sounding as high as (or higher than) the next string up are
    // awkward and unusual: the notes should rise across the strings.
    let crossings = notes.windows(2).filter(|w| w[0] >= w[1]).count();
    let mut c = 0.5 * lo as f32 + 1.2 * strings + 3.0 * inner_mutes as f32 + 0.5 * fingers as f32 - 0.2 * opens as f32 + 1.5 * missing as f32 + 1.5 * crossings as f32;
    // Reaching across four frets is hard.
    if hi - lo >= 3 {
        c += 2.0;
    }
    Some(c)
}

/// The highest stretch of the neck where open strings still go with fretted
/// notes (the first position); higher up, every shape is a moveable one.
const OPEN_UP_TO: i8 = 4;

/// The best fingering whose lowest fretted note is at each fret from 1 to
/// `last_start` (reaching at most four frets; at fret 1, also fingerings of
/// open strings only), with its cost. Anchoring each fingering to its lowest
/// fret keeps a cheap low fingering from standing in for every stretch above it,
/// so each part of the neck gets a shape of its own.
pub fn voicing_list(tuning: &[i32; 6], root: i32, f: &Formula, bass: Option<i32>, last_start: i8) -> Vec<(f32, [i8; 6])> {
    let bass = bass.unwrap_or(root);
    let mut allowed: Vec<i32> = f.tones.iter().map(|t| (root + t.0).rem_euclid(12)).collect();
    if !allowed.contains(&bass.rem_euclid(12)) {
        allowed.push(bass.rem_euclid(12));
    }
    let mut out = vec![];
    for start in 1..=last_start {
        let open = start <= OPEN_UP_TO;
        let options: Vec<Vec<i8>> = (0..6)
            .map(|s| {
                let mut o = vec![-1i8];
                for fr in std::iter::once(0).filter(|_| open).chain(start..start + 4) {
                    if allowed.contains(&((tuning[s] + fr as i32).rem_euclid(12))) {
                        o.push(fr);
                    }
                }
                o
            })
            .collect();
        let mut frets = [-1i8; 6];
        let mut best: Option<(f32, [i8; 6])> = None;
        // Near the nut, also the best moveable one (no open strings), so a
        // shape like E major's D shape (x x 2 4 5 4) isn't lost to an open one.
        let mut closed: Option<(f32, [i8; 6])> = None;
        search(&options, 0, &mut frets, &mut |fr| {
            let low = lowest_fret(fr);
            if low != start && !(start == 1 && low == 0) {
                return;
            }
            if let Some(c) = cost(tuning, fr, f, root, bass) {
                if best.map(|b| c < b.0 - 1e-6).unwrap_or(true) {
                    best = Some((c, *fr));
                }
                if open && !fr.contains(&0) && closed.map(|b| c < b.0 - 1e-6).unwrap_or(true) {
                    closed = Some((c, *fr));
                }
            }
        });
        out.extend(best);
        if closed.map(|c| c.1) != best.map(|b| b.1) {
            out.extend(closed);
        }
    }
    out
}

/// The best fingering found for a chord in standard tuning: root, the chord's
/// notes and the bass note (the root unless a slash chord says otherwise).
pub fn voicing(root: i32, f: &Formula, bass: Option<i32>) -> Option<[i8; 6]> {
    voicing_list(&TUNING, root, f, bass, 12)
        .into_iter()
        .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
        .map(|b| b.1)
}

/// The lowest fretted fret of a fingering (0 when only open strings are played).
pub fn lowest_fret(frets: &[i8; 6]) -> i8 {
    frets.iter().copied().filter(|&f| f > 0).min().unwrap_or(0)
}

static LISTS: Mutex<Option<HashMap<String, Vec<[i8; 6]>>>> = Mutex::new(None);

/// Playable fingerings of a chord, one for each part of the neck up to
/// `max_fret`, nearest the nut first. `quality` is the name after the root
/// ("m7", "sus4", "add9"). Worked out once and remembered.
pub fn voicings(tuning: &[i32; 6], root: i32, quality: &str, bass: Option<i32>, max_fret: i8) -> Vec<[i8; 6]> {
    voicings_omitting(tuning, root, quality, &[], bass, max_fret)
}

/// The same, leaving out the chord's notes at the given distances above the root
/// (0 the root, 3 or 4 the third, 7 the fifth ...). Leaving out every note is ignored.
pub fn voicings_omitting(tuning: &[i32; 6], root: i32, quality: &str, omit: &[i32], bass: Option<i32>, max_fret: i8) -> Vec<[i8; 6]> {
    let key = format!("{tuning:?}|{root}|{quality}|{omit:?}|{bass:?}|{max_fret}");
    if let Ok(c) = LISTS.lock()
        && let Some(v) = c.as_ref().and_then(|m| m.get(&key))
    {
        return v.clone();
    }
    let Some(mut f) = formula(quality) else { return vec![] };
    if !omit.is_empty() {
        let kept: Vec<(i32, bool)> = f.tones.iter().copied().filter(|t| !omit.contains(&t.0)).collect();
        if !kept.is_empty() {
            f.tones = kept;
        }
    }
    let mut all = voicing_list(tuning, root, &f, bass, (max_fret - 3).max(1));
    // Fragments of two or three strings aren't the chord's shapes (a power
    // chord excepted), unless nothing fuller can be played.
    let full = |v: &[i8; 6]| f.power || v.iter().filter(|&&x| x >= 0).count() >= 4;
    if all.iter().any(|a| full(&a.1)) {
        all.retain(|a| full(&a.1));
    }
    // Leave out awkward ones: judged without the cost of being high on the
    // neck (every part of the neck gets its best), a fingering must not be far
    // worse than the best of them. (Room for the stretchier moveable shapes,
    // like the D and C shapes of the CAGED system.)
    let plain = |c: f32, v: &[i8; 6]| c - 0.5 * lowest_fret(v) as f32;
    let best = all.iter().map(|a| plain(a.0, &a.1)).fold(f32::MAX, f32::min);
    all.retain(|a| plain(a.0, &a.1) <= best + 6.0);
    // Where the hand is: the nut for a fingering with open strings, else its lowest fret.
    let at = |v: &[i8; 6]| if v.contains(&0) { 0 } else { lowest_fret(v) };
    // One fingering per place on the neck: of two a fret or less apart, the better one.
    let mut out: Vec<[i8; 6]> = vec![];
    let mut by_cost: Vec<&(f32, [i8; 6])> = all.iter().collect();
    by_cost.sort_by(|a, b| plain(a.0, &a.1).partial_cmp(&plain(b.0, &b.1)).unwrap_or(std::cmp::Ordering::Equal));
    for (_, v) in by_cost {
        if !out.iter().any(|o| (at(o) - at(v)).abs() <= 1) {
            out.push(*v);
        }
    }
    // Nearest the nut first.
    out.sort_by_key(|v| (at(v), lowest_fret(v)));
    if let Ok(mut c) = LISTS.lock() {
        c.get_or_insert_with(HashMap::new).insert(key, out.clone());
    }
    out
}

/// Which finger plays each string of a fingering (0 = none, 1 index ... 4
/// little finger). When more than four strings are fretted, the index finger
/// lies across them as a barre at the lowest fret.
pub fn fingers(frets: &[i8; 6]) -> [u8; 6] {
    let fretted: Vec<usize> = (0..6).filter(|&s| frets[s] > 0).collect();
    let mut out = [0u8; 6];
    let Some(min) = fretted.iter().map(|&s| frets[s]).min() else { return out };
    let at_min: Vec<usize> = fretted.iter().copied().filter(|&s| frets[s] == min).collect();
    let barre = fretted.len() > 4 && at_min.len() >= 2;
    let mut next = 1u8;
    if barre {
        for &s in &at_min {
            out[s] = 1;
        }
        next = 2;
    }
    let mut rest: Vec<usize> = fretted.iter().copied().filter(|s| !barre || frets[*s] != min).collect();
    rest.sort_by_key(|&s| (frets[s], s));
    for s in rest {
        out[s] = next.min(4);
        next += 1;
    }
    out
}

fn search(options: &[Vec<i8>], s: usize, frets: &mut [i8; 6], visit: &mut dyn FnMut(&[i8; 6])) {
    if s == 6 {
        visit(frets);
        return;
    }
    for &o in &options[s] {
        frets[s] = o;
        search(options, s + 1, frets, visit);
    }
}

static CACHE: Mutex<Option<HashMap<String, Option<[i8; 6]>>>> = Mutex::new(None);

/// A chord's diagram from its name ("Badd9", "F#m7b5", "D/F#"), remembered
/// once worked out. None when the name can't be read or no hand could play it.
pub fn shape_of(name: &str, root: i32, quality: &str, bass: Option<i32>) -> Option<[i8; 6]> {
    if let Ok(mut c) = CACHE.lock() {
        let map = c.get_or_insert_with(HashMap::new);
        if let Some(v) = map.get(name) {
            return *v;
        }
        let v = formula(quality).and_then(|f| voicing(root, &f, bass));
        map.insert(name.to_string(), v);
        return v;
    }
    formula(quality).and_then(|f| voicing(root, &f, bass))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shape(name: &str) -> Option<[i8; 6]> {
        crate::chart::shape(name)
    }

    /// The notes a fingering sounds, as pitch classes.
    fn sounds(frets: [i8; 6]) -> Vec<i32> {
        (0..6).filter(|&s| frets[s] >= 0).map(|s| (TUNING[s] + frets[s] as i32).rem_euclid(12)).collect()
    }

    /// Lists some fingerings (cargo test list_shapes -- --ignored --nocapture).
    #[test]
    #[ignore]
    fn list_shapes() {
        for n in ["Badd9", "C#m", "C#", "B7", "E5", "D/F#", "G6/9", "Cdim7", "F#m7b5", "Ebmaj7", "Bb", "Am7", "Cadd9", "Gsus4", "E7#9", "A13", "Dmaj9", "Fm", "Abm", "C/G"] {
            println!("{n:8} {:?}", shape(n));
        }
    }

    #[test]
    fn common_chords_get_their_usual_shapes() {
        for (name, want) in [
            ("E", [0, 2, 2, 1, 0, 0]),
            ("A", [-1, 0, 2, 2, 2, 0]),
            ("D", [-1, -1, 0, 2, 3, 2]),
            ("G", [3, 2, 0, 0, 0, 3]),
            ("C", [-1, 3, 2, 0, 1, 0]),
            ("Am", [-1, 0, 2, 2, 1, 0]),
            ("Em", [0, 2, 2, 0, 0, 0]),
            ("Dm", [-1, -1, 0, 2, 3, 1]),
            ("F", [1, 3, 3, 2, 1, 1]),
            ("B", [-1, 2, 4, 4, 4, 2]),
            ("E7", [0, 2, 0, 1, 0, 0]),
            ("A7", [-1, 0, 2, 0, 2, 0]),
            ("E5", [0, 2, 2, -1, -1, -1]),
        ] {
            assert_eq!(shape(name), Some(want), "{name}");
        }
    }

    #[test]
    fn unusual_chords_are_worked_out_from_their_names() {
        let pc = |n: &str| crate::chart::split_chord(n).unwrap().0;
        for (name, notes) in [
            ("Badd9", vec!["B", "D#", "F#", "C#"]),
            ("C#m7b5", vec!["C#", "E", "G", "B"]),
            ("F#7#9", vec!["F#", "A#", "E", "A"]),
            ("Ebmaj7", vec!["Eb", "G", "Bb", "D"]),
            ("G6/9", vec!["G", "B", "E", "A"]),
            ("Bbm(maj7)", vec!["Bb", "Db", "A"]),
            ("Dsus2", vec!["D", "E", "A"]),
            ("Aadd11", vec!["A", "C#", "D"]),
            ("E7b9", vec!["E", "G#", "D", "F"]),
            ("Cdim7", vec!["C", "Eb", "Gb", "A"]),
            ("Gm11", vec!["G", "Bb", "F", "C"]),
            ("C13", vec!["C", "E", "Bb", "A"]),
            ("Cmadd9", vec!["C", "Eb", "D"]),
            ("Faug", vec!["F", "A", "C#"]),
            ("Asus4", vec!["A", "D", "E"]),
            ("D7sus4", vec!["D", "G", "C"]),
        ] {
            let frets = shape(name).unwrap_or_else(|| panic!("no diagram for {name}"));
            let heard = sounds(frets);
            for n in notes {
                assert!(heard.contains(&pc(n)), "{name} {frets:?} is missing {n}");
            }
            // The root is the lowest note.
            let low = (0..6).find(|&s| frets[s] >= 0).unwrap();
            assert_eq!((TUNING[low] + frets[low] as i32).rem_euclid(12), pc(name), "{name} {frets:?}");
            // A hand can play it: within four frets.
            let fr: Vec<i8> = frets.iter().copied().filter(|f| *f > 0).collect();
            if let (Some(a), Some(b)) = (fr.iter().min(), fr.iter().max()) {
                assert!(b - a <= 3, "{name} {frets:?}");
            }
        }
        // Slash chords put their bass note lowest.
        let d = shape("D/F#").unwrap();
        let low = (0..6).find(|&s| d[s] >= 0).unwrap();
        assert_eq!((TUNING[low] + d[low] as i32).rem_euclid(12), 6);
        // Names that aren't chords get no diagram.
        assert_eq!(shape("N.C."), None);
        assert_eq!(shape("Cxyz"), None);
    }
}

#[cfg(test)]
mod voicing_tests {
    use super::*;

    #[test]
    fn ordinary_chords_have_several_fingerings_up_the_neck() {
        let std = [40, 45, 50, 55, 59, 64];
        for (root, q) in [(0, ""), (9, "m"), (7, "7"), (2, "maj7"), (4, "sus4")] {
            let v = voicings(&std, root, q, None, 17);
            assert!(v.len() >= 2, "{root}{q}: {}", v.len());
            // Nearest the nut first.
            assert!(v.windows(2).all(|w| lowest_fret(&w[0]) <= lowest_fret(&w[1])));
        }
        // Another tuning gives different fingerings: open strings of DADGAD make D5/sus fingerings.
        let dadgad = [38, 45, 50, 55, 57, 62];
        let d = voicings(&dadgad, 2, "sus4", None, 17);
        assert!(d.iter().any(|f| f.iter().filter(|&&x| x == 0).count() >= 4), "{d:?}");
        // The CAGED system: a major chord has its open shape and the five
        // moveable shapes (E, D, C, A and G) up the neck.
        assert_eq!(
            voicings(&std, 7, "", None, 17),
            vec![[3, 2, 0, 0, 0, 3], [3, 5, 5, 4, 3, 3], [-1, -1, 5, 7, 8, 7], [-1, 10, 9, 7, 8, 7], [-1, 10, 12, 12, 12, 10], [15, 14, 12, 12, 12, 15]],
            "G"
        );
        assert_eq!(
            voicings(&std, 0, "", None, 17),
            vec![[-1, 3, 2, 0, 1, 0], [-1, 3, 5, 5, 5, 3], [8, 7, 5, 5, 5, 8], [8, 10, 10, 9, 8, 8], [-1, -1, 10, 12, 13, 12], [-1, 15, 14, 12, 13, 12]],
            "C"
        );
        // E major keeps its D and C shapes near the nut alongside the open E.
        let e = voicings(&std, 4, "", None, 17);
        for want in [[-1, -1, 2, 4, 5, 4], [-1, 7, 6, 4, 5, 4], [-1, 7, 9, 9, 9, 7], [12, 11, 9, 9, 9, 12], [12, 14, 14, 13, 12, 12]] {
            assert!(e.contains(&want), "E is missing {want:?}: {e:?}");
        }
        // Fingers: an open C uses fingers 3, 2, 1.
        assert_eq!(fingers(&[-1, 3, 2, 0, 1, 0]), [0, 3, 2, 0, 1, 0]);
        // A barre chord: F uses the index finger across fret 1.
        assert_eq!(fingers(&[1, 3, 3, 2, 1, 1]), [1, 3, 4, 2, 1, 1]);
    }
}
