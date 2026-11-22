//! Templates: empty documents with ready-made tracks.
//!
//! The standard templates (File > New from Template, and the start screen's
//! Templates tab) are built into GuitarCraft. Your own are made with File > Save
//! As Template, which keeps the song's tracks, tunings and styles with empty
//! bars, and saves it, through the standard Save dialog, as a .gpt file (as Guitar
//! Pro 8 names templates), starting in the `My Templates` folder of GuitarCraft's
//! settings folder (on Windows `%APPDATA%\GuitarCraft\My Templates`). Only your
//! own templates are kept there; the standard ones are part of the program.

use super::App;
use crate::fileio;
use crate::model::*;
use std::path::{Path, PathBuf};

/// The folder for your own templates.
pub fn my_templates_dir() -> Option<PathBuf> {
    Some(super::home::app_dir()?.join("My Templates"))
}

/// Where templates were kept before there was a My Templates folder; still read.
fn old_templates_dir() -> Option<PathBuf> {
    Some(super::home::app_dir()?.join("Templates"))
}

/// A song as a template: its tracks and settings with every bar empty and no
/// text of its own.
pub fn as_template(song: &Song) -> Song {
    let mut t = song.clone();
    for tr in &mut t.tracks {
        tr.lyrics.clear();
        for b in &mut tr.bars {
            *b = Bar::empty();
        }
    }
    for m in &mut t.master {
        m.marker = None;
        m.repeat_open = false;
        m.repeat_close = 0;
        m.endings = 0;
    }
    t.title = "Untitled".into();
    t.subtitle.clear();
    t.artist.clear();
    t.album.clear();
    t.words.clear();
    t.music_by.clear();
    t.tabbed_by.clear();
    t.notice.clear();
    t.normalize();
    t
}

/// Saves a song as a template file (ending in .gpt, whatever the name given), making its folder if needed.
pub fn write_template(song: &Song, path: &Path) -> Result<PathBuf, String> {
    let path = path.with_extension(fileio::GPT_EXT);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    fileio::save_song(&as_template(song), &path)?;
    Ok(path)
}

impl App {
    /// Your own templates, by name: the .gpt files in My Templates, and the .gp
    /// templates earlier versions of GuitarCraft saved (there and in the older
    /// Templates folder).
    pub(super) fn user_templates() -> Vec<PathBuf> {
        let mut v = super::home::files_with_ext(my_templates_dir(), fileio::GPT_EXT);
        let mut old = super::home::files_with_ext(my_templates_dir(), fileio::GP_EXT);
        old.extend(super::home::files_with_ext(old_templates_dir(), fileio::GP_EXT));
        for p in old {
            let stem = p.file_stem().map(|n| n.to_ascii_lowercase());
            if !v.iter().any(|q| q.file_stem().map(|n| n.to_ascii_lowercase()) == stem) {
                v.push(p);
            }
        }
        v.sort_by_key(|p| p.file_stem().map(|s| s.to_string_lossy().to_lowercase()));
        v
    }

    /// File > Save As Template: the standard Save dialog, starting in My Templates.
    pub(super) fn save_as_template(&mut self) {
        if self.chart.is_some() {
            self.refuse("Templates are for songs; a chord chart can be saved as a copy instead.");
            return;
        }
        if self.screenshot.is_some() {
            return;
        }
        let name = if self.song.title.trim().is_empty() || self.song.title == "Untitled" { "My template".to_string() } else { self.song.title.trim().to_string() };
        let mut d = rfd::FileDialog::new()
            .set_title("Save As Template")
            .add_filter("GuitarCraft template", &[fileio::GPT_EXT])
            .set_file_name(format!("{}.{}", super::widgets::sanitize(&name), fileio::GPT_EXT));
        if let Some(dir) = my_templates_dir() {
            let _ = std::fs::create_dir_all(&dir);
            d = d.set_directory(dir);
        }
        let Some(p) = d.save_file() else { return };
        self.status = match write_template(&self.song, &p) {
            Ok(saved) => {
                let name = saved.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                format!("Saved the template \u{201C}{name}\u{201D}. Find it in File \u{25B8} New from Template.")
            }
            Err(e) => super::notice::failed(format!("Couldn't save the template: {e}")),
        };
    }

    /// Shows the My Templates folder in the file manager.
    pub(super) fn open_templates_folder(&mut self) {
        let Some(d) = my_templates_dir() else { return };
        let _ = std::fs::create_dir_all(&d);
        #[cfg(target_os = "windows")]
        let r = std::process::Command::new("explorer").arg(&d).spawn();
        #[cfg(target_os = "macos")]
        let r = std::process::Command::new("open").arg(&d).spawn();
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        let r = std::process::Command::new("xdg-open").arg(&d).spawn();
        if let Err(e) = r {
            self.status = super::notice::failed(format!("Couldn't open the folder: {e}"));
        }
    }

    pub(super) fn open_template_file(&mut self, p: PathBuf) {
        match fileio::load_song(&p) {
            Ok(mut s) => {
                self.begin_new_doc();
                s.title = "Untitled".into();
                self.song = s;
                self.path = None;
                self.reset_after_load();
                self.show_home = false;
            }
            Err(e) => self.status = super::notice::failed(format!("Couldn't open the template: {e}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_standard_templates_are_empty_documents_with_tracks() {
        use super::super::home::{TEMPLATES, template};
        assert_eq!(TEMPLATES.len(), 24);
        assert_eq!(TEMPLATES[0], "Empty (default)");
        assert_eq!(TEMPLATES[1], "Acoustic Bass");
        assert_eq!(TEMPLATES[23], "Thrash Band");
        for (k, name) in TEMPLATES.iter().enumerate() {
            let s = template(k);
            assert!(!s.tracks.is_empty(), "{name}");
            assert!(s.tracks.iter().all(|t| t.bars.iter().all(|b| b.is_empty())), "{name} has empty bars");
            assert!(s.tracks.iter().all(|t| !t.name.is_empty() && !t.tuning.is_empty()), "{name}");
        }
        assert_eq!(template(3).tracks.len(), 3, "Blues Power Trio");
        assert_eq!(template(15).tracks.len(), 3, "Jazz Trio");
        assert_eq!(template(14).tracks.len(), 5, "Jazz Quintet");
        assert!(template(6).tracks[0].instrument == Instrument::Drums, "Drumkit");
        assert_eq!(template(1).tracks[0].tuning.len(), 4, "Acoustic Bass has four strings");
    }

    #[test]
    fn a_saved_template_is_a_gpt_file_with_the_tracks_and_empty_bars() {
        let dir = std::env::temp_dir().join(format!("guitarcraft-tmpl-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let song = Song::demo();
        // Whatever ending is typed, a template ends in .gpt.
        let p = write_template(&song, &dir.join("My Templates").join("Demo band.gp")).unwrap();
        assert_eq!(p, dir.join("My Templates").join("Demo band.gpt"));
        assert!(p.exists());
        let back = fileio::load_song(&p).unwrap();
        assert_eq!(back.tracks.len(), song.tracks.len());
        assert_eq!(back.tracks[0].name, song.tracks[0].name);
        assert_eq!(back.tracks[0].tuning, song.tracks[0].tuning);
        assert!(back.tracks.iter().all(|t| t.bars.iter().all(|b| b.is_empty())));
        assert!(back.master.iter().all(|m| m.marker.is_none()));
        // It is the same zip file as a .gp song, which Guitar Pro 8 opens.
        assert!(std::fs::read(&p).unwrap().starts_with(b"PK"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
