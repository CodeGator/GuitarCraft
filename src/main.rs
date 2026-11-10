//! GuitarCraft: a guitar and bass tablature editor.

mod app;
mod audio;
mod fileio;
mod fretboard;
mod gp5write;
mod gpif;
mod gpifwrite;
mod gpimport;
mod gpsv;
mod model;
mod score;
mod style;
mod textmidi;

use egui::{FontData, FontDefinitions, FontFamily};
use std::path::PathBuf;
use std::sync::Arc;

fn main() -> eframe::Result {
    let mut launch = app::LaunchOptions::default();
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        if a == "--convert" {
            // Save a song (or "demo") in another format and exit: --convert <from> <to>.
            let from = args.next().unwrap_or_default();
            let to = args.next().unwrap_or_default();
            let song = if from == "demo" {
                Ok(model::Song::demo())
            } else {
                fileio::load_song(std::path::Path::new(&from))
            };
            match song.and_then(|s| fileio::export_song(&s, std::path::Path::new(&to))) {
                Ok(()) => std::process::exit(0),
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            }
        }
        if a == "--gp-tests" {
            let dir = std::path::PathBuf::from(args.next().unwrap_or_else(|| ".".into()));
            let song = model::Song::demo();
            for (name, chords, views) in [
                ("Test 1 - plain", false, false),
                ("Test 2 - chord names", true, false),
                ("Test 3 - layout views", false, true),
                ("Test 4 - both", true, true),
            ] {
                let data = gpifwrite::export_gp_with(&song, gpifwrite::GpOptions { chords, views })
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
    if music {
        if let Some(mono) = fonts.families.get_mut(&FontFamily::Monospace) {
            mono.push("music".into());
        }
    }
    ctx.set_fonts(fonts);
    music
}

pub fn setup_style(ctx: &egui::Context) {
    ctx.set_theme(egui::Theme::Dark);
    ctx.style_mut_of(egui::Theme::Dark, |s| {
        s.spacing.button_padding = egui::vec2(8.0, 4.0);
        s.visuals.panel_fill = egui::Color32::from_rgb(36, 38, 43);
        s.visuals.window_fill = egui::Color32::from_rgb(44, 47, 53);
        s.visuals.selection.bg_fill = egui::Color32::from_rgb(55, 100, 180);
        s.visuals.selection.stroke.color = egui::Color32::WHITE;
        s.visuals.widgets.inactive.weak_bg_fill = egui::Color32::from_rgb(56, 60, 67);
        s.visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(56, 60, 67);
        s.visuals.extreme_bg_color = egui::Color32::from_rgb(26, 28, 32);
    });
}
