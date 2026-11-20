//! GuitarCraft's built-in instruments. No sound bank is needed: each instrument is a small
//! model of how the real one makes its sound.
//!
//! * Guitars, bass, ukulele and banjo: a plucked string (a looped delay line that is struck
//!   with a pluck-shaped burst and loses its high overtones faster than its low ones), then
//!   the wooden body's resonances for acoustic instruments, or a tube-style overdrive and a
//!   speaker cabinet for the electric ones.
//! * Piano: a hammer-struck string group with slightly stretched overtones, two strings per
//!   note that beat against each other, and a quick-then-slow fade.
//! * Electric piano: a tine and tone bar (two-operator FM).
//! * Organ: drawbar tones with a rotating-speaker wobble and a key click.
//! * Violin/cello/strings, brass, saxophone, flute, voices and synth: band-limited sawtooth
//!   or pulse tones through filters that open and close like a bow, lips, reed or throat.
//! * Drums: tuned and swept tones, filtered noise and the metallic ring of cymbals.

use crate::audio::{NoteEvent, Timbre};
use std::f32::consts::{PI, TAU};

// ---------------------------------------------------------------------------------------------
// Small building blocks
// ---------------------------------------------------------------------------------------------

/// A quick noise source (-1..1).
#[derive(Clone)]
pub struct Rng(u32);

impl Rng {
    pub fn new(seed: u32) -> Rng {
        Rng(seed | 1)
    }
    #[inline]
    pub fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

/// sin(2*pi*p), p in cycles (any value), accurate to about 0.1%.
#[inline]
pub fn fast_sin(p: f32) -> f32 {
    let p = p - p.floor();
    let x = p * TAU - PI;
    let y = 1.273_239_5 * x - 0.405_284_73 * x * x.abs();
    let y = 0.225 * (y * y.abs() - y) + y;
    -y
}

/// A state-variable filter that stays stable while its cutoff moves.
#[derive(Clone, Copy, Default)]
struct Svf {
    ic1: f32,
    ic2: f32,
    a1: f32,
    a2: f32,
    a3: f32,
    k: f32,
}

impl Svf {
    fn new(fc: f32, q: f32, sr: f32) -> Svf {
        let mut s = Svf::default();
        s.set(fc, q, sr);
        s
    }
    fn set(&mut self, fc: f32, q: f32, sr: f32) {
        let fc = fc.clamp(20.0, sr * 0.45);
        let g = (PI * fc / sr).tan();
        self.k = 1.0 / q.max(0.3);
        self.a1 = 1.0 / (1.0 + g * (g + self.k));
        self.a2 = g * self.a1;
        self.a3 = g * self.a2;
    }
    /// (low-pass, band-pass with gain 1 at the centre, high-pass)
    #[inline]
    fn run(&mut self, x: f32) -> (f32, f32, f32) {
        let v3 = x - self.ic2;
        let v1 = self.a1 * self.ic1 + self.a2 * v3;
        let v2 = self.ic2 + self.a2 * self.ic1 + self.a3 * v3;
        self.ic1 = 2.0 * v1 - self.ic1;
        self.ic2 = 2.0 * v2 - self.ic2;
        (v2, self.k * v1, x - self.k * v1 - v2)
    }
    #[inline]
    fn lp(&mut self, x: f32) -> f32 {
        self.run(x).0
    }
    #[inline]
    fn bp(&mut self, x: f32) -> f32 {
        self.run(x).1
    }
    #[inline]
    fn hp(&mut self, x: f32) -> f32 {
        self.run(x).2
    }
}

/// The step that smooths the corner of a sawtooth or pulse (no harsh aliasing).
#[inline]
fn blep(t: f32, dt: f32) -> f32 {
    if t < dt {
        let x = t / dt;
        x + x - x * x - 1.0
    } else if t > 1.0 - dt {
        let x = (t - 1.0) / dt;
        x * x + x + x + 1.0
    } else {
        0.0
    }
}

#[inline]
fn cents(c: f32) -> f32 {
    (c / 1200.0).exp2()
}

#[inline]
fn one_pole(fc: f32, sr: f32) -> f32 {
    1.0 - (-TAU * fc / sr).exp()
}

// ---------------------------------------------------------------------------------------------
// Plucked strings
// ---------------------------------------------------------------------------------------------

struct StrP {
    /// Seconds the fundamental takes to fade to 1/e (at 220 Hz).
    tau: f32,
    /// How quickly high overtones die compared with low ones (at 220 Hz).
    damp: f32,
    /// 0 = a round, soft pluck; 1 = a sharp, bright one.
    bright: f32,
    /// Where along the string it is plucked (fraction of its length).
    pp: f32,
    /// Pick noise in the starting shape.
    nz: f32,
    /// Body resonances: (frequency, sharpness, strength).
    body: [(f32, f32, f32); 2],
    nb: usize,
    /// Overdrive amount (0 = none), asymmetry, speaker rolloff, pre-filter.
    drive: f32,
    bias: f32,
    cab: f32,
    pre: f32,
    /// A final low-pass (Hz; 0 = none).
    tone: f32,
    release: f32,
    level: f32,
}

fn str_params(t: Timbre) -> StrP {
    let base = StrP {
        tau: 2.6,
        damp: 0.11,
        bright: 0.5,
        pp: 0.16,
        nz: 0.25,
        body: [(0.0, 1.0, 0.0); 2],
        nb: 0,
        drive: 0.0,
        bias: 0.0,
        cab: 0.0,
        pre: 0.0,
        tone: 0.0,
        release: 0.035,
        level: 1.0,
    };
    match t {
        Timbre::Steel => StrP {
            body: [(100.0, 5.0, 0.8), (205.0, 6.0, 0.55)],
            nb: 2,
            level: 1.0,
            ..base
        },
        Timbre::Nylon => StrP {
            tau: 2.0,
            damp: 0.2,
            bright: 0.22,
            pp: 0.22,
            nz: 0.2,
            body: [(95.0, 5.0, 0.9), (190.0, 6.0, 0.6)],
            nb: 2,
            release: 0.04,
            ..base
        },
        Timbre::Clean => StrP {
            tau: 3.0,
            damp: 0.12,
            bright: 0.42,
            pp: 0.2,
            nz: 0.1,
            ..base
        },
        Timbre::Overdrive => StrP {
            tau: 3.2,
            damp: 0.12,
            bright: 0.5,
            pp: 0.2,
            nz: 0.12,
            drive: 7.0,
            bias: 0.1,
            cab: 4800.0,
            pre: 110.0,
            release: 0.05,
            ..base
        },
        Timbre::Distortion => StrP {
            tau: 3.8,
            damp: 0.1,
            bright: 0.55,
            pp: 0.2,
            nz: 0.12,
            drive: 28.0,
            bias: 0.15,
            cab: 4200.0,
            pre: 160.0,
            release: 0.05,
            ..base
        },
        Timbre::Bass => StrP {
            tau: 2.4,
            damp: 0.3,
            bright: 0.2,
            pp: 0.3,
            nz: 0.1,
            tone: 1500.0,
            release: 0.05,
            ..base
        },
        Timbre::Uke => StrP {
            tau: 0.9,
            damp: 0.22,
            bright: 0.3,
            pp: 0.2,
            nz: 0.2,
            body: [(300.0, 4.0, 0.8), (640.0, 5.0, 0.45)],
            nb: 2,
            release: 0.03,
            ..base
        },
        Timbre::Banjo => StrP {
            tau: 0.8,
            damp: 0.08,
            bright: 0.8,
            pp: 0.12,
            nz: 0.35,
            body: [(600.0, 3.0, 0.7), (1100.0, 6.0, 0.55)],
            nb: 2,
            release: 0.02,
            ..base
        },
        _ => base,
    }
}

struct Str {
    buf: Vec<f32>,
    mask: usize,
    w: usize,
    /// The loop's low-pass state, and its coefficient (1 = lets everything through).
    state: f32,
    a: f32,
    /// Phase delay of that low-pass in samples (so the pitch stays in tune).
    pd: f32,
    g: f32,
    body: [Svf; 2],
    body_g: [f32; 2],
    nb: usize,
    drive: f32,
    bias: f32,
    pre: f32,
    pre_c: f32,
    xprev: f32,
    cab_c: f32,
    c1: f32,
    c2: f32,
    tone_c: f32,
    tone_s: f32,
    atk: f32,
    atk_dec: f32,
    nlp: f32,
    dcx: f32,
    dcy: f32,
    env: f32,
}

impl Str {
    fn new(e: &NoteEvent, sr: f32, p: &StrP, vel: f32, rng: &mut Rng) -> Str {
        let f = e.freq.max(20.0);
        let mut tau = p.tau * (220.0 / f).powf(0.5).clamp(0.45, 1.6);
        let mut damp0 = p.damp;
        let mut bright = (p.bright + 0.3 * (vel - 0.6)).clamp(0.0, 1.0);
        let mut nz = p.nz;
        let mut atk = 0.35 * vel;
        if e.muted {
            tau = tau.min(0.16);
            damp0 = damp0 * 2.5 + 0.1;
            bright *= 0.5;
        }
        if e.dead {
            tau = 0.05;
            damp0 = 0.9;
            atk = 0.5;
        }
        if e.soft_attack {
            bright *= 0.6;
            nz *= 0.3;
            atk = 0.0;
        }
        let damp = (damp0 * (220.0 / f).powf(0.7)).clamp(0.015, 0.8);
        let a = 1.0 - damp;
        let pd = damp / a;
        let g = (-1.0 / (tau * f)).exp();

        let size = (((sr / f) * 1.6) as usize + 16).next_power_of_two();
        let mut buf = vec![0.0f32; size];
        let d = (sr / f - pd).max(2.0);
        let n = (d.round() as usize).clamp(2, size - 4);
        // The starting shape of the string: a plucked triangle, its sharper "velocity" twin,
        // and a little pick noise.
        let nf = n as f32;
        let ppn = (p.pp * nf).clamp(1.0, nf - 1.0);
        let mut tri = vec![0.0f32; n];
        for (i, x) in tri.iter_mut().enumerate() {
            let i = i as f32;
            *x = if i < ppn { i / ppn } else { (nf - i) / (nf - ppn) };
        }
        let mut sharp = vec![0.0f32; n];
        for i in 0..n {
            sharp[i] = tri[i] - tri[(i + n - 1) % n];
        }
        let mut noise = vec![0.0f32; n];
        let mut lp = 0.0;
        let nc = 0.2 + 0.6 * bright;
        for x in noise.iter_mut() {
            lp += nc * (rng.next() - lp);
            *x = lp;
        }
        let tidy = |v: &mut Vec<f32>| {
            let mean = v.iter().sum::<f32>() / v.len() as f32;
            let mut rms = 0.0;
            for x in v.iter_mut() {
                *x -= mean;
                rms += *x * *x;
            }
            let rms = (rms / v.len() as f32).sqrt().max(1e-6);
            for x in v.iter_mut() {
                *x /= rms;
            }
        };
        tidy(&mut tri);
        tidy(&mut sharp);
        tidy(&mut noise);
        let mut exc = vec![0.0f32; n];
        if e.harmonic {
            // A natural or pinched harmonic is just the string's own purest tone.
            for (i, x) in exc.iter_mut().enumerate() {
                *x = (TAU * i as f32 / nf).sin() * 1.4 + noise[i] * 0.05;
            }
        } else if e.dead {
            exc[..n].copy_from_slice(&noise[..n]);
        } else {
            for i in 0..n {
                exc[i] = (1.0 - bright) * tri[i] + bright * sharp[i] + nz * noise[i];
            }
        }
        // Scale so every pluck starts with the same strength.
        let rms = (exc.iter().map(|x| x * x).sum::<f32>() / nf).sqrt().max(1e-6);
        let k = 0.28 / rms;
        // The loop starts with the burst just behind the write position.
        let l = n + 2;
        for i in 0..n {
            buf[l - n + i] = exc[i] * k;
        }
        let mask = size - 1;
        let mut body = [Svf::default(); 2];
        let mut body_g = [0.0; 2];
        for i in 0..p.nb {
            body[i] = Svf::new(p.body[i].0, p.body[i].1, sr);
            body_g[i] = p.body[i].2;
        }
        Str {
            buf,
            mask,
            w: l & mask,
            state: 0.0,
            a,
            pd,
            g,
            body,
            body_g,
            nb: p.nb,
            drive: p.drive,
            bias: p.bias,
            pre: 0.0,
            pre_c: one_pole(p.pre.max(20.0), sr),
            xprev: 0.0,
            cab_c: if p.cab > 0.0 { one_pole(p.cab, sr) } else { 1.0 },
            c1: 0.0,
            c2: 0.0,
            tone_c: if p.tone > 0.0 { one_pole(p.tone, sr) } else { 1.0 },
            tone_s: 0.0,
            atk: if e.soft_attack { 0.0 } else { atk },
            atk_dec: (-1.0 / (0.004 * sr)).exp(),
            nlp: 0.0,
            dcx: 0.0,
            dcy: 0.0,
            env: 1.0,
        }
    }

    #[inline]
    fn run(&mut self, f: f32, n: f32, tsec: f32, sr: f32) -> (f32, bool) {
        let size = self.buf.len();
        let d = (sr / f - self.pd).clamp(2.0, (size - 4) as f32);
        let rp = (self.w + size) as f32 - d;
        let i = rp as usize;
        let fr = rp - i as f32;
        let y = self.buf[i & self.mask] * (1.0 - fr) + self.buf[(i + 1) & self.mask] * fr;
        let filt = self.a * y + (1.0 - self.a) * self.state;
        self.state = filt;
        self.buf[self.w] = filt * self.g;
        self.w = (self.w + 1) & self.mask;
        self.env = (self.env * 0.9995).max(y.abs());
        let mut s = y;
        if self.atk > 1e-4 {
            // The click of the pick.
            let hp = n - self.nlp;
            self.nlp += 0.3 * (n - self.nlp);
            s += hp * self.atk;
            self.atk *= self.atk_dec;
        }
        if self.nb > 0 {
            let mut acc = s;
            for i in 0..self.nb {
                acc += self.body_g[i] * self.body[i].bp(s);
            }
            s = acc;
        }
        if self.drive > 0.0 {
            // Tighten the lows, push it into the amp (twice as fine to keep it smooth),
            // then the speaker cabinet takes the fizz off the top.
            let hp = s - self.pre;
            self.pre += self.pre_c * (s - self.pre);
            let x = hp * self.drive;
            let mid = 0.5 * (x + self.xprev);
            self.xprev = x;
            let b = self.bias.tanh();
            let z = 0.5 * (((mid + self.bias).tanh() - b) + ((x + self.bias).tanh() - b));
            self.c1 += self.cab_c * (z - self.c1);
            self.c2 += self.cab_c * (self.c1 - self.c2);
            s = self.c2 * 0.75;
        }
        if self.tone_c < 1.0 {
            self.tone_s += self.tone_c * (s - self.tone_s);
            s = self.tone_s;
        }
        // Keep any slow drift out.
        let o = s - self.dcx + 0.998 * self.dcy;
        self.dcx = s;
        self.dcy = o;
        let alive = self.env > 1e-5 || tsec < 0.05;
        (o, alive)
    }
}

// ---------------------------------------------------------------------------------------------
// Piano
// ---------------------------------------------------------------------------------------------

const NP: usize = 20;

struct Piano {
    n: usize,
    ph1: [f32; NP],
    ph2: [f32; NP],
    inc1: [f32; NP],
    inc2: [f32; NP],
    e1: [f32; NP],
    e2: [f32; NP],
    d1: [f32; NP],
    d2: [f32; NP],
    f0: f32,
    thump: f32,
    thump_dec: f32,
    tlp: f32,
}

impl Piano {
    fn new(e: &NoteEvent, sr: f32, vel: f32, rng: &mut Rng) -> Piano {
        let f = e.freq.max(25.0);
        let b = 0.00006 + 0.0003 * (f / 1000.0).powi(2);
        let tau0 = (7.0 * (261.6 / f).powf(0.7)).clamp(0.6, 14.0);
        let expo = 0.5 + 1.1 * (1.0 - vel);
        let detune = (0.3 + 1.2 * (f / 1000.0).clamp(0.0, 1.5)).min(1.6);
        let mut p = Piano {
            n: 0,
            ph1: [0.0; NP],
            ph2: [0.0; NP],
            inc1: [0.0; NP],
            inc2: [0.0; NP],
            e1: [0.0; NP],
            e2: [0.0; NP],
            d1: [1.0; NP],
            d2: [1.0; NP],
            f0: f,
            thump: 0.0,
            thump_dec: (-1.0 / (0.012 * sr)).exp(),
            tlp: 0.0,
        };
        let mut sumsq = 0.0;
        for k in 1..=NP {
            let kf = k as f32;
            let fk = f * kf * (1.0 + b * kf * kf).sqrt();
            if fk > sr * 0.45 {
                break;
            }
            // The hammer hits an eighth of the way along the string.
            let mut a = (kf * PI / 8.0).sin().abs() * kf.powf(-expo);
            if k == 1 {
                a *= (f / 200.0).clamp(0.3, 1.0);
            }
            if a < 0.003 {
                continue;
            }
            let i = p.n;
            p.n += 1;
            let tk = tau0 / (1.0 + 0.35 * (kf - 1.0));
            p.inc1[i] = fk * cents(-detune) / sr;
            p.inc2[i] = fk * cents(detune) / sr;
            p.ph1[i] = rng.next() * 0.5 + 0.5;
            p.ph2[i] = rng.next() * 0.5 + 0.5;
            p.e1[i] = 0.6 * a;
            p.e2[i] = 0.4 * a;
            p.d1[i] = (-1.0 / (tk * 0.1 * sr)).exp();
            p.d2[i] = (-1.0 / (tk * sr)).exp();
            sumsq += a * a;
        }
        let norm = 0.5 / (0.5 * sumsq).sqrt().max(0.05);
        for i in 0..p.n {
            p.e1[i] *= norm;
            p.e2[i] *= norm;
        }
        p.thump = 0.45 * vel * (1.0 - (f / 1500.0).min(0.8));
        p
    }

    #[inline]
    fn run(&mut self, f: f32, n: f32, _tsec: f32, _sr: f32) -> (f32, bool) {
        let scale = f / self.f0;
        let mut s = 0.0;
        let mut live = 0.0;
        for i in 0..self.n {
            self.ph1[i] += self.inc1[i] * scale;
            self.ph2[i] += self.inc2[i] * scale;
            if self.ph1[i] >= 1.0 {
                self.ph1[i] -= 1.0;
            }
            if self.ph2[i] >= 1.0 {
                self.ph2[i] -= 1.0;
            }
            let a = self.e1[i] + self.e2[i];
            s += a * 0.55 * (fast_sin(self.ph1[i]) + fast_sin(self.ph2[i]));
            self.e1[i] *= self.d1[i];
            self.e2[i] *= self.d2[i];
            live += a;
        }
        if self.thump > 1e-4 {
            // The felt hammer landing.
            self.tlp += 0.12 * (n - self.tlp);
            s += self.tlp * self.thump;
            self.thump *= self.thump_dec;
        }
        (s, live > 1e-4 || self.thump > 1e-4)
    }
}

// ---------------------------------------------------------------------------------------------
// Electric piano
// ---------------------------------------------------------------------------------------------

struct EPiano {
    phc: f32,
    phm: f32,
    pht: f32,
    idx: f32,
    idx_dec: f32,
    e1: f32,
    e2: f32,
    d1: f32,
    d2: f32,
    tine: f32,
    tine_dec: f32,
}

impl EPiano {
    fn new(e: &NoteEvent, sr: f32, vel: f32) -> EPiano {
        let f = e.freq.max(25.0);
        let tau = (2.4 * (261.6 / f).powf(0.5)).clamp(0.4, 5.0);
        EPiano {
            phc: 0.0,
            phm: 0.0,
            pht: 0.0,
            idx: 1.0 + 2.4 * vel,
            idx_dec: (-1.0 / (0.35 * (261.6 / f).powf(0.3).clamp(0.4, 1.5) * sr)).exp(),
            e1: 0.55,
            e2: 0.45,
            d1: (-1.0 / (tau * 0.15 * sr)).exp(),
            d2: (-1.0 / (tau * sr)).exp(),
            tine: 0.2 * vel,
            tine_dec: (-1.0 / (0.06 * sr)).exp(),
        }
    }

    #[inline]
    fn run(&mut self, f: f32, _n: f32, _tsec: f32, sr: f32) -> (f32, bool) {
        let inc = f / sr;
        self.phc = (self.phc + inc).fract();
        self.phm = (self.phm + inc).fract();
        self.pht = (self.pht + inc * 7.0).fract();
        let m = fast_sin(self.phm);
        // The modulation fades to a gentle bell-like floor.
        let idx = self.idx + 0.25;
        let s = fast_sin(self.phc + idx * m / TAU);
        let tine = fast_sin(self.pht) * self.tine;
        let env = self.e1 + self.e2;
        self.idx *= self.idx_dec;
        self.e1 *= self.d1;
        self.e2 *= self.d2;
        self.tine *= self.tine_dec;
        ((s * env + tine) * 0.9, env > 1e-4)
    }
}

// ---------------------------------------------------------------------------------------------
// Organ
// ---------------------------------------------------------------------------------------------

const DRAWBARS: [(f32, f32); 9] = [
    (0.5, 0.7),
    (1.0, 1.0),
    (1.5, 0.5),
    (2.0, 0.8),
    (3.0, 0.55),
    (4.0, 0.4),
    (5.0, 0.2),
    (6.0, 0.2),
    (8.0, 0.15),
];

struct Organ {
    ph: [f32; 9],
    lfo: f32,
    perc: f32,
    perc_dec: f32,
    click: f32,
    click_dec: f32,
    nlp: f32,
}

impl Organ {
    fn new(sr: f32, vel: f32) -> Organ {
        Organ {
            ph: [0.0; 9],
            lfo: 0.0,
            perc: 0.45 * vel,
            perc_dec: (-1.0 / (0.25 * sr)).exp(),
            click: 0.25,
            click_dec: (-1.0 / (0.006 * sr)).exp(),
            nlp: 0.0,
        }
    }

    #[inline]
    fn run(&mut self, f: f32, n: f32, _tsec: f32, sr: f32) -> (f32, bool) {
        // The rotating speaker: a slow wobble in pitch and loudness.
        self.lfo = (self.lfo + 6.7 / sr).fract();
        let wob = fast_sin(self.lfo);
        let f = f * (1.0 + 0.0035 * wob);
        let mut s = 0.0;
        let mut total = 0.0;
        for (i, (ratio, w)) in DRAWBARS.iter().enumerate() {
            self.ph[i] = (self.ph[i] + f * ratio / sr).fract();
            s += w * fast_sin(self.ph[i]);
            total += w;
        }
        s /= total * 0.55;
        // Percussion: a quick third harmonic at the start of each note.
        if self.perc > 1e-4 {
            s += self.perc * fast_sin(self.ph[4]);
            self.perc *= self.perc_dec;
        }
        if self.click > 1e-4 {
            let hp = n - self.nlp;
            self.nlp += 0.4 * (n - self.nlp);
            s += hp * self.click;
            self.click *= self.click_dec;
        }
        (s * (1.0 + 0.05 * wob), true)
    }
}

// ---------------------------------------------------------------------------------------------
// Bowed strings, brass, reeds, flute, voices and synth: a source and filters
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
enum Wave {
    Saw,
    Pulse,
    Sine,
}

struct Sub {
    wave: Wave,
    duty: f32,
    n: usize,
    ph: [f32; 3],
    det: [f32; 3],
    /// The main filter: its cutoff glides from `lp_from` to `lp_to`.
    lp: Svf,
    lp_from: f32,
    lp_to: f32,
    lp_rise: f32,
    q: f32,
    direct: f32,
    form: [Svf; 3],
    form_g: [f32; 3],
    form_q: [f32; 3],
    nform: usize,
    /// The first vowel resonance follows the pitch for high voices.
    follow: bool,
    breath: f32,
    breath_bp: Svf,
    chiff: f32,
    chiff_dec: f32,
    drive: f32,
    amp_sus: f32,
    amp_tau: f32,
    tick: u32,
    level: f32,
}

impl Sub {
    fn new(t: Timbre, f: f32, vel: f32, sr: f32) -> Sub {
        let mut s = Sub {
            wave: Wave::Saw,
            duty: 0.5,
            n: 1,
            ph: [0.0; 3],
            det: [1.0; 3],
            lp: Svf::new(2000.0, 0.9, sr),
            lp_from: 2000.0,
            lp_to: 2000.0,
            lp_rise: 0.1,
            q: 0.9,
            direct: 1.0,
            form: [Svf::default(); 3],
            form_g: [0.0; 3],
            form_q: [1.0; 3],
            nform: 0,
            follow: false,
            breath: 0.0,
            breath_bp: Svf::new(3000.0, 1.0, sr),
            chiff: 0.0,
            chiff_dec: (-1.0 / (0.04 * sr)).exp(),
            drive: 0.0,
            amp_sus: 1.0,
            amp_tau: 0.3,
            tick: 0,
            level: 1.0,
        };
        let set_form = |s: &mut Sub, list: &[(f32, f32, f32)]| {
            s.nform = list.len().min(3);
            for (i, (ff, q, g)) in list.iter().take(3).enumerate() {
                s.form_q[i] = *q;
                s.form_g[i] = *g;
                s.form[i] = Svf::new(*ff, *q, sr);
            }
        };
        match t {
            Timbre::Bowed => {
                s.n = 2;
                s.det = [cents(-4.0), cents(4.0), 1.0];
                let c = (f * 8.0).clamp(1800.0, 5500.0);
                s.lp_from = c * 0.5;
                s.lp_to = c;
                s.lp_rise = 0.12;
                s.direct = 0.55;
                set_form(&mut s, &[(280.0, 7.0, 0.45), (540.0, 8.0, 0.35), (1250.0, 6.0, 0.3)]);
                s.breath = 0.015;
                s.level = 0.9;
            }
            Timbre::Brass => {
                s.n = 2;
                s.det = [cents(-2.0), cents(2.0), 1.0];
                s.lp_from = f * 1.5;
                s.lp_to = (f * 9.0).min(7500.0) * (0.6 + 0.4 * vel);
                s.lp_rise = 0.05;
                s.q = 1.3;
                s.direct = 0.8;
                set_form(&mut s, &[(1100.0, 3.0, 0.35)]);
                s.breath = 0.02;
                s.level = 0.8;
            }
            Timbre::Reed => {
                s.wave = Wave::Pulse;
                s.duty = 0.3;
                s.lp_from = f * 3.0;
                s.lp_to = (f * 6.0).clamp(1200.0, 4500.0);
                s.lp_rise = 0.06;
                s.q = 1.6;
                s.direct = 0.6;
                set_form(&mut s, &[(1000.0, 4.0, 0.5), (2300.0, 5.0, 0.3)]);
                s.breath = 0.05;
                s.level = 0.85;
            }
            Timbre::Flute => {
                s.wave = Wave::Sine;
                s.lp_from = 9000.0;
                s.lp_to = 9000.0;
                s.q = 0.7;
                s.direct = 1.0;
                s.breath = 0.1;
                s.breath_bp = Svf::new((f * 4.0).clamp(1500.0, 6000.0), 2.0, sr);
                s.chiff = 2.5;
                s.level = 0.9;
            }
            Timbre::Voice => {
                s.n = 2;
                s.det = [cents(-5.0), cents(5.0), 1.0];
                s.lp_from = 2500.0;
                s.lp_to = 4000.0;
                s.lp_rise = 0.1;
                s.q = 0.7;
                s.direct = 0.15;
                s.follow = true;
                let f1 = 730.0f32.max(f * 1.05);
                set_form(&mut s, &[(f1, 8.0, 1.0), (1090.0, 10.0, 0.55), (2440.0, 12.0, 0.3)]);
                s.breath = 0.025;
                // Lower voices need more help to be heard than high ones.
                s.level = (3.4 - f / 350.0).clamp(0.8, 3.2);
            }
            _ => {
                // Synth: three detuned saws through a filter that closes after the start.
                s.n = 3;
                s.det = [cents(-7.0), 1.0, cents(7.0)];
                s.lp_from = (f * 14.0).min(12000.0);
                s.lp_to = (f * 2.5).max(400.0);
                s.lp_rise = 0.3;
                s.q = 1.8;
                s.direct = 1.0;
                s.drive = 1.4;
                s.amp_sus = 0.8;
                s.amp_tau = 0.3;
                s.level = 0.6;
            }
        }
        s.lp.set(s.lp_from, s.q, sr);
        s.ph = [0.0, 0.31, 0.67];
        s
    }

    #[inline]
    fn run(&mut self, f: f32, n: f32, tsec: f32, sr: f32) -> (f32, bool) {
        let mut src = 0.0;
        for i in 0..self.n {
            let inc = f * self.det[i] / sr;
            self.ph[i] += inc;
            if self.ph[i] >= 1.0 {
                self.ph[i] -= 1.0;
            }
            let p = self.ph[i];
            src += match self.wave {
                Wave::Saw => 2.0 * p - 1.0 - blep(p, inc),
                Wave::Pulse => {
                    let v = if p < self.duty { 1.0 } else { -1.0 };
                    v + blep(p, inc) - blep((p + 1.0 - self.duty).fract(), inc)
                }
                Wave::Sine => fast_sin(p) + 0.18 * fast_sin(p * 2.0) + 0.05 * fast_sin(p * 3.0),
            };
        }
        src /= (self.n as f32).sqrt();
        if self.tick.is_multiple_of(32) {
            let u = 1.0 - (-tsec / self.lp_rise.max(0.001)).exp();
            self.lp.set(self.lp_from + (self.lp_to - self.lp_from) * u, self.q, sr);
            if self.follow {
                self.form[0].set(730.0f32.max(f * 1.05), self.form_q[0], sr);
            }
        }
        self.tick = self.tick.wrapping_add(1);
        let mut out = self.lp.lp(src) * self.direct;
        for i in 0..self.nform {
            out += self.form[i].bp(src) * self.form_g[i];
        }
        if self.breath > 0.0 {
            let mut b = self.breath;
            if self.chiff > 1e-3 {
                b *= 1.0 + self.chiff;
                self.chiff *= self.chiff_dec;
            }
            out += self.breath_bp.bp(n) * b * 2.0;
        }
        if self.drive > 0.0 {
            out = (out * self.drive).tanh() / self.drive.tanh().max(0.5) * 0.9;
        }
        if self.amp_sus < 1.0 {
            out *= self.amp_sus + (1.0 - self.amp_sus) * (-tsec / self.amp_tau).exp();
        }
        (out * self.level, true)
    }
}

// ---------------------------------------------------------------------------------------------
// Drums
// ---------------------------------------------------------------------------------------------

const METAL: [f32; 6] = [205.3, 304.4, 369.6, 522.7, 540.0, 800.0];

struct Drum {
    id: u8,
    ph: [f32; 6],
    f: [Svf; 2],
}

impl Drum {
    fn new(id: u8, sr: f32) -> Drum {
        let mut d = Drum { id, ph: [0.0; 6], f: [Svf::default(); 2] };
        let q = 0.7;
        match id {
            35 | 36 => d.f[0] = Svf::new(2000.0, q, sr),
            37 => d.f[0] = Svf::new(1800.0, 1.5, sr),
            38 | 40 => {
                d.f[0] = Svf::new(1800.0, q, sr);
                d.f[1] = Svf::new(9000.0, q, sr);
            }
            39 => d.f[0] = Svf::new(1300.0, 1.2, sr),
            42 | 44 | 46 => {
                d.f[0] = Svf::new(6500.0, q, sr);
                d.f[1] = Svf::new(15000.0, q, sr);
            }
            49 | 52 | 55 | 57 => {
                d.f[0] = Svf::new(3500.0, q, sr);
                d.f[1] = Svf::new(15000.0, q, sr);
            }
            51 | 53 | 59 => {
                d.f[0] = Svf::new(6000.0, 0.6, sr);
                d.f[1] = Svf::new(2500.0, q, sr);
            }
            54 => d.f[0] = Svf::new(5500.0, q, sr),
            56 => d.f[0] = Svf::new(800.0, 2.5, sr),
            69 | 70 | 82 => d.f[0] = Svf::new(5000.0, q, sr),
            41 | 43 | 45 | 47 | 48 | 50 => d.f[0] = Svf::new(1500.0, q, sr),
            _ => d.f[0] = Svf::new(3000.0, q, sr),
        }
        d
    }

    /// The ring of a cymbal or hat: six square tones beating together.
    fn metal(&mut self, scale: f32, sr: f32) -> f32 {
        let mut m = 0.0;
        for (ph, fr) in self.ph.iter_mut().zip(METAL) {
            *ph = (*ph + fr * scale / sr).fract();
            m += if *ph < 0.5 { 1.0 } else { -1.0 };
        }
        m / 6.0
    }

    fn tone(&mut self, i: usize, fr: f32, sr: f32) -> f32 {
        self.ph[i] = (self.ph[i] + fr / sr).fract();
        fast_sin(self.ph[i])
    }

    /// The cowbell's two tones as one square-ish signal.
    fn cowbell(&mut self, sr: f32) -> f32 {
        self.ph[0] = (self.ph[0] + 562.0 / sr).fract();
        self.ph[1] = (self.ph[1] + 845.0 / sr).fract();
        (if self.ph[0] < 0.5 { 1.0 } else { -1.0 }) + (if self.ph[1] < 0.5 { 1.0 } else { -1.0 })
    }

    fn run(&mut self, n: f32, t: f32, sr: f32) -> (f32, bool) {
        let id = self.id;
        let (s, dur) = match id {
            35 | 36 => {
                let fr = 48.0 + 130.0 * (-t * 32.0).exp();
                let body = self.tone(0, fr, sr);
                let click = self.f[0].hp(n);
                (
                    (body * 1.6).tanh() * (-t * 8.0).exp() * 0.9 + click * (-t * 300.0).exp() * 0.25,
                    0.6,
                )
            }
            37 => {
                let c = self.f[0].bp(n);
                let b = self.tone(0, 880.0, sr);
                (c * (-t * 90.0).exp() * 1.4 + b * (-t * 70.0).exp() * 0.5, 0.15)
            }
            38 | 40 => {
                let shell = if id == 40 { 200.0 } else { 185.0 };
                let a = self.tone(0, shell * (1.0 + 0.15 * (-t * 60.0).exp()), sr);
                let b = self.tone(1, shell * 1.78, sr);
                let h = self.f[0].hp(n);
                let wires = self.f[1].lp(h);
                (
                    a * (-t * 20.0).exp() * 0.6
                        + b * (-t * 28.0).exp() * 0.3
                        + wires * (-t * 14.0).exp() * 1.1,
                    0.45,
                )
            }
            39 => {
                let c = self.f[0].bp(n);
                // Three hands then the room.
                let burst = if t < 0.033 { (-(t % 0.011) * 600.0).exp() } else { 0.0 };
                let tail = if t >= 0.033 { (-(t - 0.033) * 18.0).exp() * 0.35 } else { 0.0 };
                (c * (burst + tail) * 2.0, 0.35)
            }
            41 | 43 | 45 | 47 | 48 | 50 => {
                let idx = [41, 43, 45, 47, 48, 50].iter().position(|x| *x == id).unwrap_or(0);
                let base = [82.0, 98.0, 116.0, 135.0, 155.0, 180.0][idx];
                let fr = base * (1.0 + 0.5 * (-t * 20.0).exp());
                let body = self.tone(0, fr, sr);
                let click = self.f[0].bp(n);
                (body * (-t * 7.0).exp() * 1.1 + click * (-t * 80.0).exp() * 0.3, 0.7)
            }
            42 | 44 | 46 => {
                let m = self.metal(2.0, sr);
                let h = self.f[0].hp(0.6 * n + 0.5 * m);
                let x = self.f[1].lp(h);
                let rate = match id {
                    42 => 60.0,
                    44 => 40.0,
                    _ => 7.0,
                };
                (x * (-t * rate).exp() * 0.9, if id == 46 { 0.8 } else { 0.15 })
            }
            49 | 52 | 55 | 57 => {
                let m = self.metal(2.7, sr);
                let h = self.f[0].hp(0.6 * n + 0.5 * m);
                let x = self.f[1].lp(h);
                let rate = if id == 55 { 4.0 } else { 1.6 };
                let hit = 1.0 - (-t * 300.0).exp();
                (x * hit * (-t * rate).exp() * 0.9, if id == 55 { 1.2 } else { 3.0 })
            }
            51 | 53 | 59 => {
                let m = self.metal(2.2, sr);
                let b = self.f[0].bp(0.4 * n + m);
                let x = self.f[1].lp(b) * 1.2 + self.f[0].hp(n) * 0.1;
                let ping = self.tone(0, 3300.0, sr) * 0.12 + self.tone(1, 5000.0, sr) * 0.06;
                let bell = if id == 53 {
                    (self.tone(2, 1400.0, sr) + 0.6 * self.tone(3, 2100.0, sr)) * (-t * 3.0).exp() * 0.5
                } else {
                    0.0
                };
                (
                    x * (-t * 2.5).exp() * 0.8
                        + ping * (-t * 4.0).exp()
                        + bell
                        + n * (-t * 150.0).exp() * 0.15,
                    1.8,
                )
            }
            54 => {
                let m = self.metal(3.0, sr);
                let x = self.f[0].hp(0.7 * n + 0.4 * m);
                (x * (-t * 14.0).exp() * 0.9, 0.3)
            }
            56 => {
                let a = self.cowbell(sr);
                (self.f[0].bp(a + 0.2 * n) * (-t * 9.0).exp() * 1.6, 0.5)
            }
            60..=64 => {
                let base = 330.0 - (id as f32 - 60.0) * 40.0;
                let fr = base * (1.0 + 0.2 * (-t * 40.0).exp());
                let body = self.tone(0, fr, sr);
                (body * (-t * 11.0).exp() * 0.9 + n * (-t * 150.0).exp() * 0.15, 0.4)
            }
            69 | 70 | 82 => {
                let x = self.f[0].hp(n);
                (x * (t / 0.012).min(1.0) * (-t * 35.0).exp() * 0.9, 0.2)
            }
            75 => (self.tone(0, 2500.0, sr) * (-t * 60.0).exp() * 0.9, 0.12),
            76 | 77 => {
                let base = if id == 76 { 880.0 } else { 660.0 };
                (
                    (self.tone(0, base, sr) + 0.4 * self.tone(1, base * 1.5, sr)) * (-t * 40.0).exp() * 0.7,
                    0.15,
                )
            }
            80 | 81 => {
                let rate = if id == 80 { 30.0 } else { 3.0 };
                (
                    (self.tone(0, 4200.0, sr) + 0.6 * self.tone(1, 6300.0, sr)) * (-t * rate).exp() * 0.35,
                    if id == 80 { 0.25 } else { 2.0 },
                )
            }
            _ => (self.f[0].hp(n) * (-t * 25.0).exp() * 0.6, 0.3),
        };
        let gain = match id {
            35 | 36 => 1.6,
            37 => 1.8,
            39 => 3.5,
            41 | 43 | 45 | 47 | 48 | 50 => 1.2,
            42 | 44 => 3.2,
            46 => 3.0,
            49 | 52 | 57 => 2.0,
            51 | 59 => 4.0,
            53 => 1.4,
            54 => 1.8,
            55 => 2.2,
            56 => 0.5,
            _ => 1.0,
        };
        (s * gain, t < dur)
    }
}

// ---------------------------------------------------------------------------------------------
// One sounding note
// ---------------------------------------------------------------------------------------------

enum Engine {
    Str(Box<Str>),
    Piano(Box<Piano>),
    EPiano(EPiano),
    Organ(Organ),
    Sub(Box<Sub>),
    Drum(Drum),
    Click { ph: f32, e: f32, dec: f32 },
}

pub struct Voice {
    pub delay: u64,
    pub t: u64,
    pub len: u64,
    pub track: usize,
    pub string: u8,
    pub fret: u8,
    pub click: bool,
    gl: f32,
    gr: f32,
    release: f32,
    rel_k: f32,
    attack: u64,
    f0: f32,
    bend: f32,
    bend_shape: u8,
    slide: f32,
    slide_in: f32,
    vibrato: bool,
    vib_depth: f32,
    rng: Rng,
    eng: Engine,
}

/// Evens out how loud each instrument is next to the others.
fn trim(t: Timbre) -> f32 {
    match t {
        Timbre::Steel => 1.25,
        Timbre::Nylon => 1.3,
        Timbre::Clean => 1.45,
        Timbre::Overdrive => 0.68,
        Timbre::Distortion => 0.58,
        Timbre::Bass => 1.4,
        Timbre::Uke => 1.5,
        Timbre::Banjo => 1.6,
        Timbre::Piano => 1.2,
        Timbre::EPiano => 0.85,
        Timbre::Organ => 0.65,
        Timbre::Synth => 1.3,
        Timbre::Bowed => 1.3,
        Timbre::Reed => 0.6,
        Timbre::Flute => 0.55,
        _ => 1.0,
    }
}

impl Voice {
    pub fn new(e: &NoteEvent, sr: f32) -> Voice {
        let mut rng = Rng::new(
            0x1234_5678
                ^ (e.start as u32).wrapping_mul(2_654_435_761)
                ^ ((e.string as u32) << 8)
                ^ ((e.fret as u32) << 16),
        );
        let vel = e.vel.clamp(0.05, 1.0);
        let pan = e.pan.clamp(-1.0, 1.0);
        let ang = (pan + 1.0) * std::f32::consts::FRAC_PI_4;
        let t = e.timbre;
        let mut vol = e.vel * 0.55 * trim(t);
        let held = t.sustained();
        // Seconds to fade in, and the time constant of the fade out after the note ends.
        let (attack_s, release_s) = match t {
            Timbre::Organ => (0.008, 0.04),
            Timbre::Synth => (0.015, 0.12),
            Timbre::Voice => (0.08, 0.18),
            Timbre::Bowed => (0.09, 0.18),
            Timbre::Brass => (0.045, 0.05),
            Timbre::Reed => (0.04, 0.06),
            Timbre::Flute => (0.06, 0.07),
            Timbre::Piano => (0.002, 0.15),
            Timbre::EPiano => (0.002, 0.12),
            Timbre::Drum => (0.0, 0.1),
            Timbre::ClickHigh | Timbre::ClickLow => (0.0, 0.012),
            _ => {
                let p = str_params(t);
                (if e.soft_attack { 0.012 } else { 0.0015 }, p.release)
            }
        };
        let attack = if e.fade_in {
            (e.len as f32 * 0.6).max(0.05 * sr) as u64
        } else {
            (attack_s * sr) as u64
        };
        let eng = match t {
            Timbre::Drum => {
                let id = (69.0 + 12.0 * (e.freq.max(1.0) / 440.0).log2())
                    .round()
                    .clamp(1.0, 127.0) as u8;
                Engine::Drum(Drum::new(id, sr))
            }
            Timbre::ClickHigh | Timbre::ClickLow => Engine::Click {
                ph: 0.0,
                e: 1.0,
                dec: (-1.0 / (0.012 * sr)).exp(),
            },
            Timbre::Piano => Engine::Piano(Box::new(Piano::new(e, sr, vel, &mut rng))),
            Timbre::EPiano => Engine::EPiano(EPiano::new(e, sr, vel)),
            Timbre::Organ => Engine::Organ(Organ::new(sr, vel)),
            Timbre::Synth | Timbre::Voice | Timbre::Bowed | Timbre::Brass | Timbre::Reed | Timbre::Flute => {
                Engine::Sub(Box::new(Sub::new(t, e.freq.max(20.0), vel, sr)))
            }
            _ => {
                let p = str_params(t);
                vol *= p.level;
                Engine::Str(Box::new(Str::new(e, sr, &p, vel, &mut rng)))
            }
        };
        let click = matches!(t, Timbre::ClickHigh | Timbre::ClickLow);
        Voice {
            delay: 0,
            t: 0,
            len: e.len,
            track: e.track,
            string: e.string,
            fret: e.fret,
            click,
            gl: vol * ang.cos(),
            gr: vol * ang.sin(),
            release: 1.0,
            rel_k: (-1.0 / (release_s * sr)).exp(),
            attack,
            f0: e.freq,
            bend: e.bend,
            bend_shape: e.bend_shape,
            slide: e.slide,
            slide_in: e.slide_in,
            vibrato: e.vibrato
                || matches!(t, Timbre::Voice | Timbre::Bowed | Timbre::Flute)
                || (held && e.wide_vibrato),
            vib_depth: if e.wide_vibrato {
                0.9
            } else if matches!(t, Timbre::Voice | Timbre::Bowed | Timbre::Flute) && !e.vibrato {
                0.18
            } else {
                0.35
            },
            rng,
            eng,
        }
    }

    /// One stereo sample, and whether the note is still sounding.
    pub fn next(&mut self, sr: f32) -> (f32, f32, bool) {
        if self.delay > 0 {
            self.delay -= 1;
            return (0.0, 0.0, true);
        }
        let tsec = self.t as f32 / sr;
        let lensec = self.len as f32 / sr;
        let mut semis = 0.0;
        if self.bend != 0.0 {
            let half = (lensec * 0.5).max(0.05);
            let up = (tsec / half).min(1.0);
            let down = ((tsec - half) / half).clamp(0.0, 1.0);
            semis += self.bend
                * match self.bend_shape {
                    1 => up - down,
                    2 => 1.0,
                    3 => 1.0 - up,
                    _ => up,
                };
        }
        if self.slide_in != 0.0 {
            let p = (tsec / 0.08).min(1.0);
            semis += (self.slide_in * (1.0 - p)).round();
        }
        if self.slide != 0.0 && lensec > 0.0 {
            let start = lensec * 0.6;
            if tsec > start {
                let p = ((tsec - start) / (lensec - start).max(0.01)).min(1.0);
                semis += (self.slide * p).round();
            }
        }
        if self.vibrato && tsec > 0.08 {
            let grow = ((tsec - 0.08) / 0.25).min(1.0);
            semis += self.vib_depth * grow * (tsec * 5.5 * TAU).sin();
        }
        let f = if semis != 0.0 { self.f0 * (semis / 12.0).exp2() } else { self.f0 };
        let n = self.rng.next();
        let (s, mut alive) = match &mut self.eng {
            Engine::Str(x) => x.run(f, n, tsec, sr),
            Engine::Piano(x) => x.run(f, n, tsec, sr),
            Engine::EPiano(x) => x.run(f, n, tsec, sr),
            Engine::Organ(x) => x.run(f, n, tsec, sr),
            Engine::Sub(x) => x.run(f, n, tsec, sr),
            Engine::Drum(x) => x.run(n, tsec, sr),
            Engine::Click { ph, e, dec } => {
                *ph = (*ph + f / sr).fract();
                let s = (fast_sin(*ph) + 0.4 * fast_sin(*ph * 2.0)) * *e * 1.5;
                *e *= *dec;
                (s, *e > 1e-3)
            }
        };
        let drum = matches!(self.eng, Engine::Drum(_));
        let mut amp = 1.0;
        if self.t < self.attack {
            amp = self.t as f32 / self.attack.max(1) as f32;
        }
        // Drums and clicks ring out on their own; everything else fades when the note ends.
        if self.t >= self.len && !drum && !self.click {
            self.release *= self.rel_k;
            if self.release < 0.001 {
                alive = false;
            }
        }
        amp *= self.release;
        self.t += 1;
        (s * amp * self.gl, s * amp * self.gr, alive)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(t: Timbre, freq: f32) -> NoteEvent {
        NoteEvent {
            start: 0,
            len: 44100,
            freq,
            bend: 0.0,
            slide: 0.0,
            vibrato: false,
            vel: 0.8,
            pan: 0.0,
            timbre: t,
            muted: false,
            dead: false,
            harmonic: false,
            soft_attack: false,
            track: 0,
            string: 0,
            fret: 0,
            bend_shape: 0,
            slide_in: 0.0,
            wide_vibrato: false,
            fade_in: false,
        }
    }

    fn render(e: &NoteEvent, secs: f32, sr: f32) -> Vec<f32> {
        let mut v = Voice::new(e, sr);
        let mut out = vec![];
        for _ in 0..(secs * sr) as usize {
            let (l, r, alive) = v.next(sr);
            out.push((l + r) * 0.5);
            if !alive {
                break;
            }
        }
        out
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
    }

    /// How strongly the sound has a tone at `f` (a single Fourier bin).
    fn tone_at(x: &[f32], f: f32, sr: f32) -> f32 {
        let (mut c, mut s) = (0.0f64, 0.0f64);
        for (i, v) in x.iter().enumerate() {
            let a = std::f64::consts::TAU * f as f64 * i as f64 / sr as f64;
            c += *v as f64 * a.cos();
            s += *v as f64 * a.sin();
        }
        ((c * c + s * s).sqrt() / x.len() as f64) as f32
    }

    const ALL: [Timbre; 17] = [
        Timbre::Steel,
        Timbre::Nylon,
        Timbre::Clean,
        Timbre::Overdrive,
        Timbre::Distortion,
        Timbre::Bass,
        Timbre::Uke,
        Timbre::Banjo,
        Timbre::Piano,
        Timbre::EPiano,
        Timbre::Organ,
        Timbre::Synth,
        Timbre::Voice,
        Timbre::Bowed,
        Timbre::Brass,
        Timbre::Reed,
        Timbre::Flute,
    ];

    #[test]
    fn every_instrument_makes_a_clean_note_at_a_sensible_level() {
        let sr = 44100.0;
        for t in ALL {
            for f in [82.4, 220.0, 440.0, 988.0] {
                let x = render(&event(t, f), 1.2, sr);
                assert!(x.iter().all(|v| v.is_finite()), "{t:?} {f}");
                let peak = x.iter().fold(0.0f32, |m, v| m.max(v.abs()));
                let r = rms(&x[..x.len().min(22050)]);
                println!("{t:?} {f:6.1} Hz  peak {peak:.3}  rms {r:.3}");
                assert!(peak < 1.2, "{t:?} {f} too loud: {peak}");
                assert!(r > 0.02, "{t:?} {f} too quiet: {r}");
            }
        }
    }

    #[test]
    fn instruments_play_the_pitch_they_are_asked_for() {
        let sr = 44100.0;
        for t in [
            Timbre::Steel,
            Timbre::Nylon,
            Timbre::Clean,
            Timbre::Bass,
            Timbre::Uke,
            Timbre::Banjo,
            Timbre::Piano,
            Timbre::EPiano,
            Timbre::Synth,
            Timbre::Voice,
            Timbre::Bowed,
            Timbre::Brass,
            Timbre::Reed,
            Timbre::Flute,
        ] {
            for f in [110.0f32, 220.0, 329.6, 659.3] {
                let x = render(&event(t, f), 0.6, sr);
                let seg = &x[2000..(sr * 0.5) as usize];
                // Slightly off either side should be clearly weaker than the note itself.
                let here = (0..9)
                    .map(|i| tone_at(seg, f * (1.0 + (i as f32 - 4.0) * 0.0007), sr))
                    .fold(0.0f32, f32::max);
                let flat = tone_at(seg, f * 0.97, sr);
                let sharp = tone_at(seg, f * 1.03, sr);
                assert!(
                    here > 1.5 * flat.max(sharp),
                    "{t:?} at {f} Hz is out of tune (here {here}, flat {flat}, sharp {sharp})"
                );
            }
        }
    }

    #[test]
    fn plucked_strings_fade_and_a_muted_string_fades_faster() {
        let sr = 44100.0;
        let open = render(&event(Timbre::Steel, 196.0), 3.0, sr);
        assert!(rms(&open[..4410]) > 2.0 * rms(&open[open.len() - 4410..]));
        let mut m = event(Timbre::Steel, 196.0);
        m.muted = true;
        let muted = render(&m, 3.0, sr);
        assert!(muted.len() < open.len() || rms(&muted[22050..26460]) < 0.2 * rms(&open[22050..26460]));
    }

    #[test]
    fn a_note_stops_after_it_ends_and_drums_ring_out_by_themselves() {
        let sr = 44100.0;
        let mut e = event(Timbre::Organ, 220.0);
        e.len = 4410;
        let x = render(&e, 3.0, sr);
        assert!(x.len() < (sr * 0.6) as usize, "organ kept sounding: {} samples", x.len());
        // A kick from a very short event still thumps.
        let mut k = event(Timbre::Drum, 440.0 * 2f32.powf((36.0 - 69.0) / 12.0));
        k.len = 10;
        let x = render(&k, 1.0, sr);
        assert!(rms(&x[..4410]) > 0.05);
        assert!(x.len() > 10000);
    }

    #[test]
    fn every_drum_sound_is_finite_and_ends() {
        let sr = 44100.0;
        for id in 27..=87u8 {
            let mut e = event(Timbre::Drum, 440.0 * 2f32.powf((id as f32 - 69.0) / 12.0));
            e.len = 100;
            let x = render(&e, 5.0, sr);
            assert!(x.iter().all(|v| v.is_finite()), "drum {id}");
            assert!(x.len() < (sr * 4.0) as usize, "drum {id} never ends");
            let peak = x.iter().fold(0.0f32, |m, v| m.max(v.abs()));
            println!("drum {id}: peak {peak:.3}, {} samples", x.len());
            assert!(peak < 1.5, "drum {id} too loud: {peak}");
        }
    }
}
