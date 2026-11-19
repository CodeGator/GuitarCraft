//! Several files open at once, each in its own tab, as in Guitar Pro 8. The
//! tab bar sits under the toolbar (and stays when the start screen is
//! showing); opening or creating a song or chord chart adds a tab, and a
//! click on a tab switches to it for editing or playback. Each tab keeps its
//! own song or chart, cursor, selection and undo history; the clipboard is
//! shared, so notes can be copied from one tab to another.
//!
//! The open tab's data lives in the App's own fields, as before; the other
//! tabs' data is parked in `App::tabs` and swapped in when a tab is chosen.

use super::chartui::ChartState;
use super::panels::STRIP_BG;
use super::{App, Pending};
use crate::model::Song;
use egui::{Align2, Color32, FontFamily, FontId, Pos2, Rect, Sense, Ui, Vec2};
use std::path::PathBuf;

/// The data of one open file while its tab isn't the one showing.
pub struct Doc {
    pub song: Song,
    pub path: Option<PathBuf>,
    pub dirty: bool,
    pub track: usize,
    pub bar: usize,
    pub beat: usize,
    pub string: usize,
    pub voice: usize,
    pub sel_anchor: Option<(usize, usize)>,
    pub std_cursor: Option<i32>,
    pub undo: Vec<Song>,
    pub redo: Vec<Song>,
    pub save_hint: Option<PathBuf>,
    pub chart: Option<ChartState>,
}

impl Doc {
    /// A tab's name: the file's name without its ending, otherwise the title.
    fn label(&self) -> String {
        label_of(self.path.as_deref(), self.chart.as_ref().map(|c| c.doc.title.as_str()).unwrap_or(&self.song.title))
    }
}

fn label_of(path: Option<&std::path::Path>, title: &str) -> String {
    if let Some(stem) = path.and_then(|p| p.file_stem()) {
        return stem.to_string_lossy().to_string();
    }
    if title.trim().is_empty() { "Untitled".into() } else { title.trim().to_string() }
}

/// The first parked tab with unsaved changes (the showing tab's entry is None).
fn first_dirty(tabs: &[Option<Doc>]) -> Option<usize> {
    tabs.iter().position(|d| d.as_ref().map(|d| d.dirty).unwrap_or(false))
}

/// A tab's name, cut short with "..." to fit `room` points.
fn fit_text(ui: &Ui, name: &str, font: &FontId, room: f32) -> String {
    let width = |t: &str| ui.painter().layout_no_wrap(t.to_string(), font.clone(), Color32::WHITE).size().x;
    if width(name) <= room {
        return name.to_string();
    }
    let chars: Vec<char> = name.chars().collect();
    let mut n = chars.len();
    while n > 1 {
        n -= 1;
        let cut: String = chars[..n].iter().collect::<String>().trim_end().to_string() + "\u{2026}";
        if width(&cut) <= room {
            return cut;
        }
    }
    "\u{2026}".to_string()
}

enum TabAct {
    Switch(usize),
    Close(usize),
    New,
}

const TAB_H: f32 = 24.0;
const PLUS_W: f32 = 28.0;

impl App {
    // ----- keeping the tabs

    /// Moves the open tab's data out of the App's fields.
    fn take_doc(&mut self) -> Doc {
        Doc {
            song: std::mem::take(&mut self.song),
            path: self.path.take(),
            dirty: std::mem::replace(&mut self.dirty, false),
            track: self.track,
            bar: self.bar,
            beat: self.beat,
            string: self.string,
            voice: self.voice,
            sel_anchor: self.sel_anchor.take(),
            std_cursor: self.std_cursor.take(),
            undo: std::mem::take(&mut self.undo),
            redo: std::mem::take(&mut self.redo),
            save_hint: self.save_hint.take(),
            chart: self.chart.take(),
        }
    }

    /// Makes a tab's data the open one.
    fn put_doc(&mut self, d: Doc) {
        self.song = d.song;
        self.path = d.path;
        self.dirty = d.dirty;
        self.track = d.track;
        self.bar = d.bar;
        self.beat = d.beat;
        self.string = d.string;
        self.voice = d.voice;
        self.sel_anchor = d.sel_anchor;
        self.std_cursor = d.std_cursor;
        self.undo = d.undo;
        self.redo = d.redo;
        self.save_hint = d.save_hint;
        self.chart = d.chart;
        self.last_digit = None;
        self.scroll_to_cursor = true;
        self.total_secs = self.song_seconds();
    }

    /// Windows that belong to the file being edited close when another tab is chosen.
    fn close_doc_windows(&mut self) {
        self.popup = None;
        self.style_dlg = None;
        self.tuning_dlg = None;
        self.chord_open = false;
        self.chord_lib_open = false;
        self.lyrics_open = false;
        self.voices_open = false;
        self.auto_open = false;
        self.export_dlg = None;
        self.print_preview = None;
        self.special_paste = None;
        self.goto_open = false;
        self.transpose_open = false;
        self.dir_open = false;
        self.at.open = false;
        self.pending = None;
    }

    /// Called before a file is opened or created: the open tab is put away and
    /// a new tab takes its place for the new file.
    pub(super) fn begin_new_doc(&mut self) {
        self.stop();
        if self.tabs.is_empty() {
            self.tabs.push(None);
            self.active_tab = 0;
        } else {
            self.ensure_tab();
            let d = self.take_doc();
            self.tabs[self.active_tab] = Some(d);
            self.tabs.push(None);
            self.active_tab = self.tabs.len() - 1;
        }
        self.close_doc_windows();
    }

    /// A song that is showing always has a tab (pictures for checks set one up directly).
    pub(super) fn ensure_tab(&mut self) {
        if self.tabs.is_empty() && !self.show_home {
            self.tabs.push(None);
            self.active_tab = 0;
        }
    }

    /// Shows tab `i`, leaving the start screen if it was showing.
    pub(super) fn switch_to(&mut self, i: usize) {
        self.ensure_tab();
        if i >= self.tabs.len() {
            return;
        }
        if i != self.active_tab {
            self.stop();
            let d = self.take_doc();
            self.tabs[self.active_tab] = Some(d);
            if let Some(nd) = self.tabs[i].take() {
                self.put_doc(nd);
            }
            self.active_tab = i;
            self.close_doc_windows();
        }
        self.show_home = false;
    }

    /// Switches to the next (+1) or previous (-1) tab.
    pub(super) fn cycle_tab(&mut self, dir: i32) {
        let n = self.tabs.len() as i32;
        if n < 2 {
            return;
        }
        let at = if self.show_home { if dir > 0 { -1 } else { 0 } } else { self.active_tab as i32 };
        self.switch_to((at + dir).rem_euclid(n) as usize);
    }

    /// Closes the showing tab (after any question about unsaved changes) and
    /// shows a neighbour, or the start screen when it was the last.
    pub(super) fn close_song(&mut self) {
        self.stop();
        self.close_doc_windows();
        self.pending = None;
        if self.tabs.is_empty() {
            self.show_home = true;
            return;
        }
        self.tabs.remove(self.active_tab);
        // The empty page that stands in while no tab is showing.
        let blank = self.take_doc();
        drop(blank);
        self.song = Song::new_blank();
        self.reset_after_load();
        if self.tabs.is_empty() {
            self.active_tab = 0;
            self.show_home = true;
            return;
        }
        let next = self.active_tab.min(self.tabs.len() - 1);
        if let Some(d) = self.tabs[next].take() {
            self.put_doc(d);
        }
        self.active_tab = next;
        self.show_home = false;
    }

    /// Closes every open file (after the questions about unsaved changes) and shows the start screen.
    pub(super) fn close_all(&mut self) {
        self.stop();
        self.close_doc_windows();
        self.pending = None;
        self.tabs.clear();
        let blank = self.take_doc();
        drop(blank);
        self.song = Song::new_blank();
        self.reset_after_load();
        self.active_tab = 0;
        self.show_home = true;
    }

    /// The tab that already holds this file, if it is open.
    pub(super) fn find_open(&self, p: &std::path::Path) -> Option<usize> {
        if let Some(i) = self.tabs.iter().position(|d| d.as_ref().map(|d| d.path.as_deref() == Some(p)).unwrap_or(false)) {
            return Some(i);
        }
        (self.path.as_deref() == Some(p) && !self.tabs.is_empty()).then_some(self.active_tab)
    }

    /// Whether any open file has changes that haven't been saved.
    pub(super) fn any_dirty(&self) -> bool {
        self.dirty || self.tabs.iter().flatten().any(|d| d.dirty)
    }

    /// Another tab (not the showing one) with unsaved changes.
    pub(super) fn other_dirty_tab(&self) -> Option<usize> {
        first_dirty(&self.tabs)
    }

    /// How many open files have unsaved changes.
    pub(super) fn dirty_count(&self) -> usize {
        usize::from(self.dirty) + self.tabs.iter().flatten().filter(|d| d.dirty).count()
    }

    /// The names of the tabs, with whether each has unsaved changes.
    pub(super) fn tab_labels(&self) -> Vec<(String, bool)> {
        let own = label_of(self.path.as_deref(), self.chart.as_ref().map(|c| c.doc.title.as_str()).unwrap_or(&self.song.title));
        self.tabs
            .iter()
            .map(|d| match d {
                Some(d) => (d.label(), d.dirty),
                None => (own.clone(), self.dirty),
            })
            .collect()
    }

    /// The showing file's name, for questions about it.
    pub(super) fn doc_label(&self) -> String {
        label_of(self.path.as_deref(), self.chart.as_ref().map(|c| c.doc.title.as_str()).unwrap_or(&self.song.title))
    }

    /// Asks to close tab `i`: it is shown first, so the question about unsaved
    /// changes is about the right file.
    fn close_tab_at(&mut self, i: usize) {
        let home = self.show_home;
        self.switch_to(i);
        self.request(Pending::Close);
        if home && self.pending.is_none() && !self.tabs.is_empty() {
            self.show_home = true;
        }
    }

    // ----- the tab bar

    /// The tab bar, under the toolbar. With the start screen showing no tab is
    /// lit, and clicking one goes back to that file.
    pub(super) fn tab_panel(&mut self, ui: &mut Ui) {
        let labels = self.tab_labels();
        let lit = if self.show_home { usize::MAX } else { self.active_tab };
        let mut act: Option<TabAct> = None;
        egui::Panel::top("tabs").exact_size(TAB_H).frame(egui::Frame::new().fill(STRIP_BG)).show(ui, |ui| {
            let full = ui.max_rect();
            let n = labels.len().max(1);
            let tabs_w = full.width() - PLUS_W;
            let w = (tabs_w / n as f32).max(60.0);
            let bold = FontFamily::Name("bold".into());
            for (i, (name, dirty)) in labels.iter().enumerate() {
                let r = Rect::from_min_size(Pos2::new(full.left() + i as f32 * w, full.top()), Vec2::new(w, full.height()));
                let resp = ui.interact(r, ui.id().with(("tab", i)), Sense::click());
                let on = i == lit;
                let bg = if on {
                    Color32::from_rgb(152, 164, 194)
                } else if resp.hovered() {
                    Color32::from_rgb(118, 130, 160)
                } else {
                    Color32::from_rgb(98, 108, 136)
                };
                ui.painter().rect_filled(r.shrink2(Vec2::new(0.0, 0.0)), 0.0, bg);
                if i > 0 {
                    ui.painter().line_segment([r.left_top(), r.left_bottom()], egui::Stroke::new(1.0, Color32::from_black_alpha(40)));
                }
                let font = FontId::new(12.5, if on { bold.clone() } else { FontFamily::Proportional });
                let room = w - 44.0 - if *dirty { 14.0 } else { 0.0 };
                let shown = fit_text(ui, name, &font, room);
                let text = if *dirty { format!("{shown} *") } else { shown };
                ui.painter().text(r.center(), Align2::CENTER_CENTER, text, font, Color32::WHITE);
                // A close mark when the pointer is over the tab.
                if resp.hovered() || on {
                    let cr = Rect::from_center_size(Pos2::new(r.right() - 12.0, r.center().y), Vec2::splat(16.0));
                    let cresp = ui.interact(cr, ui.id().with(("tab-close", i)), Sense::click());
                    if resp.hovered() || cresp.hovered() {
                        if cresp.hovered() {
                            ui.painter().rect_filled(cr, 3.0, Color32::from_white_alpha(50));
                        }
                        ui.painter().text(cr.center(), Align2::CENTER_CENTER, "\u{2715}", FontId::proportional(10.5), Color32::WHITE);
                    }
                    if cresp.on_hover_text("Close this tab").clicked() {
                        act = Some(TabAct::Close(i));
                        continue;
                    }
                }
                if resp.clicked() {
                    act = Some(TabAct::Switch(i));
                }
                if resp.middle_clicked() {
                    act = Some(TabAct::Close(i));
                }
                resp.context_menu(|ui| {
                    if ui.button("Close").clicked() {
                        act = Some(TabAct::Close(i));
                        ui.close();
                    }
                });
            }
            // New song.
            let pr = Rect::from_min_max(Pos2::new(full.right() - PLUS_W, full.top()), full.right_bottom());
            let presp = ui.interact(pr, ui.id().with("tab-plus"), Sense::click()).on_hover_text("New song (Ctrl+N)");
            ui.painter().rect_filled(pr, 0.0, if presp.hovered() { Color32::from_rgb(118, 130, 160) } else { Color32::from_rgb(98, 108, 136) });
            ui.painter().text(pr.center(), Align2::CENTER_CENTER, "+", FontId::proportional(18.0), Color32::WHITE);
            if presp.clicked() {
                act = Some(TabAct::New);
            }
            if labels.is_empty() {
                ui.painter().text(full.center(), Align2::CENTER_CENTER, "GuitarCraft", FontId::new(12.5, bold), Color32::WHITE);
            }
        });
        match act {
            Some(TabAct::Switch(i)) => self.switch_to(i),
            Some(TabAct::Close(i)) => self.close_tab_at(i),
            Some(TabAct::New) => self.request(Pending::New),
            None => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(dirty: bool) -> Option<Doc> {
        Some(Doc {
            song: Song::new_blank(),
            path: None,
            dirty,
            track: 0,
            bar: 0,
            beat: 0,
            string: 0,
            voice: 0,
            sel_anchor: None,
            std_cursor: None,
            undo: vec![],
            redo: vec![],
            save_hint: None,
            chart: None,
        })
    }

    #[test]
    fn quitting_visits_each_tab_with_unsaved_changes() {
        // The showing tab (None) is skipped; parked tabs are asked about in order.
        let mut tabs = vec![doc(false), None, doc(true), doc(false), doc(true)];
        assert_eq!(first_dirty(&tabs), Some(2));
        // Once the first is dealt with (saved or discarded), the next is asked about.
        tabs[2].as_mut().unwrap().dirty = false;
        assert_eq!(first_dirty(&tabs), Some(4));
        tabs[4].as_mut().unwrap().dirty = false;
        assert_eq!(first_dirty(&tabs), None, "nothing left to ask: the app may close");
    }

    #[test]
    fn tab_names_come_from_the_file_or_the_title() {
        assert_eq!(label_of(Some(std::path::Path::new("C:/songs/Morning Walk - golpe.gp")), "Morning Walk"), "Morning Walk - golpe");
        assert_eq!(label_of(None, "My Song"), "My Song");
        assert_eq!(label_of(None, "  "), "Untitled");
    }
}
