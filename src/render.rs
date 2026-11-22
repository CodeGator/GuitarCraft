//! Draws score pages off screen at print resolution, for printing and for
//! PDF and PNG export. The pages are painted with the same code as on screen
//! (in an egui context of their own), turned into triangles by egui, and
//! filled in here with a small software rasterizer.

use crate::model::Song;
use crate::score::{self, Notation};
use egui::epaint::{ClippedPrimitive, Primitive};
use egui::{Color32, ColorImage, Pos2, Rect, TextureId, Vec2};
use std::collections::HashMap;

/// What to draw: which tracks the score shows, in which view.
pub struct PrintSpec<'a> {
    pub song: &'a Song,
    pub shown: Vec<usize>,
    pub track: usize,
    pub notation: Notation,
}

impl PrintSpec<'_> {
    fn layout(&self) -> score::Layout {
        score::layout_ex(self.song, &self.shown, self.track, 1.0, self.notation, None, false)
    }
    /// How many pages the score has.
    pub fn pages(&self) -> usize {
        self.layout().pages
    }
    /// The page size in millimetres.
    pub fn page_mm(&self) -> (f32, f32) {
        self.song.style.page_mm()
    }
    /// The pages that hold bars `a..=b`.
    pub fn pages_of_bars(&self, a: usize, b: usize) -> (usize, usize) {
        let lay = self.layout();
        let page = |bar: usize| lay.bar_pos.get(bar).and_then(|(s, _)| lay.systems.get(*s)).map(|s| s.page).unwrap_or(0);
        (page(a.min(b)), page(a.max(b)))
    }
}

/// A page as RGBA pixels (not premultiplied).
pub type PageImage = image::RgbaImage;

/// Renders the given pages (0-based) at `dpi` dots per inch.
pub fn render_pages(spec: &PrintSpec, pages: &[usize], dpi: f32, transparent: bool) -> Vec<PageImage> {
    let lay = spec.layout();
    let (w_mm, _) = spec.page_mm();
    let size = Vec2::new(lay.page_w(), lay.page_h());
    render_with(size, w_mm, pages, dpi, &mut |painter, page, music_font| {
        let dc = score::DrawCtx {
            song: spec.song,
            ti: spec.track,
            cursor: (usize::MAX, 0, 0),
            selection: None,
            play_tick: None,
            music_font,
            show_cursor: false,
            voice: 0,
            cursor_std: None,
            printing: true,
            transparent,
            play_cursor: crate::score::PlayCursor::Smooth,
            play_bar: true,
            bar_errors: 0,
 unreached: &[],
        };
        score::draw_page(painter, &lay, &dc, page, Pos2::ZERO);
    })
}

/// Renders pages (0-based) of a chord chart at `dpi` dots per inch.
pub fn render_chart(doc: &crate::chart::ChordChart, pages: &[usize], dpi: f32) -> Vec<PageImage> {
    use crate::chart;
    let size = Vec2::new(chart::PAGE_W, chart::PAGE_H);
    render_with(size, chart::PAGE_MM.0, pages, dpi, &mut |painter, page, _| {
        let lay = chart::layout(doc, &chart::painter_measure(painter));
        chart::draw_page(painter, doc, &lay, page, Pos2::ZERO, 1.0, None, true);
    })
}

/// The number of pages of a chord chart (measured with the real fonts).
pub fn chart_page_count(doc: &crate::chart::ChordChart) -> usize {
    use crate::chart;
    let mut n = 1;
    render_with(Vec2::new(chart::PAGE_W, chart::PAGE_H), chart::PAGE_MM.0, &[0], 24.0, &mut |painter, _, _| {
        n = chart::layout(doc, &chart::painter_measure(painter)).len().max(1);
    });
    n
}

/// Renders pages of any kind: `size` is a page in points at zoom 1, `w_mm`
/// its real width, and `draw` paints page `n` at the origin.
pub fn render_with(size: Vec2, w_mm: f32, pages: &[usize], dpi: f32, draw: &mut dyn FnMut(&egui::Painter, usize, bool)) -> Vec<PageImage> {
    let px_w = (w_mm / 25.4 * dpi).round().max(16.0);
    let ppp = px_w / size.x;
    let ctx = egui::Context::default();
    let music_font = crate::setup_fonts(&ctx);
    let mut textures: HashMap<TextureId, ColorImage> = HashMap::new();
    let input = || {
        let mut raw = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
            max_texture_side: Some(16384),
            ..Default::default()
        };
        raw.viewports.insert(
            egui::ViewportId::ROOT,
            egui::ViewportInfo { native_pixels_per_point: Some(ppp), ..Default::default() },
        );
        raw
    };
    // A first pass loads the fonts.
    ctx.begin_pass(input());
    let out = ctx.end_pass();
    apply_textures(&mut textures, out.textures_delta);
    let mut images = vec![];
    for &page in pages {
        ctx.begin_pass(input());
        let painter = ctx.layer_painter(egui::LayerId::background());
        draw(&painter, page, music_font);
        let out = ctx.end_pass();
        apply_textures(&mut textures, out.textures_delta);
        let ppp = out.pixels_per_point;
        let prims = ctx.tessellate(out.shapes, ppp);
        let w = (size.x * ppp).round() as usize;
        let h = (size.y * ppp).round() as usize;
        images.push(rasterize(&prims, ppp, w, h, &textures));
    }
    images
}

fn apply_textures(store: &mut HashMap<TextureId, ColorImage>, mut delta: egui::TexturesDelta) {
    for (id, ds) in std::mem::take(&mut delta.set) {
        for d in ds {
        let egui::ImageData::Color(img) = &d.image;
        match d.pos {
            None => {
                store.insert(id, (**img).clone());
            }
            Some([x0, y0]) => {
                if let Some(t) = store.get_mut(&id) {
                    for y in 0..img.size[1] {
                        for x in 0..img.size[0] {
                            let (tx, ty) = (x0 + x, y0 + y);
                            if tx < t.size[0] && ty < t.size[1] {
                                t.pixels[ty * t.size[0] + tx] = img.pixels[y * img.size[0] + x];
                            }
                        }
                    }
                }
            }
        }
        }
    }
    for id in std::mem::take(&mut delta.free) {
        store.remove(&id);
    }
}

/// Bilinear sample of a premultiplied texture.
fn sample(t: &ColorImage, u: f32, v: f32) -> [f32; 4] {
    let (w, h) = (t.size[0] as f32, t.size[1] as f32);
    let x = (u * w - 0.5).clamp(0.0, w - 1.0);
    let y = (v * h - 0.5).clamp(0.0, h - 1.0);
    let (x0, y0) = (x.floor() as usize, y.floor() as usize);
    let (x1, y1) = ((x0 + 1).min(t.size[0] - 1), (y0 + 1).min(t.size[1] - 1));
    let (fx, fy) = (x - x0 as f32, y - y0 as f32);
    let px = |x: usize, y: usize| {
        let c = t.pixels[y * t.size[0] + x];
        [c.r() as f32, c.g() as f32, c.b() as f32, c.a() as f32]
    };
    let (a, b, c, d) = (px(x0, y0), px(x1, y0), px(x0, y1), px(x1, y1));
    let mut o = [0.0; 4];
    for k in 0..4 {
        let top = a[k] + (b[k] - a[k]) * fx;
        let bot = c[k] + (d[k] - c[k]) * fx;
        o[k] = (top + (bot - top) * fy) / 255.0;
    }
    o
}

/// Fills egui's triangles into an image (premultiplied blending, as egui does).
fn rasterize(prims: &[ClippedPrimitive], ppp: f32, w: usize, h: usize, textures: &HashMap<TextureId, ColorImage>) -> PageImage {
    let mut buf = vec![[0.0f32; 4]; w * h];
    for cp in prims {
        let Primitive::Mesh(mesh) = &cp.primitive else { continue };
        let tex = textures.get(&mesh.texture_id);
        let cx0 = (cp.clip_rect.min.x * ppp).floor().max(0.0) as i64;
        let cy0 = (cp.clip_rect.min.y * ppp).floor().max(0.0) as i64;
        let cx1 = ((cp.clip_rect.max.x * ppp).ceil() as i64).min(w as i64);
        let cy1 = ((cp.clip_rect.max.y * ppp).ceil() as i64).min(h as i64);
        for tri in mesh.indices.as_chunks::<3>().0 {
            let v = [&mesh.vertices[tri[0] as usize], &mesh.vertices[tri[1] as usize], &mesh.vertices[tri[2] as usize]];
            let p = [v[0].pos * ppp, v[1].pos * ppp, v[2].pos * ppp];
            let edge = |a: Pos2, b: Pos2, x: f32, y: f32| (b.x - a.x) * (y - a.y) - (b.y - a.y) * (x - a.x);
            let area = edge(p[0], p[1], p[2].x, p[2].y);
            if area.abs() < 1e-9 {
                continue;
            }
            let minx = (p[0].x.min(p[1].x).min(p[2].x).floor() as i64).max(cx0);
            let maxx = (p[0].x.max(p[1].x).max(p[2].x).ceil() as i64).min(cx1);
            let miny = (p[0].y.min(p[1].y).min(p[2].y).floor() as i64).max(cy0);
            let maxy = (p[0].y.max(p[1].y).max(p[2].y).ceil() as i64).min(cy1);
            if minx >= maxx || miny >= maxy {
                continue;
            }
            let col = |c: Color32| [c.r() as f32 / 255.0, c.g() as f32 / 255.0, c.b() as f32 / 255.0, c.a() as f32 / 255.0];
            let c = [col(v[0].color), col(v[1].color), col(v[2].color)];
            for y in miny..maxy {
                let py = y as f32 + 0.5;
                for x in minx..maxx {
                    let px = x as f32 + 0.5;
                    let w0 = edge(p[1], p[2], px, py) / area;
                    let w1 = edge(p[2], p[0], px, py) / area;
                    let w2 = edge(p[0], p[1], px, py) / area;
                    if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                        continue;
                    }
                    let mut s = [0.0f32; 4];
                    for k in 0..4 {
                        s[k] = w0 * c[0][k] + w1 * c[1][k] + w2 * c[2][k];
                    }
                    if let Some(t) = tex {
                        let u = w0 * v[0].uv.x + w1 * v[1].uv.x + w2 * v[2].uv.x;
                        let vv = w0 * v[0].uv.y + w1 * v[1].uv.y + w2 * v[2].uv.y;
                        let ts = sample(t, u, vv);
                        for k in 0..4 {
                            s[k] *= ts[k];
                        }
                    }
                    if s[3] <= 0.0 && s[0] <= 0.0 && s[1] <= 0.0 && s[2] <= 0.0 {
                        continue;
                    }
                    let d = &mut buf[y as usize * w + x as usize];
                    let inv = 1.0 - s[3].clamp(0.0, 1.0);
                    for k in 0..4 {
                        d[k] = (s[k] + d[k] * inv).min(1.0);
                    }
                }
            }
        }
    }
    let mut img = image::RgbaImage::new(w as u32, h as u32);
    for (i, px) in img.pixels_mut().enumerate() {
        let [r, g, b, a] = buf[i];
        let un = |c: f32| if a > 0.0 { (c / a).clamp(0.0, 1.0) } else { 0.0 };
        *px = image::Rgba([(un(r) * 255.0).round() as u8, (un(g) * 255.0).round() as u8, (un(b) * 255.0).round() as u8, (a * 255.0).round() as u8]);
    }
    img
}

/// A PDF with one page per image, each the given size in millimetres. The
/// pages are pictures of the score (on white paper).
pub fn pdf(pages: &[PageImage], w_mm: f32, h_mm: f32, title: &str) -> Vec<u8> {
    use std::io::Write;
    let pt = |mm: f32| mm / 25.4 * 72.0;
    let (wp, hp) = (pt(w_mm), pt(h_mm));
    let mut out: Vec<u8> = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets: Vec<usize> = vec![];
    let n = pages.len();
    // Objects: 1 catalog, 2 page tree, 3 info, then per page: page, content, image.
    let page_obj = |k: usize| 4 + k * 3;
    let obj = |out: &mut Vec<u8>, offsets: &mut Vec<usize>, body: &[u8]| {
        offsets.push(out.len());
        let id = offsets.len();
        let _ = writeln!(out, "{id} 0 obj");
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    };
    obj(&mut out, &mut offsets, b"<< /Type /Catalog /Pages 2 0 R >>");
    let kids: Vec<String> = (0..n).map(|k| format!("{} 0 R", page_obj(k))).collect();
    obj(&mut out, &mut offsets, format!("<< /Type /Pages /Kids [{}] /Count {n} >>", kids.join(" ")).as_bytes());
    let safe: String = title.chars().filter(|c| c.is_ascii() && !matches!(c, '(' | ')' | '\\')).collect();
    obj(&mut out, &mut offsets, format!("<< /Title ({safe}) /Producer (GuitarCraft) >>").as_bytes());
    for (k, img) in pages.iter().enumerate() {
        let (iw, ih) = img.dimensions();
        obj(
            &mut out,
            &mut offsets,
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {wp:.2} {hp:.2}] /Resources << /XObject << /Im0 {} 0 R >> >> /Contents {} 0 R >>",
                page_obj(k) + 2,
                page_obj(k) + 1
            )
            .as_bytes(),
        );
        let content = format!("q {wp:.2} 0 0 {hp:.2} 0 0 cm /Im0 Do Q");
        obj(&mut out, &mut offsets, format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()).as_bytes());
        // RGB on white paper, compressed.
        let mut rgb = Vec::with_capacity((iw * ih * 3) as usize);
        for p in img.pixels() {
            let a = p[3] as u32;
            for c in 0..3 {
                rgb.push(((p[c] as u32 * a + 255 * (255 - a)) / 255) as u8);
            }
        }
        let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        let _ = z.write_all(&rgb);
        let data = z.finish().unwrap_or_default();
        let mut body = format!(
            "<< /Type /XObject /Subtype /Image /Width {iw} /Height {ih} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /FlateDecode /Length {} >>\nstream\n",
            data.len()
        )
        .into_bytes();
        body.extend_from_slice(&data);
        body.extend_from_slice(b"\nendstream");
        obj(&mut out, &mut offsets, &body);
    }
    let xref = out.len();
    let _ = write!(out, "xref\n0 {}\n0000000000 65535 f \n", offsets.len() + 1);
    for o in &offsets {
        let _ = writeln!(out, "{o:010} 00000 n ");
    }
    let _ = write!(out, "trailer\n<< /Size {} /Root 1 0 R /Info 3 0 R >>\nstartxref\n{xref}\n%%EOF\n", offsets.len() + 1);
    out
}

/// Pages stacked one under another in a single image (for PNG export).
pub fn stack(pages: &[PageImage]) -> PageImage {
    let w = pages.iter().map(|p| p.width()).max().unwrap_or(1);
    let h: u32 = pages.iter().map(|p| p.height()).sum::<u32>().max(1);
    let mut out = image::RgbaImage::new(w, h);
    let mut y = 0;
    for p in pages {
        image::imageops::replace(&mut out, p, 0, y as i64);
        y += p.height();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_and_writes_pdf() {
        let song = Song::demo();
        let spec = PrintSpec { song: &song, shown: vec![0, 1], track: 0, notation: Notation::Both };
        assert!(spec.pages() >= 1);
        let imgs = render_pages(&spec, &[0], 40.0, false);
        let img = &imgs[0];
        // A4 at 40 dpi is about 331 pixels wide.
        assert!((img.width() as i32 - 331).abs() <= 2);
        // White paper with dark ink on it.
        let dark = img.pixels().filter(|p| p[0] < 100 && p[3] > 200).count();
        let white = img.pixels().filter(|p| p[0] > 240 && p[3] > 240).count();
        assert!(dark > 200 && white > img.pixels().len() / 2);
        let data = pdf(&imgs, 210.0, 297.0, "Morning Walk");
        assert!(data.starts_with(b"%PDF-1.4"));
        assert!(data.ends_with(b"%%EOF\n"));
        // Transparent export leaves the paper out.
        let t = render_pages(&spec, &[0], 40.0, true);
        assert_eq!(t[0].get_pixel(2, 2)[3], 0);
    }
}

// ----- SVG

/// Pages as SVG drawings: the same drawing as on screen and in print, kept as
/// vector lines, shapes and text (in the score's fonts) instead of pixels.
pub fn svg_pages(spec: &PrintSpec, pages: &[usize]) -> Vec<String> {
    let lay = spec.layout();
    let (w_mm, h_mm) = spec.page_mm();
    let size = Vec2::new(lay.page_w(), lay.page_h());
    svg_with(size, (w_mm, h_mm), pages, &mut |painter, page, music_font| {
        let dc = score::DrawCtx {
            song: spec.song,
            ti: spec.track,
            cursor: (usize::MAX, 0, 0),
            selection: None,
            play_tick: None,
            music_font,
            show_cursor: false,
            voice: 0,
            cursor_std: None,
            printing: true,
            transparent: false,
            play_cursor: crate::score::PlayCursor::Smooth,
            play_bar: true,
            bar_errors: 0,
 unreached: &[],
        };
        score::draw_page(painter, &lay, &dc, page, Pos2::ZERO);
    })
}

/// Draws pages with `draw` and writes what was painted as SVG.
pub fn svg_with(size: Vec2, mm: (f32, f32), pages: &[usize], draw: &mut dyn FnMut(&egui::Painter, usize, bool)) -> Vec<String> {
    let ctx = egui::Context::default();
    let music_font = crate::setup_fonts(&ctx);
    let input = || egui::RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)), ..Default::default() };
    ctx.begin_pass(input());
    // No pixels are made here, so the font textures aren't needed.
    ctx.end_pass().textures_delta.clear();
    let mut out = vec![];
    for &page in pages {
        ctx.begin_pass(input());
        let painter = ctx.layer_painter(egui::LayerId::background());
        draw(&painter, page, music_font);
        let mut o = ctx.end_pass();
        o.textures_delta.clear();
        let mut s = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}mm\" height=\"{}mm\" viewBox=\"0 0 {} {}\">\n",
            mm.0, mm.1, size.x, size.y
        );
        for cs in o.shapes {
            svg_shape(&mut s, &cs.shape);
        }
        s.push_str("</svg>\n");
        out.push(s);
    }
    out
}

fn svg_color(c: Color32) -> String {
    let [r, g, b, a] = c.to_srgba_unmultiplied();
    if a == 255 { format!("#{r:02x}{g:02x}{b:02x}") } else { format!("rgba({r},{g},{b},{:.3})", a as f32 / 255.0) }
}

fn svg_paint(fill: Color32, stroke: Option<(f32, Color32)>) -> String {
    let mut s = if fill.a() == 0 { "fill=\"none\"".to_string() } else { format!("fill=\"{}\"", svg_color(fill)) };
    if let Some((w, c)) = stroke
        && w > 0.0
        && c.a() > 0
    {
        s.push_str(&format!(" stroke=\"{}\" stroke-width=\"{w:.2}\"", svg_color(c)));
    }
    s
}

fn path_stroke(p: &egui::epaint::PathStroke) -> Option<(f32, Color32)> {
    match p.color {
        egui::epaint::ColorMode::Solid(c) => Some((p.width, c)),
        _ => None,
    }
}

fn xml_text(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// A font as SVG font-family, weight and style.
fn svg_font(f: &egui::FontFamily) -> (String, &'static str, &'static str) {
    match f {
        egui::FontFamily::Name(n) => {
            let n: &str = n.as_ref();
            if let Some((fam, k)) = n.split_once('|') {
                let k: u8 = k.parse().unwrap_or(0);
                return (fam.to_string(), if k & 1 != 0 { "bold" } else { "normal" }, if k & 2 != 0 { "italic" } else { "normal" });
            }
            match n {
                "music" => ("Segoe UI Symbol".into(), "normal", "normal"),
                "bold" | "ui-bold" => ("Segoe UI".into(), "bold", "normal"),
                "ui-italic" => ("Segoe UI".into(), "normal", "italic"),
                "serif" => ("Times New Roman".into(), "normal", "normal"),
                "serif-bold" => ("Times New Roman".into(), "bold", "normal"),
                "serif-bi" => ("Times New Roman".into(), "bold", "italic"),
                other => (other.to_string(), "normal", "normal"),
            }
        }
        egui::FontFamily::Monospace => ("Consolas, monospace".into(), "normal", "normal"),
        egui::FontFamily::Proportional => ("Segoe UI, sans-serif".into(), "normal", "normal"),
    }
}

fn svg_shape(s: &mut String, shape: &egui::Shape) {
    use egui::Shape;
    match shape {
        Shape::Vec(v) => v.iter().for_each(|x| svg_shape(s, x)),
        Shape::LineSegment { points, stroke } => {
            if stroke.width > 0.0 && stroke.color.a() > 0 {
                s.push_str(&format!(
                    "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{}\" stroke-width=\"{:.2}\"/>\n",
                    points[0].x, points[0].y, points[1].x, points[1].y, svg_color(stroke.color), stroke.width
                ));
            }
        }
        Shape::Rect(r) => {
            let rad = (r.corner_radius.nw as f32 + r.corner_radius.ne as f32 + r.corner_radius.sw as f32 + r.corner_radius.se as f32) / 4.0;
            s.push_str(&format!(
                "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"{rad:.2}\" {}/>\n",
                r.rect.min.x,
                r.rect.min.y,
                r.rect.width(),
                r.rect.height(),
                svg_paint(r.fill, Some((r.stroke.width, r.stroke.color)))
            ));
        }
        Shape::Circle(c) => s.push_str(&format!(
            "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"{:.2}\" {}/>\n",
            c.center.x,
            c.center.y,
            c.radius,
            svg_paint(c.fill, Some((c.stroke.width, c.stroke.color)))
        )),
        Shape::Ellipse(e) => s.push_str(&format!(
            "<ellipse cx=\"{:.2}\" cy=\"{:.2}\" rx=\"{:.2}\" ry=\"{:.2}\" {}/>\n",
            e.center.x,
            e.center.y,
            e.radius.x,
            e.radius.y,
            svg_paint(e.fill, Some((e.stroke.width, e.stroke.color)))
        )),
        Shape::Path(p) => {
            if p.points.len() < 2 {
                return;
            }
            let pts: Vec<String> = p.points.iter().map(|q| format!("{:.2},{:.2}", q.x, q.y)).collect();
            let tag = if p.closed { "polygon" } else { "polyline" };
            let fill = if p.closed { p.fill } else { Color32::TRANSPARENT };
            s.push_str(&format!("<{tag} points=\"{}\" {}/>\n", pts.join(" "), svg_paint(fill, path_stroke(&p.stroke))));
        }
        Shape::QuadraticBezier(b) => {
            let [a, c, d] = b.points;
            let fill = if b.closed { b.fill } else { Color32::TRANSPARENT };
            s.push_str(&format!(
                "<path d=\"M{:.2},{:.2} Q{:.2},{:.2} {:.2},{:.2}{}\" {}/>\n",
                a.x,
                a.y,
                c.x,
                c.y,
                d.x,
                d.y,
                if b.closed { " Z" } else { "" },
                svg_paint(fill, path_stroke(&b.stroke))
            ));
        }
        Shape::CubicBezier(b) => {
            let [a, c1, c2, d] = b.points;
            let fill = if b.closed { b.fill } else { Color32::TRANSPARENT };
            s.push_str(&format!(
                "<path d=\"M{:.2},{:.2} C{:.2},{:.2} {:.2},{:.2} {:.2},{:.2}{}\" {}/>\n",
                a.x,
                a.y,
                c1.x,
                c1.y,
                c2.x,
                c2.y,
                d.x,
                d.y,
                if b.closed { " Z" } else { "" },
                svg_paint(fill, path_stroke(&b.stroke))
            ));
        }
        Shape::Text(t) => {
            let job = &t.galley.job;
            let Some(sec) = job.sections.first() else { return };
            let fmt = &sec.format;
            let color = t.override_text_color.unwrap_or(if fmt.color == Color32::PLACEHOLDER { t.fallback_color } else { fmt.color });
            let (family, weight, style) = svg_font(&fmt.font_id.family);
            for row in &t.galley.rows {
                let glyphs: Vec<_> = row.row.glyphs.iter().filter(|g| !g.chr.is_control()).collect();
                if glyphs.is_empty() || glyphs.iter().all(|g| g.chr.is_whitespace()) {
                    continue;
                }
                // Each character at its own place, as laid out.
                let xs: Vec<String> = glyphs.iter().map(|g| format!("{:.2}", t.pos.x + row.pos.x + g.pos.x)).collect();
                let y = t.pos.y + row.pos.y + glyphs[0].pos.y;
                let text: String = glyphs.iter().map(|g| g.chr).collect();
                s.push_str(&format!(
                    "<text x=\"{}\" y=\"{y:.2}\" font-family=\"{}\" font-size=\"{:.2}\" font-weight=\"{weight}\" font-style=\"{style}\" fill=\"{}\" xml:space=\"preserve\">{}</text>\n",
                    xs.join(" "),
                    xml_text(&family),
                    fmt.font_id.size,
                    svg_color(color),
                    xml_text(&text)
                ));
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod svg_tests {
    #[test]
    fn score_pages_become_svg() {
        let song = crate::model::Song::demo();
        let spec = super::PrintSpec { song: &song, shown: vec![0, 1], track: 0, notation: crate::score::Notation::Both };
        let pages = super::svg_pages(&spec, &[0]);
        let svg = &pages[0];
        assert!(svg.starts_with("<?xml") && svg.trim_end().ends_with("</svg>"));
        assert!(svg.contains("Morning Walk"), "the title is text");
        assert!(svg.matches("<line").count() > 20, "staff lines are lines");
        assert!(roxmltree::Document::parse(svg).is_ok(), "well-formed XML");
    }
}
