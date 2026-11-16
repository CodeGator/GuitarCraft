//! The chord chart's stylesheet (Chart > Stylesheet, F7): the score
//! stylesheet's idea for chord charts. Four tabs (Page & Columns, Header &
//! Footer, Sections, Chords & Lyrics), ready-made Standard and Stage styles,
//! saving and loading styles, and a style for new charts. Changes are made on
//! a copy and reach the chart with Apply or OK.

use super::App;
use super::home::{app_dir, files_with_ext};
use super::stylesheet::{BLUE, BODY_BG, choice, heading, row, subheading, switch, tab_row};
use crate::chart::{self, ChartLook, TextLook};
use crate::style::{FONTS, STYLE_NAMES};
use egui::{Align, Align2, Color32, FontFamily, FontId, Layout, Pos2, Rect, RichText, Stroke, Ui, Vec2};
use std::path::PathBuf;

pub const TABS: [&str; 4] = ["Page & Columns", "Header & Footer", "Sections", "Chords & Lyrics"];

/// The window's state: the copy being edited and what is showing.
pub struct ChartStyleDlg {
    pub tab: usize,
    pub draft: ChartLook,
    /// Give every section its kind's colour from this style when applied.
    pub recolor: bool,
    pub note: String,
}

fn styles_dir() -> Option<PathBuf> {
    Some(app_dir()?.join("Chart Styles"))
}

fn default_file() -> Option<PathBuf> {
    Some(app_dir()?.join("default-chart-style.json"))
}

fn read_style(p: &std::path::Path) -> Result<ChartLook, String> {
    let d = std::fs::read(p).map_err(|e| e.to_string())?;
    let mut s = serde_json::from_slice::<ChartLook>(&d).map_err(|e| format!("Not a chart stylesheet: {e}"))?;
    s.fix_kind_colors();
    Ok(s)
}

fn write_style(p: &std::path::Path, s: &ChartLook) -> Result<(), String> {
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let json = serde_json::to_string_pretty(s).map_err(|e| e.to_string())?;
    std::fs::write(p, json).map_err(|e| e.to_string())
}

/// The stylesheet new charts start with (the standard one unless the person chose another).
pub fn default_chart_style() -> ChartLook {
    default_file().and_then(|f| read_style(&f).ok()).unwrap_or_else(ChartLook::standard)
}

fn saved_styles() -> Vec<PathBuf> {
    files_with_ext(styles_dir(), "json")
}

/// A kind of text: font with a preview, size, style and colour.
fn text_row(ui: &mut Ui, label: &str, t: &mut TextLook) {
    row(ui, label, |ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        let fam = if FONTS.iter().any(|x| x.0 == t.family) { t.family.clone() } else { "Arial".to_string() };
        let style = t.bold as usize + 2 * t.italic as usize;
        // A preview of the font in its colour, on paper white.
        ui.label(RichText::new(" Aa ").font(FontId::new(14.0, FontFamily::Name(format!("{fam}|{style}").into()))).color(t.ink()).background_color(Color32::WHITE));
        egui::ComboBox::from_id_salt((label, "fam")).selected_text(t.family.clone()).width(135.0).show_ui(ui, |ui| {
            for (name, _) in FONTS {
                let fid = FontId::new(14.0, FontFamily::Name(format!("{name}|0").into()));
                if ui.selectable_label(t.family == name, RichText::new(name).font(fid)).clicked() {
                    t.family = name.to_string();
                }
            }
        });
        ui.add(egui::DragValue::new(&mut t.size).range(4.0..=72.0).speed(0.1).fixed_decimals(1).suffix(" pt"));
        let mut k = style as u8;
        choice(ui, &format!("{label}-style"), &mut k, &STYLE_NAMES, 90.0);
        t.bold = k & 1 != 0;
        t.italic = k & 2 != 0;
        ui.color_edit_button_srgb(&mut t.color).on_hover_text("Colour");
    });
}

fn note(ui: &mut Ui, t: &str) {
    ui.label(RichText::new(t).size(12.0).color(Color32::from_gray(160)));
}

fn page_tab(ui: &mut Ui, s: &mut ChartLook) {
    heading(ui, "COLUMNS");
    row(ui, "Columns of text", |ui| {
        ui.add(egui::DragValue::new(&mut s.columns).range(1..=chart::MAX_COLUMNS));
    });
    note(ui, "Lines run down a column, then on to the next column and the next page. Long lines wrap to the column's width.");
    heading(ui, "PRINTED PARTS");
    row(ui, "Map under the title", |ui| {
        switch(ui, &mut s.show_map);
    });
    row(ui, "Chord diagrams", |ui| {
        switch(ui, &mut s.show_diagrams);
    });
    note(ui, "The diagrams' chords are chosen in the CHART tab.");
}

fn header_tab(ui: &mut Ui, s: &mut ChartLook) {
    heading(ui, "HEADER");
    text_row(ui, "Title", &mut s.title);
    text_row(ui, "Song details", &mut s.info);
    note(ui, "Artist, capo, tuning, BPM, time and key.");
    text_row(ui, "Map", &mut s.map);
    heading(ui, "FOOTER");
    text_row(ui, "Copyright", &mut s.copyright);
    note(ui, "The copyright's words are written in the CHART tab.");
}

fn sections_tab(ui: &mut Ui, dlg: &mut ChartStyleDlg) {
    let s = &mut dlg.draft;
    heading(ui, "TEXTS");
    text_row(ui, "Section names", &mut s.heading);
    text_row(ui, "Repeat lines", &mut s.repeat);
    heading(ui, "COLOURS");
    row(ui, "Shade lines with the section's colour", |ui| {
        switch(ui, &mut s.shade);
    });
    subheading(ui, "Colour of a new section, by kind");
    note(ui, "Any section's own colour can still be changed in the SECTION tab.");
    s.fix_kind_colors();
    for (kind, c) in s.kind_colors.iter_mut() {
        row(ui, kind.label(), |ui| {
            let mut on = c.is_some();
            if ui.checkbox(&mut on, "").changed() {
                *c = if on { Some(kind.tint().map(chart::rgb).unwrap_or([255, 242, 204])) } else { None };
            }
            match c {
                Some(v) => {
                    ui.color_edit_button_srgb(v);
                }
                None => {
                    ui.label(RichText::new("No colour").color(Color32::from_gray(150)));
                }
            }
        });
    }
    ui.add_space(6.0);
    row(ui, "", |ui| {
        ui.checkbox(&mut dlg.recolor, "Also give every section of the chart these colours");
    });
}

fn chords_tab(ui: &mut Ui, s: &mut ChartLook) {
    heading(ui, "TEXTS");
    text_row(ui, "Chord names", &mut s.chords);
    text_row(ui, "Lyrics", &mut s.lyrics);
    text_row(ui, "Bar lines", &mut s.bars);
    note(ui, "On a dimly lit stage, chord names larger, bolder or in their own colour stand apart from the words.");
}

/// The tabs with their pictures.
fn tab_strip(ui: &mut Ui, tab: &mut usize) {
    tab_row(ui, &TABS, "cs-tab", tab, |p, i, ic, c| {
        let st = Stroke::new(1.3, c);
        let thin = Stroke::new(1.0, c);
        let pr = Rect::from_center_size(ic, Vec2::new(18.0, 22.0));
        match i {
            0 => {
                // A page with two columns of lines.
                p.rect_stroke(pr, 1.0, st, egui::StrokeKind::Middle);
                for k in 0..4 {
                    let y = pr.top() + 5.0 + k as f32 * 3.8;
                    p.line_segment([Pos2::new(pr.left() + 3.0, y), Pos2::new(pr.center().x - 1.5, y)], thin);
                    p.line_segment([Pos2::new(pr.center().x + 1.5, y), Pos2::new(pr.right() - 3.0, y)], thin);
                }
            }
            1 => {
                // A page with a header and a footer bar.
                p.rect_stroke(pr, 2.0, st, egui::StrokeKind::Middle);
                p.line_segment([Pos2::new(pr.left() + 3.0, pr.top() + 4.0), Pos2::new(pr.right() - 3.0, pr.top() + 4.0)], Stroke::new(1.8, c));
                p.line_segment([Pos2::new(pr.left() + 3.0, pr.bottom() - 4.0), Pos2::new(pr.right() - 3.0, pr.bottom() - 4.0)], Stroke::new(1.0, c));
            }
            2 => {
                // Coloured bands under a heading.
                p.line_segment([Pos2::new(pr.left(), pr.top() + 2.0), Pos2::new(pr.left() + 9.0, pr.top() + 2.0)], Stroke::new(2.0, Color32::from_rgb(200, 150, 255)));
                for (k, col) in [Color32::from_rgb(244, 204, 204), Color32::from_rgb(255, 242, 204), Color32::from_rgb(208, 224, 227)].iter().enumerate() {
                    let y = pr.top() + 7.0 + k as f32 * 5.0;
                    p.rect_filled(Rect::from_min_size(Pos2::new(pr.left(), y), Vec2::new(18.0, 3.5)), 0.0, *col);
                }
            }
            _ => {
                // A chord name over a word.
                p.text(Pos2::new(ic.x, ic.y - 6.0), Align2::CENTER_CENTER, "Am", FontId::new(13.0, FontFamily::Name("Arial|1".into())), c);
                p.line_segment([Pos2::new(pr.left(), ic.y + 6.0), Pos2::new(pr.right(), ic.y + 6.0)], thin);
                p.line_segment([Pos2::new(pr.left(), ic.y + 10.0), Pos2::new(pr.right() - 5.0, ic.y + 10.0)], thin);
            }
        }
    });
}

impl App {
    pub(super) fn open_chart_style(&mut self, tab: usize) {
        if let Some(st) = &mut self.chart {
            let mut draft = st.doc.look.clone();
            draft.fix_kind_colors();
            st.style_dlg = Some(ChartStyleDlg { tab, draft, recolor: false, note: String::new() });
        }
    }

    /// The window; Apply and OK change the chart (one undo step each).
    pub(super) fn chart_style_window(&mut self, ctx: &egui::Context) {
        let shot = self.screenshot.is_some();
        let Some(st) = &mut self.chart else { return };
        let Some(mut dlg) = st.style_dlg.take() else { return };
        let mut open = true;
        let mut action: Option<u8> = None; // 0 apply, 1 cancel, 2 ok
        let screen = ctx.content_rect();
        let h = (screen.height() - 60.0).clamp(360.0, 820.0);
        egui::Window::new("Chord Chart Stylesheet")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .pivot(Align2::CENTER_CENTER)
            .default_pos(screen.center())
            .default_size(Vec2::new(720.0, h))
            .min_size(Vec2::new(420.0, 300.0))
            .constrain(true)
            .frame(egui::Frame::window(&ctx.global_style()).fill(BODY_BG).inner_margin(0.0))
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = 6.0;
                egui::Panel::top("cs-tabs").frame(egui::Frame::NONE).resizable(false).show(ui, |ui| tab_strip(ui, &mut dlg.tab));
                egui::Panel::bottom("cs-bar").frame(egui::Frame::new().fill(BODY_BG).inner_margin(egui::Margin::symmetric(14, 10))).resizable(false).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.menu_button(RichText::new("\u{2699} Options \u{25B4}\u{25BE}").size(14.0), |ui| {
                            if ui.button("Apply Standard style").clicked() {
                                dlg.draft = ChartLook::standard();
                                dlg.note = "Standard \u{2013} press Apply or OK to use it.".into();
                                ui.close();
                            }
                            if ui.button("Apply Stage style (large type)").clicked() {
                                dlg.draft = ChartLook::stage();
                                dlg.note = "Stage \u{2013} press Apply or OK to use it.".into();
                                ui.close();
                            }
                            let saved = saved_styles();
                            if !saved.is_empty() {
                                ui.separator();
                                for p in saved {
                                    let name = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                                    if ui.button(format!("Apply \u{201C}{name}\u{201D}")).clicked() {
                                        match read_style(&p) {
                                            Ok(s) => {
                                                dlg.draft = s;
                                                dlg.note = format!("Style \u{201C}{name}\u{201D} loaded.");
                                            }
                                            Err(e) => dlg.note = e,
                                        }
                                        ui.close();
                                    }
                                }
                            }
                            ui.separator();
                            if ui.button("Load Style...").clicked() {
                                ui.close();
                                if !shot {
                                    let mut d = rfd::FileDialog::new().add_filter("Chord chart stylesheet", &["json"]);
                                    if let Some(dir) = styles_dir() {
                                        d = d.set_directory(dir);
                                    }
                                    if let Some(p) = d.pick_file() {
                                        match read_style(&p) {
                                            Ok(s) => {
                                                dlg.draft = s;
                                                dlg.note = "Style loaded \u{2013} press Apply or OK to use it.".into();
                                            }
                                            Err(e) => dlg.note = e,
                                        }
                                    }
                                }
                            }
                            if ui.button("Save Style...").clicked() {
                                ui.close();
                                if !shot {
                                    let mut d = rfd::FileDialog::new().add_filter("Chord chart stylesheet", &["json"]).set_file_name("My chart style.json");
                                    if let Some(dir) = styles_dir() {
                                        let _ = std::fs::create_dir_all(&dir);
                                        d = d.set_directory(dir);
                                    }
                                    if let Some(p) = d.save_file() {
                                        dlg.note = match write_style(&p, &dlg.draft) {
                                            Ok(()) => format!("Saved {}.", p.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default()),
                                            Err(e) => format!("Couldn't save the style: {e}"),
                                        };
                                    }
                                }
                            }
                            ui.separator();
                            if ui.button("Use for new charts").on_hover_text("New chord charts will start with this style.").clicked() {
                                ui.close();
                                if !shot && let Some(f) = default_file() {
                                    dlg.note = match write_style(&f, &dlg.draft) {
                                        Ok(()) => "New charts will use this style.".into(),
                                        Err(e) => format!("Couldn't keep the style: {e}"),
                                    };
                                }
                            }
                        });
                        ui.label(RichText::new(&dlg.note).size(12.0).color(Color32::from_gray(170)));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let big = |t: &str| RichText::new(t).size(14.0);
                            if ui.add(egui::Button::new(big("Apply")).min_size(Vec2::new(64.0, 28.0))).clicked() {
                                action = Some(0);
                            }
                            if ui.add(egui::Button::new(big("Cancel")).min_size(Vec2::new(70.0, 28.0))).clicked() {
                                action = Some(1);
                            }
                            if ui.add(egui::Button::new(big("OK").color(Color32::WHITE)).fill(BLUE).min_size(Vec2::new(54.0, 28.0))).clicked() {
                                action = Some(2);
                            }
                        });
                    });
                });
                egui::CentralPanel::default().frame(egui::Frame::NONE).show(ui, |ui| {
                    ui.spacing_mut().scroll = egui::style::ScrollStyle::solid();
                    egui::ScrollArea::both()
                        .id_salt(("cs-body", dlg.tab))
                        .auto_shrink([false, false])
                        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded)
                        .show(ui, |ui| {
                            egui::Frame::new().inner_margin(egui::Margin { left: 40, right: 24, top: 4, bottom: 12 }).show(ui, |ui| match dlg.tab {
                                0 => page_tab(ui, &mut dlg.draft),
                                1 => header_tab(ui, &mut dlg.draft),
                                2 => sections_tab(ui, &mut dlg),
                                _ => chords_tab(ui, &mut dlg.draft),
                            });
                        });
                });
            });
        let apply = |st: &mut super::chartui::ChartState, dlg: &ChartStyleDlg| {
            st.doc.look = dlg.draft.clone();
            if dlg.recolor {
                for s in &mut st.doc.sections {
                    s.color = dlg.draft.kind_color(s.kind);
                }
            }
        };
        match action {
            Some(0) => {
                apply(st, &dlg);
                dlg.recolor = false;
                dlg.note = "Applied.".into();
            }
            Some(2) => {
                apply(st, &dlg);
                self.status = "Stylesheet applied.".into();
                return;
            }
            Some(_) => return,
            None => {}
        }
        if open && !ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            st.style_dlg = Some(dlg);
        }
    }
}
