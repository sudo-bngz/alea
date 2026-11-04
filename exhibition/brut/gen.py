#!/usr/bin/env python3
"""Writes ../brut.alea (python3 gen.py ../brut.alea): ten scenes cut on a 128 bpm grid, 32 bars, 60 s."""
import sys

BPM = 128
BEAT = 60 / BPM
BEATS = [12, 12, 12, 12, 16, 12, 12, 12, 12, 16]          # 128 beats = 32 bars = 60 s
assert sum(BEATS) == 128
STARTS = [sum(BEATS[:k]) for k in range(10)]
LEN = [b * BEAT for b in BEATS]
TOTAL = 32 * 16 * BEAT / 4

def bars(*parts):
    """A steps string over 32 bars from (count, bar) pairs."""
    out = []
    for count, bar in parts:
        assert len(bar) == 16, bar
        out += [bar] * count
    assert len(out) == 32, len(out)
    return "|".join(out)

R = "................"
kick = bars((4, "X...X...X...X..."), (4, "X...X...X...X..."), (4, "X...X...X..xX..."), (4, "X...X...X..xX..x"),
            (2, R), (2, "X...............") , (4, "X...X...X..xX..."), (4, "X...X...X..xX..x"), (3, "X...X...X..xX..x"), (1, "X...X...X......."))
clap = bars((2, R), (2, "....X.......X..."), (12, "....X.......X..."), (4, R), (4, "....X.......X..."), (4, "....X.......X.x."), (3, "....X.......X.x."), (1, "....X.......X..."))
hatc = bars((2, "..x...x...x...x."), (2, "x.x.x.x.x.x.x.x."), (12, "x.x.x.x.x.x.x.x."), (4, "..x...x...x...x."), (4, "x.x.x.x.x.x.x.x."), (4, "xxx.xxx.xxx.xxx."), (4, "x.x.x.x.x.x.x.x."))
hato = bars((4, R), (4, "......X.......X."), (8, "..X...X...X...X."), (4, R), (4, "......X.......X."), (8, "..X...X...X...X."))
cow = bars((12, R), (4, "..x.......x..x.."), (4, R), (4, "..x.......x..x.."), (4, "..x..x....x..x.."), (4, "..x.......x..x.."))
bass = bars((1, R), (3, "X..X..X.X..X..X."), (4, "X..X..X.X..X..X."), (4, "X..X..XxX..X..X."), (4, "X..X..XxX..X..X."), (4, R), (4, "X..X..X.X..X..X."), (4, "X..X..XxX..X..X."), (4, "X..X..XxX..X..X."))
stab = bars((4, R), (4, R), (4, "..X.....X......X"), (4, "..X.....X......X"), (4, "..X............."), (4, "..X.....X......X"), (4, "..X..X..X......X"), (4, "..X.....X......X"))
arpr = bars((12, R), (4, "x.x.x.x.x.x.x.x."), (4, R), (4, R), (8, "x.x.x.x.x.x.x.x."))

def gates(s):
    """The on/off/level of each sixteenth from a steps string."""
    lv = {"X": 1.0, "x": 0.65, "o": 0.35}
    return [lv.get(c, 0) for c in s.replace("|", "")]

def lst(rows, j):
    return "[" + ", ".join(str(round(r[j], 3)) for r in rows) + "]"

import random
rnd = random.Random(0xA1EA)
BASS = []
for i, g in enumerate(gates(bass)):
    if g:
        y = [0.04, 0.04, 0.37, 0.04, 0.26, 0.04, 0.37, 0.04][i % 8] + (0.11 if (i // 64) % 2 and i % 8 == 4 else 0)
        BASS.append((i, y, g))
STAB = []
for i, g in enumerate(gates(stab)):
    if g:
        for j in range(3):
            STAB.append((i, [0.39, 0.5, 0.61][j], 0.9, 0.3 + 0.2 * j))
leadg = bars((8, R), (4, "xxxxxxxxxxxxxxxx"), (4, "xxxxxxxxxxxxxxxx"), (4, R), (4, "xxxxxxxxxxxxxxxx"), (4, "xxxxxxxxxxxxxxxx"), (4, R))
LEAD = [(i, 0, 0) for i, g in enumerate(gates(leadg)) if g and i % 2 == 0]

src = f'''// Brut — a phone clip of the basement, torn to pieces: ten scenes cut on a
// 128 bpm grid, the clip's own sound chopped through a modular patch, and
// a Detroit track underneath. Inputs: brut/make-inputs.py CLIP.mov
work Brut {{
  // ---- the clip, as sheets of frames (one picture lookup per frame) ----
  let sheets = files("brut/sheets/hi*.jpg")
  let fh = 1.778
  let hx(k) = 0.5 - ((k % 18) % 6 + 0.5 - 3)
  let hy(k) = 0.5 - (floor((k % 18) / 6) + 0.5 - 1.5) * fh
  let hi(k, ch) = sequence([picture(f, hx(k), hy(k), 6, channel: ch) for f in sheets], fade: 0, time: (floor(k / 18) + 0.5) / 7)
  let lo(k, ch) = picture("brut/sheets/lo.jpg", 0.5 - ((k % 16) + 0.5 - 8), 0.5 - (floor(k / 16) + 0.5 - 4) * fh, 16, channel: ch)
  let red(f) = paint(f, [[0, "#000000"], [1, "#ff0000"]], range: [0, 1])
  let green(f) = paint(f, [[0, "#000000"], [1, "#00ff00"]], range: [0, 1])
  let blue(f) = paint(f, [[0, "#000000"], [1, "#0000ff"]], range: [0, 1])
  let rgb(r, g, b) = overlay(red(r), green(g), blue(b), blend: "add")
  let bw = [[0, "#000000"], [1, "#f2efe8"]]
  let hot = [[0, "#000000"], [0.45, "#ff4d00"], [0.8, "#ffb000"], [1, "#fff6e0"]]
  let hash(k) = fract(sin(k * 12.9898 + 78.233) * 43758.5453)
  let beat = {BEAT}
  let globe(sz) = picture("brut/globe.png", 0.5, 0.5, sz, channel: "alpha")
  let word = picture("brut/word.png", 0.5, 1.18, 0.7, channel: "alpha")
  let white(f) = paint(f, bw, range: [0, 1], alpha: f)
  let duo(f, m) = overlay(paint(f, bw, range: [0, 1]), paint(f, hot, range: [0, 1]), mask: m)

  // ---- 1 · strobe: a random still every sixteenth, black and white ----
  let a1 = ({STARTS[0]} + t * {BEATS[0]})
  let s1_16 = floor(a1 * 4)
  let s1_k = floor(hash(s1_16) * 126)
  let s1_on = step(0.18, hash(s1_16 * 3 + 7))
  let s1_inv = step(0.78, hash(s1_16 * 5 + 2))
  let s1_f = posterize(lo(s1_k, "luminance") * 1.3 - 0.1, levels: 3)
  let s1_pic = film(mix(s1_f, 1 - s1_f, s1_inv) * s1_on, amount: 0.15)
  let s1 = overlay(paint(s1_pic, bw, range: [0, 1]), white(globe(0.36) * step(0.5, hash(floor(a1 * 2) * 11))))

  // ---- 2 · tear: the clip at speed, red green and blue pulled apart on every kick ----
  let a2 = ({STARTS[1]} + t * {BEATS[1]})
  let s2_16 = floor(a2 * 4)
  let s2_k = (a2 * beat * 15.18) % 126
  let s2_kick = exp(-5 * fract(a2))
  let s2_tear = 0.12 * (hash(floor(y * 24) + s2_16 * 7) - 0.5) * step(0.45, hash(s2_16 * 3 + 1))
  let s2_jit = 0.03 * (hash(s2_16 * 13) - 0.5) * step(0.7, hash(s2_16 * 17))
  let s2_r = move(hi(s2_k, "red"), 0.015 * s2_kick + s2_tear, s2_jit)
  let s2_g = move(hi(s2_k, "green"), s2_tear, 0)
  let s2_b = move(hi(s2_k, "blue"), -0.015 * s2_kick + s2_tear, -s2_jit)
  let s2 = film(rgb(s2_r, s2_g, s2_b), amount: 0.1)

  // ---- 3 · grid: sixteen cells, each on its own frame, flickering on eighths ----
  let a3 = ({STARTS[2]} + t * {BEATS[2]})
  let s3_8 = floor(a3 * 2)
  let s3_cell(c) = lo(floor(hash(s3_8 * 16 + c) * 126), "luminance") * step(0.12, hash(s3_8 * 7 + c * 3))
  let s3_logo = white(globe(0.7) * step(0.5, hash(s3_8 * 19)))
  let s3_cells = [duo(s3_cell(c) * (0.7 + 0.6 * hash(c + s3_8)), step(0.75, hash(c * 5 + s3_8 * 2))) for c in [0, 1, 2, 3, 4, 5, 6, 7, 8, 10, 11, 12, 13, 14, 15]]
  let s3 = film(mosaic(join(slice(s3_cells, 0, 9), s3_logo, slice(s3_cells, 9, 15)), columns: 4, gap: 0.004), amount: 0.08)

  // ---- 4 · kaleidoscope: the orange record, mirrored four to eight ways, in heat ----
  let a4 = ({STARTS[3]} + t * {BEATS[3]})
  let s4_b = floor(a4)
  let s4_k = floor(hash(s4_b * 3) * 28) + (floor(a4 * 4) % 4)
  let s4_kick = exp(-4 * fract(a4))
  let s4_f = zoom(hi(s4_k, "luminance"), by: 1.5 + 0.3 * s4_kick, x: 0.35, y: 0.42)
  let s4_sides = 4 + 2 * (s4_b % 3)
  let s4_m = mix(kaleidoscope(s4_f, sides: 4, x: 0.5, y: 0.5), mix(kaleidoscope(s4_f, sides: 6, x: 0.5, y: 0.5), kaleidoscope(s4_f, sides: 8, x: 0.5, y: 0.5), step(7, s4_sides)), step(5, s4_sides))
  let s4 = film(paint(posterize(s4_m, levels: 5), hot, range: [0, 1]), amount: 0.12)

  // ---- 5 · the globe: the clip seen through the logo, then the logo cut out of the clip ----
  let a5 = ({STARTS[4]} + t * {BEATS[4]})
  let s5_16 = floor(a5 * 4)
  let s5_k = (a5 * beat * 15.18 * 0.5 + 40) % 126
  let s5_tear = 0.1 * (hash(floor(y * 30) + s5_16 * 5) - 0.5) * step(0.6, hash(s5_16 * 3 + 4))
  let s5_disc = circle(0.5, 0.5, 0.84 + 0.06 * exp(-5 * fract(a5)), softness: 0.004)
  let s5_flip = step(8, a5 - {STARTS[4]})
  let s5_alpha = mix(s5_disc, 1 - s5_disc, s5_flip)
  let s5_r = move(hi(s5_k, "red"), 0.01 + s5_tear, 0)
  let s5_g = move(hi(s5_k, "green"), s5_tear, 0)
  let s5_b = move(hi(s5_k, "blue"), -0.01 + s5_tear, 0)
  let s5_pic = overlay(red(s5_r * s5_alpha), green(s5_g * s5_alpha), blue(s5_b * s5_alpha), blend: "add")
  let s5_lines = white(picture("brut/globe.png", 0.5, 0.5, 0.86 + 0.06 * exp(-5 * fract(a5)), 360 * fract(a5 / 32), channel: "alpha"))
  let s5_word = white(word * step(0.3, hash(s5_16 * 23)))
  let s5 = film(overlay(s5_pic, s5_lines, s5_word), amount: 0.08)

  // ---- 6 · melt: the break; the clip runs backwards through warped space ----
  let a6 = ({STARTS[5]} + t * {BEATS[5]})
  let s6_8 = floor(a6 * 2)
  let s6_k = (300 - a6 * beat * 15.18 * 0.7) % 126
  let s6_amt = 0.02 + 0.07 * hash(s6_8 * 3 + 9)
  let s6_r = warp(hi(s6_k, "red"), amount: s6_amt, scale: 3)
  let s6_g = warp(hi(s6_k, "green"), amount: s6_amt * 1.3, scale: 3)
  let s6_b = warp(hi(s6_k, "blue"), amount: s6_amt * 0.7, scale: 3)
  let s6 = chroma(bloom(film(rgb(s6_r, s6_g, s6_b), amount: 0.06), amount: 1.2), amount: 1.1)

  // ---- 7 · lines: edges and scanlines, with a flash of colour on each kick ----
  let a7 = ({STARTS[6]} + t * {BEATS[6]})
  let s7_8 = floor(a7 * 2)
  let s7_k = (a7 * beat * 15.18 + 60) % 126
  let s7_px = step(0.5, hash(s7_8 * 11))
  let s7_l = mix(pixelate(hi(s7_k, "luminance"), size: 0.005), pixelate(hi(s7_k, "luminance"), size: 0.035), s7_px)
  let s7_e = clamp(edges(s7_l, step: 0.003) * 12, 0, 1) * (0.6 + 0.4 * step(0.5, fract(y * 160)))
  let s7_flash = step(0.88, 1 - fract(a7)) * step(0.4, hash(floor(a7) * 3))
  let s7_col = rgb(hi(s7_k, "red") * s7_flash, hi(s7_k, "green") * s7_flash, hi(s7_k, "blue") * s7_flash)
  let s7 = overlay(duo(s7_e, step(0.5, hash(s7_8 * 5 + 1))), s7_col, blend: "add")

  // ---- 8 · ghosts: two moments of the clip, their difference; tiles on some beats ----
  let a8 = ({STARTS[7]} + t * {BEATS[7]})
  let s8_8 = floor(a8 * 2)
  let s8_k = (a8 * beat * 15.18 + 20) % 126
  let s8_d = 2 + floor(hash(s8_8 * 3) * 24)
  let s8_tiles = step(0.65, hash(s8_8 * 7 + 5))
  let s8_pair(ch) = abs(hi(s8_k, ch) - hi((s8_k + s8_d) % 126, ch))
  let s8_ch(ch) = mix(s8_pair(ch), tile(s8_pair(ch), 2), s8_tiles)
  let s8_inv = step(0.5, fract(a8 / 2 + 0.25)) * step(0.5, hash(floor(a8) * 9))
  let s8_g(ch) = clamp(s8_ch(ch) * 2.5, 0, 1)
  let s8_bg = rgb(mix(s8_g("red"), 1 - s8_g("red"), s8_inv), mix(s8_g("green"), 1 - s8_g("green"), s8_inv), mix(s8_g("blue"), 1 - s8_g("blue"), s8_inv))
  let s8 = film(s8_bg, amount: 0.1)

  // ---- 9 · strips: eight columns, each stuttering on its own two frames ----
  let a9 = ({STARTS[8]} + t * {BEATS[8]})
  let s9_16 = floor(a9 * 4)
  let s9_col = floor(x * 8)
  let s9_base = floor(hash(floor(a9 / 2) * 5 + s9_col) * 120)
  let s9_k = (s9_base + (s9_16 % 2) * (1 + floor(hash(s9_col * 3) * 3)) + floor(hash(s9_16 * 7 + s9_col) * 4) * step(0.8, hash(s9_16 + s9_col * 11))) % 126
  let s9_dark = step(0.2, hash(s9_col * 13 + floor(a9 * 2) * 3))
  let s9_off = 0.006 * (hash(s9_col + s9_16) - 0.5)
  let s9_r = move(hi(s9_k, "red"), s9_off, 0) * s9_dark
  let s9_g = hi(s9_k, "green") * s9_dark
  let s9_b = move(hi(s9_k, "blue"), -s9_off, 0) * s9_dark
  let s9 = film(rgb(s9_r, s9_g, s9_b), amount: 0.12)

  // ---- 10 · out: the clip clean, then the stamp, then black ----
  let a10 = ({STARTS[9]} + t * {BEATS[9]})
  let b10 = a10 - {STARTS[9]}
  let s10_8 = floor(a10 * 2)
  let s10_k = (a10 * beat * 15.18 + 90) % 126
  let s10_stamp = step(8, b10) * step(0.5, fract(a10 / 2 + 0.5)) * step(0.5, 12.5 - b10) + step(12.5, b10)
  let s10_logo = white((globe(0.5) + picture("brut/word.png", 0.5, 0.9, 0.62, channel: "alpha")) * step(0.5, 15.2 - b10))
  let s10_pic = rgb(hi(s10_k, "red") * (1 - s10_stamp), hi(s10_k, "green") * (1 - s10_stamp), hi(s10_k, "blue") * (1 - s10_stamp))
  let s10 = film(overlay(s10_pic, overlay(paint(0 * x, bw, range: [0, 1]), s10_logo), mask: s10_stamp), amount: 0.07)

  let movie = timeline([
    scene(s1, {LEN[0]}), glitch(0.3, amount: 0.9),
    scene(s2, {LEN[1]}), glitch(0.25),
    scene(s3, {LEN[2]}), pixels(0.4, size: 0.12),
    scene(s4, {LEN[3]}), glitch(0.3),
    scene(s5, {LEN[4]}), jump(),
    scene(s6, {LEN[5]}), glitch(0.5, amount: 1),
    scene(s7, {LEN[6]}), glitch(0.25),
    scene(s8, {LEN[7]}), glitch(0.3, amount: 0.8),
    scene(s9, {LEN[8]}), glitch(0.3),
    scene(s10, {LEN[9]})
  ])

  // ---- the track: 808 drums, a bass that never sits still, stabs, Model 500 style ----
  let m = meter(tempo: {BPM}, bars: 32)
  let prog = chords("i i VI VII", key: "A2", scale: "minor", bars: m, repeat: 8)
  let kickS = synth {{ drive(sine(48 * (1 + 9 * perc(attack: 0.0005, release: 0.03))) * perc(attack: 0.001, release: 0.42) * note.velocity + highpass(noise(), cutoff: 2500) * perc(release: 0.008) * 0.25, amount: 0.35) }}
  let clapS = synth {{ bandpass(noise(), cutoff: 1400, resonance: 0.25) * (perc(release: 0.015) + perc(attack: 0.011, release: 0.015) * 0.8 + perc(attack: 0.022, release: 0.18) * 0.9) * note.velocity }}
  let hatS  = synth {{ highpass(noise(), cutoff: 8500, resonance: 0.3) * perc(attack: 0.001, release: 0.04) * note.velocity }}
  let ohatS = synth {{ highpass(noise() * 0.7 + square(6100) * square(8700) * 0.4, cutoff: 7000, resonance: 0.3) * perc(attack: 0.001, release: 0.28) * note.velocity }}
  let cowS  = synth {{ bandpass(square(540) + square(800), cutoff: 680, resonance: 0.45) * perc(attack: 0.001, release: 0.14) * note.velocity }}
  let bassS = synth {{ lowpass(saw(note.freq) * 0.7 + square(note.freq * 0.5) * 0.5, cutoff: 180 + 1600 * adsr(attack: 0.004, decay: 0.13, sustain: 0.15, release: 0.08), resonance: 0.55) * adsr(attack: 0.003, decay: 0.16, sustain: 0.45, release: 0.06) }}
  let stabS = synth {{ lowpass(saw(note.freq) + saw(note.freq * ~around(1.004, 0.002)) + pulse(note.freq * 2, width: 0.3) * 0.4, cutoff: 500 + 3200 * adsr(attack: 0.004, decay: 0.18, sustain: 0.08, release: 0.1), resonance: 0.35) * adsr(attack: 0.003, decay: 0.22, sustain: 0.12, release: 0.12) }}
  let leadS = synth {{ lowpass(chorus(square(note.freq) * 0.5 + saw(note.freq * 0.5) * 0.35, depth: 0.4, rate: 0.6), cutoff: 900 + 2500 * adsr(attack: 0.02, decay: 0.2, sustain: 0.5, release: 0.2), resonance: 0.3) * adsr(attack: 0.01, decay: 0.12, sustain: 0.6, release: 0.25) }}
  let pluckS = synth {{ lowpass(saw(note.freq) * 0.6 + sine(note.freq * 2) * 0.3, cutoff: 600 + 4000 * perc(release: 0.12), resonance: 0.3) * perc(attack: 0.002, release: 0.22) * note.velocity }}

  pattern bassP = repeat {len(BASS)} event(t: {lst(BASS, 0)}[i] / 512, x: 0.5, y: {lst(BASS, 1)}[i], size: 0.004, intensity: {lst(BASS, 2)}[i])
  pattern stabP = repeat {len(STAB)} event(t: {lst(STAB, 0)}[i] / 512, x: {lst(STAB, 3)}[i], y: {lst(STAB, 1)}[i], size: 0.006, intensity: {lst(STAB, 2)}[i])
  pattern leadP = repeat {len(LEAD)} event(t: {lst(LEAD, 0)}[i] / 512, x: 0.5, y: ~between(0.45, 0.9), size: 0.01, intensity: 0.9 * (~between(0, 1) < 0.35))

  let drums = [
    notes(steps("{kick}", m), instrument: kickS, scale: "chromatic", key: "A1", gain: 1.3),
    notes(steps("{clap}", m), instrument: clapS, scale: "chromatic", key: "A3", gain: 0.75),
    notes(steps("{hatc}", m), instrument: hatS, scale: "chromatic", key: "A5", gain: 0.42),
    notes(steps("{hato}", m), instrument: ohatS, scale: "chromatic", key: "A5", gain: 0.38),
    notes(steps("{cow}", m), instrument: cowS, scale: "chromatic", key: "A4", gain: 0.3)
  ]
  let music = [
    notes(bassP, instrument: bassS, harmony: prog, key: "A1", tempo: {BPM}, gain: 1.0),
    notes(stabP, instrument: stabS, harmony: prog, key: "A3", tempo: {BPM}, gain: 0.55),
    arp(prog, rhythm: steps("{arpr}", m), mode: "up", octaves: 2, instrument: pluckS, key: "A4", gain: 0.28),
    notes(leadP, instrument: leadS, scale: "minor-pentatonic", key: "A3", tempo: {BPM}, gain: 0.45)
  ]
  let chop = open("brut/chop.wav")

  render video(movie, width: 1080, height: 1920, fps: 30, sound: join(drums, music, chop), room: 0.12, echo: 0.2)
  render image(at(movie, 0.4), width: 1080, height: 1920)
}}
'''
open(sys.argv[1], "w").write(src)
print("ok", TOTAL)
