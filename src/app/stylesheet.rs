//! The Stylesheet window (File > Stylesheet, F7): five tabs of settings for how
//! the score looks (Page & Score format, Systems & Staves, Header & Footer,
//! Texts & Styles, Notation), ready-made Rock, Classic and Jazz styles, and
//! saving and loading styles. Changes are made on a copy and reach the song
//! with Apply or OK.

use super::App;
use crate::style::{FONTS, FontSpec, HeaderItem, PAGE_FORMATS, STYLE_NAMES, Stylesheet, TOKENS};
use egui::{
    Align, Align2, Color32, FontFamily, FontId, Layout, Pos2, Rect, RichText, Sense, Stroke, Ui,
    Vec2,
};
use std::path::PathBuf;

const BLUE: Color32 = Color32::from_rgb(61, 123, 224);
const TAB_BG: Color32 = Color32::from_rgb(76, 82, 100);
const BODY_BG: Color32 = Color32::from_rgb(38, 40, 48);
const LABEL_W: f32 = 230.0;
const WIN_W: f32 = 760.0;

pub const TABS: [&str; 5] = [
    "Page & Score format",
    "Systems & Staves",
    "Header & Footer",
    "Texts & Styles",
    "Notation",
];

/// The window's state: the copy being edited and what is showing.
pub struct StyleDlg {
    pub tab: usize,
    pub draft: Stylesheet,
    /// Header & Footer: editing the other pages instead of the first.
    pub other_pages: bool,
    /// Texts & Styles: the symbol label being edited.
    pub symbol: usize,
    pub note: String,
}

fn styles_dir() -> Option<PathBuf> {
    let base = std::env::var_os("APPDATA").or_else(|| std::env::var_os("HOME"))?;
    Some(PathBuf::from(base).join("GuitarCraft").join("Styles"))
}

fn default_style_file() -> Option<PathBuf> {
    let base = std::env::var_os("APPDATA").or_else(|| std::env::var_os("HOME"))?;
    Some(
        PathBuf::from(base)
            .join("GuitarCraft")
            .join("default-style.json"),
    )
}

/// The style chosen for new songs (the standard one unless the person set another).
pub fn default_style() -> Stylesheet {
    default_style_file()
        .and_then(|f| std::fs::read(f).ok())
        .and_then(|d| serde_json::from_slice::<Stylesheet>(&d).ok())
        .map(Stylesheet::fixed)
        .unwrap_or_else(Stylesheet::standard)
}

fn saved_styles() -> Vec<PathBuf> {
    let Some(d) = styles_dir() else { return vec![] };
    let mut v: Vec<PathBuf> = std::fs::read_dir(d)
        .map(|r| {
            r.filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.extension().map(|e| e == "json").unwrap_or(false))
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

fn load_style(p: &std::path::Path) -> Result<Stylesheet, String> {
    let d = std::fs::read(p).map_err(|e| e.to_string())?;
    serde_json::from_slice::<Stylesheet>(&d)
        .map(Stylesheet::fixed)
        .map_err(|e| format!("Not a stylesheet: {e}"))
}

// ----- small widgets in the window's look -----

fn heading(ui: &mut Ui, t: &str) {
    ui.add_space(10.0);
    ui.label(
        RichText::new(t)
            .size(17.0)
            .family(FontFamily::Name("bold".into()))
            .color(Color32::from_gray(240)),
    );
    ui.add_space(4.0);
}

fn subheading(ui: &mut Ui, t: &str) {
    ui.add_space(4.0);
    ui.label(RichText::new(t).size(14.0).color(Color32::from_gray(225)));
}

/// A settings row: the label right-aligned in a fixed column, then the controls.
fn row(ui: &mut Ui, label: &str, add: impl FnOnce(&mut Ui)) {
    row_w(ui, LABEL_W, label, add);
}

fn row_w(ui: &mut Ui, w: f32, label: &str, add: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(
            Vec2::new(w, 22.0),
            Layout::right_to_left(Align::Center),
            |ui| {
                ui.set_min_width(w);
                ui.label(
                    RichText::new(label)
                        .size(13.0)
                        .color(Color32::from_gray(220)),
                );
            },
        );
        add(ui);
    });
}

/// An On/Off switch. Returns true when flipped.
fn switch(ui: &mut Ui, on: &mut bool) -> bool {
    let (r, resp) = ui.allocate_exact_size(Vec2::new(32.0, 17.0), Sense::click());
    if resp.clicked() {
        *on = !*on;
    }
    let p = ui.painter();
    p.rect_filled(
        r,
        8.5,
        if *on {
            BLUE
        } else {
            Color32::from_rgb(70, 74, 84)
        },
    );
    let cx = if *on { r.right() - 8.5 } else { r.left() + 8.5 };
    p.circle_filled(Pos2::new(cx, r.center().y), 6.0, Color32::WHITE);
    ui.label(if *on { "On" } else { "Off" });
    resp.clicked()
}

/// A drop-down list of choices.
fn choice(ui: &mut Ui, id: &str, v: &mut u8, names: &[&str], width: f32) {
    let cur = names.get(*v as usize).copied().unwrap_or("");
    egui::ComboBox::from_id_salt(id)
        .selected_text(cur)
        .width(width)
        .show_ui(ui, |ui| {
            for (k, n) in names.iter().enumerate() {
                ui.selectable_value(v, k as u8, *n);
            }
        });
}

fn number(
    ui: &mut Ui,
    v: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    speed: f64,
    decimals: usize,
    suffix: &str,
) {
    ui.add(
        egui::DragValue::new(v)
            .range(range)
            .speed(speed)
            .fixed_decimals(decimals)
            .suffix(suffix),
    );
}

/// A checkbox for a setting the app keeps and saves but doesn't draw yet
/// (shown dimmer, with a note when hovered).
fn later_check(ui: &mut Ui, v: &mut bool, label: &str) {
    ui.checkbox(v, RichText::new(label).color(Color32::from_gray(150)))
        .on_hover_text("Kept with the stylesheet and saved; the score doesn't show this yet.");
}

/// Font family, size and style of a text.
fn font_picker(ui: &mut Ui, id: &str, f: &mut FontSpec) {
    ui.spacing_mut().item_spacing.x = 4.0;
    let fam_id = FontId::new(
        14.0,
        FontFamily::Name(
            format!(
                "{}|0",
                if FONTS.iter().any(|x| x.0 == f.family) {
                    f.family.as_str()
                } else {
                    "Times New Roman"
                }
            )
            .into(),
        ),
    );
    ui.label(RichText::new("Aa").font(fam_id));
    egui::ComboBox::from_id_salt((id, "fam"))
        .selected_text(f.family.clone())
        .width(135.0)
        .show_ui(ui, |ui| {
            for (name, _) in FONTS {
                let fid = FontId::new(14.0, FontFamily::Name(format!("{name}|0").into()));
                if ui
                    .selectable_label(f.family == name, RichText::new(name).font(fid))
                    .clicked()
                {
                    f.family = name.to_string();
                }
            }
        });
    ui.add(
        egui::DragValue::new(&mut f.size)
            .range(4.0..=72.0)
            .speed(0.1)
            .fixed_decimals(1)
            .suffix(" pt"),
    );
    let mut k = f.style_index() as u8;
    choice(ui, &format!("{id}-style"), &mut k, &STYLE_NAMES, 90.0);
    f.set_style_index(k as usize);
}

/// Left, centre and right alignment buttons.
fn align_buttons(ui: &mut Ui, id: &str, a: &mut u8) {
    ui.spacing_mut().item_spacing.x = 1.0;
    for k in 0..3u8 {
        let (r, resp) = ui.allocate_exact_size(Vec2::new(26.0, 22.0), Sense::click());
        let resp = resp.on_hover_text(["Left", "Centre", "Right"][k as usize]);
        if resp.clicked() {
            *a = k;
        }
        let p = ui.painter();
        p.rect_filled(
            r,
            3.0,
            if *a == k {
                BLUE
            } else {
                Color32::from_rgb(70, 74, 84)
            },
        );
        // Four short lines, lined up like the text would be.
        for i in 0..4 {
            let w = if i % 2 == 0 { 14.0 } else { 9.0 };
            let y = r.top() + 6.0 + i as f32 * 3.4;
            let x0 = match k {
                0 => r.left() + 6.0,
                1 => r.center().x - w * 0.5,
                _ => r.right() - 6.0 - w,
            };
            p.line_segment(
                [Pos2::new(x0, y), Pos2::new(x0 + w, y)],
                Stroke::new(1.2, Color32::WHITE),
            );
        }
    }
    let _ = id;
}

fn header_items(ui: &mut Ui, id: &str, items: &mut [HeaderItem]) {
    for (k, it) in items.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            ui.allocate_ui_with_layout(
                Vec2::new(150.0, 24.0),
                Layout::left_to_right(Align::Center),
                |ui| {
                    ui.set_min_width(150.0);
                    ui.checkbox(&mut it.on, RichText::new(it.name.clone()).size(13.0));
                },
            );
            ui.add_enabled_ui(it.on, |ui| {
                ui.vertical(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut it.text).desired_width(420.0));
                    ui.horizontal(|ui| {
                        align_buttons(ui, &format!("{id}{k}"), &mut it.align);
                        ui.add_space(8.0);
                        font_picker(ui, &format!("{id}-font{k}"), &mut it.font);
                    });
                });
            });
        });
        ui.add_space(6.0);
    }
}

impl App {
    pub(super) fn open_stylesheet(&mut self) {
        self.style_dlg = Some(StyleDlg {
            tab: 0,
            draft: self.song.style.clone(),
            other_pages: false,
            symbol: 0,
            note: String::new(),
        });
    }

    fn apply_style(&mut self, st: Stylesheet) {
        if st != self.song.style {
            self.snapshot();
            self.song.style = st;
            self.status = "Stylesheet applied.".into();
        }
    }

    pub(super) fn stylesheet_window(&mut self, ctx: &egui::Context) {
        let Some(mut dlg) = self.style_dlg.take() else {
            return;
        };
        let mut open = true;
        let mut action: Option<u8> = None; // 0 apply, 1 cancel, 2 ok
        // Like the original: a tall window that can be moved and resized; the
        // tabs and the button bar stay put and the settings scroll between them.
        let screen = ctx.content_rect();
        let h = (screen.height() - 40.0).clamp(360.0, 1000.0);
        egui::Window::new("Stylesheet")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .pivot(Align2::CENTER_CENTER)
            .default_pos(screen.center())
            .default_size(Vec2::new(WIN_W, h))
            .min_size(Vec2::new(420.0, 300.0))
            .constrain(true)
            .frame(egui::Frame::window(&ctx.global_style()).fill(BODY_BG).inner_margin(0.0))
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = 6.0;
                egui::Panel::top("ss-tabs").frame(egui::Frame::NONE).resizable(false).show(ui, |ui| {
                    tab_strip(ui, &mut dlg.tab);
                });
                // Bottom bar: Options on the left, OK / Cancel / Apply on the right.
                egui::Panel::bottom("ss-bar").frame(egui::Frame::new().fill(BODY_BG).inner_margin(egui::Margin::symmetric(14, 10))).resizable(false).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.menu_button(RichText::new("\u{2699} Options \u{25B4}\u{25BE}").size(14.0), |ui| {
                            for (name, f) in [("Apply Rock style", Stylesheet::rock as fn() -> Stylesheet), ("Apply Classic style", Stylesheet::classic), ("Apply Jazz style", Stylesheet::jazz), ("Apply standard style", Stylesheet::standard)] {
                                if ui.button(name).clicked() {
                                    dlg.draft = f();
                                    dlg.note = format!("{} \u{2013} press Apply or OK to use it.", name.trim_start_matches("Apply "));
                                    ui.close();
                                }
                            }
                            let saved = saved_styles();
                            if !saved.is_empty() {
                                ui.separator();
                                for p in saved {
                                    let name = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                                    if ui.button(format!("Apply \u{201C}{name}\u{201D}")).clicked() {
                                        match load_style(&p) {
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
                                if self.screenshot.is_none() {
                                    let mut d = rfd::FileDialog::new().add_filter("Stylesheet", &["json"]);
                                    if let Some(dir) = styles_dir() {
                                        d = d.set_directory(dir);
                                    }
                                    if let Some(p) = d.pick_file() {
                                        match load_style(&p) {
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
                                if self.screenshot.is_none() {
                                    let mut d = rfd::FileDialog::new().add_filter("Stylesheet", &["json"]).set_file_name("My style.json");
                                    if let Some(dir) = styles_dir() {
                                        let _ = std::fs::create_dir_all(&dir);
                                        d = d.set_directory(dir);
                                    }
                                    if let Some(p) = d.save_file() {
                                        let r = serde_json::to_string_pretty(&dlg.draft).map_err(|e| e.to_string()).and_then(|s| std::fs::write(&p, s).map_err(|e| e.to_string()));
                                        dlg.note = match r {
                                            Ok(()) => format!("Saved {}.", p.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default()),
                                            Err(e) => format!("Couldn't save the style: {e}"),
                                        };
                                    }
                                }
                            }
                            ui.separator();
                            if ui.button("Use for new songs").on_hover_text("New songs will start with this style.").clicked() {
                                ui.close();
                                if let Some(f) = default_style_file() {
                                    if let Some(dir) = f.parent() {
                                        let _ = std::fs::create_dir_all(dir);
                                    }
                                    let r = serde_json::to_string_pretty(&dlg.draft).map_err(|e| e.to_string()).and_then(|s| std::fs::write(&f, s).map_err(|e| e.to_string()));
                                    dlg.note = match r {
                                        Ok(()) => "New songs will use this style.".into(),
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
                    // Solid scroll bars (shown when needed), so it is clear there is more to see.
                    ui.spacing_mut().scroll = egui::style::ScrollStyle::solid();
                    egui::ScrollArea::both().id_salt(("ss-body", dlg.tab)).auto_shrink([false, false]).scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded).show(ui, |ui| {
                        egui::Frame::new().inner_margin(egui::Margin { left: 40, right: 24, top: 4, bottom: 12 }).show(ui, |ui| {
                            // The rows take the room they need: a sideways scroll bar appears
                            // only when the window is made narrower than them.
                            match dlg.tab {
                                0 => page_tab(ui, &mut dlg.draft),
                                1 => systems_tab(ui, &mut dlg.draft),
                                2 => header_tab(ui, &mut dlg),
                                3 => texts_tab(ui, &mut dlg),
                                _ => notation_tab(ui, &mut dlg.draft),
                            }
                        });
                    });
                });
            });
        match action {
            Some(0) => {
                self.apply_style(dlg.draft.clone());
                dlg.note = "Applied.".into();
            }
            Some(2) => {
                self.apply_style(dlg.draft.clone());
                return;
            }
            Some(_) => return,
            None => {}
        }
        if open {
            self.style_dlg = Some(dlg);
        }
    }
}

/// The five big tabs with their pictures.
fn tab_strip(ui: &mut Ui, tab: &mut usize) {
    let full = ui.available_rect_before_wrap();
    let w = full.width() / TABS.len() as f32;
    let (strip, _) = ui.allocate_exact_size(Vec2::new(full.width(), 64.0), Sense::hover());
    for (i, name) in TABS.iter().enumerate() {
        let r = Rect::from_min_size(
            Pos2::new(strip.left() + i as f32 * w, strip.top()),
            Vec2::new(w - 1.0, 64.0),
        );
        let resp = ui.interact(r, ui.id().with(("ss-tab", i)), Sense::click());
        if resp.clicked() {
            *tab = i;
        }
        let sel = *tab == i;
        let p = ui.painter();
        p.rect_filled(
            r,
            0.0,
            if sel {
                BLUE
            } else if resp.hovered() {
                Color32::from_rgb(90, 96, 116)
            } else {
                TAB_BG
            },
        );
        let c = Color32::WHITE;
        let ic = Pos2::new(r.center().x, r.top() + 22.0);
        let st = Stroke::new(1.3, c);
        match i {
            0 => {
                // A page with lines of music.
                let pr = Rect::from_center_size(ic, Vec2::new(16.0, 20.0));
                p.rect_stroke(pr, 1.0, st, egui::StrokeKind::Middle);
                for k in 0..4 {
                    let y = pr.top() + 5.0 + k as f32 * 3.6;
                    p.line_segment(
                        [
                            Pos2::new(pr.left() + 3.0, y),
                            Pos2::new(pr.right() - 3.0, y),
                        ],
                        Stroke::new(1.0, c),
                    );
                }
            }
            1 => {
                // A brace before staff lines.
                p.text(
                    Pos2::new(ic.x - 10.0, ic.y),
                    Align2::CENTER_CENTER,
                    "{",
                    FontId::proportional(22.0),
                    c,
                );
                for k in 0..4 {
                    let y = ic.y - 7.0 + k as f32 * 4.6;
                    p.line_segment(
                        [Pos2::new(ic.x - 3.0, y), Pos2::new(ic.x + 14.0, y)],
                        Stroke::new(1.2, c),
                    );
                }
            }
            2 => {
                // A page with a header and a footer bar.
                let pr = Rect::from_center_size(ic, Vec2::new(16.0, 22.0));
                p.rect_stroke(pr, 2.0, st, egui::StrokeKind::Middle);
                p.line_segment(
                    [
                        Pos2::new(pr.left() + 3.0, pr.top() + 4.0),
                        Pos2::new(pr.right() - 3.0, pr.top() + 4.0),
                    ],
                    Stroke::new(1.6, c),
                );
                p.line_segment(
                    [
                        Pos2::new(pr.left() + 3.0, pr.bottom() - 4.0),
                        Pos2::new(pr.right() - 3.0, pr.bottom() - 4.0),
                    ],
                    Stroke::new(1.6, c),
                );
            }
            3 => {
                p.text(
                    ic,
                    Align2::CENTER_CENTER,
                    "A",
                    FontId::new(26.0, FontFamily::Name("Times New Roman|1".into())),
                    c,
                );
            }
            _ => {
                p.text(
                    ic,
                    Align2::CENTER_CENTER,
                    "\u{1D11E}",
                    FontId::new(26.0, FontFamily::Name("music".into())),
                    c,
                );
            }
        }
        p.text(
            Pos2::new(r.center().x, r.bottom() - 13.0),
            Align2::CENTER_CENTER,
            *name,
            FontId::proportional(13.5),
            c,
        );
    }
}

fn page_tab(ui: &mut Ui, s: &mut Stylesheet) {
    heading(ui, "PAGE");
    row(ui, "Format:", |ui| {
        let cur = if s.page_format < PAGE_FORMATS.len() {
            let (n, w, h) = PAGE_FORMATS[s.page_format];
            format!("{n} ({}x{}mm)", w.round(), h.round())
        } else {
            "Custom".to_string()
        };
        egui::ComboBox::from_id_salt("ss-format")
            .selected_text(cur)
            .width(170.0)
            .show_ui(ui, |ui| {
                for (k, (n, w, h)) in PAGE_FORMATS.iter().enumerate() {
                    if ui
                        .selectable_label(
                            s.page_format == k,
                            format!("{n} ({}x{}mm)", w.round(), h.round()),
                        )
                        .clicked()
                    {
                        s.page_format = k;
                        s.page_w = *w;
                        s.page_h = *h;
                    }
                }
                if ui
                    .selectable_label(s.page_format >= PAGE_FORMATS.len(), "Custom")
                    .clicked()
                {
                    s.page_format = PAGE_FORMATS.len();
                }
            });
        ui.label("=");
        let (mut w, mut h) = (s.page_w, s.page_h);
        let a = ui
            .add(
                egui::DragValue::new(&mut w)
                    .range(50.0..=600.0)
                    .speed(1.0)
                    .fixed_decimals(0),
            )
            .changed();
        ui.label("x");
        let b = ui
            .add(
                egui::DragValue::new(&mut h)
                    .range(50.0..=800.0)
                    .speed(1.0)
                    .fixed_decimals(0),
            )
            .changed();
        if a || b {
            s.page_w = w;
            s.page_h = h;
            s.page_format = PAGE_FORMATS
                .iter()
                .position(|f| (f.1 - w).abs() < 0.6 && (f.2 - h).abs() < 0.6)
                .unwrap_or(PAGE_FORMATS.len());
        }
    });
    row(ui, "Orientation:", |ui| {
        ui.radio_value(&mut s.landscape, false, "Portrait");
        ui.radio_value(&mut s.landscape, true, "Landscape");
    });
    // Margins around a small picture of the page.
    row(ui, "Margins:", |ui| {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.add_space(96.0);
                ui.label("Top");
                number(ui, &mut s.margin_top, 0.0..=80.0, 0.5, 0, "");
            });
            ui.horizontal(|ui| {
                ui.label("Left");
                number(ui, &mut s.margin_left, 0.0..=80.0, 0.5, 0, "");
                let (pw, ph) = s.page_mm();
                let k = 52.0 / pw.max(ph);
                let (r, _) =
                    ui.allocate_exact_size(Vec2::new(pw * k + 8.0, ph * k), Sense::hover());
                let pr = Rect::from_center_size(r.center(), Vec2::new(pw * k, ph * k));
                let p = ui.painter();
                p.rect_filled(pr, 0.0, Color32::WHITE);
                let inner = Rect::from_min_max(
                    Pos2::new(pr.left() + s.margin_left * k, pr.top() + s.margin_top * k),
                    Pos2::new(
                        pr.right() - s.margin_right * k,
                        pr.bottom() - s.margin_bottom * k,
                    ),
                );
                p.rect_stroke(inner, 0.0, Stroke::new(1.0, BLUE), egui::StrokeKind::Middle);
                number(ui, &mut s.margin_right, 0.0..=80.0, 0.5, 0, "");
                ui.label("Right");
            });
            ui.horizontal(|ui| {
                ui.add_space(96.0);
                ui.label("Bottom");
                number(ui, &mut s.margin_bottom, 0.0..=80.0, 0.5, 0, "");
            });
        });
    });

    heading(ui, "SIZES");
    row(ui, "Global score proportions:", |ui| {
        number(ui, &mut s.proportion, 0.8..=4.0, 0.01, 2, " mm");
    });
    row(ui, "Affects fonts and chords sizes:", |ui| {
        switch(ui, &mut s.scale_fonts);
    });
    row(ui, "Rhythm proportion:", |ui| {
        number(ui, &mut s.rhythm_spacing, 0.0..=1.0, 0.01, 2, "");
    });

    heading(ui, "TUNING");
    row(ui, "Display tuning:", |ui| {
        switch(ui, &mut s.tuning_show);
    });
    ui.add_enabled_ui(s.tuning_show, |ui| {
        row(ui, "Position:", |ui| {
            choice(
                ui,
                "ss-tpos",
                &mut s.tuning_pos,
                &["On Top of Score", "Before Track"],
                170.0,
            )
        });
        if s.tuning_pos == 0 {
            row(ui, "Mode:", |ui| {
                choice(
                    ui,
                    "ss-tmode",
                    &mut s.tuning_mode,
                    &["Name Only", "Name and Notes", "Unusually Tuned Strings"],
                    220.0,
                )
            });
            row(ui, "Columns:", |ui| {
                choice(
                    ui,
                    "ss-tcols",
                    &mut s.tuning_cols,
                    &["Two Columns", "Horizontally"],
                    150.0,
                )
            });
            row(ui, "Framed:", |ui| {
                switch(ui, &mut s.tuning_framed);
            });
        }
    });

    heading(ui, "CHORD DIAGRAMS");
    row(ui, "Notation:", |ui| {
        ui.radio_value(&mut s.chord_style, 0, "Classic");
        ui.radio_value(&mut s.chord_style, 1, "Jazz");
        ui.radio_value(&mut s.chord_style, 2, "Rock");
    });
    subheading(ui, "On Top of Score");
    row(ui, "Visible:", |ui| {
        switch(ui, &mut s.chords_top);
    });
    row(ui, "Size:", |ui| {
        number(ui, &mut s.chords_top_size, 5.0..=20.0, 0.1, 2, "mm")
    });
    row(ui, "Spacing:", |ui| {
        number(ui, &mut s.chords_top_spacing, 0.0..=20.0, 0.1, 2, "mm")
    });
    subheading(ui, "In Score");
    row(ui, "Visible:", |ui| {
        switch(ui, &mut s.chords_in_score);
    });
    row(ui, "Size:", |ui| {
        number(ui, &mut s.chords_in_score_size, 5.0..=20.0, 0.1, 2, "mm")
    });
}

fn systems_tab(ui: &mut Ui, s: &mut Stylesheet) {
    heading(ui, "STAVES");
    row(ui, "Staff line thickness:", |ui| {
        number(ui, &mut s.staff_line, 0.05..=0.5, 0.005, 2, "sp")
    });
    row(ui, "Spacing between systems:", |ui| {
        number(ui, &mut s.system_gap, 0.0..=40.0, 0.1, 2, "mm")
    });
    ui.horizontal(|ui| {
        ui.checkbox(&mut s.indent_on, "First system indentation");
        ui.add_enabled_ui(s.indent_on, |ui| {
            number(ui, &mut s.indent, 0.0..=60.0, 0.1, 2, "mm")
        });
    });
    later_check(
        ui,
        &mut s.slash_lines,
        "Display stave lines when using slashes notation",
    );
    later_check(
        ui,
        &mut s.slash_barlines,
        "Display barlines when using slashes notation",
    );
    ui.checkbox(
        &mut s.extended_barlines,
        "Display extended barlines in system",
    );
    ui.checkbox(
        &mut s.clef_each_system,
        "Display clef at the beginning of each system",
    );
    ui.checkbox(
        &mut s.time_each_system,
        "Display time signature at the beginning of each system",
    );
    ui.checkbox(
        &mut s.time_each_section,
        "Display time signature at the beginning of section",
    );
    ui.checkbox(
        &mut s.capo_affects_std,
        "Capo and shifted tunings impact standard notation",
    );

    heading(ui, "BRACKETS");
    ui.horizontal(|ui| {
        ui.label("Mode:");
        choice(
            ui,
            "ss-brackets",
            &mut s.brackets,
            &[
                "Do Not Draw Brackets",
                "Link Tablature and Standard Notation",
                "Group Similar Instruments",
            ],
            300.0,
        );
    });
    ui.checkbox(&mut s.separator, "Use // symbol to separate systems");

    heading(ui, "TRACK NAME");
    let policies = ["First System", "First System of Each Page", "All Systems"];
    subheading(ui, "Single track");
    row(ui, "Display:", |ui| {
        switch(ui, &mut s.name_single);
    });
    row(ui, "Policy:", |ui| {
        choice(ui, "ss-np1", &mut s.name_single_policy, &policies, 220.0)
    });
    subheading(ui, "Multitrack");
    row(ui, "Display:", |ui| {
        switch(ui, &mut s.name_multi);
    });
    row(ui, "Policy:", |ui| {
        choice(ui, "ss-np2", &mut s.name_multi_policy, &policies, 220.0)
    });
    subheading(ui, "First System");
    row(ui, "Mode:", |ui| {
        choice(
            ui,
            "ss-nm1",
            &mut s.name_first_mode,
            &["Full name", "Abbreviated name"],
            170.0,
        )
    });
    row(ui, "Orientation:", |ui| {
        choice(
            ui,
            "ss-nor",
            &mut s.name_first_orient,
            &["Vertical", "Horizontal"],
            130.0,
        )
    });
    subheading(ui, "Other systems");
    row(ui, "Mode:", |ui| {
        choice(
            ui,
            "ss-nm2",
            &mut s.name_other_mode,
            &["Full name", "Abbreviated name"],
            170.0,
        )
    });
}

fn header_tab(ui: &mut Ui, dlg: &mut StyleDlg) {
    // First page / other pages.
    ui.horizontal(|ui| {
        ui.add_space(150.0);
        ui.spacing_mut().item_spacing.x = 0.0;
        for (k, lab) in ["\u{1F5CE} First Page", "\u{1F5D0} Other Pages"]
            .iter()
            .enumerate()
        {
            let sel = dlg.other_pages == (k == 1);
            if ui
                .add(
                    egui::Button::new(RichText::new(*lab).size(14.0).color(Color32::WHITE))
                        .fill(if sel {
                            BLUE
                        } else {
                            Color32::from_rgb(70, 74, 84)
                        })
                        .min_size(Vec2::new(150.0, 28.0)),
                )
                .clicked()
            {
                dlg.other_pages = k == 1;
            }
        }
    });
    let s = &mut dlg.draft;
    let (hdr, ftr) = if dlg.other_pages {
        (&mut s.other_header, &mut s.other_footer)
    } else {
        (&mut s.first_header, &mut s.first_footer)
    };
    heading(ui, "HEADER");
    header_items(ui, if dlg.other_pages { "oh" } else { "fh" }, hdr);
    heading(ui, "FOOTER");
    header_items(ui, if dlg.other_pages { "of" } else { "ff" }, ftr);
    ui.add_space(6.0);
    ui.label(
        RichText::new(format!(
            "Tokens filled in from the song's information: {}",
            TOKENS.join("  ")
        ))
        .size(12.0)
        .color(Color32::from_gray(160)),
    );
    ui.label(RichText::new("A line is left out when its song information is empty. Edit the information in the Song panel (F5).").size(12.0).color(Color32::from_gray(160)));
}

fn texts_tab(ui: &mut Ui, dlg: &mut StyleDlg) {
    let s = &mut dlg.draft;
    heading(ui, "TEXTS");
    row_w(ui, 190.0, "Fret numbers:", |ui| {
        font_picker(ui, "t-fret", &mut s.font_fret)
    });
    row_w(ui, 190.0, "Grace note fret numbers:", |ui| {
        font_picker(ui, "t-grace", &mut s.font_grace)
    });
    row_w(ui, 190.0, "Texts:", |ui| {
        font_picker(ui, "t-text", &mut s.font_text)
    });
    row_w(ui, 190.0, "Chord names:", |ui| {
        font_picker(ui, "t-chord", &mut s.font_chord)
    });
    row_w(ui, 190.0, "Tuning:", |ui| {
        font_picker(ui, "t-tuning", &mut s.font_tuning)
    });
    row_w(ui, 190.0, "Sections:", |ui| {
        font_picker(ui, "t-sect", &mut s.font_section)
    });
    row_w(ui, 190.0, "Tempo:", |ui| {
        font_picker(ui, "t-tempo", &mut s.font_tempo);
        ui.checkbox(&mut s.tempo_decimal, "Show decimal");
    });
    row_w(ui, 190.0, "Timers:", |ui| {
        ui.add_enabled_ui(true, |ui| font_picker(ui, "t-timer", &mut s.font_timer))
            .response
            .on_hover_text("Kept with the stylesheet; the score doesn't show timers yet.");
    });
    row_w(ui, 190.0, "Capo:", |ui| {
        font_picker(ui, "t-capo", &mut s.font_capo)
    });
    row_w(ui, 190.0, "Barre chords:", |ui| {
        ui.add_enabled_ui(true, |ui| font_picker(ui, "t-barre", &mut s.font_barre))
            .response
            .on_hover_text("Kept with the stylesheet; the score doesn't show barre marks yet.");
    });
    row_w(ui, 190.0, "Horizontal track names:", |ui| {
        font_picker(ui, "t-trh", &mut s.font_track_h)
    });
    row_w(ui, 190.0, "Vertical track names:", |ui| {
        font_picker(ui, "t-trv", &mut s.font_track_v)
    });
    row_w(ui, 190.0, "Lyrics:", |ui| {
        font_picker(ui, "t-lyr", &mut s.font_lyrics);
        choice(
            ui,
            "t-lyrpos",
            &mut s.lyrics_pos,
            &["Below Staff", "Above Staff", "Hidden"],
            110.0,
        );
    });
    row_w(ui, 190.0, "Bar numbers:", |ui| {
        font_picker(ui, "t-bar", &mut s.font_bar_number);
        choice(
            ui,
            "t-barpos",
            &mut s.bar_numbers,
            &["All bars", "First bar of each system", "Hidden"],
            150.0,
        );
    });

    heading(ui, "MUSICAL SYMBOLS");
    row_w(ui, 190.0, "Global style:", |ui| {
        ui.radio_value(&mut s.symbol_style, 0, "Classic");
        ui.radio_value(&mut s.symbol_style, 1, "Jazz");
    });
    let k = dlg.symbol.min(s.symbols.len().saturating_sub(1));
    ui.horizontal(|ui| {
        let names: Vec<String> = s.symbols.iter().map(|x| x.name.clone()).collect();
        egui::ComboBox::from_id_salt("t-sym")
            .selected_text(names.get(k).cloned().unwrap_or_default())
            .width(190.0)
            .show_ui(ui, |ui| {
                for (i, n) in names.iter().enumerate() {
                    ui.selectable_value(&mut dlg.symbol, i, n);
                }
            });
        if let Some(sym) = s.symbols.get_mut(k) {
            font_picker(ui, "t-symfont", &mut sym.font);
            ui.add(egui::TextEdit::singleline(&mut sym.text).desired_width(110.0));
        }
    });
    if k >= crate::style::SYM_SCRAPE {
        ui.label(RichText::new("This label is kept with the stylesheet; the score doesn't draw this technique yet.").size(12.0).color(Color32::from_gray(150)));
    }
    row_w(ui, 190.0, "Symbol lines extend over rests:", |ui| {
        switch(ui, &mut s.lines_over_rests);
    });
}

fn notation_tab(ui: &mut Ui, s: &mut Stylesheet) {
    heading(ui, "RHYTHMIC");
    ui.checkbox(
        &mut s.rhythm_hide_with_std,
        "Hide in tablature when using standard notation",
    );
    ui.checkbox(&mut s.rhythm_extend, "Extend rhythmic inside tablature");
    later_check(
        ui,
        &mut s.beams_on_rests,
        "Display beams on rests in tablature notation",
    );
    ui.checkbox(
        &mut s.circle_half_whole,
        "Display circle around half and whole notes in tablature",
    );
    later_check(
        ui,
        &mut s.horizontal_beams,
        "Force horizontal beams in standard notation",
    );
    later_check(
        ui,
        &mut s.horizontal_beams_drums,
        "Force horizontal beams on drum notation",
    );
    subheading(ui, "Position in tablature");
    for v in 0..4 {
        ui.horizontal(|ui| {
            ui.label(format!("Voice {}:", v + 1));
            choice(
                ui,
                &format!("n-rv{v}"),
                &mut s.rhythm_voice[v],
                &["Above", "Below", "Hidden"],
                90.0,
            );
        });
    }

    heading(ui, "SYMBOLS");
    later_check(
        ui,
        &mut s.harmonic_string,
        "Indicate where the string is touched for artificial harmonics in tablature",
    );
    later_check(
        ui,
        &mut s.harmonic_dot,
        "Alternate representation \".\" for artificial octave harmonics in tablature",
    );
    ui.checkbox(
        &mut s.common_time_c,
        "Alternate representation \"C\" for 4/4 and 2/2 signatures",
    );
    later_check(
        ui,
        &mut s.bend_real_pitch,
        "Display real pitch of heard notes for bend effects in standard notation",
    );
    ui.checkbox(&mut s.arpeggio_arrow, "Display arpeggio arrow");
    ui.checkbox(&mut s.trill_second, "Display second note for trill effect");
    ui.checkbox(&mut s.tied_in_tab, "Always display tied notes on tablature");
    ui.horizontal(|ui| {
        later_check(
            ui,
            &mut s.coda_space_on,
            "Add spacing between bars for Coda of",
        );
        number(ui, &mut s.coda_space, 0.0..=40.0, 0.5, 0, " mm");
    });
    ui.checkbox(&mut s.hopo_letters, "Add \"H/P\" symbols for HoPo");
    later_check(
        ui,
        &mut s.hopo_avoid,
        "Avoid notes collisions for HoPo slur in tablature",
    );
    later_check(
        ui,
        &mut s.hopo_outer,
        "Display HoPo slur only on outer notes in tablature",
    );
    later_check(
        ui,
        &mut s.hopo_single,
        "Display a single HoPo slur over a group of notes in tablature",
    );
    ui.checkbox(&mut s.slide_sl, "Add \"sl.\" symbol for slide effect");
    ui.checkbox(
        &mut s.color_voices,
        "Colorize fret numbers in tablature for voices 2, 3 and 4",
    );
    later_check(
        ui,
        &mut s.hide_useless_rests,
        "Hide useless rests on standard notation",
    );
    ui.checkbox(
        &mut s.dash_quarter_rest,
        "Alternate representation \"-\" for quarter rests",
    );
    ui.checkbox(&mut s.hide_dynamics, "Hide dynamics");
    later_check(
        ui,
        &mut s.slash_fret_string,
        "Show fret and string for single notes in slash notation",
    );
    ui.checkbox(&mut s.tuplet_brackets, "Show tuplet brackets");
    row(ui, "Scale diagrams grid spacing:", |ui| {
        number(ui, &mut s.scale_grid, 0.5..=3.0, 0.01, 2, "")
    });

    heading(ui, "FINGERINGS");
    ui.label(RichText::new("Fingering marks can't be added to notes in this app yet; these choices are kept for when they can.").size(12.0).color(Color32::from_gray(150)));
    subheading(ui, "Left hand");
    row(ui, "Fingering:", |ui| {
        choice(ui, "n-lh", &mut s.lh_type, &["T1234", "P1234"], 90.0)
    });
    row(ui, "Scale:", |ui| {
        number(ui, &mut s.lh_scale, 0.5..=2.0, 0.01, 2, "")
    });
    row(ui, "Position in staff:", |ui| {
        choice(
            ui,
            "n-lhs",
            &mut s.lh_staff,
            &["Above", "Before Note", "Below"],
            120.0,
        )
    });
    row(ui, "Position in tablature:", |ui| {
        choice(ui, "n-lht", &mut s.lh_tab, &["Above", "Below"], 90.0)
    });
    subheading(ui, "Right hand");
    row(ui, "Fingering:", |ui| {
        choice(
            ui,
            "n-rh",
            &mut s.rh_type,
            &["pimac", "pimax", "pimae", "timao"],
            90.0,
        )
    });
    row(ui, "Scale:", |ui| {
        number(ui, &mut s.rh_scale, 0.5..=2.0, 0.01, 2, "")
    });
    row(ui, "Position in staff:", |ui| {
        choice(
            ui,
            "n-rhs",
            &mut s.rh_staff,
            &["Above", "Before Note", "Below"],
            120.0,
        )
    });
    row(ui, "Position in tablature:", |ui| {
        choice(ui, "n-rht", &mut s.rh_tab, &["Above", "Below"], 90.0)
    });
}
