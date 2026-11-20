//! The sequencer: turns a song into timed notes and plays them with the built-in
//! instruments (see synth.rs), so songs can be heard without any sound bank.

use crate::model::*;
use crate::synth::Voice;
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
        matches!(
            self,
            Timbre::Organ
                | Timbre::Synth
                | Timbre::Voice
                | Timbre::Bowed
                | Timbre::Brass
                | Timbre::Reed
                | Timbre::Flute
        )
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
                while self.next_ev < self.events.len()
                    && self.events[self.next_ev].start <= self.pos
                {
                    let e = self.events[self.next_ev].clone();
                    // Cut a previous note on the same string of the same track.
                    if e.timbre != Timbre::ClickHigh && e.timbre != Timbre::ClickLow {
                        for v in &mut self.voices {
                            if v.track == e.track && v.string == e.string && !v.click && v.t < v.len
                            {
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
    /// Sound through the named output device (the system's default when it is None
    /// or no longer connected), with a fixed buffer size in samples when one is given.
    pub fn with_output(device: Option<&str>, buffer: Option<u32>) -> Audio {
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
        // A buffer size the device refuses falls back to the device's own.
        let tried = a.start(device, buffer).or_else(|e| if buffer.is_some() { a.start(device, None) } else { Err(e) });
        match tried {
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

    fn start(&self, name: Option<&str>, buffer: Option<u32>) -> Result<cpal::Stream, String> {
        let host = cpal::default_host();
        let chosen = name.and_then(|n| {
            host.output_devices().ok()?.find(|d| d.name().map(|x| x == n).unwrap_or(false))
        });
        let device = chosen
            .or_else(|| host.default_output_device())
            .ok_or_else(|| "No sound output device found".to_string())?;
        let cfg = device.default_output_config().map_err(|e| e.to_string())?;
        let channels = cfg.channels() as usize;
        let sr = cfg.sample_rate().0 as f32;
        self.state.lock().unwrap().sample_rate = sr;
        let mut config: cpal::StreamConfig = cfg.config();
        if let Some(b) = buffer {
            config.buffer_size = cpal::BufferSize::Fixed(b);
        }
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

    /// The names of the sound output devices that are connected now.
    pub fn output_device_names() -> Vec<String> {
        cpal::default_host()
            .output_devices()
            .map(|it| it.filter_map(|d| d.name().ok()).collect())
            .unwrap_or_default()
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
            let opt2 = ScheduleOptions {
                from_bar: opt.from_bar,
                metronome: false,
                count_in: opt.count_in,
                speed: opt.speed,
                only_bars: opt.only_bars,
            };
            let extra = build_voice_schedule(&s2, sr, &opt2);
            sch.events.extend(
                extra
                    .events
                    .into_iter()
                    .filter(|e| !matches!(e.timbre, Timbre::ClickHigh | Timbre::ClickLow)),
            );
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
    if opt.count_in
        && let Some(&b0) = order.first()
    {
        let mb = &song.master[b0];
        let spt = sr as f64 * 60.0 / (song.tempo_at(b0) as f64 * speed) / QUARTER as f64;
        let beat_ticks = QUARTER * 4 / mb.den.max(1) as u32;
        for i in 0..mb.num as u32 {
            events.push(click(
                t + (i as f64 * beat_ticks as f64 * spt) as u64,
                i == 0,
                sr,
            ));
        }
        t += (mb.ticks() as f64 * spt) as u64;
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
                events.push(click(
                    t + (i as f64 * beat_ticks as f64 * spt) as u64,
                    i == 0,
                    sr,
                ));
            }
        }
        for (ti, track) in song.tracks.iter().enumerate() {
            let audible = !track.mute && (!any_solo || track.solo);
            if !audible {
                continue;
            }
            // A bar repeat sign plays the bar (or two) it stands for.
            let bar = simile_source(track, bi);
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
                    } else if tick.is_multiple_of(QUARTER) {
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
                    let pos = if b > a {
                        (bj - a) as f32 / (b - a) as f32
                    } else {
                        0.5
                    };
                    hair = if beat.hairpin == 1 {
                        0.75 + 0.5 * pos
                    } else {
                        1.25 - 0.5 * pos
                    };
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
                    if n.slide
                        && let Some(nx) = &next
                    {
                        slide = track.pitch(nx) as f32 - track.pitch(n) as f32;
                    }
                    if n.slide_out && slide == 0.0 {
                        slide = -5.0;
                    }
                    let soft = prev_hammer(song, ti, bi, bj, n.string) || n.fx.lh_tap;
                    if n.let_ring || track.auto_let_ring {
                        len_ticks = len_ticks.max(QUARTER as f64 * 4.0);
                    }
                    if beat.fermata {
                        len_ticks *= 1.6;
                    }
                    let stroke = if beat.stroke == Stroke::None
                        && track.auto_brush
                        && beat.notes.len() > 2
                    {
                        Stroke::Down
                    } else {
                        beat.stroke
                    };
                    // An arpeggio spreads the notes over its duration (480 to the
                    // quarter note), starting on the beat or ending on it.
                    let arp_total = if beat.fx.arp_len == 0 { 480.0 } else { beat.fx.arp_len as f64 } * 2.0 * spt;
                    let arp_step = if nstr > 1 { arp_total / (nstr - 1) as f64 } else { 0.0 };
                    let arp_shift = -(beat.fx.arp_falls as f64) * arp_total;
                    let stroke_off = match beat.fx.arpeggio {
                        1 => (nstr - 1 - k.min(nstr - 1)) as f64 * arp_step + arp_shift,
                        2 => k as f64 * arp_step + arp_shift,
                        _ => {
                            (match stroke {
                                Stroke::None => 0.0,
                                Stroke::Down => (nstr - 1 - k.min(nstr - 1)) as f64,
                                Stroke::Up => k as f64,
                            }) * sr as f64
                                * 0.012
                        }
                    };
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
                    let start = (t as f64 + on_tick * spt + stroke_off).max(0.0) as u64;
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
                        fade_in: beat.fade_in || beat.fx.swell,
                    };
                    // Grace note just before the beat.
                    if let Some(g) = n.grace {
                        let gl = (QUARTER as f64 / 8.0 * spt) as u64;
                        let gp = (track.tuning.get(n.string as usize).copied().unwrap_or(40) as i32
                            + g as i32
                            + track.capo_at(n.string as usize) as i32)
                            as f32;
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
                        let step_ticks = if n.trill > 0 {
                            QUARTER as f64 / 8.0
                        } else {
                            QUARTER as f64 / 4.0
                        };
                        let step = ((step_ticks * spt) as u64).max(1);
                        let trill_p = (track.tuning.get(n.string as usize).copied().unwrap_or(40)
                            as i32
                            + n.trill as i32
                            + track.capo_at(n.string as usize) as i32)
                            as f32;
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
                                vel: if n.trill > 0 && i > 0 {
                                    vel * 0.85
                                } else {
                                    vel
                                },
                                ..base.clone()
                            });
                            at += step;
                            i += 1;
                        }
                    } else {
                        events.push(NoteEvent {
                            len: total.max(1),
                            ..base
                        });
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
        timbre: if strong {
            Timbre::ClickHigh
        } else {
            Timbre::ClickLow
        },
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
    // Look through the rest of this bar and the whole of the next one.
    let mut start = bj + 1;
    for bar in tr.bars.iter().skip(bi).take(2) {
        for beat in bar.beats.iter().skip(start) {
            if let Some(n) = beat.note_on(s) {
                return Some(n.clone());
            }
            if !beat.is_rest() {
                return None;
            }
        }
        start = 0;
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
    prev.and_then(|b| b.note_on(s))
        .map(|n| n.hammer || n.slide || n.fx.legato)
        .unwrap_or(false)
}

/// The bar whose notes bar `bi` plays: itself, or the one a repeat sign stands for.
fn simile_source(track: &Track, bi: usize) -> &Bar {
    let mut b = bi;
    for _ in 0..16 {
        match track.bars[b].simile {
            1 if b >= 1 => b -= 1,
            2 if b >= 2 => b -= 2,
            _ => break,
        }
    }
    &track.bars[b]
}

/// Renders the whole song with the built-in sound (stereo, interleaved).
pub fn render_song(song: &Song, sr: f32, speed: f32) -> Vec<f32> {
    let opt = ScheduleOptions {
        from_bar: 0,
        metronome: false,
        count_in: false,
        speed,
        only_bars: None,
    };
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
        let opt = ScheduleOptions {
            from_bar: 0,
            metronome: true,
            count_in: false,
            speed: 1.0,
            only_bars: None,
        };
        let sch = build_schedule(&song, sr, &opt);
        assert!(!sch.events.is_empty());
        // 16 bars played (4 repeated twice + 8).
        assert_eq!(sch.timeline.len(), 16);
        // A second voice adds its notes.
        let mut s2 = song.clone();
        s2.tracks[0].bars[0].pad_voices();
        s2.tracks[0].bars[0].voices[0] = vec![Beat {
            notes: vec![Note::new(0, 12)],
            ..Beat::rest(Dur::Whole)
        }];
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
    fn the_whole_demo_song_renders_much_faster_than_it_plays() {
        let song = Song::demo();
        let t0 = std::time::Instant::now();
        let out = render_song(&song, 44100.0, 1.0);
        let took = t0.elapsed().as_secs_f32();
        let secs = out.len() as f32 / 2.0 / 44100.0;
        println!("rendered {secs:.1}s of music in {took:.2}s");
        assert!(out.iter().all(|x| x.is_finite()));
        let peak = out.iter().fold(0.0f32, |m, x| m.max(x.abs()));
        println!("peak {peak:.3}");
        assert!(peak > 0.1 && peak <= 1.0);
        assert!(took < secs / 3.0, "too slow: {took}s for {secs}s");
    }

    #[test]
    fn loop_wraps_and_keeps_playing() {
        let song = Song::demo();
        let sr = 8000.0;
        let opt = ScheduleOptions {
            from_bar: 0,
            metronome: false,
            count_in: false,
            speed: 1.0,
            only_bars: Some((4, 5)),
        };
        let sch = build_schedule(&song, sr, &opt);
        assert_eq!(
            sch.timeline.iter().map(|t| t.bar).collect::<Vec<_>>(),
            vec![4, 5]
        );
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
        assert!(
            st.pos >= a && st.pos < b,
            "position {} should stay inside {a}..{b}",
            st.pos
        );
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
