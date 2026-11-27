//! Suggest Chords: asks Claude for chords to put on the words of one section of a chord chart.
//! The window (the Section panel's button, or Section > Suggest Chords) shows the section as it
//! is beside the suggestion, and nothing changes until the person presses Use These; one Undo
//! takes it back.
//!
//! The words are never taken from the reply: [`merge`] checks that it kept them, then puts its
//! chords on the person's own lines. Only Claude is asked (through Claude Code or an API key):
//! the small local model tried on 2026-11-27 often answered nothing or garbled the words.

use super::ai::{self, Ask, Pending, Provider, Turn};
use super::stylesheet::BLUE;
use super::App;
use crate::chart::{self, Line};
use egui::{Align, Align2, Color32, Layout, RichText, Ui, Vec2};

/// What Claude is told it is doing.
const SYSTEM: &str = "You suggest guitar chords for a chord chart. Put each chord in square brackets immediately before \
the syllable where it starts, for example: [E]Let my heart be a [B]temple. Reply with only the lyrics with your chords \
added: keep every word and line break exactly as given, with no commentary and no code fence.";

/// The size of the words and chords in the window: large enough to read easily.
const TEXT_SIZE: f32 = 14.0;
const WINDOW_W: f32 = 760.0;
const LIST_H: f32 = 320.0;

/// The window while it is open.
pub struct SuggestDlg {
    /// The section it works on (its id: sections move about when the map changes).
    pub section: u32,
    /// What the person asks for, such as "simple folk". May be empty.
    pub style: String,
    /// A question on its way. Dropping it (Cancel, or closing) stops it.
    pending: Option<Pending>,
    /// The last answer, as it came.
    pub reply: Option<String>,
    /// Why there is no answer.
    error: Option<String>,
    /// Use These was pressed.
    pub accept: bool,
}

impl SuggestDlg {
    fn new(section: u32) -> SuggestDlg {
        SuggestDlg { section, style: String::new(), pending: None, reply: None, error: None, accept: false }
    }
}

/// The words of a line that can take chords: a lyric line with something on it besides chords.
fn lyric_of(line: &str) -> Option<(String, Vec<(usize, String)>)> {
    match chart::parse(line).into_iter().next() {
        Some(Line::Lyric { text, chords }) if !text.trim().is_empty() => Some((text, chords)),
        _ => None,
    }
}

/// A section's words with no chords, a line for each lyric line: what is sent. Empty when the
/// section has none (it is empty, or only lines of bars).
pub fn words_of(text: &str) -> String {
    text.split('\n').filter_map(lyric_of).map(|(words, _)| words.trim().to_string()).collect::<Vec<_>>().join("\n")
}

/// A character as it is compared: a model may curl a straight quote.
fn plain(c: char) -> char {
    match c {
        '\u{2018}' | '\u{2019}' => '\'',
        '\u{201C}' | '\u{201D}' => '"',
        c => c,
    }
}

/// The section's text with the reply's chords on it, in place of the chords it had. Lines of
/// bars, blank lines and lines of chords alone stay as they are, and every word stays exactly as
/// the person wrote it. An error says why the reply can't be used.
pub fn merge(now: &str, reply: &str) -> Result<String, String> {
    let mut theirs = reply.split('\n').filter(|l| !l.trim_start().starts_with("```")).filter_map(lyric_of);
    let mut out = vec![];
    let mut placed = 0;
    for line in now.split('\n') {
        let Some((words, _)) = lyric_of(line) else {
            out.push(line.to_string());
            continue;
        };
        let (new_words, chords) = theirs.next().ok_or("The reply left out some of your lines, so it wasn't used.")?;
        let letters = |s: &str| s.chars().filter(|c| !c.is_whitespace()).map(plain).collect::<String>();
        if letters(&words) != letters(&new_words) {
            return Err("The reply changed your words, so it wasn't used.".into());
        }
        // A chord belongs before the same letter in the person's line, however either is spaced.
        let at: Vec<usize> = words.char_indices().filter(|(_, c)| !c.is_whitespace()).map(|(i, _)| i).collect();
        let mut marks: Vec<(usize, bool, String)> = vec![];
        for (pos, name) in chords {
            if chart::split_chord(&name).is_none() {
                return Err(format!("The reply has \u{201C}{name}\u{201D}, which isn't a chord name, so it wasn't used."));
            }
            let before: Vec<char> = new_words.chars().take(pos).collect();
            let letter = before.iter().filter(|c| !c.is_whitespace()).count();
            // After the last word: at the end of the line, apart from it if the reply had it so.
            let apart = letter >= at.len() && before.last().is_some_and(|c| c.is_whitespace());
            marks.push((at.get(letter).copied().unwrap_or(words.len()), apart, name));
        }
        placed += marks.len();
        let mut built = String::new();
        let mut from = 0;
        for (i, apart, name) in marks {
            built.push_str(&words[from..i]);
            if apart && !built.ends_with(' ') {
                built.push(' ');
            }
            built.push_str(&format!("[{name}]"));
            from = i;
        }
        built.push_str(&words[from..]);
        out.push(built);
    }
    if theirs.next().is_some() {
        return Err("The reply added lines of its own, so it wasn't used.".into());
    }
    if placed == 0 {
        return Err("The reply had no chords.".into());
    }
    Ok(out.join("\n"))
}

/// A section's text as it is read: each line's chords on a line above its words (None when it has
/// no chords), placed by character, for a fixed-width font.
pub fn chords_above(text: &str) -> Vec<(Option<String>, String)> {
    chart::parse(text)
        .into_iter()
        .map(|line| match line {
            Line::Bars(bars) => (None, bars),
            Line::Lyric { text, chords } => {
                let mut above = String::new();
                for (pos, name) in chords {
                    let used = above.chars().count();
                    // Over its syllable, but never run into the chord before.
                    let gap = if used == 0 { pos } else { pos.saturating_sub(used).max(1) };
                    above.extend(std::iter::repeat_n(' ', gap));
                    above.push_str(&name);
                }
                ((!above.is_empty()).then_some(above), text)
            }
        })
        .collect()
}

/// The question: the key and the style when there are any, then the words.
fn question(key: &str, style: &str, words: &str) -> String {
    let mut head = vec![];
    if !key.trim().is_empty() {
        head.push(format!("Key: {}.", key.trim()));
    }
    if !style.trim().is_empty() {
        head.push(format!("Style: {}.", style.trim()));
    }
    if head.is_empty() { words.to_string() } else { format!("{}\n\n{words}", head.join(" ")) }
}

/// One side of the comparison.
fn side(ui: &mut Ui, id: &str, title: &str, text: Option<&str>) {
    ui.label(RichText::new(title).strong().size(TEXT_SIZE));
    egui::Frame::new().fill(Color32::from_rgb(30, 32, 38)).corner_radius(4.0).inner_margin(egui::Margin::same(8)).show(ui, |ui| {
        egui::ScrollArea::both().id_salt(id).max_height(LIST_H).min_scrolled_height(LIST_H).auto_shrink([false, false]).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            let Some(text) = text else { return };
            let line = |ui: &mut Ui, t: &str, color: Color32| {
                ui.add(egui::Label::new(RichText::new(t).monospace().size(TEXT_SIZE).color(color)).extend());
            };
            for (above, words) in chords_above(text) {
                if let Some(above) = above {
                    line(ui, &above, Color32::from_rgb(130, 180, 255));
                }
                line(ui, if words.is_empty() { " " } else { &words }, Color32::from_gray(225));
            }
        });
    });
}

impl App {
    /// The provider asked for chords: the first Claude one switched on, with its model.
    pub(super) fn chord_provider(&self) -> Option<(Provider, String)> {
        self.ai_providers().into_iter().find(|p| p.0 != Provider::Ollama)
    }

    /// Why Suggest Chords can't be used on the selected section, or None when it can.
    pub(super) fn suggest_blocked(&self) -> Option<&'static str> {
        let st = self.chart.as_ref()?;
        if self.chord_provider().is_none() {
            return Some("Switch on Claude Code (or Claude) in Preferences > AI to use this.");
        }
        let has_words = st.doc.sections.get(st.sel).is_some_and(|s| !words_of(&s.text).is_empty());
        (!has_words).then_some("This section has no words to put chords on.")
    }

    /// Opens the window for the selected section.
    pub(super) fn open_suggest_chords(&mut self) {
        if self.suggest_blocked().is_some() {
            return;
        }
        if let Some(st) = &mut self.chart
            && let Some(s) = st.doc.sections.get(st.sel)
        {
            st.suggest = Some(SuggestDlg::new(s.id));
        }
    }

    /// Sends the words to Claude.
    fn ask_for_chords(&self, ctx: &egui::Context, key: &str, style: &str, words: &str) -> Result<Pending, String> {
        let (provider, model) = self.chord_provider().ok_or("Switch on Claude Code (or Claude) in Preferences > AI.")?;
        if model.trim().is_empty() && provider == Provider::Claude {
            return Err("Choose a Claude model in Preferences > AI first.".into());
        }
        Ok(ai::ask(
            ctx,
            Ask {
                provider,
                ollama_url: String::new(),
                ollama_context: ai::OLLAMA_CONTEXT,
                ollama_keep: false,
                model,
                key: if provider == Provider::Claude { ai::key::get().unwrap_or_default() } else { String::new() },
                system: SYSTEM.into(),
                turns: vec![Turn { from_user: true, text: question(key, style, words) }],
            },
        ))
    }

    pub(super) fn suggest_window(&mut self, ctx: &egui::Context) {
        let Some(st) = &mut self.chart else { return };
        let Some(mut dlg) = st.suggest.take() else { return };
        // The section may have gone (deleted, or undone) while the window was open.
        let Some(section) = st.doc.section(dlg.section) else { return };
        let (name, now, key) = (section.name.clone(), section.text.clone(), st.doc.key.clone());
        if let Some(answer) = dlg.pending.as_ref().and_then(|p| p.poll()) {
            dlg.pending = None;
            match answer {
                Ok(text) => dlg.reply = Some(text),
                Err(e) => dlg.error = Some(e),
            }
        }
        // Worked out again each frame, so it follows any typing in the section.
        let merged = dlg.reply.as_deref().map(|r| merge(&now, r));
        let words = words_of(&now);
        let (mut open, mut ask, mut cancel) = (true, false, false);
        egui::Window::new(format!("Suggest Chords: {name}"))
            .id(egui::Id::new("chart-suggest"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.set_width(WINDOW_W);
                ui.horizontal(|ui| {
                    let key = if key.trim().is_empty() { "not set" } else { key.trim() };
                    ui.label(RichText::new(format!("Key: {key}")).size(TEXT_SIZE));
                    ui.add_space(12.0);
                    ui.label(RichText::new("Style:").size(TEXT_SIZE));
                    ui.add(egui::TextEdit::singleline(&mut dlg.style).hint_text("optional, such as simple folk or jazzy").desired_width(360.0));
                    let label = if dlg.reply.is_some() { "Try Again" } else { "Suggest" };
                    ask = ui.add_enabled(dlg.pending.is_none() && !words.is_empty(), egui::Button::new(label)).clicked();
                });
                ui.add_space(8.0);
                ui.columns(2, |cols| {
                    side(&mut cols[0], "suggest-now", "Now", Some(&now));
                    side(&mut cols[1], "suggest-new", "Suggested", merged.as_ref().and_then(|m| m.as_ref().ok()).map(String::as_str));
                });
                ui.add_space(6.0);
                let problem = dlg.error.as_deref().or(merged.as_ref().and_then(|m| m.as_ref().err()).map(String::as_str));
                if dlg.pending.is_some() {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(RichText::new("Asking Claude\u{2026}").weak());
                    });
                } else if let Some(e) = problem {
                    ui.label(RichText::new(e).color(Color32::from_rgb(235, 120, 110)));
                }
                ui.add_space(4.0);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    cancel = ui.button("Cancel").clicked();
                    let ready = dlg.pending.is_none() && matches!(merged, Some(Ok(_)));
                    if ui.add_enabled(ready, egui::Button::new(RichText::new("Use These").color(Color32::WHITE)).fill(BLUE)).clicked() {
                        dlg.accept = true;
                    }
                });
            });
        if dlg.accept {
            // The change to the chart becomes one undo step (see `chart_screen`).
            if let Some(Ok(text)) = merged
                && let Some(st) = &mut self.chart
                && let Some(s) = st.doc.sections.iter_mut().find(|s| s.id == dlg.section)
            {
                s.text = text;
                self.status = "Used the suggested chords. Edit > Undo takes them back.".into();
            }
            return;
        }
        if !open || cancel || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            return;
        }
        if ask {
            dlg.reply = None;
            dlg.error = None;
            match self.ask_for_chords(ctx, &key, &dlg.style, &words) {
                Ok(p) => dlg.pending = Some(p),
                Err(e) => dlg.error = Some(e),
            }
        }
        if let Some(st) = &mut self.chart {
            st.suggest = Some(dlg);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VERSE: &str = "[E]Out along the river road\n[B]Where the morning light is slow";

    #[test]
    fn the_words_are_sent_without_chords_or_bars() {
        assert_eq!(words_of(VERSE), "Out along the river road\nWhere the morning light is slow");
        assert_eq!(words_of("| E /// | B //// |\n\n[E] [A]\n  [C]And [G]so"), "And so");
        assert_eq!(words_of("| E /// | B //// |"), "");
        assert_eq!(words_of(""), "");
    }

    #[test]
    fn the_new_chords_go_on_the_persons_own_words() {
        let reply = "[A]Out along the [D]river road\nWhere the morn[E]ing light is slow";
        assert_eq!(merge(VERSE, reply), Ok(reply.into()));
        // Bar lines, blank lines and chords on their own stay; so does the spacing of the words.
        let now = "| E /// |\n\n  Out  along\n[E] [A]\nthe river";
        assert_eq!(merge(now, "[A]Out along\nthe [D]river"), Ok("| E /// |\n\n  [A]Out  along\n[E] [A]\nthe [D]river".into()));
        // A reply spaced its own way, in a code fence, with curled quotes.
        assert_eq!(merge("I don't know", "```\n[C] I don\u{2019}t [G] know\n```"), Ok("[C]I don't [G]know".into()));
        // After the last word, and two chords inside a word.
        assert_eq!(merge("walking on and on", "walk[A]i[B]ng on and on [E]"), Ok("walk[A]i[B]ng on and on [E]".into()));
        assert_eq!(merge("on and on", "on and on[E]"), Ok("on and on[E]".into()));
        // Letters outside ASCII are counted as one each.
        assert_eq!(merge("Caf\u{e9} \u{e0} midi", "Caf\u{e9} [Am]\u{e0} [G]midi"), Ok("Caf\u{e9} [Am]\u{e0} [G]midi".into()));
    }

    #[test]
    fn a_reply_that_changes_the_words_is_not_used() {
        let refused = |reply: &str| merge(VERSE, reply).unwrap_err();
        assert!(refused("[E]Out along the river path\n[B]Where the morning light is slow").contains("changed your words"));
        assert!(refused("[E]out along the river road\n[B]Where the morning light is slow").contains("changed your words"));
        assert!(refused("[E]Out along the river road").contains("left out"));
        assert!(refused("Here you go:\n[E]Out along the river road\n[B]Where the morning light is slow").contains("changed your words"));
        assert!(refused("[E]Out along the river road\n[B]Where the morning light is slow\nEnjoy!").contains("added lines"));
        assert!(refused("Out along the river road\nWhere the morning light is slow").contains("no chords"));
        assert!(refused("[Verse]Out along the river road\n[B]Where the morning light is slow").contains("isn't a chord name"));
    }

    #[test]
    fn chords_are_shown_above_their_words() {
        let shown = chords_above("[E]Out al[B]ong\n[C#m7][A]Go\nplain\n| E /// |");
        assert_eq!(shown[0], (Some("E     B".into()), "Out along".into()));
        assert_eq!(shown[1], (Some("C#m7 A".into()), "Go".into()), "never run together");
        assert_eq!(shown[2], (None, "plain".into()));
        assert_eq!(shown[3], (None, "| E /// |".into()));
    }

    #[test]
    fn the_question_names_the_key_and_style_only_when_there_are_any() {
        assert_eq!(question("E", "simple folk", "la"), "Key: E. Style: simple folk.\n\nla");
        assert_eq!(question(" ", "", "la"), "la");
        assert_eq!(question("Am", "  ", "la"), "Key: Am.\n\nla");
    }

    /// Asks Claude for chords through the installed, signed-in Claude Code and puts them on a
    /// verse. Uses a little of the plan's allowance. Run by hand:
    /// `cargo test --release suggest_chords_live -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn suggest_chords_live() {
        let turns = vec![Turn { from_user: true, text: question("E", "simple folk", &words_of(VERSE)) }];
        let reply = super::super::aicode::ask(SYSTEM, super::super::aicode::MODEL, &turns, std::time::Duration::from_secs(120), &Default::default()).expect("Claude should answer");
        println!("REPLY:\n{reply}");
        let merged = merge(VERSE, &reply).expect("usable chords");
        println!("MERGED:\n{merged}");
        assert_eq!(words_of(&merged), words_of(VERSE));
    }
}
