//! The track list with mixer and bar overview, the fretboard, and the score view.

use super::panels::BLUE;
use super::widgets::*;
use super::{App, InspectorTab};
use crate::fretboard::{self, FretAction};
use crate::score;
use egui::{
    Align2, Color32, FontFamily, FontId, Pos2, Rect, RichText, Sense, Stroke as PStroke, Ui,
    UiBuilder, Vec2,
};

const ROW_H: f32 = 25.0;
const HEAD_H: f32 = 24.0;
const LEFT_W: f32 = 430.0;
const CELL: f32 = 22.0;

impl App {
    pub(super) fn tracks_panel(&mut self, ui: &mut Ui) {
        let full = ui.max_rect();
        let n = self.song.tracks.len();
        let bars = self.song.bar_count();
        let p = ui.painter().clone();
        let grey = Color32::from_gray(170);
        // Header.
        let head = Rect::from_min_size(full.min, Vec2::new(full.width(), HEAD_H));
        p.rect_filled(head, 0.0, Color32::from_rgb(46, 49, 57));
        let plus = Rect::from_min_size(head.min + Vec2::new(4.0, 2.0), Vec2::new(20.0, 20.0));
        let pr = ui
            .interact(plus, ui.id().with("addtrack"), Sense::click())
            .on_hover_text("Add a guitar track");
        p.text(
            plus.center(),
            Align2::CENTER_CENTER,
            "+",
            FontId::proportional(18.0),
            Color32::WHITE,
        );
        if pr.clicked() {
            self.add_track(false);
        }
        p.text(
            head.min + Vec2::new(30.0, 12.0),
            Align2::LEFT_CENTER,
            "Tracks",
            FontId::new(12.5, FontFamily::Name("bold".into())),
            Color32::from_gray(225),
        );
        // Move the selected track up or down.
        for (k, lab, tip) in [
            (-1i32, "\u{25B2}", "Move the track up"),
            (1, "\u{25BC}", "Move the track down"),
        ] {
            let r = Rect::from_center_size(
                head.min + Vec2::new(if k < 0 { 96.0 } else { 116.0 }, 12.0),
                Vec2::splat(18.0),
            );
            let resp = ui
                .interact(r, ui.id().with(("mvtr", k)), Sense::click())
                .on_hover_text(tip);
            p.text(
                r.center(),
                Align2::CENTER_CENTER,
                lab,
                FontId::proportional(10.0),
                if resp.hovered() {
                    Color32::WHITE
                } else {
                    Color32::from_gray(160)
                },
            );
            if resp.clicked() {
                self.move_track(k);
            }
        }
        // Multitrack button above the eye column: all tracks, or only the selected one.
        let mt = Rect::from_center_size(head.min + Vec2::new(196.0, 12.0), Vec2::new(22.0, 20.0));
        let mt_resp = ui
            .interact(mt, ui.id().with("multitrack"), Sense::click())
            .on_hover_text(if self.single_track {
                "Multitrack is off: only the selected track is shown (F3)"
            } else {
                "Multitrack is on: all tracks with an open eye are shown (F3)"
            });
        let multi = !self.single_track;
        if multi {
            p.rect_filled(mt, 3.0, BLUE);
        } else if mt_resp.hovered() {
            p.rect_filled(mt, 3.0, Color32::from_gray(80));
        }
        multitrack_icon(
            &p,
            mt.shrink2(Vec2::new(4.0, 3.0)),
            if multi { Color32::WHITE } else { grey },
        );
        if mt_resp.clicked() {
            self.single_track = !self.single_track;
            self.status = if self.single_track {
                "Single track: only the selected track is shown.".into()
            } else {
                "Multitrack: every track with an open eye is shown.".into()
            };
        }
        speaker_icon(
            &p,
            Rect::from_center_size(head.min + Vec2::new(220.0, 12.0), Vec2::splat(14.0)),
            grey,
        );
        headphones_icon(
            &p,
            Rect::from_center_size(head.min + Vec2::new(244.0, 12.0), Vec2::splat(14.0)),
            grey,
        );
        p.text(
            head.min + Vec2::new(318.0, 12.0),
            Align2::CENTER_CENTER,
            "Vol.",
            FontId::proportional(11.5),
            grey,
        );
        p.text(
            head.min + Vec2::new(405.0, 12.0),
            Align2::CENTER_CENTER,
            "Pan.",
            FontId::proportional(11.5),
            grey,
        );
        let mut select: Option<(usize, Option<usize>)> = None;
        let mut track_action: Option<(usize, u8)> = None;
        let mut section_sel: Option<(usize, usize)> = None;
        let mut changed = false;
        let mut master = self.master_volume;
        // Left columns.
        for ti in 0..=n {
            let y = full.top() + HEAD_H + ti as f32 * ROW_H;
            let row = Rect::from_min_size(Pos2::new(full.left(), y), Vec2::new(LEFT_W, ROW_H));
            let is_master = ti == n;
            if !is_master && ti == self.track {
                p.rect_filled(row, 0.0, Color32::from_rgb(56, 60, 70));
            }
            p.line_segment(
                [row.left_bottom(), Pos2::new(full.right(), row.bottom())],
                PStroke::new(1.0, Color32::from_rgb(28, 30, 35)),
            );
            let (vol, pan_opt) = if is_master {
                p.text(
                    row.min + Vec2::new(46.0, ROW_H * 0.5),
                    Align2::LEFT_CENTER,
                    "Master",
                    FontId::proportional(13.0),
                    Color32::from_gray(210),
                );
                (&mut master, None)
            } else {
                let t = &mut self.song.tracks[ti];
                // As in Guitar Pro: the track's colour as a thin chip at the
                // row's edge, and a light picture of the instrument.
                let col = Color32::from_rgb(t.color[0], t.color[1], t.color[2]);
                p.rect_filled(
                    Rect::from_min_size(row.min + Vec2::new(0.0, 1.0), Vec2::new(4.0, ROW_H - 2.0)),
                    0.0,
                    col,
                );
                super::trackicons::draw(
                    &p,
                    Rect::from_center_size(
                        row.min + Vec2::new(17.0, ROW_H * 0.5),
                        Vec2::new(14.0, 17.0),
                    ),
                    Color32::from_gray(222),
                    super::trackicons::icon_of(t),
                );
                let name_r =
                    Rect::from_min_size(row.min + Vec2::new(28.0, 0.0), Vec2::new(150.0, ROW_H));
                let nr = ui.interact(name_r, ui.id().with(("trk", ti)), Sense::click());
                let label = format!("{}. {}", ti + 1, t.name);
                let tc = if ti == self.track {
                    Color32::WHITE
                } else {
                    Color32::from_gray(205)
                };
                let gal = p.layout_no_wrap(label, FontId::proportional(13.0), tc);
                p.with_clip_rect(name_r).galley(
                    Pos2::new(name_r.left() + 6.0, name_r.center().y - gal.size().y * 0.5),
                    gal,
                    tc,
                );
                if nr.clicked() {
                    select = Some((ti, None));
                }
                nr.context_menu(|ui| {
                    for (code, lab) in [
                        (0u8, "Add a track..."),
                        (1, "Duplicate"),
                        (2, "Move up"),
                        (3, "Move down"),
                        (4, "Show / hide in the score"),
                        (5, "Remove"),
                    ] {
                        if ui.button(lab).clicked() {
                            track_action = Some((ti, code));
                            ui.close();
                        }
                    }
                });
                // Eye, mute, solo.
                // The eyes choose the tracks shown in multitrack mode; in single-track mode they're greyed.
                let er = Rect::from_center_size(
                    row.min + Vec2::new(196.0, ROW_H * 0.5),
                    Vec2::splat(20.0),
                );
                if multi {
                    if ui
                        .interact(er, ui.id().with(("eye", ti)), Sense::click())
                        .on_hover_text("Show this track in the score")
                        .clicked()
                    {
                        t.visible = !t.visible;
                        changed = true;
                    }
                } else {
                    ui.interact(er, ui.id().with(("eye", ti)), Sense::hover())
                        .on_hover_text(
                            "Turn on multitrack (the button above) to choose which tracks to show",
                        );
                }
                let eye_col = if !multi {
                    Color32::from_gray(70)
                } else if t.visible {
                    Color32::from_gray(225)
                } else {
                    Color32::from_gray(90)
                };
                eye_icon(&p, er.shrink(3.0), eye_col, t.visible);
                let mr = Rect::from_center_size(
                    row.min + Vec2::new(220.0, ROW_H * 0.5),
                    Vec2::splat(20.0),
                );
                if ui
                    .interact(mr, ui.id().with(("mute", ti)), Sense::click())
                    .on_hover_text("Mute")
                    .clicked()
                {
                    t.mute = !t.mute;
                    changed = true;
                }
                if t.mute {
                    p.rect_filled(mr, 3.0, Color32::from_rgb(200, 70, 70));
                }
                speaker_icon(
                    &p,
                    mr.shrink(3.0),
                    if t.mute {
                        Color32::WHITE
                    } else {
                        Color32::from_gray(130)
                    },
                );
                let sr = Rect::from_center_size(
                    row.min + Vec2::new(244.0, ROW_H * 0.5),
                    Vec2::splat(20.0),
                );
                if ui
                    .interact(sr, ui.id().with(("solo", ti)), Sense::click())
                    .on_hover_text("Solo")
                    .clicked()
                {
                    t.solo = !t.solo;
                    changed = true;
                }
                if t.solo {
                    p.rect_filled(sr, 3.0, Color32::from_rgb(220, 180, 50));
                }
                headphones_icon(
                    &p,
                    sr.shrink(3.0),
                    if t.solo {
                        Color32::BLACK
                    } else {
                        Color32::from_gray(130)
                    },
                );
                (&mut t.volume, Some(&mut t.pan))
            };
            // Volume bar.
            let vr = Rect::from_min_size(
                row.min + Vec2::new(262.0, ROW_H * 0.5 - 4.0),
                Vec2::new(112.0, 8.0),
            );
            let vresp = ui
                .interact(
                    vr.expand2(Vec2::new(4.0, 6.0)),
                    ui.id().with(("vol", ti)),
                    Sense::click_and_drag(),
                )
                .on_hover_text(format!("Volume {}%", (*vol * 100.0).round()));
            if (vresp.dragged() || vresp.clicked())
                && let Some(pp) = vresp.interact_pointer_pos()
            {
                *vol = ((pp.x - vr.left()) / vr.width())
                    .clamp(0.0, if is_master { 1.5 } else { 1.0 })
                    * if is_master { 1.5 } else { 1.0 };
                if is_master {
                    *vol = ((pp.x - vr.left()) / vr.width()).clamp(0.0, 1.0) * 1.5;
                }
                changed = true;
            }
            let frac = if is_master { *vol / 1.5 } else { *vol };
            p.rect_filled(vr, 2.0, Color32::from_rgb(24, 26, 30));
            p.rect_filled(
                Rect::from_min_size(
                    vr.min,
                    Vec2::new(vr.width() * frac.clamp(0.0, 1.0), vr.height()),
                ),
                2.0,
                Color32::from_gray(150),
            );
            let hx = vr.left() + vr.width() * frac.clamp(0.0, 1.0);
            p.rect_filled(
                Rect::from_center_size(Pos2::new(hx, vr.center().y), Vec2::new(5.0, 16.0)),
                1.0,
                Color32::from_gray(225),
            );
            // Pan knob.
            let kc = row.min + Vec2::new(405.0, ROW_H * 0.5);
            let kr = Rect::from_center_size(kc, Vec2::splat(18.0));
            let value = pan_opt.as_ref().map(|v| **v).unwrap_or(0.0);
            if let Some(pan) = pan_opt {
                let kresp = ui
                    .interact(kr, ui.id().with(("pan", ti)), Sense::click_and_drag())
                    .on_hover_text("Pan: drag left or right, double-click to centre");
                if kresp.dragged() {
                    *pan = (*pan + kresp.drag_delta().x / 60.0 - kresp.drag_delta().y / 60.0)
                        .clamp(-1.0, 1.0);
                    changed = true;
                }
                if kresp.double_clicked() {
                    *pan = 0.0;
                    changed = true;
                }
            }
            knob(&p, kc, 8.0, value);
        }
        if (master - self.master_volume).abs() > 1e-6 {
            self.master_volume = master;
            if let Some(a) = &self.audio
                && let Ok(mut st) = a.state.lock()
            {
                st.master = master;
            }
        }
        // Bar grid with section names underneath (scrolls sideways).
        let grid_r = Rect::from_min_max(Pos2::new(full.left() + LEFT_W, full.top()), full.max);
        let mut child = ui.new_child(UiBuilder::new().max_rect(grid_r));
        egui::ScrollArea::horizontal()
            .id_salt("bargrid")
            .auto_shrink([false, false])
            .show(&mut child, |ui| {
                let total = Vec2::new(bars as f32 * CELL + 30.0, grid_r.height());
                let (r, resp) = ui.allocate_exact_size(total, Sense::click());
                let p = ui.painter();
                let head = Rect::from_min_size(r.min, Vec2::new(r.width(), HEAD_H));
                p.rect_filled(head, 0.0, Color32::from_rgb(46, 49, 57));
                for b in 0..bars {
                    let x = r.left() + b as f32 * CELL;
                    if b == 0 || (b + 1) % 4 == 0 {
                        p.text(
                            Pos2::new(x + 3.0, r.top() + 12.0),
                            Align2::LEFT_CENTER,
                            format!("{}", b + 1),
                            FontId::proportional(11.5),
                            Color32::from_gray(200),
                        );
                    }
                    if self.play_beat.map(|x| x.0) == Some(b) || (!self.playing && b == self.bar) {
                        p.add(egui::Shape::convex_polygon(
                            vec![
                                Pos2::new(x + 6.0, r.top() + HEAD_H - 6.0),
                                Pos2::new(x + 16.0, r.top() + HEAD_H - 6.0),
                                Pos2::new(x + 11.0, r.top() + HEAD_H),
                            ],
                            BLUE,
                            PStroke::NONE,
                        ));
                    }
                }
                for ti in 0..n {
                    let t = &self.song.tracks[ti];
                    let col = Color32::from_rgb(t.color[0], t.color[1], t.color[2]);
                    let y = r.top() + HEAD_H + ti as f32 * ROW_H;
                    for b in 0..bars {
                        let cr = Rect::from_min_size(
                            Pos2::new(r.left() + b as f32 * CELL + 1.0, y + 1.0),
                            Vec2::new(CELL - 2.0, ROW_H - 2.0),
                        );
                        let filled = !t.bars[b].is_empty();
                        p.rect_filled(
                            cr,
                            0.0,
                            if filled {
                                col
                            } else {
                                Color32::from_rgb(52, 55, 63)
                            },
                        );
                        if ti == self.track && b == self.bar {
                            p.rect_stroke(
                                cr,
                                0.0,
                                PStroke::new(2.0, Color32::from_rgb(20, 20, 24)),
                                egui::StrokeKind::Inside,
                            );
                        }
                        if self.play_beat.map(|x| x.0) == Some(b) {
                            p.rect_filled(cr, 0.0, Color32::from_white_alpha(70));
                        }
                    }
                }
                // Section names under the grid.
                let my = r.top() + HEAD_H + (n + 1) as f32 * ROW_H;
                p.rect_filled(
                    Rect::from_min_max(Pos2::new(r.left(), my), Pos2::new(r.right(), my + 22.0)),
                    0.0,
                    Color32::from_rgb(28, 30, 35),
                );
                for (b, m) in self.song.master.iter().enumerate() {
                    if let Some(name) = &m.marker {
                        let x = r.left() + b as f32 * CELL + 6.0;
                        flag_icon(
                            p,
                            Rect::from_min_size(Pos2::new(x, my + 5.0), Vec2::new(9.0, 12.0)),
                            Color32::from_gray(220),
                        );
                        p.text(
                            Pos2::new(x + 14.0, my + 11.0),
                            Align2::LEFT_CENTER,
                            name,
                            FontId::proportional(12.0),
                            Color32::from_gray(220),
                        );
                    }
                }
                // Section names: click jumps there, double-click selects the section.
                if (resp.double_clicked() || resp.clicked())
                    && let Some(pp) = resp.interact_pointer_pos()
                    && pp.y >= my
                    && pp.y <= my + 22.0
                {
                    let b = (((pp.x - r.left()) / CELL) as usize).min(bars.saturating_sub(1));
                    let marks: Vec<usize> = self
                        .song
                        .master
                        .iter()
                        .enumerate()
                        .filter(|(_, m)| m.marker.is_some())
                        .map(|(i, _)| i)
                        .collect();
                    if let Some(&start) = marks.iter().rev().find(|&&m| m <= b) {
                        let end = marks
                            .iter()
                            .find(|&&m| m > start)
                            .map(|&m| m - 1)
                            .unwrap_or(bars - 1);
                        if resp.double_clicked() {
                            section_sel = Some((start, end));
                        } else {
                            select = Some((self.track, Some(start)));
                        }
                    }
                }
                if resp.clicked()
                    && let Some(pp) = resp.interact_pointer_pos()
                {
                    let b = ((pp.x - r.left()) / CELL) as usize;
                    let row = ((pp.y - r.top() - HEAD_H) / ROW_H).floor();
                    if pp.y >= my {
                        return;
                    }
                    if row >= 0.0 && (row as usize) < n && b < bars {
                        select = Some((row as usize, Some(b)));
                    } else if b < bars {
                        select = Some((self.track, Some(b)));
                    }
                }
            });
        if changed {
            self.dirty = true;
        }
        if let Some((ti, code)) = track_action {
            self.track = ti;
            self.clamp_cursor();
            match code {
                0 => self.addtrack_open = true,
                1 => self.duplicate_track(),
                2 => self.move_track(-1),
                3 => self.move_track(1),
                4 => {
                    self.song.tracks[ti].visible = !self.song.tracks[ti].visible;
                    self.dirty = true;
                }
                _ => self.remove_track(),
            }
        }
        if let Some((a, b)) = section_sel {
            self.sel_anchor = Some((a, 0));
            self.bar = b;
            self.beat = self.song.tracks[self.track].bars[b].voice(self.voice).len() - 1;
            self.status = format!("Selected bars {}\u{2013}{}.", a + 1, b + 1);
        }
        if let Some((ti, b)) = select {
            self.track = ti;
            if let Some(b) = b {
                self.goto_bar(b);
            }
            self.clamp_cursor();
            if self.inspector == InspectorTab::Song {
                self.inspector = InspectorTab::Track;
            }
        }
    }

    pub(super) fn fretboard_panel(&mut self, ui: &mut Ui, sounding: &[(usize, u8, u8)]) {
        let tr = self.song.tracks[self.track].clone();
        if !tr.has_strings() {
            ui.add_space(30.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new(format!(
                        "{} has no strings. Use the Keyboard view (Ctrl+F6) to enter notes.",
                        tr.name
                    ))
                    .color(Color32::from_gray(170)),
                );
            });
            return;
        }
        let mut marks: Vec<(u8, u8, Color32)> = vec![];
        if self.scale_shown {
            let iv = super::tools::SCALES[self.scale_kind].1;
            for s in 0..tr.strings() {
                for f in 0..=tr.frets.min(24) {
                    let pc = ((tr.tuning[s] as u32 + f as u32 + tr.capo_at(s) as u32) % 12) as u8;
                    let rel = (pc + 12 - self.scale_root) % 12;
                    if iv.contains(&rel) {
                        let col = if rel == 0 {
                            Color32::from_rgb(200, 120, 60)
                        } else {
                            Color32::from_rgb(80, 96, 120)
                        };
                        marks.push((s as u8, f, col));
                    }
                }
            }
        }
        if self.playing {
            let col = Color32::from_rgb(240, 130, 20);
            for &(t, s, f) in sounding {
                if t == self.track {
                    marks.push((s, f, col));
                }
            }
        } else {
            for n in &self.cur_beat().notes {
                let col = if n.string as usize == self.string {
                    BLUE
                } else {
                    Color32::from_rgb(tr.color[0], tr.color[1], tr.color[2])
                };
                marks.push((n.string, n.fret, col));
            }
        }
        if let FretAction::Toggle(s, f) = fretboard::fretboard(ui, &tr, &marks, self.string) {
            self.string = s as usize;
            let exists = self
                .cur_beat()
                .note_on(s)
                .map(|n| n.fret == f)
                .unwrap_or(false);
            if exists {
                self.edit(|so, t, b, j, _| {
                    so.tracks[t].bars[b].beats[j]
                        .notes
                        .retain(|n| n.string != s)
                });
            } else {
                self.set_fret(s as usize, f);
            }
        }
    }

    pub(super) fn score(&mut self, ui: &mut Ui) {
        let shown = self.shown_tracks();
        let avail = ui.available_size();
        // Ctrl + mouse wheel zooms.
        if ui.rect_contains_pointer(ui.max_rect()) {
            let zd = ui.input(|i| i.zoom_delta());
            if (zd - 1.0).abs() > 0.001 {
                self.zoom = (self.zoom * zd).clamp(0.3, 3.0);
            }
        }
        let screen_w = match self.view_mode {
            3 => Some((avail.x - 2.0 * score::PAGE_GAP - 16.0).max(300.0)),
            4 => Some(1.0e6),
            _ => None,
        };
        let mut lay = score::layout_ex(
            &self.song,
            &shown,
            self.track,
            self.zoom,
            self.notation,
            screen_w,
            self.view_mode == 4,
        );
        lay.cols = match self.view_mode {
            1 => lay.pages,
            2 => (((avail.x - score::PAGE_GAP) / (lay.page_w() + score::PAGE_GAP)).floor()
                as usize)
                .max(1)
                .min(lay.pages),
            _ => 1,
        };
        egui::ScrollArea::both()
            .id_salt("score")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let total = Vec2::new(lay.total_w().max(avail.x), lay.total_h());
                let (rect, resp) = ui.allocate_exact_size(total, Sense::click_and_drag());
                let left =
                    rect.left() + ((rect.width() - lay.total_w()) * 0.5).max(0.0) + score::PAGE_GAP;
                let painter = ui.painter().clone();
                let clip = ui.clip_rect();
                let dc = score::DrawCtx {
                    song: &self.song,
                    ti: self.track,
                    cursor: (self.bar, self.beat, self.string),
                    selection: self.selection(),
                    play_tick: if self.playing { self.play_tick } else { None },
                    music_font: self.music_font,
                    show_cursor: !self.playing,
                    voice: self.voice,
                    cursor_std: self.std_cursor,
                };
                for page in 0..lay.pages {
                    let o = lay.page_origin(page, left, rect.top());
                    let pr = Rect::from_min_size(o, Vec2::new(lay.page_w(), lay.page_h()));
                    if pr.intersects(clip) {
                        score::draw_page(&painter, &lay, &dc, page, o);
                    }
                }
                // Which page and beat is under a screen point.
                let locate = |pp: Pos2| -> Option<(usize, Pos2)> {
                    (0..lay.pages).find_map(|page| {
                        let o = lay.page_origin(page, left, rect.top());
                        let pr = Rect::from_min_size(o, Vec2::new(lay.page_w(), lay.page_h()));
                        pr.contains(pp).then(|| (page, (pp - o).to_pos2()))
                    })
                };
                // Drag across the score to select beats of one track.
                if resp.drag_started() {
                    if let Some(pp) = resp.interact_pointer_pos()
                        && let Some((page, rel)) = locate(pp)
                        && let Some((ti, b, j, _)) =
                            score::hit_test(&lay, &self.song, self.voice, page, rel)
                    {
                        self.track = ti;
                        self.bar = b;
                        self.beat = j;
                        self.clamp_cursor();
                        self.sel_anchor = Some((self.bar, self.beat));
                    }
                } else if resp.dragged()
                    && let Some(pp) = resp.interact_pointer_pos()
                    && let Some((page, rel)) = locate(pp)
                    && let Some((ti, b, j, _)) =
                        score::hit_test(&lay, &self.song, self.voice, page, rel)
                    && ti == self.track
                {
                    self.bar = b;
                    self.beat = j;
                    self.clamp_cursor();
                }
                if resp.clicked()
                    && let Some(pp) = resp.interact_pointer_pos()
                    && let Some((page, rel)) = locate(pp)
                {
                    if let Some((ti, b, j, s)) =
                        score::hit_test(&lay, &self.song, self.voice, page, rel)
                    {
                        let shift = ui.input(|i| i.modifiers.shift);
                        if shift && ti == self.track {
                            if self.sel_anchor.is_none() {
                                self.sel_anchor = Some((self.bar, self.beat));
                            }
                        } else {
                            self.sel_anchor = None;
                        }
                        let was_playing = self.playing;
                        if was_playing {
                            // A click while playing sets a new starting point.
                            self.stop();
                        }
                        self.track = ti;
                        self.bar = b;
                        self.beat = j;
                        match s {
                            Some(score::Hit::Tab(st)) => {
                                self.string = st;
                                self.std_cursor = None;
                            }
                            Some(score::Hit::Std(pos)) => self.std_cursor = Some(pos),
                            _ => {}
                        }
                        self.clamp_cursor();
                        self.last_digit = None;
                        if was_playing {
                            self.play();
                        }
                        match s {
                            Some(score::Hit::Tempo) => self.open_automations(b),
                            Some(score::Hit::Head(part)) => {
                                // Clef: no clef choice yet, so nothing opens.
                                if part == 1 {
                                    self.open_popup(super::popups::Popup::KeySig);
                                } else if part == 2 {
                                    self.open_popup(super::popups::Popup::TimeSig);
                                }
                            }
                            Some(score::Hit::Above(zone)) => {
                                let beat = self.cur_beat().clone();
                                if zone == 0 && self.song.master[self.bar].marker.is_some() {
                                    self.open_popup(super::popups::Popup::Section);
                                } else if beat.chord.is_some() {
                                    self.open_chord_tool();
                                } else if beat.text.is_some() {
                                    self.open_popup(super::popups::Popup::Text);
                                }
                            }
                            _ => {}
                        }
                    } else if let Some(zone) = score::header_zone(&lay, page, rel) {
                        // Clicking the title, tunings or tempo opens their settings.
                        self.show_inspector = true;
                        if zone == 2 {
                            // The tempo marking at the top of the score.
                            self.open_automations(0);
                        } else {
                            self.inspector = if zone == 1 {
                                InspectorTab::Track
                            } else {
                                InspectorTab::Song
                            };
                        }
                    }
                }
                if self.scroll_to_cursor {
                    self.scroll_to_cursor = false;
                    let target = self.play_beat.unwrap_or((self.bar, self.beat));
                    if let Some((page, r)) =
                        lay.beat_rect(&self.song, target.0, target.1, self.voice)
                    {
                        let o = lay.page_origin(page, left, rect.top());
                        ui.scroll_to_rect(r.translate(o.to_vec2()).expand(20.0), None);
                    }
                }
            });
    }
}
