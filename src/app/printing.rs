//! Printing (File > Print, Ctrl+P) and exporting the score as PDF or PNG
//! (File > Export), as the user guide describes: the printer's own dialog
//! chooses the printer, pages and copies; the export window has the file
//! name (with naming keywords), folder, resolution, pages or selection, a
//! transparent background for PNG, and opening the folder afterwards.

use super::App;
use crate::render::{self, PrintSpec};
use egui::{Align2, Color32, RichText, Vec2};
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq)]
pub enum ExportKind {
    Pdf,
    Png,
}

impl ExportKind {
    fn ext(self) -> &'static str {
        match self {
            ExportKind::Pdf => "pdf",
            ExportKind::Png => "png",
        }
    }
}

/// The print preview's state.
pub struct PrintPreview {
    pub page: usize,
    /// Fit the page width (otherwise the whole page).
    pub fit_width: bool,
    pub zoom: f32,
    /// The zoom was set by hand (not by a fit button).
    pub fixed: bool,
}

pub struct ExportDlg {
    pub kind: ExportKind,
    pub name: String,
    pub folder: PathBuf,
    pub dpi: u32,
    /// 0 all pages, 1 a range of pages, 2 the selection.
    pub scope: u8,
    pub from: usize,
    pub to: usize,
    pub transparent: bool,
    pub open_folder: bool,
}

/// Fills in the naming keywords of the export window.
fn fill_name(pattern: &str, title: &str, track: &str, track_no: usize, tracks: usize) -> String {
    let now = local_now();
    let s = pattern
        .replace("%T", title)
        .replace("%t", track)
        .replace("%n", &track_no.to_string())
        .replace("%N", &tracks.to_string())
        .replace("%d", &now.0)
        .replace("%h", &now.1);
    let s = super::widgets::sanitize(&s);
    if s.trim().is_empty() { "Untitled".into() } else { s }
}

/// Today's date (2026-11-14) and the time (13-03), from the system clock.
fn local_now() -> (String, String) {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    // UTC; good enough for a file name.
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (y, m, d) = civil(days);
    (format!("{y:04}-{m:02}-{d:02}"), format!("{:02}-{:02}", rem / 3600, (rem % 3600) / 60))
}

/// Days since 1970-01-01 to a calendar date.
fn civil(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + if m <= 2 { 1 } else { 0 }, m, d)
}

impl App {
    fn print_spec(&self) -> PrintSpec<'_> {
        PrintSpec { song: &self.song, shown: self.shown_tracks(), track: self.track, notation: self.notation }
    }

    /// The number of pages of what's open (a score or a chord chart).
    fn doc_pages(&self) -> usize {
        match &self.chart {
            Some(st) => st.page_count.max(1),
            None => self.print_spec().pages(),
        }
    }

    /// The page size of what's open, in millimetres.
    fn doc_mm(&self) -> (f32, f32) {
        match &self.chart {
            Some(_) => crate::chart::PAGE_MM,
            None => self.print_spec().page_mm(),
        }
    }

    fn doc_title(&self) -> String {
        match &self.chart {
            Some(st) => st.doc.title.clone(),
            None => self.song.title.clone(),
        }
    }

    /// Pictures of the given pages of what's open.
    fn doc_render(&self, pages: &[usize], dpi: f32, transparent: bool) -> Vec<render::PageImage> {
        match &self.chart {
            Some(st) => render::render_chart(&st.doc, pages, dpi),
            None => render::render_pages(&self.print_spec(), pages, dpi, transparent),
        }
    }

    pub(super) fn open_export(&mut self, kind: ExportKind) {
        let pages = self.doc_pages();
        let folder = self
            .path
            .as_ref()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
            .or_else(|| self.recent.first().and_then(|p| p.parent().map(|d| d.to_path_buf())))
            .or_else(|| std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")).map(|h| PathBuf::from(h).join("Documents")))
            .unwrap_or_default();
        self.export_dlg = Some(ExportDlg {
            kind,
            name: "%T".into(),
            folder,
            dpi: if kind == ExportKind::Pdf { 300 } else { 150 },
            scope: 0,
            from: 1,
            to: pages,
            transparent: false,
            open_folder: false,
        });
    }

    pub(super) fn export_window(&mut self, ctx: &egui::Context) {
        let Some(mut dlg) = self.export_dlg.take() else { return };
        let pages = self.doc_pages();
        let charting = self.chart.is_some();
        let has_sel = !charting && self.selection().is_some();
        let file_name = if charting {
            format!("{}.{}", fill_name(&dlg.name, &self.doc_title(), "", 1, 1), dlg.kind.ext())
        } else {
            format!(
                "{}.{}",
                fill_name(&dlg.name, &self.song.title, &self.tr().name, self.track + 1, self.song.tracks.len()),
                dlg.kind.ext()
            )
        };
        let mut open = true;
        let (mut go, mut cancel) = (false, false);
        let title = match dlg.kind {
            ExportKind::Pdf => "Export PDF",
            ExportKind::Png => "Export PNG",
        };
        egui::Window::new(title)
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.set_width(420.0);
                egui::Grid::new("export").num_columns(2).spacing(Vec2::new(10.0, 8.0)).show(ui, |ui| {
                    ui.label("File name:");
                    ui.horizontal(|ui| {
                        ui.add(egui::TextEdit::singleline(&mut dlg.name).desired_width(250.0));
                        ui.label(RichText::new("\u{24D8}").size(15.0)).on_hover_text(
                            "Keywords for naming the file:\n%T  score title\n%t  track name\n%n  track number\n%N  number of tracks\n%d  date\n%h  time",
                        );
                    });
                    ui.end_row();
                    ui.label("");
                    ui.label(RichText::new(&file_name).weak());
                    ui.end_row();
                    ui.label("Folder:");
                    ui.horizontal(|ui| {
                        let shown = dlg.folder.display().to_string();
                        ui.add(egui::Label::new(RichText::new(&shown)).truncate()).on_hover_text(&shown);
                        if ui.button("Choose\u{2026}").clicked() && self.screenshot.is_none() {
                            if let Some(d) = rfd::FileDialog::new().set_directory(&dlg.folder).pick_folder() {
                                dlg.folder = d;
                            }
                        }
                    });
                    ui.end_row();
                    ui.label("Resolution:");
                    egui::ComboBox::from_id_salt("export-dpi").selected_text(format!("{} dpi", dlg.dpi)).show_ui(ui, |ui| {
                        for d in [72u32, 150, 300, 600] {
                            ui.selectable_value(&mut dlg.dpi, d, format!("{d} dpi"));
                        }
                    });
                    ui.end_row();
                    ui.label("Export:");
                    ui.vertical(|ui| {
                        ui.radio_value(&mut dlg.scope, 0, format!("All pages ({pages})"));
                        ui.horizontal(|ui| {
                            ui.radio_value(&mut dlg.scope, 1, "Pages");
                            ui.add_enabled_ui(dlg.scope == 1, |ui| {
                                ui.add(egui::DragValue::new(&mut dlg.from).range(1..=pages));
                                ui.label("to");
                                ui.add(egui::DragValue::new(&mut dlg.to).range(1..=pages));
                            });
                        });
                        ui.add_enabled_ui(has_sel, |ui| {
                            if charting {
                                return;
                            }
                            ui.radio_value(&mut dlg.scope, 2, "Selection").on_disabled_hover_text("Select some beats on the score first.");
                        });
                    });
                    ui.end_row();
                    if dlg.kind == ExportKind::Png && !charting {
                        ui.label("");
                        ui.checkbox(&mut dlg.transparent, "Transparent background");
                        ui.end_row();
                    }
                    ui.label("");
                    ui.checkbox(&mut dlg.open_folder, "Open the folder after exporting");
                    ui.end_row();
                });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Cancel").clicked() {
                            cancel = true;
                        }
                        if ui.add(egui::Button::new(RichText::new("Export").color(Color32::WHITE)).fill(super::panels::BLUE)).clicked() {
                            go = true;
                        }
                    });
                });
            });
        if go {
            let list: Vec<usize> = match dlg.scope {
                1 => {
                    let (a, b) = (dlg.from.min(dlg.to).max(1), dlg.from.max(dlg.to).min(pages));
                    (a - 1..b).collect()
                }
                2 => match self.selection() {
                    Some((a, b)) => {
                        let (p0, p1) = self.print_spec().pages_of_bars(a.0, b.0);
                        (p0..=p1).collect()
                    }
                    None => (0..pages).collect(),
                },
                _ => (0..pages).collect(),
            };
            let path = dlg.folder.join(&file_name);
            self.export_pages(dlg.kind, &path, &list, dlg.dpi as f32, dlg.transparent, dlg.open_folder);
            return;
        }
        if open && !cancel {
            self.export_dlg = Some(dlg);
        }
    }

    fn export_pages(&mut self, kind: ExportKind, path: &std::path::Path, pages: &[usize], dpi: f32, transparent: bool, open_folder: bool) {
        if self.screenshot.is_some() {
            return;
        }
        let imgs = self.doc_render(pages, dpi, transparent && kind == ExportKind::Png);
        let (w, h) = self.doc_mm();
        let r = match kind {
            ExportKind::Pdf => std::fs::write(path, render::pdf(&imgs, w, h, &self.doc_title())).map_err(|e| e.to_string()),
            ExportKind::Png => render::stack(&imgs).save(path).map_err(|e| e.to_string()),
        };
        self.status = match r {
            Ok(()) => format!("Exported {} page(s) to {}.", pages.len(), path.display()),
            Err(e) => format!("Couldn't export: {e}"),
        };
        if open_folder {
            show_in_folder(path);
        }
    }

    /// File > Print (Ctrl+P): opens the print preview, as Guitar Pro 8 does.
    pub(super) fn print_song(&mut self) {
        self.print_preview = Some(PrintPreview { page: 0, fit_width: true, zoom: 1.0, fixed: false });
    }

    /// The print preview: the pages as they will print, with fit buttons, zoom,
    /// page navigation and a Print button (laid out like the person's picture
    /// of the original).
    pub(super) fn print_preview_window(&mut self, ctx: &egui::Context) {
        let Some(mut pv) = self.print_preview.take() else { return };
        let pages = self.doc_pages().max(1);
        pv.page = pv.page.min(pages - 1);
        let mut open = true;
        let mut print = false;
        let screen = ctx.content_rect();
        let name = self
            .path
            .as_ref()
            .and_then(|p| p.file_name().map(|f| f.to_string_lossy().to_string()))
            .unwrap_or_else(|| self.doc_title());
        let size = Vec2::new((screen.width() * 0.7).clamp(600.0, 1000.0), (screen.height() - 40.0).max(400.0));
        let bar_bg = Color32::from_rgb(38, 41, 50);
        let blue = super::panels::BLUE;
        // A window of its own, so it can be moved and resized beyond the main
        // window, as in Guitar Pro. (Pictures taken for checks keep it inside.)
        if self.screenshot.is_some() {
            ctx.set_embed_viewports(true);
        }
        let builder = egui::ViewportBuilder::default()
            .with_title(format!("Print Preview: {name} - GuitarCraft"))
            .with_inner_size(size)
            .with_min_inner_size([480.0, 360.0]);
        let page_bg = Color32::from_rgb(27, 29, 34);
        let closed = ctx.show_viewport_immediate(egui::ViewportId::from_hash_of("print-preview"), builder, |ui, class| {
                ui.painter().rect_filled(ui.max_rect(), 0.0, page_bg);
                ui.spacing_mut().item_spacing.y = 0.0;
                // ---- toolbar
                egui::Frame::new().fill(bar_bg).inner_margin(egui::Margin::symmetric(10, 7)).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;
                        for (fw, tip) in [(false, "Fit the whole page"), (true, "Fit the page width")] {
                            let (r, resp) = ui.allocate_exact_size(Vec2::new(24.0, 22.0), egui::Sense::click());
                            let on = pv.fit_width == fw;
                            let p = ui.painter();
                            p.rect_filled(r, 3.0, if on { blue } else if resp.hovered() { Color32::from_gray(80) } else { Color32::from_rgb(56, 60, 70) });
                            let c = Color32::WHITE;
                            let st = egui::Stroke::new(1.3, c);
                            let q = r.shrink(5.0);
                            if fw {
                                // A page with arrows across it.
                                p.line_segment([q.left_top(), q.left_bottom()], st);
                                p.line_segment([egui::Pos2::new(q.right() - 3.0, q.top()), egui::Pos2::new(q.right() - 3.0, q.bottom())], st);
                                p.line_segment([q.left_center(), egui::Pos2::new(q.right() - 4.0, q.center().y)], st);
                            } else {
                                // A page with arrows top to bottom.
                                p.line_segment([q.left_top(), q.right_top()], st);
                                p.line_segment([q.left_bottom(), q.right_bottom()], st);
                                p.line_segment([q.center_top(), q.center_bottom()], st);
                            }
                            if resp.on_hover_text(tip).clicked() {
                                pv.fit_width = fw;
                                pv.fixed = false;
                            }
                        }
                        ui.add_space(14.0);
                        let mut pct = (pv.zoom * 100.0).round();
                        if ui.add(egui::DragValue::new(&mut pct).range(20.0..=400.0).suffix(" %")).changed() {
                            pv.zoom = pct / 100.0;
                            pv.fit_width = false;
                            pv.fixed = true;
                        }
                        if ui.button("\u{2212}").on_hover_text("Zoom out").clicked() {
                            pv.zoom = (pv.zoom / 1.2).max(0.2);
                            pv.fixed = true;
                        }
                        if ui.button("+").on_hover_text("Zoom in").clicked() {
                            pv.zoom = (pv.zoom * 1.2).min(4.0);
                            pv.fixed = true;
                        }
                        // Page navigation, in the middle.
                        let mid = ui.available_width() * 0.5 - 150.0;
                        if mid > 0.0 {
                            ui.add_space(mid);
                        }
                        let nav = |ui: &mut egui::Ui, on: bool, t: &str, tip: &str| ui.add_enabled(on, egui::Button::new(t)).on_hover_text(tip).clicked();
                        if nav(ui, pv.page > 0, "\u{00AB}", "First page") {
                            pv.page = 0;
                        }
                        if nav(ui, pv.page > 0, "\u{2039}", "Previous page") {
                            pv.page -= 1;
                        }
                        let mut shown = pv.page + 1;
                        if ui.add(egui::DragValue::new(&mut shown).range(1..=pages)).changed() {
                            pv.page = shown - 1;
                        }
                        ui.label(format!("/ {pages}"));
                        if nav(ui, pv.page + 1 < pages, "\u{203A}", "Next page") {
                            pv.page += 1;
                        }
                        if nav(ui, pv.page + 1 < pages, "\u{00BB}", "Last page") {
                            pv.page = pages - 1;
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.add(egui::Button::new(RichText::new("\u{1F5B6} Print").size(13.0)).min_size(Vec2::new(64.0, 24.0))).on_hover_text("Choose the printer and print (Ctrl+P)").clicked() {
                                print = true;
                            }
                        });
                    });
                });
                // ---- the page
                let avail = ui.available_size();
                if let Some(st) = &self.chart {
                    // A chord chart's page.
                    let (cw, ch) = (crate::chart::PAGE_W, crate::chart::PAGE_H);
                    if !pv.fixed {
                        pv.zoom = if pv.fit_width { (avail.x - 60.0) / cw } else { ((avail.x - 60.0) / cw).min((avail.y - 50.0) / ch) }.clamp(0.2, 4.0);
                    }
                    let z = pv.zoom;
                    let lay = crate::chart::layout(&st.doc, &crate::chart::painter_measure(ui.painter()));
                    egui::ScrollArea::both().id_salt(("pv", pv.page)).auto_shrink([false, false]).show(ui, |ui| {
                        let total = Vec2::new((cw * z).max(avail.x - 20.0) + 40.0, ch * z + 50.0);
                        let (rect, _) = ui.allocate_exact_size(total, egui::Sense::hover());
                        let origin = egui::Pos2::new(rect.center().x - cw * z * 0.5, rect.top() + 25.0);
                        let p = ui.painter();
                        p.rect_filled(egui::Rect::from_min_size(origin, Vec2::new(cw * z, ch * z)).translate(Vec2::new(3.0, 4.0)), 0.0, Color32::from_black_alpha(90));
                        crate::chart::draw_page(p, &st.doc, &lay, pv.page, origin, z, None, true);
                    });
                    let esc = ui.ctx().input(|i| i.key_pressed(egui::Key::Escape));
                    let close = class != egui::ViewportClass::EmbeddedWindow && ui.ctx().input(|i| i.viewport().close_requested());
                    return esc || close;
                }
                let base = crate::score::layout_ex(&self.song, &self.shown_tracks(), self.track, 1.0, self.notation, None, false);
                if !pv.fixed {
                    pv.zoom = if pv.fit_width {
                        (avail.x - 60.0) / base.page_w()
                    } else {
                        ((avail.x - 60.0) / base.page_w()).min((avail.y - 50.0) / base.page_h())
                    }
                    .clamp(0.2, 4.0);
                }
                let lay = crate::score::layout_ex(&self.song, &self.shown_tracks(), self.track, pv.zoom, self.notation, None, false);
                egui::ScrollArea::both().id_salt(("pv", pv.page)).auto_shrink([false, false]).show(ui, |ui| {
                    let total = Vec2::new(lay.page_w().max(avail.x - 20.0) + 40.0, lay.page_h() + 50.0);
                    let (rect, _) = ui.allocate_exact_size(total, egui::Sense::hover());
                    let origin = egui::Pos2::new(rect.center().x - lay.page_w() * 0.5, rect.top() + 25.0);
                    let page_rect = egui::Rect::from_min_size(origin, Vec2::new(lay.page_w(), lay.page_h()));
                    let p = ui.painter();
                    p.rect_filled(page_rect.translate(Vec2::new(3.0, 4.0)), 0.0, Color32::from_black_alpha(90));
                    let dc = crate::score::DrawCtx {
                        song: &self.song,
                        ti: self.track,
                        cursor: (usize::MAX, 0, 0),
                        selection: None,
                        play_tick: None,
                        music_font: self.music_font,
                        show_cursor: false,
                        voice: 0,
                        cursor_std: None,
                        printing: true,
                        transparent: false,
                    };
                    crate::score::draw_page(p, &lay, &dc, pv.page, origin);
                });
                let esc = ui.ctx().input(|i| i.key_pressed(egui::Key::Escape));
                let close = class != egui::ViewportClass::EmbeddedWindow && ui.ctx().input(|i| i.viewport().close_requested());
                esc || close
            });
        if closed {
            open = false;
        }
        if print {
            self.print_now();
            open = false;
        }
        if open {
            self.print_preview = Some(pv);
        }
    }

    /// The printer's own dialog, then the pages it asks for.
    pub(super) fn print_now(&mut self) {
        if self.screenshot.is_some() {
            return;
        }
        let pages = self.doc_pages();
        #[cfg(target_os = "windows")]
        {
            self.status = match print_windows(self, pages) {
                Ok(Some(n)) => format!("Sent {n} page(s) to the printer."),
                Ok(None) => "Printing cancelled.".into(),
                Err(e) => format!("Couldn't print: {e}"),
            };
        }
        #[cfg(not(target_os = "windows"))]
        {
            // Elsewhere: a PDF opened in the system's viewer, to print from there.
            let path = std::env::temp_dir().join(format!("{}.pdf", super::widgets::sanitize(&self.doc_title())));
            let list: Vec<usize> = (0..pages).collect();
            self.export_pages(ExportKind::Pdf, &path, &list, 300.0, false, false);
            #[cfg(target_os = "macos")]
            let _ = std::process::Command::new("open").arg(&path).spawn();
            #[cfg(not(target_os = "macos"))]
            let _ = std::process::Command::new("xdg-open").arg(&path).spawn();
            self.status = "The score opened as a PDF: print it from there.".into();
        }
    }
}

fn show_in_folder(path: &std::path::Path) {
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("explorer").arg(format!("/select,{}", path.display())).spawn();
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg("-R").arg(path).spawn();
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    let _ = std::process::Command::new("xdg-open").arg(path.parent().unwrap_or(path)).spawn();
}

/// Shows Windows' print dialog and prints the chosen pages. Ok(None) when cancelled.
#[cfg(target_os = "windows")]
fn print_windows(app: &App, pages: usize) -> Result<Option<usize>, String> {
    use windows_sys::Win32::Foundation::GlobalFree;
    use windows_sys::Win32::Graphics::Gdi::*;
    use windows_sys::Win32::Storage::Xps::*;
    use windows_sys::Win32::UI::Controls::Dialogs::*;

    let max_page = pages.clamp(1, u16::MAX as usize) as u16;
    let mut pd: PRINTDLGW = unsafe { std::mem::zeroed() };
    pd.lStructSize = std::mem::size_of::<PRINTDLGW>() as u32;
    pd.Flags = PD_RETURNDC | PD_USEDEVMODECOPIESANDCOLLATE | PD_NOSELECTION;
    pd.nMinPage = 1;
    pd.nMaxPage = max_page;
    pd.nFromPage = 1;
    pd.nToPage = max_page;
    pd.nCopies = 1;
    if unsafe { PrintDlgW(&mut pd) } == 0 {
        return Ok(None);
    }
    let hdc = pd.hDC;
    let free = |pd: &PRINTDLGW| unsafe {
        if !pd.hDC.is_null() {
            DeleteDC(pd.hDC);
        }
        if !pd.hDevMode.is_null() {
            GlobalFree(pd.hDevMode);
        }
        if !pd.hDevNames.is_null() {
            GlobalFree(pd.hDevNames);
        }
    };
    if hdc.is_null() {
        free(&pd);
        return Err("the printer couldn't be opened".into());
    }
    let list: Vec<usize> = if pd.Flags & PD_PAGENUMS != 0 {
        let (a, b) = (pd.nFromPage.min(pd.nToPage).max(1) as usize, pd.nFromPage.max(pd.nToPage).min(max_page) as usize);
        (a - 1..b).collect()
    } else {
        (0..pages).collect()
    };
    let copies = pd.nCopies.max(1) as usize;
    // The printer's resolution and printable area.
    let (dpi_x, dpi_y, area_w, area_h) = unsafe {
        (
            GetDeviceCaps(hdc, LOGPIXELSX as i32).max(72) as f32,
            GetDeviceCaps(hdc, LOGPIXELSY as i32).max(72) as f32,
            GetDeviceCaps(hdc, HORZRES as i32).max(1) as f32,
            GetDeviceCaps(hdc, VERTRES as i32).max(1) as f32,
        )
    };
    let (w_mm, h_mm) = app.doc_mm();
    // The page at its real size, shrunk only if the printable area is smaller.
    let (mut pw, mut ph) = (w_mm / 25.4 * dpi_x, h_mm / 25.4 * dpi_y);
    let fit = (area_w / pw).min(area_h / ph).min(1.0);
    pw *= fit;
    ph *= fit;
    let (ox, oy) = (((area_w - pw) * 0.5).max(0.0), ((area_h - ph) * 0.5).max(0.0));
    // Pictures at up to 300 dpi; the printer scales them to its own.
    let render_dpi = dpi_x.min(300.0);
    let title: Vec<u16> = app.doc_title().encode_utf16().chain(std::iter::once(0)).collect();
    let di = DOCINFOW { cbSize: std::mem::size_of::<DOCINFOW>() as i32, lpszDocName: title.as_ptr(), lpszOutput: std::ptr::null(), lpszDatatype: std::ptr::null(), fwType: 0 };
    if unsafe { StartDocW(hdc, &di) } <= 0 {
        free(&pd);
        return Err("the printer didn't start the document".into());
    }
    let mut sent = 0;
    'outer: for _ in 0..copies {
        for &page in &list {
            let img = app.doc_render(&[page], render_dpi, false).remove(0);
            let (iw, ih) = img.dimensions();
            // Windows wants blue, green, red, then padding, top row first.
            let mut bgra = Vec::with_capacity((iw * ih * 4) as usize);
            for p in img.pixels() {
                let a = p[3] as u32;
                let mix = |c: u8| ((c as u32 * a + 255 * (255 - a)) / 255) as u8;
                bgra.extend_from_slice(&[mix(p[2]), mix(p[1]), mix(p[0]), 0]);
            }
            let mut bmi: BITMAPINFO = unsafe { std::mem::zeroed() };
            bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            bmi.bmiHeader.biWidth = iw as i32;
            bmi.bmiHeader.biHeight = -(ih as i32);
            bmi.bmiHeader.biPlanes = 1;
            bmi.bmiHeader.biBitCount = 32;
            bmi.bmiHeader.biCompression = BI_RGB;
            unsafe {
                if StartPage(hdc) <= 0 {
                    AbortDoc(hdc);
                    free(&pd);
                    return Err("the printer refused a page".into());
                }
                SetStretchBltMode(hdc, HALFTONE);
                StretchDIBits(hdc, ox as i32, oy as i32, pw as i32, ph as i32, 0, 0, iw as i32, ih as i32, bgra.as_ptr() as *const _, &bmi, DIB_RGB_COLORS, SRCCOPY);
                if EndPage(hdc) <= 0 {
                    break 'outer;
                }
            }
            sent += 1;
        }
    }
    unsafe {
        EndDoc(hdc);
    }
    free(&pd);
    Ok(Some(sent))
}
