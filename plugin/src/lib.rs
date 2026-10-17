//! alea as a plugin (VST3 and CLAP): load a work, and its music plays in time
//! with the host, a new edition every few bars if you like. The notes come out
//! as MIDI, for the instruments after it (in Ableton Live: a MIDI track whose
//! "MIDI From" is this plugin).

use std::collections::HashMap;
use std::num::NonZeroU32;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use nih_plug::prelude::*;
use nih_plug_egui::EguiState;

pub mod editor;
pub mod engine;
pub mod knob;
pub mod ports;

use engine::{Block, Edition, Group, Player, Shared, GROUPS, LAYERS, LAYER_NAMES, MACROS};

pub struct Alea {
    params: Arc<AleaParams>,
    shared: Arc<Shared>,
    player: Player,
    /// The editions this block may play (copied from the shared ones).
    playing: HashMap<u64, Arc<Edition>>,
    out: engine::Out,
    sample_rate: f32,
    max_buffer: u32,
    /// Playing ahead for the port (its notes go through Live's MIDI input,
    /// which plays them a buffer later).
    port_player: Player,
    port_out: engine::Out,
    port_tx: std::sync::mpsc::SyncSender<ports::Timed>,
    port_expected: Option<f64>,
    port_was_on: bool,
    threads: Vec<std::thread::JoinHandle<()>>,
    /// The work this instance plays (its group's key), and what it last asked for.
    key: String,
    last_own: Option<Group>,
    last_group: Option<Group>,
}

#[derive(Params)]
pub struct AleaParams {
    #[persist = "editor-state"]
    pub editor_state: Arc<EguiState>,
    /// The work's file, saved with the project.
    #[persist = "work-path"]
    pub path: Arc<std::sync::RwLock<String>>,
    /// Where "Layers to tracks" sends: "" for alea's own port, or an existing port.
    #[persist = "port-target"]
    pub port_target: Arc<std::sync::RwLock<String>>,

    #[id = "seed"]
    pub seed: IntParam,
    #[id = "every"]
    pub every: IntParam,
    #[id = "macros"]
    pub macros_on: BoolParam,
    /// The soloed layer (0: none). Saved projects call it "layer": soloing one
    /// layer is what it always did.
    #[id = "layer"]
    pub layer: IntParam,
    /// Follow the other alea on the same work: one edition for all of them.
    #[id = "link"]
    pub link: BoolParam,
    /// Send each layer, on its own channel, to the plugin's own MIDI port.
    #[id = "ports"]
    pub ports: BoolParam,
    /// Earlier (−) or later (+) for the port, in milliseconds.
    #[id = "port-timing"]
    pub port_timing: FloatParam,
    #[nested(array, group = "Macros")]
    pub macros: [MacroParam; MACROS],
    /// Each layer's switch, like a track's activator in a mixer.
    #[nested(array, group = "Layers")]
    pub layers: [LayerParam; LAYERS],
}

#[derive(Params)]
pub struct LayerParam {
    #[id = "on"]
    pub on: BoolParam,
}

#[derive(Params)]
pub struct MacroParam {
    #[id = "macro"]
    pub value: FloatParam,
}

impl Default for AleaParams {
    fn default() -> Self {
        AleaParams {
            editor_state: EguiState::from_size(860, 460),
            path: Arc::new(std::sync::RwLock::new(String::new())),
            port_target: Arc::new(std::sync::RwLock::new(String::new())),
            seed: IntParam::new("Edition", 0, IntRange::Linear { min: 0, max: 65535 })
                .with_value_to_string(Arc::new(|v| format!("0x{v:x}")))
                .with_string_to_value(Arc::new(|s| {
                    let s = s.trim();
                    match s.strip_prefix("0x") {
                        Some(h) => i32::from_str_radix(h, 16).ok(),
                        None => s.parse().ok(),
                    }
                })),
            every: IntParam::new("New edition every", 0, IntRange::Linear { min: 0, max: 64 })
                .with_value_to_string(Arc::new(|v| if v == 0 { "never (loop)".into() } else { format!("{v} bar{}", if v == 1 { "" } else { "s" }) }))
                .with_string_to_value(Arc::new(|s| {
                    let s = s.trim();
                    if s.starts_with("never") || s == "off" {
                        return Some(0);
                    }
                    s.trim_end_matches("bars").trim_end_matches("bar").trim().parse().ok()
                })),
            macros_on: BoolParam::new("Macros", false),
            layer: IntParam::new("Solo", 0, IntRange::Linear { min: 0, max: 32 })
                .with_value_to_string(Arc::new(|v| {
                    if v == 0 {
                        return "none".into();
                    }
                    match LAYER_NAMES.try_lock().ok().and_then(|n| n.get(v as usize - 1).cloned()) {
                        Some(name) => format!("{v} · {name}"),
                        None => format!("layer {v}"),
                    }
                }))
                .with_string_to_value(Arc::new(|s| {
                    let s = s.trim();
                    if s.starts_with("all") || s.starts_with("none") {
                        return Some(0);
                    }
                    s.trim_start_matches("layer").trim().split(|c: char| !c.is_ascii_digit()).next().and_then(|d| d.parse().ok())
                })),
            link: BoolParam::new("Link", true),
            ports: BoolParam::new("Layers to tracks", false),
            port_timing: FloatParam::new("Port timing", 0.0, FloatRange::Linear { min: -50.0, max: 50.0 })
                .with_step_size(0.5)
                .with_unit(" ms"),
            macros: std::array::from_fn(|k| MacroParam {
                value: FloatParam::new(format!("Macro {}", k + 1), 0.5, FloatRange::Linear { min: 0.0, max: 1.0 }).with_step_size(0.001),
            }),
            layers: std::array::from_fn(|k| LayerParam {
                on: BoolParam::new(format!("Layer {}", k + 1), true)
                    .with_value_to_string(Arc::new(|v| if v { "on".into() } else { "off".into() }))
                    .with_string_to_value(Arc::new(|s| match s.trim().to_ascii_lowercase().as_str() {
                        "on" | "1" | "true" => Some(true),
                        "off" | "0" | "false" => Some(false),
                        _ => None,
                    })),
            }),
        }
    }
}

impl Default for Alea {
    fn default() -> Self {
        let shared = Arc::new(Shared::default());
        Alea {
            params: Arc::new(AleaParams::default()),
            shared: shared.clone(),
            player: Player::default(),
            playing: HashMap::with_capacity(8),
            out: Vec::with_capacity(1024),
            sample_rate: 48_000.0,
            max_buffer: 512,
            port_player: Player::default(),
            port_out: Vec::with_capacity(1024),
            port_tx: shared.port_tx.clone(),
            port_expected: None,
            port_was_on: false,
            threads: Vec::new(),
            key: String::with_capacity(256),
            last_own: None,
            last_group: None,
        }
    }
}

impl Alea {
    fn send(context: &mut impl ProcessContext<Self>, timing: u32, m: &engine::Msg) {
        let (kind, channel) = (m.bytes[0] & 0xF0, m.bytes[0] & 0x0F);
        let event = match kind {
            0x90 if m.bytes[2] > 0 => NoteEvent::NoteOn { timing, voice_id: None, channel, note: m.bytes[1], velocity: m.bytes[2] as f32 / 127.0 },
            0x80 | 0x90 => NoteEvent::NoteOff { timing, voice_id: None, channel, note: m.bytes[1], velocity: 0.0 },
            0xB0 => NoteEvent::MidiCC { timing, channel, cc: m.bytes[1], value: m.bytes[2] as f32 / 127.0 },
            0xC0 => NoteEvent::MidiProgramChange { timing, channel, program: m.bytes[1] },
            0xE0 => NoteEvent::MidiPitchBend { timing, channel, value: ((m.bytes[2] as u32) << 7 | m.bytes[1] as u32) as f32 / 16383.0 },
            _ => return,
        };
        context.send_event(event);
    }
}

impl Plugin for Alea {
    const NAME: &'static str = "alea";
    const VENDOR: &'static str = "alea";
    const URL: &'static str = "https://github.com/";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    // An instrument (stereo, silent for now) so that hosts accept it on a MIDI track.
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] =
        &[AudioIOLayout { main_input_channels: None, main_output_channels: NonZeroU32::new(2), ..AudioIOLayout::const_default() }];

    // Hosts (Ableton Live among them) only accept an instrument with a MIDI
    // input. Incoming notes are not used yet: they are read and dropped.
    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::MidiCCs;
    const SAMPLE_ACCURATE_AUTOMATION: bool = false;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        editor::create(self.params.clone(), self.shared.clone())
    }

    fn initialize(&mut self, _layout: &AudioIOLayout, buffer_config: &BufferConfig, _context: &mut impl InitContext<Self>) -> bool {
        self.sample_rate = buffer_config.sample_rate;
        self.max_buffer = buffer_config.max_buffer_size.max(1);
        let mut path = self.params.path.read().unwrap().clone();
        if path.is_empty() {
            // for trying it out: a work given by the environment
            if let Ok(p) = std::env::var("ALEA_WORK") {
                path = p.clone();
                *self.params.path.write().unwrap() = p;
            }
        }
        *self.shared.path.lock().unwrap() = path;
        *self.shared.port.target.lock().unwrap() = self.params.port_target.read().unwrap().clone();
        if self.threads.is_empty() {
            let (a, b) = (self.shared.clone(), self.shared.clone());
            self.threads.push(std::thread::Builder::new().name("alea worker".into()).spawn(move || engine::worker(a)).expect("thread"));
            self.threads.push(std::thread::Builder::new().name("alea painter".into()).spawn(move || engine::painter(b)).expect("thread"));
            let c = self.shared.clone();
            self.threads.push(std::thread::Builder::new().name("alea port".into()).spawn(move || ports::run(&c.port, &c.quit)).expect("thread"));
        }
        true
    }

    fn reset(&mut self) {
        let mut out = Vec::new();
        self.player.stop(&mut out);
        self.port_player.stop(&mut out);
        self.port_expected = None;
    }

    fn process(&mut self, buffer: &mut Buffer, _aux: &mut AuxiliaryBuffers, context: &mut impl ProcessContext<Self>) -> ProcessStatus {
        for channel in buffer.as_slice() {
            channel.fill(0.0);
        }
        while context.next_event().is_some() {}
        // what this instance asks for, and what its group (the other alea on
        // the same work, when linked) plays
        let own = Group {
            seed: self.params.seed.value(),
            every: self.params.every.value(),
            macros_on: self.params.macros_on.value(),
            macros: std::array::from_fn(|k| self.params.macros[k].value.value()),
        };
        if let Ok(p) = self.shared.path.try_lock() {
            if *p != self.key {
                self.key.clear();
                self.key.push_str(&p);
                self.last_own = None;
            }
        }
        let mut eff = own;
        if self.params.link.value() && !self.key.is_empty() {
            match GROUPS.try_lock() {
                Ok(mut groups) => {
                    let exists = groups.contains_key(&self.key);
                    let changed_here = self.last_own.is_some_and(|l| l != own);
                    // a change made here leads the group; a newcomer follows it
                    if changed_here || !exists {
                        groups.insert(self.key.clone(), own);
                    }
                    if let Some(g) = groups.get(&self.key) {
                        eff = *g;
                    }
                    self.last_group = Some(eff);
                }
                Err(_) => eff = self.last_group.unwrap_or(own),
            }
            if let Ok(mut g) = self.shared.group.try_lock() {
                *g = Some(eff);
            }
        } else if let Ok(mut g) = self.shared.group.try_lock() {
            *g = None;
        }
        self.last_own = Some(own);
        // the parameters the worker draws with
        for k in 0..MACROS {
            self.shared.macros[k].store(eff.macros[k].to_bits(), Ordering::Relaxed);
        }
        self.shared.macros_on.store(eff.macros_on, Ordering::Relaxed);

        let t = context.transport();
        let transport_playing = t.playing;
        let tempo = t.tempo.unwrap_or(120.0);
        let beat = t.pos_beats().or_else(|| t.pos_seconds().map(|s| s * tempo / 60.0)).unwrap_or(0.0);
        let bar = match (t.time_sig_numerator, t.time_sig_denominator) {
            (Some(n), Some(d)) if n > 0 && d > 0 => n as f64 * 4.0 / d as f64,
            _ => 4.0,
        };
        let block = Block {
            playing: transport_playing,
            beat,
            beats_per_sample: tempo / 60.0 / self.sample_rate as f64,
            samples: buffer.samples() as u32,
            bar,
            layers: {
                let on: [bool; LAYERS] = std::array::from_fn(|k| self.params.layers[k].on.value());
                engine::layers(&on, self.params.layer.value())
            },
        };
        let every = eff.every.max(0) as u32;
        let base = eff.seed.max(0) as u64;

        // the editions this block needs (this one, and the next for a new edition)
        let here = engine::place(beat, bar, every, base, None).0;
        let generation = self.shared.generation.load(Ordering::Relaxed);
        for seed in [here, here + 1] {
            if seed == here + 1 && every == 0 {
                break;
            }
            let fresh = match self.shared.editions.try_lock() {
                Ok(eds) => match eds.get(&seed) {
                    Some(e) => {
                        let current = e.generation == generation;
                        self.playing.insert(seed, e.clone());
                        current
                    }
                    None => false,
                },
                Err(_) => true, // the worker is busy: ask next time
            };
            if !fresh {
                self.shared.want(seed);
            }
        }
        if self.playing.len() > 4 {
            self.playing.retain(|s, _| s.abs_diff(here) <= 1);
        }

        self.out.clear();
        let playing = &self.playing;
        self.player.block(block, every, base, |s| playing.get(&s).map(|e| e.as_ref()), &mut self.out);
        for (timing, m) in &self.out {
            Self::send(context, *timing, m);
        }

        // the same music, each layer on its channel, to the plugin's own port
        let ports_on = self.params.ports.value();
        self.shared.port.on.store(ports_on, Ordering::Relaxed);
        if ports_on || self.port_was_on {
            let now = std::time::Instant::now();
            let sr = self.sample_rate as f64;
            let n = buffer.samples() as u32;
            let mut offset = 0i64;
            if ports_on {
                // a buffer ahead (and the timing knob), since Live plays what
                // comes into its MIDI input in its next buffer
                let ahead = (self.max_buffer as f64 + self.params.port_timing.value() as f64 * sr / 1000.0).max(0.0).round() as u32;
                let (port_block, o) = ports::ahead(block, self.port_expected, ahead);
                offset = o;
                self.port_out.clear();
                self.port_player.block(port_block, every, base, |s| playing.get(&s).map(|e| e.as_ref()), &mut self.port_out);
            } else {
                self.port_out.clear();
                self.port_player.stop(&mut self.port_out);
            }
            for (at, m) in &self.port_out {
                let delay = (*at as i64 + offset).max(0) as f64 / sr;
                self.shared.port.queued.fetch_add(1, Ordering::Relaxed);
                let _ = self.port_tx.try_send(ports::Timed { at: now + std::time::Duration::from_secs_f64(delay), len: m.len, bytes: ports::rechannel(m.bytes, m.track) });
            }
            self.port_expected = transport_playing.then(|| beat + n as f64 * block.beats_per_sample);
            self.port_was_on = ports_on;
        }

        // for the window
        let length = self.playing.get(&here).map(|e| e.beats);
        let (_, pos, _) = engine::place(beat, bar, every, base, length);
        self.shared.now_seed.store(here, Ordering::Relaxed);
        self.shared.now_beat.store(pos.to_bits(), Ordering::Relaxed);
        self.shared.now_playing.store(transport_playing, Ordering::Relaxed);
        ProcessStatus::Normal
    }

    fn deactivate(&mut self) {
        self.shared.quit.store(true, Ordering::Relaxed);
        for t in self.threads.drain(..) {
            let _ = t.join();
        }
        self.shared.quit.store(false, Ordering::Relaxed);
    }
}

impl ClapPlugin for Alea {
    const CLAP_ID: &'static str = "org.alea.alea";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("A work of alea, played in time: its notes as MIDI, a new edition every few bars");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::Instrument, ClapFeature::NoteEffect, ClapFeature::Utility];
}

impl Vst3Plugin for Alea {
    const VST3_CLASS_ID: [u8; 16] = *b"aleaGenerativeV1";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Instrument, Vst3SubCategory::Generator];
}

nih_export_clap!(Alea);
nih_export_vst3!(Alea);
