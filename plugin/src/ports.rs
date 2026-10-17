//! One alea, many tracks: the plugin opens a MIDI port of its own (a virtual
//! port, seen by Live like a keyboard plugged in) and sends each layer of the
//! work on its own channel: layer 1 on channel 1, layer 2 on channel 2…
//! In Live, each instrument track takes "MIDI From: alea", the channel of its
//! layer, Monitor In.
//!
//! The audio thread only puts timed messages in a queue (no allocation, no
//! lock); a thread of its own sends them when they are due.

use std::cmp::Reverse;
use std::collections::{BTreeSet, BinaryHeap};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, SyncSender};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// How many messages can wait in the queue.
const QUEUE: usize = 8192;

/// A message for the port, and when to send it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Timed {
    pub at: Instant,
    pub len: u8,
    pub bytes: [u8; 3],
}

/// The port of one alea, as the window sees it.
pub struct Port {
    /// Whether the port should be open.
    pub on: AtomicBool,
    /// What the window says about it.
    pub status: Mutex<String>,
    /// The port's name, once open ("alea", "alea 2"…).
    pub name: Mutex<String>,
    /// Where to send: "" for a port of alea's own, or the name of an existing
    /// port (the IAC Driver's bus on macOS, say).
    pub target: Mutex<String>,
    /// The existing ports alea can send to.
    pub outputs: Mutex<Vec<String>>,
    /// Messages sent, and those the system refused (with the last reason).
    pub sent: AtomicU64,
    pub refused: AtomicU64,
    /// Messages the audio thread queued for the port.
    pub queued: AtomicU64,
    pub last_error: Mutex<String>,
    /// Asked from the window: a C on every channel.
    pub test: AtomicBool,
    /// Asked from the window: list the existing ports again.
    pub list: AtomicBool,
    queue: Mutex<Option<Receiver<Timed>>>,
}

impl Port {
    /// A port, and the sender the audio thread uses.
    pub fn new() -> (Port, SyncSender<Timed>) {
        let (tx, rx) = std::sync::mpsc::sync_channel(QUEUE);
        (
            Port {
                on: AtomicBool::new(false),
                status: Mutex::new(String::new()),
                name: Mutex::new(String::new()),
                target: Mutex::new(String::new()),
                outputs: Mutex::new(Vec::new()),
                sent: AtomicU64::new(0),
                refused: AtomicU64::new(0),
                queued: AtomicU64::new(0),
                last_error: Mutex::new(String::new()),
                test: AtomicBool::new(false),
                list: AtomicBool::new(true),
                queue: Mutex::new(Some(rx)),
            },
            tx,
        )
    }
}

/// The channel a layer is sent on (its number, from 0), whatever channel
/// the work gave it: a track in Live listens to one channel.
pub fn rechannel(bytes: [u8; 3], layer: u8) -> [u8; 3] {
    if bytes[0] < 0x80 || bytes[0] >= 0xF0 {
        return bytes;
    }
    [(bytes[0] & 0xF0) | (layer % 16), bytes[1], bytes[2]]
}

/// The block to play for the port, `ahead` samples ahead of the host's, and
/// the shift (in samples) from its samples to the moments to send them. On a
/// start or a jump (`expected`: where the last block ended), the block also
/// covers the samples in between, sent at once.
pub fn ahead(b: crate::engine::Block, expected: Option<f64>, ahead: u32) -> (crate::engine::Block, i64) {
    let mut out = b;
    if b.playing && expected.is_some_and(|e| (e - b.beat).abs() < 1e-6) {
        out.beat = b.beat + ahead as f64 * b.beats_per_sample;
        (out, 0)
    } else {
        out.samples = b.samples + ahead;
        (out, -(ahead as i64))
    }
}

/// Messages in the order they are due (in the order they came, for the same time).
#[derive(Default)]
pub struct Schedule {
    heap: BinaryHeap<Reverse<(Instant, u64, u8, [u8; 3])>>,
    count: u64,
}

impl Schedule {
    pub fn push(&mut self, t: Timed) {
        self.count += 1;
        self.heap.push(Reverse((t.at, self.count, t.len, t.bytes)));
    }

    /// The messages due by `now`, in order.
    pub fn due(&mut self, now: Instant, mut send: impl FnMut(&[u8])) {
        while let Some(Reverse((at, _, len, bytes))) = self.heap.peek().copied() {
            if at > now {
                break;
            }
            self.heap.pop();
            send(&bytes[..len.clamp(1, 3) as usize]);
        }
    }

    /// When the next message is due.
    pub fn next(&self) -> Option<Instant> {
        self.heap.peek().map(|Reverse((at, ..))| *at)
    }

    /// Everything still waiting, now (before the port closes).
    pub fn flush(&mut self, send: impl FnMut(&[u8])) {
        let far = Instant::now() + Duration::from_secs(3600);
        self.due(far, send);
    }
}

/// The names of the ports open in this process (Live loads every alea in it).
static NAMES: Mutex<BTreeSet<String>> = Mutex::new(BTreeSet::new());

fn take_name() -> String {
    let mut names = NAMES.lock().unwrap();
    let name = (1..)
        .map(|k| if k == 1 { "alea".to_string() } else { format!("alea {k}") })
        .find(|n| !names.contains(n))
        .unwrap();
    names.insert(name.clone());
    name
}

fn give_back(name: &str) {
    NAMES.lock().unwrap().remove(name);
}

/// An open port.
trait Output {
    fn send(&mut self, bytes: &[u8]) -> Result<(), String>;
}

#[cfg(unix)]
struct Midir(midir::MidiOutputConnection);

#[cfg(unix)]
impl Output for Midir {
    fn send(&mut self, bytes: &[u8]) -> Result<(), String> {
        self.0.send(bytes).map_err(|e| e.to_string())
    }
}

/// A port of alea's own (`target` empty), or an existing one, by name.
#[cfg(unix)]
fn open(target: &str, name: &str) -> Result<Box<dyn Output>, String> {
    let out = midir::MidiOutput::new(name).map_err(|e| e.to_string())?;
    if target.is_empty() {
        use midir::os::unix::VirtualOutput;
        let conn = out.create_virtual(name).map_err(|e| e.to_string())?;
        return Ok(Box::new(Midir(conn)));
    }
    let port = out.ports().into_iter().find(|p| out.port_name(p).is_ok_and(|n| n == target)).ok_or_else(|| format!("no port named \u{201c}{target}\u{201d}"))?;
    let conn = out.connect(&port, name).map_err(|e| e.to_string())?;
    Ok(Box::new(Midir(conn)))
}

#[cfg(not(unix))]
fn open(_target: &str, _name: &str) -> Result<Box<dyn Output>, String> {
    Err("sending to a port needs macOS or Linux".into())
}

/// The existing ports alea can send to.
#[cfg(unix)]
fn outputs() -> Vec<String> {
    match midir::MidiOutput::new("alea list") {
        Ok(out) => out.ports().iter().filter_map(|p| out.port_name(p).ok()).collect(),
        Err(_) => Vec::new(),
    }
}

#[cfg(not(unix))]
fn outputs() -> Vec<String> {
    Vec::new()
}

/// Sends, and counts.
fn put(port: &Port, out: &mut dyn Output, bytes: &[u8]) {
    match out.send(bytes) {
        Ok(()) => {
            port.sent.fetch_add(1, Ordering::Relaxed);
        }
        Err(e) => {
            port.refused.fetch_add(1, Ordering::Relaxed);
            if let Ok(mut l) = port.last_error.try_lock() {
                if *l != e {
                    *l = e;
                }
            }
        }
    }
}

/// Every note off, on every channel.
fn hush(out: &mut dyn Output) {
    for ch in 0..16u8 {
        let _ = out.send(&[0xB0 | ch, 123, 0]); // all notes off
        let _ = out.send(&[0xB0 | ch, 64, 0]); // sustain off
    }
}

/// The thread that sends: opens the port when asked, closes it when not.
pub fn run(port: &Port, quit: &AtomicBool) {
    run_with(port, quit, open)
}

fn run_with(port: &Port, quit: &AtomicBool, open: fn(&str, &str) -> Result<Box<dyn Output>, String>) {
    let Some(rx) = port.queue.lock().unwrap().take() else { return };
    let mut schedule = Schedule::default();
    // the port open: its name, what it was opened for (its target), and it
    let mut open_port: Option<(String, String, Box<dyn Output>)> = None;
    let mut failed: Option<(String, Instant)> = None;
    loop {
        let stop = quit.load(Ordering::Relaxed);
        let want = port.on.load(Ordering::Relaxed) && !stop;
        let target = port.target.lock().unwrap().clone();
        if port.list.swap(false, Ordering::Relaxed) {
            *port.outputs.lock().unwrap() = outputs();
        }
        // another target chosen: close, to open the new one
        let close = !want || open_port.as_ref().is_some_and(|(_, t, _)| *t != target);
        if close {
            if let Some((name, _, mut out)) = open_port.take() {
                // the last note-offs first, then silence on every channel
                while let Ok(t) = rx.try_recv() {
                    schedule.push(t);
                }
                schedule.flush(|b| put(port, out.as_mut(), b));
                hush(out.as_mut());
                if target_is_own(&name) {
                    give_back(&name);
                }
                port.name.lock().unwrap().clear();
                *port.status.lock().unwrap() = String::new();
            }
        }
        let retry = failed.as_ref().is_none_or(|(t, at)| *t != target || at.elapsed() > Duration::from_secs(3));
        if want && open_port.is_none() && retry {
            let name = if target.is_empty() { take_name() } else { "alea".to_string() };
            match open(&target, &name) {
                Ok(out) => {
                    let shown = if target.is_empty() { name.clone() } else { target.clone() };
                    *port.status.lock().unwrap() = format!("sending to \u{201c}{shown}\u{201d}: layer N on channel N");
                    *port.name.lock().unwrap() = shown;
                    open_port = Some((if target.is_empty() { name } else { String::new() }, target.clone(), out));
                    failed = None;
                }
                Err(e) => {
                    if target.is_empty() {
                        give_back(&name);
                    }
                    *port.status.lock().unwrap() = format!("could not open the port: {e}");
                    failed = Some((target.clone(), Instant::now()));
                }
            }
        }
        if stop {
            break;
        }
        if port.test.swap(false, Ordering::Relaxed) {
            if let Some((_, _, out)) = &mut open_port {
                for ch in 0..16u8 {
                    put(port, out.as_mut(), &[0x90 | ch, 60, 100]);
                    schedule.push(Timed { at: Instant::now() + Duration::from_millis(400), len: 3, bytes: [0x80 | ch, 60, 0] });
                }
            }
        }

        // wait for a message, or until the next one is due
        let wait = schedule.next().map(|at| at.saturating_duration_since(Instant::now())).unwrap_or(Duration::from_millis(20)).min(Duration::from_millis(20));
        match rx.recv_timeout(wait) {
            Ok(t) => {
                schedule.push(t);
                while let Ok(t) = rx.try_recv() {
                    schedule.push(t);
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
        match &mut open_port {
            Some((_, _, out)) => schedule.due(Instant::now(), |b| put(port, out.as_mut(), b)),
            None => schedule.due(Instant::now() + Duration::from_secs(3600), |_| {}),
        }
    }
    if let Some((name, _, mut out)) = open_port.take() {
        hush(out.as_mut());
        if target_is_own(&name) {
            give_back(&name);
        }
    }
    *port.queue.lock().unwrap() = Some(rx);
}

/// Names of ports alea opened itself (an existing port is kept as "").
fn target_is_own(name: &str) -> bool {
    !name.is_empty()
}

/// For the window and the README: the steps in Live.
pub const HOW: &str = "In Live: Settings › Link, Tempo & MIDI › MIDI Ports: the port's Input, Track On (once). Then, on each instrument's MIDI track: MIDI From that port, the channel of its layer, Monitor In.";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_time_and_in_order() {
        let t0 = Instant::now();
        let ms = |k: u64| t0 + Duration::from_millis(k);
        let mut s = Schedule::default();
        s.push(Timed { at: ms(20), len: 3, bytes: [0x90, 62, 100] });
        s.push(Timed { at: ms(10), len: 3, bytes: [0x80, 60, 0] });
        s.push(Timed { at: ms(10), len: 3, bytes: [0x90, 60, 90] });
        let mut out = Vec::new();
        s.due(ms(15), |b| out.push(b.to_vec()));
        assert_eq!(out, vec![vec![0x80, 60, 0], vec![0x90, 60, 90]], "the off before the on, as they came");
        assert_eq!(s.next(), Some(ms(20)));
        out.clear();
        s.flush(|b| out.push(b.to_vec()));
        assert_eq!(out, vec![vec![0x90, 62, 100]]);
    }

    #[test]
    fn each_layer_its_channel() {
        assert_eq!(rechannel([0x99, 36, 100], 2), [0x92, 36, 100]);
        assert_eq!(rechannel([0xB0, 74, 64], 7), [0xB7, 74, 64]);
        assert_eq!(rechannel([0x80, 60, 0], 17), [0x81, 60, 0]);
    }

    #[test]
    fn every_note_once_and_early() {
        use crate::engine::{Block, Edition, Msg, Player, ALL};
        // a note on every beat, 8 beats
        let mut msgs = Vec::new();
        for k in 0..8 {
            msgs.push(Msg { beat: k as f64, track: 1, len: 3, bytes: [0x90, 60 + k as u8, 100] });
            msgs.push(Msg { beat: k as f64 + 0.5, track: 1, len: 3, bytes: [0x80, 60 + k as u8, 0] });
        }
        let ed = Edition::for_tests(8.0, msgs);
        let (sr, tempo, n, lead) = (48_000.0, 120.0, 512u32, 512u32);
        let bps = tempo / 60.0 / sr;
        let mut p = Player::default();
        let mut expected = None;
        let mut sent = Vec::new(); // (sample of the host when sent, note)
        let mut beat = 0.0;
        for k in 0..((4.0 / bps) as u32 / n) {
            let block = Block { playing: true, beat, beats_per_sample: bps, samples: n, bar: 4.0, layers: ALL };
            let (pb, offset) = ahead(block, expected, lead);
            let mut out = Vec::new();
            p.block(pb, 0, 1, |_| Some(&ed), &mut out);
            for (at, m) in out {
                if m.bytes[0] & 0xF0 == 0x90 {
                    sent.push(((k * n) as i64 + (at as i64 + offset).max(0), m.bytes[1]));
                }
            }
            expected = Some(beat + n as f64 * bps);
            beat += n as f64 * bps;
        }
        let notes: Vec<u8> = sent.iter().map(|s| s.1).collect();
        assert_eq!(notes, vec![60, 61, 62, 63, 64], "each note once, none lost at the start (and beat 4 already sent, early)");
        for (when, note) in &sent[1..] {
            let due = ((note - 60) as f64 / bps) as i64;
            assert!((due - when - lead as i64).abs() <= 1, "note {note}: sent {} samples early", due - when);
        }
        assert_eq!(sent[0].0, 0, "the first note at once");
    }

    static GOT: Mutex<Vec<(Instant, Vec<u8>)>> = Mutex::new(Vec::new());
    struct Fake;
    impl Output for Fake {
        fn send(&mut self, bytes: &[u8]) -> Result<(), String> {
            GOT.lock().unwrap().push((Instant::now(), bytes.to_vec()));
            Ok(())
        }
    }
    fn fake(_: &str, _: &str) -> Result<Box<dyn Output>, String> {
        Ok(Box::new(Fake))
    }

    #[test]
    fn the_thread_sends_in_time() {
        let (port, tx) = Port::new();
        let port = std::sync::Arc::new(port);
        let quit = std::sync::Arc::new(AtomicBool::new(false));
        port.on.store(true, Ordering::Relaxed);
        let (p, q) = (port.clone(), quit.clone());
        let thread = std::thread::spawn(move || run_with(&p, &q, fake));
        std::thread::sleep(Duration::from_millis(50));
        assert!(port.status.lock().unwrap().starts_with("sending to"), "{}", port.status.lock().unwrap());
        let t0 = Instant::now();
        tx.send(Timed { at: t0 + Duration::from_millis(30), len: 3, bytes: [0x91, 64, 90] }).unwrap();
        tx.send(Timed { at: t0, len: 3, bytes: [0x90, 60, 100] }).unwrap();
        std::thread::sleep(Duration::from_millis(80));
        port.test.store(true, Ordering::Relaxed);
        std::thread::sleep(Duration::from_millis(60));
        let got = GOT.lock().unwrap().clone();
        let notes: Vec<_> = got.iter().map(|g| g.1.clone()).collect();
        assert_eq!(notes[0], vec![0x90, 60, 100]);
        assert_eq!(notes[1], vec![0x91, 64, 90]);
        let late = got[1].0.duration_since(t0 + Duration::from_millis(30));
        assert!(late < Duration::from_millis(25), "on time: {late:?} late");
        assert_eq!(notes.iter().filter(|n| n[1] == 60 && n[0] & 0xF0 == 0x90).count(), 17, "the test: a C on all 16 channels");
        assert_eq!(port.sent.load(Ordering::Relaxed), got.len() as u64);
        port.on.store(false, Ordering::Relaxed);
        std::thread::sleep(Duration::from_millis(60));
        assert!(GOT.lock().unwrap().iter().any(|g| g.1 == vec![0xB0, 123, 0]), "closing: all notes off");
        quit.store(true, Ordering::Relaxed);
        thread.join().unwrap();
    }

    #[test]
    fn names_for_each_alea() {
        let a = take_name();
        let b = take_name();
        assert_ne!(a, b);
        give_back(&a);
        assert_eq!(take_name(), a, "a name given back is used again");
        give_back(&a);
        give_back(&b);
    }
}
