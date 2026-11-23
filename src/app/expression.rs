//! Expression Text (Ctrl+Shift+E, or ">" in the command palette): type a short term and put
//! it on the score at the cursor. Dynamics (ff), chord names (G maj, Am7), key signatures
//! (G maj, E min), octave signs (8va), directions (Coda, D.S. al Fine) and, for anything
//! else, free text above the beat.

use super::App;
use crate::model::*;

/// One thing the typed term can put on the score.
#[derive(Clone, Debug, PartialEq)]
pub enum Entry {
    Dynamic(Dynamic),
    Chord(String),
    Key(i8, bool),
    Octave(i8),
    Target(u8),
    Jump(u8),
    Text(String),
}

impl Entry {
    /// The name the list shows.
    pub fn name(&self) -> String {
        match self {
            Entry::Dynamic(d) => d.label().to_string(),
            Entry::Chord(c) => c.clone(),
            Entry::Key(k, minor) => {
                let short = super::popups::key_short(*k, *minor);
                if *minor {
                    format!("{} minor", short.trim_end_matches('m'))
                } else {
                    format!("{short} major")
                }
            }
            Entry::Octave(v) => octave_name(*v).to_string(),
            Entry::Target(t) => TARGET_NAMES[*t as usize].to_string(),
            Entry::Jump(j) => JUMP_NAMES[*j as usize].to_string(),
            Entry::Text(t) => format!("\u{201C}{t}\u{201D}"),
        }
    }

    /// What it does, shown under the list for the highlighted entry.
    pub fn describe(&self) -> &'static str {
        match self {
            Entry::Dynamic(_) => "Set dynamic",
            Entry::Chord(_) => "Set chord",
            Entry::Key(..) => "Set key signature",
            Entry::Octave(_) => "Set octave sign",
            Entry::Target(_) => "Set direction at the start of the bar",
            Entry::Jump(_) => "Set direction at the end of the bar",
            Entry::Text(_) => "Set text",
        }
    }

    pub fn apply(&self, app: &mut App) {
        match self.clone() {
            Entry::Dynamic(d) => app.beat_edit(move |b| b.dynamic = d),
            Entry::Chord(c) => app.beat_edit(move |b| b.chord = Some(c.clone())),
            Entry::Key(k, minor) => app.bar_edit(move |m| {
                m.key = k;
                m.key_minor = minor;
            }),
            Entry::Octave(v) => app.beat_edit(move |b| b.ottava = v),
            Entry::Target(t) => app.bar_edit(move |m| m.direction_target = t),
            Entry::Jump(j) => app.bar_edit(move |m| m.direction_jump = j),
            Entry::Text(t) => app.beat_edit(move |b| b.text = Some(t.clone())),
        }
    }
}

fn octave_name(v: i8) -> &'static str {
    match v {
        1 => "8va",
        2 => "15ma",
        -1 => "8vb",
        _ => "15mb",
    }
}

/// Flats and sharps of the major and minor keys, by name ("Bb", "F#m"), as the key dialog writes them.
fn key_named(root: &str, minor: bool) -> Option<i8> {
    (-7..=7i8).find(|&k| super::popups::key_short(k, minor) == format!("{root}{}", if minor { "m" } else { "" }))
}

/// The chord endings that are offered, as typed, and how the score writes them.
const CHORD_ENDINGS: [(&str, &str); 22] = [
    ("", ""),
    ("maj", ""),
    ("major", ""),
    ("m", "m"),
    ("min", "m"),
    ("minor", "m"),
    ("7", "7"),
    ("maj7", "maj7"),
    ("m7", "m7"),
    ("min7", "m7"),
    ("6", "6"),
    ("m6", "m6"),
    ("9", "9"),
    ("add9", "add9"),
    ("sus2", "sus2"),
    ("sus4", "sus4"),
    ("dim", "dim"),
    ("dim7", "dim7"),
    ("aug", "aug"),
    ("+", "aug"),
    ("5", "5"),
    ("m7b5", "m7b5"),
];

/// A chord or key typed as a root and an ending ("G maj", "Bbm7", "f#min"): the root as it is
/// written, and the ending in lower case.
fn root_and_ending(term: &str) -> Option<(String, String)> {
    let t = term.trim();
    let mut chars = t.chars();
    let first = chars.next()?.to_ascii_uppercase();
    if !('A'..='G').contains(&first) {
        return None;
    }
    let rest = chars.as_str();
    let (acc, ending) = match rest.chars().next() {
        Some('#') => ("#", &rest[1..]),
        Some('b') if rest.len() == 1 || !rest[1..].starts_with(|c: char| c.is_ascii_alphabetic()) || rest.starts_with("bm") => ("b", &rest[1..]),
        _ => ("", rest),
    };
    Some((format!("{first}{acc}"), ending.trim().to_lowercase()))
}

/// Everything that can be put on the score, in the order the list shows it with nothing typed:
/// the chord roots, then dynamics, octave signs, key signatures and directions.
pub fn catalog() -> Vec<Entry> {
    const ROOTS: [&str; 17] = ["C", "C#", "Db", "D", "D#", "Eb", "E", "F", "F#", "Gb", "G", "G#", "Ab", "A", "A#", "Bb", "B"];
    let mut out: Vec<Entry> = ROOTS.iter().map(|r| Entry::Chord(r.to_string())).collect();
    out.extend(Dynamic::ALL.iter().map(|d| Entry::Dynamic(*d)));
    out.extend([1, 2, -1, -2].map(Entry::Octave));
    for minor in [false, true] {
        out.extend((-7..=7i8).map(|k| Entry::Key(k, minor)));
    }
    out.extend((1..TARGET_NAMES.len()).map(|k| Entry::Target(k as u8)));
    out.extend((1..JUMP_NAMES.len()).map(|k| Entry::Jump(k as u8)));
    out
}

/// What the typed term can put on the score, the likeliest first. With nothing typed it is the
/// whole catalog; otherwise what the term says (a chord such as Bbm7, say), then the catalog
/// entries it matches, and last the term itself as text.
pub fn entries(term: &str) -> Vec<Entry> {
    let t = term.trim();
    if t.is_empty() {
        return catalog();
    }
    let low = t.to_lowercase();
    let mut out = vec![];
    for d in Dynamic::ALL {
        if d.label() == low {
            out.push(Entry::Dynamic(d));
        }
    }
    if let Some((root, ending)) = root_and_ending(t) {
        if let Some((_, written)) = CHORD_ENDINGS.iter().find(|(typed, _)| *typed == ending) {
            out.push(Entry::Chord(format!("{root}{written}")));
        }
        let minor = matches!(ending.as_str(), "m" | "min" | "minor");
        if (minor || matches!(ending.as_str(), "maj" | "major"))
            && let Some(k) = key_named(&root, minor)
        {
            out.push(Entry::Key(k, minor));
        }
    }
    for v in [1, 2, -1, -2] {
        if octave_name(v) == low {
            out.push(Entry::Octave(v));
        }
    }
    let words = |name: &str| low.split_whitespace().all(|w| name.to_lowercase().contains(w));
    for e in catalog() {
        let name = e.name();
        let short_jump = match &e {
            Entry::Jump(_) => name.replace("Da Capo", "D.C.").replace("Dal Segno", "D.S."),
            _ => String::new(),
        };
        if (words(&name) || (!short_jump.is_empty() && words(&short_jump))) && !out.contains(&e) {
            out.push(e);
        }
    }
    // Anything is also text, so there is always something to put on the score.
    out.push(Entry::Text(t.to_string()));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dynamics_chords_and_keys() {
        assert_eq!(entries("ff")[0], Entry::Dynamic(Dynamic::Ff));
        assert_eq!(entries("mf")[0], Entry::Dynamic(Dynamic::Mf));
        let g = entries("G maj");
        assert_eq!(g[0], Entry::Chord("G".into()));
        assert_eq!(g[1], Entry::Key(1, false), "G major has one sharp");
        let e = entries("e min");
        assert_eq!(e[0], Entry::Chord("Em".into()));
        assert_eq!(e[1], Entry::Key(1, true), "E minor has one sharp");
        assert_eq!(entries("Bbm7")[0], Entry::Chord("Bbm7".into()));
        assert_eq!(entries("F#")[0], Entry::Chord("F#".into()));
        assert!(entries("Bb maj").contains(&Entry::Key(-2, false)));
    }

    #[test]
    fn octaves_directions_and_text() {
        assert_eq!(entries("8va")[0], Entry::Octave(1));
        assert_eq!(entries("15mb")[0], Entry::Octave(-2));
        assert!(entries("coda").contains(&Entry::Target(DIR_CODA)));
        assert!(entries("D.S. al fine").contains(&Entry::Jump(6)));
        assert_eq!(entries("dolce").last(), Some(&Entry::Text("dolce".into())));
        assert_eq!(entries("  "), catalog(), "nothing typed lists everything");
        assert_eq!(catalog()[0], Entry::Chord("C".into()), "the chord roots come first, as in Guitar Pro 8");
        assert_eq!(catalog()[1], Entry::Chord("C#".into()));
    }
}
