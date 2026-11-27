//! Drawing the dock: the panels and the score in one `egui_dock` area under the toolbar and the
//! document tabs, panels popped out into windows of their own, and the switches the toolbar, the
//! View menu and the shortcuts use. The arrangement itself (`Pane`, where panels start, saving)
//! is in `dock.rs`.

use super::dock::{self, Pane};
use super::float;
use super::panels::PANEL_BG;
use super::App;
use egui::{Color32, Rect, Ui};
use egui_dock::{DockArea, DockState, NodePath, TabViewer};

/// The score's page colour.
const SCORE_BG: Color32 = Color32::from_rgb(232, 233, 237);
/// The dark strip behind a panel's tabs.
const TAB_BAR_BG: Color32 = Color32::from_rgb(32, 34, 40);

/// What the dock needs to draw each panel: the app, and what the person asked for this frame.
struct PaneViewer<'a> {
    app: &'a mut App,
    score_rect: Rect,
    /// "Pop out" was chosen on this panel's tab.
    pop: Option<Pane>,
}

impl TabViewer for PaneViewer<'_> {
    type Tab = Pane;

    fn id(&mut self, tab: &mut Pane) -> egui::Id {
        egui::Id::new(("pane", *tab))
    }

    fn title(&mut self, tab: &mut Pane) -> egui::WidgetText {
        tab.title().into()
    }

    fn ui(&mut self, ui: &mut Ui, tab: &mut Pane) {
        if *tab == Pane::Score {
            self.score_rect = ui.max_rect();
        }
        self.app.pane_ui(ui, *tab);
    }

    fn context_menu(&mut self, ui: &mut Ui, tab: &mut Pane, _path: NodePath) {
        if *tab == Pane::Score {
            return;
        }
        if ui.button("Pop out").on_hover_text("A window of its own, which can leave the main window").clicked() {
            self.pop = Some(*tab);
            ui.close();
        }
        // (The dock adds its own "Eject" (float in the window) and "Close" below.)
    }

    /// A tab dragged out of the window becomes a window of its own.
    fn on_tab_button(&mut self, tab: &mut Pane, response: &egui::Response) {
        if *tab == Pane::Score || !response.dragged() {
            return;
        }
        let ctx = &response.ctx;
        let outside = match ctx.input(|i| i.pointer.latest_pos()) {
            None => true,
            Some(p) => !ctx.content_rect().contains(p),
        };
        if outside {
            self.pop = Some(*tab);
        }
    }

    fn is_closeable(&self, tab: &Pane) -> bool {
        tab.closeable()
    }

    fn allowed_in_windows(&self, tab: &mut Pane) -> bool {
        tab.closeable()
    }

    // Each panel scrolls itself where it needs to.
    fn scroll_bars(&self, _tab: &Pane) -> [bool; 2] {
        [false, false]
    }

    fn clear_background(&self, _tab: &Pane) -> bool {
        false
    }
}

/// The dock's colours, to go with the rest of the window.
fn dock_style(ui: &Ui) -> egui_dock::Style {
    let mut s = egui_dock::Style::from_egui(ui.style());
    s.tab_bar.bg_fill = TAB_BAR_BG;
    s.tab_bar.hline_color = Color32::from_rgb(20, 21, 25);
    s.tab_bar.height = 22.0;
    s.tab.tab_body.inner_margin = egui::Margin::ZERO;
    s.tab.tab_body.stroke = egui::Stroke::NONE;
    s.tab.tab_body.bg_fill = PANEL_BG;
    s.main_surface_border_stroke = egui::Stroke::NONE;
    // The drop targets that show while a tab is dragged, clear against the white score.
    let blue = Color32::from_rgb(61, 123, 224);
    s.overlay.button_color = Color32::from_rgb(40, 44, 56);
    s.overlay.button_border_stroke = egui::Stroke::new(2.0, blue);
    s.overlay.selection_color = Color32::from_rgba_unmultiplied(61, 123, 224, 150);
    s.overlay.selection_stroke_width = 3.0;
    s.overlay.surface_fade_opacity = 0.55;
    s.separator.color_idle = Color32::from_rgb(20, 21, 25);
    s.separator.width = 2.0;
    for t in [&mut s.tab.active, &mut s.tab.focused, &mut s.tab.hovered] {
        t.bg_fill = PANEL_BG;
        t.text_color = Color32::from_gray(235);
        t.outline_color = Color32::TRANSPARENT;
    }
    s.tab.inactive.bg_fill = TAB_BAR_BG;
    s.tab.inactive.text_color = Color32::from_gray(160);
    s.tab.inactive.outline_color = Color32::TRANSPARENT;
    s
}

/// A panel's background, filling all the room it has.
fn filled(ui: &mut Ui, fill: Color32, margin: i8, body: impl FnOnce(&mut Ui)) {
    egui::Frame::new().fill(fill).inner_margin(egui::Margin::same(margin)).show(ui, |ui| {
        ui.set_min_size(ui.available_size());
        body(ui);
    });
}

impl App {
    /// Whether the panel is showing, docked or in a window of its own.
    pub(super) fn panel_shown(&self, pane: Pane) -> bool {
        dock::is_open(&self.dock, pane) || self.popped.contains(&pane) || self.dock_cmds.iter().rev().find(|c| c.0 == pane).is_some_and(|c| c.1)
    }

    /// Shows the panel (where it was, or where it starts) and brings it to the front.
    pub(super) fn show_panel(&mut self, pane: Pane) {
        if self.dock_busy {
            self.dock_cmds.push((pane, true));
        } else if !self.popped.contains(&pane) {
            dock::open(&mut self.dock, pane);
            dock::bring_to_front(&mut self.dock, pane);
        }
    }

    pub(super) fn hide_panel(&mut self, pane: Pane) {
        if self.dock_busy {
            self.dock_cmds.push((pane, false));
        } else {
            dock::close(&mut self.dock, pane);
            self.popped.retain(|p| *p != pane);
        }
    }

    pub(super) fn toggle_panel(&mut self, pane: Pane) {
        if self.panel_shown(pane) {
            self.hide_panel(pane);
        } else {
            self.show_panel(pane);
        }
    }

    /// Moves a panel out of the main window into a window of its own.
    pub(super) fn pop_out(&mut self, pane: Pane) {
        if !pane.closeable() {
            return;
        }
        self.hide_panel(pane);
        if !self.popped.contains(&pane) {
            self.popped.push(pane);
        }
    }

    /// Puts a popped-out panel back in the main window.
    pub(super) fn dock_panel(&mut self, pane: Pane) {
        self.popped.retain(|p| *p != pane);
        self.show_panel(pane);
    }

    /// View > Reset Panel Layout.
    pub(super) fn reset_panel_layout(&mut self) {
        self.popped.clear();
        self.dock = dock::default_layout();
    }

    /// The contents of one pane, as the dock or a window of its own shows it.
    pub(super) fn pane_ui(&mut self, ui: &mut Ui, pane: Pane) {
        match pane {
            Pane::Score => filled(ui, SCORE_BG, 0, |ui| self.score(ui)),
            Pane::Palette => filled(ui, PANEL_BG, 8, |ui| {
                egui::ScrollArea::vertical().id_salt("pal").auto_shrink([false, false]).show(ui, |ui| self.palette(ui));
            }),
            Pane::Inspector => filled(ui, PANEL_BG, 0, |ui| self.inspector_ui(ui)),
            Pane::Tracks => filled(ui, Color32::from_rgb(34, 36, 42), 0, |ui| self.tracks_panel(ui)),
            Pane::Fretboard => {
                let sounding = std::mem::take(&mut self.sounding);
                filled(ui, Color32::from_rgb(28, 30, 34), 6, |ui| self.fretboard_panel(ui, &sounding));
                self.sounding = sounding;
            }
            Pane::Keyboard => {
                let sounding = std::mem::take(&mut self.sounding);
                filled(ui, Color32::from_rgb(28, 30, 34), 0, |ui| self.keyboard_panel(ui, &sounding));
                self.sounding = sounding;
            }
            Pane::Drums => {
                let sounding = std::mem::take(&mut self.sounding);
                filled(ui, Color32::from_rgb(28, 30, 34), 0, |ui| self.drum_panel(ui, &sounding));
                self.sounding = sounding;
            }
            Pane::Ai => filled(ui, PANEL_BG, 0, |ui| self.ai_panel(ui)),
            Pane::Retired => {}
        }
    }

    /// The panels and the score, filling what is left under the toolbar and the document tabs.
    /// Returns where the score was drawn.
    pub(super) fn dock_ui(&mut self, ui: &mut Ui) -> Rect {
        let mut state = std::mem::replace(&mut self.dock, DockState::new(vec![]));
        dock::tidy_score_bar(&mut state);
        self.dock_busy = true;
        let style = dock_style(ui);
        let mut viewer = PaneViewer { app: self, score_rect: ui.max_rect(), pop: None };
        DockArea::new(&mut state)
            .style(style)
            .hidable_tab_bars(true)
            .show_leaf_collapse_buttons(false)
            .show_leaf_close_all_buttons(false)
            .show_inside(ui, &mut viewer);
        let (score_rect, pop) = (viewer.score_rect, viewer.pop);
        self.dock = state;
        self.dock_busy = false;
        self.score_rect = score_rect;
        for (pane, on) in std::mem::take(&mut self.dock_cmds) {
            if on {
                self.show_panel(pane);
            } else {
                self.hide_panel(pane);
            }
        }
        if let Some(p) = pop {
            self.pop_out(p);
        }
        score_rect
    }

    /// The panels popped out into windows of their own.
    pub(super) fn popped_panels(&mut self, ctx: &egui::Context) {
        for pane in self.popped.clone() {
            let out = float::popped_window(ctx, pane, |ui| self.pane_ui(ui, pane));
            if out.dock_back {
                self.dock_panel(pane);
            } else if out.closed {
                self.popped.retain(|p| *p != pane);
            }
        }
        self.save_layout(ctx);
    }

    /// Writes the layout to its file when it has changed (not while a drag is going on).
    fn save_layout(&mut self, ctx: &egui::Context) {
        let Some(saved) = &self.layout_saved else { return };
        if ctx.input(|i| i.pointer.any_down()) {
            return;
        }
        let now = dock::to_json(&self.dock, &self.popped);
        if &now != saved {
            dock::save(&now);
            self.layout_saved = Some(now);
        }
    }
}
