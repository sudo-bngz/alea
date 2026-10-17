# alea

[![CI](https://github.com/sudo-bngz/alea/actions/workflows/ci.yml/badge.svg)](https://github.com/sudo-bngz/alea/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/sudo-bngz/alea)](https://github.com/sudo-bngz/alea/releases/latest)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

**[alea.bngz.io](https://alea.bngz.io)** · [Playground](https://alea.bngz.io/playground.html) · [Handbook](https://alea.bngz.io/handbook.html) · [Instagram @sudo.mp3](https://www.instagram.com/sudo.mp3/)

A language whose programs don't compile to a binary but to an **artwork**: a
space of possibilities. Every run draws an **edition** from it and renders it
as an image, a sound, a video, or a printable sculpture.

```alea
work Moon {
  let moon  = circle(size: ~between(0.25, 0.4))   // where? alea decides
  let shade = move(moon, x: 0.08)                  // the same circle, nudged right
  render image(merge(clouds() * 0.25, cut(moon, shade)), palette: "night")
}
```

Run it twice and you get two different moons. Each edition has a seed, and the
same seed gives back exactly the same files, on any machine and in the browser.

## Install

**Try it without installing anything:** the [playground](https://alea.bngz.io/playground.html) runs alea in the browser.

**macOS and Linux, with Homebrew:**

```sh
brew install sudo-bngz/tap/alea
```

**Any platform:** download the archive for your system from the
[latest release](https://github.com/sudo-bngz/alea/releases/latest)
(macOS, Linux x86_64 and arm64, Windows), unpack it, and put `alea` on your
`PATH`. On macOS, the first time: `xattr -d com.apple.quarantine alea`.

**With Rust:**

```sh
cargo install alea-lang                  # the command is `alea`
cargo install alea-lang --features midi  # with live MIDI (Linux: libasound2-dev first)
```

**The plugin** (VST3 and CLAP, for Ableton Live, Bitwig, Reaper…): the
`alea-plugin-…` archive of the release. See [plugin/README.md](plugin/README.md).

**Editor:** syntax highlighting for VS Code and compatible editors in
[editors/vscode](editors/vscode/).

Videos and Reels need [ffmpeg](https://ffmpeg.org).

## Start here

| | |
|---|---|
| **[The guide](docs/guide.md)** | learn the language, one concept per chapter |
| **[The tour](examples/tour/)** | twenty-six small works that go with the guide, from `01-hello` to `26-harmony` |
| **[The reference](docs/reference.md)** | every statement, primitive, function, render option and limit |
| **The handbook** (`web/dist/handbook.html`) | the guide and the reference as one illustrated web page |
| **The playground** (`web/dist/playground.html`) | write and run alea in the browser, with nothing installed |
| **[The gallery](examples/)** | finished works: `rosace`, `moon`, `phases`, `poster`, `tide`, `constellation`… |
| **[The exhibition](exhibition/)** | larger works, made for Instagram: `isohypses`, `correspondances`, `culture`… |

## Quick start

From a clone of the repository:

```sh
cargo build --release
./target/release/alea examples/tour/01-hello.alea         # one edition
./target/release/alea examples/tide.alea -n 5             # five editions
./target/release/alea examples/tide.alea --seed 0x2a      # replay edition 0x2a
./target/release/alea examples/tide.alea --check          # check, write nothing
./target/release/alea examples/moon.alea --web moon.html  # a page: a new edition per visit
```

As a plugin in a DAW (Ableton Live, Bitwig, Reaper…): `./plugin/build-macos.sh`
on macOS, or see [plugin/README.md](plugin/README.md).

`video` renders need [ffmpeg](https://ffmpeg.org). Live MIDI (`--midi`) needs
`cargo build --release --features midi`, and on Linux ALSA's headers first
(`sudo apt install libasound2-dev`).

## The language in one screen

```alea
use "lib/flower.alea"                      // other works, used as shapes

work Garden(petals = choose { 5 | 8 }) {   // parameters, for when another work calls this one
  let size  = ~between(0.1, 0.3)           // ~ draws from a distribution, once
  let grain = ~~uniform(0, 1) * 0.1        // ~~ draws at every point
  let shape = choose { 2: circle() | 1: blob() }   // a structural choice
  let ring  = kaleidoscope(shape, sides: petals)   // primitives: shapes, textures, transforms

  let star(cx, cy) = circle(cx, cy, ~between(0.01, 0.03))   // your own functions
  let crown = [star(0.5 + 0.4 * cos(k), 0.5 + 0.4 * sin(k)) for k in range(12)]   // lists, built by loops
  pattern stars = repeat ~poisson(80) event(size: ~between(0.002, 0.006))
  constraint count(stars) > 60             // every edition must satisfy this

  grow over 128 for 300 {                  // fields with a memory: each generation from the last
    cells = ~~uniform(0, 1) < 0.3
      then neighbours(cells) == 3 or (cells and neighbours(cells) == 2)
  }

  output merge(ring, merge(crown), stars) + grain   // what callers receive
  render image(output, palette: ["#0b1d3a", "#e94f37", "#f6f7eb"], bands: 6)   // palettes: a name or a list
  render image(mosaic([paint(output, palette: "night"), paint(merge(crown), palette: "ember")]))   // paintings, side by side
  render video(rotate(output, angle: t * 360), duration: 8)   // t is time
  render video(bloom(paint(at(output, ease(t, "out")))), duration: 8)   // time eased, filters on the pixels
}
```

Works read what they are given and write to the pipe, so they chain:

```sh
cat portrait.png | alea contours.alea > map.png              # input(), photo.luma, colours(photo, 6)
ffmpeg -i song.mp3 -f wav - | alea equaliser.alea -o .      # song.bass, song.pulse, sound: [song]
cat a.png | alea soften.alea | alea poster.alea > print.png
alea modes.alea --midi --loop                                # live MIDI, a new edition each time (--features midi)
```

and make music in bars:

```alea
let m    = meter(100, bars: 8)
let prog = chords("functional", key: "D3", scale: "dorian", bars: m)   // a new progression each edition
render midi([drone(sky, harmony: prog), arp(prog, euclid(7, 16, bars: m), mode: "updown")], duration: m.length)
```

## Architecture

```
source + used files ──lexer/parser──▶ AST (Work) ──resolve──▶ Edition ──render──▶ .png .wav .mp4 .stl .mid
                                      space of possibilities  one draw
```

| File | Role |
|---|---|
| `lexer.rs` | tokens and source positions |
| `parser.rs` | recursive descent (statements) + Pratt (expressions) |
| `ast.rs` | what the artist wrote, before any draw |
| `eval.rs` | `Work → Edition`: draws, choices, constraints, resolved fields; the distributions |
| `prims.rs` | artist-level primitives, each lowered to an ordinary field |
| `raster.rs` | text rasterization (embedded fonts) and picture loading, as fields |
| `input.rs` | inputs from the pipe or `--in`: images (channels, palettes), sounds (loudness, bands, beats), texts |
| `library.rs` | loads a work and the works it `use`s; detects cycles |
| `media.rs` | what each medium computes: frames, colours, sound synthesis, meshes |
| `bounds.rs` | tile culling: proves, tile by tile, which parts of a field can't change there, and skips them |
| `compile.rs` | compiles a simplified tile to a flat program, run once per pixel |
| `sound.rs` | sound layers: drones, notes, textures, the room and the echo |
| `harmony.rs` | note names, meters, progressions (numerals, names, functional), voice leading, arpeggios |
| `midi.rs` | the same layers as a MIDI score: tracks, notes, controllers; Standard MIDI Files |
| `live.rs` | playing a score live: on time, or following a DAW's clock (ports through `midir`, feature `midi`) |
| `paint.rs` | paintings: fields in colour, mosaics, placements, overlays |
| `render.rs` | command line: writes PNG / WAV / STL, streams video frames into ffmpeg |
| `web.rs` | browser: the WebAssembly entry points |
| `vfs.rs` | file access: the disk, or in-memory files in the browser |
| `export.rs` | `--web`: a self-contained page for one work |
| `error.rs` | diagnostics with line, column and underline |
| `web/` | the playground, the `--web` page template, the shared JavaScript engine |
| `plugin/` | alea as a VST3 / CLAP plugin: a work's music in time with the DAW (see [plugin/README.md](plugin/README.md)) |
| `editors/vscode/` | syntax highlighting and snippets for VS Code |

The `Work` / `Edition` split lives in the types: a renderer only ever receives
an `Edition`, in which no undrawn distribution remains.

**Speed without changing a pixel.** Before a frame is computed, every part of
the field that doesn't depend on the position (a moving train's centre, a fade)
is reduced to a number. The frame is then cut into 32 × 32 tiles, and for each
tile, interval arithmetic proves which parts are the same over the whole tile
(a train far away is exactly 0 there) and replaces them by that value. What
is left is compiled into a flat program over numbered slots (`compile.rs`),
where every shared part is computed once per pixel and every constant once
per tile, and the program runs pixel after pixel in one tight loop; a `grow`
block compiles its rules the same way at every generation. The result is byte for byte what a plain
evaluation gives, often ten to a hundred times faster.

The browser runs the same engine compiled to WebAssembly and computes the same
bytes as the command line. For that, alea uses its own maths (`libm`) and its
own samplers, rather than the operating system's.

## Building the browser engine

```sh
rustup target add wasm32-unknown-unknown
web/build.sh                 # → web/dist/alea.wasm and web/dist/playground.html
cargo build --release        # re-embeds the engine, for --web
python3 web/handbook.py      # → web/dist/handbook.html, illustrated with the tour
```

## Next steps

- **Faster rendering, further**: frames are computed in tiles, each
  simplified to what can change there and compiled to a flat program. Next:
  vectorize that program over many pixels at once (SIMD), or run it on the GPU.
- **Declarative constraints**: replace rejection with MCMC sampling for narrow spaces.
- **Faster growing**: compile grow rules to a stencil (or the GPU); a 256-cell
  grid over thousands of generations takes a couple of minutes today.
- **Video in the browser**: record the preview to a WebM file.
- **More inputs**: a sound's spectrogram as a field (`song.spectrum`), video
  frames as an input (`ffmpeg … | alea` frame by frame), the microphone live
  in the playground.

## Licence

alea is under the [MIT licence](LICENSE). The plugin (`plugin/`) is under the
[GPL-3.0](plugin/LICENSE), as the VST3 bindings it builds on require. The
fonts have their own licences ([fonts/LICENSES.md](fonts/LICENSES.md)).
