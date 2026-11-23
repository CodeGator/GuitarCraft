//! The AI Assistant panel: a conversation with Ollama or Claude, docked like the other panels.
//! Nothing is sent until the person presses Return (Shift+Return adds a line; there is no Send
//! button). They choose whether the bars at the cursor (or the selected ones) go along as
//! ASCII tablature. The assistant only answers in words: it never changes the score.
//! See `docs/ai-design.md`.

use super::ai::{self, Ask, Pending, Provider, Turn};
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
    /// Send the bars at the cursor along with the question.
    pub with_bars: bool,
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
        // What the program works out from the notes (voice 1, which is what the tablature shows).
        let beats: Vec<super::aifacts::BeatNotes> = track.bars[a..=b]
            .iter()
            .enumerate()
            .flat_map(|(i, bar)| {
                bar.beats.iter().enumerate().map(move |(j, beat)| (a + i + 1, j + 1, beat))
            })
            .map(|(bar, beat, bt)| super::aifacts::BeatNotes {
                bar,
                beat,
                pitches: bt.notes.iter().filter(|n| !n.dead).map(|n| track.pitch(n)).collect(),
            })
            .collect();
        let facts = super::aifacts::passage_facts(&beats);
        let text = if facts.is_empty() { format!("{head}\n\n{}", tab.trim()) } else { format!("{head}\n\n{}\n\n{facts}", tab.trim()) };
        Some((a, b, text))
    }

    /// Where a question to this provider goes, in words.
    fn ai_destination(&self, p: Provider) -> String {
        match p {
            Provider::Claude => "Claude, over the internet (api.anthropic.com)".into(),
            Provider::Ollama => {
                let url = self.prefs.ollama_url.trim();
                format!("Ollama at {}", if url.is_empty() { ai::OLLAMA_URL } else { url })
            }
        }
    }

    /// The text that goes with the question this time, if the person asked for the bars.
    fn ai_context(&self, panel: &AiPanel) -> Option<String> {
        if panel.with_bars { self.ai_bars().map(|b| b.2) } else { None }
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
        if model.trim().is_empty() {
            panel.error = Some("Choose a model in Preferences > AI first.".into());
            return;
        }
        let context = self.ai_context(panel);
        panel.turns.push(Turn { from_user: true, text: question.clone() });
        // The bars go with this question only, not with the ones before it.
        let mut turns = panel.turns.clone();
        if let (Some(ctx_text), Some(last)) = (context, turns.last_mut()) {
            last.text = format!("{ctx_text}\n\nQuestion: {question}");
        }
        let ask = Ask {
            provider,
            ollama_url: self.prefs.ollama_url.clone(),
            ollama_context: self.prefs.ollama_context,
            ollama_keep: self.prefs.ollama_warm,
            model,
            key: if provider == Provider::Claude { ai::key::get().unwrap_or_default() } else { String::new() },
            system: ai::SYSTEM.to_string(),
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
        egui::Panel::top("ai-top").frame(Frame::new().inner_margin(Margin::symmetric(10, 8))).resizable(false).show(ui, |ui| {
            match &current {
                None => {
                    ui.label(RichText::new("No AI provider is switched on.").color(egui::Color32::from_rgb(235, 170, 90)));
                    ui.horizontal_wrapped(|ui| {
                        if ui.button("Open Preferences > AI").clicked() {
                            self.open_prefs_ai();
                        }
                        new_chat |= ui.add_enabled(can_clear, egui::Button::new("New chat")).clicked();
                    });
                }
                Some((now, model)) => {
                    ui.horizontal_wrapped(|ui| {
                        ui.label("Ask using:");
                        if providers.len() > 1 {
                            egui::ComboBox::from_id_salt("ai-provider")
                                .selected_text(now.name())
                                .show_ui(ui, |ui| {
                                    for (q, _) in &providers {
                                        if ui.selectable_label(q == now, q.name()).clicked() {
                                            p.choice = Some(*q);
                                        }
                                    }
                                });
                        } else {
                            ui.label(RichText::new(now.name()).strong());
                        }
                        if !model.trim().is_empty() {
                            ui.label(RichText::new(model).weak());
                        }
                        new_chat |= ui.add_enabled(can_clear, egui::Button::new("New chat")).clicked();
                    });
                    ui.label(RichText::new(format!("Goes to {}.", self.ai_destination(*now))).small().weak());
                }
            }
        });

        if new_chat {
            p.turns.clear();
            p.error = None;
        }

        egui::Panel::bottom("ai-bottom").frame(Frame::new().inner_margin(Margin::same(10))).resizable(false).show(ui, |ui| {
            if let Some(e) = &p.error {
                ui.label(RichText::new(e).color(egui::Color32::from_rgb(235, 120, 110)));
                ui.add_space(4.0);
            }
            // The bars at the cursor, if the person wants them in the question.
            let bars = self.ai_bars();
            let label = match &bars {
                Some((a, b, _)) if a == b => format!("Include bar {} of this track", a + 1),
                Some((a, b, _)) => format!("Include bars {} to {} of this track", a + 1, b + 1),
                None => "Include the bars of this track".to_string(),
            };
            ui.add_enabled(bars.is_some(), egui::Checkbox::new(&mut p.with_bars, label))
                .on_hover_text("Sent as ASCII tablature with the key and time signature, and the notes, scales and chords GuitarCraft finds in them. The title, artist and file name are not sent.")
                .on_disabled_hover_text("Only tracks with strings can be sent as tablature");
            let id =ui.make_persistent_id("ai-input");
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

        egui::CentralPanel::default().frame(Frame::new().inner_margin(Margin::symmetric(10, 4))).show(ui, |ui| {
            egui::ScrollArea::vertical().id_salt("ai-history").stick_to_bottom(true).auto_shrink([false, false]).show(ui, |ui| {
                if p.turns.is_empty() {
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(
                            "Ask a question about your music or about GuitarCraft. Nothing is sent until you press Return. \
                             The assistant answers in words; it doesn't change your score.\n\n\
                             Try: \u{201C}What key is this in?\u{201D} (with the bars included), \u{201C}What scale fits these bars?\u{201D}, \
                             or \u{201C}How do I add a repeat?\u{201D}",
                        )
                        .weak(),
                    );
                }
                let who = current.as_ref().map(|c| c.0.name()).unwrap_or("Assistant");
                for t in &p.turns {
                    ui.add_space(6.0);
                    let fill = if t.from_user { egui::Color32::from_rgb(44, 48, 56) } else { egui::Color32::from_rgb(34, 36, 42) };
                    Frame::new().fill(fill).corner_radius(6.0).inner_margin(Margin::same(8)).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(RichText::new(if t.from_user { "You" } else { who }).small().strong());
                        ui.add(egui::Label::new(&t.text).selectable(true));
                        if !t.from_user && ui.small_button("Copy").clicked() {
                            ui.ctx().copy_text(t.text.clone());
                        }
                    });
                }
            });
        });
        self.ai = p;
    }
}
