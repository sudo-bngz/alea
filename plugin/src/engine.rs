//! What the plugin does, apart from the plugin formats: editions prepared in
//! the background, and the notes of the edition sent in time with the host.
//!
//! Everything here is plain Rust, tested without a host.

use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use alea::eval::{Field, Val};

/// The macros a work can be given (its first numeric parameters).
pub const MACROS: usize = 8;

/// One MIDI message of an edition, at a beat (quarter notes from its start).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Msg {
    pub beat: f64,
    /// Which layer of the work (track of its score) it belongs to.
    pub track: u8,
    pub len: u8,
    pub bytes: [u8; 3],
}

/// A note, for drawing the piano roll.
#[derive(Clone, Copy, Debug)]
pub struct RollNote {
    pub start: f64,
    pub end: f64,
    pub note: u8,
    pub channel: u8,
    pub velocity: u8,
    pub track: u8,
}

/// An edition, ready to play: its messages in beats, and what the window shows.
pub struct Edition {
    pub seed: u64,
    pub name: String,
    /// The work's own tempo (the host's tempo is the one that plays).
    pub tempo: f64,
    /// Its length, in beats.
    pub beats: f64,
    pub msgs: Vec<Msg>,
    pub roll: Vec<RollNote>,
    pub tracks: Vec<String>,
    /// The drawn edition, for its picture.
    pub drawn: Arc<alea::eval::Edition>,
    /// Which version of the work (and of its macros) it was drawn from.
    pub generation: u64,
}

impl Edition {
    /// An edition of these messages (for tests).
    #[cfg(test)]
    pub fn for_tests(beats: f64, msgs: Vec<Msg>) -> Edition {
        let module = alea::library::load(concat!(env!("CARGO_MANIFEST_DIR"), "/../examples/tour/01-hello.alea")).unwrap();
        let drawn = alea::eval::resolve(&module, 1).unwrap();
        Edition { seed: 1, name: "t".into(), tempo: 120.0, beats, msgs, roll: vec![], tracks: vec![], drawn: Arc::new(drawn), generation: 0 }
    }

    /// Turns an edition of a work into what the plugin plays.
    pub fn from(ed: alea::eval::Edition, generation: u64) -> Result<Edition, String> {
        let score = alea::media::live_score(&ed)
            .ok_or_else(|| "this work has no music: give it `render midi(…)`, or sound layers (`render sound(drone(…), notes(…))`)".to_string())?
            .map_err(|e| e.msg.clone())?;
        let per_second = score.tempo / 60.0;
        let msgs: Vec<Msg> = score
            .msgs
            .iter()
            .map(|m| {
                let mut bytes = [0u8; 3];
                let n = m.bytes.len().min(3);
                bytes[..n].copy_from_slice(&m.bytes[..n]);
                Msg { beat: m.at * per_second, track: m.track.min(255) as u8, len: n as u8, bytes }
            })
            .collect();
        let beats = score.duration * per_second;
        // pair notes for the roll
        let mut open: HashMap<(u8, u8), (f64, u8, u8)> = HashMap::new();
        let mut roll = Vec::new();
        for m in &msgs {
            let (kind, ch) = (m.bytes[0] & 0xF0, m.bytes[0] & 0x0F);
            match kind {
                0x90 if m.bytes[2] > 0 => {
                    open.insert((ch, m.bytes[1]), (m.beat, m.bytes[2], m.track));
                }
                0x80 | 0x90 => {
                    if let Some((start, velocity, track)) = open.remove(&(ch, m.bytes[1])) {
                        roll.push(RollNote { start, end: m.beat, note: m.bytes[1], channel: ch, velocity, track });
                    }
                }
                _ => {}
            }
        }
        for ((ch, note), (start, velocity, track)) in open {
            roll.push(RollNote { start, end: beats, note, channel: ch, velocity, track });
        }
        Ok(Edition { seed: ed.seed, name: ed.name.clone(), tempo: score.tempo, beats, msgs, roll, tracks: score.tracks.clone(), drawn: Arc::new(ed), generation })
    }

    /// The index of the first message at or after `beat`.
    fn first_from(&self, beat: f64) -> usize {
        self.msgs.partition_point(|m| m.beat < beat)
    }
}

// ───────────────────────────── following the host ─────────────────────────────

/// Where the host is, for one block of audio.
#[derive(Clone, Copy, Debug)]
pub struct Block {
    pub playing: bool,
    /// The position at the start of the block, in quarter notes.
    pub beat: f64,
    /// Quarter notes per sample (the host's tempo).
    pub beats_per_sample: f64,
    pub samples: u32,
    /// Quarter notes in a bar (4 in 4/4, 3 in 3/4, 3 in 6/8).
    pub bar: f64,
    /// The layers of the work that sound: bit k for layer k (see `layers`).
    pub layers: u64,
}

/// How many layers have their own on/off switch (later ones are always on,
/// unless another layer is soloed).
pub const LAYERS: usize = 16;

/// Every layer on.
pub const ALL: u64 = u64::MAX;

/// The layers that sound, as in a mixer: the switched-on ones, or, with a
/// layer soloed (`solo` = layer + 1, 0 for none), only that one if it is on.
pub fn layers(on: &[bool], solo: i32) -> u64 {
    let mut mask = ALL;
    for (k, &v) in on.iter().enumerate().take(LAYERS) {
        if !v {
            mask &= !(1u64 << k);
        }
    }
    if solo > 0 && solo <= 64 {
        mask &= 1u64 << (solo - 1);
    }
    mask
}

fn sounds(mask: u64, track: u8) -> bool {
    track >= 64 || mask & (1u64 << track) != 0
}

/// Which edition plays at a beat of the song, and where in it: with
/// `every` > 0, a new edition every `every` bars (seed, seed + 1, …);
/// otherwise the same one, again and again.
pub fn place(beat: f64, bar: f64, every: u32, base: u64, length: Option<f64>) -> (u64, f64, f64) {
    let (k, local, span_end) = if every > 0 {
        let span = every as f64 * bar;
        let k = (beat / span).floor();
        (k, beat - k * span, span)
    } else {
        (0.0, beat, f64::INFINITY)
    };
    let seed = base.wrapping_add(k.max(0.0) as u64);
    // inside the edition, which loops if it is shorter than its span
    let (pos, next) = match length {
        Some(l) if l > 0.0 => {
            let pos = local.rem_euclid(l);
            (pos, (local - pos + l).min(span_end))
        }
        _ => (local, span_end),
    };
    // `next`: where (in the span) the next loop or the next edition begins
    (seed, pos, next - local)
}

/// What to send, and when (a sample in the block).
pub type Out = Vec<(u32, Msg)>;

/// Plays editions in time with the host, keeping track of the notes that
/// sound, to silence them on a jump, a stop, a loop or a change of edition.
#[derive(Default)]
pub struct Player {
    /// The notes that sound: (layer, channel, note).
    held: BTreeSet<(u8, u8, u8)>,
    /// The layers of the last block, to end the notes of a layer switched off.
    mask: Option<u64>,
    expected: Option<f64>,
    playing: bool,
}

impl Player {
    fn silence(&mut self, at: u32, out: &mut Out) {
        for (track, ch, note) in std::mem::take(&mut self.held) {
            out.push((at, Msg { beat: 0.0, track, len: 3, bytes: [0x80 | ch, note, 0] }));
        }
    }

    /// End the notes of the layers that no longer sound.
    fn mute(&mut self, mask: u64, out: &mut Out) {
        let gone: Vec<_> = self.held.iter().filter(|(t, _, _)| !sounds(mask, *t)).copied().collect();
        for (track, ch, note) in gone {
            self.held.remove(&(track, ch, note));
            out.push((0, Msg { beat: 0.0, track, len: 3, bytes: [0x80 | ch, note, 0] }));
        }
    }

    fn send(&mut self, at: u32, m: Msg, out: &mut Out) {
        let (kind, ch) = (m.bytes[0] & 0xF0, m.bytes[0] & 0x0F);
        match kind {
            0x90 if m.bytes[2] > 0 => {
                if self.held.contains(&(m.track, ch, m.bytes[1])) {
                    // the same note again: end the first one
                    out.push((at, Msg { beat: m.beat, track: m.track, len: 3, bytes: [0x80 | ch, m.bytes[1], 0] }));
                }
                self.held.insert((m.track, ch, m.bytes[1]));
            }
            0x80 | 0x90 => {
                if !self.held.remove(&(m.track, ch, m.bytes[1])) {
                    return; // already silenced (by a jump or a loop)
                }
            }
            _ => {}
        }
        out.push((at, m));
    }

    /// The messages of one block. `edition(seed)` gives the edition of a seed
    /// if it is ready (otherwise that part of the block is silent).
    pub fn block<'a>(&mut self, b: Block, every: u32, base: u64, mut edition: impl FnMut(u64) -> Option<&'a Edition>, out: &mut Out) {
        if self.mask != Some(b.layers) {
            self.mute(b.layers, out);
            self.mask = Some(b.layers);
        }
        if !b.playing {
            if self.playing {
                self.silence(0, out);
            }
            self.playing = false;
            self.expected = None;
            return;
        }
        // a jump (a loop in the host, a click in the timeline): silence
        if self.expected.is_some_and(|e| (e - b.beat).abs() > 1e-6) {
            self.silence(0, out);
        }
        self.playing = true;
        let end = b.beat + b.beats_per_sample * b.samples as f64;
        let sample_of = |beat: f64| (((beat - b.beat) / b.beats_per_sample).floor().max(0.0) as u32).min(b.samples.saturating_sub(1));
        let mut at = b.beat;
        let mut guard = 0;
        while at < end - 1e-12 && guard < 64 {
            guard += 1;
            let length = edition(place(at, b.bar, every, base, None).0).map(|e| e.beats);
            let (seed, pos, to_next) = place(at, b.bar, every, base, length);
            let seg_end = (at + to_next).min(end);
            if let Some(ed) = edition(seed) {
                // a new loop or a new edition starts here: the old notes end
                if pos < 1e-9 && at > b.beat - 1e-12 {
                    self.silence(sample_of(at), out);
                }
                let stop = pos + (seg_end - at);
                for m in &ed.msgs[ed.first_from(pos)..] {
                    if m.beat >= stop - 1e-12 {
                        break;
                    }
                    if !sounds(b.layers, m.track) {
                        continue;
                    }
                    self.send(sample_of(at + (m.beat - pos)), *m, out);
                }
            }
            at = seg_end;
        }
        self.expected = Some(end);
    }

    /// Everything off (when the plugin is reset or removed).
    pub fn stop(&mut self, out: &mut Out) {
        self.silence(0, out);
        self.expected = None;
    }
}

// ───────────────────────────── preparing editions ─────────────────────────────

/// What several instances on the same work share when linked: the edition
/// and how it changes, and the macros. Every alea in the host process sees it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Group {
    pub seed: i32,
    pub every: i32,
    pub macros_on: bool,
    pub macros: [f32; MACROS],
}

pub static GROUPS: std::sync::LazyLock<Mutex<HashMap<String, Group>>> = std::sync::LazyLock::new(Default::default);

/// The names of the layers of the last work loaded, for the Layer parameter's text.
pub static LAYER_NAMES: std::sync::LazyLock<Mutex<Vec<String>>> = std::sync::LazyLock::new(Default::default);

/// What the window shows about the work.
#[derive(Default, Clone)]
pub struct Status {
    pub work: String,
    pub message: String,
    pub error: Option<String>,
    /// The work's parameters that the macros drive, in order.
    pub macro_names: Vec<String>,
}

/// Shared between the audio thread, the window and the worker.
pub struct Shared {
    pub path: Mutex<String>,
    pub editions: Mutex<HashMap<u64, Arc<Edition>>>,
    pub wanted: Mutex<Vec<u64>>,
    pub status: Mutex<Status>,
    /// The macros as last applied (bits of f32), and whether they are on.
    pub macros: [AtomicU32; MACROS],
    pub macros_on: AtomicBool,
    /// Bumped whenever the work or its macros change: older editions are stale.
    pub generation: AtomicU64,
    /// For the window: what plays now.
    pub now_seed: AtomicU64,
    pub now_beat: AtomicU64,
    pub now_playing: AtomicBool,
    pub window_open: AtomicBool,
    pub picture: Mutex<Option<(usize, usize, Vec<u8>, u64)>>,
    /// A file chosen in the system's dialog, for the window to pick up.
    pub picked: Mutex<Option<String>>,
    /// What the linked group plays (for the window to show on its knobs).
    pub group: Mutex<Option<Group>>,
    /// Why there is no picture, if there is none.
    pub picture_note: Mutex<String>,
    pub quit: AtomicBool,
    /// The plugin's own MIDI port, for sending each layer to its own track.
    pub port: crate::ports::Port,
    pub port_tx: std::sync::mpsc::SyncSender<crate::ports::Timed>,
}

impl Default for Shared {
    fn default() -> Self {
        let (port, port_tx) = crate::ports::Port::new();
        Shared {
            path: Mutex::new(String::new()),
            editions: Mutex::new(HashMap::new()),
            wanted: Mutex::new(Vec::with_capacity(16)),
            status: Mutex::new(Status { message: "choose a work (.alea)".into(), ..Default::default() }),
            macros: std::array::from_fn(|_| AtomicU32::new(0.5f32.to_bits())),
            macros_on: AtomicBool::new(false),
            generation: AtomicU64::new(0),
            now_seed: AtomicU64::new(0),
            now_beat: AtomicU64::new(0f64.to_bits()),
            now_playing: AtomicBool::new(false),
            window_open: AtomicBool::new(false),
            picture: Mutex::new(None),
            picked: Mutex::new(None),
            group: Mutex::new(None),
            picture_note: Mutex::new(String::new()),
            quit: AtomicBool::new(false),
            port,
            port_tx,
        }
    }
}

impl Shared {
    /// Asks for an edition (from the audio thread: never waits).
    pub fn want(&self, seed: u64) {
        if let Ok(mut w) = self.wanted.try_lock() {
            if !w.contains(&seed) && w.len() < w.capacity() {
                w.push(seed);
            }
        }
    }
}

/// The worker: loads the work (again when its file changes), and prepares the
/// editions asked for. Runs until `quit`.
pub fn worker(shared: Arc<Shared>) {
    let mut module: Option<Arc<alea::library::Module>> = None;
    let mut loaded: Option<(String, Option<SystemTime>)> = None;
    let mut applied: ([u32; MACROS], bool) = ([0; MACROS], false);
    let mut last_check = std::time::Instant::now() - Duration::from_secs(10);
    while !shared.quit.load(Ordering::Relaxed) {
        // the file: new, or saved again (checked twice a second)
        if last_check.elapsed() > Duration::from_millis(500) {
            last_check = std::time::Instant::now();
            let path = shared.path.lock().unwrap().clone();
            let mtime = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
            let changed = loaded.as_ref().is_none_or(|(p, t)| *p != path || *t != mtime);
            if changed && !path.is_empty() {
                loaded = Some((path.clone(), mtime));
                match alea::library::load(&path) {
                    Ok(m) => {
                        let names: Vec<String> = m.work.params.iter().map(|p| p.name.clone()).take(MACROS).collect();
                        *shared.status.lock().unwrap() =
                            Status { work: m.work.name.clone(), message: "loaded".into(), error: None, macro_names: names };
                        module = Some(m);
                        shared.generation.fetch_add(1, Ordering::SeqCst);
                        shared.editions.lock().unwrap().clear();
                    }
                    Err(e) => {
                        let mut st = shared.status.lock().unwrap();
                        st.error = Some(e.display("", &path));
                        st.message = "the work has an error: the last good version keeps playing".into();
                    }
                }
            }
        }
        // the macros: when they move, the editions are drawn again
        let now: [u32; MACROS] = std::array::from_fn(|k| shared.macros[k].load(Ordering::Relaxed));
        let on = shared.macros_on.load(Ordering::Relaxed);
        if (now, on) != applied {
            applied = (now, on);
            shared.generation.fetch_add(1, Ordering::SeqCst);
            // the editions playing stay until the new ones are ready
            let seeds: Vec<u64> = shared.editions.lock().unwrap().keys().copied().collect();
            shared.wanted.lock().unwrap().extend(seeds);
        }
        let next = {
            let mut w = shared.wanted.lock().unwrap();
            if w.is_empty() { None } else { Some(w.remove(0)) }
        };
        let (Some(seed), Some(m)) = (next, module.clone()) else {
            std::thread::sleep(Duration::from_millis(15));
            continue;
        };
        let generation = shared.generation.load(Ordering::SeqCst);
        if shared.editions.lock().unwrap().get(&seed).is_some_and(|e| e.generation == generation) {
            continue; // already drawn for this version
        }
        let mut given = HashMap::new();
        if on {
            for (k, p) in m.work.params.iter().take(MACROS).enumerate() {
                given.insert(p.name.clone(), Val::Field(Field::Const(f32::from_bits(now[k]) as f64)));
            }
        }
        let started = std::time::Instant::now();
        // a work that makes the engine fail stops itself, never the host
        let drawn = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            alea::eval::resolve_with(&m, seed, &given)
                .map_err(|e| e.display(&m.src, &m.file))
                .and_then(|ed| Edition::from(ed, generation))
        }));
        let result = drawn.unwrap_or_else(|p| Err(format!("alea stopped on this work: {}", panic_text(&p))));
        match result {
            Ok(ed) => {
                if let Ok(mut names) = LAYER_NAMES.lock() {
                    *names = ed.tracks.clone();
                }
                let mut eds = shared.editions.lock().unwrap();
                eds.insert(seed, Arc::new(ed));
                // keep a few: the current one and its neighbours
                if eds.len() > 8 {
                    let current = shared.now_seed.load(Ordering::Relaxed);
                    let far: Vec<u64> = eds.keys().copied().filter(|s| s.abs_diff(current) > 2).collect();
                    for s in far {
                        eds.remove(&s);
                    }
                }
                let mut st = shared.status.lock().unwrap();
                st.error = None;
                st.message = format!("edition 0x{seed:x} ready ({:.1} s)", started.elapsed().as_secs_f64());
            }
            Err(e) => {
                let mut st = shared.status.lock().unwrap();
                st.error = Some(e);
                st.message = format!("edition 0x{seed:x} could not be drawn: the last good one keeps playing");
            }
        }
    }
}

/// The painter: while the window is open, the edition's picture at the
/// moment playing, about ten times a second.
pub fn painter(shared: Arc<Shared>) {
    let mut frame = 0u64;
    let mut bounds: HashMap<u64, (f64, f64)> = HashMap::new();
    while !shared.quit.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_millis(100));
        if !shared.window_open.load(Ordering::Relaxed) {
            continue;
        }
        let seed = shared.now_seed.load(Ordering::Relaxed);
        let Some(ed) = shared.editions.lock().unwrap().get(&seed).cloned() else { continue };
        let beat = f64::from_bits(shared.now_beat.load(Ordering::Relaxed));
        let t = if ed.beats > 0.0 { (beat / ed.beats).clamp(0.0, 1.0) } else { 0.0 };
        let drawn = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| picture(&ed, t, 180, &mut bounds)));
        *shared.picture_note.lock().unwrap() = match &drawn {
            Ok(Some(_)) => String::new(),
            Ok(None) => "this work has no image or video to show".into(),
            Err(p) => format!("the picture could not be drawn: {}", panic_text(p)),
        };
        if let Ok(Some(px)) = drawn {
            frame += 1;
            *shared.picture.lock().unwrap() = Some((180, 180, px, frame));
        }
    }
}

/// The first picture of an edition (its first image or video) at time t.
pub fn picture(ed: &Edition, t: f64, size: usize, bounds: &mut HashMap<u64, (f64, f64)>) -> Option<Vec<u8>> {
    use alea::media;
    // a video moves with the music; otherwise the first image
    let r = ed.drawn.renders.iter().find(|r| r.kind == "video").or_else(|| ed.drawn.renders.iter().find(|r| r.kind == "image"))?;
    let visual = r.visual("").ok()?;
    let (stops, range) = if r.kind == "video" {
        let s = media::video_spec(r).ok()?;
        (s.stops, s.range)
    } else {
        let s = media::image_spec(r).ok()?;
        (s.stops, s.range)
    };
    let fixed = match (&visual, range) {
        (_, Some(r)) => Some(r),
        (media::Visual::Field(f), None) => {
            Some(*bounds.entry(ed.seed).or_insert_with(|| media::animation_bounds(f, 48, 48, ed.seed)))
        }
        _ => None,
    };
    Some(visual.rgb(size, size, t, ed.seed, &stops, fixed))
}

fn panic_text(p: &Box<dyn std::any::Any + Send>) -> String {
    p.downcast_ref::<&str>().map(|s| s.to_string()).or_else(|| p.downcast_ref::<String>().cloned()).unwrap_or_else(|| "an internal error".into())
}

/// Opens the system's file chooser in its own process (so it can't disturb
/// the host), and hands the chosen file to the window.
pub fn choose_file(shared: Arc<Shared>) {
    std::thread::spawn(move || {
        #[cfg(target_os = "macos")]
        let out = std::process::Command::new("osascript")
            .args(["-e", "set f to choose file with prompt \"Choose a work of alea (.alea)\"", "-e", "POSIX path of f"])
            .output();
        #[cfg(target_os = "linux")]
        let out = std::process::Command::new("zenity").args(["--file-selection", "--title=Choose a work of alea", "--file-filter=*.alea"]).output();
        #[cfg(target_os = "windows")]
        let out = std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", "Add-Type -AssemblyName System.Windows.Forms; $d = New-Object System.Windows.Forms.OpenFileDialog; $d.Filter = 'alea (*.alea)|*.alea'; if ($d.ShowDialog() -eq 'OK') { $d.FileName }"])
            .output();
        if let Ok(o) = out {
            let path = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if o.status.success() && !path.is_empty() {
                *shared.picked.lock().unwrap() = Some(path);
            }
        }
    });
}

pub fn default_path() -> PathBuf {
    PathBuf::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edition(beats: f64, msgs: Vec<(f64, [u8; 3])>) -> Edition {
        let module = alea::library::load(concat!(env!("CARGO_MANIFEST_DIR"), "/../examples/tour/01-hello.alea")).unwrap();
        let drawn = alea::eval::resolve(&module, 1).unwrap();
        Edition {
            seed: 1,
            name: "t".into(),
            tempo: 120.0,
            beats,
            msgs: msgs.into_iter().map(|(beat, bytes)| Msg { beat, track: 0, len: 3, bytes }).collect(),
            roll: vec![],
            tracks: vec![],
            drawn: Arc::new(drawn),
            generation: 0,
        }
    }

    fn run(p: &mut Player, ed: &Edition, from: f64, blocks: usize, every: u32) -> Vec<(f64, [u8; 3])> {
        // 120 BPM at 48 kHz: 2 beats a second, blocks of 512 samples
        let bps = 2.0 / 48_000.0;
        let mut all = Vec::new();
        let mut beat = from;
        for _ in 0..blocks {
            let mut out = Vec::new();
            p.block(Block { playing: true, beat, beats_per_sample: bps, samples: 512, bar: 4.0, layers: ALL }, every, 1, |_| Some(ed), &mut out);
            for (s, m) in out {
                all.push((beat + s as f64 * bps, m.bytes));
            }
            beat += 512.0 * bps;
        }
        all
    }

    #[test]
    fn notes_at_their_beats() {
        let ed = edition(4.0, vec![(0.0, [0x90, 60, 100]), (1.0, [0x80, 60, 0]), (2.5, [0x90, 64, 90]), (3.0, [0x80, 64, 0])]);
        let mut p = Player::default();
        let got = run(&mut p, &ed, 0.0, 210, 0); // 2.24 seconds: 4.48 beats
        let ons: Vec<(f64, u8)> = got.iter().filter(|m| m.1[0] == 0x90).map(|m| (m.0, m.1[1])).collect();
        assert_eq!(ons.len(), 3, "{got:?}"); // 60, 64, then 60 again at the loop
        assert!((ons[1].0 - 2.5).abs() < 1e-3);
        assert!((ons[2].0 - 4.0).abs() < 1e-3);
    }

    #[test]
    fn a_loop_ends_its_notes() {
        // a note held across the end: the loop ends it before starting again
        let ed = edition(2.0, vec![(1.5, [0x90, 67, 100])]);
        let mut p = Player::default();
        let got = run(&mut p, &ed, 0.0, 120, 0);
        let off = got.iter().position(|m| m.1 == [0x80, 67, 0]).expect("ended");
        assert!((got[off].0 - 2.0).abs() < 1e-3, "{got:?}");
    }

    #[test]
    fn a_jump_or_a_stop_silences() {
        let ed = edition(8.0, vec![(0.0, [0x90, 60, 100])]);
        let mut p = Player::default();
        run(&mut p, &ed, 0.0, 2, 0);
        let mut out = Vec::new();
        p.block(Block { playing: true, beat: 5.0, beats_per_sample: 1e-4, samples: 64, bar: 4.0, layers: ALL }, 0, 1, |_| Some(&ed), &mut out);
        assert_eq!(out[0].1.bytes, [0x80, 60, 0]);
        let mut out = Vec::new();
        run(&mut p, &ed, 0.0, 1, 0);
        p.block(Block { playing: false, beat: 0.0, beats_per_sample: 1e-4, samples: 64, bar: 4.0, layers: ALL }, 0, 1, |_| Some(&ed), &mut out);
        assert_eq!(out.iter().filter(|m| m.1.bytes[0] == 0x80).count(), 1);
    }

    #[test]
    fn a_real_track_in_a_host() {
        // tour 26, a new edition every 8 bars, in a host at 120 BPM (the work has its own tempo)
        let module = alea::library::load(concat!(env!("CARGO_MANIFEST_DIR"), "/../examples/tour/26-harmony.alea")).unwrap();
        let eds: HashMap<u64, Edition> = (5..8)
            .map(|s| (s, Edition::from(alea::eval::resolve(&module, s).unwrap(), 0).unwrap()))
            .collect();
        let mut p = Player::default();
        let (sr, tempo) = (48_000.0, 120.0);
        let bps = tempo / 60.0 / sr;
        let mut beat = 0.0;
        let mut sent: Vec<(f64, [u8; 3])> = Vec::new();
        while beat < 4.0 * 20.0 {
            let mut out = Vec::new();
            p.block(Block { playing: true, beat, beats_per_sample: bps, samples: 256, bar: 4.0, layers: ALL }, 8, 5, |s| eds.get(&s), &mut out);
            for (k, m) in out {
                sent.push((beat + k as f64 * bps, m.bytes));
            }
            beat += 256.0 * bps;
        }
        let mut out = Vec::new();
        p.block(Block { playing: false, beat, beats_per_sample: bps, samples: 256, bar: 4.0, layers: ALL }, 8, 5, |s| eds.get(&s), &mut out);
        for (_, m) in out {
            sent.push((beat, m.bytes));
        }
        // every note that starts also ends
        let mut held: HashMap<(u8, u8), i32> = HashMap::new();
        for (_, b) in &sent {
            match b[0] & 0xF0 {
                0x90 if b[2] > 0 => *held.entry((b[0] & 15, b[1])).or_default() += 1,
                0x80 => *held.entry((b[0] & 15, b[1])).or_default() -= 1,
                _ => {}
            }
        }
        assert!(held.values().all(|v| *v == 0), "notes left on: {:?}", held.iter().filter(|x| *x.1 != 0).collect::<Vec<_>>());
        // the first edition's first notes at their beats, the second edition from bar 9
        let first = &eds[&5];
        let on: Vec<f64> = sent.iter().filter(|m| m.1[0] & 0xF0 == 0x90 && m.1[2] > 0).map(|m| m.0).collect();
        let want: Vec<f64> = first.msgs.iter().filter(|m| m.bytes[0] & 0xF0 == 0x90 && m.bytes[2] > 0).map(|m| m.beat).take(20).collect();
        for (a, b) in on.iter().zip(&want) {
            assert!((a - b).abs() < 2.0 * bps * 256.0 / 256.0 + 1e-6, "{a} vs {b}");
        }
        let second = &eds[&6];
        let first_of_second = second.msgs.iter().find(|m| m.bytes[0] & 0xF0 == 0x90 && m.bytes[2] > 0).unwrap().beat;
        assert!(on.iter().any(|b| (b - (32.0 + first_of_second)).abs() < 1e-3), "edition 6 begins at bar 9");
    }

    #[test]
    fn a_layer_switched_off() {
        // layer 0: a long note; layer 1: short notes on every beat
        let mut ed = edition(4.0, vec![(0.0, [0x90, 48, 100]), (0.0, [0x99, 36, 100]), (0.5, [0x89, 36, 0]), (1.0, [0x99, 36, 100]), (1.5, [0x89, 36, 0]), (3.9, [0x80, 48, 0])]);
        for m in ed.msgs.iter_mut().filter(|m| m.bytes[0] & 0x0F == 9) {
            m.track = 1;
        }
        let mut on = [true; LAYERS];
        on[1] = false;
        let mut p = Player::default();
        let mut out = Vec::new();
        p.block(Block { playing: true, beat: 0.0, beats_per_sample: 1e-3, samples: 2000, bar: 4.0, layers: layers(&on, 0) }, 0, 1, |_| Some(&ed), &mut out);
        assert!(out.iter().any(|(_, m)| m.bytes == [0x90, 48, 100]), "the other layer plays");
        assert!(out.iter().all(|(_, m)| m.track != 1), "nothing from the switched-off layer");
        // switching the sounding layer off ends its note at once
        on = [true; LAYERS];
        on[0] = false;
        out.clear();
        p.block(Block { playing: true, beat: 2.0, beats_per_sample: 1e-3, samples: 10, bar: 4.0, layers: layers(&on, 0) }, 0, 1, |_| Some(&ed), &mut out);
        assert_eq!(out.first().map(|(at, m)| (*at, m.bytes)), Some((0, [0x80, 48, 0])), "its note ends");
        // a solo of a switched-off layer stays silent, as in a mixer
        assert_eq!(layers(&on, 1), 0);
        assert_eq!(layers(&[true; LAYERS], 2), 0b10);
    }

    #[test]
    fn one_layer_only() {
        let mut ed = edition(4.0, vec![(0.0, [0x90, 60, 100]), (0.0, [0x99, 36, 100]), (1.0, [0x80, 60, 0]), (1.0, [0x89, 36, 0])]);
        ed.msgs[1].track = 1;
        ed.msgs[3].track = 1;
        let mut p = Player::default();
        let mut out = Vec::new();
        p.block(Block { playing: true, beat: 0.0, beats_per_sample: 1e-3, samples: 2000, bar: 4.0, layers: layers(&[true; LAYERS], 2) }, 0, 1, |_| Some(&ed), &mut out);
        let notes: Vec<u8> = out.iter().map(|m| m.1.bytes[1]).collect();
        assert_eq!(notes, vec![36, 36], "only the drum layer");
    }

    #[test]
    fn new_editions_every_n_bars() {
        assert_eq!(place(0.0, 4.0, 2, 10, Some(16.0)).0, 10);
        assert_eq!(place(7.9, 4.0, 2, 10, Some(16.0)).0, 10);
        assert_eq!(place(8.0, 4.0, 2, 10, Some(16.0)).0, 11);
        // an edition longer than its span is cut at the span
        let (_, pos, to_next) = place(6.0, 4.0, 2, 10, Some(16.0));
        assert_eq!((pos, to_next), (6.0, 2.0));
        // a shorter one loops inside it
        let (_, pos, to_next) = place(5.0, 4.0, 2, 10, Some(3.0));
        assert_eq!((pos, to_next), (2.0, 1.0));
    }
}

#[cfg(test)]
mod picture_tests {
    use super::*;
    #[test]
    fn tour_26_has_a_picture() {
        let module = alea::library::load(concat!(env!("CARGO_MANIFEST_DIR"), "/../examples/tour/26-harmony.alea")).unwrap();
        let ed = Edition::from(alea::eval::resolve(&module, 0xbe2c).unwrap(), 0).unwrap();
        let px = picture(&ed, 0.3, 180, &mut HashMap::new());
        assert!(px.is_some_and(|p| p.len() == 180 * 180 * 3));
    }
}
