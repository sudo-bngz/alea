# alea as a plugin (VST3 and CLAP)

A work of alea, played in time with your DAW: the notes of its music come out
of the plugin as MIDI, for the instruments after it. Every edition is a
different piece; the plugin can draw a new one every few bars.

## Build and install

**macOS** (Ableton Live, Bitwig, Reaper):

```sh
./plugin/build-macos.sh
```

It builds one bundle for Apple silicon and Intel, installs `alea.vst3` in
`~/Library/Audio/Plug-Ins/VST3` and `alea.clap` in `…/CLAP`, and signs them
locally. It needs the Xcode command line tools (`xcode-select --install`) and
Rust (<https://rustup.rs>).

**Linux and Windows**:

```sh
cd plugin
cargo xtask bundle alea_plugin --profile plugin     # → target/bundled/alea.vst3 and alea.clap
```

On Linux the window needs the X11 and OpenGL development packages
(`libx11-dev libx11-xcb-dev libxcb1-dev libxcb-dri2-0-dev libxcb-icccm4-dev
libxcursor-dev libxkbcommon-dev libgl1-mesa-dev`).

## In Ableton Live

1. *Settings → Plug-Ins*: turn on *Use VST3 Plug-in System Folders*, then *Rescan*.
2. Drop **alea** (*Plug-ins → VST3*) on a MIDI track. Open its window and choose
   a work (*Open…*), for example `examples/tour/26-harmony.alea`.
3. Make a second MIDI track with an instrument. Set its *MIDI From* to the alea
   track, and below that to *alea*; set *Monitor* to *In*.
4. Press play.

### One instrument per layer: one alea, many tracks

Live mixes the MIDI channels together when one track listens to another, so
alea sends its layers another way: through a MIDI port of its own, each
layer on its own channel (layer 1 on channel 1, layer 2 on channel 2…). Live
sees that port like a keyboard plugged in.

1. Put one alea on a MIDI track, load the work, and turn on **Layers to
   tracks**. The window shows each layer's channel (`ch 1`, `ch 2`…).
2. The first time only: in Live's *Settings › Link, Tempo & MIDI*, under
   *MIDI Ports*, find *Input: alea* and turn **Track** on. (If it is not
   listed, restart Live once with an alea in the set.)
3. For each layer, a MIDI track with its instrument: *MIDI From*: **alea**,
   then the channel of its layer (*Ch. 3* for layer 3), *Monitor*: **In**.

**If the tracks stay silent**, the window has a **Test** button (a C on
every channel, at once) and counters: *queued* (notes alea prepared), *sent*
(notes the system accepted) and *refused* (with the reason). Or send through
macOS's own MIDI bus instead: open *Audio MIDI Setup*, *Window › Show MIDI
Studio*, double-click *IAC Driver*, tick *Device is online*; then choose
*IAC Driver Bus 1* in alea's **Send to**, turn its *Track* on in Live's
settings, and set the tracks' *MIDI From* to it.

The switches and solos still work: they decide what goes out on the port.
Notes go to the port a buffer early, since Live plays what comes into a MIDI
input one buffer later; if they sound early or late, turn **Port timing**
(± 50 ms), or use Live's *Track Delay*. Leave the instrument tracks'
*MIDI From* off *All Ins*, or they hear every layer. Two alea with the port
on open *alea* and *alea 2*. The port needs macOS or Linux (on Windows, use
one alea per layer, below).

### Or one alea per layer

Duplicate the alea track (⌘D) once per instrument and press the **S** (solo)
of a different layer on each. For each, a MIDI track with its instrument,
*MIDI From* that alea track (and *alea* below), *Monitor: In*. This is
sample-accurate, and works everywhere.

With **Link** on (the default), every alea on the same work plays the same
edition: change the edition, *New edition every*, or the macros on any of
them, and the others follow. They all follow Live's clock, so they stay
together to the sample.

## What it does

- **In time with the host.** The work's notes are placed in beats and follow
  the host's tempo, playhead, loop and time signature. Moving the playhead
  lands in the same place of the same edition: editions are deterministic.
  A stop, a jump or a loop silences the notes still on.
- **Edition**: which edition plays (automatable). *New edition* draws a new
  one.
- **New edition every N bars**: edition, edition + 1, … every N bars, each
  prepared while the previous one plays. A work shorter than N bars loops
  inside; a longer one is cut at N bars. *Never* loops one edition.
- **Layers**, as in Live's mixer: each layer has a switch (its number,
  amber when on) and a solo (**S**). Switch layers off and on while it
  plays, to build up or strip down the piece; a layer switched off ends its
  notes at once. Cmd/Ctrl-click a switch to keep only that layer; *all on*
  resets everything. The switches (*Layer 1* … *Layer 16*) and the solo are
  parameters, so Live can automate them or map them to a controller. A
  soloed layer that is switched off stays silent, as in Live. The roll dims
  what is not heard.
- **Link**: every alea on the same work file plays the same edition, with the
  same macros (on by default).
- **Macros**: when on, eight knobs give values from 0 to 1 to the work's first
  eight parameters (`work Track(density = 0.5, tension = 0.4)`), in order,
  automatable; the window shows which knob is which. The editions are drawn
  again when a knob moves (the old one plays until the new one is ready).
- **Live reload**: save the `.alea` file in your editor, and the plugin
  draws it again while it plays. If the work has an error, the last good
  version keeps playing and the window shows the error.
- **The window** shows the edition's picture moving with the music (the
  work's first video, or its first image), its notes as a piano roll with
  the playhead, and its layers.

What plays is the work's `render midi`, or, if it has none, the layers of its
first `render sound` or video `sound:`. The plugin is an instrument with a
silent audio output (so that hosts accept it on a MIDI track); alea's own
sound is not played yet.

## The window

The knobs turn by dragging up or down (hold Shift for fine steps), by
scrolling over them, and back to their default with a double-click. Live
records them as automation like any other parameter. Drag the corner at
the bottom right to resize the window; if it is smaller than its
contents, scroll.

## Without a host

```sh
cd plugin
cargo run --release --features standalone --bin alea_standalone -- --help
ALEA_WORK=../examples/tour/26-harmony.alea cargo run --release --features standalone --bin alea_standalone
```

## Checked

- Steinberg's VST3 validator: 47 tests passed, 0 failed (Linux build).
- clap-validator: the plugin's own tests pass; the state tests fail the same
  way for NIH-plug's own example plugins with this version of the validator.
- The scheduler is tested with a simulated host: notes at their beats, loops,
  jumps and stops silencing, a new edition every N bars, and a real track
  (tour 26) at another tempo with no note left hanging.

## Licence

The VST3 build uses NIH-plug's VST3 bindings, which are GPLv3: the plugin is
distributed under the GPL-3.0-or-later.
