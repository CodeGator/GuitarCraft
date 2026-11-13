//! Window layout: menus, toolbar, title strip, dialogs and panel arrangement.

use super::widgets::*;
use super::{App, Pending};
use crate::model::Dur;
use crate::score::Notation;
use egui::{Align2, Color32, FontId, Pos2, Rect, Sense, Ui, Vec2};

pub const TOOLBAR_BG: Color32 = Color32::from_rgb(58, 63, 76);
pub const PANEL_BG: Color32 = Color32::from_rgb(42, 45, 53);
pub const STRIP_BG: Color32 = Color32::from_rgb(136, 148, 176);
pub const BLUE: Color32 = Color32::from_rgb(61, 123, 224);

impl App {
    fn toolbar(&mut self, ui: &mut Ui) {
        let full_w = ui.available_width();
        let left_x = ui.cursor().min.x;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 3.0;
            if icon_toggle(ui, self.show_home, "Start screen", icon_home).clicked() {
                self.show_home = !self.show_home;
            }
            ui.add_space(6.0);
            if icon_toggle(ui, self.show_palette, "Edition palette", icon_panel_left).clicked() {
                self.show_palette = !self.show_palette;
            }
            if icon_toggle(ui, self.show_tracks, "Tracks and mixer", icon_panel_bottom).clicked() {
                self.show_tracks = !self.show_tracks;
            }
            if icon_toggle(
                ui,
                self.show_inspector,
                "Song and track settings",
                icon_panel_right,
            )
            .clicked()
            {
                self.show_inspector = !self.show_inspector;
            }
            ui.add_space(6.0);
            let (r, _) = ui.allocate_exact_size(Vec2::new(18.0, 30.0), Sense::hover());
            icon_zoom(
                ui.painter(),
                Rect::from_center_size(r.center(), Vec2::splat(14.0)),
                Color32::from_gray(225),
            );
            egui::ComboBox::from_id_salt("zoom")
                .width(58.0)
                .selected_text(format!("{}%", (self.zoom * 100.0).round() as i32))
                .show_ui(ui, |ui| {
                    for z in [0.5f32, 0.75, 1.0, 1.25, 1.5, 2.0] {
                        ui.selectable_value(&mut self.zoom, z, format!("{}%", (z * 100.0) as i32));
                    }
                });
            ui.add_space(6.0);
            if icon_toggle(ui, false, "Undo (Ctrl+Z)", icon_undo).clicked() {
                self.do_undo();
            }
            if icon_toggle(ui, false, "Redo (Ctrl+Y)", icon_redo).clicked() {
                self.do_redo();
            }
            // Centre the transport and display.
            let block_w = 5.0 * 33.0 + 392.0 + 140.0;
            let target = left_x + full_w * 0.5 - block_w * 0.5;
            let here = ui.cursor().min.x;
            if target > here {
                ui.add_space(target - here);
            }
            if icon_toggle(ui, false, "Go to the first bar", icon_first).clicked() {
                self.stop();
                self.goto_bar(0);
            }
            if icon_toggle(ui, false, "Previous bar", icon_prev).clicked() {
                self.goto_bar(self.bar.saturating_sub(1));
            }
            if play_button(ui, self.playing)
                .on_hover_text("Play / pause (Space)")
                .clicked()
            {
                self.toggle_play();
            }
            if icon_toggle(ui, false, "Next bar", icon_next).clicked() {
                self.goto_bar(self.bar + 1);
            }
            if icon_toggle(ui, false, "Go to the last bar", icon_last).clicked() {
                self.stop();
                self.goto_bar(self.song.bar_count() - 1);
            }
            ui.add_space(4.0);
            self.lcd(ui);
            ui.add_space(6.0);
            if icon_toggle(
                ui,
                self.looping,
                "Loop the selected bars (or the whole song)",
                icon_loop,
            )
            .clicked()
            {
                self.looping = !self.looping;
            }
            if icon_toggle(ui, self.metronome, "Metronome", icon_metronome).clicked() {
                self.metronome = !self.metronome;
            }
            if icon_toggle(
                ui,
                self.count_in,
                "Count-in: one bar of clicks before playing",
                icon_countin,
            )
            .clicked()
            {
                self.count_in = !self.count_in;
            }
            let sp = ui
                .add(
                    egui::Button::selectable(
                        (self.speed - 100.0).abs() > 0.5 || self.trainer_on,
                        format!("{}%", self.speed.round() as i32),
                    )
                    .frame_when_inactive(true)
                    .min_size(Vec2::new(52.0, 28.0)),
                )
                .on_hover_text("Playback speed");
            egui::Popup::menu(&sp).show(|ui| {
                ui.label(egui::RichText::new("Relative speed").strong());
                for v in [25.0f32, 50.0, 60.0, 70.0, 80.0, 90.0, 100.0] {
                    if ui
                        .selectable_label((self.speed - v).abs() < 0.5, format!("{}%", v as i32))
                        .clicked()
                    {
                        self.speed = v;
                        ui.close();
                    }
                }
                ui.add(egui::Slider::new(&mut self.speed, 10.0..=300.0).suffix("%"));
                ui.separator();
                if ui
                    .checkbox(&mut self.trainer_on, "Speed trainer (faster on each loop)")
                    .changed()
                    && self.trainer_on
                {
                    self.looping = true;
                }
                if ui.button("Speed trainer settings...").clicked() {
                    self.trainer_open = true;
                    ui.close();
                }
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(4.0);
                if icon_toggle(
                    ui,
                    self.show_keyboard,
                    "Keyboard view (Ctrl+F6)",
                    icon_keyboard,
                )
                .clicked()
                {
                    self.show_keyboard = !self.show_keyboard;
                }
                if icon_toggle(ui, self.show_fretboard, "Fretboard view", icon_fretboard).clicked()
                {
                    self.show_fretboard = !self.show_fretboard;
                }
                egui::ComboBox::from_id_salt("notation")
                    .width(96.0)
                    .selected_text(match self.notation {
                        Notation::Both => "Score + Tab",
                        Notation::TabOnly => "Tab only",
                        Notation::StandardOnly => "Score only",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.notation, Notation::Both, "Score + Tab");
                        ui.selectable_value(&mut self.notation, Notation::TabOnly, "Tab only");
                        ui.selectable_value(
                            &mut self.notation,
                            Notation::StandardOnly,
                            "Score only",
                        );
                    });
            });
        });
    }

    fn lcd(&mut self, ui: &mut Ui) {
        let (all, _) = ui.allocate_exact_size(Vec2::new(392.0, 40.0), Sense::hover());
        let p = ui.painter();
        p.rect_filled(all, 4.0, Color32::from_rgb(26, 28, 33));
        let r = Rect::from_min_size(all.min, Vec2::new(330.0, 40.0));
        let txt = Color32::from_gray(235);
        let dim = Color32::from_gray(150);
        let (bar, _) = self.play_beat.unwrap_or((self.bar, self.beat));
        let mb = &self.song.master[bar];
        let tr = &self.song.tracks[self.track];
        super::trackicons::draw(
            p,
            Rect::from_min_size(r.min + Vec2::new(7.0, 4.0), Vec2::new(13.0, 16.0)),
            Color32::from_gray(230),
            super::trackicons::icon_of(tr),
        );
        p.text(
            r.min + Vec2::new(26.0, 3.0),
            Align2::LEFT_TOP,
            format!("{}. {}", self.track + 1, tr.name),
            FontId::new(13.5, egui::FontFamily::Name("bold".into())),
            txt,
        );
        p.text(
            r.right_top() + Vec2::new(-8.0, 5.0),
            Align2::RIGHT_TOP,
            if self.playing { "PLAY" } else { "STOP" },
            FontId::monospace(10.0),
            dim,
        );
        let y2 = r.top() + 23.0;
        p.text(
            Pos2::new(r.left() + 8.0, y2),
            Align2::LEFT_TOP,
            format!("{}/{}", bar + 1, self.song.bar_count()),
            FontId::proportional(11.5),
            txt,
        );
        // Bar length: filled beats against the time signature, with a light that is
        // green when the bar is complete and orange otherwise (and flashes on each beat).
        let cur_bar = &self.song.tracks[self.track].bars[bar];
        let unit = (crate::model::QUARTER * 4 / mb.den.max(1) as u32) as f32;
        let vb = cur_bar.voice(self.voice);
        let fill = if vb.len() == 1 && vb[0].is_rest() && vb[0].dur == Dur::Whole {
            0
        } else {
            cur_bar.voice_ticks(self.voice)
        };
        let complete = fill == mb.ticks();
        let errors = cur_bar.length_errors(mb.ticks());
        let lit = self.playing && self.metro_flash;
        let light = if lit {
            Color32::from_rgb(255, 230, 120)
        } else if !errors.is_empty() {
            Color32::from_rgb(225, 55, 55)
        } else if complete {
            Color32::from_rgb(80, 200, 110)
        } else {
            Color32::from_rgb(240, 150, 40)
        };
        let dot = Pos2::new(r.left() + 62.0, y2 + 7.0);
        if !errors.is_empty() && !lit {
            p.circle_filled(dot, 5.0, light);
            p.text(
                dot,
                Align2::CENTER_CENTER,
                "!",
                FontId::new(9.0, egui::FontFamily::Name("bold".into())),
                Color32::WHITE,
            );
        } else {
            p.circle_filled(dot, if lit { 4.5 } else { 3.8 }, light);
        }
        let fill_col = if fill > mb.ticks() {
            Color32::from_rgb(255, 110, 100)
        } else {
            txt
        };
        let dur_rect = p.text(
            Pos2::new(r.left() + 70.0, y2),
            Align2::LEFT_TOP,
            format!("{:.1}:{:.1}", fill as f32 / unit, mb.num as f32),
            FontId::proportional(11.5),
            fill_col,
        );
        // Hovering explains the bar's length, and what to add or remove when it is wrong.
        let hover = Rect::from_min_max(
            Pos2::new(dot.x - 6.0, dur_rect.top() - 2.0),
            dur_rect.right_bottom() + Vec2::new(2.0, 2.0),
        );
        let mut tip = String::from("Bar current duration");
        if errors.is_empty() {
            if !complete && fill > 0 {
                tip.push_str(&format!(
                    "\nStill to fill: {}",
                    crate::model::beats_words(mb.ticks().saturating_sub(fill), mb.den)
                ));
            }
        } else {
            tip.push_str("\nErrors found in bar completion:");
            for (v, d) in &errors {
                tip.push_str(&format!(
                    "\n{}",
                    crate::model::length_advice(*v, *d, mb.den)
                ));
            }
        }
        ui.interact(hover, ui.id().with("lcd_duration"), Sense::hover())
            .on_hover_text(tip);
        p.text(
            Pos2::new(r.left() + 128.0, y2),
            Align2::LEFT_TOP,
            format!(
                "{} / {}",
                fmt_time(self.play_secs),
                fmt_time(self.total_secs)
            ),
            FontId::proportional(11.5),
            txt,
        );
        let tempo = (self.song.tempo_at(bar) as f32 * self.speed / 100.0).round() as u32;
        let tr_rect = p.text(
            Pos2::new(r.left() + 228.0, y2),
            Align2::LEFT_TOP,
            format!("\u{2669} = {tempo}"),
            FontId::proportional(11.5),
            txt,
        );
        let tempo_resp = ui
            .interact(
                tr_rect.expand(4.0),
                ui.id().with("lcd_tempo"),
                Sense::click(),
            )
            .on_hover_text("Edit the tempo (F10)");
        if tempo_resp.hovered() {
            ui.painter().rect_stroke(
                tr_rect.expand(3.0),
                3.0,
                egui::Stroke::new(1.0, Color32::from_rgb(90, 150, 240)),
                egui::StrokeKind::Outside,
            );
        }
        if tempo_resp.clicked() {
            self.open_automations(bar);
        }
        p.text(
            Pos2::new(r.right() - 8.0, y2),
            Align2::RIGHT_TOP,
            format!("{}%", self.speed.round() as i32),
            FontId::proportional(11.5),
            dim,
        );
        // The name of the note under the cursor.
        let nr = Rect::from_min_max(Pos2::new(r.right(), all.top()), all.max);
        p.line_segment(
            [
                Pos2::new(nr.left(), nr.top() + 6.0),
                Pos2::new(nr.left(), nr.bottom() - 6.0),
            ],
            egui::Stroke::new(1.0, Color32::from_gray(60)),
        );
        if !self.playing
            && let Some(name) = self.cursor_note_name()
        {
            p.text(
                nr.center(),
                Align2::CENTER_CENTER,
                name,
                FontId::proportional(19.0),
                Color32::from_gray(170),
            );
        }
    }

    fn title_strip(&mut self, ui: &mut Ui) {
        let r = ui.max_rect();
        let mut title = self.song.title.clone();
        if !self.song.artist.is_empty() {
            title = format!("{} - {}", self.song.artist, title);
        }
        if self.dirty {
            title.push_str(" *");
        }
        ui.painter().text(
            r.center(),
            Align2::CENTER_CENTER,
            title,
            FontId::new(12.5, egui::FontFamily::Name("bold".into())),
            Color32::WHITE,
        );
        let pr = Rect::from_min_max(Pos2::new(r.right() - 26.0, r.top()), r.right_bottom());
        let resp = ui
            .interact(pr, ui.id().with("strip_plus"), Sense::click())
            .on_hover_text("New song");
        if resp.hovered() {
            ui.painter()
                .rect_filled(pr, 0.0, Color32::from_white_alpha(30));
        }
        ui.painter().text(
            pr.center(),
            Align2::CENTER_CENTER,
            "+",
            FontId::proportional(18.0),
            Color32::WHITE,
        );
        if resp.clicked() {
            self.request(Pending::New);
        }
    }

    fn dialogs(&mut self, ctx: &egui::Context) {
        if let Some(p) = self.pending {
            let mut choice = None;
            egui::Window::new("Unsaved changes")
                .collapsible(false)
                .resizable(false)
                .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.label("This song has changes that haven't been saved.");
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("Save first").clicked() {
                            choice = Some(0);
                        }
                        if ui.button("Don't save").clicked() {
                            choice = Some(1);
                        }
                        if ui.button("Cancel").clicked() {
                            choice = Some(2);
                        }
                    });
                });
            match choice {
                Some(0) => {
                    self.pending = None;
                    self.save(false);
                    if !self.dirty {
                        self.run_pending(p);
                        if p == Pending::Quit {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    }
                }
                Some(1) => {
                    self.pending = None;
                    self.dirty = false;
                    self.run_pending(p);
                    if p == Pending::Quit {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                }
                Some(_) => self.pending = None,
                None => {}
            }
        }
        if self.show_help {
            egui::Window::new("Keyboard shortcuts")
                .open(&mut self.show_help)
                .resizable(false)
                .show(ctx, |ui| {
                    super::actions::shortcut_rows(ui);
                });
        }
        if self.show_about {
            egui::Window::new("About").open(&mut self.show_about).resizable(false).show(ctx, |ui| {
                ui.heading("GuitarCraft");
                ui.label("A guitar and bass score and tablature editor written in Rust.");
                ui.label("Write tabs and standard notation, hear them with the built-in sound, and save, open or export your songs.");
                ui.label("Opens its own songs and Guitar Pro files (.gp3, .gp4, .gp5, .gpx, .gp).");
            });
        }
    }

    fn update_title(&mut self, ctx: &egui::Context) {
        let t = format!(
            "{}{} - GuitarCraft",
            if self.dirty { "* " } else { "" },
            if self.song.title.is_empty() {
                "Untitled"
            } else {
                &self.song.title
            }
        );
        if t != self.title_cache {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(t.clone()));
            self.title_cache = t;
        }
    }

    fn toast_ui(&mut self, ctx: &egui::Context, area: Rect) {
        let now = ctx.input(|i| i.time);
        if self.status != self.toast.0 {
            self.toast = (self.status.clone(), now);
        }
        let age = now - self.toast.1;
        if self.toast.0.is_empty() || age > 5.0 {
            return;
        }
        let alpha = if age > 4.0 { (5.0 - age) as f32 } else { 1.0 };
        let p = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("toast"),
        ));
        let gal = p.layout_no_wrap(
            self.toast.0.clone(),
            FontId::proportional(13.0),
            Color32::WHITE.gamma_multiply(alpha),
        );
        let r = Rect::from_min_size(
            Pos2::new(area.left() + 14.0, area.bottom() - gal.size().y - 26.0),
            gal.size() + Vec2::new(20.0, 12.0),
        );
        p.rect_filled(
            r,
            6.0,
            Color32::from_rgba_unmultiplied(30, 32, 38, (220.0 * alpha) as u8),
        );
        p.galley(r.min + Vec2::new(10.0, 6.0), gal, Color32::WHITE);
        ctx.request_repaint();
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.handle_screenshot(&ctx);
        // Only text fields keep the keyboard: a clicked button (like Loop) mustn't
        // be pressed again by Space, which plays and pauses.
        if !ctx.egui_wants_keyboard_input()
            && let Some(id) = ctx.memory(|m| m.focused())
        {
            ctx.memory_mut(|m| m.surrender_focus(id));
        }
        self.fix_cursor_mode();
        if ctx.input(|i| i.viewport().close_requested()) && self.dirty && self.screenshot.is_none()
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.pending = Some(Pending::Quit);
        }
        if !self.show_home {
            self.handle_keys(&ctx);
        } else {
            self.handle_global_keys(&ctx);
        }
        let sounding = self.poll_playback();
        if self.playing || !sounding.is_empty() {
            ctx.request_repaint();
        }
        self.update_title(&ctx);
        if self.fullscreen_changed {
            self.fullscreen_changed = false;
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.fullscreen));
        }

        egui::Panel::top("menu")
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(32, 34, 40))
                    .inner_margin(egui::Margin::symmetric(6, 2)),
            )
            .show(ui, |ui| self.menu_bar(ui));
        if self.show_home {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(ui, |ui| self.home_ui(ui));
            self.dialogs(&ctx);
            return;
        }
        egui::Panel::top("toolbar")
            .frame(
                egui::Frame::new()
                    .fill(TOOLBAR_BG)
                    .inner_margin(egui::Margin::symmetric(8, 5)),
            )
            .show(ui, |ui| self.toolbar(ui));
        egui::Panel::top("strip")
            .exact_size(22.0)
            .frame(egui::Frame::new().fill(STRIP_BG))
            .show(ui, |ui| self.title_strip(ui));
        if self.show_keyboard {
            egui::Panel::bottom("keyboard")
                .exact_size(112.0)
                .frame(egui::Frame::new().fill(Color32::from_rgb(28, 30, 34)))
                .show(ui, |ui| self.keyboard_panel(ui, &sounding));
        }
        if self.show_fretboard {
            egui::Panel::bottom("fretboard")
                .exact_size(150.0)
                .frame(
                    egui::Frame::new()
                        .fill(Color32::from_rgb(28, 30, 34))
                        .inner_margin(egui::Margin::same(6)),
                )
                .show(ui, |ui| self.fretboard_panel(ui, &sounding));
        }
        if self.show_tracks {
            let h = (self.song.tracks.len() as f32 * 25.0 + 25.0 + 25.0 + 22.0 + 8.0)
                .clamp(110.0, 280.0);
            egui::Panel::bottom("tracks")
                .exact_size(h)
                .frame(egui::Frame::new().fill(Color32::from_rgb(34, 36, 42)))
                .show(ui, |ui| self.tracks_panel(ui));
        }
        if self.show_palette {
            egui::Panel::left("palette")
                .exact_size(232.0)
                .frame(
                    egui::Frame::new()
                        .fill(PANEL_BG)
                        .inner_margin(egui::Margin::same(8)),
                )
                .show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("pal")
                        .show(ui, |ui| self.palette(ui));
                });
        }
        if self.show_inspector {
            egui::Panel::right("inspector")
                .exact_size(262.0)
                .frame(
                    egui::Frame::new()
                        .fill(PANEL_BG)
                        .inner_margin(egui::Margin::symmetric(0, 0)),
                )
                .show(ui, |ui| self.inspector_ui(ui));
        }
        let central = egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(Color32::from_rgb(232, 233, 237)))
            .show(ui, |ui| self.score(ui));
        self.toast_ui(&ctx, central.response.rect);
        self.dialogs(&ctx);
        self.small_dialogs(&ctx);
        self.tool_windows(&ctx);
        self.popup_window(&ctx);
        self.voices_window(&ctx);
        self.automation_window(&ctx);
        if let Some(id) = self.focus_request.take() {
            ctx.memory_mut(|m| m.request_focus(egui::Id::new(id)));
        }
    }
}
