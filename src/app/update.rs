//! Help > Check for Updates...: asks GitHub which release of GuitarCraft is the newest
//! and says whether this copy is up to date. It looks when the person asks, or at start
//! when they chose so in Preferences > General (then the window only appears if there is
//! a newer version). Nothing about the person or their files is sent.
//! The web request goes through the `curl` program that comes with Windows, macOS and
//! most Linux systems, on a separate thread so the window never freezes.

use super::App;
use egui::{Align2, Color32, RichText, Vec2};
use std::sync::mpsc::{Receiver, channel};

/// Where the newest release is announced.
const LATEST_RELEASE: &str = "https://api.github.com/repos/CodeGator/GuitarCraft/releases/latest";
/// The page to open when there is no better link.
const RELEASES_PAGE: &str = "https://github.com/CodeGator/GuitarCraft/releases";

#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    /// This copy is the newest (or newer: a build made before the release).
    UpToDate { latest: String },
    /// A newer release exists.
    Newer { latest: String, url: String, notes: String },
    /// Nothing has been published yet.
    NoReleases,
    /// The check couldn't be done (offline, blocked, no curl...).
    Failed(String),
}

#[derive(Default)]
pub struct UpdateState {
    pub open: bool,
    checking: bool,
    /// A check started by itself: the window opens only for a newer version.
    quiet: bool,
    outcome: Option<Outcome>,
    rx: Option<Receiver<Outcome>>,
}

/// "v1.2.3" / "1.2.3-beta" -> [1, 2, 3]; missing or odd parts count as 0.
fn version_parts(v: &str) -> Vec<u64> {
    let v = v.trim().trim_start_matches(['v', 'V']);
    let v = v.split(['-', '+']).next().unwrap_or("");
    let mut parts: Vec<u64> = v.split('.').map(|p| p.trim().parse().unwrap_or(0)).collect();
    while parts.len() < 3 {
        parts.push(0);
    }
    parts
}

/// Is `latest` a higher version than `current`?
pub fn is_newer(latest: &str, current: &str) -> bool {
    version_parts(latest) > version_parts(current)
}

/// Reads GitHub's answer for "the latest release" and compares it with `current`.
pub fn read_release(json: &str, current: &str) -> Outcome {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(json) else {
        return Outcome::Failed("GitHub's answer wasn't something GuitarCraft understands.".into());
    };
    let Some(tag) = v.get("tag_name").and_then(|t| t.as_str()) else {
        // A repository without releases answers "Not Found".
        return if v.get("message").and_then(|m| m.as_str()) == Some("Not Found") {
            Outcome::NoReleases
        } else {
            Outcome::Failed(
                v.get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("GitHub's answer had no version in it.")
                    .to_string(),
            )
        };
    };
    let latest = tag.trim_start_matches(['v', 'V']).to_string();
    if is_newer(&latest, current) {
        let url = v
            .get("html_url")
            .and_then(|u| u.as_str())
            .filter(|u| u.starts_with("https://github.com/"))
            .unwrap_or(RELEASES_PAGE)
            .to_string();
        let notes = v.get("body").and_then(|b| b.as_str()).unwrap_or("").trim();
        let notes: String = notes.chars().take(600).collect();
        Outcome::Newer { latest, url, notes }
    } else {
        Outcome::UpToDate { latest }
    }
}

/// Fetches the page with `curl`.
fn fetch(url: &str) -> Result<String, String> {
    let mut cmd = std::process::Command::new("curl");
    cmd.args(["--silent", "--location", "--max-time", "15"])
        .args(["--header", "Accept: application/vnd.github+json"])
        .args(["--header", "User-Agent: GuitarCraft"])
        .arg(url);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // no console window flashing up
    }
    let out = cmd
        .output()
        .map_err(|_| "GuitarCraft couldn't start the program it uses to go online (curl).".to_string())?;
    if !out.status.success() {
        return Err("Couldn't reach GitHub. Check the internet connection and try again.".into());
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Opens a web page in the person's browser.
fn open_page(url: &str) {
    #[cfg(windows)]
    let r = std::process::Command::new("explorer").arg(url).spawn();
    #[cfg(target_os = "macos")]
    let r = std::process::Command::new("open").arg(url).spawn();
    #[cfg(not(any(windows, target_os = "macos")))]
    let r = std::process::Command::new("xdg-open").arg(url).spawn();
    let _ = r;
}

impl App {
    /// Help > Check for Updates...: opens the window and starts looking.
    pub(super) fn check_for_updates(&mut self, ctx: &egui::Context) {
        self.update.open = true;
        self.update.quiet = false;
        self.start_check(ctx);
    }

    /// The check at start (Preferences > General > Look for a new version).
    pub(super) fn check_for_updates_quietly(&mut self, ctx: &egui::Context) {
        self.update.quiet = true;
        self.start_check(ctx);
    }

    fn start_check(&mut self, ctx: &egui::Context) {
        if self.update.checking {
            return;
        }
        self.update.checking = true;
        self.update.outcome = None;
        let (tx, rx) = channel();
        self.update.rx = Some(rx);
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let outcome = match fetch(LATEST_RELEASE) {
                Ok(body) => read_release(&body, env!("CARGO_PKG_VERSION")),
                Err(e) => Outcome::Failed(e),
            };
            let _ = tx.send(outcome);
            ctx.request_repaint();
        });
    }

    pub(super) fn update_window(&mut self, ctx: &egui::Context) {
        if let Some(Ok(o)) = self.update.rx.as_ref().map(|rx| rx.try_recv()) {
            self.update.outcome = Some(o);
            self.update.checking = false;
            self.update.rx = None;
            if self.update.quiet && matches!(self.update.outcome, Some(Outcome::Newer { .. })) {
                self.update.open = true;
            }
        }
        if !self.update.open {
            return;
        }
        let mut open = true;
        let mut again = false;
        let mut go: Option<String> = None;
        egui::Window::new("Check for Updates")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .pivot(Align2::CENTER_CENTER)
            .default_pos(ctx.content_rect().center())
            .fixed_size(Vec2::new(420.0, 0.0))
            .constrain(true)
            .show(ctx, |ui| {
                let current = env!("CARGO_PKG_VERSION");
                ui.label(RichText::new(format!("You have GuitarCraft {current}.")).color(Color32::from_gray(200)));
                ui.add_space(8.0);
                match &self.update.outcome {
                    None => {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label("Looking for a newer version...");
                        });
                    }
                    Some(Outcome::UpToDate { latest }) => {
                        ui.label(
                            RichText::new(format!("GuitarCraft is up to date (the newest version is {latest})."))
                                .color(Color32::from_rgb(110, 200, 130)),
                        );
                    }
                    Some(Outcome::Newer { latest, url, notes }) => {
                        ui.label(
                            RichText::new(format!("A new version is available: GuitarCraft {latest}."))
                                .strong()
                                .color(Color32::from_rgb(240, 180, 80)),
                        );
                        if !notes.is_empty() {
                            ui.add_space(6.0);
                            egui::ScrollArea::vertical().max_height(140.0).show(ui, |ui| {
                                ui.label(RichText::new(notes.as_str()).size(12.5).color(Color32::from_gray(170)));
                            });
                        }
                        ui.add_space(8.0);
                        if ui.button("Open the download page").clicked() {
                            go = Some(url.clone());
                        }
                    }
                    Some(Outcome::NoReleases) => {
                        ui.label("No version has been published yet, so there is nothing newer to get.");
                    }
                    Some(Outcome::Failed(why)) => {
                        ui.label(RichText::new(why.as_str()).color(Color32::from_rgb(230, 120, 110)));
                    }
                }
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.add_enabled(!self.update.checking, egui::Button::new("Check again")).clicked() {
                        again = true;
                    }
                    if ui.button("Close").clicked() {
                        self.update.open = false;
                    }
                });
            });
        if !open || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.update.open = false;
        }
        if let Some(u) = go {
            open_page(&u);
        }
        if again {
            self.check_for_updates(ctx);
        }
    }

    /// For pictures and tests: shows the window with a ready-made answer.
    pub(super) fn show_update_outcome(&mut self, o: Outcome) {
        self.update.open = true;
        self.update.checking = false;
        self.update.outcome = Some(o);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_compare_by_number_not_by_text() {
        assert!(is_newer("0.10.0", "0.9.0"));
        assert!(is_newer("v1.0.0", "0.4.0"));
        assert!(!is_newer("0.4.0", "0.4.0"));
        assert!(!is_newer("0.3.9", "0.4.0"));
        assert!(!is_newer("0.4.0-beta", "0.4.0"));
        assert!(is_newer("0.4.1", "0.4"));
    }

    #[test]
    fn a_release_answer_says_newer_or_up_to_date() {
        let newer = r#"{"tag_name":"v0.5.0","html_url":"https://github.com/CodeGator/GuitarCraft/releases/tag/v0.5.0","body":"New things"}"#;
        match read_release(newer, "0.4.0") {
            Outcome::Newer { latest, url, notes } => {
                assert_eq!(latest, "0.5.0");
                assert!(url.ends_with("v0.5.0"));
                assert_eq!(notes, "New things");
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(
            read_release(newer, "0.5.0"),
            Outcome::UpToDate { latest: "0.5.0".into() }
        );
    }

    #[test]
    fn odd_answers_are_explained_not_crashed_on() {
        assert_eq!(read_release(r#"{"message":"Not Found"}"#, "0.4.0"), Outcome::NoReleases);
        assert!(matches!(read_release("<html>", "0.4.0"), Outcome::Failed(_)));
        assert!(matches!(read_release(r#"{"message":"API rate limit exceeded"}"#, "0.4.0"), Outcome::Failed(m) if m.contains("rate limit")));
        // A link that isn't on GitHub is never opened.
        let odd = r#"{"tag_name":"v9.0.0","html_url":"https://example.com/x"}"#;
        assert!(matches!(read_release(odd, "0.4.0"), Outcome::Newer { url, .. } if url.starts_with("https://github.com/")));
    }
}
