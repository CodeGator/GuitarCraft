//! A small built-in synthesizer and sequencer so songs can be heard without
//! any sound bank. Plucked strings are made from decaying harmonics.

use crate::model::*;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Timbre {
    Steel,
    Nylon,
    Clean,
    Overdrive,
    Distortion,
    Bass,
    Uke,
    Banjo,
    Piano,
    EPiano,
    Organ,
    Synth,
    Voice,
    Bowed,
    Brass,
    Reed,
    Flute,
    Drum,
    ClickHigh,
    ClickLow,
}

impl Timbre {
    pub fn of(i: Instrument) -> Timbre {
        match i {
            Instrument::SteelGuitar => Timbre::Steel,
            Instrument::NylonGuitar => Timbre::Nylon,
            Instrument::CleanGuitar => Timbre::Clean,
            Instrument::OverdriveGuitar => Timbre::Overdrive,
            Instrument::DistortionGuitar => Timbre::Distortion,
            Instrument::Bass => Timbre::Bass,
            Instrument::Ukulele => Timbre::Uke,
            Instrument::Banjo => Timbre::Banjo,
            Instrument::Piano => Timbre::Piano,
            Instrument::ElectricPiano => Timbre::EPiano,
            Instrument::Organ => Timbre::Organ,
            Instrument::Synth => Timbre::Synth,
            Instrument::LeadVocals | Instrument::Choir => Timbre::Voice,
            Instrument::Violin | Instrument::Cello | Instrument::Strings => Timbre::Bowed,
            Instrument::Trumpet => Timbre::Brass,
            Instrument::Saxophone => Timbre::Reed,
            Instrument::Flute => Timbre::Flute,
            Instrument::Drums => Timbre::Drum,
        }
    }
    /// Held notes (voices, bowed strings, winds, organ) keep sounding until released.
    pub fn sustained(self) -> bool {
        matches!(self, Timbre::Organ | Timbre::Synth | Timbre::Voice | Timbre::Bowed | Timbre::Brass | Timbre::Reed | Timbre::Flute)
    }
}

#[derive(Clone, Debug)]
pub struct NoteEvent {
    pub start: u64,
    pub len: u64,
    pub freq: f32,
    /// Pitch change in semitones reached at `bend_end` fraction of the note.
    pub bend: f32,
    /// Slide target in semitones, applied over the last part of the note.
    pub slide: f32,
    pub vibrato: bool,
    pub vel: f32,
    pub pan: f32,
    pub timbre: Timbre,
    pub muted: bool,
    pub dead: bool,
    pub harmonic: bool,
    pub soft_attack: bool,
    pub track: usize,
    pub string: u8,
    pub fret: u8,
    /// 0 bend up, 1 bend and release, 2 pre-bend, 3 pre-bend and release.
    pub bend_shape: u8,
    /// Starting pitch offset (semitones) that glides to the note.
    pub slide_in: f32,
    pub wide_vibrato: bool,
    pub fade_in: bool,
}

const H: usize = 14;

struct Voice {
    delay: u64,
    bend_shape: u8,
    slide_in: f32,
    vib_depth: f32,
    t: u64,
    len: u64,
    f0: f32,
    bend: f32,
    slide: f32,
    vibrato: bool,
    phase: f32,
    env: [f32; H],
    dec: [f32; H],
    gl: f32,
    gr: f32,
    drive: f32,
    noise: f32,
    noise_dec: f32,
    rng: u32,
    lp: f32,
    release: f32,
    attack: u64,
    pub track: usize,
    pub string: u8,
    pub fret: u8,
    click: bool,
    /// General MIDI drum sound (0 = not a drum).
    drum: u8,
}

impl Voice {
    fn new(e: &NoteEvent, sr: f32) -> Voice {
        let (pluck, base_decay, bright, harmonics, drive): (f32, f32, f32, usize, f32) = match e.timbre {
            Timbre::Steel => (0.16, 2.6, 1.0, 14, 0.0),
            Timbre::Nylon => (0.3, 2.0, 0.55, 10, 0.0),
            Timbre::Clean => (0.12, 3.2, 0.8, 12, 0.0),
            Timbre::Overdrive => (0.15, 3.5, 0.8, 10, 3.5),
            Timbre::Distortion => (0.15, 4.0, 0.9, 10, 9.0),
            Timbre::Bass => (0.22, 3.5, 0.5, 9, 0.0),
            Timbre::Uke => (0.25, 1.4, 0.7, 10, 0.0),
            Timbre::Banjo => (0.1, 1.2, 1.0, 14, 0.0),
            Timbre::Piano => (0.12, 4.0, 0.7, 12, 0.0),
            Timbre::EPiano => (0.5, 3.0, 0.35, 6, 0.0),
            Timbre::Synth => (0.5, 30.0, 0.9, 12, 1.2),
            Timbre::Organ | Timbre::Voice | Timbre::Bowed | Timbre::Brass | Timbre::Reed | Timbre::Flute => (0.5, 30.0, 1.0, 12, 0.0),
            Timbre::Drum => (0.5, 0.3, 1.0, 1, 0.0),
            Timbre::ClickHigh | Timbre::ClickLow => (0.5, 0.03, 1.0, 1, 0.0),
        };
        // Held instruments: a fixed blend of harmonics instead of a plucked spectrum.
        let shape: Option<[f32; 12]> = match e.timbre {
            Timbre::Organ => Some([1.0, 0.8, 0.5, 0.45, 0.0, 0.3, 0.0, 0.25, 0.0, 0.0, 0.0, 0.1]),
            Timbre::Voice => Some([1.0, 0.7, 0.55, 0.35, 0.18, 0.1, 0.06, 0.04, 0.02, 0.0, 0.0, 0.0]),
            Timbre::Bowed => Some([1.0, 0.5, 0.33, 0.25, 0.2, 0.16, 0.12, 0.1, 0.08, 0.06, 0.05, 0.04]),
            Timbre::Brass => Some([1.0, 0.8, 0.6, 0.45, 0.35, 0.25, 0.18, 0.12, 0.08, 0.05, 0.03, 0.02]),
            Timbre::Reed => Some([1.0, 0.25, 0.6, 0.2, 0.4, 0.15, 0.25, 0.1, 0.15, 0.05, 0.08, 0.03]),
            Timbre::Flute => Some([1.0, 0.25, 0.08, 0.03, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
            Timbre::Synth => Some([1.0, 0.5, 0.33, 0.25, 0.2, 0.17, 0.14, 0.12, 0.11, 0.1, 0.09, 0.08]),
            _ => None,
        };
        let held = e.timbre.sustained();
        let mut env = [0.0f32; H];
        let mut dec = [1.0f32; H];
        let mut decay_s = base_decay * (220.0 / e.freq.max(40.0)).powf(0.35);
        if e.muted {
            decay_s = 0.12;
        }
        if e.dead {
            decay_s = 0.03;
        }
        for k in 0..harmonics.min(H) {
            let kf = (k + 1) as f32;
            if e.freq * kf > sr * 0.45 {
                break;
            }
            let mut a = ((std::f32::consts::PI * kf * pluck).sin().abs() / kf).max(0.0);
            a *= bright.powf(k as f32 * 0.35);
            if e.muted {
                a *= 0.6f32.powf(k as f32);
            }
            if e.harmonic {
                // Natural harmonic: keep mostly the octave-and-up partials.
                a = if (k + 1) % 2 == 0 { 1.0 / kf } else { 0.05 / kf };
            }
            if let Some(sh) = shape {
                a = sh.get(k).copied().unwrap_or(0.0);
            }
            env[k] = a;
            let ds = if held { decay_s } else { decay_s / (1.0 + 0.35 * k as f32) };
            dec[k] = (-1.0 / (ds * sr)).exp();
        }
        // Normalise.
        let sum: f32 = env.iter().sum::<f32>().max(0.001);
        for x in &mut env {
            *x /= sum;
        }
        let pan = e.pan.clamp(-1.0, 1.0);
        let ang = (pan + 1.0) * std::f32::consts::FRAC_PI_4;
        let vol = e.vel * 0.55;
        let click = matches!(e.timbre, Timbre::ClickHigh | Timbre::ClickLow);
        Voice {
            delay: 0,
            bend_shape: e.bend_shape,
            slide_in: e.slide_in,
            vib_depth: if e.wide_vibrato { 0.9 } else if matches!(e.timbre, Timbre::Voice | Timbre::Bowed | Timbre::Flute) && !e.vibrato { 0.18 } else { 0.35 },
            t: 0,
            len: e.len,
            f0: e.freq,
            bend: e.bend,
            slide: e.slide,
            vibrato: e.vibrato || matches!(e.timbre, Timbre::Voice | Timbre::Bowed | Timbre::Flute),
            phase: 0.0,
            env,
            dec,
            gl: vol * ang.cos(),
            gr: vol * ang.sin(),
            drive,
            noise: if e.soft_attack || click || held {
                0.0
            } else if e.dead {
                0.5
            } else {
                0.12
            },
            noise_dec: (-1.0 / (0.006 * sr)).exp(),
            rng: 0x1234_5678 ^ (e.start as u32).wrapping_mul(2654435761),
            lp: 0.0,
            release: 1.0,
            attack: if held && !e.fade_in {
                ((if e.timbre == Timbre::Brass { 0.035 } else { 0.07 }) * sr) as u64
            } else if e.fade_in {
                ((e.len as f32 * 0.6).max(0.05 * sr)) as u64
            } else if e.soft_attack {
                (0.012 * sr) as u64
            } else {
                (0.0015 * sr) as u64
            },
            track: e.track,
            string: e.string,
            fret: e.fret,
            click,
            drum: if e.timbre == Timbre::Drum { (69.0 + 12.0 * (e.freq.max(1.0) / 440.0).log2()).round().clamp(1.0, 127.0) as u8 } else { 0 },
        }
    }

    fn white(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        (self.rng as f32 / u32::MAX as f32) * 2.0 - 1.0
    }

    /// One sample of a drum or cymbal: tuned sine sweeps for drums, filtered noise for cymbals.
    fn drum_next(&mut self, sr: f32) -> (f32, f32, bool) {
        use std::f32::consts::TAU;
        let t = self.t as f32 / sr;
        self.t += 1;
        let n = self.white();
        self.lp += 0.45 * (n - self.lp);
        let hp = n - self.lp;
        let d = self.drum;
        let tone = |f: f32, ph: &mut f32| {
            *ph += f / sr;
            if *ph > 1.0 {
                *ph -= 1.0;
            }
            (*ph * TAU).sin()
        };
        let mut ph = self.phase;
        let (s, dur) = match d {
            35 | 36 => {
                let f = 48.0 + 110.0 * (-t * 28.0).exp();
                (tone(f, &mut ph) * 1.5 * (-t * 8.0).exp() + n * 0.15 * (-t * 90.0).exp(), 0.5)
            }
            37 => (n * 0.6 * (-t * 70.0).exp() + tone(420.0, &mut ph) * 0.4 * (-t * 50.0).exp(), 0.12),
            38 | 40 | 39 => (n * 0.65 * (-t * 15.0).exp() + tone(190.0, &mut ph) * 0.55 * (-t * 22.0).exp(), 0.4),
            42 | 44 => (hp * 0.55 * (-t * 45.0).exp(), 0.12),
            46 => (hp * 0.45 * (-t * 5.5).exp(), 0.7),
            49 | 57 | 52 | 55 => (hp * 0.5 * (-t * 2.2).exp(), 2.0),
            51 | 59 | 53 => (hp * 0.22 * (-t * 3.0).exp() + tone(2900.0, &mut ph) * 0.06 * (-t * 2.0).exp(), 1.6),
            41 | 43 | 45 | 47 | 48 | 50 => {
                let base = 70.0 + (d as f32 - 41.0) * 17.0;
                let f = base * (1.0 + 0.4 * (-t * 18.0).exp());
                (tone(f, &mut ph) * 1.1 * (-t * 7.0).exp() + n * 0.1 * (-t * 60.0).exp(), 0.7)
            }
            56 => (tone(560.0, &mut ph) * 0.5 * (-t * 14.0).exp(), 0.3),
            _ => (hp * 0.5 * (-t * 22.0).exp(), 0.3),
        };
        self.phase = ph;
        let alive = t < dur;
        (s * self.gl, s * self.gr, alive)
    }

    fn next(&mut self, sr: f32) -> (f32, f32, bool) {
        if self.delay > 0 {
            self.delay -= 1;
            return (0.0, 0.0, true);
        }
        if self.drum != 0 {
            return self.drum_next(sr);
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
            semis += self.vib_depth * (tsec * 5.5 * std::f32::consts::TAU).sin();
        }
        let f = self.f0 * 2f32.powf(semis / 12.0);
        self.phase += f / sr;
        if self.phase > 1.0 {
            self.phase -= 1.0;
        }
        let mut s = 0.0;
        let mut alive = false;
        let ph = self.phase * std::f32::consts::TAU;
        for k in 0..H {
            let a = self.env[k];
            if a < 1e-5 {
                continue;
            }
            alive = true;
            s += a * (ph * (k + 1) as f32).sin();
            self.env[k] = a * self.dec[k];
        }
        if self.click {
            s *= 1.5;
        }
        // Pick noise at the attack.
        if self.noise > 1e-4 {
            self.rng ^= self.rng << 13;
            self.rng ^= self.rng >> 17;
            self.rng ^= self.rng << 5;
            let n = (self.rng as f32 / u32::MAX as f32) * 2.0 - 1.0;
            self.lp += 0.35 * (n - self.lp);
            s += self.lp * self.noise;
            self.noise *= self.noise_dec;
            alive = true;
        }
        if self.drive > 0.0 {
            s = (s * self.drive * 3.0).tanh() * 0.45;
        }
        // Envelope.
        let mut amp = 1.0;
        if self.t < self.attack {
            amp = self.t as f32 / self.attack.max(1) as f32;
        }
        if self.t >= self.len {
            self.release *= 0.9985;
            if self.release < 0.001 {
                alive = false;
            }
        }
        amp *= self.release;
        self.t += 1;
        (s * amp * self.gl, s * amp * self.gr, alive)
    }
}

pub struct PlayerState {
    pub playing: bool,
    pub pos: u64,
    pub end: u64,
    pub events: Vec<NoteEvent>,
    pub next_ev: usize,
    voices: Vec<Voice>,
    pub sample_rate: f32,
    pub master: f32,
    pub loop_range: Option<(u64, u64)>,
    /// Voices currently sounding (track, string, fret) for the fretboard.
    pub sounding: Vec<(usize, u8, u8)>,
    /// One-shot preview notes played even when stopped.
    pub preview: Vec<NoteEvent>,
}

impl PlayerState {
    fn render(&mut self, out: &mut [f32], channels: usize) {
        let sr = self.sample_rate;
        for e in self.preview.drain(..) {
            let mut v = Voice::new(&e, sr);
            v.delay = e.start;
            self.voices.push(v);
        }
        let frames = out.len() / channels.max(1);
        for i in 0..frames {
            if self.playing {
                while self.next_ev < self.events.len() && self.events[self.next_ev].start <= self.pos {
                    let e = self.events[self.next_ev].clone();
                    // Cut a previous note on the same string of the same track.
                    if e.timbre != Timbre::ClickHigh && e.timbre != Timbre::ClickLow {
                        for v in &mut self.voices {
                            if v.track == e.track && v.string == e.string && !v.click && v.t < v.len {
                                v.len = v.t;
                            }
                        }
                    }
                    self.voices.push(Voice::new(&e, sr));
                    self.next_ev += 1;
                }
                self.pos += 1;
                if let Some((a, b)) = self.loop_range {
                    if self.pos >= b {
                        self.pos = a;
                        self.next_ev = self.events.partition_point(|e| e.start < a);
                        for v in &mut self.voices {
                            if v.len > v.t {
                                v.len = v.t;
                            }
                        }
                    }
                } else if self.pos >= self.end {
                    self.playing = false;
                }
            }
            let (mut l, mut r) = (0.0f32, 0.0f32);
            self.voices.retain_mut(|v| {
                let (a, b, alive) = v.next(sr);
                l += a;
                r += b;
                alive
            });
            l = (l * self.master).tanh();
            r = (r * self.master).tanh();
            let base = i * channels;
            if channels == 1 {
                out[base] = (l + r) * 0.5;
            } else {
                out[base] = l;
                out[base + 1] = r;
                for c in 2..channels {
                    out[base + c] = 0.0;
                }
            }
        }
        self.sounding = self
            .voices
            .iter()
            .filter(|v| !v.click && v.t < v.len)
            .map(|v| (v.track, v.string, v.fret))
            .collect();
    }

    pub fn stop_all(&mut self) {
        self.playing = false;
        for v in &mut self.voices {
            if v.len > v.t {
                v.len = v.t;
            }
        }
    }
}

pub struct Audio {
    pub state: Arc<Mutex<PlayerState>>,
    _stream: Option<cpal::Stream>,
    pub error: Option<String>,
}

impl Audio {
    pub fn new() -> Audio {
        let state = Arc::new(Mutex::new(PlayerState {
            playing: false,
            pos: 0,
            end: 0,
            events: vec![],
            next_ev: 0,
            voices: vec![],
            sample_rate: 44100.0,
            master: 0.9,
            loop_range: None,
            sounding: vec![],
            preview: vec![],
        }));
        let mut a = Audio {
            state,
            _stream: None,
            error: None,
        };
        match a.start() {
            Ok(s) => a._stream = Some(s),
            Err(e) => a.error = Some(e),
        }
        a
    }

    /// Without a sound device (used for screenshots and tests).
    pub fn silent() -> Audio {
        let mut a = Audio::new_uninit();
        a.error = Some("Sound is off".into());
        a
    }

    fn new_uninit() -> Audio {
        Audio {
            state: Arc::new(Mutex::new(PlayerState {
                playing: false,
                pos: 0,
                end: 0,
                events: vec![],
                next_ev: 0,
                voices: vec![],
                sample_rate: 44100.0,
                master: 0.9,
                loop_range: None,
                sounding: vec![],
                preview: vec![],
            })),
            _stream: None,
            error: None,
        }
    }

    fn start(&self) -> Result<cpal::Stream, String> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| "No sound output device found".to_string())?;
        let cfg = device.default_output_config().map_err(|e| e.to_string())?;
        let channels = cfg.channels() as usize;
        let sr = cfg.sample_rate().0 as f32;
        self.state.lock().unwrap().sample_rate = sr;
        let config: cpal::StreamConfig = cfg.config();
        let st = self.state.clone();
        let err_fn = |e| eprintln!("audio error: {e}");
        let stream = match cfg.sample_format() {
            cpal::SampleFormat::F32 => device.build_output_stream(
                &config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    if let Ok(mut s) = st.lock() {
                        s.render(data, channels);
                    } else {
                        data.fill(0.0);
                    }
                },
                err_fn,
                None,
            ),
            cpal::SampleFormat::I16 => {
                let mut buf: Vec<f32> = vec![];
                device.build_output_stream(
                    &config,
                    move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                        buf.resize(data.len(), 0.0);
                        if let Ok(mut s) = st.lock() {
                            s.render(&mut buf, channels);
                        }
                        for (d, x) in data.iter_mut().zip(buf.iter()) {
                            *d = (x.clamp(-1.0, 1.0) * 32767.0) as i16;
                        }
                    },
                    err_fn,
                    None,
                )
            }
            cpal::SampleFormat::U16 => {
                let mut buf: Vec<f32> = vec![];
                device.build_output_stream(
                    &config,
                    move |data: &mut [u16], _: &cpal::OutputCallbackInfo| {
                        buf.resize(data.len(), 0.0);
                        if let Ok(mut s) = st.lock() {
                            s.render(&mut buf, channels);
                        }
                        for (d, x) in data.iter_mut().zip(buf.iter()) {
                            *d = ((x.clamp(-1.0, 1.0) * 0.5 + 0.5) * 65535.0) as u16;
                        }
                    },
                    err_fn,
                    None,
                )
            }
            f => return Err(format!("Unsupported sample format {f:?}")),
        }
        .map_err(|e| e.to_string())?;
        stream.play().map_err(|e| e.to_string())?;
        Ok(stream)
    }

    pub fn sample_rate(&self) -> f32 {
        self.state.lock().map(|s| s.sample_rate).unwrap_or(44100.0)
    }
}

/// One played bar in the schedule, used to map the playing position back to the score.
#[derive(Clone, Debug)]
pub struct TimelineBar {
    pub start: u64,
    pub bar: usize,
    pub ticks: u32,
    pub samples_per_tick: f64,
}

pub struct Schedule {
    pub events: Vec<NoteEvent>,
    pub timeline: Vec<TimelineBar>,
    pub end: u64,
    pub start_offset: u64,
}

pub struct ScheduleOptions {
    pub from_bar: usize,
    pub metronome: bool,
    pub count_in: bool,
    pub speed: f32,
    pub only_bars: Option<(usize, usize)>,
}

fn freq_of(midi: f32) -> f32 {
    440.0 * 2f32.powf((midi - 69.0) / 12.0)
}

pub fn note_preview(track: &Track, ti: usize, n: &Note, sr: f32) -> NoteEvent {
    let p = track.pitch(n) as f32;
    NoteEvent {
        start: 0,
        len: (sr * 0.6) as u64,
        freq: freq_of(if n.harmonic { p + 12.0 } else { p }),
        bend: 0.0,
        slide: 0.0,
        vibrato: false,
        vel: 0.8 * track.volume.max(0.3),
        pan: track.pan,
        timbre: Timbre::of(track.instrument),
        muted: n.palm_mute,
        dead: n.dead,
        harmonic: n.harmonic,
        soft_attack: false,
        track: ti,
        string: n.string,
        fret: n.fret,
        bend_shape: 0,
        slide_in: 0.0,
        wide_vibrato: false,
        fade_in: false,
    }
}

/// Turns the song into timed note events.
/// Turns the song into timed note events, for every voice of every track.
pub fn build_schedule(song: &Song, sr: f32, opt: &ScheduleOptions) -> Schedule {
    let mut sch = build_voice_schedule(song, sr, opt);
    let any_solo = song.tracks.iter().any(|t| t.solo);
    for ti in 0..song.tracks.len() {
        let tr = &song.tracks[ti];
        if tr.mute || (any_solo && !tr.solo) {
            continue;
        }
        for v in 1..VOICES {
            if !tr.bars.iter().any(|b| b.voice_used(v)) {
                continue;
            }
            // Schedule this voice on its own, as if it were voice 1 of a lone track.
            let mut s2 = song.clone();
            s2.swap_in(ti, v);
            for (k, t) in s2.tracks.iter_mut().enumerate() {
                t.mute = k != ti;
                t.solo = false;
            }
            let opt2 = ScheduleOptions { from_bar: opt.from_bar, metronome: false, count_in: opt.count_in, speed: opt.speed, only_bars: opt.only_bars };
            let extra = build_voice_schedule(&s2, sr, &opt2);
            sch.events.extend(extra.events.into_iter().filter(|e| !matches!(e.timbre, Timbre::ClickHigh | Timbre::ClickLow)));
        }
    }
    sch.events.sort_by_key(|e| e.start);
    sch
}

fn build_voice_schedule(song: &Song, sr: f32, opt: &ScheduleOptions) -> Schedule {
    let mut events = vec![];
    let mut timeline = vec![];
    let speed = opt.speed.clamp(0.1, 4.0) as f64;
    let any_solo = song.tracks.iter().any(|t| t.solo);
    let order: Vec<usize> = match opt.only_bars {
        Some((a, b)) => (a..=b.min(song.bar_count().saturating_sub(1))).collect(),
        None => {
            let full = song.play_order();
            // Start from the first occurrence of from_bar.
            let p = full.iter().position(|&b| b == opt.from_bar).unwrap_or(0);
            full[p..].to_vec()
        }
    };
    let mut t: u64 = 0;
    // Count-in bar.
    if opt.count_in {
        if let Some(&b0) = order.first() {
            let mb = &song.master[b0];
            let spt = sr as f64 * 60.0 / (song.tempo_at(b0) as f64 * speed) / QUARTER as f64;
            let beat_ticks = QUARTER * 4 / mb.den.max(1) as u32;
            for i in 0..mb.num as u32 {
                events.push(click(t + (i as f64 * beat_ticks as f64 * spt) as u64, i == 0, sr));
            }
            t += (mb.ticks() as f64 * spt) as u64;
        }
    }
    let start_offset = t;
    for &bi in &order {
        let mb = &song.master[bi];
        let spt = sr as f64 * 60.0 / (song.tempo_at(bi) as f64 * speed) / QUARTER as f64;
        let bar_ticks = mb.ticks();
        timeline.push(TimelineBar {
            start: t,
            bar: bi,
            ticks: bar_ticks,
            samples_per_tick: spt,
        });
        if opt.metronome {
            let beat_ticks = QUARTER * 4 / mb.den.max(1) as u32;
            for i in 0..mb.num as u32 {
                events.push(click(t + (i as f64 * beat_ticks as f64 * spt) as u64, i == 0, sr));
            }
        }
        for (ti, track) in song.tracks.iter().enumerate() {
            let audible = !track.mute && (!any_solo || track.solo);
            if !audible {
                continue;
            }
            let bar = &track.bars[bi];
            let mut tick: u32 = 0;
            for (bj, beat) in bar.beats.iter().enumerate() {
                let bt = beat.ticks();
                let nstr = beat.notes.len();
                // Swing (triplet feel) for plain eighth notes.
                let (mut on_tick, mut swing_len) = (tick as f64, 0.0f64);
                if mb.triplet_feel && beat.dur == Dur::Eighth && beat.tuplet == 0 && !beat.dotted {
                    if tick % QUARTER == QUARTER / 2 {
                        on_tick += 160.0;
                        swing_len = -160.0;
                    } else if tick % QUARTER == 0 {
                        swing_len = 160.0;
                    }
                }
                // Crescendo / decrescendo across a run of beats.
                let mut hair = 1.0f32;
                if beat.hairpin != 0 {
                    let mut a = bj;
                    while a > 0 && bar.beats[a - 1].hairpin == beat.hairpin {
                        a -= 1;
                    }
                    let mut b = bj;
                    while b + 1 < bar.beats.len() && bar.beats[b + 1].hairpin == beat.hairpin {
                        b += 1;
                    }
                    let pos = if b > a { (bj - a) as f32 / (b - a) as f32 } else { 0.5 };
                    hair = if beat.hairpin == 1 { 0.75 + 0.5 * pos } else { 1.25 - 0.5 * pos };
                }
                for (k, n) in beat.notes.iter().enumerate() {
                    if n.tie {
                        continue;
                    }
                    let mut len_ticks = bt as f64 + swing_len;
                    // Extend through tied notes in following beats.
                    let mut nb = bj + 1;
                    let mut cur_bar = bi;
                    loop {
                        let beats = &song.tracks[ti].bars[cur_bar].beats;
                        if nb >= beats.len() {
                            if cur_bar + 1 < song.bar_count() {
                                cur_bar += 1;
                                nb = 0;
                                continue;
                            }
                            break;
                        }
                        match beats[nb].note_on(n.string) {
                            Some(x) if x.tie => {
                                len_ticks += beats[nb].ticks() as f64;
                                nb += 1;
                            }
                            _ => break,
                        }
                    }
                    let next = next_note_on_string(song, ti, bi, bj, n.string);
                    let mut slide = 0.0;
                    if n.slide {
                        if let Some(nx) = &next {
                            slide = track.pitch(nx) as f32 - track.pitch(n) as f32;
                        }
                    }
                    if n.slide_out && slide == 0.0 {
                        slide = -5.0;
                    }
                    let soft = prev_hammer(song, ti, bi, bj, n.string);
                    if n.let_ring || track.auto_let_ring {
                        len_ticks = len_ticks.max(QUARTER as f64 * 4.0);
                    }
                    if beat.fermata {
                        len_ticks *= 1.6;
                    }
                    let stroke = if beat.stroke == Stroke::None && track.auto_brush && beat.notes.len() > 2 { Stroke::Down } else { beat.stroke };
                    let stroke_off = match stroke {
                        Stroke::None => 0.0,
                        Stroke::Down => (nstr - 1 - k.min(nstr - 1)) as f64,
                        Stroke::Up => k as f64,
                    } * sr as f64
                        * 0.012;
                    let mut p = track.pitch(n) as f32;
                    if n.harmonic {
                        p += 12.0;
                    }
                    if n.pinch_harmonic {
                        p += 19.0;
                    }
                    let mut vel = beat.dynamic.velocity() as f32 / 100.0 * track.volume * hair;
                    if n.ghost {
                        vel *= 0.5;
                    }
                    if n.accent {
                        vel *= 1.3;
                    }
                    if n.heavy_accent {
                        vel *= 1.45;
                    }
                    if beat.technique >= 2 {
                        vel *= 1.15;
                    }
                    if tick == 0 && track.accentuation > 0.0 {
                        vel *= 1.0 + 0.4 * track.accentuation;
                    }
                    if soft {
                        vel *= 0.8;
                    }
                    let start = t + (on_tick * spt + stroke_off) as u64;
                    let base = NoteEvent {
                        start,
                        len: 1,
                        freq: freq_of(p),
                        bend: n.bend as f32 / 2.0,
                        slide,
                        vibrato: n.vibrato || n.wide_vibrato,
                        vel,
                        pan: track.pan,
                        timbre: Timbre::of(track.instrument),
                        muted: n.palm_mute || track.palm_mute >= 0.5,
                        dead: n.dead,
                        harmonic: n.harmonic || n.pinch_harmonic,
                        soft_attack: soft || beat.technique == 1 || track.playing_style == 2,
                        track: ti,
                        string: n.string,
                        fret: n.fret,
                        bend_shape: n.bend_kind,
                        slide_in: if n.slide_in { -4.0 } else { 0.0 },
                        wide_vibrato: n.wide_vibrato,
                        fade_in: beat.fade_in,
                    };
                    // Grace note just before the beat.
                    if let Some(g) = n.grace {
                        let gl = (QUARTER as f64 / 8.0 * spt) as u64;
                        let gp = (track.tuning.get(n.string as usize).copied().unwrap_or(40) as i32 + g as i32 + track.capo_at(n.string as usize) as i32) as f32;
                        events.push(NoteEvent {
                            start: start.saturating_sub(gl),
                            len: gl.max(1),
                            freq: freq_of(gp),
                            bend: 0.0,
                            slide: 0.0,
                            vibrato: false,
                            slide_in: 0.0,
                            fade_in: false,
                            bend_shape: 0,
                            fret: g,
                            vel: vel * 0.8,
                            ..base.clone()
                        });
                    }
                    let mut total = (len_ticks * spt) as u64;
                    if n.staccato {
                        total /= 2;
                    }
                    if beat.tremolo || n.trill > 0 {
                        // Repeated notes (tremolo picking) or alternating notes (trill).
                        let step_ticks = if n.trill > 0 { QUARTER as f64 / 8.0 } else { QUARTER as f64 / 4.0 };
                        let step = ((step_ticks * spt) as u64).max(1);
                        let trill_p = (track.tuning.get(n.string as usize).copied().unwrap_or(40) as i32 + n.trill as i32 + track.capo_at(n.string as usize) as i32) as f32;
                        let total = total.max(1);
                        let mut at = 0u64;
                        let mut i = 0;
                        while at < total {
                            let alt = n.trill > 0 && i % 2 == 1;
                            events.push(NoteEvent {
                                start: start + at,
                                len: step.min(total - at).max(1),
                                freq: if alt { freq_of(trill_p) } else { base.freq },
                                fret: if alt { n.trill } else { n.fret },
                                soft_attack: n.trill > 0 && i > 0,
                                vel: if n.trill > 0 && i > 0 { vel * 0.85 } else { vel },
                                ..base.clone()
                            });
                            at += step;
                            i += 1;
                        }
                    } else {
                        events.push(NoteEvent { len: total.max(1), ..base });
                    }
                }
                tick += bt;
            }
        }
        t += (bar_ticks as f64 * spt) as u64;
    }
    events.sort_by_key(|e| e.start);
    Schedule {
        events,
        timeline,
        end: t + (sr * 0.5) as u64,
        start_offset,
    }
}

fn click(at: u64, strong: bool, sr: f32) -> NoteEvent {
    NoteEvent {
        start: at,
        len: (sr * 0.03) as u64,
        freq: if strong { 1760.0 } else { 1320.0 },
        bend: 0.0,
        slide: 0.0,
        vibrato: false,
        vel: if strong { 0.6 } else { 0.4 },
        pan: 0.0,
        timbre: if strong { Timbre::ClickHigh } else { Timbre::ClickLow },
        muted: false,
        dead: false,
        harmonic: false,
        soft_attack: false,
        track: usize::MAX,
        string: 255,
        fret: 0,
        bend_shape: 0,
        slide_in: 0.0,
        wide_vibrato: false,
        fade_in: false,
    }
}

fn next_note_on_string(song: &Song, ti: usize, bi: usize, bj: usize, s: u8) -> Option<Note> {
    let tr = &song.tracks[ti];
    let mut b = bi;
    let mut j = bj + 1;
    for _ in 0..2 {
        while j < tr.bars[b].beats.len() {
            if let Some(n) = tr.bars[b].beats[j].note_on(s) {
                return Some(n.clone());
            }
            if !tr.bars[b].beats[j].is_rest() {
                return None;
            }
            j += 1;
        }
        if b + 1 >= tr.bars.len() {
            return None;
        }
        b += 1;
        j = 0;
    }
    None
}

fn prev_hammer(song: &Song, ti: usize, bi: usize, bj: usize, s: u8) -> bool {
    let tr = &song.tracks[ti];
    let prev = if bj > 0 {
        tr.bars[bi].beats.get(bj - 1)
    } else if bi > 0 {
        tr.bars[bi - 1].beats.last()
    } else {
        None
    };
    prev.and_then(|b| b.note_on(s)).map(|n| n.hammer || n.slide).unwrap_or(false)
}

/// Renders the whole song with the built-in sound (stereo, interleaved).
pub fn render_song(song: &Song, sr: f32, speed: f32) -> Vec<f32> {
    let opt = ScheduleOptions { from_bar: 0, metronome: false, count_in: false, speed, only_bars: None };
    let sch = build_schedule(song, sr, &opt);
    let end = sch.end + (sr * 2.0) as u64;
    let mut st = PlayerState {
        playing: true,
        pos: 0,
        end: sch.end,
        events: sch.events,
        next_ev: 0,
        voices: vec![],
        sample_rate: sr,
        master: 0.9,
        loop_range: None,
        sounding: vec![],
        preview: vec![],
    };
    let frames = end.min((sr * 60.0 * 30.0) as u64) as usize;
    let mut out = vec![0.0f32; frames * 2];
    for chunk in out.chunks_mut(2048) {
        st.render(chunk, 2);
    }
    // Trim the silent tail.
    let last = out.iter().rposition(|x| x.abs() > 1e-4).unwrap_or(0);
    out.truncate(((last / 2) + 1 + (sr * 0.3) as usize) * 2);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_song_makes_sound() {
        let song = Song::demo();
        let sr = 44100.0;
        let opt = ScheduleOptions { from_bar: 0, metronome: true, count_in: false, speed: 1.0, only_bars: None };
        let sch = build_schedule(&song, sr, &opt);
        assert!(!sch.events.is_empty());
        // 16 bars played (4 repeated twice + 8).
        assert_eq!(sch.timeline.len(), 16);
        // A second voice adds its notes.
        let mut s2 = song.clone();
        s2.tracks[0].bars[0].pad_voices();
        s2.tracks[0].bars[0].voices[0] = vec![Beat { notes: vec![Note::new(0, 12)], ..Beat::rest(Dur::Whole) }];
        let sch2 = build_schedule(&s2, sr, &opt);
        assert!(sch2.events.len() > sch.events.len());
        let mut st = PlayerState {
            playing: true,
            pos: 0,
            end: sch.end,
            events: sch.events,
            next_ev: 0,
            voices: vec![],
            sample_rate: sr,
            master: 0.9,
            loop_range: None,
            sounding: vec![],
            preview: vec![],
        };
        let mut buf = vec![0.0f32; 44100 * 2 * 3];
        for chunk in buf.chunks_mut(1024) {
            st.render(chunk, 2);
        }
        let peak = buf.iter().fold(0.0f32, |m, x| m.max(x.abs()));
        assert!(buf.iter().all(|x| x.is_finite()));
        assert!(peak > 0.05, "too quiet: {peak}");
        assert!(peak <= 1.0);
    }

    #[test]
    fn loop_wraps_and_keeps_playing() {
        let song = Song::demo();
        let sr = 8000.0;
        let opt = ScheduleOptions { from_bar: 0, metronome: false, count_in: false, speed: 1.0, only_bars: Some((4, 5)) };
        let sch = build_schedule(&song, sr, &opt);
        assert_eq!(sch.timeline.iter().map(|t| t.bar).collect::<Vec<_>>(), vec![4, 5]);
        let t0 = &sch.timeline[0];
        // Loop the second half of bar 5 (ticks 1920..3840).
        let a = t0.start + (1920.0 * t0.samples_per_tick) as u64;
        let b = t0.start + (3840.0 * t0.samples_per_tick) as u64;
        let mut st = PlayerState {
            playing: true,
            pos: a,
            end: sch.end,
            next_ev: sch.events.partition_point(|e| e.start < a),
            events: sch.events,
            voices: vec![],
            sample_rate: sr,
            master: 0.9,
            loop_range: Some((a, b)),
            sounding: vec![],
            preview: vec![],
        };
        let mut buf = vec![0.0f32; ((b - a) as usize * 3) * 2];
        st.render(&mut buf, 2);
        assert!(st.playing);
        assert!(st.pos >= a && st.pos < b, "position {} should stay inside {a}..{b}", st.pos);
    }

    #[test]
    fn midi_and_text_export() {
        let song = Song::demo();
        let m = crate::fileio::export_midi(&song);
        assert_eq!(&m[0..4], b"MThd");
        let t = crate::fileio::export_ascii(&song, 0);
        assert!(t.contains("Morning Walk"));
        let j = serde_json::to_string(&song).unwrap();
        let mut back: Song = serde_json::from_str(&j).unwrap();
        back.normalize();
        assert_eq!(back, song);
    }
}
