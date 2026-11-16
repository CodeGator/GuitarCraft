//! `cargo xtask icons`: regenerate every app icon from `assets/app-icon/icon.svg`.
//! `cargo xtask ico <out.ico> <in.png>…`: pack PNG images into a Windows `.ico`.
//!
//! storytold's apps do this with `packaging/icons.sh` and the resvg command-line tool; here the
//! resvg crate renders the SVG, so nothing needs installing. The outputs are committed, so builds
//! and packaging never run this.
//!
//! Vista and later read PNG-compressed icon entries, so each PNG is stored in the `.ico` as is.
//! The size comes from the PNG's IHDR chunk; 256 px is written as 0, per the ICO format.

use std::path::Path;

/// The sizes packed into the Windows `.ico` (Explorer, taskbar, Start menu, Alt-Tab, at every scaling).
const ICO_SIZES: [u32; 9] = [16, 20, 24, 32, 40, 48, 64, 128, 256];

/// `svg` rendered as a `size`×`size` PNG.
fn render(svg: &[u8], size: u32) -> Result<Vec<u8>, String> {
    let tree = resvg::usvg::Tree::from_data(svg, &resvg::usvg::Options::default()).map_err(|e| format!("icon.svg: {e}"))?;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size, size).ok_or("bad icon size")?;
    let s = tree.size();
    let ts = resvg::tiny_skia::Transform::from_scale(size as f32 / s.width(), size as f32 / s.height());
    resvg::render(&tree, ts, &mut pixmap.as_mut());
    pixmap.encode_png().map_err(|e| e.to_string())
}

fn write(path: &Path, data: &[u8]) -> Result<(), String> {
    std::fs::write(path, data).map_err(|e| format!("write {}: {e}", path.display()))?;
    println!("{} ({} bytes)", path.display(), data.len());
    Ok(())
}

/// `cargo xtask icons`.
pub fn run(args: &[String]) -> Result<(), String> {
    if !args.is_empty() {
        return Err("usage: cargo xtask icons".into());
    }
    let dir = crate::root().join("assets").join("app-icon");
    let svg_path = dir.join("icon.svg");
    let svg = std::fs::read(&svg_path).map_err(|e| format!("read {}: {e}", svg_path.display()))?;
    // Large artwork for the README and stores; 256 px is the runtime window icon (main.rs).
    write(&dir.join("guitarcraft-1024.png"), &render(&svg, 1024)?)?;
    write(&dir.join("guitarcraft-256.png"), &render(&svg, 256)?)?;
    let pngs = ICO_SIZES.iter().map(|&s| render(&svg, s)).collect::<Result<Vec<_>, _>>()?;
    write(&dir.join("guitarcraft.ico"), &pack(&pngs)?)
}

/// Width and height from a PNG's IHDR chunk.
pub fn png_size(png: &[u8]) -> Result<(u32, u32), String> {
    const SIG: &[u8] = b"\x89PNG\r\n\x1a\n";
    if png.len() < 24 || &png[..8] != SIG || &png[12..16] != b"IHDR" {
        return Err("not a PNG file".into());
    }
    let w = u32::from_be_bytes([png[16], png[17], png[18], png[19]]);
    let h = u32::from_be_bytes([png[20], png[21], png[22], png[23]]);
    Ok((w, h))
}

/// An ICO file holding `pngs`, which must be square and at most 256 px.
pub fn pack(pngs: &[Vec<u8>]) -> Result<Vec<u8>, String> {
    if pngs.is_empty() || pngs.len() > u16::MAX as usize {
        return Err("need at least one PNG".into());
    }
    let mut out = Vec::new();
    out.extend_from_slice(&0u16.to_le_bytes()); // reserved
    out.extend_from_slice(&1u16.to_le_bytes()); // type: icon
    out.extend_from_slice(&(pngs.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * pngs.len();
    for png in pngs {
        let (w, h) = png_size(png)?;
        if w != h || w == 0 || w > 256 {
            return Err(format!("icon images must be square and at most 256 px, got {w}x{h}"));
        }
        let dim = if w == 256 { 0 } else { w as u8 };
        out.extend_from_slice(&[dim, dim, 0, 0]); // width, height, palette size, reserved
        out.extend_from_slice(&1u16.to_le_bytes()); // colour planes
        out.extend_from_slice(&32u16.to_le_bytes()); // bits per pixel
        out.extend_from_slice(&(png.len() as u32).to_le_bytes());
        out.extend_from_slice(&(offset as u32).to_le_bytes());
        offset += png.len();
    }
    for png in pngs {
        out.extend_from_slice(png);
    }
    Ok(out)
}

/// `cargo xtask ico <out.ico> <in.png>…`; `args` are the words after `ico`.
pub fn run_ico(args: &[String]) -> Result<(), String> {
    let [out, inputs @ ..] = args else {
        return Err("usage: cargo xtask ico <out.ico> <in.png>…".into());
    };
    let pngs = inputs.iter().map(|p| std::fs::read(p).map_err(|e| format!("read {p}: {e}"))).collect::<Result<Vec<_>, _>>()?;
    write(Path::new(out), &pack(&pngs)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_png(size: u32) -> Vec<u8> {
        let mut v = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR".to_vec();
        v.extend_from_slice(&size.to_be_bytes());
        v.extend_from_slice(&size.to_be_bytes());
        v.extend_from_slice(&[8, 6, 0, 0, 0, 1, 2, 3, 4]);
        v
    }

    #[test]
    fn packs_directory_and_payloads() {
        let a = fake_png(16);
        let b = fake_png(256);
        let ico = pack(&[a.clone(), b.clone()]).unwrap();
        assert_eq!(&ico[..6], &[0, 0, 1, 0, 2, 0]);
        // Entry 0: 16 px at offset 6 + 32.
        assert_eq!(&ico[6..8], &[16, 16]);
        assert_eq!(u32::from_le_bytes(ico[14..18].try_into().unwrap()), a.len() as u32);
        assert_eq!(u32::from_le_bytes(ico[18..22].try_into().unwrap()), 38);
        // Entry 1: 256 px stored as 0, right after the first payload.
        assert_eq!(&ico[22..24], &[0, 0]);
        assert_eq!(u32::from_le_bytes(ico[34..38].try_into().unwrap()), 38 + a.len() as u32);
        assert_eq!(&ico[38..38 + a.len()], &a[..]);
        assert_eq!(&ico[38 + a.len()..], &b[..]);
    }

    #[test]
    fn rejects_bad_input() {
        assert!(pack(&[]).is_err());
        assert!(pack(&[fake_png(512)]).is_err());
        assert!(pack(&[b"GIF89a".to_vec()]).is_err());
        assert_eq!(png_size(&fake_png(48)).unwrap(), (48, 48));
    }

    #[test]
    fn committed_icon_is_valid() {
        let path = crate::root().join("assets/app-icon/guitarcraft.ico");
        let ico = std::fs::read(&path).unwrap();
        assert_eq!(&ico[..4], &[0, 0, 1, 0]);
        let n = u16::from_le_bytes([ico[4], ico[5]]) as usize;
        assert_eq!(n, ICO_SIZES.len(), "expected every size in {}", path.display());
    }
}
