//! Every message and question the app shows, drawn the same way: a modal box in the app's own
//! theme with a coloured icon, a title, the message in readable type and buttons on the right.
//! Enter presses the highlighted button and Escape cancels.
//!
//! Code anywhere can say something with [`error`], [`warning`] or `App::refuse` without a
//! window to hand: the message is queued, and [`show`] draws the oldest one each frame until
//! it is dismissed. Routine feedback ("Undone.", "Pasted 3 beats.") stays in the status bar.

use egui::{Align, Align2, Color32, Context, FontId, Frame, Id, Key, Layout, Margin, RichText, Sense, Ui, Vec2};
use std::cell::RefCell;

/// Width of every box's content (points).
const WIDTH: f32 = 420.0;
const BUTTON: Vec2 = Vec2::new(100.0, 28.0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Warning,
    Error,
    Question,
}

impl Kind {
    fn color(self) -> Color32 {
        match self {
            Kind::Warning => Color32::from_rgb(220, 150, 30),
            Kind::Error => Color32::from_rgb(210, 65, 65),
            Kind::Question => Color32::from_rgb(70, 140, 220),
        }
    }

    fn glyph(self) -> &'static str {
        match self {
            Kind::Warning | Kind::Error => "!",
            Kind::Question => "?",
        }
    }

    #[cfg(test)]
    fn name(self) -> &'static str {
        match self {
            Kind::Warning => "warning",
            Kind::Error => "error",
            Kind::Question => "question",
        }
    }

    fn title(self) -> &'static str {
        match self {
            Kind::Warning => "Notice",
            Kind::Error => "Something went wrong",
            Kind::Question => "Unsaved changes",
        }
    }
}

/// The answer to "this file has changes that haven't been saved".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unsaved {
    Save,
    DontSave,
    Cancel,
}

thread_local! {
    /// Messages waiting to be shown, oldest first.
    static QUEUE: RefCell<Vec<(Kind, String)>> = const { RefCell::new(vec![]) };
    /// For tests: answers to the next unsaved-changes questions, in place of the box.
    #[cfg(test)]
    static ANSWERS: RefCell<Vec<Unsaved>> = const { RefCell::new(vec![]) };
}

/// Something went wrong.
pub fn error(text: &str) {
    QUEUE.with(|q| q.borrow_mut().push((Kind::Error, text.to_string())));
}

/// Something wasn't done, or needs the person's attention.
pub fn warning(text: &str) {
    QUEUE.with(|q| q.borrow_mut().push((Kind::Warning, text.to_string())));
}

impl super::App {
    /// Says why something wasn't done: a warning box, and the same words in the status bar.
    pub(super) fn refuse(&mut self, text: impl Into<String>) {
        let text = text.into();
        warning(&text);
        self.status = text;
    }
}

/// Shows `text` as an error box and hands it back, to set as the status text as well:
/// `self.status = notice::failed(format!("Couldn't save: {e}"))`.
pub fn failed(text: String) -> String {
    error(&text);
    text
}

/// Is a message waiting or showing?
#[cfg(test)]
pub fn pending() -> bool {
    QUEUE.with(|q| !q.borrow().is_empty())
}

/// Everything a box needs: what it is, its buttons (left to right), which one Enter presses
/// and which one Escape presses.
struct Layout4<'a> {
    id: &'a str,
    kind: Kind,
    title: &'a str,
    buttons: &'a [&'a str],
    default: usize,
    cancel: Option<usize>,
}

/// Draws a box: icon and title, the body, then the buttons. Returns the button pressed this frame.
fn draw(ctx: &Context, b: Layout4, body: impl FnOnce(&mut Ui)) -> Option<usize> {
    let mut answer = None;
    let frame = Frame::popup(&ctx.global_style()).inner_margin(Margin::same(20));
    egui::Modal::new(Id::new(b.id)).frame(frame).show(ctx, |ui| {
        ui.set_width(WIDTH);
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(34.0), Sense::hover());
            ui.painter().circle_filled(rect.center(), 17.0, b.kind.color());
            ui.painter().text(rect.center(), Align2::CENTER_CENTER, b.kind.glyph(), FontId::proportional(22.0), Color32::WHITE);
            ui.add_space(6.0);
            ui.label(RichText::new(b.title).size(19.0).strong());
        });
        ui.add_space(12.0);
        body(ui);
        ui.add_space(18.0);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            // Right to left, so the last button is added first.
            for (i, label) in b.buttons.iter().enumerate().rev() {
                let text = RichText::new(*label).size(14.5);
                let button = if i == b.default {
                    egui::Button::new(text.strong()).fill(ui.visuals().selection.bg_fill)
                } else {
                    egui::Button::new(text)
                };
                if ui.add(button.min_size(BUTTON)).clicked() {
                    answer = Some(i);
                }
            }
        });
    });
    if answer.is_none() {
        if ctx.input(|i| i.key_pressed(Key::Enter)) {
            answer = Some(b.default);
        } else if ctx.input(|i| i.key_pressed(Key::Escape)) {
            answer = b.cancel;
        }
    }
    answer
}

/// Draws the oldest waiting message, if any, until it is dismissed. Call it every frame.
/// True while a message is showing, so the caller can hold other questions back.
pub fn show(ctx: &Context) -> bool {
    let Some((kind, text)) = QUEUE.with(|q| q.borrow().first().cloned()) else {
        return false;
    };
    let more = QUEUE.with(|q| q.borrow().len()) - 1;
    let b = Layout4 { id: "notice", kind, title: kind.title(), buttons: &["OK"], default: 0, cancel: Some(0) };
    let answered = draw(ctx, b, |ui| {
        ui.label(RichText::new(&text).size(14.5));
        if more > 0 {
            ui.add_space(8.0);
            ui.label(RichText::new(format!("{more} more after this one.")).size(12.5).weak());
        }
    });
    if answered.is_some() {
        QUEUE.with(|q| q.borrow_mut().remove(0));
    }
    true
}

/// "“name” has changes that haven't been saved." with Save, Don't save and Cancel; `more` is
/// an extra line (how many other files are waiting), or empty. Enter saves, Escape cancels.
/// `None` until a button is pressed.
pub fn unsaved(ctx: &Context, name: &str, more: &str) -> Option<Unsaved> {
    #[cfg(test)]
    {
        let queued = ANSWERS.with(|a| {
            let mut a = a.borrow_mut();
            if a.is_empty() { None } else { Some(a.remove(0)) }
        });
        if queued.is_some() {
            return queued;
        }
    }
    let b = Layout4 {
        id: "unsaved",
        kind: Kind::Question,
        title: Kind::Question.title(),
        buttons: &["Save", "Don't save", "Cancel"],
        default: 0,
        cancel: Some(2),
    };
    let answer = draw(ctx, b, |ui| {
        ui.label(RichText::new(format!("\u{201C}{name}\u{201D} has changes that haven't been saved.")).size(14.5));
        if !more.is_empty() {
            ui.add_space(8.0);
            ui.label(RichText::new(more).size(12.5).weak());
        }
    })?;
    Some([Unsaved::Save, Unsaved::DontSave, Unsaved::Cancel][answer])
}

/// For tests: the messages waiting, as (kind, text), emptied.
#[cfg(test)]
pub fn take_shown() -> Vec<(&'static str, String)> {
    QUEUE.with(|q| std::mem::take(&mut *q.borrow_mut()).into_iter().map(|(k, t)| (k.name(), t)).collect())
}

/// For tests: what the next unsaved-changes questions are answered with.
#[cfg(test)]
pub fn answer_next(a: &[Unsaved]) {
    ANSWERS.with(|x| *x.borrow_mut() = a.to_vec());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(key: Key) -> egui::RawInput {
        let mut input = egui::RawInput::default();
        input.events.push(egui::Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::NONE });
        input
    }

    /// One frame; the texture updates aren't needed as nothing is drawn to a screen.
    fn run(ctx: &Context, input: egui::RawInput, f: impl FnMut(&mut Ui)) {
        ctx.run_ui(input, f).textures_delta.clear();
    }

    #[test]
    fn a_message_shows_until_ok_and_they_come_one_at_a_time() {
        take_shown();
        let ctx = Context::default();
        error("Couldn't save: disk full");
        warning("Careful");
        assert!(pending());
        let mut showing = false;
        run(&ctx, egui::RawInput::default(), |ui| showing = show(&ui.ctx().clone()));
        assert!(showing, "the first message is on screen");
        assert_eq!(QUEUE.with(|q| q.borrow().len()), 2, "it stays until answered");
        // Enter dismisses the first; the second is still waiting.
        run(&ctx, key(Key::Enter), |ui| showing = show(&ui.ctx().clone()));
        assert_eq!(take_shown(), vec![("warning", "Careful".to_string())]);
        assert!(!pending());
    }

    #[test]
    fn the_unsaved_question_saves_on_enter_and_cancels_on_escape() {
        let ctx = Context::default();
        let run = |input: egui::RawInput| {
            let mut answer = None;
            run(&ctx, input, |ui| answer = unsaved(&ui.ctx().clone(), "Song", ""));
            answer
        };
        assert_eq!(run(egui::RawInput::default()), None);
        assert_eq!(run(key(Key::Enter)), Some(Unsaved::Save));
        assert_eq!(run(key(Key::Escape)), Some(Unsaved::Cancel));
    }
}
