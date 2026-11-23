//! Plans: changes to the score that the assistant proposes and the person approves.
//!
//! The assistant writes a short list of steps (copy bars, paste them, delete bars) in a fenced
//! block tagged [`TAG`]. This module reads that list, checks every step against the real song,
//! and runs it. The model only chooses the steps and their settings; the program does the editing,
//! so an answer that doesn't make sense (a track that isn't there, bars past the end) is reported
//! in plain words and changes nothing.
//!
//! [`run`] works on a copy of the song first, to show the person what would happen (see the card
//! in the AI panel); applying runs the same steps on the song as one undoable edit.

use crate::model::{Bar, MasterBar, Song};
use serde::Deserialize;

/// The tag of the fenced block that holds a plan.
pub const TAG: &str = "guitarcraft-plan";

/// The most steps in one plan, and the most bars a song may reach.
pub const MAX_STEPS: usize = 12;
const MAX_BARS: usize = 2000;

/// What the assistant is told about plans (added to its instructions).
pub const PLAN_HELP: &str = r#"You can change the score by proposing a plan. The person sees it as a card with an Apply button, so nothing changes until they press it. Write the plan as a fenced block tagged guitarcraft-plan holding a JSON array of steps, and say in a sentence or two what it does. Bars are numbered from 1. Use the track and section names from the song outline exactly. The steps:
{"op":"copy_bars","tracks":"all","section":"Verse"} copies bars. "tracks" is "all" or a track name or a list of names; the bars are a "section" name, or "from" and "to" bar numbers.
{"op":"paste_bars","at":12,"mode":"insert"} pastes what the last copy_bars copied. "at" is a bar number, or "end" for after the last bar. "insert" adds new bars before that bar in every track (tracks that weren't copied get empty bars). "replace" overwrites the copied tracks' bars from that bar on and leaves the other tracks as they are, adding bars at the end if it runs past it.
{"op":"delete_bars","section":"Intro"} removes bars from every track; give a "section", or "from" and "to".
Example, to copy the intro to the end of the song:
```guitarcraft-plan
[{"op":"copy_bars","tracks":"all","section":"Intro"},{"op":"paste_bars","at":"end","mode":"insert"}]
```
Only write a plan when the person asks you to change the score. If you can't tell what they want (insert or replace, say), ask them first and write no plan. If the change needs something these steps can't do, say so and explain how to do it by hand."#;

/// Said to the model right after the question, where it counts most: a question that comes with
/// guide text about doing something by hand can otherwise pull it away from writing a plan.
pub const REMINDER: &str = "(If this asks you to change the score, reply with a guitarcraft-plan block as your instructions describe, not with steps for the person to do by hand. If it is only a question, just answer it.)";

/// Whether the question is an order to change the score ("Copy the intro to the end", "Please delete
/// bars 3 to 4") and not a question about how ("How do I copy bars?"). The guide's how-to text is
/// left out of an order, so it doesn't talk the model into explaining instead of doing.
pub fn is_change_request(question: &str) -> bool {
    const VERBS: [&str; 14] = [
        "copy", "paste", "duplicate", "clone", "delete", "remove", "insert", "move", "replace", "repeat", "cut", "append", "double", "add",
    ];
    const POLITE: [&str; 9] = ["please ", "can you ", "could you ", "would you ", "will you ", "i want you to ", "i'd like you to ", "i want to ", "i would like to "];
    let mut q = question.trim().to_lowercase();
    loop {
        match POLITE.iter().find(|p| q.starts_with(**p)) {
            Some(p) => q = q[p.len()..].trim_start().to_string(),
            None => break,
        }
    }
    q.split(|c: char| !c.is_alphabetic()).next().is_some_and(|w| VERBS.contains(&w))
}

/// Which tracks: "all", one name, or a list of names.
#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum Tracks {
    One(String),
    Many(Vec<String>),
}

/// Where to paste: a bar number, or a word ("end").
#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum At {
    Bar(usize),
    Word(String),
}

/// How to paste: add bars, or write over the ones that are there.
#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Default)]
pub enum Mode {
    #[default]
    #[serde(rename = "insert", alias = "Insert")]
    Insert,
    #[serde(rename = "replace", alias = "Replace")]
    Replace,
}

/// One step of a plan.
#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Step {
    CopyBars {
        #[serde(default, alias = "track")]
        tracks: Option<Tracks>,
        section: Option<String>,
        from: Option<usize>,
        to: Option<usize>,
    },
    PasteBars {
        at: At,
        #[serde(default)]
        mode: Mode,
    },
    DeleteBars {
        section: Option<String>,
        from: Option<usize>,
        to: Option<usize>,
    },
}

/// What a plan would do, in words, for the card.
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    /// One line for each step.
    pub lines: Vec<String>,
    pub bars_before: usize,
    pub bars_after: usize,
    /// Things to know before applying (a bar that won't fit where it goes).
    pub notes: Vec<String>,
}

// ----- reading a plan

/// The steps in the text of a plan block: a JSON array, or an object with a "steps" array.
pub fn parse(text: &str) -> Result<Vec<Step>, String> {
    #[derive(Deserialize)]
    struct Wrapped {
        steps: Vec<Step>,
    }
    let text = text.trim();
    let steps = match serde_json::from_str::<Vec<Step>>(text) {
        Ok(s) => s,
        Err(first) => match serde_json::from_str::<Wrapped>(text) {
            Ok(w) => w.steps,
            Err(_) => return Err(format!("The plan couldn't be read ({first}).")),
        },
    };
    if steps.is_empty() {
        return Err("The plan has no steps.".into());
    }
    if steps.len() > MAX_STEPS {
        return Err(format!("The plan has {} steps; the most is {MAX_STEPS}.", steps.len()));
    }
    Ok(steps)
}

/// Whether a fenced block is a plan: tagged as one, or untagged JSON that has steps in it.
pub fn is_plan(lang: &str, text: &str) -> bool {
    lang.eq_ignore_ascii_case(TAG)
        || (matches!(lang.to_ascii_lowercase().as_str(), "json" | "") && text.contains("\"op\"") && text.contains("_bars"))
}

// ----- the song, as the plan names it

fn norm(s: &str) -> String {
    s.trim().trim_matches(|c| c == '[' || c == ']').trim().to_lowercase()
}

/// The named sections: (name, first bar, last bar), bars counted from 0. A section runs from the
/// bar that names it to the bar before the next name.
pub fn sections(song: &Song) -> Vec<(String, usize, usize)> {
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

fn track_list(song: &Song) -> String {
    song.tracks.iter().map(|t| format!("\"{}\"", t.name)).collect::<Vec<_>>().join(", ")
}

/// The track a name means: the one with that name, or the only one whose name holds it.
fn find_track(song: &Song, name: &str) -> Result<usize, String> {
    let n = norm(name);
    let exact: Vec<usize> = (0..song.tracks.len()).filter(|i| norm(&song.tracks[*i].name) == n).collect();
    let partial: Vec<usize> = (0..song.tracks.len())
        .filter(|i| {
            let t = norm(&song.tracks[*i].name);
            !n.is_empty() && !t.is_empty() && (t.contains(&n) || n.contains(&t))
        })
        .collect();
    // A number means the track in that place (the outline lists them as 1, 2, ...), as in "2" or
    // "track 2", unless a track is really named that.
    if exact.is_empty()
        && let Ok(k) = n.strip_prefix("track").unwrap_or(&n).trim().parse::<usize>()
    {
        return if (1..=song.tracks.len()).contains(&k) {
            Ok(k - 1)
        } else {
            Err(format!("There is no track number {k}. The tracks are {}.", track_list(song)))
        };
    }
    let found = if exact.is_empty() { partial } else { exact };
    match found.as_slice() {
        [one] => Ok(*one),
        [] => Err(format!("There is no track called \"{name}\". The tracks are {}.", track_list(song))),
        many => Err(format!(
            "\"{name}\" could mean any of {}. Say which one.",
            many.iter().map(|i| format!("\"{}\"", song.tracks[*i].name)).collect::<Vec<_>>().join(", ")
        )),
    }
}

/// The tracks a step means (all of them, by default), and whether that is every track.
fn find_tracks(song: &Song, sel: &Option<Tracks>) -> Result<(Vec<usize>, bool), String> {
    let names: Vec<&String> = match sel {
        None => vec![],
        Some(Tracks::One(n)) => vec![n],
        Some(Tracks::Many(v)) => v.iter().collect(),
    };
    let every = |n: &str| matches!(norm(n).as_str(), "all" | "every" | "all tracks" | "every track" | "everything" | "*");
    if names.is_empty() || names.iter().any(|n| every(n)) {
        return Ok(((0..song.tracks.len()).collect(), true));
    }
    let mut out: Vec<usize> = vec![];
    for n in names {
        let i = find_track(song, n)?;
        if !out.contains(&i) {
            out.push(i);
        }
    }
    out.sort();
    let all = out.len() == song.tracks.len();
    Ok((out, all))
}

/// The bars a step means, counted from 0: a section by name, or `from` to `to` counted from 1.
/// Also what to call them.
fn find_bars(song: &Song, section: &Option<String>, from: Option<usize>, to: Option<usize>) -> Result<(usize, usize, String), String> {
    let count = song.master.len();
    if let Some(name) = section {
        let n = norm(name);
        let secs = sections(song);
        let exact: Vec<&(String, usize, usize)> = secs.iter().filter(|s| norm(&s.0) == n).collect();
        let partial: Vec<&(String, usize, usize)> = secs.iter().filter(|s| !n.is_empty() && (norm(&s.0).contains(&n) || n.contains(&norm(&s.0)))).collect();
        let found = if exact.is_empty() { partial } else { exact };
        return match found.as_slice() {
            [s] => Ok((s.1, s.2, format!("{} (bars {}-{})", s.0, s.1 + 1, s.2 + 1))),
            [] if secs.is_empty() => Err(format!("The song has no named sections, so \"{name}\" can't be found. Use bar numbers.")),
            [] => Err(format!(
                "There is no section called \"{name}\". The sections are {}.",
                secs.iter().map(|s| format!("\"{}\" (bars {}-{})", s.0, s.1 + 1, s.2 + 1)).collect::<Vec<_>>().join(", ")
            )),
            many => Err(format!(
                "\"{name}\" could mean {}. Say which, or give bar numbers.",
                many.iter().map(|s| format!("\"{}\" (bars {}-{})", s.0, s.1 + 1, s.2 + 1)).collect::<Vec<_>>().join(" or ")
            )),
        };
    }
    let Some(a) = from else {
        return Err("The step needs a section, or the bars to use (\"from\" and \"to\").".into());
    };
    let b = to.unwrap_or(a);
    if a < 1 || b < a || b > count {
        return Err(format!("Bars {a} to {b} aren't in the song, which has bars 1 to {count}."));
    }
    Ok((a - 1, b - 1, if a == b { format!("bar {a}") } else { format!("bars {a}-{b}") }))
}

fn names_of(song: &Song, tracks: &[usize], all: bool) -> String {
    if all {
        return "every track".into();
    }
    let names: Vec<String> = tracks.iter().map(|i| song.tracks[*i].name.clone()).collect();
    match names.as_slice() {
        [one] => one.clone(),
        [init @ .., last] => format!("{} and {}", init.join(", "), last),
        [] => String::new(),
    }
}

// ----- running a plan

/// What the last copy_bars took, so paste_bars can put it somewhere.
struct Clip {
    tracks: Vec<usize>,
    /// The master bars of the copied bars (time signature, tempo, repeats, section names).
    master: Vec<MasterBar>,
    /// For each copied bar, the bar of each copied track.
    bars: Vec<Vec<Bar>>,
    label: String,
    who: String,
}

/// Runs the steps on `song`. On an error the song may be half changed, so run it on a copy first
/// (and apply by running again on the real one).
pub fn run(song: &mut Song, steps: &[Step]) -> Result<Outcome, String> {
    let before = song.master.len();
    let mut lines: Vec<String> = vec![];
    let mut notes: Vec<String> = vec![];
    let mut clip: Option<Clip> = None;
    for (n, step) in steps.iter().enumerate() {
        let at = |e: String| format!("Step {}: {e}", n + 1);
        match step {
            Step::CopyBars { tracks, section, from, to } => {
                let (tr, all) = find_tracks(song, tracks).map_err(at)?;
                let (a, b, label) = find_bars(song, section, *from, *to).map_err(at)?;
                let who = names_of(song, &tr, all);
                lines.push(format!("Copy {label} of {who}"));
                clip = Some(Clip {
                    master: song.master[a..=b].to_vec(),
                    bars: (a..=b).map(|i| tr.iter().map(|t| song.tracks[*t].bars[i].clone()).collect()).collect(),
                    tracks: tr,
                    label,
                    who,
                });
            }
            Step::PasteBars { at: place, mode } => {
                let Some(c) = clip.as_ref() else {
                    return Err(at("Nothing has been copied yet, so there is nothing to paste. A copy_bars step has to come first.".into()));
                };
                let count = song.master.len();
                let pos = match place {
                    At::Bar(b) => *b,
                    At::Word(w) if matches!(norm(w).as_str(), "end" | "the end" | "last" | "after the last bar" | "append") => count + 1,
                    At::Word(w) => return Err(at(format!("\"{w}\" isn't a place to paste. Use a bar number, or \"end\"."))),
                };
                if pos < 1 || pos > count + 1 {
                    return Err(at(format!("Bar {pos} isn't in the song, which has bars 1 to {count} (or \"end\" to add after the last).")));
                }
                let idx = pos - 1;
                let len = c.master.len();
                if count + len > MAX_BARS {
                    return Err(at(format!("That would make the song longer than {MAX_BARS} bars.")));
                }
                match mode {
                    Mode::Insert => {
                        for (k, mb) in c.master.iter().enumerate() {
                            song.master.insert(idx + k, mb.clone());
                            for (ti, t) in song.tracks.iter_mut().enumerate() {
                                let bar = c.tracks.iter().position(|x| *x == ti).map(|p| c.bars[k][p].clone()).unwrap_or_else(Bar::empty);
                                t.bars.insert(idx + k, bar);
                            }
                        }
                        let place = if pos == count + 1 { format!("after the last bar (bar {count})") } else { format!("before bar {pos}") };
                        lines.push(format!("Insert {} ({}) {place}", c.label, c.who));
                    }
                    Mode::Replace => {
                        let mut mismatched = 0;
                        for k in 0..len {
                            let i = idx + k;
                            while i >= song.master.len() {
                                song.master.push(c.master[k].clone());
                                for t in song.tracks.iter_mut() {
                                    t.bars.push(Bar::empty());
                                }
                            }
                            if song.master[i].ticks() != c.master[k].ticks() {
                                mismatched += 1;
                            }
                            for (p, ti) in c.tracks.iter().enumerate() {
                                song.tracks[*ti].bars[i] = c.bars[k][p].clone();
                            }
                        }
                        if mismatched > 0 {
                            notes.push(format!(
                                "{mismatched} bar(s) are a different length from the bars they replace (a different time signature), so they will show as too long or too short."
                            ));
                        }
                        let grew = song.master.len().saturating_sub(count);
                        lines.push(format!(
                            "Replace bars {}-{} of {} with {}{}",
                            pos,
                            pos + len - 1,
                            c.who,
                            c.label,
                            if grew > 0 { format!(" (the song grows by {grew} bar(s))") } else { String::new() }
                        ));
                    }
                }
            }
            Step::DeleteBars { section, from, to } => {
                let (a, b, label) = find_bars(song, section, *from, *to).map_err(at)?;
                if b - a + 1 >= song.master.len() {
                    return Err(at("That would delete every bar; a song needs at least one.".into()));
                }
                song.master.drain(a..=b);
                for t in song.tracks.iter_mut() {
                    t.bars.drain(a..=b);
                }
                lines.push(format!("Delete {label} from every track"));
            }
        }
    }
    song.normalize();
    Ok(Outcome { lines, bars_before: before, bars_after: song.master.len(), notes })
}

// ----- the assistant's reply, as prose and blocks

/// A piece of a reply: text, or a fenced block with the tag after its opening fence.
#[derive(Debug, PartialEq)]
pub enum Block<'a> {
    Prose(&'a str),
    Code { lang: &'a str, text: &'a str },
}

/// Splits a reply at its fenced code blocks. A fence that never closes runs to the end.
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
        let lang = after[..nl].trim();
        let body = &after[(nl + 1).min(after.len())..];
        match body.find("```") {
            Some(j) => {
                out.push(Block::Code { lang, text: body[..j].trim_end() });
                rest = &body[j + 3..];
            }
            None => {
                out.push(Block::Code { lang, text: body.trim_end() });
                break;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Track;

    /// A song of `n` bars with the named tracks, each bar's first beat holding its own number as
    /// the fret of a note so that a bar can be told from another, and the given sections.
    fn song(tracks: &[&str], n: usize, sections: &[(usize, &str)]) -> Song {
        let mut s = Song::new_blank();
        s.master = vec![MasterBar::default(); n];
        s.tracks = tracks
            .iter()
            .map(|name| {
                let mut t = Track { name: name.to_string(), ..Track::default() };
                t.bars = (0..n)
                    .map(|i| {
                        let mut bar = Bar::empty();
                        bar.beats[0].notes.push(crate::model::Note { string: 0, fret: i as u8, ..Default::default() });
                        bar
                    })
                    .collect();
                t
            })
            .collect();
        for (bar, name) in sections {
            s.master[*bar].marker = Some(name.to_string());
        }
        s
    }

    fn fret(s: &Song, track: usize, bar: usize) -> u8 {
        s.tracks[track].bars[bar].beats[0].notes[0].fret
    }

    fn plan(json: &str) -> Vec<Step> {
        parse(json).unwrap_or_else(|e| panic!("{e}"))
    }

    #[test]
    fn plans_are_read_in_either_shape_and_odd_ones_are_refused() {
        let a = plan(r#"[{"op":"copy_bars","tracks":"all","section":"Intro"},{"op":"paste_bars","at":"end","mode":"insert"}]"#);
        assert_eq!(a.len(), 2);
        assert_eq!(a, plan(r#"{"steps":[{"op":"copy_bars","track":"all","section":"Intro"},{"op":"paste_bars","at":"end"}]}"#));
        assert!(matches!(&a[1], Step::PasteBars { at: At::Word(w), mode: Mode::Insert } if w == "end"));
        assert!(parse("not json").unwrap_err().contains("couldn't be read"));
        assert!(parse(r#"[{"op":"explode"}]"#).is_err());
        assert!(parse("[]").unwrap_err().contains("no steps"));
        let many = format!("[{}]", vec![r#"{"op":"delete_bars","from":1}"#; MAX_STEPS + 1].join(","));
        assert!(parse(&many).unwrap_err().contains("the most is"));
    }

    #[test]
    fn a_fenced_block_is_a_plan_when_tagged_or_obviously_one() {
        assert!(is_plan("guitarcraft-plan", "[]"));
        assert!(is_plan("json", r#"[{"op":"copy_bars"}]"#));
        assert!(!is_plan("json", r#"{"name":"x"}"#));
        assert!(!is_plan("rust", r#"let x = "op";"#));
    }

    #[test]
    fn replies_are_split_into_prose_and_blocks() {
        let b = split_blocks("Here you go.\n```guitarcraft-plan\n[1]\n```\nAnd done.");
        assert_eq!(b, vec![Block::Prose("Here you go."), Block::Code { lang: "guitarcraft-plan", text: "[1]" }, Block::Prose("And done.")]);
        assert_eq!(split_blocks("no code"), vec![Block::Prose("no code")]);
        // An unfinished fence is code to the end, so a plan cut off mid-answer isn't shown as prose.
        assert_eq!(split_blocks("x\n```json\n[1"), vec![Block::Prose("x"), Block::Code { lang: "json", text: "[1" }]);
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
    fn the_intro_is_copied_to_the_end_in_every_track() {
        let mut s = song(&["Guitar", "Bass"], 12, &[(0, "Intro"), (4, "Verse")]);
        let o = run(&mut s, &plan(r#"[{"op":"copy_bars","tracks":"all","section":"Intro"},{"op":"paste_bars","at":"end","mode":"insert"}]"#)).unwrap();
        assert_eq!((o.bars_before, o.bars_after), (12, 16));
        assert_eq!(s.master.len(), 16);
        for t in 0..2 {
            assert_eq!(s.tracks[t].bars.len(), 16);
            assert_eq!((12..16).map(|b| fret(&s, t, b)).collect::<Vec<_>>(), vec![0, 1, 2, 3]);
        }
        assert_eq!(o.lines, vec!["Copy Intro (bars 1-4) of every track", "Insert Intro (bars 1-4) (every track) after the last bar (bar 12)"]);
    }

    #[test]
    fn one_track_of_a_section_replaces_bars_of_that_track_only() {
        let mut s = song(&["Guitar", "Bass"], 16, &[(4, "Verse 1"), (8, "Chorus")]);
        let o = run(&mut s, &plan(r#"[{"op":"copy_bars","tracks":"guitar","section":"Verse 1"},{"op":"paste_bars","at":12,"mode":"replace"}]"#)).unwrap();
        assert_eq!((o.bars_before, o.bars_after), (16, 16), "nothing was added");
        // Guitar bars 12 to 15 (counted from 0: 11 to 14) now hold Verse 1's bars 5 to 8 (4 to 7).
        assert_eq!((11..15).map(|b| fret(&s, 0, b)).collect::<Vec<_>>(), vec![4, 5, 6, 7]);
        assert_eq!(fret(&s, 0, 15), 15, "the bar after is untouched");
        assert_eq!((11..16).map(|b| fret(&s, 1, b)).collect::<Vec<_>>(), vec![11, 12, 13, 14, 15], "the bass is untouched");
        assert_eq!(o.lines[1], "Replace bars 12-15 of Guitar with Verse 1 (bars 5-8)");
    }

    #[test]
    fn replacing_past_the_end_grows_every_track() {
        let mut s = song(&["Guitar", "Bass"], 16, &[(4, "Verse 1"), (8, "Chorus")]);
        let o = run(&mut s, &plan(r#"[{"op":"copy_bars","tracks":"guitar","section":"Verse 1"},{"op":"paste_bars","at":15,"mode":"replace"}]"#)).unwrap();
        assert_eq!(o.bars_after, 18);
        assert_eq!(o.lines[1], "Replace bars 15-18 of Guitar with Verse 1 (bars 5-8) (the song grows by 2 bar(s))");
        assert!(s.tracks.iter().all(|t| t.bars.len() == 18), "every track grows with the song");
        assert_eq!((14..18).map(|b| fret(&s, 0, b)).collect::<Vec<_>>(), vec![4, 5, 6, 7]);
        assert!(s.tracks[1].bars[16].is_empty() && s.tracks[1].bars[17].is_empty(), "the bass gets empty bars");
    }

    #[test]
    fn inserting_bars_of_some_tracks_gives_the_others_empty_bars() {
        let mut s = song(&["Guitar", "Bass"], 8, &[]);
        run(&mut s, &plan(r#"[{"op":"copy_bars","tracks":["Guitar"],"from":1,"to":2},{"op":"paste_bars","at":5}]"#)).unwrap();
        assert_eq!(s.master.len(), 10);
        assert_eq!((4..6).map(|b| fret(&s, 0, b)).collect::<Vec<_>>(), vec![0, 1]);
        assert!(s.tracks[1].bars[4].is_empty() && s.tracks[1].bars[5].is_empty());
        assert_eq!(fret(&s, 1, 6), 4, "the bass bars after it moved along");
    }

    #[test]
    fn bars_are_deleted_from_every_track() {
        let mut s = song(&["Guitar", "Bass"], 8, &[(0, "Intro"), (2, "Verse")]);
        let o = run(&mut s, &plan(r#"[{"op":"delete_bars","section":"intro"}]"#)).unwrap();
        assert_eq!((o.bars_before, o.bars_after), (8, 6));
        assert_eq!(fret(&s, 1, 0), 2);
        assert_eq!(o.lines, vec!["Delete Intro (bars 1-2) from every track"]);
        assert!(run(&mut s, &plan(r#"[{"op":"delete_bars","from":1,"to":6}]"#)).unwrap_err().contains("at least one"));
    }

    #[test]
    fn mistakes_are_named_plainly() {
        let s = song(&["Guitar", "Bass"], 8, &[(0, "Verse A"), (4, "Verse B")]);
        let err = |json: &str| run(&mut s.clone(), &plan(json)).unwrap_err();
        assert!(err(r#"[{"op":"copy_bars","tracks":"Banjo","from":1}]"#).contains("no track called \"Banjo\". The tracks are \"Guitar\", \"Bass\""));
        assert!(err(r#"[{"op":"copy_bars","section":"Bridge"}]"#).contains("no section called \"Bridge\""));
        assert!(err(r#"[{"op":"copy_bars","section":"Verse"}]"#).contains("could mean \"Verse A\" (bars 1-4) or \"Verse B\" (bars 5-8)"));
        assert!(err(r#"[{"op":"copy_bars","from":5,"to":20}]"#).contains("aren't in the song"));
        assert!(err(r#"[{"op":"paste_bars","at":"end"}]"#).contains("Nothing has been copied"));
        assert!(err(r#"[{"op":"copy_bars","from":1},{"op":"paste_bars","at":99}]"#).starts_with("Step 2: Bar 99"));
        assert!(err(r#"[{"op":"copy_bars","from":1},{"op":"paste_bars","at":"somewhere"}]"#).contains("isn't a place to paste"));
        assert!(err(r#"[{"op":"copy_bars"}]"#).contains("needs a section"));
    }

    #[test]
    fn a_track_name_may_be_part_of_the_real_one() {
        let s = song(&["Lead Guitar", "Rhythm Guitar", "Bass"], 4, &[]);
        assert_eq!(find_track(&s, "bass"), Ok(2));
        assert_eq!(find_track(&s, "lead"), Ok(0));
        assert!(find_track(&s, "guitar").unwrap_err().contains("Say which one"));
    }

    #[test]
    fn a_track_may_be_named_by_its_number_in_the_outline() {
        let s = song(&["Steel Guitar", "Electric Bass"], 4, &[]);
        assert_eq!(find_track(&s, "2"), Ok(1));
        assert_eq!(find_track(&s, "Track 1"), Ok(0));
        assert!(find_track(&s, "3").unwrap_err().contains("no track number 3"));
        // A track really named "2" wins over the place.
        let t = song(&["Bass", "2"], 4, &[]);
        assert_eq!(find_track(&t, "2"), Ok(1));
        assert_eq!(find_tracks(&s, &Some(Tracks::One("2".into()))), Ok((vec![1], false)));
    }

    #[test]
    fn replacing_bars_of_a_different_length_is_noted() {
        let mut s = song(&["Guitar"], 8, &[]);
        s.master[4].num = 3; // a bar of 3/4 among the 4/4 ones
        let o = run(&mut s, &plan(r#"[{"op":"copy_bars","from":5},{"op":"paste_bars","at":1,"mode":"replace"}]"#)).unwrap();
        assert_eq!(o.notes.len(), 1);
        assert!(o.notes[0].contains("different length"));
    }

    #[test]
    fn orders_are_told_from_questions() {
        for yes in ["Copy the intro to the end of the song", "please delete bars 3 to 4", "Can you copy the guitar track for Verse 1?", "I want you to remove the outro", "  Paste it at bar 12", "Add a copy of the chorus at the end"] {
            assert!(is_change_request(yes), "{yes}");
        }
        for no in ["How do I copy bars?", "What key is this in?", "Where do I paste?", "Why is bar 3 red?", "", "Can you tell me what scale fits?", "Is the intro 4 bars?"] {
            assert!(!is_change_request(no), "{no}");
        }
    }
}
