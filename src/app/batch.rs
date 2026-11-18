//! File > Batch Converter: turns many song files into another format at once
//! (Guitar Pro .gp/.gpx/.gp5, MIDI, MusicXML, ASCII tab or PDF), saved next to
//! each file or in one folder. Scores only: chord charts aren't converted.

use super::App;
use crate::render::PrintSpec;
use egui::{Align2, Color32, RichText, Vec2};
use std::path::{Path, PathBuf};

/// Formats the converter can write: (name shown, file extension).
pub const FORMATS: [(&str, &str); 7] = [
    ("Guitar Pro (.gp)", "gp"),
    ("Guitar Pro 6 (.gpx)", "gpx"),
    ("Guitar Pro 5 (.gp5)", "gp5"),
    ("MIDI (.mid)", "mid"),
    ("MusicXML (.musicxml)", "musicxml"),
    ("ASCII tab (.txt)", "txt"),
    ("PDF (.pdf)", "pdf"),
];

/// Song files the converter can read.
pub const READS: [&str; 12] = ["gp", "gpx", "gp3", "gp4", "gp5", "mid", "midi", "musicxml", "xml", "mxl", "txt", "tab"];

#[derive(Default)]
pub struct BatchDlg {
    pub files: Vec<PathBuf>,
    pub format: usize,
    /// Save everything here; None: next to each file.
    pub folder: Option<PathBuf>,
    /// What happened to each file: (file name, Ok(new file) or Err(why)).
    pub results: Vec<(String, Result<String, String>)>,
}

fn readable(p: &Path) -> bool {
    p.extension().map(|e| READS.contains(&e.to_string_lossy().to_ascii_lowercase().as_str())).unwrap_or(false)
}

/// Converts one file; returns the new file's path.
pub fn convert(src: &Path, ext: &str, folder: Option<&Path>) -> Result<PathBuf, String> {
    let song = crate::fileio::load_song(src)?;
    let stem = src.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "Untitled".into());
    let dir = folder.map(|f| f.to_path_buf()).or_else(|| src.parent().map(|p| p.to_path_buf())).unwrap_or_default();
    let dst = dir.join(format!("{stem}.{ext}"));
    if dst == src {
        return Err("it is already in this format here (choose another folder)".into());
    }
    if ext == "pdf" {
        let mut shown: Vec<usize> = (0..song.tracks.len()).filter(|&i| song.tracks[i].visible).collect();
        if shown.is_empty() {
            shown = (0..song.tracks.len()).collect();
        }
        let spec = PrintSpec { song: &song, shown, track: 0, notation: crate::score::Notation::Both };
        let pages: Vec<usize> = (0..spec.pages()).collect();
        let imgs = crate::render::render_pages(&spec, &pages, 200.0, false);
        let (w, h) = spec.page_mm();
        std::fs::write(&dst, crate::render::pdf(&imgs, w, h, &song.title)).map_err(|e| e.to_string())?;
    } else {
        crate::fileio::export_song(&song, &dst)?;
    }
    Ok(dst)
}

impl App {
    pub(super) fn open_batch(&mut self) {
        if self.batch.is_none() {
            self.batch = Some(BatchDlg::default());
        }
    }

    pub(super) fn batch_window(&mut self, ctx: &egui::Context) {
        let shot = self.screenshot.is_some();
        let start_dir = self.dialog_dir();
        let Some(dlg) = &mut self.batch else { return };
        let mut open = true;
        let mut run = false;
        egui::Window::new("Batch Converter")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.set_width(520.0);
                ui.label(RichText::new("Convert many song files to another format at once.").color(Color32::from_gray(190)));
                ui.add_space(6.0);
                ui.label(RichText::new(format!("Files to convert ({})", dlg.files.len())).strong());
                egui::Frame::new().fill(Color32::from_rgb(28, 30, 36)).corner_radius(4.0).inner_margin(egui::Margin::same(6)).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    egui::ScrollArea::vertical().max_height(170.0).auto_shrink([false, true]).show(ui, |ui| {
                        if dlg.files.is_empty() {
                            ui.label(RichText::new("No files yet: add files or a folder.").weak());
                        }
                        let mut remove = None;
                        for (k, f) in dlg.files.iter().enumerate() {
                            ui.horizontal(|ui| {
                                if ui.small_button("\u{2715}").on_hover_text("Take this file off the list").clicked() {
                                    remove = Some(k);
                                }
                                let name = f.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                                ui.label(name).on_hover_text(f.display().to_string());
                            });
                        }
                        if let Some(k) = remove {
                            dlg.files.remove(k);
                        }
                    });
                });
                ui.horizontal(|ui| {
                    if ui.button("Add Files\u{2026}").clicked() && !shot {
                        let mut d = rfd::FileDialog::new().add_filter("Song files", &READS);
                        if let Some(dir) = &start_dir {
                            d = d.set_directory(dir);
                        }
                        for p in d.pick_files().unwrap_or_default() {
                            if !dlg.files.contains(&p) {
                                dlg.files.push(p);
                            }
                        }
                    }
                    if ui.button("Add Folder\u{2026}").on_hover_text("Every song file in a folder").clicked() && !shot {
                        let mut d = rfd::FileDialog::new();
                        if let Some(dir) = &start_dir {
                            d = d.set_directory(dir);
                        }
                        if let Some(dir) = d.pick_folder() {
                            let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
                                .map(|r| r.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.is_file() && readable(p)).collect())
                                .unwrap_or_default();
                            found.sort();
                            for p in found {
                                if !dlg.files.contains(&p) {
                                    dlg.files.push(p);
                                }
                            }
                        }
                    }
                    if ui.add_enabled(!dlg.files.is_empty(), egui::Button::new("Clear")).clicked() {
                        dlg.files.clear();
                        dlg.results.clear();
                    }
                });
                ui.add_space(6.0);
                egui::Grid::new("batch").num_columns(2).spacing(Vec2::new(10.0, 8.0)).show(ui, |ui| {
                    ui.label("Convert to:");
                    egui::ComboBox::from_id_salt("batch-format").width(220.0).selected_text(FORMATS[dlg.format].0).show_ui(ui, |ui| {
                        for (k, (name, _)) in FORMATS.iter().enumerate() {
                            ui.selectable_value(&mut dlg.format, k, *name);
                        }
                    });
                    ui.end_row();
                    ui.label("Save in:");
                    ui.vertical(|ui| {
                        let mut here = dlg.folder.is_none();
                        if ui.radio(here, "The same folder as each file").clicked() {
                            here = true;
                            dlg.folder = None;
                        }
                        ui.horizontal(|ui| {
                            if ui.radio(!here, "This folder:").clicked() && dlg.folder.is_none() {
                                dlg.folder = start_dir.clone();
                            }
                            let shown = dlg.folder.as_ref().map(|f| f.display().to_string()).unwrap_or_else(|| "(none chosen)".into());
                            ui.add(egui::Label::new(RichText::new(&shown).weak()).truncate()).on_hover_text(&shown);
                            if ui.button("Choose\u{2026}").clicked() && !shot {
                                let mut d = rfd::FileDialog::new();
                                if let Some(dir) = dlg.folder.as_ref().or(start_dir.as_ref()) {
                                    d = d.set_directory(dir);
                                }
                                if let Some(f) = d.pick_folder() {
                                    dlg.folder = Some(f);
                                }
                            }
                        });
                    });
                    ui.end_row();
                });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add_enabled(!dlg.files.is_empty(), egui::Button::new(RichText::new("Convert").color(Color32::WHITE)).fill(super::panels::BLUE)).clicked() {
                            run = true;
                        }
                    });
                });
                if !dlg.results.is_empty() {
                    ui.separator();
                    let ok = dlg.results.iter().filter(|r| r.1.is_ok()).count();
                    ui.label(RichText::new(format!("{ok} of {} converted", dlg.results.len())).strong());
                    egui::ScrollArea::vertical().id_salt("batch-results").max_height(140.0).show(ui, |ui| {
                        for (name, r) in &dlg.results {
                            match r {
                                Ok(out) => ui.label(RichText::new(format!("\u{2714} {name} \u{2192} {out}")).color(Color32::from_rgb(120, 200, 130))),
                                Err(e) => ui.label(RichText::new(format!("\u{2716} {name}: {e}")).color(Color32::from_rgb(235, 120, 110))),
                            };
                        }
                    });
                }
            });
        if run && !shot {
            let ext = FORMATS[dlg.format].1;
            let folder = dlg.folder.clone();
            dlg.results = dlg
                .files
                .iter()
                .map(|f| {
                    let name = f.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                    let r = convert(f, ext, folder.as_deref()).map(|p| p.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default());
                    (name, r)
                })
                .collect();
            let ok = dlg.results.iter().filter(|r| r.1.is_ok()).count();
            self.status = format!("Batch Converter: {ok} of {} files converted.", dlg.results.len());
        }
        if !open || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.batch = None;
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn converts_between_formats() {
        let d = std::env::temp_dir().join(format!("guitarcraft-batch-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        let src = d.join("song.gp");
        crate::fileio::save_song(&crate::model::Song::demo(), &src).unwrap();
        for (_, ext) in super::FORMATS.iter().filter(|f| f.1 != "gp") {
            let out = super::convert(&src, ext, None).unwrap_or_else(|e| panic!("{ext}: {e}"));
            assert!(out.exists() && std::fs::metadata(&out).unwrap().len() > 0, "{ext}");
        }
        // Converting a file into its own format in its own folder is refused.
        assert!(super::convert(&src, "gp", None).is_err());
        let _ = std::fs::remove_dir_all(&d);
    }
}
