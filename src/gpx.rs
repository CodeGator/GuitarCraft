//! Guitar Pro 6 files (.gpx), for File > Export > GPX. A .gpx file is a small
//! file system in 4096-byte sectors ("BCFS"), compressed with Guitar Pro's own
//! scheme ("BCFZ"). Laid out as in a .gpx file Guitar Pro 8 saved for the
//! person (tests/fixtures/gp8/Morning Walk - GP8.gpx):
//!
//! - sector 0: a map of the sectors in use, one bit per sector after it
//!   (lowest bit first; 0 = in use, 1 = free);
//! - sector 1: the root folder "/", whose data lists its files' entry sectors;
//! - for each file, an entry sector (type 2, name, size, its data sectors)
//!   followed by the data, the folder's list coming right after the first entry;
//! - the files: score.gpif, PartConfiguration, LayoutConfiguration,
//!   BinaryStylesheet and misc.xml, in that order.
//!
//! The compression here uses only literal runs (no back-references): plain to
//! write, and read like any other. The layout mirrors what `gpif::import_gpx` reads.

use crate::model::Song;

const SECTOR: usize = 0x1000;

fn put(s: &mut [u8], at: usize, v: u32) {
    s[at..at + 4].copy_from_slice(&v.to_le_bytes());
}

/// An entry sector: type (1 folder, 2 file), name, size and data sectors.
fn entry(name: &str, size: usize, sectors: &[usize], kind: u32) -> Vec<u8> {
    let mut e = vec![0u8; SECTOR];
    put(&mut e, 0, kind);
    let n = name.as_bytes();
    let len = n.len().min(126);
    e[4..4 + len].copy_from_slice(&n[..len]);
    put(&mut e, 0x88, 1);
    put(&mut e, 0x8C, size as u32);
    let max_list = (SECTOR - 0x94) / 4 - 1;
    for (k, s) in sectors.iter().take(max_list).enumerate() {
        put(&mut e, 0x94 + 4 * k, *s as u32);
    }
    e
}

/// The file system, without its "BCFS" mark.
fn bcfs(files: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let sectors_of = |len: usize| len.div_ceil(SECTOR).max(1);
    // Where everything goes: 0 map, 1 folder, then each file's entry and data
    // (the folder's list right after the first entry, as Guitar Pro does it).
    let mut next = 2;
    let mut entry_at = vec![];
    let mut data_at: Vec<Vec<usize>> = vec![];
    let mut folder_data = 0;
    for (k, (_, d)) in files.iter().enumerate() {
        entry_at.push(next);
        next += 1;
        if k == 0 {
            folder_data = next;
            next += 1;
        }
        let n = sectors_of(d.len());
        data_at.push((next..next + n).collect());
        next += n;
    }
    if files.is_empty() {
        folder_data = next;
        next += 1;
    }
    let total = next;
    let mut fs = vec![0u8; total * SECTOR];
    // The map of sectors in use: bit k stands for sector k + 1.
    for k in 0..SECTOR * 8 {
        if k + 1 >= total {
            fs[k / 8] |= 1 << (k % 8);
        }
    }
    let list: Vec<u8> = entry_at.iter().flat_map(|s| (*s as u32).to_le_bytes()).collect();
    fs[SECTOR..2 * SECTOR].copy_from_slice(&entry("/", list.len(), &[folder_data], 1));
    fs[folder_data * SECTOR..folder_data * SECTOR + list.len()].copy_from_slice(&list);
    for (k, (name, d)) in files.iter().enumerate() {
        let at = entry_at[k] * SECTOR;
        fs[at..at + SECTOR].copy_from_slice(&entry(name, d.len(), &data_at[k], 2));
        let start = data_at[k][0] * SECTOR;
        fs[start..start + d.len()].copy_from_slice(d);
    }
    fs
}

/// Bits written most significant first.
struct BitWriter {
    out: Vec<u8>,
    n: u32,
}

impl BitWriter {
    fn bit(&mut self, b: u32) {
        if self.n == 0 {
            self.out.push(0);
        }
        if b != 0 {
            let last = self.out.len() - 1;
            self.out[last] |= 1 << (7 - self.n);
        }
        self.n = (self.n + 1) % 8;
    }
    fn bits(&mut self, v: u32, n: u32) {
        for i in 0..n {
            self.bit((v >> (n - 1 - i)) & 1);
        }
    }
    fn bits_rev(&mut self, v: u32, n: u32) {
        for i in 0..n {
            self.bit((v >> i) & 1);
        }
    }
}

/// "BCFZ", the length when expanded, then runs of up to three literal bytes.
fn bcfz(data: &[u8]) -> Vec<u8> {
    let mut w = BitWriter { out: vec![], n: 0 };
    for chunk in data.chunks(3) {
        w.bit(0);
        w.bits_rev(chunk.len() as u32, 2);
        for &b in chunk {
            w.bits(b as u32, 8);
        }
    }
    let mut out = b"BCFZ".to_vec();
    out.extend_from_slice(&(data.len() as i32).to_le_bytes());
    out.extend(w.out);
    out
}

/// A .gpx file holding these files.
pub fn pack(files: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let mut stream = b"BCFS".to_vec();
    stream.extend(bcfs(files));
    bcfz(&stream)
}

/// The files a .gpx file holds, as Guitar Pro 8 writes them.
pub fn files(song: &Song) -> Vec<(&'static str, Vec<u8>)> {
    // Settings kept from a .gp file are for that format; a .gpx file is the older one.
    let mut older = song.clone();
    older.forget_kept();
    let song = &older;
    let xml = crate::gpifwrite::gpif_xml_with(song, crate::gpifwrite::GpOptions::safe(song));
    let misc = format!(
        "<MISC>\t<TracksView>\t\t<PreferredTrackCount>{}</PreferredTrackCount>\t</TracksView></MISC>",
        song.tracks.len().max(1)
    );
    vec![
        ("score.gpif", xml.into_bytes()),
        ("PartConfiguration", crate::gpifwrite::part_configuration(song)),
        ("LayoutConfiguration", crate::gpifwrite::LAYOUT_CONFIGURATION.to_vec()),
        ("BinaryStylesheet", crate::gpstyle::write(&song.style)),
        ("misc.xml", misc.into_bytes()),
    ]
}

/// The whole .gpx file.
pub fn export_gpx(song: &Song) -> Vec<u8> {
    pack(&files(song))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpx_reads_back() {
        let song = crate::model::Song::demo();
        let data = export_gpx(&song);
        assert!(data.starts_with(b"BCFZ"));
        let back = crate::gpif::import_gpx(&data).unwrap();
        assert_eq!(back.title, song.title);
        assert_eq!(back.tracks.len(), song.tracks.len());
        assert_eq!(back.bar_count(), song.bar_count());
        assert_eq!(back.tracks[0].bars[0].beats[0].notes, song.tracks[0].bars[0].beats[0].notes);
        // Every file comes back out, under the folder, as Guitar Pro lays them out.
        let names: Vec<String> = crate::gpif::gpx_files(&data).unwrap().into_iter().map(|f| f.0).collect();
        assert_eq!(names, ["score.gpif", "PartConfiguration", "LayoutConfiguration", "BinaryStylesheet", "misc.xml"]);
    }

    #[test]
    fn layout_matches_guitar_pro() {
        // Small files: the same sectors as in the person's Guitar Pro 8 file
        // (folder at 1, first entry at 2, the folder's list at 3, data from 4).
        let fs = bcfs(&[("a", vec![1; 10]), ("b", vec![2; 5000])]);
        let int = |at: usize| u32::from_le_bytes(fs[at..at + 4].try_into().unwrap());
        assert_eq!(fs.len(), 8 * SECTOR);
        assert_eq!(int(SECTOR), 1);
        assert_eq!(&fs[SECTOR + 4..SECTOR + 6], b"/\0");
        assert_eq!(int(SECTOR + 0x8C), 8);
        assert_eq!(int(SECTOR + 0x94), 3);
        assert_eq!((int(3 * SECTOR), int(3 * SECTOR + 4)), (2, 5));
        assert_eq!((int(2 * SECTOR), int(2 * SECTOR + 0x88), int(2 * SECTOR + 0x94)), (2, 1, 4));
        assert_eq!((int(5 * SECTOR + 0x94), int(5 * SECTOR + 0x98)), (6, 7));
        // The map: sectors 1 to 7 in use, the rest free.
        assert_eq!(&fs[..2], &[0x80, 0xff]);
    }
}
