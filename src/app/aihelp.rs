//! Help from the assistant: questions about using GuitarCraft are answered from the user guide
//! (the text of the Help window), not from what the model happens to remember about other
//! programs. The best matching sections of the guide go along with the question; the model is
//! told to answer from them and to say so when they don't cover it.
//!
//! The guide is the program's own text, so sending it carries nothing of the person's.

/// The most guide sections sent with one question, so a small model's context isn't swamped.
pub const MAX_SECTIONS: usize = 4;

/// Words too common to say what a question is about: small words, and verbs that fit almost any
/// question ("change the tuning", "add a repeat": it is the tuning and the repeat that matter).
const STOP: &[&str] = &[
    "the", "and", "how", "can", "what", "does", "with", "for", "you", "this", "that", "into", "from", "are", "have",
    "make", "use", "get", "not", "but", "why", "when", "where", "which", "who", "would", "could", "should", "will",
    "there", "their", "them", "then", "than", "about", "your", "please", "want", "need", "like", "change", "add", "set",
    "turn", "show", "put", "find", "way", "just", "work", "does", "did",
];

/// A word reduced to its stem, crudely: "repeats" and "repeating" both give "repeat".
fn stem(w: &str) -> String {
    let w = w.to_lowercase();
    let w = if w.len() > 5 { w.trim_end_matches("ing").to_string() } else { w };
    let w = if w.len() > 3 { w.trim_end_matches('s').to_string() } else { w };
    w
}

fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric()).filter(|w| w.len() >= 3).map(stem).collect()
}

/// Which sections (by index) best match the question, best first, at most `max`. A section's
/// title counts three times as much as its text. Sections that share no word with the question
/// are left out, so a question about music theory brings none.
pub fn pick(question: &str, sections: &[(String, String)], max: usize) -> Vec<usize> {
    let wanted: Vec<String> = words(question).into_iter().filter(|w| !STOP.contains(&w.as_str())).collect();
    if wanted.is_empty() {
        return vec![];
    }
    let mut scored: Vec<(usize, usize)> = sections
        .iter()
        .enumerate()
        .map(|(i, (title, body))| {
            let in_title = words(title);
            let in_body = words(body);
            let score: usize = wanted
                .iter()
                .map(|q| 3 * in_title.iter().filter(|w| *w == q).count() + in_body.iter().filter(|w| *w == q).count())
                .sum();
            (i, score)
        })
        .filter(|(_, s)| *s > 0)
        .collect();
    scored.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    scored.truncate(max);
    scored.into_iter().map(|(i, _)| i).collect()
}

/// The guide text to send with a question, or None when no section matches.
pub fn guide_context(question: &str, sections: &[(String, String)]) -> Option<String> {
    let mut picked = pick(question, sections, MAX_SECTIONS);
    if picked.is_empty() {
        return None;
    }
    picked.sort();
    let text: Vec<String> = picked.iter().map(|i| format!("## {}\n{}", sections[*i].0, sections[*i].1)).collect();
    Some(format!(
        "From the GuitarCraft user guide (answer questions about using GuitarCraft from this; if it doesn't cover the question, say so instead of guessing):\n\n{}",
        text.join("\n\n")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn guide() -> Vec<(String, String)> {
        [
            ("Bars, repeats and directions", "Repeat open and repeat close mark repeats. Bar > Insert bar changes every track."),
            ("Tracks and the mixer", "Add tracks from the Track menu. The Track tab sets the name, tuning, capo and sound."),
            ("Playing back", "Play plays from the cursor. Loop repeats the selection. The metronome is in the toolbar."),
            ("Lyrics and chords", "Type the words, separate syllables with spaces or hyphens."),
        ]
        .iter()
        .map(|(t, b)| (t.to_string(), b.to_string()))
        .collect()
    }

    #[test]
    fn the_section_about_the_subject_is_picked() {
        let g = guide();
        assert_eq!(pick("How do I add a repeat?", &g, 2)[0], 0);
        assert_eq!(pick("How do I change the tuning?", &g, 2)[0], 1);
        assert_eq!(pick("Where are the lyrics?", &g, 2)[0], 3);
    }

    #[test]
    fn a_title_match_outweighs_a_passing_mention() {
        // "repeats" is in the text of Playing back too, but the title of the first section.
        assert_eq!(pick("repeats", &guide(), 2), vec![0, 2]);
    }

    #[test]
    fn nothing_matching_brings_nothing() {
        let g = guide();
        assert!(pick("What scale fits these notes?", &g, 4).is_empty());
        assert!(pick("how can you do that", &g, 4).is_empty());
        assert!(pick("", &g, 4).is_empty());
        assert_eq!(guide_context("What scale fits these notes?", &g), None);
    }

    #[test]
    fn at_most_the_asked_number_of_sections_are_sent() {
        let g = guide();
        assert_eq!(pick("track repeat metronome lyrics", &g, 2).len(), 2);
    }

    #[test]
    fn the_context_names_its_source_and_keeps_the_guide_order() {
        let g = guide();
        let c = guide_context("repeats and tuning", &g).unwrap();
        assert!(c.starts_with("From the GuitarCraft user guide"));
        let (a, b) = (c.find("## Bars, repeats").unwrap(), c.find("## Tracks and the mixer").unwrap());
        assert!(a < b, "sections are in the guide's order, not the order of the match");
    }

    #[test]
    fn plurals_and_endings_do_not_stop_a_match() {
        assert_eq!(stem("repeats"), stem("repeat"));
        assert_eq!(stem("Repeating"), stem("repeat"));
        assert_eq!(pick("repeating", &guide(), 1), vec![0]);
    }

    #[test]
    fn real_questions_find_their_section_in_the_real_guide() {
        let g = crate::app::tools::guide_sections();
        let top = |q: &str| g[pick(q, &g, 1).first().copied().unwrap_or_else(|| panic!("nothing matched: {q}"))].0.clone();
        assert_eq!(top("How do I add a repeat?"), "Bars, repeats and directions");
        assert_eq!(top("How do I add a new track?"), "Tracks and the mixer");
        assert_eq!(top("Where do I type the lyrics?"), "Lyrics and chords");
        assert_eq!(top("How do I split the score view?"), "Views");
        assert_eq!(top("How do I connect a MIDI keyboard?"), "MIDI");
        assert_eq!(top("How do I use the AI assistant?"), "The AI assistant");
        assert_eq!(top("How do I copy and paste bars?"), "Copy and paste");
    }

    /// Asks the Ollama on this computer a GuitarCraft question with the guide sections attached,
    /// through the real request path. Run by hand:
    /// `cargo test help_answers_live -- --ignored --nocapture` (set OLLAMA_MODEL to choose).
    #[test]
    #[ignore]
    fn help_answers_live() {
        let question = std::env::var("HELP_QUESTION").unwrap_or_else(|_| "How do I add a repeat?".into());
        let guide = guide_context(&question, &crate::app::tools::guide_sections()).expect("the guide should match");
        let a = crate::app::ai::Ask {
            provider: crate::app::ai::Provider::Ollama,
            ollama_url: String::new(),
            ollama_context: 16_384,
            ollama_keep: true,
            model: std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "qwen3:8b".into()),
            key: String::new(),
            system: crate::app::ai::system_prompt(),
            turns: vec![crate::app::ai::Turn { from_user: true, text: format!("{guide}\n\nQuestion: {question}") }],
        };
        let answer = crate::app::ai::ask_now(&a).expect("Ollama should answer");
        println!("QUESTION: {question}\nANSWER: {answer}");
        assert!(!answer.is_empty());
    }
}
