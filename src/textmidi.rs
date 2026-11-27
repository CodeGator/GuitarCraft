//! Plain-text (ASCII) tablature export and import, and MIDI file import.

use crate::model::*;

// ---------------------------------------------------------------- ASCII export

/// Symbols written after a fret number.
fn note_suffix(n: &Note) -> String {
    let mut c = String::new();
    if n.hammer {
        c.push('h');
    }
    if n.bend > 0 {
        c.push('b');
    }
    if n.slide || n.slide_in || n.slide_out {
        c.push('s');
    }
    if n.wide_vibrato {
        c.push('W');
    } else if n.vibrato {
        c.push('~');
    }
    if n.trill > 0 {
        c.push('t');
    }
    if n.accent || n.heavy_accent {
        c.push('>');
    }
    if n.staccato {
        c.push('.');
    }
    c
}

/// Symbols for the whole beat, written on a line above the tab.
fn beat_marks(b: &Beat) -> String {
    let mut c = String::new();
    match b.stroke {
        Stroke::Down => c.push('^'),
        Stroke::Up => c.push('v'),
        Stroke::None => {}
    }
    match b.pick {
        1 => c.push('n'),
        2 => c.push('V'),
        _ => {}
    }
    match b.technique {
        1 => c.push('+'),
        2 => c.push('S'),
        3 => c.push('P'),
        _ => {}
    }
    if b.notes.iter().any(|n| n.palm_mute) {
        c.push('M');
    }
    if b.tremolo {
        c.push('=');
    }
    if b.fade_in {
        c.push('<');
    }
    c
}

/// Plain-text tablature of one track.
pub fn export_ascii(song: &Song, ti: usize) -> String {
    let tr = &song.tracks[ti];
    let mut out = String::new();
    out.push_str(&format!("{}\n", song.title));
    if !song.artist.is_empty() {
        out.push_str(&format!("{}\n", song.artist));
    }
    out.push_str(&format!(
        "Track: {}   Tuning: {}   Tempo: {}\n\n",
        tr.name,
        tuning_name(&tr.tuning),
        song.tempo
    ));
    let n = tr.strings();
    let per_line = 4;
    for chunk_start in (0..song.bar_count()).step_by(per_line) {
        let mut marks = String::from("   ");
        let mut lines: Vec<String> = (0..n)
            .map(|s| format!("{:<2}|", note_name(tr.tuning[s])))
            .collect();
        for bi in chunk_start..(chunk_start + per_line).min(song.bar_count()) {
            for beat in &tr.played_bar(bi).beats {
                let cells: Vec<String> = (0..n)
                    .map(|s| match beat.note_on(s as u8) {
                        Some(nn) if nn.tie => "L".to_string(),
                        Some(nn) if nn.dead => format!("x{}", note_suffix(nn)),
                        Some(nn) if nn.ghost => format!("g{}{}", nn.fret, note_suffix(nn)),
                        Some(nn) => format!("{}{}", nn.fret, note_suffix(nn)),
                        None => String::new(),
                    })
                    .collect();
                let base = match beat.dur {
                    Dur::Whole => 8,
                    Dur::Half => 6,
                    Dur::Quarter => 4,
                    _ => 3,
                };
                let bm = beat_marks(beat);
                let w = cells
                    .iter()
                    .map(|c| c.len() + 1)
                    .max()
                    .unwrap_or(0)
                    .max(base)
                    .max(bm.len() + 1);
                marks.push_str(&bm);
                for _ in bm.len()..w {
                    marks.push(' ');
                }
                for (line, cell) in lines.iter_mut().zip(cells) {
                    line.push_str(&cell);
                    for _ in cell.len()..w {
                        line.push('-');
                    }
                }
            }
            marks.push(' ');
            for line in lines.iter_mut() {
                line.push('|');
            }
        }
        if !marks.trim().is_empty() {
            out.push_str(marks.trim_end());
            out.push('\n');
        }
        for l in lines {
            out.push_str(&l);
            out.push('\n');
        }
        out.push('\n');
    }
    out
}

// ---------------------------------------------------------------- shared helpers

/// Splits a length in ticks into standard (possibly dotted) note values.
fn split_ticks(mut t: u32) -> Vec<(Dur, bool)> {
    let mut cand: Vec<(u32, Dur, bool)> = vec![];
    for d in Dur::ALL {
        cand.push((d.ticks(), d, false));
        if d != Dur::SixtyFourth {
            cand.push((d.ticks() * 3 / 2, d, true));
        }
    }
    cand.sort_by_key(|c| std::cmp::Reverse(c.0));
    let mut out = vec![];
    while t >= 60 {
        match cand.iter().find(|c| c.0 <= t) {
            Some(&(len, d, dot)) => {
                out.push((d, dot));
                t -= len;
            }
            None => break,
        }
    }
    out
}

/// Chooses strings and frets for a set of pitches on a track.
pub(crate) fn place_pitches(track: &Track, pitches: &[u8]) -> Vec<Note> {
    let maxf = track.frets as i32;
    let mut used = vec![false; track.strings()];
    let mut out = vec![];
    let mut ps: Vec<i32> = pitches.iter().map(|&p| p as i32).collect();
    ps.sort_by(|a, b| b.cmp(a));
    ps.dedup();
    let lowest = track.tuning.iter().map(|&x| x as i32).min().unwrap_or(40) + track.capo as i32;
    let highest =
        track.tuning.iter().map(|&x| x as i32).max().unwrap_or(64) + maxf + track.capo as i32;
    for mut p in ps {
        while p < lowest {
            p += 12;
        }
        while p > highest {
            p -= 12;
        }
        let mut best: Option<(usize, i32)> = None;
        for (s, &open) in track.tuning.iter().enumerate() {
            let f = p - open as i32 - track.capo as i32;
            if used[s] || f < 0 || f > maxf {
                continue;
            }
            if best.map(|(_, bf)| f < bf).unwrap_or(true) {
                best = Some((s, f));
            }
        }
        if let Some((s, f)) = best {
            used[s] = true;
            out.push(Note::new(s as u8, f as u8));
        }
    }
    out.sort_by_key(|n| n.string);
    out
}

// ---------------------------------------------------------------- ASCII import

fn is_tab_line(l: &str) -> bool {
    let t = l.trim();
    if t.len() < 6 {
        return false;
    }
    let body = match t.find('|') {
        Some(i) if i <= 4 => &t[i..],
        _ => match t.find('-') {
            Some(i) if i <= 3 => &t[i..],
            _ => return false,
        },
    };
    let dashes = body.chars().filter(|&c| c == '-').count();
    let allowed = body
        .chars()
        .all(|c| c.is_ascii_digit() || "-|hpbrs/\\~xXtWLMg<>().=^vnVSP+*".contains(c) || c == ' ');
    allowed && dashes * 3 >= body.len()
}

fn label_pitch(label: &str) -> Option<i32> {
    let mut ch = label.trim().chars();
    let letter = ch.next()?.to_ascii_uppercase();
    let mut pc = match letter {
        'C' => 0,
        'D' => 2,
        'E' => 4,
        'F' => 5,
        'G' => 7,
        'A' => 9,
        'B' => 11,
        _ => return None,
    };
    match ch.next() {
        Some('#') => pc += 1,
        Some('b') => pc -= 1,
        _ => {}
    }
    Some((pc + 12) % 12)
}

// Tab copied from mail, forums and word processors arrives damaged in a few ordinary ways:
// quoted with ">", its dashes turned into typographic ones, spaces typed where dashes belong, and
// blank lines let in between the strings. These are put right before it is read.

/// A line with quoting marks, tabs and look-alike characters put right. `em` is what a long dash
/// stands for: word processors make one of two typed hyphens, or of one.
fn plain_line(line: &str, em: &str) -> String {
    let mut text = line.trim_end();
    // Quoted mail and forum text: "> e|---" and ">> e|---".
    while let Some(rest) = text.trim_start().strip_prefix('>') {
        text = rest;
    }
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\u{2014}' | '\u{2015}' => out.push_str(em),
            '\u{2010}'..='\u{2013}' | '\u{2212}' | '\u{2500}' | '\u{2501}' | '\u{FE58}' | '\u{FE63}' | '\u{FF0D}' => out.push('-'),
            '\u{2502}' | '\u{2503}' | '\u{00A6}' | '\u{FF5C}' => out.push('|'),
            '\u{00A0}' | '\t' => out.push(' '),
            _ => out.push(c),
        }
    }
    dashes_for_spaces(&out).unwrap_or(out)
}

/// A string's line typed with spaces where the dashes belong, as "e|   3    5  |", with the
/// dashes put in. None for anything else: it takes a string's name (or nothing), bar lines at
/// both ends and only tab signs between them.
fn dashes_for_spaces(line: &str) -> Option<String> {
    let t = line.trim();
    let open = t.find('|')?;
    let (label, body) = t.split_at(open);
    let label = label.trim();
    let named = label.is_empty() || (label.len() <= 2 && label_pitch(label).is_some());
    let signs = |c: char| c.is_ascii_digit() || " -|hpbrs/\\~xXtWLg<>().^=".contains(c);
    if !named || body.len() < 6 || !body.ends_with('|') || !body.contains(' ') || !body.chars().all(signs) {
        return None;
    }
    Some(format!("{label}{}", body.replace(' ', "-")))
}

/// A line of nothing but dashes, with no string name or bar line: a rule between parts.
fn is_rule(line: &str) -> bool {
    line.trim().chars().all(|c| c == '-')
}

/// The lines of each system of tab: the strings of one line of music. Blank lines within a system
/// are passed over; any other text ends it.
fn tab_systems(lines: &[String]) -> Vec<Vec<usize>> {
    const STRINGS: std::ops::RangeInclusive<usize> = 4..=8;
    let mut systems = vec![];
    // Runs of tab lines with nothing but blank lines between them.
    let mut runs: Vec<Vec<usize>> = vec![];
    let mut run: Vec<usize> = vec![];
    for (i, line) in lines.iter().map(String::as_str).chain([".end"]).enumerate() {
        if i < lines.len() && is_tab_line(line) {
            run.push(i);
            continue;
        }
        if !run.is_empty() {
            let rules = run.iter().all(|&k| is_rule(&lines[k]));
            let taken = std::mem::take(&mut run);
            if !rules {
                runs.push(taken);
            }
        }
        if i < lines.len() && line.trim().is_empty() {
            continue;
        }
        // The end of a stretch of tab.
        let group = std::mem::take(&mut runs);
        if group.iter().all(|r| STRINGS.contains(&r.len())) {
            systems.extend(group);
        } else {
            // Blank lines came between the strings, or systems with nothing between them.
            let all: Vec<usize> = group.concat();
            let each = match all.len() {
                n if n % 6 == 0 => 6,
                n if n % 4 == 0 => 4,
                n if STRINGS.contains(&n) => n,
                _ => 6,
            };
            systems.extend(all.chunks(each).filter(|c| STRINGS.contains(&c.len())).map(<[_]>::to_vec));
        }
    }
    systems
}

/// The systems of tab in a text, each as its lines, with the ordinary damage put right.
fn tab_blocks(text: &str) -> Vec<Vec<String>> {
    let read = |em: &str| text.lines().map(|l| plain_line(l, em)).collect::<Vec<_>>();
    let (one, two) = (read("-"), read("--"));
    // How much the lines of a system differ in length: the right reading of a long dash lines
    // them up.
    let ragged = |lines: &[String], system: &[usize]| {
        let lengths = || system.iter().map(|&i| lines[i].trim().len());
        lengths().max().unwrap_or(0) - lengths().min().unwrap_or(0)
    };
    tab_systems(&one)
        .into_iter()
        .map(|system| {
            let lines = if ragged(&two, &system) < ragged(&one, &system) { &two } else { &one };
            system.iter().map(|&i| lines[i].clone()).collect()
        })
        .collect()
}

/// Reads plain-text tablature. Returns a track and the bars it fills.
pub fn import_ascii(text: &str) -> Result<Track, String> {
    let blocks = tab_blocks(text);
    if blocks.is_empty() {
        return Err("No tablature was found in this text file.".into());
    }
    let nstr = blocks[0].len();
    let mut track = Track {
        tuning: tuning_presets(nstr)[0].1.clone(),
        ..Track::default()
    };
    if nstr <= 5 && track.tuning.iter().all(|&t| t < 50) {
        track.instrument = Instrument::Bass;
        track.name = "Bass".into();
    } else {
        track.name = "Guitar".into();
    }
    // Labels like "e|" or "D|" can change the tuning.
    for (s, l) in blocks[0].iter().enumerate() {
        let t = l.trim();
        if let Some(i) = t.find(['|', '-'])
            && let Some(pc) = label_pitch(&t[..i])
            && let Some(def) = track.tuning.get(s).copied()
        {
            let def = def as i32;
            let cands = [def - 12, def, def + 12];
            let mut best = def;
            let mut bd = 99;
            for c in cands {
                let base = c - (c.rem_euclid(12)) + pc;
                for x in [base - 12, base, base + 12] {
                    if (x - def).abs() < bd {
                        bd = (x - def).abs();
                        best = x;
                    }
                }
            }
            track.tuning[s] = best.clamp(20, 90) as u8;
        }
    }
    let mut bars: Vec<Bar> = vec![];
    for block in blocks.iter().filter(|b| b.len() == nstr) {
        // Strip labels up to the first bar line.
        let rows: Vec<Vec<char>> = block
            .iter()
            .map(|l| {
                let t = l.trim();
                let start = t
                    .find('|')
                    .filter(|&i| i <= 4)
                    .map(|i| i + 1)
                    .unwrap_or_else(|| t.find('-').unwrap_or(0));
                t[start..].chars().collect()
            })
            .collect();
        let width = rows.iter().map(|r| r.len()).max().unwrap_or(0);
        let rows: Vec<Vec<char>> = rows
            .into_iter()
            .map(|mut r| {
                r.resize(width, '-');
                r
            })
            .collect();
        // Bar lines are columns where most strings have '|'.
        let mut cuts = vec![0usize];
        for c in 0..width {
            if rows.iter().filter(|r| r[c] == '|').count() * 2 > nstr {
                cuts.push(c);
            }
        }
        cuts.push(width);
        for w in cuts.windows(2) {
            let (a, b) = (if w[0] == 0 { 0 } else { w[0] + 1 }, w[1]);
            if b <= a + 1 {
                continue;
            }
            // Events: columns where a fret number or x starts.
            let mut events: Vec<(usize, Vec<Note>)> = vec![];
            for c in a..b {
                let mut notes = vec![];
                for (s, r) in rows.iter().enumerate() {
                    let ch = r[c];
                    let prev = if c > a { r[c - 1] } else { '-' };
                    if ch.is_ascii_digit() && !prev.is_ascii_digit() {
                        let mut f = ch.to_digit(10).unwrap();
                        let mut k = c + 1;
                        if k < b
                            && r[k].is_ascii_digit()
                            && f * 10 + r[k].to_digit(10).unwrap() <= 24
                        {
                            f = f * 10 + r[k].to_digit(10).unwrap();
                            k += 1;
                        }
                        let mut n = Note::new(s as u8, f as u8);
                        while k < b && !r[k].is_ascii_digit() && r[k] != '-' && r[k] != '|' {
                            match r[k] {
                                'h' | 'p' => n.hammer = true,
                                'b' | 'r' => n.bend = 4,
                                '/' | '\\' | 's' => n.slide = true,
                                '~' => n.vibrato = true,
                                'W' => n.wide_vibrato = true,
                                't' => n.trill = (n.fret + 2).min(24),
                                '>' => n.accent = true,
                                '.' => n.staccato = true,
                                _ => {}
                            }
                            k += 1;
                        }
                        notes.push(n);
                    } else if (ch == 'x' || ch == 'X') && prev != 'x' {
                        let mut n = Note::new(s as u8, 0);
                        n.dead = true;
                        notes.push(n);
                    }
                }
                if !notes.is_empty() {
                    events.push((c, notes));
                }
            }
            let bar_ticks = QUARTER * 4;
            let span_w = (b - a).max(1) as f32;
            let mut beats = vec![];
            if events.is_empty() {
                bars.push(Bar::empty());
                continue;
            }
            // Leading rest.
            let lead = ((events[0].0 - a) as f32 / span_w * bar_ticks as f32) as u32;
            if lead >= 480 {
                for (d, dot) in split_ticks(lead) {
                    let mut r = Beat::rest(d);
                    r.dotted = dot;
                    beats.push(r);
                }
            }
            for (k, (c, notes)) in events.iter().enumerate() {
                let next = events.get(k + 1).map(|e| e.0).unwrap_or(b);
                let ticks = ((next - c) as f32 / span_w * bar_ticks as f32) as u32;
                let (d, dot) = nearest_value(ticks);
                let mut beat = Beat::rest(d);
                beat.dotted = dot;
                beat.notes = notes.clone();
                beats.push(beat);
            }
            bars.push(Bar::new(beats));
        }
    }
    if bars.is_empty() {
        return Err("No bars were found in this text file.".into());
    }
    track.bars = bars;
    Ok(track)
}

/// The standard note value closest to a length in ticks.
fn nearest_value(t: u32) -> (Dur, bool) {
    let mut best = (Dur::Quarter, false);
    let mut bd = f32::MAX;
    for d in Dur::ALL {
        for dot in [false, true] {
            let len = if dot { d.ticks() * 3 / 2 } else { d.ticks() } as f32;
            let diff = ((t.max(1) as f32) / len).ln().abs();
            if diff < bd {
                bd = diff;
                best = (d, dot);
            }
        }
    }
    best
}

// ---------------------------------------------------------------- MIDI import

struct MidiNote {
    start: u32,
    end: u32,
    pitch: u8,
    vel: u8,
}

fn read_vlq(d: &[u8], p: &mut usize) -> Option<u32> {
    let mut v = 0u32;
    for _ in 0..4 {
        let b = *d.get(*p)?;
        *p += 1;
        v = (v << 7) | (b & 0x7f) as u32;
        if b & 0x80 == 0 {
            return Some(v);
        }
    }
    Some(v)
}

/// Reads a standard MIDI file (format 0 or 1) into a song of guitar and bass tracks.
pub fn import_midi(data: &[u8], title: &str) -> Result<Song, String> {
    let bad = || "This doesn't look like a MIDI file.".to_string();
    if data.len() < 14 || &data[0..4] != b"MThd" {
        return Err(bad());
    }
    let ntracks = u16::from_be_bytes([data[10], data[11]]) as usize;
    let division = i16::from_be_bytes([data[12], data[13]]);
    if division <= 0 {
        return Err("MIDI files timed in SMPTE frames aren't supported.".into());
    }
    let div = division as u32;
    let to_ticks = |t: u32| (t as u64 * QUARTER as u64 / div as u64) as u32;
    let mut p = 8 + u32::from_be_bytes([data[4], data[5], data[6], data[7]]) as usize;
    let mut tempos: Vec<(u32, u32)> = vec![];
    let mut sigs: Vec<(u32, u8, u8)> = vec![];
    let mut keys: Vec<(u32, i8, bool)> = vec![];
    let mut markers: Vec<(u32, String)> = vec![];
    // The first track's name, which is the song's title when that track has no notes.
    let mut first_name = String::new();
    // (track index, channel) -> notes, program, name
    let mut parts: std::collections::BTreeMap<(usize, u8), (Vec<MidiNote>, u8, String)> =
        Default::default();
    for ti in 0..ntracks {
        if p + 8 > data.len() || &data[p..p + 4] != b"MTrk" {
            break;
        }
        let len = u32::from_be_bytes([data[p + 4], data[p + 5], data[p + 6], data[p + 7]]) as usize;
        let mut q = p + 8;
        let end = (q + len).min(data.len());
        p = q + len;
        let mut t = 0u32;
        let mut status = 0u8;
        let mut name = String::new();
        let mut programs = [0u8; 16];
        let mut open: std::collections::HashMap<(u8, u8), (u32, u8)> = Default::default();
        while q < end {
            let Some(dt) = read_vlq(data, &mut q) else {
                break;
            };
            t += dt;
            let Some(&b) = data.get(q) else { break };
            if b == 0xff {
                let kind = *data.get(q + 1).unwrap_or(&0);
                q += 2;
                let Some(l) = read_vlq(data, &mut q) else {
                    break;
                };
                let body = &data[q.min(end)..(q + l as usize).min(end)];
                match kind {
                    0x03 => name = String::from_utf8_lossy(body).trim().to_string(),
                    0x51 if body.len() == 3 => {
                        let us =
                            ((body[0] as u32) << 16) | ((body[1] as u32) << 8) | body[2] as u32;
                        if us > 0 {
                            tempos.push((to_ticks(t), (60_000_000.0 / us as f32).round() as u32));
                        }
                    }
                    0x58 if body.len() >= 2 => {
                        sigs.push((to_ticks(t), body[0].max(1), 1u8 << body[1].min(5)))
                    }
                    0x59 if !body.is_empty() => {
                        keys.push((to_ticks(t), body[0] as i8, body.get(1) == Some(&1)))
                    }
                    0x06 => markers.push((
                        to_ticks(t),
                        String::from_utf8_lossy(body).trim().to_string(),
                    )),
                    _ => {}
                }
                q += l as usize;
                continue;
            }
            if b == 0xf0 || b == 0xf7 {
                q += 1;
                let Some(l) = read_vlq(data, &mut q) else {
                    break;
                };
                q += l as usize;
                continue;
            }
            let (st, first) = if b & 0x80 != 0 {
                status = b;
                q += 1;
                (b, None)
            } else {
                (status, Some(b))
            };
            let arg = |q: &mut usize| -> u8 {
                let v = data.get(*q).copied().unwrap_or(0);
                *q += 1;
                v
            };
            let ch = st & 0x0f;
            match st & 0xf0 {
                0x80 | 0x90 => {
                    let key = first.unwrap_or_else(|| arg(&mut q));
                    let vel = arg(&mut q);
                    if st & 0xf0 == 0x90 && vel > 0 {
                        open.insert((ch, key), (to_ticks(t), vel));
                    } else if let Some((s0, v)) = open.remove(&(ch, key)) {
                        let e = parts
                            .entry((ti, ch))
                            .or_insert_with(|| (vec![], programs[ch as usize], name.clone()));
                        e.0.push(MidiNote {
                            start: s0,
                            end: to_ticks(t).max(s0 + 1),
                            pitch: key,
                            vel: v,
                        });
                    }
                }
                0xA0 | 0xB0 | 0xE0 => {
                    if first.is_none() {
                        arg(&mut q);
                    }
                    arg(&mut q);
                }
                0xC0 => {
                    let prog = first.unwrap_or_else(|| arg(&mut q));
                    programs[ch as usize] = prog;
                    if let Some(e) = parts.get_mut(&(ti, ch)) {
                        e.1 = prog;
                    }
                }
                0xD0 => {
                    if first.is_none() {
                        arg(&mut q);
                    }
                }
                _ => break,
            }
        }
        if ti == 0 {
            first_name = name.clone();
        }
        for e in parts.iter_mut().filter(|((t2, _), _)| *t2 == ti) {
            if e.1.2.is_empty() {
                e.1.2 = name.clone();
            }
        }
    }
    parts.retain(|_, v| !v.0.is_empty());
    if parts.is_empty() {
        return Err("This MIDI file has no notes.".into());
    }
    let last = parts
        .values()
        .flat_map(|v| v.0.iter().map(|n| n.end))
        .max()
        .unwrap_or(0);
    // Master bars from time signatures.
    sigs.sort_by_key(|s| s.0);
    let mut master = vec![];
    let mut t = 0u32;
    let (mut num, mut den) = (4u8, 4u8);
    while t < last.max(1) && master.len() < 2000 {
        for s in sigs.iter().filter(|s| s.0 <= t) {
            num = s.1;
            den = s.2;
        }
        let mut mb = MasterBar {
            num,
            den,
            ..MasterBar::default()
        };
        if let Some(k) = keys.iter().rev().find(|k| k.0 <= t) {
            mb.key = k.1.clamp(-7, 7);
            mb.key_minor = k.2;
        }
        t += mb.ticks();
        master.push(mb);
    }
    let mut song = Song::new_blank();
    let conductor = !first_name.is_empty() && !parts.keys().any(|&(ti, _)| ti == 0);
    song.title = if conductor {
        first_name
    } else {
        title.to_string()
    };
    tempos.sort_by_key(|x| x.0);
    song.tempo = tempos.first().map(|x| x.1).unwrap_or(120).clamp(20, 400);
    let mut bar_starts = vec![];
    let mut acc = 0u32;
    for m in &master {
        bar_starts.push(acc);
        acc += m.ticks();
    }
    for &(tt, bpm) in tempos.iter().skip(1) {
        if let Some(bi) = bar_starts.iter().rposition(|&s| s <= tt)
            && bi > 0
        {
            master[bi].tempo = Some(bpm.clamp(20, 400));
        }
    }
    // Section markers go on the bar they fall in; the first one there wins.
    for (tt, text) in markers {
        if let Some(bi) = bar_starts.iter().rposition(|&s| s <= tt)
            && !text.is_empty()
            && master[bi].marker.is_none()
        {
            master[bi].marker = Some(text);
        }
    }
    song.master = master;
    song.tracks.clear();
    const GRID: u32 = QUARTER / 4;
    for (k, ((_, ch), (notes, prog, name))) in parts.into_iter().enumerate() {
        let avg = notes.iter().map(|n| n.pitch as u32).sum::<u32>() / notes.len() as u32;
        let drum = ch == 9;
        let other = !drum
            && !Instrument::from_midi_program(prog).has_strings_kind()
            && !(32..=39).contains(&prog);
        let bass = !drum && !other && ((32..=39).contains(&prog) || avg < 45);
        let mut tr = if drum {
            Track::of_instrument(Instrument::Drums, "Drums")
        } else if other {
            let inst = Instrument::from_midi_program(prog);
            Track::of_instrument(inst, inst.name())
        } else if bass {
            Track {
                instrument: Instrument::Bass,
                tuning: STANDARD_BASS.to_vec(),
                ..Track::default()
            }
        } else {
            Track {
                instrument: Instrument::from_midi_program(prog),
                ..Track::default()
            }
        };
        tr.color = TRACK_COLORS[k % TRACK_COLORS.len()];
        let label = if drum {
            "Drums"
        } else if other {
            tr.instrument.name()
        } else if bass {
            "Bass"
        } else {
            "Guitar"
        };
        tr.name = if name.is_empty() {
            format!("{} {}", label, k + 1)
        } else {
            name
        };
        // Quantise and group by onset.
        let mut by_onset: std::collections::BTreeMap<u32, (Vec<u8>, u32, u8)> = Default::default();
        for n in &notes {
            let s = (n.start + GRID / 2) / GRID * GRID;
            let len = (((n.end - n.start) + GRID / 2) / GRID * GRID).max(GRID);
            let e = by_onset.entry(s).or_insert((vec![], 0, 0));
            e.0.push(n.pitch);
            e.1 = e.1.max(len);
            e.2 = e.2.max(n.vel);
        }
        let onsets: Vec<u32> = by_onset.keys().copied().collect();
        let mut bars = vec![];
        for (bi, m) in song.master.iter().enumerate() {
            let bs = bar_starts[bi];
            let be = bs + m.ticks();
            let mut beats: Vec<Beat> = vec![];
            let mut pos = bs;
            let push_rest = |beats: &mut Vec<Beat>, len: u32| {
                for (d, dot) in split_ticks(len) {
                    let mut r = Beat::rest(d);
                    r.dotted = dot;
                    beats.push(r);
                }
            };
            for (i, &o) in onsets
                .iter()
                .enumerate()
                .filter(|&(_, &o)| o >= bs && o < be)
            {
                if o > pos {
                    push_rest(&mut beats, o - pos);
                }
                let next = onsets.get(i + 1).copied().unwrap_or(be).min(be);
                let (pitches, len, vel) = &by_onset[&o];
                let span = next - o;
                let sound = (*len).min(span).max(GRID.min(span));
                let placed = place_pitches(&tr, pitches);
                for (j, (d, dot)) in split_ticks(sound).into_iter().enumerate() {
                    let mut b = Beat::rest(d);
                    b.dotted = dot;
                    b.dynamic = Dynamic::from_velocity(*vel);
                    b.notes = placed
                        .iter()
                        .map(|n| {
                            let mut x = n.clone();
                            x.tie = j > 0;
                            x
                        })
                        .collect();
                    beats.push(b);
                }
                if span > sound {
                    push_rest(&mut beats, span - sound);
                }
                pos = next;
            }
            if beats.is_empty() {
                bars.push(Bar::empty());
            } else {
                if pos < be {
                    push_rest(&mut beats, be - pos);
                }
                bars.push(Bar::new(beats));
            }
        }
        tr.bars = bars;
        song.tracks.push(tr);
    }
    song.normalize();
    Ok(song)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The demo song's guitar part as text tab, and what the importer reads from it: the
    /// number of beats, and every beat's notes.
    fn demo_tab() -> (String, Vec<Vec<(u8, u8)>>) {
        let text = export_ascii(&Song::demo(), 0);
        let read = notes_of(&text);
        (text, read)
    }

    fn notes_of(text: &str) -> Vec<Vec<(u8, u8)>> {
        let track = import_ascii(text).expect("tab");
        track.bars.iter().flat_map(|b| &b.beats).map(|b| b.notes.iter().map(|n| (n.string, n.fret)).collect()).collect()
    }

    /// Applies `damage` to every line of tab (those with bar lines).
    fn damaged(text: &str, damage: impl Fn(&str) -> String) -> String {
        text.lines().map(|l| if l.contains('|') { damage(l) } else { l.to_string() }).collect::<Vec<_>>().join("\n")
    }

    #[test]
    fn tab_damaged_in_the_ordinary_ways_reads_as_the_clean_tab_does() {
        let (text, clean) = demo_tab();
        assert!(clean.len() > 40, "{} beats", clean.len());
        let quoted = text.lines().map(|l| format!("> {l}")).collect::<Vec<_>>().join("\n");
        let twice = text.lines().map(|l| format!(">> {l}")).collect::<Vec<_>>().join("\n");
        let spaced = damaged(&text, |l| l.replace('-', " "));
        let blank = damaged(&text, |l| format!("{l}\n"));
        // A word processor's dashes: each hyphen a dash, or each pair of them one long dash.
        let short = damaged(&text, |l| l.replace('-', "\u{2013}"));
        let long = damaged(&text, |l| l.replace("--", "\u{2014}"));
        let boxed = damaged(&text, |l| l.replace('-', "\u{2500}").replace('|', "\u{2502}"));
        for (name, bad) in [("quoted", quoted), ("quoted twice", twice), ("spaces", spaced), ("blank lines", blank), ("short dashes", short), ("long dashes", long), ("box drawing", boxed)] {
            assert_eq!(notes_of(&bad), clean, "{name}");
        }
        // All of it at once.
        let all = damaged(&text, |l| format!("> {}\n>", l.replace('-', " ")));
        assert_eq!(notes_of(&all), clean, "everything");
    }

    #[test]
    fn systems_are_told_apart_whatever_stands_between_them() {
        let system = |names: &[&str]| names.iter().map(|n| format!("{n}|--0--2--|--3--0--|")).collect::<Vec<_>>().join("\n");
        let guitar = system(&["e", "B", "G", "D", "A", "E"]);
        let bass = system(&["G", "D", "A", "E"]);
        let bars = |text: String| import_ascii(&text).map(|t| (t.tuning.len(), t.bars.len()));
        assert_eq!(bars(format!("{guitar}\n\n{guitar}")), Ok((6, 4)));
        assert_eq!(bars(format!("{guitar}\n{guitar}")), Ok((6, 4)), "nothing between them");
        assert_eq!(bars(format!("{bass}\n\n{bass}")), Ok((4, 4)), "two bass systems, not eight strings");
        // A rule of dashes above a system is not a string of it.
        assert_eq!(bars(format!("Intro\n----------\n\n{guitar}")), Ok((6, 2)));
        // A blank line part of the way down.
        let (top, bottom) = guitar.split_at(guitar.match_indices('\n').nth(3).unwrap().0);
        assert_eq!(bars(format!("{top}\n{bottom}")), Ok((6, 2)));
    }

    #[test]
    fn text_that_is_not_tab_is_left_alone() {
        for text in ["| G  | C  | D  | G  |\n| Em | C  | D  | G  |", "> Thanks for the tab!\n> It sounds great.", "e.g. | one | two |"] {
            assert!(import_ascii(text).is_err(), "{text}");
        }
        assert_eq!(dashes_for_spaces("e|  3   5  |").as_deref(), Some("e|--3---5--|"));
        assert_eq!(dashes_for_spaces("Verse |  3  |"), None);
    }

    #[test]
    fn ascii_round_trip() {
        let song = Song::demo();
        let text = export_ascii(&song, 0);
        let t = import_ascii(&text).unwrap();
        assert_eq!(t.tuning, song.tracks[0].tuning);
        assert!(t.bars.len() >= song.bar_count());
        assert_eq!(t.bars[0].beats[0].note_on(5).map(|n| n.fret), Some(3));
    }

    #[test]
    fn ascii_plain_tab() {
        let tab =
            "e|-----0---|\nB|---1-----|\nG|-2-------|\nD|---------|\nA|3--------|\nE|---------|\n";
        let t = import_ascii(tab).unwrap();
        assert_eq!(t.bars.len(), 1);
        let notes: Vec<u8> = t.bars[0]
            .beats
            .iter()
            .filter_map(|b| b.notes.first().map(|n| n.fret))
            .collect();
        assert_eq!(notes, vec![3, 2, 1, 0]);
    }

    #[test]
    fn midi_round_trip() {
        let song = Song::demo();
        let bytes = crate::fileio::export_midi(&song);
        let back = import_midi(&bytes, "Demo").unwrap();
        assert_eq!(back.tracks.len(), 2);
        assert_eq!(back.tempo, song.tempo);
        assert!(back.bar_count() >= 12);
        // The first guitar note is the low G.
        let first = &back.tracks[0].bars[0].beats[0];
        assert_eq!(
            back.tracks[0].pitch(&first.notes[0]),
            song.tracks[0].pitch(&song.tracks[0].bars[0].beats[0].notes[0])
        );
    }

    #[test]
    fn midi_keeps_title_key_and_sections() {
        let song = Song::demo();
        let back = import_midi(&crate::fileio::export_midi(&song), "file name").unwrap();
        assert_eq!(back.title, song.title);
        assert_eq!(back.master[0].key, song.master[0].key);
        assert_eq!(back.master[0].key_minor, song.master[0].key_minor);
        let names = |s: &Song| -> Vec<String> {
            s.master.iter().filter_map(|m| m.marker.clone()).collect()
        };
        // Repeats are played out, so a section can appear more than once.
        for name in names(&song) {
            assert!(names(&back).contains(&name), "lost section {name}");
        }
    }

    #[test]
    fn midi_holds_tied_notes() {
        let mut song = Song::new_blank();
        let half = |tie: bool| Beat {
            dur: Dur::Half,
            notes: vec![Note {
                tie,
                ..Note::new(0, 3)
            }],
            ..Beat::rest(Dur::Half)
        };
        song.tracks[0].bars[0] = Bar::new(vec![half(false), half(true)]);
        let back = import_midi(&crate::fileio::export_midi(&song), "Tie").unwrap();
        // One note held for the whole bar, not a half note and a half rest.
        assert_eq!(back.tracks[0].bars[0].beats[0].dur, Dur::Whole);
    }

    #[test]
    fn midi_keeps_long_non_ascii_titles() {
        let mut song = Song::demo();
        song.title = "Café ".repeat(40).trim().to_string();
        let back = import_midi(&crate::fileio::export_midi(&song), "x").unwrap();
        assert_eq!(back.title, song.title);
    }
}
