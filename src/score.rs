//! Lays out and draws the song as pages of standard notation and tablature,
//! with every shown track stacked in each system (a full score).

mod draw;
pub use draw::{key_positions, note_staff_pos, set_ink, staff_pos_name, staff_pos_pitch, written_name};

use crate::model::*;
use draw::*;
use egui::{epaint::TextShape, Align2, Color32, FontFamily, FontId, Painter, Pos2, Rect, Stroke as PStroke, Vec2};
use std::collections::BTreeMap;

pub const PAGE_W: f32 = 820.0;
pub const PAGE_H: f32 = 1160.0;
pub const PAGE_GAP: f32 = 24.0;
const MARGIN_X: f32 = 56.0;
const MARGIN_TOP: f32 = 44.0;
const MARGIN_BOTTOM: f32 = 56.0;

pub const INK: Color32 = Color32::from_rgb(20, 20, 24);
pub const PLAY: Color32 = Color32::from_rgb(240, 130, 20);

#[derive(Clone, Copy, PartialEq)]
pub enum Notation {
    Both,
    TabOnly,
    StandardOnly,
}

/// Vertical layout of one track inside a system (offsets from the system top).
#[derive(Clone, Copy)]
pub struct Geom {
    pub z: f32,
    pub ls: f32,
    pub ts: f32,
    pub std: bool,
    pub tab: bool,
    pub strings: usize,
    pub top: f32,
    pub chord_y: f32,
    pub std_top: f32,
    pub lane_top: f32,
    pub tab_top: f32,
    pub rhythm_top: f32,
    pub bottom: f32,
    /// Keyboards: a second (bass clef) staff joined by a brace.
    pub grand: bool,
    pub std2_top: f32,
    pub clef: Clef,
}

/// Which staves a track shows (notation, tablature) for the chosen view. Only
/// stringed instruments have tablature; other tracks always show their notation.
pub fn staves(t: &Track, notation: Notation) -> (bool, bool) {
    let tab_ok = t.has_tab();
    match notation {
        Notation::Both => (t.show_std || !tab_ok, tab_ok),
        Notation::TabOnly => (!tab_ok, tab_ok),
        Notation::StandardOnly => (true, false),
    }
}

impl Geom {
    fn new(z: f32, notation: Notation, track: &Track, top: f32, first: bool, lyrics: bool, diagrams: bool) -> Geom {
        let strings = track.strings();
        let ls = 7.5 * z;
        let ts = 12.5 * z;
        let (std, tab) = staves(track, notation);
        let grand = std && track.grand();
        let chord_y = top + if first { 40.0 } else { 16.0 } * z + if diagrams { 52.0 * z } else { 0.0 };
        let mut y = chord_y + 2.0 * z;
        let std_top = y + 16.0 * z;
        let mut std2_top = std_top;
        if std {
            y = std_top + 4.0 * ls + 34.0 * z;
            if grand {
                std2_top = std_top + 4.0 * ls + 48.0 * z;
                y = std2_top + 4.0 * ls + 34.0 * z;
            }
        }
        let lane_top = y;
        let tab_top = lane_top + 18.0 * z;
        let mut rhythm_top = tab_top;
        if tab {
            rhythm_top = tab_top + (strings as f32 - 1.0) * ts + 4.0 * z;
            y = rhythm_top + 26.0 * z;
        }
        Geom {
            z,
            ls,
            ts,
            std,
            tab,
            strings,
            top,
            chord_y,
            std_top,
            lane_top,
            tab_top,
            rhythm_top,
            bottom: y + if lyrics { 20.0 } else { 4.0 } * z,
            grand,
            std2_top,
            clef: track.clef(),
        }
    }
    /// Offset from the system top of a notation position (half-spaces above the
    /// first staff's bottom line). On a grand staff, notes below middle C go on
    /// the lower staff.
    pub fn pos_y(&self, pos: i32) -> f32 {
        if self.grand && pos <= -3 {
            self.std2_top + 4.0 * self.ls - (pos + 12) as f32 * self.ls * 0.5
        } else {
            self.std_top + 4.0 * self.ls - pos as f32 * self.ls * 0.5
        }
    }
    /// Top of each notation staff with its clef.
    pub fn std_staves(&self) -> Vec<(f32, Clef)> {
        match (self.std, self.grand) {
            (false, _) => vec![],
            (true, false) => vec![(self.std_top, self.clef)],
            (true, true) => vec![(self.std_top, self.clef), (self.std2_top, Clef::Bass)],
        }
    }
    pub fn string_y(&self, s: usize) -> f32 {
        self.tab_top + s as f32 * self.ts
    }
    /// Top and bottom staff lines of this track.
    pub fn staff_span(&self) -> (f32, f32) {
        let top = if self.std { self.std_top } else { self.tab_top };
        let bottom = if self.tab { self.string_y(self.strings - 1) } else if self.grand { self.std2_top + 4.0 * self.ls } else { self.std_top + 4.0 * self.ls };
        (top, bottom)
    }
}

#[derive(Clone)]
pub struct Block {
    pub ti: usize,
    pub g: Geom,
}

#[derive(Clone)]
pub struct BarLay {
    pub bar: usize,
    pub x: f32,
    pub w: f32,
    /// Per block: note x of each beat, and the column width it owns.
    pub beats_x: Vec<Vec<f32>>,
    pub beats_w: Vec<Vec<f32>>,
    pub clef: bool,
    pub time: bool,
    pub key_shown: bool,
    /// The shared time grid: onset ticks, column x and width, and where the columns end.
    pub cols_t: Vec<u32>,
    pub cols_x: Vec<f32>,
    pub cols_w: Vec<f32>,
    pub cols_end: f32,
}

impl BarLay {
    /// Note x and column width of each beat of a list of beats (any voice).
    pub fn voice_xs(&self, beats: &[Beat]) -> (Vec<f32>, Vec<f32>) {
        let n = self.cols_t.len();
        let mut idxs = vec![];
        let mut t = 0u32;
        for b in beats {
            idxs.push(self.cols_t.binary_search(&t).unwrap_or_else(|e| e.min(n.saturating_sub(1))));
            t += b.ticks();
        }
        let mut xs = vec![];
        let mut ws = vec![];
        for (j, &idx) in idxs.iter().enumerate() {
            let end = idxs.get(j + 1).copied().unwrap_or(n).max(idx + 1).min(self.cols_w.len());
            let w: f32 = self.cols_w.get(idx..end).map(|x| x.iter().sum()).unwrap_or(20.0);
            let cw = self.cols_w.get(idx).copied().unwrap_or(20.0);
            xs.push(self.cols_x.get(idx).copied().unwrap_or(self.cols_end) + cw * 0.35);
            ws.push(w.max(cw));
        }
        (xs, ws)
    }
}

#[derive(Clone)]
pub struct SysLay {
    pub page: usize,
    pub y: f32,
    pub bars: Vec<BarLay>,
}

pub struct Layout {
    pub z: f32,
    pub blocks: Vec<Block>,
    pub cur_block: usize,
    pub sys_h: f32,
    pub pages: usize,
    pub systems: Vec<SysLay>,
    pub bar_pos: Vec<(usize, usize)>,
    /// Page size in unzoomed units.
    pub pw: f32,
    pub ph: f32,
    /// Pages per row (1 = pages one under another).
    pub cols: usize,
}

impl Layout {
    pub fn page_w(&self) -> f32 {
        self.pw * self.z
    }
    pub fn page_h(&self) -> f32 {
        self.ph * self.z
    }
    pub fn total_h(&self) -> f32 {
        let rows = self.pages.div_ceil(self.cols.max(1));
        rows as f32 * (self.page_h() + PAGE_GAP) + PAGE_GAP
    }
    pub fn total_w(&self) -> f32 {
        let cols = self.cols.max(1).min(self.pages.max(1));
        cols as f32 * (self.page_w() + PAGE_GAP) + PAGE_GAP
    }
    pub fn page_origin(&self, page: usize, left: f32, top: f32) -> Pos2 {
        let cols = self.cols.max(1);
        let (row, col) = (page / cols, page % cols);
        Pos2::new(left + col as f32 * (self.page_w() + PAGE_GAP), top + PAGE_GAP + row as f32 * (self.page_h() + PAGE_GAP))
    }
    /// Rect of a beat of the current track (relative to the page origin), and its page.
    pub fn beat_rect(&self, song: &Song, bar: usize, beat: usize, voice: usize) -> Option<(usize, Rect)> {
        let (si, bi) = *self.bar_pos.get(bar)?;
        let sys = &self.systems[si];
        let b = &sys.bars[bi];
        let (xs, wsv) = b.voice_xs(song.tracks[self.blocks[self.cur_block].ti].bars[bar].voice(voice));
        let x = *xs.get(beat)?;
        let w = wsv[beat];
        let g = self.blocks[self.cur_block].g;
        Some((sys.page, Rect::from_min_max(Pos2::new(x - w * 0.35, sys.y + g.top), Pos2::new(x + w * 0.65, sys.y + g.bottom))))
    }
}

fn col_width(eff_ticks: u32, extra: f32, z: f32) -> f32 {
    let r = eff_ticks as f32 / QUARTER as f32;
    (15.0 + 30.0 * r.powf(0.55) + extra) * z
}

/// Shared time grid of one bar over all shown tracks: onset ticks and widths.
fn bar_columns(song: &Song, tis: &[usize], bi: usize, z: f32) -> (Vec<u32>, Vec<f32>) {
    let mut onsets: BTreeMap<u32, (u32, f32)> = BTreeMap::new();
    for (&ti, v) in tis.iter().flat_map(|ti| (0..VOICES).map(move |v| (ti, v))) {
        let bar = &song.tracks[ti].bars[bi];
        if v > 0 && !bar.voice_used(v) {
            continue;
        }
        let mut t = 0u32;
        for b in bar.voice(v) {
            let mut extra: f32 = 0.0;
            if song.tracks[ti].has_tab() && b.notes.iter().any(|n| n.fret >= 10) {
                extra += 4.0;
            }
            if b.notes.iter().any(|n| n.ghost || n.harmonic || n.tie) {
                extra += 6.0;
            }
            let e = onsets.entry(t).or_insert((u32::MAX, 0.0));
            e.0 = e.0.min(b.ticks().max(1));
            e.1 = e.1.max(extra);
            t += b.ticks();
        }
    }
    let keys: Vec<u32> = onsets.keys().copied().collect();
    let mut widths = vec![];
    for (k, &t) in keys.iter().enumerate() {
        let (d, extra) = onsets[&t];
        let eff = match keys.get(k + 1) {
            Some(&n) => d.min(n - t),
            None => d,
        };
        widths.push(col_width(eff, extra, z));
    }
    (keys, widths)
}

pub fn layout(song: &Song, shown: &[usize], cur: usize, z: f32, notation: Notation) -> Layout {
    layout_ex(song, shown, cur, z, notation, None, false)
}

/// What part of the page header a click is on: 0 title, 1 tracks and tunings, 2 tempo.
pub fn header_zone(lay: &Layout, page: usize, rel: Pos2) -> Option<u8> {
    if page != 0 || lay.systems.first().map(|s| rel.y >= s.y).unwrap_or(true) || rel.y < MARGIN_TOP * lay.z {
        return None;
    }
    let y = rel.y / lay.z - MARGIN_TOP;
    Some(if y < 96.0 {
        0
    } else if y < 132.0 {
        1
    } else {
        2
    })
}

/// With `screen_w` (in screen pixels) the music runs on one continuous sheet
/// as wide as the window instead of separate pages.
pub fn layout_ex(song: &Song, shown: &[usize], cur: usize, z: f32, notation: Notation, screen_w: Option<f32>, horizontal: bool) -> Layout {
    let mut pw = screen_w.map(|w| (w / z).max(560.0)).unwrap_or(PAGE_W);
    let continuous = screen_w.is_some();
    let mut tis: Vec<usize> = shown.iter().copied().filter(|&t| t < song.tracks.len()).collect();
    if !tis.contains(&cur) {
        tis.push(cur);
        tis.sort();
    }
    let mut blocks = vec![];
    let mut top = 0.0;
    for (k, &ti) in tis.iter().enumerate() {
        let t = &song.tracks[ti];
        let lyrics = !t.lyrics.trim().is_empty() || t.bars.iter().any(|b| b.beats.iter().any(|x| x.lyric.is_some()));
        let diagrams = t.bars.iter().any(|b| b.beats.iter().any(|x| x.chord_diagram && !x.is_rest()));
        let g = Geom::new(z, notation, t, top, k == 0, lyrics, diagrams);
        top = g.bottom;
        blocks.push(Block { ti, g });
    }
    let cur_block = tis.iter().position(|&t| t == cur).unwrap_or(0);
    let sys_h = top + 16.0 * z;
    let content_w = (pw - 2.0 * MARGIN_X) * z;
    let header_h = (150.0 + 0.0) * z;

    struct Pre {
        bi: usize,
        head: f32,
        onsets: Vec<u32>,
        widths: Vec<f32>,
        clef: bool,
        time: bool,
        key_shown: bool,
    }
    let head_w = |clef: bool, time: bool, rep: bool, keys: usize| -> f32 {
        let mut h = 10.0 * z;
        if clef {
            h += 30.0 * z;
        }
        if time {
            h += 20.0 * z;
        }
        h += keys as f32 * 9.0 * z;
        if keys > 0 {
            h += 6.0 * z;
        }
        if rep {
            h += 8.0 * z;
        }
        h
    };
    let mut groups: Vec<Vec<Pre>> = vec![];
    let mut cur_g: Vec<Pre> = vec![];
    let mut used = 0.0;
    let mut prev_sig = (0u8, 0u8);
    let mut prev_key = 0i8;
    for bi in 0..song.bar_count() {
        let mb = &song.master[bi];
        let time = (mb.num, mb.den) != prev_sig;
        prev_sig = (mb.num, mb.den);
        let key_changed = mb.key != prev_key;
        prev_key = mb.key;
        let kn = mb.key.unsigned_abs() as usize;
        let (onsets, widths) = bar_columns(song, &tis, bi, z);
        let sum: f32 = widths.iter().sum();
        let clef = cur_g.is_empty();
        let keys_here = if clef || key_changed { kn } else { 0 };
        let w = head_w(clef, time, mb.repeat_open, keys_here) + sum + 8.0 * z;
        if !cur_g.is_empty() && used + w > content_w {
            groups.push(std::mem::take(&mut cur_g));
            let head = head_w(true, time, mb.repeat_open, kn);
            used = head + sum + 8.0 * z;
            cur_g.push(Pre { bi, head, onsets, widths, clef: true, time, key_shown: kn > 0 });
        } else {
            used += w;
            cur_g.push(Pre { bi, head: head_w(clef, time, mb.repeat_open, keys_here), onsets, widths, clef, time, key_shown: keys_here > 0 });
        }
        if mb.line_break && !cur_g.is_empty() {
            groups.push(std::mem::take(&mut cur_g));
            used = 0.0;
        }
    }
    if !cur_g.is_empty() {
        groups.push(cur_g);
    }

    let mut systems = vec![];
    let mut bar_pos = vec![(0usize, 0usize); song.bar_count()];
    let mut page = 0usize;
    let mut y = MARGIN_TOP * z + header_h + tis.len().saturating_sub(1) as f32 * 0.0;
    let ng = groups.len();
    for (gi, g) in groups.into_iter().enumerate() {
        let fixed: f32 = g.iter().map(|p| p.widths.iter().sum::<f32>()).sum();
        let heads: f32 = g.iter().map(|p| p.head + 8.0 * z).sum();
        let last = gi + 1 == ng;
        let natural = fixed + heads;
        let k = if last && natural < content_w * 0.75 { 1.0 } else { ((content_w - heads) / fixed.max(1.0)).max(0.3) };
        if !continuous && y + sys_h > (PAGE_H - MARGIN_BOTTOM) * z && !systems.is_empty() {
            page += 1;
            y = MARGIN_TOP * z;
        }
        let mut x = MARGIN_X * z;
        let mut bars = vec![];
        for p in g {
            // Column x positions.
            let mut cx = vec![];
            let mut cw = vec![];
            let mut bx = x + p.head;
            for w in &p.widths {
                cx.push(bx);
                cw.push(w * k);
                bx += w * k;
            }
            let mut beats_x = vec![];
            let mut beats_w = vec![];
            for blk in &blocks {
                let beats = &song.tracks[blk.ti].bars[p.bi].beats;
                let mut t = 0u32;
                let mut xs = vec![];
                let mut ws = vec![];
                let mut idxs = vec![];
                for b in beats {
                    let idx = p.onsets.binary_search(&t).unwrap_or_else(|e| e.min(p.onsets.len().saturating_sub(1)));
                    idxs.push(idx);
                    t += b.ticks();
                }
                for (j, &idx) in idxs.iter().enumerate() {
                    let end = idxs.get(j + 1).copied().unwrap_or(p.onsets.len()).max(idx + 1).min(cw.len());
                    let w: f32 = cw[idx..end].iter().sum();
                    xs.push(cx.get(idx).copied().unwrap_or(bx) + cw.get(idx).copied().unwrap_or(20.0) * 0.35);
                    ws.push(w.max(cw.get(idx).copied().unwrap_or(20.0)));
                }
                beats_x.push(xs);
                beats_w.push(ws);
            }
            let end = bx + 8.0 * z;
            bar_pos[p.bi] = (systems.len(), bars.len());
            bars.push(BarLay { bar: p.bi, x, w: end - x, beats_x, beats_w, clef: p.clef, time: p.time, key_shown: p.key_shown, cols_t: p.onsets.clone(), cols_x: cx.clone(), cols_w: cw.clone(), cols_end: bx });
            x = end;
        }
        systems.push(SysLay { page, y, bars });
        y += sys_h;
    }
    Layout {
        z,
        blocks,
        cur_block,
        sys_h,
        pages: page + 1,
        cols: 1,
        bar_pos,
        pw: {
            if horizontal {
                // One long line: the sheet is as wide as the music.
                pw = systems.iter().flat_map(|s| s.bars.last().map(|b| (b.x + b.w) / z)).fold(560.0, f32::max) + MARGIN_X;
            }
            pw
        },
        systems,
        ph: if continuous { (y / z + MARGIN_BOTTOM + 20.0).max(400.0) } else { PAGE_H },
    }
}

pub struct DrawCtx<'a> {
    pub song: &'a Song,
    pub ti: usize,
    pub cursor: (usize, usize, usize),
    pub selection: Option<((usize, usize), (usize, usize))>,
    /// During playback: the bar and the exact tick being heard.
    pub play_tick: Option<(usize, f32)>,
    pub music_font: bool,
    pub show_cursor: bool,
    /// The voice being edited (0 = voice 1).
    pub voice: usize,
    /// Staff position of the cursor when it is in the standard notation.
    pub cursor_std: Option<i32>,
}

/// Where a click landed: on a tab string or a position of the notation staff.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Hit {
    Tab(usize),
    Std(i32),
    /// The start of a bar: 0 = clef, 1 = key signature, 2 = time signature.
    Head(u8),
    /// Above the staves: 0 = section name row, 1 = chord and text row.
    Above(u8),
    /// A tempo marking.
    Tempo,
}

pub(crate) fn fid(size: f32) -> FontId {
    FontId::proportional(size)
}
pub(crate) fn bold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("bold".into()))
}
pub(crate) fn italic(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("italic".into()))
}
pub(crate) fn serif(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("serif".into()))
}
pub(crate) fn serif_bold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("serif-bold".into()))
}
pub(crate) fn serif_bi(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("serif-bi".into()))
}
pub(crate) fn music(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("music".into()))
}

pub fn track_short_name(t: &Track) -> String {
    if t.short_name.trim().is_empty() {
        short_name(&t.name)
    } else {
        t.short_name.clone()
    }
}

pub fn short_name(name: &str) -> String {
    let n: String = name.split_whitespace().last().unwrap_or(name).chars().take(5).collect::<String>().to_lowercase();
    format!("{n}.")
}

pub fn draw_page(p: &Painter, lay: &Layout, dc: &DrawCtx, page: usize, origin: Pos2) {
    let z = lay.z;
    let page_rect = Rect::from_min_size(origin, Vec2::new(lay.page_w(), lay.page_h()));
    p.rect_filled(page_rect.translate(Vec2::new(2.0, 3.0)), 1.0, Color32::from_black_alpha(30));
    p.rect_filled(page_rect, 0.0, Color32::WHITE);
    let song = dc.song;
    let cx = origin.x + lay.page_w() * 0.5;
    if page == 0 {
        let mut y = origin.y + MARGIN_TOP * z;
        p.text(Pos2::new(cx, y), Align2::CENTER_TOP, &song.title, serif(34.0 * z), INK);
        y += 42.0 * z;
        if !song.subtitle.is_empty() {
            p.text(Pos2::new(cx, y), Align2::CENTER_TOP, &song.subtitle, serif_bold(17.0 * z), INK);
            y += 21.0 * z;
        }
        if !song.artist.is_empty() {
            p.text(Pos2::new(cx, y), Align2::CENTER_TOP, &song.artist, serif(15.0 * z), INK);
            y += 19.0 * z;
        }
        if !song.music_by.is_empty() {
            p.text(Pos2::new(cx, y), Align2::CENTER_TOP, format!("Music by {}", song.music_by), serif(12.0 * z), INK);
        }
        // Track names and tunings.
        let mut lx = origin.x + MARGIN_X * z;
        let ty = origin.y + (MARGIN_TOP + 98.0) * z;
        for blk in &lay.blocks {
            let tr = &song.tracks[blk.ti];
            let a = p.text(Pos2::new(lx, ty), Align2::LEFT_TOP, &tr.name, serif(12.0 * z), INK);
            // Only stringed instruments have a tuning to show.
            let mut t2 = if tr.has_strings() && tr.tuning_label_visible { tuning_name(&tr.tuning) } else { String::new() };
            if t2.contains('(') {
                t2 = "Standard tuning".into();
            }
            if tr.has_strings() && tr.tuning_label_visible && !tr.tuning_label.trim().is_empty() {
                t2 = tr.tuning_label.clone();
            }
            if tr.capo > 0 {
                t2 = format!("{t2}, capo {}", tr.capo);
            }
            let b = p.text(Pos2::new(lx, ty + 16.0 * z), Align2::LEFT_TOP, t2, serif(12.0 * z), INK);
            lx += a.width().max(b.width()) + 22.0 * z;
        }
        let tp = Pos2::new(origin.x + (MARGIN_X + 30.0) * z, ty + 46.0 * z);
        draw_small_note(p, tp, z);
        let r = p.text(Pos2::new(tp.x + 9.0 * z, tp.y), Align2::LEFT_CENTER, format!("= {}", song.tempo), serif(14.0 * z), INK);
        if song.master[0].tempo_ramp {
            let next = song.tempo_points().get(1).map(|q| q.1).unwrap_or(song.tempo);
            let word = if next > song.tempo { "accel." } else if next < song.tempo { "rit." } else { "" };
            p.text(Pos2::new(r.right() + 6.0 * z, tp.y), Align2::LEFT_CENTER, word, serif_bi(12.0 * z), INK);
        }
    }
    p.text(Pos2::new(origin.x + lay.page_w() - MARGIN_X * z, origin.y + lay.page_h() - 28.0 * z), Align2::RIGHT_CENTER, format!("{}/{}", page + 1, lay.pages), serif(10.0 * z), INK);
    for (si, sys) in lay.systems.iter().enumerate() {
        if sys.page == page {
            draw_system(p, lay, dc, si, origin);
        }
    }
}

fn draw_system(p: &Painter, lay: &Layout, dc: &DrawCtx, si: usize, o: Pos2) {
    let z = lay.z;
    let sys = &lay.systems[si];
    let song = dc.song;
    let y0 = o.y + sys.y;
    let x_start = o.x + sys.bars[0].x;
    let last = sys.bars.last().unwrap();
    let x_end = o.x + last.x + last.w;
    let line = PStroke::new(0.8 * z.max(0.8), Color32::from_gray(70));
    let thick = PStroke::new(1.0 * z.max(1.0), INK);
    let cb = lay.cur_block;
    let cg = lay.blocks[cb].g;

    // Highlights for the current track (under everything).
    for bl in &sys.bars {
        let (vxs, vws) = bl.voice_xs(song.tracks[dc.ti].bars[bl.bar].voice(dc.voice));
        for (bj, (&bx, &bw)) in vxs.iter().zip(vws.iter()).enumerate() {
            let col = Rect::from_min_max(Pos2::new(o.x + bx - bw * 0.35, y0 + cg.chord_y + 4.0 * z), Pos2::new(o.x + bx + bw * 0.65, y0 + cg.bottom - 4.0 * z));
            if let Some((a, b)) = dc.selection {
                let k = (bl.bar, bj);
                if k >= a && k <= b {
                    p.rect_filled(col, 0.0, Color32::from_rgba_unmultiplied(80, 140, 240, 45));
                }
            }
        }
        // Playback: the bar being played is lightly marked, the notes sounding now
        // (in every track) are highlighted, and a line glides from note to note.
        if let Some((pb, tick)) = dc.play_tick {
            if pb == bl.bar && !bl.cols_t.is_empty() {
                let top = y0 + 20.0 * z;
                let bottom = y0 + lay.sys_h - 18.0 * z;
                let bar_r = Rect::from_min_max(Pos2::new(o.x + bl.x, top), Pos2::new(o.x + bl.x + bl.w, bottom));
                p.rect_filled(bar_r, 0.0, Color32::from_rgba_unmultiplied(250, 150, 30, 14));
                let k = bl.cols_t.partition_point(|&t| t as f32 <= tick).saturating_sub(1);
                let cw = bl.cols_w.get(k).copied().unwrap_or(20.0);
                let cx = bl.cols_x.get(k).copied().unwrap_or(bl.cols_end);
                let col = Rect::from_min_max(Pos2::new(o.x + cx, top), Pos2::new(o.x + cx + cw, bottom));
                p.rect_filled(col, 2.0, Color32::from_rgba_unmultiplied(250, 150, 30, 46));
                let t0 = bl.cols_t[k] as f32;
                let t1 = bl.cols_t.get(k + 1).map(|&t| t as f32).unwrap_or(song.master[bl.bar].ticks() as f32).max(t0 + 1.0);
                let x0 = o.x + cx + cw * 0.35;
                let x1 = o.x + bl.cols_x.get(k + 1).map(|&x| x + bl.cols_w[k + 1] * 0.35).unwrap_or(bl.x + bl.w);
                let f = ((tick - t0) / (t1 - t0)).clamp(0.0, 1.0);
                let x = x0 - 6.0 * z + (x1 - x0) * f;
                p.line_segment([Pos2::new(x, top), Pos2::new(x, bottom)], PStroke::new(2.0 * z, PLAY));
            }
        }
    }

    let first_top = lay.blocks[0].g.staff_span().0;
    let last_bottom = lay.blocks.last().unwrap().g.staff_span().1;
    // System bracket across all tracks.
    p.line_segment([Pos2::new(x_start, y0 + first_top), Pos2::new(x_start, y0 + last_bottom)], thick);

    for (k, blk) in lay.blocks.iter().enumerate() {
        let g = blk.g;
        let track = &song.tracks[blk.ti];
        for (st, _) in g.std_staves() {
            for i in 0..5 {
                let y = y0 + st + i as f32 * g.ls;
                p.line_segment([Pos2::new(x_start, y), Pos2::new(x_end, y)], line);
            }
        }
        if g.grand {
            // The brace joining the two staves of a keyboard part.
            let (a, b) = (y0 + g.std_top, y0 + g.std2_top + 4.0 * g.ls);
            let m = (a + b) * 0.5;
            let bx = x_start - 5.0 * z;
            for (from, to) in [(a, m), (b, m)] {
                let mid = (from + to) * 0.5;
                let pts = [Pos2::new(bx, from), Pos2::new(bx - 7.0 * z, mid), Pos2::new(bx - 1.0 * z, to - (to - from).signum() * 2.0 * z)];
                p.add(egui::epaint::QuadraticBezierShape::from_points_stroke(pts, false, Color32::TRANSPARENT, PStroke::new(2.2 * z, INK)));
                p.line_segment([Pos2::new(bx - 1.0 * z, to - (to - from).signum() * 2.0 * z), Pos2::new(bx - 4.0 * z, m)], PStroke::new(1.6 * z, INK));
            }
        }
        if g.tab {
            for s in 0..g.strings {
                let y = y0 + g.string_y(s);
                p.line_segment([Pos2::new(x_start, y), Pos2::new(x_end, y)], line);
            }
        }
        // Rotated short track name in the left margin.
        let (t, b) = g.staff_span();
        let col = if blk.ti == dc.ti { INK } else { Color32::from_gray(90) };
        let gal = p.layout_no_wrap(track_short_name(track), serif(11.0 * z), col);
        let w = gal.size().x;
        let pos = Pos2::new(x_start - if g.grand { 24.0 } else { 18.0 } * z, y0 + (t + b) * 0.5 + w * 0.5);
        p.add(TextShape::new(pos, gal, col).with_angle(-std::f32::consts::FRAC_PI_2));
        let _ = k;
    }

    let lyric_maps: Vec<std::collections::HashMap<(usize, usize), String>> = lay.blocks.iter().map(|b| song.lyric_map(b.ti)).collect();
    for (kbar, bl) in sys.bars.iter().enumerate() {
        let bi = bl.bar;
        let mb = &song.master[bi];
        let bx = o.x + bl.x;
        let ex = bx + bl.w;
        let g0 = lay.blocks[0].g;
        let top0 = y0 + g0.staff_span().0;
        // Bar number, section name and tempo changes, above the first track.
        p.text(Pos2::new(bx + 2.0 * z, top0 - 3.0 * z), Align2::LEFT_BOTTOM, format!("{}", bi + 1), fid(9.0 * z), Color32::from_gray(100));
        if mb.endings != 0 {
            let ey = top0 - 13.0 * z;
            let first = bi == 0 || song.master[bi - 1].endings != mb.endings;
            let last = bi + 1 >= song.bar_count() || song.master[bi + 1].endings != mb.endings;
            let st = PStroke::new(1.0 * z, INK);
            p.line_segment([Pos2::new(bx + 2.0 * z, ey), Pos2::new(ex - 3.0 * z, ey)], st);
            if first {
                p.line_segment([Pos2::new(bx + 2.0 * z, ey), Pos2::new(bx + 2.0 * z, ey + 8.0 * z)], st);
                let lab: Vec<String> = (0..8).filter(|k| mb.endings & (1 << k) != 0).map(|k| format!("{}.", k + 1)).collect();
                p.text(Pos2::new(bx + 5.0 * z, ey + 1.0 * z), Align2::LEFT_TOP, lab.join(" "), serif(11.0 * z), INK);
            }
            if last {
                p.line_segment([Pos2::new(ex - 3.0 * z, ey), Pos2::new(ex - 3.0 * z, ey + 8.0 * z)], st);
            }
        }
        let prev_feel = bi > 0 && song.master[bi - 1].triplet_feel;
        if mb.triplet_feel != prev_feel {
            let lab = if mb.triplet_feel { "Swing feel (\u{266B} = \u{2669}\u{00B3}\u{266A})" } else { "Straight feel" };
            p.text(Pos2::new(bx + if mb.marker.is_some() { 60.0 } else { 4.0 } * z, y0 + 7.0 * z), Align2::LEFT_TOP, lab, serif_bi(11.0 * z), INK);
        }
        if let Some(m) = &mb.marker {
            let framed = m.starts_with('[') && m.ends_with(']') && m.len() > 2;
            let label = if framed { &m[1..m.len() - 1] } else { m.as_str() };
            let r = p.text(Pos2::new(bx + 2.0 * z + if framed { 3.0 * z } else { 0.0 }, y0 + 6.0 * z), Align2::LEFT_TOP, label, serif_bold(14.0 * z), INK);
            if framed {
                p.rect_stroke(r.expand2(Vec2::new(3.0 * z, 1.0 * z)), 0.0, PStroke::new(1.0 * z, INK), egui::StrokeKind::Outside);
            }
        }
        // Direction targets (Segno, Coda, Fine) at the start of the bar, jumps at its end.
        if mb.direction_target != 0 {
            let tx = bx + if mb.marker.is_some() { 4.0 } else { 6.0 } * z;
            let ty = top0 - 16.0 * z;
            match mb.direction_target {
                DIR_FINE => {
                    p.text(Pos2::new(tx, ty), Align2::LEFT_CENTER, "Fine", serif_bi(13.0 * z), INK);
                }
                t => {
                    let glyph = match t {
                        DIR_SEGNO => "\u{1D10B}",
                        DIR_SEGNO_SEGNO => "\u{1D10B}\u{1D10B}",
                        DIR_CODA => "\u{1D10C}",
                        _ => "\u{1D10C}\u{1D10C}",
                    };
                    let gx = if mb.marker.is_some() { bx + bl.w * 0.5 } else { tx + 6.0 * z };
                    p.text(Pos2::new(gx, ty - 2.0 * z), Align2::CENTER_CENTER, glyph, music(20.0 * z), INK);
                }
            }
        }
        if mb.direction_jump != 0 {
            let mut lab = jump_label(mb.direction_jump).to_string();
            if matches!(mb.direction_jump, 9 | 10) {
                lab.push_str(if mb.direction_jump == 9 { " \u{1D10C}" } else { " \u{1D10C}\u{1D10C}" });
            }
            p.text(Pos2::new(ex - 4.0 * z, top0 - 14.0 * z), Align2::RIGHT_CENTER, lab, FontId::new(12.5 * z, FontFamily::Name("serif-bi".into())), INK);
        }
        if false {
        }
        if let Some(t) = mb.tempo {
            if bi != 0 {
                let tp = Pos2::new(bx + 70.0 * z, y0 + 14.0 * z);
                draw_small_note(p, tp, z);
                let r = p.text(Pos2::new(tp.x + 9.0 * z, tp.y), Align2::LEFT_CENTER, format!("= {t}"), serif(13.0 * z), INK);
                if mb.tempo_ramp {
                    let next = song.tempo_points().into_iter().find(|q| q.0 > bi).map(|q| q.1).unwrap_or(t);
                    let word = if next > t { "accel." } else if next < t { "rit." } else { "" };
                    p.text(Pos2::new(r.right() + 6.0 * z, tp.y), Align2::LEFT_CENTER, word, serif_bi(12.0 * z), INK);
                }
            }
        }
        for (k, blk) in lay.blocks.iter().enumerate() {
            let g = blk.g;
            let track = &song.tracks[blk.ti];
            let bar = &track.bars[bi];
            let (top, bottom) = g.staff_span();
            let (top, bottom) = (y0 + top, y0 + bottom);
            let wrong = bar.wrong_length(mb.ticks());
            // Bars that are too short or too long show in red (except the one being edited).
            if wrong && dc.cursor.0 != bi {
                p.rect_filled(Rect::from_min_max(Pos2::new(bx + 1.0, top - 3.0 * z), Pos2::new(ex - 1.0, bottom + 3.0 * z)), 0.0, Color32::from_rgba_unmultiplied(235, 60, 60, 30));
                p.rect_filled(Rect::from_min_max(Pos2::new(bx + 1.0, top - 2.0 * z), Pos2::new(ex - 1.0, top)), 0.0, Color32::from_rgba_unmultiplied(230, 60, 60, 160));
            }
            // How far off the bar is: "+1 beat" to add, "-1/2 beat" to remove (for the first wrong voice).
            if wrong {
                if let Some(&(_, d)) = bar.length_errors(mb.ticks()).first() {
                    let words = crate::model::beats_words(d.unsigned_abs() as u32, mb.den);
                    let label = if d < 0 { format!("+{words} needed") } else { format!("\u{2212}{words} too many") };
                    p.text(Pos2::new(ex - 4.0 * z, top - 4.0 * z), Align2::RIGHT_BOTTOM, label, bold(8.5 * z), Color32::from_rgb(210, 45, 45));
                }
            }
            // Bar lines.
            if mb.repeat_close > 0 {
                p.line_segment([Pos2::new(ex - 4.0 * z, top), Pos2::new(ex - 4.0 * z, bottom)], line);
                p.line_segment([Pos2::new(ex, top), Pos2::new(ex, bottom)], PStroke::new(3.0 * z, INK));
                repeat_dots(p, &g, y0, ex - 9.0 * z, z);
                if mb.repeat_close > 2 && k == 0 {
                    p.text(Pos2::new(ex - 2.0 * z, top - 4.0 * z), Align2::RIGHT_BOTTOM, format!("x{}", mb.repeat_close), fid(10.0 * z), INK);
                }
            } else if mb.double_bar && bi + 1 != song.bar_count() {
                p.line_segment([Pos2::new(ex - 3.5 * z, top), Pos2::new(ex - 3.5 * z, bottom)], line);
                p.line_segment([Pos2::new(ex, top), Pos2::new(ex, bottom)], line);
            } else if bi + 1 == song.bar_count() {
                p.line_segment([Pos2::new(ex - 4.0 * z, top), Pos2::new(ex - 4.0 * z, bottom)], line);
                p.line_segment([Pos2::new(ex, top), Pos2::new(ex, bottom)], PStroke::new(3.0 * z, INK));
            } else {
                if g.std {
                    let low = if g.grand { g.std2_top } else { g.std_top };
                    p.line_segment([Pos2::new(ex, y0 + g.std_top), Pos2::new(ex, y0 + low + 4.0 * g.ls)], line);
                }
                if g.tab {
                    p.line_segment([Pos2::new(ex, y0 + g.tab_top), Pos2::new(ex, y0 + g.string_y(g.strings - 1))], line);
                }
            }
            let mut hx = bx + 4.0 * z;
            if bl.clef {
                for (st, cl) in g.std_staves() {
                    clef(p, Pos2::new(hx + 10.0 * z, y0 + st), &g, cl, dc.music_font);
                }
                if g.tab {
                    let mid = y0 + g.tab_top + (g.strings as f32 - 1.0) * g.ts * 0.5;
                    let sp = ((g.strings as f32 - 1.0) * g.ts / 3.2).min(13.0 * z);
                    for (i, ch) in ["T", "A", "B"].iter().enumerate() {
                        p.text(Pos2::new(hx + 10.0 * z, mid + (i as f32 - 1.0) * sp), Align2::CENTER_CENTER, *ch, bold(sp * 1.05), INK);
                    }
                }
                hx += 28.0 * z;
            }
            if bl.key_shown && mb.key != 0 {
                for (st, cl) in g.std_staves() {
                    if cl == Clef::Percussion {
                        continue;
                    }
                    let glyph = if mb.key > 0 { "\u{266F}" } else { "\u{266D}" };
                    for (i, pos) in draw::key_positions(mb.key).iter().enumerate() {
                        let pos = if cl.is_bass() { pos - 2 } else { *pos };
                        let yy = y0 + st + 4.0 * g.ls - pos as f32 * g.ls * 0.5;
                        p.text(Pos2::new(hx + 5.0 * z + i as f32 * 9.0 * z, yy), Align2::CENTER_CENTER, glyph, music(g.ls * 2.8), INK);
                    }
                }
                hx += mb.key.unsigned_abs() as f32 * 9.0 * z + 6.0 * z;
            }
            if mb.repeat_open {
                let lx = if kbar == 0 && !bl.clef { bx } else { hx };
                p.line_segment([Pos2::new(lx, top), Pos2::new(lx, bottom)], PStroke::new(3.0 * z, INK));
                p.line_segment([Pos2::new(lx + 4.0 * z, top), Pos2::new(lx + 4.0 * z, bottom)], line);
                repeat_dots(p, &g, y0, lx + 9.0 * z, z);
                hx += 10.0 * z;
            }
            if bl.time {
                let tx = hx + 8.0 * z;
                for (st, _) in g.std_staves() {
                    let f = bold(g.ls * 2.6);
                    p.text(Pos2::new(tx, y0 + st + g.ls), Align2::CENTER_CENTER, format!("{}", mb.num), f.clone(), INK);
                    p.text(Pos2::new(tx, y0 + st + 3.0 * g.ls), Align2::CENTER_CENTER, format!("{}", mb.den), f, INK);
                }
                if g.tab {
                    let h = (g.strings as f32 - 1.0) * g.ts;
                    let f = bold((h * 0.42).min(26.0 * z));
                    p.text(Pos2::new(tx, y0 + g.tab_top + h * 0.27), Align2::CENTER_CENTER, format!("{}", mb.num), f.clone(), INK);
                    p.text(Pos2::new(tx, y0 + g.tab_top + h * 0.73), Align2::CENTER_CENTER, format!("{}", mb.den), f, INK);
                }
            }
            // Every voice with notes, on the same staff. Voices not being edited are grey.
            let multi = (1..VOICES).any(|v| bar.voice_used(v));
            let rhythm_voice = (0..VOICES).find(|&v| bar.voice_used(v)).unwrap_or(0);
            for v in 0..VOICES {
                let editing = blk.ti == dc.ti && v == dc.voice;
                if v > 0 && !bar.voice_used(v) && !editing {
                    continue;
                }
                if v == 0 && multi && !bar.voice_used(0) && !editing {
                    continue;
                }
                let vbar = Bar::new(bar.voice(v).clone());
                let (xs, ws) = if v == 0 { (bl.beats_x[k].clone(), bl.beats_w[k].clone()) } else { bl.voice_xs(&vbar.beats) };
                let grey = multi && blk.ti == dc.ti && v != dc.voice;
                set_ink(if grey { Color32::from_gray(150) } else { INK });
                let bctx = BarCtx {
                    song,
                    ti: blk.ti,
                    bi,
                    g,
                    y0,
                    o,
                    z,
                    xs: &xs,
                    ws: &ws,
                    music_font: dc.music_font,
                    lay,
                    block: k,
                    lyrics: &lyric_maps[k],
                    voice: v,
                    vbar: &vbar,
                    multi,
                    rhythm: v == rhythm_voice,
                    vbar_main_used: bar.voice_used(0),
                    lower: false,
                };
                if g.grand {
                    let (up, low) = draw::split_grand(track, &vbar, mb.key);
                    let upper = BarCtx { vbar: &up, ..bctx };
                    draw_bar_content(p, &upper);
                    let mut g2 = g;
                    g2.std_top = g.std2_top;
                    let lower = BarCtx { vbar: &low, g: g2, lower: true, ..upper };
                    draw::draw_lower_staff(p, &lower);
                } else {
                    draw_bar_content(p, &bctx);
                }
            }
            set_ink(INK);
        }
    }

    // The cursor: a yellow box in the staff being edited (notation or tab) and a
    // grey box on the same note in the other staff.
    if dc.show_cursor {
        if let Some(bl) = sys.bars.iter().find(|b| b.bar == dc.cursor.0) {
            let track = &song.tracks[dc.ti];
            let (cxs, _) = bl.voice_xs(track.bars[dc.cursor.0].voice(dc.voice));
            if let (Some(&bx), Some(beat)) = (cxs.get(dc.cursor.1), track.bars[dc.cursor.0].voice(dc.voice).get(dc.cursor.1)) {
                let key = song.master[dc.cursor.0].key;
                let x = o.x + bx;
                let std_y = |pos: i32| y0 + cg.pos_y(pos);
                let tab_c = |s: usize| Pos2::new(x, y0 + cg.string_y(s.min(cg.strings - 1)));
                const FILL: [(u8, u8, u8); 4] = [(255, 214, 110), (130, 215, 140), (240, 150, 200), (180, 150, 240)];
                const EDGE: [(u8, u8, u8); 4] = [(196, 140, 30), (50, 140, 70), (180, 60, 130), (110, 70, 190)];
                let (fr, fg, fb) = FILL[dc.voice.min(3)];
                let (er, eg, eb) = EDGE[dc.voice.min(3)];
                let yellow = |r: Rect| {
                    p.rect_filled(r, 3.0 * z, Color32::from_rgba_unmultiplied(fr, fg, fb, 150));
                    p.rect_stroke(r, 3.0 * z, PStroke::new(1.3, Color32::from_rgb(er, eg, eb)), egui::StrokeKind::Outside);
                };
                let grey = |r: Rect| {
                    p.rect_filled(r, 3.0 * z, Color32::from_rgba_unmultiplied(150, 150, 150, 70));
                    p.rect_stroke(r, 3.0 * z, PStroke::new(1.0, Color32::from_gray(120)), egui::StrokeKind::Outside);
                };
                let tab_box = |s: usize| Rect::from_center_size(tab_c(s), Vec2::new(15.0 * z, cg.ts * 0.95));
                let std_box = |pos: i32| Rect::from_center_size(Pos2::new(x, std_y(pos)), Vec2::new(14.0 * z, cg.ls * 1.25));
                match dc.cursor_std {
                    Some(pos) if cg.std => {
                        yellow(std_box(pos));
                        if cg.tab {
                            if let Some(n) = beat.notes.iter().find(|n| draw::note_staff_pos(track, beat, key, n) == pos) {
                                grey(tab_box(n.string as usize));
                            }
                        }
                    }
                    _ if cg.tab => {
                        yellow(tab_box(dc.cursor.2));
                        if cg.std {
                            if let Some(n) = beat.note_on(dc.cursor.2 as u8) {
                                grey(std_box(draw::note_staff_pos(track, beat, key, n)));
                            }
                        }
                    }
                    _ => {
                        yellow(std_box(4));
                    }
                }
            }
        }
    }
}

/// Finds which track, beat and string a click on the page hits.
pub fn hit_test(lay: &Layout, song: &Song, voice: usize, page: usize, rel: Pos2) -> Option<(usize, usize, usize, Option<Hit>)> {
    for sys in lay.systems.iter().filter(|s| s.page == page) {
        if rel.y < sys.y || rel.y > sys.y + lay.sys_h {
            continue;
        }
        let ry = rel.y - sys.y;
        let mut bk = lay.blocks.len() - 1;
        for (k, blk) in lay.blocks.iter().enumerate() {
            if ry <= blk.g.bottom {
                bk = k;
                break;
            }
        }
        let g = lay.blocks[bk].g;
        for bl in &sys.bars {
            if rel.x < bl.x || rel.x > bl.x + bl.w {
                continue;
            }
            let mut best = 0;
            let mut bd = f32::MAX;
            let (vxs, _) = bl.voice_xs(song.tracks[lay.blocks[bk].ti].bars[bl.bar].voice(voice));
            for (j, &bx) in vxs.iter().enumerate() {
                let d = (bx - rel.x).abs();
                if d < bd {
                    bd = d;
                    best = j;
                }
            }
            let mut hit = None;
            // Before the first beat of a bar showing a key or time signature.
            if let Some(&fx) = bl.beats_x[bk].first() {
                let w0 = bl.beats_w[bk].first().copied().unwrap_or(20.0);
                if rel.x < fx - w0 * 0.35 && (bl.time || bl.key_shown) {
                    let (t, b) = g.staff_span();
                    if ry >= t - 4.0 && ry <= b + 4.0 {
                        // Same order and widths as drawn: clef, key signature, repeat sign, time signature.
                        let z = lay.z;
                        let mb = &song.master[bl.bar];
                        let clef_end = bl.x + 4.0 * z + if bl.clef { 28.0 * z } else { 0.0 };
                        let key_end = clef_end + if bl.key_shown && mb.key != 0 { mb.key.unsigned_abs() as f32 * 9.0 * z + 6.0 * z } else { 0.0 };
                        let time_start = key_end + if mb.repeat_open { 10.0 * z } else { 0.0 };
                        let part = if rel.x >= clef_end && rel.x < key_end {
                            1
                        } else if bl.time && rel.x >= time_start {
                            2
                        } else if rel.x < clef_end {
                            0
                        } else if bl.key_shown {
                            1
                        } else {
                            2
                        };
                        return Some((lay.blocks[bk].ti, bl.bar, 0, Some(Hit::Head(part))));
                    }
                }
            }
            // A tempo change marking at the top of the first track.
            if bk == 0 && bl.bar > 0 && song.master[bl.bar].tempo.is_some() && ry < g.top + 24.0 * lay.z && rel.x >= bl.x + 60.0 * lay.z && rel.x <= bl.x + 150.0 * lay.z {
                return Some((lay.blocks[bk].ti, bl.bar, 0, Some(Hit::Tempo)));
            }
            let staff_top = g.staff_span().0;
            if ry < staff_top - 12.0 * lay.z {
                let zone = if bk == 0 && ry < g.top + 22.0 * lay.z { 0 } else { 1 };
                hit = Some(Hit::Above(zone));
            }
            if g.tab && hit.is_none() {
                let ty = ry - g.tab_top;
                if ty > -g.ts * 0.6 && ty < (g.strings as f32 - 0.4) * g.ts {
                    hit = Some(Hit::Tab(((ty / g.ts).round().max(0.0) as usize).min(g.strings - 1)));
                }
            }
            if g.std && hit.is_none() {
                // Lines and spaces of the notation staff, with a few ledger lines either side.
                let bottom = g.std_top + 4.0 * g.ls;
                if g.grand && ry > (bottom + g.std2_top) * 0.5 {
                    let pos = ((g.std2_top + 4.0 * g.ls - ry) / (g.ls * 0.5)).round() as i32 - 12;
                    if (-20..=-3).contains(&pos) && ry < g.lane_top {
                        hit = Some(Hit::Std(pos));
                    }
                } else {
                    let pos = ((bottom - ry) / (g.ls * 0.5)).round() as i32;
                    let pos = if g.grand { pos.max(-2) } else { pos };
                    if (-8..=16).contains(&pos) && ry < g.lane_top {
                        hit = Some(Hit::Std(pos));
                    }
                }
            }
            return Some((lay.blocks[bk].ti, bl.bar, best, hit));
        }
    }
    None
}

#[cfg(test)]
mod hit_tests {
    use super::*;

    #[test]
    fn key_and_time_signature_clicks() {
        let song = Song::demo();
        let lay = layout(&song, &[0, 1], 0, 1.0, Notation::Both);
        let sys = &lay.systems[0];
        let bl = &sys.bars[0];
        let g = lay.blocks[0].g;
        let y = sys.y + g.std_top + 2.0 * g.ls;
        let z = lay.z;
        // Just after the clef: the key signature (one sharp in the demo).
        let key_x = bl.x + 4.0 * z + 28.0 * z + 4.0 * z;
        assert_eq!(hit_test(&lay, &song, 0, sys.page, Pos2::new(key_x, y)).map(|h| h.3), Some(Some(Hit::Head(1))));
        // After the key signature and the repeat sign: the time signature.
        let time_x = bl.x + 4.0 * z + 28.0 * z + 15.0 * z + 10.0 * z + 8.0 * z;
        assert_eq!(hit_test(&lay, &song, 0, sys.page, Pos2::new(time_x, y)).map(|h| h.3), Some(Some(Hit::Head(2))));
    }
}
