//! GuitarCraft: a guitar and bass tablature editor.

// A Windows GUI app: no console window opens with it. Command-line runs (--export, --convert)
// reattach to the terminal they were started from; see `attach_parent_console`.
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod app;
mod audio;
mod chart;
mod chordshape;
mod extras;
mod fileio;
mod fretboard;
mod gp5write;
mod gpif;
mod gpifwrite;
mod gpimport;
mod gpstyle;
mod gpx;
mod gpsv;
mod midi;
mod model;
mod musicxml;
mod render;
mod score;
mod style;
mod synth;
mod theory;
mod tuner;

/// A file from tests/fixtures/ for a test; the test fails if it is missing (see
/// tests/fixtures/README.md for what each file is).
#[cfg(test)]
pub fn fixture(path: &str) -> Vec<u8> {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(path);
    std::fs::read(&p).unwrap_or_else(|e| panic!("test fixture {} missing: {e}", p.display()))
}
mod textmidi;

use egui::{FontData, FontDefinitions, FontFamily};
use std::path::PathBuf;
use std::sync::Arc;

fn main() -> eframe::Result {
    attach_parent_console();
    let mut launch = app::LaunchOptions::default();
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        if a == "--export" {
            // Print a song (or "demo") to a PDF or PNG and exit:
            // --export <from> <to.pdf|to.png> [dpi]
            let from = args.next().unwrap_or_default();
            let to = std::path::PathBuf::from(args.next().unwrap_or_default());
            let dpi = args
                .next()
                .and_then(|d| d.parse::<f32>().ok())
                .unwrap_or(150.0);
            // A chord chart ("chart" is the sample chart).
            let is_chart = from == "chart" || chart::is_chart_path(std::path::Path::new(&from));
            if is_chart {
                let doc = if from == "chart" {
                    Ok(chart::ChordChart::sample())
                } else {
                    chart::ChordChart::load(std::path::Path::new(&from))
                };
                let r = doc.and_then(|doc| {
                    let pages: Vec<usize> = (0..render::chart_page_count(&doc)).collect();
                    if to
                        .extension()
                        .map(|e| e.eq_ignore_ascii_case(chart::EXT))
                        .unwrap_or(false)
                    {
                        // Saves the chart itself (for making sample files).
                        return doc.save(&to);
                    }
                    let imgs = render::render_chart(&doc, &pages, dpi);
                    if to
                        .extension()
                        .map(|e| e.eq_ignore_ascii_case("png"))
                        .unwrap_or(false)
                    {
                        render::stack(&imgs).save(&to).map_err(|e| e.to_string())
                    } else {
                        std::fs::write(
                            &to,
                            render::pdf(&imgs, chart::PAGE_MM.0, chart::PAGE_MM.1, &doc.title),
                        )
                        .map_err(|e| e.to_string())
                    }
                });
                exit_with(r);
            }
            let song = if from == "demo" {
                Ok(model::Song::demo())
            } else {
                fileio::load_song(std::path::Path::new(&from))
            };
            let r = song.and_then(|song| {
                let shown: Vec<usize> = (0..song.tracks.len())
                    .filter(|&i| song.tracks[i].visible)
                    .collect();
                let spec = render::PrintSpec {
                    song: &song,
                    shown,
                    track: 0,
                    notation: score::Notation::Both,
                };
                let pages: Vec<usize> = (0..spec.pages()).collect();
                let png = to
                    .extension()
                    .map(|e| e.eq_ignore_ascii_case("png"))
                    .unwrap_or(false);
                let imgs = render::render_pages(&spec, &pages, dpi, false);
                if png {
                    render::stack(&imgs).save(&to).map_err(|e| e.to_string())
                } else {
                    let (w, h) = spec.page_mm();
                    std::fs::write(&to, render::pdf(&imgs, w, h, &song.title))
                        .map_err(|e| e.to_string())
                }
            });
            exit_with(r);
        }
        if a == "--convert" {
            // Save a song (or "demo") in another format and exit: --convert <from> <to>.
            let from = args.next().unwrap_or_default();
            let to = args.next().unwrap_or_default();
            // "demo:classic" (or rock, jazz) is the demo song with a ready-made style.
            let song = if let Some(look) = from.strip_prefix("demo") {
                let mut s = model::Song::demo();
                match look {
                    ":rock" => s.style = style::Stylesheet::rock(),
                    ":classic" => s.style = style::Stylesheet::classic(),
                    ":jazz" => s.style = style::Stylesheet::jazz(),
                    _ => {}
                }
                Ok(s)
            } else {
                fileio::load_song(std::path::Path::new(&from))
            };
            exit_with(song.and_then(|s| fileio::export_song(&s, std::path::Path::new(&to))));
        }
        if a == "--gpx-tests" {
            // Files for finding out which part of a .gpx file Guitar Pro 8 objects to:
            // our export; Guitar Pro 8's own files packed by us; our score with
            // Guitar Pro 8's other files (from tests/fixtures/gp8). Written to shots/gpx-checks.
            let dir = std::path::PathBuf::from(
                args.next().unwrap_or_else(|| "shots/gpx-checks".into()),
            );
            let _ = std::fs::create_dir_all(&dir);
            let demo = model::Song::demo();
            let mut made = vec![("Check GPX 1 - GuitarCraft export", gpx::export_gpx(&demo))];
            let gp8 = std::fs::read("tests/fixtures/gp8/Morning Walk - GP8.gpx")
                .map_err(|e| e.to_string())
                .and_then(|d| gpif::gpx_files(&d));
            match gp8 {
                Ok(theirs) => {
                    let pack = |v: &[(String, Vec<u8>)]| {
                        let f: Vec<(&str, Vec<u8>)> = v.iter().map(|(n, d)| (n.as_str(), d.clone())).collect();
                        gpx::pack(&f)
                    };
                    made.push(("Check GPX 2 - GP8 files, GuitarCraft packing", pack(&theirs)));
                    let ours = gpx::files(&demo);
                    let mixed: Vec<(String, Vec<u8>)> = theirs
                        .iter()
                        .map(|(n, d)| match ours.iter().find(|o| o.0 == "score.gpif" && n == "score.gpif") {
                            Some(o) => (n.clone(), o.1.clone()),
                            None => (n.clone(), d.clone()),
                        })
                        .collect();
                    made.push(("Check GPX 3 - GuitarCraft score, GP8 other files", pack(&mixed)));
                }
                Err(e) => eprintln!("Morning Walk - GP8.gpx not read ({e}); writing only check 1."),
            }
            for (name, data) in made {
                let _ = std::fs::write(dir.join(format!("{name}.gpx")), data);
            }
            std::process::exit(0);
        }
        if a == "--gp-tests" {
            // Files for checking which parts Guitar Pro 8 accepts.
            let dir =
                std::path::PathBuf::from(args.next().unwrap_or_else(|| "shots/gp-checks".into()));
            let _ = std::fs::create_dir_all(&dir);
            let demo = model::Song::demo();
            let blank = model::Song::new_blank();
            for (name, song, chords) in [
                ("Check 2 - new format", &demo, true),
                ("Check 3 - new format, no chord names", &demo, false),
                ("Check 4 - blank song, new format", &blank, false),
            ] {
                let data = gpifwrite::export_gp_with(
                    song,
                    gpifwrite::GpOptions {
                        chords,
                        views: false,
                    },
                )
                .unwrap_or_default();
                let _ = std::fs::write(dir.join(format!("{name}.gp")), data);
            }
            std::process::exit(0);
        }
        match a.as_str() {
            "--screenshot" => launch.screenshot = args.next().map(PathBuf::from),
            "--zoom" => launch.zoom = args.next().and_then(|z| z.parse().ok()),
            "--bar" => launch.bar = args.next().and_then(|b| b.parse().ok()),
            "--track" => launch.track = args.next().and_then(|t| t.parse().ok()),
            "--home" => launch.home = true,
            // A comma-separated list, such as "keyboard,fretboard".
            "--show" => {
                if let Some(list) = args.next() {
                    launch
                        .show
                        .extend(list.split(',').map(|s| s.trim().to_string()));
                }
            }
            _ if !a.starts_with("--") => launch.open = Some(PathBuf::from(a)),
            _ => {}
        }
    }
    let screenshot = launch.screenshot.is_some();
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("GuitarCraft")
        .with_inner_size([1440.0, 900.0])
        .with_min_inner_size([900.0, 600.0]);
    if let Some(icon) = app_icon() {
        viewport = viewport.with_icon(icon);
    }
    if screenshot {
        // Taking a picture for a check: open without taking focus, behind other windows on Windows.
        viewport = viewport.with_active(false);
        #[cfg(target_os = "windows")]
        {
            viewport = viewport.with_window_level(egui::WindowLevel::AlwaysOnBottom);
        }
    }
    #[cfg_attr(not(target_os = "macos"), allow(unused_mut))] // Only changed on macOS.
    let mut options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    #[cfg(target_os = "macos")]
    if screenshot {
        use winit::platform::macos::EventLoopBuilderExtMacOS;
        options.event_loop_builder = Some(Box::new(|b| {
            b.with_activate_ignoring_other_apps(false);
        }));
    }
    eframe::run_native(
        "GuitarCraft",
        options,
        Box::new(move |cc| Ok(Box::new(app::App::new(cc, launch)))),
    )
}

/// The window, taskbar and Alt-Tab icon. Regenerate it with `cargo xtask icons`.
fn app_icon() -> Option<egui::IconData> {
    let png: &[u8] = include_bytes!("../assets/app-icon/guitarcraft-256.png");
    eframe::icon_data::from_png_bytes(png).ok()
}

/// On Windows the app has no console of its own, so when it is started from a terminal it
/// borrows that terminal's console. Messages from command-line runs (`--export`, `--convert`)
/// then appear there. Started from Explorer or the Start menu there is no parent console, and
/// this does nothing.
fn attach_parent_console() {
    #[cfg(target_os = "windows")]
    // SAFETY: AttachConsole only takes a process id; failing (no parent console) is harmless.
    unsafe {
        windows_sys::Win32::System::Console::AttachConsole(
            windows_sys::Win32::System::Console::ATTACH_PARENT_PROCESS,
        );
    }
}

fn font_file(name: &str) -> Option<Vec<u8>> {
    let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into());
    let candidates = [
        format!("{windir}\\Fonts\\{name}"),
        format!("/usr/share/fonts/truetype/{name}"),
        format!("/Library/Fonts/{name}"),
    ];
    candidates.iter().find_map(|p| std::fs::read(p).ok())
}

/// Loads system fonts when available. Returns true when a font with music
/// symbols (clefs, rests) was found.
pub fn setup_fonts(ctx: &egui::Context) -> bool {
    let mut fonts = FontDefinitions::default();
    let base_prop = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    let add = |fonts: &mut FontDefinitions, key: &str, file: &str| -> bool {
        if let Some(d) = font_file(file) {
            fonts
                .font_data
                .insert(key.to_owned(), Arc::new(FontData::from_owned(d)));
            true
        } else {
            false
        }
    };
    let ui_font = add(&mut fonts, "ui", "segoeui.ttf");
    let bold = add(&mut fonts, "ui-bold", "segoeuib.ttf");
    let italic = add(&mut fonts, "ui-italic", "segoeuii.ttf");
    let music = add(&mut fonts, "music", "seguisym.ttf");
    let serif = add(&mut fonts, "serif", "times.ttf");
    let serif_b = add(&mut fonts, "serif-bold", "timesbd.ttf");
    let serif_bi = add(&mut fonts, "serif-bi", "timesbi.ttf");

    let mut prop = base_prop.clone();
    if ui_font {
        prop.insert(0, "ui".into());
    }
    if music {
        prop.push("music".into());
    }
    fonts
        .families
        .insert(FontFamily::Proportional, prop.clone());
    let mut b = prop.clone();
    if bold {
        b.insert(0, "ui-bold".into());
    }
    fonts.families.insert(FontFamily::Name("bold".into()), b);
    let mut i = prop.clone();
    if italic {
        i.insert(0, "ui-italic".into());
    }
    fonts.families.insert(FontFamily::Name("italic".into()), i);
    for (fam, ok, key) in [
        ("serif", serif, "serif"),
        ("serif-bold", serif_b, "serif-bold"),
        ("serif-bi", serif_bi, "serif-bi"),
    ] {
        let mut v = prop.clone();
        if ok {
            v.insert(0, key.to_string());
        }
        fonts.families.insert(FontFamily::Name(fam.into()), v);
    }
    // The fonts a stylesheet can choose.
    style::register_fonts(&mut fonts, &prop, font_file);
    let mut m = vec![];
    if music {
        m.push("music".to_string());
    }
    m.extend(prop);
    fonts.families.insert(FontFamily::Name("music".into()), m);
    if music && let Some(mono) = fonts.families.get_mut(&FontFamily::Monospace) {
        mono.push("music".into());
    }
    ctx.set_fonts(fonts);
    music
}

pub fn setup_style(ctx: &egui::Context) {
    ctx.set_theme(egui::Theme::Dark);
    ctx.style_mut_of(egui::Theme::Dark, |s| {
        s.spacing.button_padding = egui::vec2(8.0, 4.0);
        s.visuals.panel_fill = egui::Color32::from_rgb(36, 38, 43);
        // Popups and message boxes: lighter than the panels behind them, with a
        // clear edge, rounder corners, a softer shadow, and text that is easy to read.
        s.visuals.window_fill = egui::Color32::from_rgb(58, 62, 71);
        s.visuals.window_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(104, 110, 124));
        s.visuals.window_corner_radius = egui::CornerRadius::same(8);
        s.visuals.menu_corner_radius = egui::CornerRadius::same(6);
        s.visuals.window_shadow = egui::Shadow {
            offset: [0, 6],
            blur: 24,
            spread: 0,
            color: egui::Color32::from_black_alpha(110),
        };
        s.visuals.popup_shadow = s.visuals.window_shadow;
        s.visuals.widgets.noninteractive.fg_stroke.color = egui::Color32::from_gray(228);
        s.visuals.widgets.open.fg_stroke.color = egui::Color32::WHITE;
        s.visuals.weak_text_color = Some(egui::Color32::from_gray(178));
        s.spacing.window_margin = egui::Margin::same(14);
        s.spacing.item_spacing = egui::vec2(8.0, 7.0);
        s.visuals.selection.bg_fill = egui::Color32::from_rgb(55, 100, 180);
        s.visuals.selection.stroke.color = egui::Color32::WHITE;
        s.visuals.widgets.inactive.weak_bg_fill = egui::Color32::from_rgb(84, 90, 103);
        s.visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(84, 90, 103);
        s.visuals.extreme_bg_color = egui::Color32::from_rgb(26, 28, 32);
    });
}

/// Ends a command-line run: exit code 0, or print the error and exit with 1.
fn exit_with(r: Result<(), String>) -> ! {
    if let Err(e) = r {
        eprintln!("{e}");
        std::process::exit(1);
    }
    std::process::exit(0)
}
