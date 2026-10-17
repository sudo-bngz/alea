# How alea works

How the engine is put together, how it stays fast without changing a pixel,
how to build the browser engine, and what comes next.

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
| `plugin/` | alea as a VST3 / CLAP plugin: a work's music in time with the DAW (see [plugin/README.md](../plugin/README.md)) |
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
