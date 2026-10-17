//! Playing a score live: `alea work.alea --midi`.
//!
//! Messages are sent at their moment, either on alea's own clock or following
//! the MIDI clock of a DAW (`--sync`: 24 ticks per beat, start, stop,
//! continue). Ctrl-C, or the DAW's stop, silences every note that is still on.
//! The ports themselves come from `midir` (feature `midi`); `--midi-port
//! print` writes the messages as text instead, and needs no MIDI at all.

use std::collections::BTreeSet;
use std::io::Write;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::midi::Score;

/// Where messages go.
pub trait Sink {
    fn send(&mut self, bytes: &[u8]) -> Result<(), String>;
}

/// `--midi-port print`: one line per message, with its time.
pub struct Printer {
    start: Instant,
}

impl Printer {
    pub fn new() -> Printer {
        Printer { start: Instant::now() }
    }
}

impl Default for Printer {
    fn default() -> Self {
        Printer::new()
    }
}

const NAMES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];

/// A message in words: "note on  ch 1  C4  velocity 100".
pub fn describe(b: &[u8]) -> String {
    let ch = (b[0] & 0x0F) + 1;
    let note = |n: u8| format!("{}{}", NAMES[n as usize % 12], n as i32 / 12 - 1);
    match (b[0] & 0xF0, b.len()) {
        (0x90, 3) if b[2] > 0 => format!("note on   ch {ch:<2} {:<4} velocity {}", note(b[1]), b[2]),
        (0x80 | 0x90, 3) => format!("note off  ch {ch:<2} {}", note(b[1])),
        (0xB0, 3) => format!("cc {:<3}    ch {ch:<2} {}", b[1], b[2]),
        (0xC0, 2) => format!("program   ch {ch:<2} {}", b[1]),
        (0xE0, 3) => format!("bend      ch {ch:<2} {}", (b[2] as i32) << 7 | b[1] as i32),
        _ => format!("{b:02x?}"),
    }
}

impl Sink for Printer {
    fn send(&mut self, bytes: &[u8]) -> Result<(), String> {
        let line = format!("{:9.3}  {}\n", self.start.elapsed().as_secs_f64(), describe(bytes));
        std::io::stdout().lock().write_all(line.as_bytes()).map_err(|e| e.to_string())
    }
}

// ───────────────────────────── stopping ─────────────────────────────

/// Set by Ctrl-C.
pub static STOP: AtomicBool = AtomicBool::new(false);

#[cfg(unix)]
pub fn catch_ctrl_c() {
    extern "C" fn on_signal(_: libc::c_int) {
        STOP.store(true, Ordering::SeqCst);
    }
    // SAFETY: the handler only stores into an atomic, which is signal-safe.
    unsafe {
        libc::signal(libc::SIGINT, on_signal as *const () as libc::sighandler_t);
        libc::signal(libc::SIGTERM, on_signal as *const () as libc::sighandler_t);
    }
}

#[cfg(not(unix))]
pub fn catch_ctrl_c() {}

// ───────────────────────────── the clock of a DAW ─────────────────────────────

/// What the DAW's clock said last: how many ticks since its start, when the
/// last one came, how far apart they are, and whether it is running.
#[derive(Default)]
pub struct SyncState {
    pub ticks: AtomicU64,
    pub running: AtomicBool,
    /// Counts the DAW's "start" messages: a new one means "from the top".
    pub starts: AtomicU64,
    /// When the last tick came (seconds, on the clock of `now`), the interval
    /// between ticks, and whether that interval is still a guess (none measured yet).
    last: Mutex<Option<(f64, f64, bool)>>,
}

impl SyncState {
    /// Takes one byte of MIDI from the DAW (real-time messages only).
    pub fn feed(&self, b: u8) {
        self.feed_at(b, RealClock.now())
    }

    /// The same, at a given time (seconds on a `Clock`).
    fn feed_at(&self, b: u8, now: f64) {
        match b {
            0xF8 => {
                let mut last = self.last.lock().unwrap();
                let (interval, guess) = match *last {
                    // the first interval measured replaces the guess outright
                    Some((t, _, true)) if now - t < 0.2 => (now - t, false),
                    // then smoothed: a jittery clock shouldn't shake the notes
                    Some((t, i, guess)) => {
                        let d = now - t;
                        if d < 0.2 { (i * 0.8 + d * 0.2, false) } else { (i, guess) }
                    }
                    // before the second tick, a guess: 120 BPM
                    None => (60.0 / 120.0 / 24.0, true),
                };
                *last = Some((now, interval, guess));
                if self.running.load(Ordering::SeqCst) {
                    self.ticks.fetch_add(1, Ordering::SeqCst);
                }
            }
            0xFA => {
                self.ticks.store(0, Ordering::SeqCst);
                self.starts.fetch_add(1, Ordering::SeqCst);
                self.running.store(true, Ordering::SeqCst);
            }
            0xFB => self.running.store(true, Ordering::SeqCst),
            0xFC => self.running.store(false, Ordering::SeqCst),
            _ => {}
        }
    }

    /// Where the DAW is at `now`, in ticks, between two ticks included.
    fn position(&self, now: f64) -> f64 {
        let n = self.ticks.load(Ordering::SeqCst) as f64;
        match *self.last.lock().unwrap() {
            Some((t, i, _)) if self.running.load(Ordering::SeqCst) => n + ((now - t).max(0.0) / i).min(0.999),
            _ => n,
        }
    }

    pub fn tempo(&self) -> Option<f64> {
        self.last.lock().unwrap().map(|(_, i, _)| 60.0 / (i * 24.0))
    }
}

pub enum Timing {
    /// alea's own clock, at the score's own tempo.
    Free,
    /// The DAW's clock: the score's beats follow its tempo.
    Sync(Arc<SyncState>),
}

#[derive(Debug, PartialEq)]
pub enum Outcome {
    Finished,
    /// Ctrl-C.
    Stopped,
    /// The DAW started again from the top.
    Restart,
}

// ───────────────────────────── time ─────────────────────────────

/// Where playing reads the time and waits: the real clock, or, in the tests,
/// a clock that moves only when asked (so they don't depend on how busy the
/// machine is).
pub trait Clock {
    /// Seconds since some fixed moment.
    fn now(&self) -> f64;
    fn sleep(&self, d: Duration);
}

/// The machine's clock.
pub struct RealClock;

impl Clock for RealClock {
    fn now(&self) -> f64 {
        static EPOCH: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
        EPOCH.get_or_init(Instant::now).elapsed().as_secs_f64()
    }
    fn sleep(&self, d: Duration) {
        std::thread::sleep(d)
    }
}

// ───────────────────────────── playing ─────────────────────────────

/// What is sounding, to silence it when playing stops.
#[derive(Default)]
struct Held {
    notes: BTreeSet<(u8, u8)>,
    channels: BTreeSet<u8>,
}

impl Held {
    fn track(&mut self, b: &[u8]) {
        let ch = b[0] & 0x0F;
        self.channels.insert(ch);
        match (b[0] & 0xF0, b.len()) {
            (0x90, 3) if b[2] > 0 => {
                self.notes.insert((ch, b[1]));
            }
            (0x80 | 0x90, 3) => {
                self.notes.remove(&(ch, b[1]));
            }
            _ => {}
        }
    }

    fn silence(&mut self, sink: &mut dyn Sink) -> Result<(), String> {
        for (ch, n) in std::mem::take(&mut self.notes) {
            sink.send(&[0x80 | ch, n, 0])?;
        }
        for ch in &self.channels {
            sink.send(&[0xB0 | ch, 123, 0])?; // all notes off
        }
        Ok(())
    }
}

/// Waits until `deadline` (seconds on `clock`; checked every millisecond near
/// the end), or until Ctrl-C. Returns false if stopped.
fn wait_until(clock: &dyn Clock, deadline: f64) -> bool {
    loop {
        if STOP.load(Ordering::SeqCst) {
            return false;
        }
        let left = deadline - clock.now();
        if left <= 0.0 {
            return true;
        }
        clock.sleep(if left > 0.012 { Duration::from_millis(10) } else { Duration::from_secs_f64(left).min(Duration::from_millis(1)) });
    }
}

/// Plays a score. With a DAW's clock, `base` is the tick where it starts;
/// it is moved to the next bar after its end, for the next edition.
pub fn play(score: &Score, sink: &mut dyn Sink, timing: &Timing, base: &mut f64) -> Result<Outcome, String> {
    play_on(&RealClock, score, sink, timing, base)
}

fn play_on(time: &dyn Clock, score: &Score, sink: &mut dyn Sink, timing: &Timing, base: &mut f64) -> Result<Outcome, String> {
    let mut held = Held::default();
    let result = (|| -> Result<Outcome, String> {
        match timing {
            Timing::Free => {
                let start = time.now();
                for m in &score.msgs {
                    if !wait_until(time, start + m.at) {
                        return Ok(Outcome::Stopped);
                    }
                    sink.send(&m.bytes)?;
                    held.track(&m.bytes);
                }
                if !wait_until(time, start + score.duration) {
                    return Ok(Outcome::Stopped);
                }
                Ok(Outcome::Finished)
            }
            Timing::Sync(clock) => {
                let ticks_per_second = score.tempo / 60.0 * 24.0;
                // not started yet: the DAW's start is our start, not a restart
                while !clock.running.load(Ordering::SeqCst) {
                    if STOP.load(Ordering::SeqCst) {
                        return Ok(Outcome::Stopped);
                    }
                    time.sleep(Duration::from_millis(1));
                }
                let starts = clock.starts.load(Ordering::SeqCst);
                let reach = |target: f64, held: &mut Held, sink: &mut dyn Sink| -> Result<Option<Outcome>, String> {
                    let mut paused = false;
                    loop {
                        if STOP.load(Ordering::SeqCst) {
                            return Ok(Some(Outcome::Stopped));
                        }
                        if clock.starts.load(Ordering::SeqCst) != starts {
                            return Ok(Some(Outcome::Restart));
                        }
                        if !clock.running.load(Ordering::SeqCst) {
                            if !paused {
                                held.silence(sink)?;
                                paused = true;
                            }
                        } else if clock.position(time.now()) >= target {
                            return Ok(None);
                        }
                        time.sleep(Duration::from_micros(500));
                    }
                };
                for m in &score.msgs {
                    if let Some(o) = reach(*base + m.at * ticks_per_second, &mut held, sink)? {
                        return Ok(o);
                    }
                    sink.send(&m.bytes)?;
                    held.track(&m.bytes);
                }
                let end = *base + score.duration * ticks_per_second;
                if let Some(o) = reach(end, &mut held, sink)? {
                    return Ok(o);
                }
                // the next edition starts on the next bar (4 beats of 24 ticks)
                *base = (end / 96.0).ceil() * 96.0;
                Ok(Outcome::Finished)
            }
        }
    })();
    held.silence(sink)?;
    result
}

// ───────────────────────────── ports ─────────────────────────────

#[cfg(feature = "midi")]
mod ports {
    use super::*;
    use midir::{Ignore, MidiInput, MidiOutput, MidiOutputConnection};

    struct Port(MidiOutputConnection);

    impl Sink for Port {
        fn send(&mut self, bytes: &[u8]) -> Result<(), String> {
            self.0.send(bytes).map_err(|e| e.to_string())
        }
    }

    fn find<P>(ports: Vec<P>, name_of: impl Fn(&P) -> Option<String>, want: &str) -> Result<P, String> {
        let names: Vec<String> = ports.iter().filter_map(&name_of).collect();
        let w = want.to_lowercase();
        ports.into_iter().find(|p| name_of(p).is_some_and(|n| n.to_lowercase().contains(&w))).ok_or_else(|| {
            if names.is_empty() {
                format!("no MIDI port matches `{want}`: there are none")
            } else {
                format!("no MIDI port matches `{want}`; there are: {}", names.join(", "))
            }
        })
    }

    /// A port to send to: an existing one whose name contains `name`, or (no
    /// name) a new virtual port called "alea" that other programs can read.
    pub fn output(name: Option<&str>) -> Result<(Box<dyn Sink>, String), String> {
        let out = MidiOutput::new("alea").map_err(|e| e.to_string())?;
        match name {
            Some(want) => {
                let port = find(out.ports(), |p| out.port_name(p).ok(), want)?;
                let shown = out.port_name(&port).unwrap_or_default();
                let c = out.connect(&port, "alea").map_err(|e| e.to_string())?;
                Ok((Box::new(Port(c)), shown))
            }
            None => {
                #[cfg(unix)]
                {
                    use midir::os::unix::VirtualOutput;
                    let c = out.create_virtual("alea").map_err(|e| e.to_string())?;
                    Ok((Box::new(Port(c)), "alea (a new virtual port)".into()))
                }
                #[cfg(not(unix))]
                Err("this system has no virtual MIDI ports: choose one with --midi-port NAME (alea --midi-ports lists them)".into())
            }
        }
    }

    /// Listens to a DAW's clock: on an existing port, or on a new virtual one
    /// called "alea clock". The connection must be kept while playing.
    pub fn clock(name: Option<&str>, state: Arc<SyncState>) -> Result<(Box<dyn std::any::Any>, String), String> {
        let mut input = MidiInput::new("alea clock").map_err(|e| e.to_string())?;
        input.ignore(Ignore::Sysex);
        let feed = move |_: u64, bytes: &[u8], _: &mut ()| {
            for b in bytes {
                state.feed(*b);
            }
        };
        match name {
            Some(want) => {
                let port = find(input.ports(), |p| input.port_name(p).ok(), want)?;
                let shown = input.port_name(&port).unwrap_or_default();
                let c = input.connect(&port, "alea clock", feed, ()).map_err(|e| e.to_string())?;
                Ok((Box::new(c), shown))
            }
            None => {
                #[cfg(unix)]
                {
                    use midir::os::unix::VirtualInput;
                    let c = input.create_virtual("alea clock", feed, ()).map_err(|e| e.to_string())?;
                    Ok((Box::new(c), "alea clock (a new virtual port: send it your DAW's MIDI clock)".into()))
                }
                #[cfg(not(unix))]
                Err("this system has no virtual MIDI ports: choose the clock's port with --sync-port NAME".into())
            }
        }
    }

    pub fn list() -> Result<String, String> {
        let out = MidiOutput::new("alea").map_err(|e| e.to_string())?;
        let input = MidiInput::new("alea").map_err(|e| e.to_string())?;
        let outs: Vec<String> = out.ports().iter().filter_map(|p| out.port_name(p).ok()).collect();
        let ins: Vec<String> = input.ports().iter().filter_map(|p| input.port_name(p).ok()).collect();
        let fmt = |v: &[String]| if v.is_empty() { "  (none)\n".to_string() } else { v.iter().map(|n| format!("  {n}\n")).collect() };
        Ok(format!("MIDI ports alea can play to (--midi-port):\n{}MIDI ports alea can follow (--sync-port):\n{}", fmt(&outs), fmt(&ins)))
    }
}

#[cfg(feature = "midi")]
pub use ports::{clock, list, output};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::midi::{Msg, Score};
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    /// A clock that moves only when playing sleeps, and plays a DAW's clock
    /// messages into `daw` when their time comes: the tests are exact, and
    /// take no time.
    #[derive(Default)]
    struct Simulated {
        t: Cell<f64>,
        daw: Option<(Arc<SyncState>, RefCell<Vec<(f64, u8)>>)>,
    }
    impl Clock for Simulated {
        fn now(&self) -> f64 {
            self.t.get()
        }
        fn sleep(&self, d: Duration) {
            self.t.set(self.t.get() + d.as_secs_f64());
            if let Some((state, events)) = &self.daw {
                let mut events = events.borrow_mut();
                while events.first().is_some_and(|(at, _)| *at <= self.t.get()) {
                    let (at, b) = events.remove(0);
                    state.feed_at(b, at);
                }
            }
        }
    }

    /// What was sent, and when (on the simulated clock).
    struct Record(Vec<(f64, Vec<u8>)>, Rc<Simulated>);
    impl Sink for Record {
        fn send(&mut self, b: &[u8]) -> Result<(), String> {
            self.0.push((self.1.now(), b.to_vec()));
            Ok(())
        }
    }

    fn score() -> Score {
        let m = |at: f64, b: Vec<u8>| Msg { at, track: 0, bytes: b };
        Score {
            tracks: vec!["t".into()],
            msgs: vec![m(0.0, vec![0x90, 60, 100]), m(0.1, vec![0x80, 60, 0]), m(0.2, vec![0x90, 64, 90])],
            tempo: 120.0,
            duration: 0.3,
        }
    }

    #[test]
    fn plays_on_time_and_silences_at_the_end() {
        let clock = Rc::new(Simulated::default());
        let mut r = Record(Vec::new(), clock.clone());
        let o = play_on(clock.as_ref(), &score(), &mut r, &Timing::Free, &mut 0.0).unwrap();
        assert_eq!(o, Outcome::Finished);
        // each message on its time (to the millisecond, the step of the wait)
        for ((t, _), want) in r.0.iter().zip([0.0, 0.1, 0.2]) {
            assert!(*t >= want - 1e-9 && *t < want + 0.0011, "{t} vs {want}");
        }
        // the note still on (64) is turned off, then all notes off on channel 1
        assert_eq!(r.0[3].1, vec![0x80, 64, 0]);
        assert_eq!(r.0[4].1, vec![0xB0, 123, 0]);
        assert!(r.0[3].0 >= 0.3, "the end waits for the score's duration");
    }

    #[test]
    fn follows_a_clock() {
        // a DAW at 240 BPM: start, then a tick every 1/96 s
        let state = Arc::new(SyncState::default());
        let mut events = vec![(0.0, 0xFA)];
        events.extend((1..=60).map(|k| (k as f64 / 96.0, 0xF8)));
        let clock = Rc::new(Simulated { daw: Some((state.clone(), RefCell::new(events))), ..Default::default() });
        let mut r = Record(Vec::new(), clock.clone());
        let mut base = 0.0;
        let o = play_on(clock.as_ref(), &score(), &mut r, &Timing::Sync(state), &mut base).unwrap();
        assert_eq!(o, Outcome::Finished);
        // at 240 BPM instead of the score's 120, everything comes twice as fast:
        // 0.1 s of score is 4.8 ticks, 0.05 s of the DAW's time
        for ((t, _), want) in r.0.iter().zip([0.0, 0.05, 0.1]) {
            assert!((t - want).abs() < 0.002, "{t} vs {want}");
        }
        assert_eq!(base, 96.0, "the next edition starts on the next bar");
    }
}
