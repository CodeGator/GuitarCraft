//! The screen for chord charts: the score tools give way to a sections list
//! and the map (the playing order, which can be rearranged) on the left, the
//! chart's pages in the middle, and the chart's details and the selected
//! section's text on the right. Charts are saved as .gcx files.

use super::icons::{icon_home, icon_toggle};
use super::widgets::{icon_redo, icon_undo};
use super::menus::item_if as item;
use super::panels::BLUE;
use super::{App, Pending};
use crate::chart::{self, ChordChart, Kind, MapEntry, Rgb, Section};
use egui::{Align2, Color32, FontFamily, FontId, RichText, Sense, Ui, Vec2};
use std::path::PathBuf;

const PANEL_BG: Color32 = Color32::from_rgb(44, 47, 54);

pub struct ChartState {
    pub doc: ChordChart,
    /// The selected section (index into `doc.sections`).
    pub sel: usize,
    /// The selected entry of the map.
    pub map_sel: Option<usize>,
    /// Right panel: 0 chart details, 1 the selected section.
    pub tab: u8,
    pub zoom: f32,
    /// Pages in the chart as last laid out (for the print and export windows).
    pub page_count: usize,
    pub undo: Vec<ChordChart>,
    pub redo: Vec<ChordChart>,
    /// The text field last typed in (typing in one field is one undo step).
    last_field: Option<egui::Id>,
    /// A change made while the mouse button is held (dragging a slider or a
    /// colour): the rest of that drag joins the same undo step.
    press_merge: bool,
    /// The chart's stylesheet window, when open.
    pub style_dlg: Option<super::chartstyle::ChartStyleDlg>,
    /// The Transpose window, when open.
    pub xpose: Option<super::chartxpose::XposeDlg>,
    /// The Chord Diagrams window is open.
    pub diagrams_open: bool,
    /// The tempo editor (the score's Automations editor, along the map).
    pub tempo_open: bool,
    pub tempo_ed: super::automation::TempoEdit,
    /// The time or key signature editor, when open.
    pub sig: Option<ChartSig>,
}

impl ChartState {
    pub fn new(mut doc: ChordChart) -> ChartState {
        // The sections list shows the playing order.
        doc.sync_sections();
        ChartState { doc, sel: 0, map_sel: None, tab: 1, zoom: 1.0, page_count: 1, undo: vec![], redo: vec![], last_field: None, press_merge: false, style_dlg: None, xpose: None, diagrams_open: false, tempo_open: false, tempo_ed: Default::default(), sig: None }
    }
}

fn heading(ui: &mut Ui, t: &str) {
    ui.add_space(6.0);
    ui.label(RichText::new(t).size(14.0).family(FontFamily::Name("bold".into())).color(Color32::from_gray(235)));
    ui.add_space(2.0);
}

/// A small chip of a section's colour (an outline when it has none).
fn swatch(ui: &mut Ui, c: Option<Rgb>) {
    let (r, _) = ui.allocate_exact_size(Vec2::new(10.0, 14.0), Sense::hover());
    let r = r.shrink2(Vec2::new(0.0, 1.0));
    match c {
        Some(c) => {
            ui.painter().rect_filled(r, 2.0, chart::color(c));
        }
        None => {
            ui.painter().rect_stroke(r, 2.0, egui::Stroke::new(1.0, Color32::from_gray(150)), egui::StrokeKind::Inside);
        }
    }
}

/// The chord chart's time or key signature being edited.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ChartSig {
    Time { num: u8, den: u8 },
    /// `none`: the chart shows no key.
    Key { key: i8, minor: bool, none: bool },
}

/// A chart's time ("6/8") as beats and beat value; 4/4 when it can't be read.
pub fn chart_time(t: &str) -> (u8, u8) {
    let mut it = t.split('/').map(|x| x.trim().parse::<u8>().ok());
    match (it.next().flatten(), it.next().flatten()) {
        (Some(n), Some(d)) if (1..=32).contains(&n) && [1, 2, 4, 8, 16, 32].contains(&d) => (n, d),
        _ => (4, 4),
    }
}

/// A chart's key ("E", "C#m", "Bb major") as a key signature: sharps (+) or
/// flats (-), and minor. The name's spelling decides between F# and Gb.
pub fn chart_key(k: &str) -> Option<(i8, bool)> {
    let (root, minor) = chart::split_key(k)?;
    let acc = |n: &str| match n.chars().nth(1) {
        Some('#') | Some('\u{266F}') => 1,
        Some('b') | Some('\u{266D}') => -1,
        _ => 0,
    };
    let want = acc(k.trim());
    let mut found: Vec<i8> = (-7i8..=7).filter(|&s| chart::split_key(&super::popups::key_short(s, minor)).map(|p| p.0) == Some(root)).collect();
    // The same spelling first, then the fewest sharps or flats.
    found.sort_by_key(|&s| (acc(&super::popups::key_short(s, minor)) != want, s.abs()));
    found.first().map(|&s| (s, minor))
}

/// A small square button with a drawn icon (dimmed when it can't be used).
fn icon_button(ui: &mut Ui, enabled: bool, tip: &str, draw: impl Fn(&egui::Painter, egui::Rect, Color32)) -> egui::Response {
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (r, resp) = ui.allocate_exact_size(Vec2::new(30.0, 26.0), sense);
    let p = ui.painter();
    let bg = if !enabled {
        Color32::from_rgb(50, 53, 60)
    } else if resp.is_pointer_button_down_on() {
        BLUE
    } else if resp.hovered() {
        Color32::from_rgb(78, 83, 96)
    } else {
        Color32::from_rgb(62, 66, 76)
    };
    p.rect_filled(r, 4.0, bg);
    let c = if enabled { Color32::from_gray(235) } else { Color32::from_gray(105) };
    draw(p, egui::Rect::from_center_size(r.center(), Vec2::splat(14.0)), c);
    resp.on_hover_text(tip)
}

fn draw_plus(p: &egui::Painter, r: egui::Rect, c: Color32) {
    let st = egui::Stroke::new(2.0, c);
    p.line_segment([r.center_top(), r.center_bottom()], st);
    p.line_segment([r.left_center(), r.right_center()], st);
}

/// Two overlapping pages.
fn draw_copy(p: &egui::Painter, r: egui::Rect, c: Color32) {
    let st = egui::Stroke::new(1.4, c);
    let back = egui::Rect::from_min_size(r.min, Vec2::new(9.0, 11.0));
    let front = egui::Rect::from_min_size(r.min + Vec2::new(5.0, 3.5), Vec2::new(9.0, 11.0));
    p.rect_stroke(back, 1.0, st, egui::StrokeKind::Middle);
    p.rect_filled(front, 1.0, Color32::from_rgb(62, 66, 76));
    p.rect_stroke(front, 1.0, st, egui::StrokeKind::Middle);
}

/// A waste bin.
fn draw_bin(p: &egui::Painter, r: egui::Rect, c: Color32) {
    let st = egui::Stroke::new(1.4, c);
    let top = r.top() + 3.0;
    p.line_segment([egui::Pos2::new(r.left(), top), egui::Pos2::new(r.right(), top)], st);
    p.line_segment([egui::Pos2::new(r.center().x - 2.5, r.top()), egui::Pos2::new(r.center().x + 2.5, r.top())], st);
    let body = [
        egui::Pos2::new(r.left() + 2.0, top),
        egui::Pos2::new(r.left() + 3.0, r.bottom()),
        egui::Pos2::new(r.right() - 3.0, r.bottom()),
        egui::Pos2::new(r.right() - 2.0, top),
    ];
    p.add(egui::Shape::line(body.to_vec(), st));
    for dx in [-2.2f32, 0.0, 2.2] {
        let x = r.center().x + dx;
        p.line_segment([egui::Pos2::new(x, top + 3.0), egui::Pos2::new(x, r.bottom() - 2.0)], egui::Stroke::new(1.0, c));
    }
}

fn draw_arrow(p: &egui::Painter, r: egui::Rect, c: Color32, up: bool) {
    let (tip, base) = if up { (r.top() + 1.0, r.bottom() - 2.0) } else { (r.bottom() - 1.0, r.top() + 2.0) };
    let w = 6.0;
    let neck = if up { tip + 6.0 } else { tip - 6.0 };
    let x = r.center().x;
    p.add(egui::Shape::convex_polygon(
        vec![egui::Pos2::new(x, tip), egui::Pos2::new(x + w, neck), egui::Pos2::new(x - w, neck)],
        c,
        egui::Stroke::NONE,
    ));
    p.line_segment([egui::Pos2::new(x, neck), egui::Pos2::new(x, base)], egui::Stroke::new(2.4, c));
}

/// Colours offered with one click (the picker has every colour).
const QUICK_COLORS: [Rgb; 10] = [
    [244, 204, 204],
    [255, 242, 204],
    [217, 234, 211],
    [208, 224, 227],
    [207, 226, 243],
    [217, 210, 233],
    [234, 209, 220],
    [252, 229, 205],
    [255, 255, 153],
    [224, 224, 224],
];

/// A colour that can be switched off: a tick box, the picker, and quick colours.
fn optional_color(ui: &mut Ui, c: &mut Option<Rgb>, default: Rgb, quick: bool) {
    ui.horizontal(|ui| {
        let mut on = c.is_some();
        if ui.checkbox(&mut on, "").on_hover_text("Use a colour").changed() {
            *c = if on { Some(default) } else { None };
        }
        match c {
            Some(v) => {
                ui.color_edit_button_srgb(v).on_hover_text("Choose the colour");
            }
            None => {
                ui.label(RichText::new("No colour").color(Color32::from_gray(160)));
            }
        }
    });
    if quick {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 3.0;
            for q in QUICK_COLORS {
                let (r, resp) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::click());
                ui.painter().rect_filled(r, 3.0, chart::color(q));
                let edge = if *c == Some(q) { egui::Stroke::new(2.0, Color32::WHITE) } else { egui::Stroke::new(1.0, Color32::from_gray(90)) };
                ui.painter().rect_stroke(r, 3.0, edge, egui::StrokeKind::Inside);
                if resp.on_hover_text("Use this colour").clicked() {
                    *c = Some(q);
                }
            }
        });
    }
}

impl App {
    /// File > New Chord Chart.
    pub(super) fn new_chart(&mut self) {
        self.stop();
        // A new chart starts with the stylesheet chosen for new charts.
        let mut doc = ChordChart::new_blank();
        doc.look = super::chartstyle::default_chart_style();
        for s in &mut doc.sections {
            s.color = doc.look.kind_color(s.kind);
        }
        self.chart = Some(ChartState::new(doc));
        self.path = None;
        self.dirty = false;
        self.show_home = false;
        self.status = "New chord chart. Write chords in [brackets] in the section text on the right.".into();
    }

    pub(super) fn open_chart_path(&mut self, p: PathBuf) {
        match ChordChart::load(&p) {
            Ok(doc) => {
                self.stop();
                self.chart = Some(ChartState::new(doc));
                self.path = Some(p.clone());
                self.dirty = false;
                self.show_home = false;
                self.add_recent(&p);
                self.status = format!("Opened {}.", p.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default());
            }
            Err(e) => self.status = format!("Couldn't open the chord chart: {e}"),
        }
    }

    /// Saves the chart (only ever as a chord chart file).
    pub(super) fn save_chart(&mut self, save_as: bool) {
        if self.screenshot.is_some() {
            return;
        }
        let Some(st) = &self.chart else { return };
        let is_chart = |p: &PathBuf| p.extension().map(|e| e.eq_ignore_ascii_case(chart::EXT)).unwrap_or(false);
        let path = match &self.path {
            Some(p) if !save_as && is_chart(p) => Some(p.clone()),
            _ => self
                .file_dialog()
                .add_filter("GuitarCraft chord chart", &[chart::EXT])
                .set_file_name(format!("{}.{}", super::widgets::sanitize(&st.doc.title), chart::EXT))
                .save_file()
                .map(|p| if is_chart(&p) { p } else { p.with_extension(chart::EXT) }),
        };
        let Some(p) = path else { return };
        match st.doc.save(&p) {
            Ok(()) => {
                self.dirty = false;
                self.add_recent(&p);
                self.path = Some(p.clone());
                self.status = format!("Saved {}.", p.display());
            }
            Err(e) => self.status = format!("Couldn't save: {e}"),
        }
    }

    fn chart_undo(&mut self) {
        if let Some(st) = &mut self.chart
            && let Some(prev) = st.undo.pop()
        {
            st.redo.push(std::mem::replace(&mut st.doc, prev));
            st.sel = st.sel.min(st.doc.sections.len().saturating_sub(1));
            self.dirty = true;
        }
    }

    fn chart_redo(&mut self) {
        if let Some(st) = &mut self.chart
            && let Some(next) = st.redo.pop()
        {
            st.undo.push(std::mem::replace(&mut st.doc, next));
            st.sel = st.sel.min(st.doc.sections.len().saturating_sub(1));
            self.dirty = true;
        }
    }

    /// Shortcuts while a chord chart is open.
    pub(super) fn handle_chart_keys(&mut self, ctx: &egui::Context) {
        if self.pending.is_some() {
            return;
        }
        let typing = ctx.egui_wants_keyboard_input();
        let events = ctx.input(|i| i.events.clone());
        for ev in events {
            if let egui::Event::Key { key, pressed: true, modifiers: m, .. } = ev {
                if !m.command {
                    if key == egui::Key::F1 {
                        self.show_help = true;
                    }
                    if key == egui::Key::F7 {
                        self.open_chart_style(0);
                    }
                    if key == egui::Key::F10
                        && let Some(st) = &mut self.chart
                    {
                        st.tempo_open = true;
                    }
                    continue;
                }
                match key {
                    egui::Key::S if m.shift => self.save_chart(true),
                    egui::Key::S => self.save_chart(false),
                    egui::Key::O => self.request(Pending::Open),
                    egui::Key::N if m.shift => self.request(Pending::NewChart),
                    egui::Key::N => self.request(Pending::New),
                    egui::Key::P => self.print_song(),
                    egui::Key::T => self.open_transpose(None),
                    egui::Key::W => self.request(Pending::Close),
                    egui::Key::Z if !typing && m.shift => self.chart_redo(),
                    egui::Key::Z if !typing => self.chart_undo(),
                    egui::Key::Y if !typing => self.chart_redo(),
                    egui::Key::Plus | egui::Key::Equals => {
                        if let Some(st) = &mut self.chart {
                            st.zoom = (st.zoom * 1.1).min(3.0);
                        }
                    }
                    egui::Key::Minus => {
                        if let Some(st) = &mut self.chart {
                            st.zoom = (st.zoom / 1.1).max(0.4);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    /// The menus while a chord chart is open: only what works for charts.
    pub(super) fn chart_menu_bar(&mut self, ui: &mut Ui) {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if item(ui, true, "New Song", "Ctrl+N") {
                    self.request(Pending::New);
                    ui.close();
                }
                if item(ui, true, "New Chord Chart", "Ctrl+Shift+N") {
                    self.request(Pending::NewChart);
                    ui.close();
                }
                if item(ui, true, "Open...", "Ctrl+O") {
                    self.request(Pending::Open);
                    ui.close();
                }
                if item(ui, true, "Browse", "") {
                    self.show_home = true;
                    ui.close();
                }
                ui.separator();
                if item(ui, true, "Close", "Ctrl+W") {
                    self.request(Pending::Close);
                    ui.close();
                }
                if item(ui, true, "Save", "Ctrl+S") {
                    self.save_chart(false);
                    ui.close();
                }
                if item(ui, true, "Save As...", "Ctrl+Shift+S") {
                    self.save_chart(true);
                    ui.close();
                }
                ui.separator();
                ui.menu_button("Export", |ui| {
                    if ui.button("PDF...").clicked() {
                        self.open_export(super::printing::ExportKind::Pdf);
                        ui.close();
                    }
                    if ui.button("PNG...").clicked() {
                        self.open_export(super::printing::ExportKind::Png);
                        ui.close();
                    }
                });
                if item(ui, true, "Print...", "Ctrl+P") {
                    ui.close();
                    self.print_song();
                }
                ui.separator();
                if item(ui, true, "Quit", "Alt+F4") {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    ui.close();
                }
            });
            let (can_undo, can_redo) = self.chart.as_ref().map(|s| (!s.undo.is_empty(), !s.redo.is_empty())).unwrap_or((false, false));
            ui.menu_button("Edit", |ui| {
                if item(ui, can_undo, "Undo", "Ctrl+Z") {
                    self.chart_undo();
                    ui.close();
                }
                if item(ui, can_redo, "Redo", "Ctrl+Y") {
                    self.chart_redo();
                    ui.close();
                }
                ui.separator();
                if item(ui, true, "Transpose...", "Ctrl+T") {
                    self.open_transpose(None);
                    ui.close();
                }
                if item(ui, true, "Transpose Up a Half Step", "") {
                    self.transpose_step(1);
                    ui.close();
                }
                if item(ui, true, "Transpose Down a Half Step", "") {
                    self.transpose_step(-1);
                    ui.close();
                }
            });
            ui.menu_button("Section", |ui| {
                for k in Kind::ALL {
                    if ui.button(format!("Add {}", k.label())).clicked() {
                        self.chart_add_section(k);
                        ui.close();
                    }
                }
                ui.separator();
                if ui.button("Duplicate Section").clicked() {
                    self.chart_duplicate();
                    ui.close();
                }
                if ui.button("Delete Section").clicked() {
                    self.chart_delete();
                    ui.close();
                }
                if ui.button("Add Section to the Map").clicked() {
                    self.chart_map_add();
                    ui.close();
                }
            });
            ui.menu_button("View", |ui| {
                if item(ui, true, "Tempo...", "F10") {
                    if let Some(st) = &mut self.chart {
                        st.tempo_open = true;
                    }
                    ui.close();
                }
                if item(ui, true, "Chord Diagrams...", "") {
                    if let Some(st) = &mut self.chart {
                        st.diagrams_open = true;
                    }
                    ui.close();
                }
                if item(ui, true, "Stylesheet...", "F7") {
                    self.open_chart_style(0);
                    ui.close();
                }
                if let Some(st) = &mut self.chart {
                    ui.menu_button("Columns", |ui| {
                        for c in 1..=chart::MAX_COLUMNS {
                            let lab = if c == 1 { "1 Column".to_string() } else { format!("{c} Columns") };
                            if ui.radio(st.doc.look.columns == c, lab).clicked() {
                                st.doc.look.columns = c;
                                ui.close();
                            }
                        }
                    });
                    ui.separator();
                    if ui.button("Zoom In").clicked() {
                        st.zoom = (st.zoom * 1.1).min(3.0);
                    }
                    if ui.button("Zoom Out").clicked() {
                        st.zoom = (st.zoom / 1.1).max(0.4);
                    }
                }
            });
            ui.menu_button("Tools", |ui| {
                if item(ui, true, "Chords & Scales...", "") {
                    self.open_chord_scales(super::chordscales::Mode::Chords);
                    ui.close();
                }
            });
            ui.menu_button("Help", |ui| {
                if ui.button("Chord chart help").clicked() {
                    self.status = "Chords go in [brackets] before the syllable: [E]Let my heart. A line starting with | is bars: | E /// | B //// |.".into();
                    ui.close();
                }
                if ui.button("About GuitarCraft").clicked() {
                    self.show_about = true;
                    ui.close();
                }
            });
        });
    }

    fn chart_add_section(&mut self, kind: Kind) {
        let Some(st) = &mut self.chart else { return };
        let id = st.doc.next_id();
        let name = st.doc.new_name(kind);
        let text = if matches!(kind, Kind::Intro | Kind::Interlude | Kind::Instrumental | Kind::Solo | Kind::Outro) { "| G /// | C /// |".to_string() } else { "[G]New line of [C]words".to_string() };
        let color = st.doc.look.kind_color(kind);
        st.doc.sections.push(Section { id, name, kind, text, color, new_column: false });
        st.sel = st.doc.sections.len() - 1;
        // A new section is also played: it goes at the end of the map.
        st.doc.map.push(MapEntry::new(id, 1));
        st.tab = 1;
    }

    fn chart_duplicate(&mut self) {
        let Some(st) = &mut self.chart else { return };
        let Some(src) = st.doc.sections.get(st.sel).cloned() else { return };
        let id = st.doc.next_id();
        let name = st.doc.new_name(src.kind);
        st.doc.sections.insert(st.sel + 1, Section { id, name, ..src });
        st.sel += 1;
        // The copy is played right after the original's first time.
        if let Some(p) = st.doc.map.iter().position(|e| e.section == src.id) {
            st.doc.map.insert(p + 1, MapEntry::new(id, 1));
        }
    }

    fn chart_delete(&mut self) {
        let Some(st) = &mut self.chart else { return };
        if st.doc.sections.len() <= 1 {
            self.status = "A chart needs at least one section.".into();
            return;
        }
        let id = st.doc.sections[st.sel].id;
        st.doc.sections.remove(st.sel);
        st.doc.map.retain(|e| e.section != id);
        st.sel = st.sel.min(st.doc.sections.len() - 1);
        st.map_sel = None;
    }

    /// Adds the selected section to the map, after the selected entry (or at the end).
    fn chart_map_add(&mut self) {
        let Some(st) = &mut self.chart else { return };
        let Some(s) = st.doc.sections.get(st.sel) else { return };
        let e = MapEntry::new(s.id, 1);
        let at = st.map_sel.map(|k| k + 1).unwrap_or(st.doc.map.len()).min(st.doc.map.len());
        st.doc.map.insert(at, e);
        st.map_sel = Some(at);
    }

    /// The chart screen (everything under the menu bar).
    pub(super) fn chart_screen(&mut self, ui: &mut Ui) {
        let Some(before) = self.chart.as_ref().map(|s| s.doc.clone()) else { return };
        let ctx = ui.ctx().clone();
        egui::Panel::top("chart-toolbar")
            .frame(egui::Frame::new().fill(Color32::from_rgb(40, 43, 50)).inner_margin(egui::Margin::symmetric(8, 5)))
            .show(ui, |ui| self.chart_toolbar(ui));
        egui::Panel::top("chart-strip").exact_size(22.0).frame(egui::Frame::new().fill(Color32::from_rgb(110, 118, 140))).show(ui, |ui| {
            let title = self.chart.as_ref().map(|s| s.doc.title.clone()).unwrap_or_default();
            ui.centered_and_justified(|ui| {
                ui.label(RichText::new(format!("GuitarCraft - {title}  \u{00B7}  chord chart")).family(FontFamily::Name("bold".into())).size(12.5).color(Color32::WHITE));
            });
        });
        egui::Panel::left("chart-sections")
            .exact_size(250.0)
            .frame(egui::Frame::new().fill(PANEL_BG).inner_margin(egui::Margin::same(8)))
            .show(ui, |ui| {
                egui::ScrollArea::vertical().id_salt("chart-left").show(ui, |ui| self.chart_left(ui));
            });
        egui::Panel::right("chart-details")
            .exact_size(320.0)
            .frame(egui::Frame::new().fill(PANEL_BG))
            .show(ui, |ui| self.chart_right(ui));
        egui::CentralPanel::default().frame(egui::Frame::new().fill(Color32::from_rgb(232, 233, 237))).show(ui, |ui| self.chart_pages(ui));
        self.chart_style_window(&ctx);
        self.transpose_window(&ctx);
        self.diagrams_window(&ctx);
        self.chart_tempo_window(&ctx);
        self.chart_sig_window(&ctx);
        // The sections list follows the map (after map edits, undo, adding ...),
        // keeping the same section selected.
        if let Some(st) = &mut self.chart {
            let sel_id = st.doc.sections.get(st.sel).map(|s| s.id);
            st.doc.sync_sections();
            if let Some(id) = sel_id {
                st.sel = st.doc.sections.iter().position(|s| s.id == id).unwrap_or(0);
            }
        }
        // One undo step per change (typing in one field counts as one).
        let focused = ctx.memory(|m| m.focused());
        let pressing = ctx.input(|i| i.pointer.any_down());
        if let Some(st) = &mut self.chart {
            if st.doc != before {
                let same_field = focused.is_some() && focused == st.last_field;
                let same_press = pressing && st.press_merge;
                st.press_merge = pressing;
                if !same_field && !same_press {
                    st.undo.push(before);
                    if st.undo.len() > 200 {
                        st.undo.remove(0);
                    }
                }
                st.redo.clear();
                st.last_field = focused;
                self.dirty = true;
            } else if focused != st.last_field && focused.is_none() {
                st.last_field = None;
            }
            if !pressing {
                st.press_merge = false;
            }
        }
    }

    fn chart_toolbar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            if icon_toggle(ui, false, "Start screen", icon_home).clicked() {
                self.show_home = true;
            }
            ui.add_space(8.0);
            let (u, r) = self.chart.as_ref().map(|s| (!s.undo.is_empty(), !s.redo.is_empty())).unwrap_or((false, false));
            if ui.add_enabled_ui(u, |ui| icon_toggle(ui, false, "Undo (Ctrl+Z)", icon_undo)).inner.clicked() {
                self.chart_undo();
            }
            if ui.add_enabled_ui(r, |ui| icon_toggle(ui, false, "Redo (Ctrl+Y)", icon_redo)).inner.clicked() {
                self.chart_redo();
            }
            ui.add_space(12.0);
            if let Some(st) = &mut self.chart {
                if ui.button("\u{2212}").on_hover_text("Zoom out").clicked() {
                    st.zoom = (st.zoom / 1.1).max(0.4);
                }
                ui.label(RichText::new(format!("{:.0} %", st.zoom * 100.0)).color(Color32::from_gray(220)));
                if ui.button("+").on_hover_text("Zoom in").clicked() {
                    st.zoom = (st.zoom * 1.1).min(3.0);
                }
                ui.add_space(12.0);
                ui.label(RichText::new("Columns:").color(Color32::from_gray(200)));
                ui.add(egui::DragValue::new(&mut st.doc.look.columns).range(1..=chart::MAX_COLUMNS)).on_hover_text("Columns of text on each page (part of the stylesheet)");
            }
            ui.add_space(12.0);
            // Transposing: a half step down, the window, a half step up.
            if ui.button(RichText::new("\u{266D}").size(15.0)).on_hover_text("Transpose the chart down a half step").clicked() {
                self.transpose_step(-1);
            }
            if ui.button("Transpose").on_hover_text("Move the chords to another key (Ctrl+T)").clicked() {
                self.open_transpose(None);
            }
            if ui.button(RichText::new("\u{266F}").size(15.0)).on_hover_text("Transpose the chart up a half step").clicked() {
                self.transpose_step(1);
            }
            ui.add_space(8.0);
            if ui.button("\u{1F3A8} Stylesheet").on_hover_text("Fonts, sizes, colours and columns of this chart (F7)").clicked() {
                self.open_chart_style(0);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("\u{1F5B6} Print").on_hover_text("Print the chart (Ctrl+P)").clicked() {
                    self.print_song();
                }
                if ui.button("Export PDF").clicked() {
                    self.open_export(super::printing::ExportKind::Pdf);
                }
                if ui.button("\u{1F4BE} Save").on_hover_text("Save the chord chart (Ctrl+S)").clicked() {
                    self.save_chart(false);
                }
            });
        });
    }

    fn chart_left(&mut self, ui: &mut Ui) {
        let Some(st) = &mut self.chart else { return };
        heading(ui, "SECTIONS");
        let mut select: Option<usize> = None;
        for (i, s) in st.doc.sections.iter().enumerate() {
            ui.horizontal(|ui| {
                swatch(ui, s.color);
                let used = st.doc.map.iter().filter(|e| e.section == s.id).count();
                let lab = if used == 0 { format!("{}  (not in the map)", s.name) } else { s.name.clone() };
                if ui.selectable_label(st.sel == i, lab).clicked() {
                    select = Some(i);
                }
            });
        }
        if let Some(i) = select {
            st.sel = i;
            st.tab = 1;
        }
        let n = st.doc.sections.len();
        let mut act: Option<u8> = None;
        // One row of icon buttons: add, duplicate and delete on the left, the
        // up and down arrows side by side on the right.
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            let add = icon_button(ui, true, "Add a section\u{2026}", draw_plus);
            egui::Popup::menu(&add).show(|ui| {
                ui.label(RichText::new("Add a section").size(11.5).color(Color32::from_gray(170)));
                for k in Kind::ALL {
                    if ui.button(k.label()).clicked() {
                        act = Some(100 + Kind::ALL.iter().position(|x| *x == k).unwrap_or(0) as u8);
                        ui.close();
                    }
                }
            });
            if icon_button(ui, n > 0, "Duplicate the section (the copy is played right after it)", draw_copy).clicked() {
                act = Some(1);
            }
            if icon_button(ui, n > 1, "Delete the section", draw_bin).clicked() {
                act = Some(2);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                if icon_button(ui, st.doc.can_move_section(st.sel, 1), "Play this section later (the map changes too)", |p, r, c| draw_arrow(p, r, c, false)).clicked() {
                    act = Some(4);
                }
                if icon_button(ui, st.doc.can_move_section(st.sel, -1), "Play this section earlier (the map changes too)", |p, r, c| draw_arrow(p, r, c, true)).clicked() {
                    act = Some(3);
                }
            });
        });
        match act {
            Some(k) if k >= 100 => self.chart_add_section(Kind::ALL[(k - 100) as usize]),
            Some(1) => self.chart_duplicate(),
            Some(2) => self.chart_delete(),
            Some(3) => {
                if let Some(st) = &mut self.chart {
                    st.sel = st.doc.move_section(st.sel, -1);
                    st.map_sel = None;
                }
            }
            Some(4) => {
                if let Some(st) = &mut self.chart {
                    st.sel = st.doc.move_section(st.sel, 1);
                    st.map_sel = None;
                }
            }
            _ => {}
        }
        let Some(st) = &mut self.chart else { return };
        ui.add_space(8.0);
        ui.separator();
        heading(ui, "MAP");
        ui.label(RichText::new("The order the song is played. A section played again prints as \u{201C}Repeat \u{2026}\u{201D}.").size(11.0).color(Color32::from_gray(160)));
        ui.add_space(4.0);
        let mut map_act: Option<(usize, u8)> = None;
        let names: Vec<(u32, String, Option<Rgb>)> = st.doc.sections.iter().map(|s| (s.id, s.name.clone(), s.color)).collect();
        for (k, e) in st.doc.map.iter_mut().enumerate() {
            let Some((_, name, col)) = names.iter().find(|x| x.0 == e.section).cloned() else { continue };
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 3.0;
                ui.label(RichText::new(format!("{:>2}.", k + 1)).monospace().color(Color32::from_gray(150)));
                swatch(ui, col);
                let r = ui.add_sized(Vec2::new(98.0, 18.0), egui::Button::selectable(st.map_sel == Some(k), name));
                if r.clicked() {
                    map_act = Some((k, 0));
                }
                ui.add(egui::DragValue::new(&mut e.repeat).range(1..=8).prefix("x")).on_hover_text("Times played in a row");
                if ui.small_button("\u{25B2}").on_hover_text("Earlier").clicked() {
                    map_act = Some((k, 1));
                }
                if ui.small_button("\u{25BC}").on_hover_text("Later").clicked() {
                    map_act = Some((k, 2));
                }
                if ui.small_button("\u{2715}").on_hover_text("Take out of the map").clicked() {
                    map_act = Some((k, 3));
                }
            });
        }
        if let Some((k, a)) = map_act {
            let len = st.doc.map.len();
            match a {
                0 => {
                    st.map_sel = Some(k);
                    if let Some(i) = st.doc.sections.iter().position(|s| s.id == st.doc.map[k].section) {
                        st.sel = i;
                        st.tab = 1;
                    }
                }
                1 if k > 0 => {
                    st.doc.map.swap(k, k - 1);
                    st.map_sel = Some(k - 1);
                }
                2 if k + 1 < len => {
                    st.doc.map.swap(k, k + 1);
                    st.map_sel = Some(k + 1);
                }
                3 => {
                    st.doc.map.remove(k);
                    st.map_sel = None;
                }
                _ => {}
            }
        }
        // The selected entry: what it prints and in which colour.
        if let Some(k) = st.map_sel.filter(|k| *k < st.doc.map.len()) {
            let sid = st.doc.map[k].section;
            let again = st.doc.map[..k].iter().any(|e| e.section == sid);
            let sname = st.doc.section(sid).map(|s| s.name.clone()).unwrap_or_default();
            let auto = if again { format!("Repeat {sname}") } else { sname };
            let default_ink = if again { st.doc.look.repeat.color } else { st.doc.look.heading.color };
            ui.add_space(6.0);
            egui::Frame::new().fill(Color32::from_rgb(52, 56, 64)).corner_radius(4.0).inner_margin(egui::Margin::same(6)).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new(format!("Entry {} prints as:", k + 1)).size(11.5).color(Color32::from_gray(210)));
                let e = &mut st.doc.map[k];
                ui.add(egui::TextEdit::singleline(&mut e.label).hint_text(auto).desired_width(f32::INFINITY));
                ui.label(RichText::new("Its own colour (otherwise the LOOK tab's):").size(11.0).color(Color32::from_gray(170)));
                optional_color(ui, &mut e.color, default_ink, false);
            });
        }
        ui.add_space(4.0);
        let sel_name = st.doc.sections.get(st.sel).map(|s| s.name.clone()).unwrap_or_default();
        if ui.button(format!("+ Add \u{201C}{sel_name}\u{201D} to the map")).on_hover_text("After the selected map entry, or at the end").clicked() {
            self.chart_map_add();
        }
    }

    fn chart_right(&mut self, ui: &mut Ui) {
        let mut open_style = false;
        let mut open_xpose = false;
        let mut open_diagrams = false;
        let mut open_tempo = false;
        let mut open_sig: Option<bool> = None;
        self.chart_right_body(ui, &mut open_style, &mut open_xpose, &mut open_diagrams, &mut open_tempo, &mut open_sig);
        if let Some(time) = open_sig {
            self.open_chart_sig(time);
        }
        if open_tempo && let Some(st) = &mut self.chart {
            st.tempo_open = true;
        }
        if open_diagrams && let Some(st) = &mut self.chart {
            st.diagrams_open = true;
        }
        if open_style {
            self.open_chart_style(0);
        }
        if open_xpose {
            let sel = self.chart.as_ref().map(|s| s.sel);
            self.open_transpose(sel);
        }
    }

    fn chart_right_body(&mut self, ui: &mut Ui, open_style: &mut bool, open_xpose: &mut bool, open_diagrams: &mut bool, open_tempo: &mut bool, open_sig: &mut Option<bool>) {
        let Some(st) = &mut self.chart else { return };
        // Tabs across the top.
        let full = ui.available_rect_before_wrap();
        let (tr, _) = ui.allocate_exact_size(Vec2::new(full.width(), 26.0), Sense::hover());
        for (i, lab) in ["CHART", "SECTION"].iter().enumerate() {
            let r = egui::Rect::from_min_size(egui::Pos2::new(tr.left() + i as f32 * tr.width() / 2.0, tr.top()), Vec2::new(tr.width() / 2.0, 26.0));
            let resp = ui.interact(r, ui.id().with(("chart-tab", i)), Sense::click());
            let on = st.tab == i as u8;
            ui.painter().rect_filled(r, 0.0, if on { BLUE } else if resp.hovered() { Color32::from_rgb(66, 70, 80) } else { Color32::from_rgb(54, 58, 67) });
            ui.painter().text(r.center(), Align2::CENTER_CENTER, *lab, FontId::new(11.5, FontFamily::Name("bold".into())), Color32::WHITE);
            if resp.clicked() {
                st.tab = i as u8;
            }
        }
        egui::ScrollArea::vertical().id_salt("chart-right").show(ui, |ui| {
            egui::Frame::new().inner_margin(egui::Margin::symmetric(10, 6)).show(ui, |ui| {
                ui.set_width(ui.available_width());
                let fw = ui.available_width();
                if st.tab == 0 {
                    heading(ui, "CHART");
                    let d = &mut st.doc;
                    for (lab, v) in [("Title:", &mut d.title), ("Artist:", &mut d.artist), ("Capo:", &mut d.capo), ("Tuning note:", &mut d.tuning)] {
                        ui.label(RichText::new(lab).color(Color32::from_gray(210)));
                        ui.add(egui::TextEdit::singleline(v).desired_width(fw));
                    }
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Key:").color(Color32::from_gray(210)));
                        let key = if d.key.trim().is_empty() { "None".to_string() } else { d.key.trim().to_string() };
                        if ui.button(key).on_hover_text("Edit the key signature").clicked() {
                            *open_sig = Some(false);
                        }
                        ui.add_space(8.0);
                        ui.label(RichText::new("Time:").color(Color32::from_gray(210)));
                        if ui.button(d.time.trim()).on_hover_text("Edit the time signature").clicked() {
                            *open_sig = Some(true);
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("BPM:").color(Color32::from_gray(210)));
                        let changes = d.tempo_points().len() - 1;
                        let text = if changes == 0 { format!("\u{2669} = {}", d.bpm) } else { format!("\u{2669} = {}  (+{changes} change{})", d.bpm, if changes == 1 { "" } else { "s" }) };
                        if ui.button(text).on_hover_text("Edit the tempo and its changes (F10)").clicked() {
                            *open_tempo = true;
                        }
                    });
                    ui.label(RichText::new("Copyright (small, at the foot of each page):").color(Color32::from_gray(210)));
                    ui.add(egui::TextEdit::multiline(&mut d.copyright).desired_width(fw).desired_rows(2).hint_text("\u{00A9} 2026 Writer name. All rights reserved."));
                    heading(ui, "STYLE");
                    ui.label(RichText::new("Fonts, sizes, colours, columns and what is printed are in the chart's stylesheet.").size(11.0).color(Color32::from_gray(165)));
                    if ui.button("\u{1F3A8} Stylesheet... (F7)").clicked() {
                        *open_style = true;
                    }
                    heading(ui, "CHORD DIAGRAMS");
                    ui.checkbox(&mut d.look.show_diagrams, "Print chord diagrams at the end of the chart");
                    let total = d.diagram_choices().len();
                    let shown = if d.diagrams.is_empty() { "none".to_string() } else { d.diagrams.join(", ") };
                    ui.label(RichText::new(format!("{} of {total} chords shown: {shown}", d.diagrams.len())).size(11.0).color(Color32::from_gray(185)));
                    if ui.button("Choose Diagrams\u{2026}").on_hover_text("Pick which chords get diagrams").clicked() {
                        *open_diagrams = true;
                    }
                } else {
                    let sel = st.sel.min(st.doc.sections.len().saturating_sub(1));
                    let look = st.doc.look.clone();
                    let Some(s) = st.doc.sections.get_mut(sel) else { return };
                    heading(ui, "SECTION");
                    ui.label(RichText::new("Name:").color(Color32::from_gray(210)));
                    ui.add(egui::TextEdit::singleline(&mut s.name).desired_width(fw));
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Kind:").color(Color32::from_gray(210)));
                        let old = s.kind;
                        egui::ComboBox::from_id_salt("chart-kind").selected_text(s.kind.label()).show_ui(ui, |ui| {
                            for k in Kind::ALL {
                                ui.selectable_value(&mut s.kind, k, k.label());
                            }
                        });
                        // A section still in its kind's suggested colour follows the new kind.
                        if s.kind != old && s.color == look.kind_color(old) {
                            s.color = look.kind_color(s.kind);
                        }
                    });
                    ui.checkbox(&mut s.new_column, "Start at the top of the next column");
                    ui.add_space(2.0);
                    ui.label(RichText::new("Colour:").color(Color32::from_gray(210)));
                    let suggested = look.kind_color(s.kind).or(s.kind.tint().map(chart::rgb)).unwrap_or([255, 242, 204]);
                    optional_color(ui, &mut s.color, suggested, true);
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Words and chords:").color(Color32::from_gray(210)));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("Transpose\u{2026}").on_hover_text("Move this section's chords to another key").clicked() {
                                *open_xpose = true;
                            }
                        });
                    });
                    let text_id = egui::Id::new(("chart-text", s.id));
                    ui.add(
                        egui::TextEdit::multiline(&mut s.text)
                            .id(text_id)
                            .font(FontId::monospace(13.0))
                            .desired_width(fw)
                            .desired_rows(16)
                            .hint_text("[E]Let the morning [B]come\n| E /// | B //// |"),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new("Put a chord in [brackets] just before the syllable it falls on: [E]Out along the [B]river. A line that starts with | is a line of bars: | E /// | B //// |.")
                            .size(11.0)
                            .color(Color32::from_gray(165)),
                    );
                    ui.label(RichText::new("Which chords get diagrams is chosen in the CHART tab.").size(11.0).color(Color32::from_gray(165)));
                }
            });
        });
    }

    /// The Chord Diagrams window: every chord in the song as a diagram, worked
    /// out from its name; click one to show it at the end of the chart or leave
    /// it out. It can be resized and scrolls when the song has many chords.
    fn diagrams_window(&mut self, ctx: &egui::Context) {
        let Some(st) = &mut self.chart else { return };
        if !st.diagrams_open {
            return;
        }
        let d = &mut st.doc;
        let mut open = true;
        let mut done = false;
        let screen = ctx.content_rect();
        egui::Window::new("Chord Diagrams")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .pivot(Align2::CENTER_CENTER)
            .default_pos(screen.center())
            .default_size(Vec2::new(470.0, 420.0))
            .min_size(Vec2::new(300.0, 240.0))
            .constrain(true)
            .show(ctx, |ui| {
                ui.checkbox(&mut d.look.show_diagrams, "Print chord diagrams at the end of the chart");
                let choices = d.diagram_choices();
                ui.label(RichText::new("Each diagram is worked out from the chord's name. Click a chord to show or leave out its diagram.").size(11.5).color(Color32::from_gray(175)));
                ui.horizontal(|ui| {
                    if ui.button("All").on_hover_text("A diagram for every chord in the song").clicked() {
                        d.diagrams = d.chords_used();
                    }
                    if ui.button("None").clicked() {
                        d.diagrams.clear();
                    }
                    ui.label(RichText::new(format!("{} of {} shown", d.diagrams.len(), choices.len())).color(Color32::from_gray(185)));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(egui::Button::new(RichText::new("Done").color(Color32::WHITE)).fill(BLUE)).clicked() {
                            done = true;
                        }
                    });
                });
                ui.separator();
                if choices.is_empty() {
                    ui.label("The song has no chords yet.");
                    return;
                }
                let mut toggle: Option<(String, bool)> = None;
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
                        for (name, in_song) in &choices {
                            let on = d.diagrams.contains(name);
                            let (r, resp) = ui.allocate_exact_size(Vec2::new(60.0, 86.0), Sense::click());
                            let p = ui.painter();
                            p.rect_filled(r, 4.0, Color32::from_gray(246));
                            chart::draw_diagrams(p, std::slice::from_ref(name), egui::Pos2::ZERO, r.min + Vec2::new(2.0, 3.0), 1.0);
                            if !on {
                                // Left out: greyed.
                                p.rect_filled(r, 4.0, Color32::from_rgba_unmultiplied(40, 43, 50, 165));
                            }
                            let edge = if on { egui::Stroke::new(2.0, BLUE) } else if resp.hovered() { egui::Stroke::new(1.5, Color32::from_gray(200)) } else { egui::Stroke::new(1.0, Color32::from_gray(90)) };
                            p.rect_stroke(r, 4.0, edge, egui::StrokeKind::Inside);
                            if on {
                                let c = egui::Pos2::new(r.right() - 8.0, r.top() + 8.0);
                                p.circle_filled(c, 6.5, BLUE);
                                p.line_segment([c + Vec2::new(-3.0, 0.0), c + Vec2::new(-1.0, 2.5)], egui::Stroke::new(1.6, Color32::WHITE));
                                p.line_segment([c + Vec2::new(-1.0, 2.5), c + Vec2::new(3.0, -2.5)], egui::Stroke::new(1.6, Color32::WHITE));
                            }
                            let mut tip = if on { format!("{name}: shown \u{2013} click to leave it out") } else { format!("{name}: click to show its diagram") };
                            if !in_song {
                                tip.push_str("\n(The song no longer uses this chord.)");
                            }
                            if chart::shape(name).is_none() {
                                tip.push_str("\n(The name can't be read as a chord, or no hand could play it: an empty grid is printed.)");
                            }
                            if resp.on_hover_text(tip).clicked() {
                                toggle = Some((name.clone(), !on));
                            }
                        }
                    });
                });
                if let Some((name, on)) = toggle {
                    d.set_diagram(&name, on);
                }
            });
        if !open || done || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            st.diagrams_open = false;
        }
    }

    /// Opens the time (`time`) or key signature editor on the chart's values.
    pub(super) fn open_chart_sig(&mut self, time: bool) {
        let Some(st) = &mut self.chart else { return };
        st.sig = Some(if time {
            let (num, den) = chart_time(&st.doc.time);
            ChartSig::Time { num, den }
        } else {
            match chart_key(&st.doc.key) {
                Some((key, minor)) => ChartSig::Key { key, minor, none: false },
                None => ChartSig::Key { key: 0, minor: false, none: true },
            }
        });
    }

    /// The score's Time Signature and Key Signature popups, for the chart's
    /// time and key (which hold for the whole chart).
    fn chart_sig_window(&mut self, ctx: &egui::Context) {
        let music_font = self.music_font;
        let Some(st) = &mut self.chart else { return };
        let Some(mut sig) = st.sig.take() else { return };
        let (mut ok, mut cancel) = (false, false);
        let title = match sig {
            ChartSig::Time { .. } => "Time Signature",
            ChartSig::Key { .. } => "Key Signature",
        };
        egui::Window::new(title).collapsible(false).resizable(false).anchor(Align2::CENTER_TOP, Vec2::new(0.0, 140.0)).show(ctx, |ui| {
            ui.set_min_width(300.0);
            match &mut sig {
                ChartSig::Time { num, den } => {
                    ui.label(RichText::new("The whole chart").weak());
                    super::popups::time_sig_body(ui, num, den);
                }
                ChartSig::Key { key, minor, none } => {
                    ui.label(RichText::new("KEY SIGNATURE").strong().size(16.0));
                    ui.label(RichText::new("The whole chart").weak());
                    ui.add_enabled_ui(!*none, |ui| super::popups::key_sig_body(ui, key, minor, false, music_font));
                    ui.checkbox(none, "Don't show a key on the chart");
                }
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.button(RichText::new("  OK  ").strong()).clicked() {
                    ok = true;
                }
                if ui.button("Cancel").clicked() {
                    cancel = true;
                }
            });
        });
        if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
            ok = true;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            cancel = true;
        }
        if ok {
            match sig {
                ChartSig::Time { num, den } => st.doc.time = format!("{num}/{den}"),
                ChartSig::Key { key, minor, none } => st.doc.key = if none { String::new() } else { super::popups::key_short(key, minor) },
            }
        } else if !cancel {
            st.sig = Some(sig);
        }
    }

    /// The chart's tempo: the score's Automations editor, with the chart's
    /// sections in playing order instead of bars.
    fn chart_tempo_window(&mut self, ctx: &egui::Context) {
        let Some(st) = &mut self.chart else { return };
        if !st.tempo_open {
            return;
        }
        let doc = &mut st.doc;
        let names: Vec<String> = doc.map.iter().map(|e| doc.section(e.section).map(|s| s.short()).unwrap_or_default()).collect();
        let label = |b: usize| names.get(b).cloned().unwrap_or_else(|| (b + 1).to_string());
        let axis = super::automation::TempoAxis { unit: "Section", n: doc.map.len().max(1), label: &label, cursor: None };
        let pts = doc.tempo_points();
        let mut result = None;
        let mut open = true;
        egui::Window::new("Automations")
            .open(&mut open)
            .default_size(Vec2::new(820.0, 300.0))
            .anchor(Align2::CENTER_TOP, Vec2::new(0.0, 120.0))
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                result = super::automation::tempo_editor(ui, &mut st.tempo_ed, &pts, &axis);
            });
        if let Some(new) = result {
            doc.set_tempo_points(&new);
        }
        if !open || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            st.tempo_open = false;
        }
    }

    fn chart_pages(&mut self, ui: &mut Ui) {
        let Some(st) = &mut self.chart else { return };
        // Ctrl + mouse wheel zooms.
        if ui.rect_contains_pointer(ui.max_rect()) {
            let zd = ui.input(|i| i.zoom_delta());
            if (zd - 1.0).abs() > 0.001 {
                st.zoom = (st.zoom * zd).clamp(0.4, 3.0);
            }
        }
        let pages = {
            let p = ui.painter();
            chart::layout(&st.doc, &chart::painter_measure(p))
        };
        st.page_count = pages.len().max(1);
        let z = st.zoom;
        let gap = 24.0;
        let avail = ui.available_size();
        let mut clicked: Option<usize> = None;
        egui::ScrollArea::both().id_salt("chart-pages").auto_shrink([false, false]).show(ui, |ui| {
            let total = Vec2::new((chart::PAGE_W * z + 2.0 * gap).max(avail.x), pages.len() as f32 * (chart::PAGE_H * z + gap) + gap);
            let (rect, resp) = ui.allocate_exact_size(total, Sense::click());
            let p = ui.painter();
            let mut hits = vec![];
            for k in 0..pages.len() {
                let o = egui::Pos2::new(rect.center().x - chart::PAGE_W * z * 0.5, rect.top() + gap + k as f32 * (chart::PAGE_H * z + gap));
                hits.extend(chart::draw_page(p, &st.doc, &pages, k, o, z, Some(st.sel), false));
            }
            if resp.clicked()
                && let Some(pp) = resp.interact_pointer_pos()
            {
                clicked = hits.iter().find(|(_, r)| r.expand(4.0).contains(pp)).map(|(i, _)| *i);
            }
        });
        if let Some(i) = clicked {
            st.sel = i;
            st.tab = 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chart_key_and_time_read_as_signatures() {
        assert_eq!(chart_key("E"), Some((4, false)));
        assert_eq!(chart_key("C#m"), Some((4, true)));
        assert_eq!(chart_key("F#"), Some((6, false)));
        assert_eq!(chart_key("Gb"), Some((-6, false)));
        assert_eq!(chart_key("Bb major"), Some((-2, false)));
        assert_eq!(chart_key("D minor"), Some((-1, true)));
        assert_eq!(chart_key(""), None);
        assert_eq!(super::super::popups::key_short(4, false), "E");
        assert_eq!(super::super::popups::key_short(-3, true), "Cm");
        assert_eq!(chart_time("6/8"), (6, 8));
        assert_eq!(chart_time("Free"), (4, 4));
    }
}
