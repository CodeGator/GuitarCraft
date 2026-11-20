//! The tuner: listens to a microphone (or line input) and works out how far each string is
//! from its note.
//!
//! Polyphonic: one strum of all the strings. For each string of the track, the sound's
//! spectrum is scanned from 60 cents below to 60 cents above the string's note (the first
//! three harmonics counted together) and the peak gives the string's offset; a string whose
//! peak doesn't stand out is "not heard". A string's harmonic can land on another string's
//! note (low E's third harmonic is a B), so a string that isn't played can sometimes show.
//! Monophonic: one string at a time, found with the YIN pitch detector.
//!
//! The microphone feeds a ring of the last second of sound, reduced to about 12 kHz (the
//! highest note a tuner needs is far below that); a background thread analyses it about
//! eight times a second and leaves the result in `Tuner::readout`.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Below this level (RMS) nothing is analysed: the strings are "not heard".
const GATE: f32 = 0.004;
/// The rate the sound is reduced to before analysis.
const WORK_RATE: f32 = 12000.0;
/// How far from a string's note the polyphonic scan looks, in cents.
const SCAN: f32 = 60.0;

/// What the tuner hears right now.
#[derive(Clone, Default, Debug)]
pub struct Readout {
    /// Input level (RMS of the last quarter second).
    pub level: f32,
    /// Each string's offset from its note in cents (None: not heard), in track order.
    pub strings: Vec<Option<f32>>,
    /// The single note heard (Hz), for the monophonic section.
    pub mono: Option<f32>,
}

struct Ring {
    data: Vec<f32>,
    pos: usize,
    filled: bool,
}

impl Ring {
    fn push(&mut self, x: f32) {
        self.data[self.pos] = x;
        self.pos = (self.pos + 1) % self.data.len();
        if self.pos == 0 {
            self.filled = true;
        }
    }

    /// The last `n` samples, oldest first (fewer while the ring is filling).
    fn last(&self, n: usize) -> Vec<f32> {
        let len = if self.filled { self.data.len() } else { self.pos };
        let n = n.min(len);
        let start = (self.pos + self.data.len() - n) % self.data.len();
        (0..n).map(|i| self.data[(start + i) % self.data.len()]).collect()
    }
}

/// A running tuner: the input stream and the analysis thread. Dropping it stops both.
pub struct Tuner {
    _stream: cpal::Stream,
    stop: Arc<AtomicBool>,
    /// The strings' notes (Hz) to tune to; the window keeps it in step with the track.
    pub targets: Arc<Mutex<Vec<f32>>>,
    pub readout: Arc<Mutex<Readout>>,
    /// The input device's name.
    pub device: String,
}

impl Drop for Tuner {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

pub fn input_device_names() -> Vec<String> {
    cpal::default_host()
        .input_devices()
        .map(|it| it.filter_map(|d| d.name().ok()).collect())
        .unwrap_or_default()
}

impl Tuner {
    /// Starts listening on the named input (None: the system's default input).
    pub fn start(name: Option<&str>) -> Result<Tuner, String> {
        let host = cpal::default_host();
        let chosen = name.and_then(|n| {
            host.input_devices().ok()?.find(|d| d.name().map(|x| x == n).unwrap_or(false))
        });
        let device = chosen
            .or_else(|| host.default_input_device())
            .ok_or_else(|| "No microphone or line input found".to_string())?;
        let device_name = device.name().unwrap_or_else(|_| "Input".into());
        let cfg = device.default_input_config().map_err(|e| e.to_string())?;
        let channels = (cfg.channels() as usize).max(1);
        let sr = cfg.sample_rate().0 as f32;
        let step = (sr / WORK_RATE).round().max(1.0) as usize;
        let rate = sr / step as f32;
        let ring = Arc::new(Mutex::new(Ring {
            data: vec![0.0; rate as usize],
            pos: 0,
            filled: false,
        }));
        // Mix the channels to mono, then average `step` samples into one.
        let make_sink = |ring: Arc<Mutex<Ring>>| {
            let (mut acc, mut cnt) = (0.0f32, 0usize);
            move |frames: &mut dyn Iterator<Item = f32>| {
                let Ok(mut r) = ring.lock() else { return };
                for x in frames {
                    acc += x;
                    cnt += 1;
                    if cnt == step * channels {
                        r.push(acc / cnt as f32);
                        acc = 0.0;
                        cnt = 0;
                    }
                }
            }
        };
        let config: cpal::StreamConfig = cfg.config();
        let err_fn = |e| eprintln!("tuner input error: {e}");
        let stream = match cfg.sample_format() {
            cpal::SampleFormat::F32 => {
                let mut sink = make_sink(ring.clone());
                device.build_input_stream(
                    &config,
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        sink(&mut data.iter().copied())
                    },
                    err_fn,
                    None,
                )
            }
            cpal::SampleFormat::I16 => {
                let mut sink = make_sink(ring.clone());
                device.build_input_stream(
                    &config,
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        sink(&mut data.iter().map(|&x| x as f32 / 32768.0))
                    },
                    err_fn,
                    None,
                )
            }
            cpal::SampleFormat::U16 => {
                let mut sink = make_sink(ring.clone());
                device.build_input_stream(
                    &config,
                    move |data: &[u16], _: &cpal::InputCallbackInfo| {
                        sink(&mut data.iter().map(|&x| (x as f32 - 32768.0) / 32768.0))
                    },
                    err_fn,
                    None,
                )
            }
            f => return Err(format!("Unsupported input sample format {f:?}")),
        }
        .map_err(|e| e.to_string())?;
        stream.play().map_err(|e| e.to_string())?;

        let stop = Arc::new(AtomicBool::new(false));
        let targets = Arc::new(Mutex::new(Vec::<f32>::new()));
        let readout = Arc::new(Mutex::new(Readout::default()));
        {
            let (stop, targets, readout) = (stop.clone(), targets.clone(), readout.clone());
            std::thread::spawn(move || {
                let mut smooth: Vec<Option<f32>> = vec![];
                while !stop.load(Ordering::Relaxed) {
                    std::thread::sleep(Duration::from_millis(120));
                    let x = match ring.lock() {
                        Ok(r) => r.last((rate * 0.75) as usize),
                        Err(_) => break,
                    };
                    let tg = targets.lock().map(|t| t.clone()).unwrap_or_default();
                    let out = analyse(&x, rate, &tg);
                    // Smooth each string's reading a little so the bars don't jitter.
                    smooth.resize(out.strings.len(), None);
                    for (s, new) in smooth.iter_mut().zip(&out.strings) {
                        *s = match (*s, *new) {
                            (Some(old), Some(c)) if (old - c).abs() < 15.0 => {
                                Some(old * 0.5 + c * 0.5)
                            }
                            (_, c) => c,
                        };
                    }
                    if let Ok(mut r) = readout.lock() {
                        *r = Readout { strings: smooth.clone(), ..out };
                    }
                }
            });
        }
        Ok(Tuner { _stream: stream, stop, targets, readout, device: device_name })
    }
}

/// Everything the tuner shows, from the last 0.75 s of sound at `sr`.
pub fn analyse(x: &[f32], sr: f32, targets: &[f32]) -> Readout {
    let tail = &x[x.len().saturating_sub((sr * 0.25) as usize)..];
    let level = rms(tail);
    if level < GATE || x.len() < (sr * 0.2) as usize {
        return Readout { level, strings: vec![None; targets.len()], mono: None };
    }
    Readout {
        level,
        strings: string_offsets(x, sr, targets),
        mono: pitch(&x[x.len().saturating_sub((sr * 0.2) as usize)..], sr)
            .map(|f| refine(x, sr, f)),
    }
}

fn rms(x: &[f32]) -> f32 {
    if x.is_empty() {
        return 0.0;
    }
    (x.iter().map(|v| v * v).sum::<f32>() / x.len() as f32).sqrt()
}

/// The amplitude of the frequency `f` in the windowed sound (Goertzel).
fn amplitude(w: &[f32], wsum: f32, f: f32, sr: f32) -> f32 {
    let coeff = 2.0 * (std::f64::consts::TAU * f as f64 / sr as f64).cos();
    let (mut s1, mut s2) = (0.0f64, 0.0f64);
    for &x in w {
        let s = x as f64 + coeff * s1 - s2;
        s2 = s1;
        s1 = s;
    }
    let p = (s1 * s1 + s2 * s2 - coeff * s1 * s2).max(0.0);
    (2.0 * p.sqrt() / wsum as f64) as f32
}

/// The sound with a Hann window, and the window's sum (to turn Goertzel into amplitude).
fn windowed(x: &[f32]) -> (Vec<f32>, f32) {
    let n = x.len();
    let w = x
        .iter()
        .enumerate()
        .map(|(i, v)| {
            v * (0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / (n - 1).max(1) as f32).cos())
        })
        .collect();
    (w, n as f32 * 0.5)
}

/// Scans `f0` +-`span` cents for the strongest peak of harmonic `h` and returns its offset
/// in cents (refined between steps), and how strongly it stands out from the rest of the
/// scan (peak / median) and its amplitude. None if `h * f0` is too high to measure.
fn scan(w: &[f32], wsum: f32, sr: f32, f0: f32, h: f32, span: f32) -> Option<(f32, f32, f32)> {
    if f0 * h * (span / 1200.0).exp2() > sr * 0.45 {
        return None;
    }
    let step = 2.0;
    let cs: Vec<f32> = (0..=((2.0 * span / step) as usize))
        .map(|i| -span + i as f32 * step)
        .collect();
    let sc: Vec<f32> = cs
        .iter()
        .map(|&c| amplitude(w, wsum, f0 * h * (c / 1200.0).exp2(), sr))
        .collect();
    let (bi, &best) = sc.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1))?;
    let mut sorted = sc.clone();
    sorted.sort_by(f32::total_cmp);
    let stand_out = best / sorted[sorted.len() / 2].max(1e-12);
    // A peak at the edge of the scan is further out than the scan reaches.
    if bi == 0 || bi == sc.len() - 1 {
        return Some((cs[bi], stand_out, best));
    }
    // Refine between the steps: a parabola through the peak and its neighbours.
    let (a, b, c) = (sc[bi - 1], sc[bi], sc[bi + 1]);
    let d = a - 2.0 * b + c;
    let off = if d.abs() > 1e-12 { 0.5 * (a - c) / d } else { 0.0 };
    Some((cs[bi] + off.clamp(-1.0, 1.0) * step, stand_out, best))
}

/// Each target string's offset in cents from one strum (None: not heard). Each string is
/// found by its fundamental; when that is too weak (low strings often are), by its 2nd
/// harmonic. Higher harmonics are left out: they land on other strings' notes.
pub fn string_offsets(x: &[f32], sr: f32, targets: &[f32]) -> Vec<Option<f32>> {
    if x.len() < 64 {
        return vec![None; targets.len()];
    }
    let (w, wsum) = windowed(x);
    let level = rms(x);
    let heard = |r: &(f32, f32, f32)| r.1 > 2.5 && r.2 > level * 0.05;
    targets
        .iter()
        .map(|&f0| {
            if f0 <= 0.0 {
                return None;
            }
            let fund = scan(&w, wsum, sr, f0, 1.0, SCAN)?;
            if heard(&fund) {
                return Some(fund.0);
            }
            scan(&w, wsum, sr, f0, 2.0, SCAN).filter(heard).map(|r| r.0)
        })
        .collect()
}

/// Sharpens a pitch estimate on the spectrum (a long window measures far more finely than
/// YIN's period at the reduced rate).
fn refine(x: &[f32], sr: f32, f: f32) -> f32 {
    let (w, wsum) = windowed(x);
    match scan(&w, wsum, sr, f, 1.0, 30.0) {
        Some((c, _, _)) if c.abs() < 29.0 => f * (c / 1200.0).exp2(),
        _ => f,
    }
}

/// The pitch of a single note (YIN), 40..1500 Hz.
pub fn pitch(x: &[f32], sr: f32) -> Option<f32> {
    let max_tau = ((sr / 40.0) as usize).min(x.len() / 2);
    let min_tau = ((sr / 1500.0) as usize).max(2);
    if max_tau <= min_tau + 2 {
        return None;
    }
    let win = x.len() - max_tau;
    let mut d = vec![0.0f32; max_tau + 1];
    for (tau, dv) in d.iter_mut().enumerate().skip(1) {
        let mut s = 0.0;
        for j in 0..win {
            let e = x[j] - x[j + tau];
            s += e * e;
        }
        *dv = s;
    }
    // Cumulative mean normalised difference.
    let mut cm = vec![1.0f32; max_tau + 1];
    let mut run = 0.0;
    for tau in 1..=max_tau {
        run += d[tau];
        cm[tau] = if run > 0.0 { d[tau] * tau as f32 / run } else { 1.0 };
    }
    let mut tau = min_tau;
    let found = loop {
        if tau >= max_tau {
            break None;
        }
        if cm[tau] < 0.15 {
            while tau + 1 < max_tau && cm[tau + 1] < cm[tau] {
                tau += 1;
            }
            break Some(tau);
        }
        tau += 1;
    }?;
    let t = found as f32;
    let (a, b, c) = (cm[found - 1], cm[found], cm[found + 1]);
    let den = a - 2.0 * b + c;
    let off = if den.abs() > 1e-9 { 0.5 * (a - c) / den } else { 0.0 };
    Some(sr / (t + off.clamp(-1.0, 1.0)))
}

/// The note frequency (Hz) of a MIDI note number.
pub fn note_hz(midi: f32) -> f32 {
    440.0 * ((midi - 69.0) / 12.0).exp2()
}

/// Cents from `target` to `f`.
pub fn cents(f: f32, target: f32) -> f32 {
    1200.0 * (f / target).log2()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A plucked string: six harmonics, falling off, fading out.
    fn pluck(f: f32, sr: f32, n: usize, out: &mut [f32]) {
        for (i, o) in out.iter_mut().enumerate().take(n) {
            let t = i as f32 / sr;
            let env = (-t * 1.5).exp();
            for h in 1..=6 {
                *o += 0.1 / h as f32 * env * (std::f32::consts::TAU * f * h as f32 * t).sin();
            }
        }
    }

    const STANDARD: [u8; 6] = [64, 59, 55, 50, 45, 40];

    #[test]
    fn a_strum_shows_each_strings_offset() {
        let sr = WORK_RATE;
        let n = (sr * 0.75) as usize;
        let targets: Vec<f32> = STANDARD.iter().map(|&m| note_hz(m as f32)).collect();
        // Strings off by these cents; the G string isn't played.
        let off = [0.0, 12.0, f32::NAN, -20.0, 5.0, -8.0];
        let mut x = vec![0.0f32; n];
        let mut rng = 1u32;
        for (t, o) in targets.iter().zip(off) {
            if !o.is_nan() {
                pluck(t * (o / 1200.0).exp2(), sr, n, &mut x);
            }
        }
        for v in &mut x {
            rng = rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            *v += ((rng >> 8) as f32 / 16_777_216.0 - 0.5) * 0.004;
        }
        let r = analyse(&x, sr, &targets);
        for (k, o) in off.iter().enumerate() {
            match r.strings[k] {
                Some(c) if !o.is_nan() => assert!((c - o).abs() < 2.0, "string {} read {c}, was {o}", k + 1),
                None if o.is_nan() => {}
                got => panic!("string {}: got {got:?}, expected {o}", k + 1),
            }
        }
    }

    /// A low string whose fundamental is weak (as a guitar's low E often is through a
    /// microphone) is found by its 2nd harmonic, even well out of tune.
    #[test]
    fn a_weak_low_string_is_found_by_its_harmonic() {
        let sr = WORK_RATE;
        let n = (sr * 0.75) as usize;
        let f = note_hz(40.0) * (-35.0f32 / 1200.0).exp2();
        let x: Vec<f32> = (0..n)
            .map(|i| {
                let t = i as f32 / sr;
                let p = std::f32::consts::TAU * f * t;
                0.004 * p.sin() + 0.1 * (2.0 * p).sin() + 0.05 * (3.0 * p).sin()
            })
            .collect();
        let r = analyse(&x, sr, &[note_hz(40.0)]);
        let c = r.strings[0].expect("heard");
        assert!((c + 35.0).abs() < 2.0, "read {c}");
    }

    #[test]
    fn silence_shows_nothing() {
        let targets = [note_hz(40.0)];
        let r = analyse(&vec![0.0; 9000], WORK_RATE, &targets);
        assert_eq!(r.strings, vec![None]);
        assert_eq!(r.mono, None);
    }

    #[test]
    fn one_string_gives_its_pitch() {
        let sr = WORK_RATE;
        for f in [82.41f32, 110.0, 196.0, 329.63, 660.0] {
            let n = (sr * 0.75) as usize;
            let mut x = vec![0.0f32; n];
            pluck(f, sr, n, &mut x);
            let got = analyse(&x, sr, &[]).mono.expect("a pitch");
            assert!(cents(got, f).abs() < 3.0, "{f} Hz read as {got}");
        }
    }
}
