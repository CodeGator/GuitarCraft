//! The AI Assistant panel: a conversation with Ollama or Claude (through Claude Code or an API
//! key), docked like the other panels.
//! Nothing is sent until the person presses Return (Shift+Return adds a line; there is no Send
//! button). They choose whether the bars at the cursor (or the selected ones) go along as
//! ASCII tablature. The assistant can propose changes to the score as cards under its answer
//! (see `aiplan`); nothing changes until the person presses Apply.
//! See `docs/ai-design.md`.

use super::ai::{self, Ask, Pending, Provider, Turn};
use super::aiplan;
use std::collections::HashMap;
use super::App;
use crate::model::*;
use egui::{Frame, Margin, RichText, Ui};

/// The size of the text in the box the question is typed in (the usual size is about 13).
const INPUT_TEXT_SIZE: f32 = 15.0;

/// The most bars sent at once, so a question stays small enough for a modest model.
const MAX_BARS: usize = 16;

/// The panel's own state (the settings it uses are in Preferences > AI).
#[derive(Default)]
pub struct AiPanel {
    /// The conversation: the person's questions as typed, and the answers.
    pub turns: Vec<Turn>,
    /// The question being typed.
    pub input: String,
    /// The provider chosen when more than one is switched on.
    pub choice: Option<Provider>,
    /// The cards for the changes the assistant proposed, by answer and block.
    pub cards: HashMap<(usize, usize), Card>,
    /// The Ollama model last loaded in the background (its address, name and context size), and
    /// when, so it is loaded again only if something changed or it has had time to unload.
    warmed: Option<(String, std::time::Instant)>,
    /// A question that hasn't been answered yet.
    pending: Option<Pending>,
    /// Why the last question got no answer.
    pub error: Option<String>,
}

impl AiPanel {
    /// Takes the answer in when it has come. A question that fails goes back in the box to be
    /// tried again.
    fn poll(&mut self) {
        let Some(answer) = self.pending.as_ref().and_then(|p| p.poll()) else {
            return;
        };
        self.pending = None;
        match answer {
            Ok(text) => self.turns.push(Turn { from_user: false, text }),
            Err(e) => {
                self.take_back();
                self.error = Some(e);
            }
        }
    }

    /// Puts the last question back in the box (it wasn't answered).
    fn take_back(&mut self) {
        if self.turns.last().is_some_and(|t| t.from_user) {
            if let Some(t) = self.turns.pop() {
                self.input = t.text;
            }
        }
    }

    pub fn busy(&self) -> bool {
        self.pending.is_some()
    }
}

impl App {
    /// The providers that are switched on in Preferences > AI, each with its model.
    fn ai_providers(&self) -> Vec<(Provider, String)> {
        let p = &self.prefs;
        let mut out = vec![];
        if p.ollama_on {
            out.push((Provider::Ollama, p.ollama_model.clone()));
        }
        if p.claude_code_on {
            out.push((Provider::ClaudeCode, p.claude_code_model.clone()));
        }
        if p.claude_on {
            out.push((Provider::Claude, p.claude_model.clone()));
        }
        out
    }

    /// The bars that would go along: the selected ones, or the one at the cursor, and what they
    /// look like as text. None for a track without strings, which has no tablature.
    fn ai_bars(&self) -> Option<(usize, usize, String)> {
        let track = self.song.tracks.get(self.track)?;
        if !track.has_strings() || track.bars.is_empty() {
            return None;
        }
        let (a, b) = match self.selection() {
            Some((sa, sb)) => (sa.0, sb.0),
            None => (self.bar, self.bar),
        };
        let a = a.min(track.bars.len() - 1);
        let b = b.min(track.bars.len() - 1).min(a + MAX_BARS - 1);
        // A song of just these bars: no title, artist or file name goes with them.
        let mut s = Song::new_blank();
        s.tempo = self.song.tempo;
        s.master = self.song.master[a..=b].to_vec();
        s.tracks = vec![Track {
            name: track.name.clone(),
            instrument: track.instrument,
            tuning: track.tuning.clone(),
            capo: track.capo,
            bars: track.bars[a..=b].to_vec(),
            ..Track::default()
        }];
        let tab = crate::textmidi::export_ascii(&s, 0);
        let mb = &self.song.master[a];
        let head = format!(
            "Bars {} to {} of the track \"{}\" (key {}, {}/{}):",
            a + 1,
            b + 1,
            track.name,
            super::popups::key_label(mb.key, mb.key_minor),
            mb.num,
            mb.den
        );
        let facts = super::aifacts::passage_facts(&beat_notes(track, a, b));
        let text = if facts.is_empty() { format!("{head}\n\n{}", tab.trim()) } else { format!("{head}\n\n{}\n\n{facts}", tab.trim()) };
        Some((a, b, text))
    }

    /// The key, the notes used and the scales that fit best, counted over every bar of the
    /// current track, so a question about a few bars has the rest of the music for context. A
    /// few bars can use two notes, which fit dozens of scales. None for a track without strings.
    fn ai_track_summary(&self) -> Option<String> {
        let track = self.song.tracks.get(self.track)?;
        if !track.has_strings() || track.bars.is_empty() {
            return None;
        }
        let beats = beat_notes(track, 0, track.bars.len() - 1);
        let facts = super::aifacts::track_facts(&beats, track.bars.len());
        if facts.is_empty() {
            return None;
        }
        let key = self.song.master.first().map(|m| super::popups::key_label(m.key, m.key_minor)).unwrap_or_default();
        Some(format!("{facts}\nThe key signature at the start is {key}."))
    }

    /// What goes with a question besides the question: the program decides, so there is nothing
    /// to tick. An order to change the score ("Copy the intro to the end") needs only the names of
    /// the tracks and sections, so it gets the song outline and nothing else; the how-to text of
    /// the user guide would only talk the model into explaining instead of doing. Any other question
    /// gets the outline, a summary of the track (a few bars say little about the key) and the bars
    /// selected, or the one the cursor is on. None for a chord chart, which has no score.
    fn ai_context(&self, order: bool) -> Option<String> {
        if self.chart.is_some() {
            return None;
        }
        let mut parts: Vec<String> = vec![aiplan::outline(&self.song)];
        if !order {
            if let Some(summary) = self.ai_track_summary() {
                parts.push(summary);
            }
            if let Some(bars) = self.ai_bars() {
                parts.push(bars.2);
            }
        }
        Some(parts.join("\n\n"))
    }

    /// Loads the Ollama model in the background when the assistant is showing it, so the first
    /// question doesn't wait for the load. Once per model and context size, and again only after
    /// it has had time to unload. Nothing of the person's is sent.
    fn ai_warm(&self, ctx: &egui::Context, p: &mut AiPanel, current: Option<&(Provider, String)>) {
        let Some((Provider::Ollama, model)) = current else { return };
        if !self.prefs.ollama_warm || model.trim().is_empty() {
            return;
        }
        let key = format!("{}|{}|{}", self.prefs.ollama_url.trim(), model.trim(), self.prefs.ollama_context);
        let fresh = p.warmed.as_ref().is_some_and(|(k, at)| *k == key && at.elapsed() < ai::OLLAMA_REWARM);
        if !fresh {
            ai::warm_ollama(ctx, &self.prefs.ollama_url, model.trim(), self.prefs.ollama_context);
            p.warmed = Some((key, std::time::Instant::now()));
        }
    }

    /// Sends what is in the box.
    fn ai_send(&mut self, ctx: &egui::Context, panel: &mut AiPanel) {
        let question = panel.input.trim().to_string();
        if question.is_empty() || panel.busy() {
            return;
        }
        let providers = self.ai_providers();
        let Some((provider, model)) = providers.iter().find(|(p, _)| Some(*p) == panel.choice).or(providers.first()).cloned() else {
            panel.error = Some("No AI provider is switched on. Turn one on in Preferences > AI.".into());
            return;
        };
        // Claude Code has a default of its own; the others must be told.
        if model.trim().is_empty() && provider != Provider::ClaudeCode {
            panel.error = Some("Choose a model in Preferences > AI first.".into());
            return;
        }
        // What goes with this question and not with the ones before it (see `ai_context`): for an
        // order only the song outline, otherwise also the sections of the user guide that match the
        // question, the summary of the track and the bars selected.
        let order = aiplan::is_change_request(&question);
        let mut extra: Vec<String> = vec![];
        if !order && let Some(g) = super::aihelp::guide_context(&question, &super::tools::guide_sections()) {
            extra.push(g);
        }
        if let Some(context) = self.ai_context(order) {
            extra.push(context);
        }
        panel.turns.push(Turn { from_user: true, text: question.clone() });
        let mut turns = panel.turns.clone();
        if !extra.is_empty()
            && let Some(last) = turns.last_mut()
        {
            last.text = format!("{}\n\nQuestion: {question}\n\n{}", extra.join("\n\n"), aiplan::REMINDER);
        }
        let ask = Ask {
            provider,
            ollama_url: self.prefs.ollama_url.clone(),
            ollama_context: self.prefs.ollama_context,
            ollama_keep: self.prefs.ollama_warm,
            model,
            key: if provider == Provider::Claude { ai::key::get().unwrap_or_default() } else { String::new() },
            system: ai::system_prompt(),
            turns,
        };
        // A question keeps the model loaded too, so the timer for loading it again starts now.
        if let Some((_, at)) = panel.warmed.as_mut() {
            *at = std::time::Instant::now();
        }
        panel.pending = Some(ai::ask(ctx, ask));
        panel.input.clear();
        panel.error = None;
    }

    /// The panel's contents.
    pub(super) fn ai_panel(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        let mut p = std::mem::take(&mut self.ai);
        p.poll();
        let providers = self.ai_providers();
        let current = providers.iter().find(|(q, _)| Some(*q) == p.choice).or(providers.first()).cloned();
        self.ai_warm(&ctx, &mut p, current.as_ref());

        // "New chat" lives at the top, where a small panel can't push it out of view.
        let can_clear = !p.turns.is_empty() && !p.busy();
        let mut new_chat = false;
        egui::Panel::top("ai-top").frame(Frame::new().inner_margin(Margin::symmetric(10, 6))).resizable(false).show(ui, |ui| {
            match &current {
                None => {
                    ui.label(RichText::new("No AI provider is switched on.").color(egui::Color32::from_rgb(235, 170, 90)));
                    ui.horizontal_wrapped(|ui| {
                        // A model that is set up but switched off: one click to switch it on.
                        let model = self.prefs.ollama_model.trim().to_string();
                        if !model.is_empty()
                            && ui.button("Switch on Ollama").on_hover_text("Use the model you chose in AI Preferences").clicked()
                        {
                            self.prefs.ollama_on = true;
                            self.prefs.save();
                        }
                        if ui.button("AI Preferences").clicked() {
                            self.open_prefs_ai();
                        }
                        new_chat |= ui.add_enabled(can_clear, egui::Button::new("New chat")).clicked();
                    });
                }
                Some(_) => {
                    // Nothing about the provider or model here: that is in AI Preferences.
                    new_chat |= ui.add_enabled(can_clear, egui::Button::new("New chat")).clicked();
                }
            }
        });

        if new_chat {
            p.turns.clear();
            p.cards.clear();
            p.error = None;
        }

        egui::Panel::bottom("ai-bottom").frame(Frame::new().inner_margin(Margin::same(10))).resizable(false).show(ui, |ui| {
            if let Some(e) = &p.error {
                ui.label(RichText::new(e).color(egui::Color32::from_rgb(235, 120, 110)));
                ui.add_space(4.0);
            }
            let id = ui.make_persistent_id("ai-input");
            // Return sends; Shift+Return adds a line. Taken before the box sees it.
            let enter = ui.memory(|m| m.has_focus(id))
                && ui.input_mut(|i| {
                    let m = i.modifiers;
                    !m.shift && i.consume_key(m, egui::Key::Enter)
                });
            ui.add(
                egui::TextEdit::multiline(&mut p.input)
                    .id(id)
                    .font(egui::FontId::proportional(INPUT_TEXT_SIZE))
                    .desired_rows(4)
                    .desired_width(f32::INFINITY)
                    .hint_text(
                        // Clearly greyer than typed text, so the two can't be mixed up.
                        RichText::new("Ask about the music, or about GuitarCraft. Press Return to send, Shift+Return for a new line.")
                            .color(egui::Color32::from_gray(110)),
                    ),
            );
            ui.horizontal(|ui| {
                if p.busy() {
                    ui.spinner();
                    ui.label(RichText::new("Waiting for the answer\u{2026}").weak());
                    if ui.button("Stop").clicked() {
                        p.pending = None;
                        p.take_back();
                    }
                } else if enter && !p.input.trim().is_empty() {
                    self.ai_send(&ctx, &mut p);
                }
            });
        });

        let mut retry = false;
        egui::CentralPanel::default().frame(Frame::new().inner_margin(Margin::symmetric(10, 4))).show(ui, |ui| {
            egui::ScrollArea::vertical().id_salt("ai-history").stick_to_bottom(true).auto_shrink([false, false]).show(ui, |ui| {
                if p.turns.is_empty() {
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(
                            "Ask a question about your music or about GuitarCraft. Nothing is sent until you press Return. \
                             It can also change your score: it proposes the change as a card, and nothing happens until you press Apply.\n\n\
                             Try: \u{201C}What key is this song in?\u{201D}, \
                             \u{201C}What scale fits these bars?\u{201D} (select some bars first), \
                             \u{201C}How do I add a repeat?\u{201D}, \
                             or \u{201C}Copy the intro and paste it after the last bar\u{201D}",
                        )
                        .weak(),
                    );
                }
                let who = current.as_ref().map(|c| c.0.name()).unwrap_or("Assistant");
                let turns = p.turns.clone();
                for (ti, t) in turns.iter().enumerate() {
                    ui.add_space(6.0);
                    let fill = if t.from_user { egui::Color32::from_rgb(44, 48, 56) } else { egui::Color32::from_rgb(34, 36, 42) };
                    Frame::new().fill(fill).corner_radius(6.0).inner_margin(Margin::same(8)).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(RichText::new(if t.from_user { "You" } else { who }).small().strong());
                        if t.from_user {
                            ui.add(egui::Label::new(&t.text).selectable(true));
                        } else {
                            // The answer in pieces: words, and a card for each proposed change.
                            for (bi, block) in aiplan::split_blocks(&t.text).into_iter().enumerate() {
                                match block {
                                    aiplan::Block::Prose(text) => {
                                        ui.add(egui::Label::new(text).selectable(true));
                                    }
                                    aiplan::Block::Code { lang, text } if aiplan::is_plan(lang, text) => {
                                        self.plan_card(ui, &mut p, (ti, bi), text);
                                    }
                                    aiplan::Block::Code { text, .. } => {
                                        ui.add(egui::Label::new(RichText::new(text).monospace().small()).selectable(true));
                                    }
                                }
                            }
                            // An order that got an explanation instead of a plan: the answer can
                            // differ each time, so offer to ask again.
                            let has_plan = aiplan::split_blocks(&t.text).iter().any(|b| matches!(b, aiplan::Block::Code { lang, text } if aiplan::is_plan(lang, text)));
                            if ti + 1 == turns.len() && ti >= 1 && !has_plan && turns[ti - 1].from_user && aiplan::is_change_request(&turns[ti - 1].text) {
                                ui.label(RichText::new("No change was proposed.").small().weak());
                                if ui.small_button("Ask again").on_hover_text("Ask the same thing again; the answer can differ each time").clicked() {
                                    retry = true;
                                }
                            }
                            if ui.small_button("Copy").clicked() {
                                ui.ctx().copy_text(t.text.clone());
                            }
                        }
                    });
                }
            });
        });
        // Ask again: take back the order and its answer and send the order once more.
        if retry && p.turns.len() >= 2 && !p.busy() {
            let order = p.turns[p.turns.len() - 2].text.clone();
            let keep = p.turns.len() - 2;
            p.turns.truncate(keep);
            p.cards.retain(|k, _| k.0 < keep);
            p.input = order;
            self.ai_send(&ctx, &mut p);
        }
        self.ai = p;
    }
}

/// A change the assistant proposed, as the card under its answer shows it.
pub struct Card {
    steps: Vec<aiplan::Step>,
    /// What the steps would do to the song as it was when the card first showed, or why they can't.
    preview: Result<aiplan::Outcome, String>,
    /// What the person did with it.
    done: Option<Done>,
}

enum Done {
    Applied(String),
    Dismissed,
    Failed(String),
}

impl Card {
    fn new(text: &str, song: &Song) -> Card {
        match aiplan::parse(text) {
            Err(e) => Card { steps: vec![], preview: Err(e), done: None },
            Ok(steps) => {
                let preview = aiplan::run(&mut song.clone(), &steps);
                Card { steps, preview, done: None }
            }
        }
    }
}

impl App {
    /// The card for a plan in the assistant's answer: what it would do, and Apply and Dismiss.
    /// Nothing changes until Apply, which is one edit that Undo takes back.
    fn plan_card(&mut self, ui: &mut Ui, p: &mut AiPanel, key: (usize, usize), text: &str) {
        let card = p.cards.entry(key).or_insert_with(|| Card::new(text, &self.song));
        let mut apply = false;
        let mut dismiss = false;
        Frame::new()
            .fill(egui::Color32::from_rgb(28, 40, 58))
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(70, 110, 170)))
            .corner_radius(6.0)
            .inner_margin(Margin::same(8))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new("Proposed change").strong());
                match &card.preview {
                    Err(e) => {
                        ui.label(RichText::new(format!("This can't be applied. {e}")).color(egui::Color32::from_rgb(235, 120, 110)));
                    }
                    Ok(o) => {
                        for (i, line) in o.lines.iter().enumerate() {
                            ui.label(format!("{}. {line}", i + 1));
                        }
                        if o.bars_after != o.bars_before {
                            ui.label(RichText::new(format!("The song goes from {} to {} bars.", o.bars_before, o.bars_after)).weak());
                        }
                        for n in &o.notes {
                            ui.label(RichText::new(n).color(egui::Color32::from_rgb(235, 170, 90)));
                        }
                    }
                }
                match &card.done {
                    Some(Done::Applied(s)) => {
                        ui.label(RichText::new(format!("Applied. {s} Edit > Undo takes it back.")).color(egui::Color32::from_rgb(120, 200, 130)));
                    }
                    Some(Done::Dismissed) => {
                        ui.label(RichText::new("Dismissed.").weak());
                    }
                    Some(Done::Failed(e)) => {
                        ui.label(RichText::new(format!("Not applied. {e}")).color(egui::Color32::from_rgb(235, 120, 110)));
                    }
                    None => {
                        ui.horizontal(|ui| {
                            if ui.add_enabled(card.preview.is_ok(), egui::Button::new("Apply")).on_hover_text("Make this change to the score. One Undo takes it back.").clicked() {
                                apply = true;
                            }
                            if ui.button("Dismiss").clicked() {
                                dismiss = true;
                            }
                        });
                    }
                }
            });
        if dismiss {
            card.done = Some(Done::Dismissed);
        }
        if apply {
            let steps = card.steps.clone();
            card.done = Some(match self.ai_apply_plan(&steps) {
                Ok(summary) => Done::Applied(summary),
                Err(e) => Done::Failed(e),
            });
        }
    }

    /// Makes the proposed change to the score as one edit. The steps are checked again against
    /// the song as it is now (it may have changed since the card showed), on a copy, so a plan that
    /// no longer fits changes nothing.
    fn ai_apply_plan(&mut self, steps: &[aiplan::Step]) -> Result<String, String> {
        if self.chart.is_some() {
            return Err("Open a score first; a chord chart can't take these changes.".into());
        }
        let mut trial = self.song.clone();
        let outcome = aiplan::run(&mut trial, steps)?;
        self.snapshot();
        self.song = trial;
        self.sel_anchor = None;
        self.clamp_cursor();
        self.total_secs = self.song_seconds();
        self.status = format!("Applied the assistant's change ({} step(s)).", outcome.lines.len());
        Ok(if outcome.bars_after != outcome.bars_before {
            format!("The song now has {} bars.", outcome.bars_after)
        } else {
            String::new()
        })
    }
}

/// The notes on each beat of bars `from` to `to` of a track (voice 1, which is what the tablature
/// shows; dead notes have no pitch), with the bar and beat counted from 1.
fn beat_notes(track: &Track, from: usize, to: usize) -> Vec<super::aifacts::BeatNotes> {
    track.bars[from..=to]
        .iter()
        .enumerate()
        .flat_map(|(i, bar)| bar.beats.iter().enumerate().map(move |(j, beat)| (from + i + 1, j + 1, beat)))
        .map(|(bar, beat, bt)| super::aifacts::BeatNotes {
            bar,
            beat,
            pitches: bt.notes.iter().filter(|n| !n.dead).map(|n| track.pitch(n)).collect(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds the facts for bars 1 and 2 of the sample song and its whole-track summary, prints
    /// them, and asks the Ollama on this computer "What scale fits these bars?" with them.
    /// Run by hand: `cargo test summary_live -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn summary_live() {
        let song = crate::fileio::load_song(std::path::Path::new("samples/Morning Walk.gp")).expect("sample song");
        let track = song.tracks.iter().find(|t| t.has_strings()).expect("a string track");
        let bars = super::super::aifacts::passage_facts(&beat_notes(track, 0, 1));
        let whole = super::super::aifacts::track_facts(&beat_notes(track, 0, track.bars.len() - 1), track.bars.len());
        println!("BARS 1-2:\n{bars}\n\nWHOLE TRACK:\n{whole}\n");
        let question = "What scale fits these bars?";
        let a = Ask {
            provider: Provider::Ollama,
            ollama_url: String::new(),
            ollama_context: 16_384,
            ollama_keep: true,
            model: std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "qwen3:8b".into()),
            key: String::new(),
            system: ai::system_prompt(),
            turns: vec![Turn { from_user: true, text: format!("Bars 1 to 2 of the track:\n{bars}\n\n{whole}\n\nQuestion: {question}") }],
        };
        let answer = ai::ask_now(&a).expect("Ollama should answer");
        println!("ANSWER: {answer}");
    }

    /// Asks the Ollama on this computer to change the sample song, through the real prompt, and
    /// shows what the card would say. Run by hand:
    /// `cargo test plan_live -- --ignored --nocapture` (PLAN_QUESTION and OLLAMA_MODEL choose).
    #[test]
    #[ignore]
    fn plan_live() {
        let song = crate::fileio::load_song(std::path::Path::new("samples/Morning Walk.gp")).expect("sample song");
        let question = std::env::var("PLAN_QUESTION").unwrap_or_else(|_| "Copy the intro and paste it past the last bar of the song".into());
        let outline = aiplan::outline(&song);
        // What the panel sends with a question: the guide sections that match (unless
        // PLAN_NO_GUIDE is set), the outline, and the summary of the track.
        let track = song.tracks.iter().find(|t| t.has_strings()).expect("a string track");
        let summary = crate::app::aifacts::track_facts(&beat_notes(track, 0, track.bars.len() - 1), track.bars.len());
        let mut extra: Vec<String> = vec![];
        // PLAN_OLD=1 sends what the panel sent before the reminder: the guide for every question,
        // and nothing after it.
        let old = std::env::var("PLAN_OLD").is_ok();
        if std::env::var("PLAN_NO_GUIDE").is_err()
            && (old || !aiplan::is_change_request(&question))
            && let Some(g) = crate::app::aihelp::guide_context(&question, &crate::app::tools::guide_sections())
        {
            extra.push(g);
        }
        extra.push(format!("{outline}\n\n{summary}"));
        let reminder = if old { String::new() } else { format!("\n\n{}", aiplan::REMINDER) };
        let sent = format!("{}\n\nQuestion: {question}{reminder}", extra.join("\n\n"));
        println!("SENT {} chars; guide attached: {}", sent.len(), sent.contains("From the GuitarCraft user guide"));
        println!("OUTLINE: {outline}\nQUESTION: {question}\n");
        let a = Ask {
            provider: Provider::Ollama,
            ollama_url: String::new(),
            ollama_context: std::env::var("PLAN_CTX").ok().and_then(|v| v.parse().ok()).unwrap_or(16_384),
            ollama_keep: true,
            model: std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "qwen3:8b".into()),
            key: String::new(),
            system: ai::system_prompt(),
            turns: vec![Turn { from_user: true, text: sent }],
        };
        let answer = ai::ask_now(&a).expect("Ollama should answer");
        println!("ANSWER:\n{answer}\n");
        for block in aiplan::split_blocks(&answer) {
            if let aiplan::Block::Code { lang, text } = block
                && aiplan::is_plan(lang, text)
            {
                let card = Card::new(text, &song);
                match card.preview {
                    Ok(o) => println!("CARD:\n{}\n{} -> {} bars; notes: {:?}", o.lines.join("\n"), o.bars_before, o.bars_after, o.notes),
                    Err(e) => println!("CARD ERROR: {e}"),
                }
            }
        }
    }
}
