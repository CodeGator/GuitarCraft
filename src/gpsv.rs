//! The score views stored in a .gp file ("Content/ScoreViews/N.gpsv"): view 0
//! is the full score and lists only the tracks that are shown (a track hidden
//! with its eye icon is left out); views 1.. are each track on its own. They
//! are small protocol-buffer messages; the layout here follows files saved by
//! Guitar Pro 8. Each staff is named by (track, kind), kind 0 for standard
//! notation and 1 for tablature.

fn varint(out: &mut Vec<u8>, mut v: u64) {
    loop {
        let b = (v & 0x7f) as u8;
        v >>= 7;
        if v == 0 {
            out.push(b);
            return;
        }
        out.push(b | 0x80);
    }
}

fn num(out: &mut Vec<u8>, field: u64, v: u64) {
    varint(out, field << 3);
    varint(out, v);
}

fn bytes(out: &mut Vec<u8>, field: u64, b: &[u8]) {
    varint(out, (field << 3) | 2);
    varint(out, b.len() as u64);
    out.extend_from_slice(b);
}

/// A staff: track number and kind (both left out when 0).
fn staff_key(t: usize, s: usize) -> Vec<u8> {
    let mut k = vec![];
    if t > 0 {
        num(&mut k, 1, t as u64);
    }
    if s > 0 {
        num(&mut k, 3, s as u64);
    }
    k
}

/// A place in the score: a label, the staff, and the bar (left out when 0).
fn place(label: &[u8], t: usize, s: usize, bar: usize, extra_empty5: bool) -> Vec<u8> {
    let mut p = vec![];
    bytes(&mut p, 1, label);
    bytes(&mut p, 2, &staff_key(t, s));
    if bar > 0 {
        num(&mut p, 3, bar as u64);
    }
    if extra_empty5 {
        bytes(&mut p, 5, &[]);
    }
    p
}

/// Wraps a place twice in field 1 (as the files do).
fn wrap2(p: &[u8]) -> Vec<u8> {
    let mut a = vec![];
    bytes(&mut a, 1, p);
    let mut b = vec![];
    bytes(&mut b, 1, &a);
    b
}

/// One view: its name, the staves it shows (track, kind), the number of bars,
/// and how many bars go on each line of music.
pub fn view(name: &str, staves: &[(usize, usize)], nbars: usize, per_line: usize) -> Vec<u8> {
    let mut out = vec![];
    bytes(&mut out, 2, name.as_bytes());
    // One entry per bar.
    for b in 0..nbars {
        let mut m = vec![];
        if b > 0 {
            num(&mut m, 1, b as u64);
        }
        num(&mut m, 2, b as u64 + 1);
        bytes(&mut m, 6, &[]);
        bytes(&mut out, 3, &m);
    }
    if staves.is_empty() {
        return out;
    }
    let per_line = per_line.max(1);
    let (ft, fs) = staves[0];
    let (lt, ls) = *staves.last().unwrap();
    // Lines of music.
    let mut start = 0;
    while start < nbars {
        let n = per_line.min(nbars - start);
        let first_line = start == 0;
        let mut sys = vec![];
        let mut range = vec![];
        let mut a = vec![];
        bytes(&mut a, 2, &staff_key(ft, fs));
        if start > 0 {
            num(&mut a, 3, start as u64);
        }
        bytes(&mut range, 1, &a);
        let mut z = vec![];
        bytes(&mut z, 2, &staff_key(lt, ls));
        num(&mut z, 3, n as u64);
        bytes(&mut range, 1, &z);
        bytes(&mut sys, 1, &range);
        if start > 0 {
            num(&mut sys, 10, start as u64);
        }
        num(&mut sys, 11, n as u64);
        // Line header.
        let mut h = vec![];
        bytes(&mut h, 9, &[]);
        bytes(&mut h, 11, &[]);
        bytes(&mut h, 12, &[]);
        bytes(&mut sys, 13, &h);
        let empties = if first_line { 3 } else { 2 };
        for &(t, s) in staves {
            let mut st = vec![];
            bytes(&mut st, 9, &staff_key(t, s));
            if s == 0 {
                for b in start..start + n {
                    let mut e = wrap2(&place(b"50", t, s, b, false));
                    num(&mut e, 9, 50);
                    bytes(&mut st, 10, &e);
                }
            }
            for _ in 0..empties {
                bytes(&mut st, 10, &[]);
            }
            if s == 0 {
                let mut e = vec![];
                bytes(&mut e, 11, &[]);
                bytes(&mut e, 11, &[]);
                bytes(&mut st, 11, &e);
                let mut nine = vec![];
                bytes(&mut nine, 9, &[]);
                let mut f = vec![];
                bytes(&mut f, 11, &nine);
                bytes(&mut f, 11, &nine);
                bytes(&mut st, 12, &f);
            } else {
                bytes(&mut st, 11, &[]);
                let mut nine = vec![];
                bytes(&mut nine, 9, &[]);
                let mut f = vec![];
                bytes(&mut f, 11, &nine);
                bytes(&mut st, 12, &f);
            }
            bytes(&mut sys, 13, &st);
        }
        if first_line {
            bytes(&mut sys, 16, &[]);
        }
        bytes(&mut out, 4, &sys);
        start += n;
    }
    // Each staff, bar by bar.
    for &(t, s) in staves {
        let mut st = vec![];
        bytes(&mut st, 1, &staff_key(t, s));
        for b in 0..nbars {
            let mut e = wrap2(&place(b"0", t, s, b, false));
            if b == 0 {
                let mut m = wrap2(&place(b"0", t, s, 0, true));
                num(&mut m, 19, 1);
                bytes(&mut e, 13, &m);
                let mut inner = vec![];
                bytes(&mut inner, 16, &wrap2(&place(b"0", t, s, 0, true)));
                bytes(&mut e, if s == 0 { 16 } else { 17 }, &inner);
            }
            bytes(&mut st, 2, &e);
        }
        bytes(&mut out, 5, &st);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(name: &str) -> Option<Vec<u8>> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("samples")
            .join("Empty.gp");
        let data = std::fs::read(p).ok()?;
        let mut z = zip::ZipArchive::new(std::io::Cursor::new(data)).ok()?;
        let mut v = vec![];
        std::io::Read::read_to_end(&mut z.by_name(name).ok()?, &mut v).ok()?;
        Some(v)
    }

    #[test]
    fn matches_views_saved_by_guitar_pro() {
        // The sample: two tracks with notation and tab, ten bars, the bass track hidden.
        let Some(full) = sample("Content/ScoreViews/0.gpsv") else {
            return;
        };
        assert_eq!(view("Full score", &[(1, 0), (1, 1)], 10, 4), full);
        let bass = sample("Content/ScoreViews/1.gpsv").unwrap();
        assert_eq!(view("Electric Bass", &[(0, 0), (0, 1)], 10, 3), bass);
        let reso = sample("Content/ScoreViews/2.gpsv").unwrap();
        assert_eq!(view("Resonator", &[(1, 0), (1, 1)], 10, 3), reso);
    }
}
