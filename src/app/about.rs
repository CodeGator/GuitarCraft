//! Help > About GuitarCraft, laid out like the original's About window but with
//! GuitarCraft's own content: the Application tab (icon, name, version,
//! copyright), the License tab (GuitarCraft is free software under the GNU GPL,
//! version 3 or later: a plain-English summary, then the full text from the
//! project's LICENSE file) and the Open Source tab (every component GuitarCraft
//! is built from, with its license; the list is made by build.rs from
//! Cargo.lock each time GuitarCraft is built, so it never needs updating by hand).
//! No license key, social media links or team page.

use super::App;
use egui::{Align, Align2, Color32, FontFamily, FontId, Layout, RichText, Sense, Vec2};
use serde::Deserialize;

const BLUE: Color32 = Color32::from_rgb(92, 150, 240);
const TABS: [&str; 3] = ["APPLICATION", "LICENSE", "OPEN SOURCE"];
/// GuitarCraft's own license (the GNU GPL, version 3).
const GPL: &str = include_str!("../../LICENSE");
const COMPONENTS: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/licenses.json.z"));

#[derive(Deserialize, Clone, Default)]
pub struct Component {
    pub name: String,
    pub version: String,
    pub description: String,
    pub license: String,
    pub link: String,
    pub text: usize,
}

#[derive(Deserialize, Default)]
pub struct Components {
    pub components: Vec<Component>,
    pub texts: Vec<String>,
}

/// The open-source components, read from what build.rs stored.
pub fn components() -> Components {
    use std::io::Read;
    let mut json = String::new();
    let mut dec = flate2::read::DeflateDecoder::new(COMPONENTS);
    if dec.read_to_string(&mut json).is_err() {
        return Components::default();
    }
    serde_json::from_str(&json).unwrap_or_default()
}

#[derive(Default)]
pub struct AboutState {
    pub tab: usize,
    /// The component shown on the Open Source tab.
    pub sel: usize,
    pub filter: String,
    data: Option<Components>,
    icon: Option<egui::TextureHandle>,
}

impl App {
    pub(super) fn about_window(&mut self, ctx: &egui::Context) {
        if !self.show_about {
            return;
        }
        let mut open = true;
        let screen = ctx.content_rect();
        let st = &mut self.about;
        egui::Window::new("About GuitarCraft")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .pivot(Align2::CENTER_CENTER)
            .default_pos(screen.center())
            .fixed_size(Vec2::new(540.0, 520.0))
            .constrain(true)
            .show(ctx, |ui| {
                // Tabs: plain words, the chosen one bright.
                ui.horizontal(|ui| {
                    let w = ui.available_width() / TABS.len() as f32;
                    for (i, name) in TABS.iter().enumerate() {
                        let (r, resp) =
                            ui.allocate_exact_size(Vec2::new(w - 4.0, 30.0), Sense::click());
                        let c = if st.tab == i {
                            Color32::WHITE
                        } else if resp.hovered() {
                            Color32::from_gray(190)
                        } else {
                            Color32::from_gray(130)
                        };
                        ui.painter().text(
                            r.center(),
                            Align2::CENTER_CENTER,
                            *name,
                            FontId::new(16.0, FontFamily::Name("bold".into())),
                            c,
                        );
                        if resp.clicked() {
                            st.tab = i;
                        }
                    }
                });
                ui.add_space(4.0);
                match st.tab {
                    0 => application_tab(ui, st),
                    1 => license_tab(ui),
                    _ => open_source_tab(ui, st),
                }
            });
        if !open || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.show_about = false;
        }
    }
}

fn application_tab(ui: &mut egui::Ui, st: &mut AboutState) {
    let icon = st
        .icon
        .get_or_insert_with(|| {
            let png: &[u8] = include_bytes!("../../assets/app-icon/codegator-256.png");
            let img = image::load_from_memory(png)
                .map(|i| i.to_rgba8())
                .unwrap_or_default();
            let size = [img.width() as usize, img.height() as usize];
            let ci = egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw());
            ui.ctx()
                .load_texture("about-icon", ci, egui::TextureOptions::LINEAR)
        })
        .clone();
    let h = ui.available_height();
    ui.allocate_ui_with_layout(Vec2::new(ui.available_width(), h), Layout::top_down(Align::Center), |ui| {
        ui.add_space(14.0);
        ui.add(egui::Image::new((icon.id(), Vec2::splat(84.0))));
        ui.add_space(6.0);
        // The name as on the start screen: "Guitar" plain, "Craft" bold blue.
        ui.horizontal(|ui| {
            let g1 = ui.painter().layout_no_wrap("Guitar".into(), FontId::new(40.0, FontFamily::Proportional), Color32::from_gray(235));
            let g2 = ui.painter().layout_no_wrap("Craft".into(), FontId::new(40.0, FontFamily::Name("bold".into())), BLUE);
            let w = g1.size().x + g2.size().x;
            ui.add_space(((ui.available_width() - w) * 0.5).max(0.0));
            let (r, _) = ui.allocate_exact_size(Vec2::new(w, g1.size().y), Sense::hover());
            let w1 = g1.size().x;
            ui.painter().galley(r.min, g1, Color32::WHITE);
            ui.painter().galley(r.min + Vec2::new(w1, 0.0), g2, Color32::WHITE);
        });
        ui.label(RichText::new(format!("Version {}", env!("CARGO_PKG_VERSION"))).color(Color32::from_gray(130)));
        ui.add_space(18.0);
        ui.label(RichText::new("A music, tablature, and chord chart editor.").color(Color32::from_gray(210)));
        ui.add_space(4.0);
        ui.label(RichText::new("Free software: you may use, share and change it under the GNU General Public License, version 3 or later (see the License tab).").color(Color32::from_gray(170)));
        let rest = ui.available_height() - 30.0;
        if rest > 0.0 {
            ui.add_space(rest);
        }
        ui.label(RichText::new("\u{00A9} 2026 by CodeGator. All rights reserved.").color(Color32::from_gray(120)));
    });
}

fn license_tab(ui: &mut egui::Ui) {
    egui::ScrollArea::vertical().id_salt("about-license").auto_shrink([false, false]).show(ui, |ui| {
        egui::Frame::new().fill(Color32::from_rgb(28, 30, 36)).corner_radius(4.0).inner_margin(egui::Margin::same(12)).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(GPL).size(12.5).color(Color32::from_gray(205)));
        });
    });
}

fn open_source_tab(ui: &mut egui::Ui, st: &mut AboutState) {
    let data = st.data.get_or_insert_with(components);
    ui.label(
        RichText::new("GuitarCraft is built using these components:")
            .size(12.0)
            .color(Color32::from_gray(170)),
    );
    ui.add(
        egui::TextEdit::singleline(&mut st.filter)
            .hint_text("\u{1F50D} Find a component")
            .desired_width(f32::INFINITY),
    );
    let needle = st.filter.trim().to_lowercase();
    let shown: Vec<usize> = (0..data.components.len())
        .filter(|&i| {
            let c = &data.components[i];
            needle.is_empty()
                || c.name.to_lowercase().contains(&needle)
                || c.license.to_lowercase().contains(&needle)
        })
        .collect();
    if !shown.contains(&st.sel)
        && let Some(&first) = shown.first()
    {
        st.sel = first;
    }
    let h = ui.available_height();
    ui.horizontal_top(|ui| {
        // ---- the list
        ui.vertical(|ui| {
            ui.set_width(170.0);
            egui::Frame::new()
                .fill(Color32::from_rgb(28, 30, 36))
                .corner_radius(4.0)
                .inner_margin(egui::Margin::same(4))
                .show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("about-list")
                        .max_height(h - 10.0)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.set_width(160.0);
                            for &i in &shown {
                                let c = &data.components[i];
                                let label = format!("{}  {}", c.name, c.version);
                                let r = ui
                                    .add_sized(
                                        Vec2::new(160.0, 20.0),
                                        egui::Button::selectable(
                                            st.sel == i,
                                            RichText::new(label).size(12.5),
                                        ),
                                    )
                                    .on_hover_text(&c.name);
                                if r.clicked() {
                                    st.sel = i;
                                }
                            }
                            if shown.is_empty() {
                                ui.label(RichText::new("Nothing found.").weak());
                            }
                        });
                });
        });
        // ---- the chosen component
        ui.vertical(|ui| {
            let Some(c) = data
                .components
                .get(st.sel)
                .filter(|_| !shown.is_empty())
                .cloned()
            else {
                return;
            };
            ui.label(
                RichText::new(format!("{}  {}", c.name, c.version))
                    .size(18.0)
                    .family(FontFamily::Name("bold".into()))
                    .color(Color32::WHITE),
            );
            if !c.link.is_empty() {
                ui.hyperlink_to(
                    RichText::new(format!("{} \u{2197}", c.link)).size(12.5),
                    &c.link,
                );
            }
            if !c.description.is_empty() {
                ui.label(RichText::new(&c.description).color(Color32::from_gray(210)));
            }
            ui.label(
                RichText::new(format!(
                    "License: {}",
                    if c.license.is_empty() {
                        "see below"
                    } else {
                        &c.license
                    }
                ))
                .color(Color32::from_gray(170)),
            );
            ui.add_space(4.0);
            let text = data.texts.get(c.text).cloned().unwrap_or_default();
            egui::Frame::new()
                .fill(Color32::from_rgb(28, 30, 36))
                .corner_radius(4.0)
                .inner_margin(egui::Margin::same(8))
                .show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt(("about-text", st.sel))
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(text)
                                    .size(12.0)
                                    .color(Color32::from_gray(205)),
                            );
                        });
                });
        });
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn components_list_is_built_in() {
        let d = super::components();
        assert!(d.components.len() > 50, "{}", d.components.len());
        let egui = d
            .components
            .iter()
            .find(|c| c.name == "egui")
            .expect("egui is listed");
        assert!(egui.license.contains("MIT"));
        // Every component has some license text to show.
        for c in &d.components {
            assert!(
                d.texts
                    .get(c.text)
                    .map(|t| !t.trim().is_empty())
                    .unwrap_or(false),
                "{}",
                c.name
            );
        }
        assert!(super::GPL.contains("GNU GENERAL PUBLIC LICENSE"));
    }
}
