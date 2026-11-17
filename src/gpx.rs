//! Guitar Pro 6 files (.gpx), for File > Export > GPX. A .gpx file is a small
//! file system in 4096-byte sectors ("BCFS") holding the score (the same
//! score.gpif XML a .gp file has) and its part settings, compressed with
//! Guitar Pro's own scheme ("BCFZ"). This writes the compressed form using only
//! literal runs (no back-references): plain to write, and read like any other.
//! The layout mirrors what `gpif::import_gpx` reads.

use crate::model::Song;

const SECTOR: usize = 0x1000;

/// The file system: a first sector, then for each file an entry sector
/// (type 2, name, size, list of data sectors) followed by its data sectors.
fn bcfs(files: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let mut fs = vec![0u8; SECTOR];
    for (name, data) in files {
        let entry_at = fs.len();
        fs.resize(entry_at + SECTOR, 0);
        let n_sectors = data.len().div_ceil(SECTOR).max(1);
        let first_data = entry_at / SECTOR + 1;
        let put = |fs: &mut Vec<u8>, at: usize, v: u32| fs[at..at + 4].copy_from_slice(&v.to_le_bytes());
        put(&mut fs, entry_at, 2);
        let name_bytes = name.as_bytes();
        fs[entry_at + 4..entry_at + 4 + name_bytes.len().min(126)].copy_from_slice(&name_bytes[..name_bytes.len().min(126)]);
        put(&mut fs, entry_at + 0x8C, data.len() as u32);
        let max_list = (SECTOR - 0x94) / 4 - 1;
        for k in 0..n_sectors.min(max_list) {
            put(&mut fs, entry_at + 0x94 + 4 * k, (first_data + k) as u32);
        }
        let mut body = data.clone();
        body.resize(n_sectors * SECTOR, 0);
        fs.extend_from_slice(&body);
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

/// The whole .gpx file.
pub fn export_gpx(song: &Song) -> Vec<u8> {
    let xml = crate::gpifwrite::gpif_xml_with(song, crate::gpifwrite::GpOptions::safe(song));
    let files = vec![("score.gpif", xml.into_bytes()), ("PartConfiguration", crate::gpifwrite::part_configuration(song))];
    let mut stream = b"BCFS".to_vec();
    stream.extend(bcfs(&files));
    bcfz(&stream)
}

#[cfg(test)]
mod tests {
    #[test]
    fn gpx_reads_back() {
        let song = crate::model::Song::demo();
        let data = super::export_gpx(&song);
        assert!(data.starts_with(b"BCFZ"));
        let back = crate::gpif::import_gpx(&data).unwrap();
        assert_eq!(back.title, song.title);
        assert_eq!(back.tracks.len(), song.tracks.len());
        assert_eq!(back.bar_count(), song.bar_count());
        assert_eq!(back.tracks[0].bars[0].beats[0].notes, song.tracks[0].bars[0].beats[0].notes);
    }
}
