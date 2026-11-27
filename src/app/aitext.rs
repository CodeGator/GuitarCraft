//! Text for and from the AI Assistant: the outline of the song that goes with a question, and
//! the answer split into its words and its blocks of code (tablature, for one).

use crate::model::Song;

/// The named sections: (name, first bar, last bar), bars counted from 0. A section runs from the
/// bar that names it to the bar before the next name.
fn sections(song: &Song) -> Vec<(String, usize, usize)> {
    let starts: Vec<(String, usize)> = song
        .master
        .iter()
        .enumerate()
        .filter_map(|(i, m)| m.marker.as_ref().map(|n| (n.trim().trim_matches(|c| c == '[' || c == ']').trim().to_string(), i)))
        .filter(|(n, _)| !n.is_empty())
        .collect();
    starts
        .iter()
        .enumerate()
        .map(|(k, (name, from))| (name.clone(), *from, starts.get(k + 1).map(|s| s.1 - 1).unwrap_or(song.master.len() - 1)))
        .collect()
}

/// A short description of the song for the assistant: how many bars, the tracks and the sections.
pub fn outline(song: &Song) -> String {
    let tracks: Vec<String> = song.tracks.iter().enumerate().map(|(i, t)| format!("{} \"{}\"", i + 1, t.name)).collect();
    let secs = sections(song);
    let sections = if secs.is_empty() {
        "no sections are named".to_string()
    } else {
        secs.iter().map(|(n, a, b)| format!("\"{n}\" (bars {}-{})", a + 1, b + 1)).collect::<Vec<_>>().join(", ")
    };
    format!("Song outline: {} bars. Tracks: {}. Sections: {}.", song.master.len(), tracks.join(", "), sections)
}

/// A piece of an answer: text, or a fenced block.
#[derive(Debug, PartialEq)]
pub enum Block<'a> {
    Prose(&'a str),
    Code(&'a str),
}

/// Splits an answer at its fenced code blocks, dropping the tag after an opening fence. A fence
/// that never closes runs to the end.
pub fn split_blocks(text: &str) -> Vec<Block<'_>> {
    let mut out = vec![];
    let mut rest = text;
    loop {
        let Some(i) = rest.find("```") else {
            if !rest.trim().is_empty() {
                out.push(Block::Prose(rest.trim()));
            }
            break;
        };
        if !rest[..i].trim().is_empty() {
            out.push(Block::Prose(rest[..i].trim()));
        }
        let after = &rest[i + 3..];
        let nl = after.find('\n').unwrap_or(after.len());
        let body = &after[(nl + 1).min(after.len())..];
        match body.find("```") {
            Some(j) => {
                out.push(Block::Code(body[..j].trim_end()));
                rest = &body[j + 3..];
            }
            None => {
                out.push(Block::Code(body.trim_end()));
                break;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{MasterBar, Track};

    fn song(tracks: &[&str], n: usize, sections: &[(usize, &str)]) -> Song {
        let mut s = Song::new_blank();
        s.master = vec![MasterBar::default(); n];
        s.tracks = tracks.iter().map(|name| Track { name: name.to_string(), ..Track::default() }).collect();
        for (bar, name) in sections {
            s.master[*bar].marker = Some(name.to_string());
        }
        s
    }

    #[test]
    fn sections_run_to_the_next_name_and_the_outline_lists_them() {
        let s = song(&["Guitar", "Bass"], 12, &[(0, "Intro"), (4, "[Verse 1]"), (8, "Chorus")]);
        assert_eq!(sections(&s), vec![("Intro".into(), 0, 3), ("Verse 1".into(), 4, 7), ("Chorus".into(), 8, 11)]);
        let o = outline(&s);
        assert!(o.contains("12 bars") && o.contains("1 \"Guitar\", 2 \"Bass\"") && o.contains("\"Verse 1\" (bars 5-8)"), "{o}");
        assert!(outline(&song(&["Guitar"], 3, &[])).contains("no sections are named"));
    }

    #[test]
    fn answers_are_split_into_prose_and_blocks() {
        let b = split_blocks("Here you go.\n```text\nE|--0--|\n```\nAnd done.");
        assert_eq!(b, vec![Block::Prose("Here you go."), Block::Code("E|--0--|"), Block::Prose("And done.")]);
        assert_eq!(split_blocks("no code"), vec![Block::Prose("no code")]);
        // An unfinished fence is code to the end.
        assert_eq!(split_blocks("x\n```\nE|--"), vec![Block::Prose("x"), Block::Code("E|--")]);
    }
}
