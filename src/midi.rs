//! MIDI devices: listing the ports, keeping the chosen output and input open, and playing
//! notes to the output. See `docs/midi-design.md`.
//!
//! The output belongs to a thread of its own. The audio thread (which must never wait) and the
//! screen only drop `MidiCmd`s into a bounded channel; the MIDI thread sends them when due.

use crate::model::{InstrumentType, Track};
use midir::{MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, SyncSender, sync_channel};
use std::time::{Duration, Instant};

const CLIENT: &str = "GuitarCraft";

/// The channel (0-based) that carries drums.
pub const DRUM_CHANNEL: u8 = 9;

/// The MIDI channel of a track: the drum channel for drums, otherwise one channel for each
/// track in turn, skipping the drum channel (tracks past the 15th share channels).
pub fn channel_for(track: usize, drums: bool) -> u8 {
    if drums {
        return DRUM_CHANNEL;
    }
    let c = (track % 15) as u8;
    if c >= DRUM_CHANNEL { c + 1 } else { c }
}

/// The names of the MIDI output ports connected now.
pub fn output_port_names() -> Vec<String> {
    match MidiOutput::new(CLIENT) {
        Ok(m) => m.ports().iter().filter_map(|p| m.port_name(p).ok()).collect(),
        Err(_) => vec![],
    }
}

/// The names of the MIDI input ports connected now.
pub fn input_port_names() -> Vec<String> {
    match MidiInput::new(CLIENT) {
        Ok(m) => m.ports().iter().filter_map(|p| m.port_name(p).ok()).collect(),
        Err(_) => vec![],
    }
}

/// What the MIDI thread is asked to do.
#[derive(Debug)]
pub enum MidiCmd {
    /// Play a note: it starts `delay` seconds from now and stops `dur` seconds after that.
    /// `bend` is a pitch curve as (seconds from the start, semitones) points joined by straight
    /// lines, held after the last point; it is empty for a note that doesn't bend.
    Note { ch: u8, pitch: u8, vel: u8, delay: f32, dur: f32, bend: Vec<(f32, f32)> },
    /// Send these bytes at once (program, volume and pan setup).
    Raw(Vec<u8>),
    /// Silence everything (playback stopped, paused or looped back).
    AllOff,
    /// Close the connection and end the thread.
    Quit,
}

/// The pitch wheel range the channels are set to (semitones each way).
pub const BEND_RANGE: f32 = 12.0;

/// The pitch bend message value for a bend in semitones (8192 is no bend).
pub fn bend_value(semis: f32) -> u16 {
    (8192.0 + semis / BEND_RANGE * 8192.0).round().clamp(0.0, 16383.0) as u16
}

/// A bend as the built-in sound plays it: (seconds, semitones) points, for the shapes
/// 0 bend up, 1 bend and release, 2 pre-bend, 3 pre-bend and release.
pub fn bend_curve(semis: f32, shape: u8, len_secs: f32) -> Vec<(f32, f32)> {
    if semis == 0.0 {
        return vec![];
    }
    let half = (len_secs * 0.5).max(0.05);
    match shape {
        1 => vec![(0.0, 0.0), (half, semis), (half * 2.0, 0.0)],
        2 => vec![(0.0, semis)],
        3 => vec![(0.0, semis), (half, 0.0)],
        _ => vec![(0.0, 0.0), (half, semis)],
    }
}

/// The pitch of a curve `t` seconds in.
fn curve_at(points: &[(f32, f32)], t: f32) -> f32 {
    let Some(&(t0, v0)) = points.first() else {
        return 0.0;
    };
    if t <= t0 {
        return v0;
    }
    for w in points.windows(2) {
        let ((ta, va), (tb, vb)) = (w[0], w[1]);
        if t <= tb {
            return va + (vb - va) * (t - ta) / (tb - ta).max(1e-6);
        }
    }
    points.last().map(|p| p.1).unwrap_or(0.0)
}

/// What kind of message is waiting. The order is the order they go in when due together:
/// a note stops, the bend returns to rest, a new bend is set, then the next note starts.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Rank {
    Off,
    Reset,
    Bend,
    On,
}

/// A message waiting for its time.
struct Due {
    at: Instant,
    bytes: [u8; 3],
    rank: Rank,
}

fn send(c: &mut MidiOutputConnection, b: &[u8]) {
    // A device that went away is noticed when Preferences next connects.
    let _ = c.send(b);
}

fn silence(c: &mut MidiOutputConnection) {
    for ch in 0..16u8 {
        send(c, &[0xB0 | ch, 120, 0]); // all sound off
        send(c, &[0xB0 | ch, 123, 0]); // all notes off
        send(c, &[0xE0 | ch, 0, 0x40]); // pitch wheel back to rest
    }
}

fn run(rx: Receiver<MidiCmd>, mut out: MidiOutputConnection) {
    let mut due: Vec<Due> = vec![];
    loop {
        let wait = match due.iter().map(|d| d.at).min() {
            Some(t) => t.saturating_duration_since(Instant::now()).min(Duration::from_millis(20)),
            None => Duration::from_millis(100),
        };
        match rx.recv_timeout(wait) {
            Ok(MidiCmd::Note { ch, pitch, vel, delay, dur, bend }) => {
                let on = Instant::now() + Duration::from_secs_f32(delay.max(0.0));
                let dur = dur.max(0.01);
                // The same note still sounding stops just before this one starts.
                for d in due.iter_mut().filter(|d| d.rank == Rank::Off && d.bytes[0] & 15 == ch && d.bytes[1] == pitch) {
                    d.at = d.at.min(on);
                }
                if !bend.is_empty() {
                    // The pitch wheel follows the curve every 10 ms, then goes back to rest.
                    let end = bend.last().map(|p| p.0).unwrap_or(0.0).max(0.0).min(dur);
                    let mut t = 0.0f32;
                    loop {
                        let v = bend_value(curve_at(&bend, t));
                        due.push(Due { at: on + Duration::from_secs_f32(t), bytes: [0xE0 | ch, (v & 127) as u8, (v >> 7) as u8], rank: Rank::Bend });
                        if t >= end {
                            break;
                        }
                        t = (t + 0.01).min(end);
                    }
                    due.push(Due { at: on + Duration::from_secs_f32(dur), bytes: [0xE0 | ch, 0, 0x40], rank: Rank::Reset });
                }
                due.push(Due { at: on, bytes: [0x90 | ch, pitch, vel], rank: Rank::On });
                due.push(Due { at: on + Duration::from_secs_f32(dur), bytes: [0x80 | ch, pitch, 0], rank: Rank::Off });
            }
            Ok(MidiCmd::Raw(b)) => send(&mut out, &b),
            Ok(MidiCmd::AllOff) => {
                due.clear();
                silence(&mut out);
            }
            Ok(MidiCmd::Quit) | Err(RecvTimeoutError::Disconnected) => {
                silence(&mut out);
                break;
            }
            Err(RecvTimeoutError::Timeout) => {}
        }
        let now = Instant::now();
        let mut ready: Vec<Due> = vec![];
        let mut i = 0;
        while i < due.len() {
            if due[i].at <= now {
                ready.push(due.swap_remove(i));
            } else {
                i += 1;
            }
        }
        ready.sort_by_key(|d| (d.at, d.rank));
        for d in ready {
            send(&mut out, &d.bytes);
        }
    }
}

/// The open MIDI connections.
#[derive(Default)]
pub struct Midi {
    output: Option<SyncSender<MidiCmd>>,
    input: Option<MidiInputConnection<()>>,
    /// Counts the messages arriving from the input, so the screen can show activity.
    activity: Arc<AtomicU32>,
}

impl Midi {
    /// Opens the named output port (None closes it). Errors say why it could not open.
    pub fn set_output(&mut self, name: Option<&str>) -> Result<(), String> {
        if let Some(old) = self.output.take() {
            let _ = old.try_send(MidiCmd::Quit);
        }
        let Some(name) = name else {
            return Ok(());
        };
        let m = MidiOutput::new(CLIENT).map_err(|e| e.to_string())?;
        let port = m
            .ports()
            .into_iter()
            .find(|p| m.port_name(p).is_ok_and(|n| n == name))
            .ok_or_else(|| format!("MIDI output \"{name}\" isn't connected"))?;
        let conn = m.connect(&port, "GuitarCraft output").map_err(|e| e.to_string())?;
        let (tx, rx) = sync_channel(4096);
        std::thread::Builder::new()
            .name("midi-out".into())
            .spawn(move || run(rx, conn))
            .map_err(|e| e.to_string())?;
        self.output = Some(tx);
        Ok(())
    }

    /// Opens the named input port (None closes it).
    pub fn set_input(&mut self, name: Option<&str>) -> Result<(), String> {
        self.input = None;
        let Some(name) = name else {
            return Ok(());
        };
        let mut m = MidiInput::new(CLIENT).map_err(|e| e.to_string())?;
        // Timing clock, active sensing and SysEx are of no use here.
        m.ignore(midir::Ignore::All);
        let port = m
            .ports()
            .into_iter()
            .find(|p| m.port_name(p).is_ok_and(|n| n == name))
            .ok_or_else(|| format!("MIDI input \"{name}\" isn't connected"))?;
        let count = self.activity.clone();
        self.input = Some(
            m.connect(
                &port,
                "GuitarCraft input",
                move |_, _, _| {
                    count.fetch_add(1, Ordering::Relaxed);
                },
                (),
            )
            .map_err(|e| e.to_string())?,
        );
        Ok(())
    }

    pub fn output_open(&self) -> bool {
        self.output.is_some()
    }

    pub fn input_open(&self) -> bool {
        self.input.is_some()
    }

    /// A way for the audio thread to send notes to the output (None when none is open).
    pub fn sender(&self) -> Option<SyncSender<MidiCmd>> {
        self.output.clone()
    }

    /// How many messages the input has received so far.
    pub fn activity(&self) -> u32 {
        self.activity.load(Ordering::Relaxed)
    }

    /// Plays one note for a moment (clicking a note in the score): the instrument is chosen,
    /// the note starts now and stops by itself after `ms` milliseconds.
    pub fn preview_note(&self, channel: u8, program: u8, pitch: u8, velocity: u8, ms: u64) {
        let Some(tx) = &self.output else {
            return;
        };
        let ch = channel & 15;
        if ch != DRUM_CHANNEL {
            let _ = tx.try_send(MidiCmd::Raw(vec![0xC0 | ch, program & 127]));
        }
        let _ = tx.try_send(MidiCmd::Note {
            ch,
            pitch: pitch & 127,
            vel: velocity.clamp(1, 127),
            delay: 0.0,
            dur: ms as f32 / 1000.0,
            bend: vec![],
        });
    }

    /// Before playback: each track's instrument, volume and pan, on its channel.
    pub fn setup_tracks(&self, tracks: &[Track]) {
        let Some(tx) = &self.output else {
            return;
        };
        for (i, t) in tracks.iter().enumerate() {
            let ch = channel_for(i, t.kind() == InstrumentType::Drums);
            if ch != DRUM_CHANNEL {
                let _ = tx.try_send(MidiCmd::Raw(vec![0xC0 | ch, t.instrument.midi_program() & 127]));
            }
            // The note velocities already carry each track's volume, so the channel volume
            // stays at a fixed, comfortable level.
            let _ = tx.try_send(MidiCmd::Raw(vec![0xB0 | ch, 7, 100]));
            let pan = ((t.pan.clamp(-1.0, 1.0) + 1.0) * 63.5).round() as u8;
            let _ = tx.try_send(MidiCmd::Raw(vec![0xB0 | ch, 10, pan]));
            // The pitch wheel reaches BEND_RANGE semitones each way (RPN 0), so a bend of any
            // size fits.
            for b in [[101u8, 0], [100, 0], [6, BEND_RANGE as u8], [38, 0], [101, 127], [100, 127]] {
                let _ = tx.try_send(MidiCmd::Raw(vec![0xB0 | ch, b[0], b[1]]));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn listing_ports_works_without_devices() {
        // Whatever is connected, listing must not fail, and an unknown port must be refused.
        let _ = output_port_names();
        let _ = input_port_names();
        let mut m = Midi::default();
        assert!(m.set_output(None).is_ok() && !m.output_open());
        assert!(m.set_output(Some("no such port")).is_err());
        assert!(m.set_input(Some("no such port")).is_err());
        assert_eq!(m.activity(), 0);
        m.preview_note(0, 0, 60, 90, 100); // no output open: nothing happens
        assert!(m.sender().is_none());
    }

    #[test]
    fn bends_follow_the_built_in_shapes() {
        assert!(bend_curve(0.0, 0, 1.0).is_empty());
        // Up: rest, then the full bend at half the note.
        assert_eq!(bend_curve(2.0, 0, 1.0), vec![(0.0, 0.0), (0.5, 2.0)]);
        // Bend and release returns to rest at the end.
        assert_eq!(bend_curve(2.0, 1, 1.0).last(), Some(&(1.0, 0.0)));
        // A pre-bend starts bent.
        assert_eq!(curve_at(&bend_curve(2.0, 2, 1.0), 0.0), 2.0);
        // Pre-bend and release starts bent and ends at rest.
        let c = bend_curve(2.0, 3, 1.0);
        assert_eq!((curve_at(&c, 0.0), curve_at(&c, 0.5), curve_at(&c, 5.0)), (2.0, 0.0, 0.0));
        // Halfway up the bend.
        assert!((curve_at(&bend_curve(2.0, 0, 1.0), 0.25) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn pitch_wheel_values() {
        assert_eq!(bend_value(0.0), 8192);
        assert_eq!(bend_value(BEND_RANGE), 16383);
        assert_eq!(bend_value(-BEND_RANGE), 0);
        // A whole tone up with a 12 semitone range.
        assert_eq!(bend_value(2.0), 8192 + 1365);
    }

    #[test]
    fn channels_skip_the_drum_channel() {
        assert_eq!(channel_for(0, false), 0);
        assert_eq!(channel_for(8, false), 8);
        assert_eq!(channel_for(9, false), 10);
        assert_eq!(channel_for(14, false), 15);
        assert_eq!(channel_for(15, false), 0);
        assert_eq!(channel_for(3, true), DRUM_CHANNEL);
        assert!((0..40).all(|t| channel_for(t, false) != DRUM_CHANNEL));
    }
}
