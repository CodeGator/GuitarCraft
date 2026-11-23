//! A panel popped out into a window of its own, which can leave the main window and go to
//! another monitor. It is an immediate viewport: the panel is drawn by the same code as when
//! docked, only into a different window. "Dock" in that window, or View > Dock All Panels, puts
//! it back; closing the window hides the panel.
//!
//! Where the window is isn't remembered, on purpose: a window that always opens at its default
//! place is the way back if one is ever lost off-screen.

use super::dock::Pane;
use egui::{Context, Ui, ViewportBuilder, ViewportId};

/// What the person did in a popped-out panel's window this frame.
#[derive(Default, Debug, PartialEq, Eq)]
pub struct Outcome {
    /// "Dock" was pressed: put the panel back in the main window.
    pub dock_back: bool,
    /// The window was closed: hide the panel.
    pub closed: bool,
}

/// Draws `pane` in a window of its own. `body` draws the panel's contents.
pub fn popped_window(ctx: &Context, pane: Pane, body: impl FnOnce(&mut Ui)) -> Outcome {
    let mut out = Outcome::default();
    let mut body = Some(body);
    let title = pane.title();
    let [w, h] = pane.window_size();
    let builder = ViewportBuilder::default().with_title(format!("{title} \u{2013} GuitarCraft")).with_inner_size([w, h]).with_min_inner_size([160.0, 80.0]);
    ctx.show_viewport_immediate(ViewportId::from_hash_of(("pane", title)), builder, |ctx, _class| {
        // When the system can't make a second window egui wraps this in a floating one; the
        // contents are drawn the same way.
        egui::CentralPanel::default().frame(egui::Frame::new()).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("\u{29C9} Dock").on_hover_text("Put this panel back in the main window").clicked() {
                        out.dock_back = true;
                    }
                });
            });
            if let Some(body) = body.take() {
                body(ui);
            }
        });
        out.closed |= ctx.input(|i| i.viewport().close_requested());
    });
    out
}
