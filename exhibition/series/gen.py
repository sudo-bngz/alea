#!/usr/bin/env python3
"""alea, one feature at a time — the template of the series.

    python3 gen.py 01-tilde.json > 01-tilde.alea          # add `site` after the JSON for the one-colour scheme

An episode is a JSON file: its number and title, the sections (start time,
label top left, the spoken line, the lines of code that type, the stage —
alea `let` lines ending in a field drawn in that section's window) and the
moments the score marks. The template is everything the episodes share:
the opening (the wave of alea.bngz.io, the handle, a cursor), the red labels,
the captions typed letter by letter as the voice says them, the code typed a
line to an eighth note, the cuts, the end card, the voice settings and the
score (a pad on tape, dust, a beat, a stutter into every cut, a boom on the
sections, a glass hit on every `hit`). Same theme as the intro and the site:
black, grey, ink, and the red of chance.
"""
import json, sys

ENGINE = "../../voices/kokoro-misaki.py"   # paths resolve from the directory alea runs in: run from series/
VOICE = "af_kore"
WRAP = 31          # characters per caption line
ADV = 0.022757     # advance of one mono character at size 0.044
CODE_STEP = 0.25   # a code line on each eighth note
# the palette, in the order a soft edge may cross it: black, the code panel, the frame
# grey, the muted grey, the greys of functions and strings, the ink, white, and the red
# of chance last. A level is k / 9.
SCHEMES = {
    # the site's own: red the only colour
    "site": ["#000000", "#0b0b0b", "#3a3a3a", "#8f8f8f", "#a9a9a9", "#b8b8b8", "#d6d6d6", "#ffffff", "#ffffff", "#ff4b2b"],
    # the same, with functions in blue, strings in olive, numbers in amber
    "colour": ["#000000", "#0b0b0b", "#3a3a3a", "#8f8f8f", "#7fb3d5", "#b5bd68", "#d6d6d6", "#f0c674", "#ffffff", "#ff4b2b"],
}
PALETTE = SCHEMES[sys.argv[2] if len(sys.argv) > 2 else "colour"]   # `python3 gen.py 01-tilde.json site` for the one-colour scheme
PAL = "[" + ", ".join(f'"{c}"' for c in PALETTE) + "]"
LEVELS = {"PANEL": 1, "RULE": 2, "GREY": 3, "FN": 4, "STR": 5, "INK": 6, "NUM": 7, "KEY": 8, "RED": 9}
CODE_SIZE = 0.042   # big enough for a phone
CODE_DY = 0.075
KEYWORDS = {"work", "let", "field", "output", "render", "use", "choose", "synth", "for", "in", "if", "else", "constraint", "true", "false", "pattern", "grow", "repeat", "event", "then", "over"}


def caption(name, say, text, start, end):
    """The caption of a spoken line, typed as it is said (word timings from say)."""
    words = text.split(" ")
    terms = []
    for k, w in enumerate(words):
        n = len(w) + (1 if k < len(words) - 1 else 0)
        terms.append(f"clamp((T - {say}.words[{k}]) / ({say}.ends[{k}] - {say}.words[{k}]), 0, 1) * {n}")
    # wrap on words
    lines, cur = [], ""
    for w in words:
        if cur and len(cur) + 1 + len(w) > WRAP:
            lines.append(cur)
            cur = w
        else:
            cur = (cur + " " + w) if cur else w
    lines.append(cur)
    out = [f"  let n_{name} = " + " + ".join(terms)]
    parts, off = [], 0
    for j, line in enumerate(lines):
        y = -0.115 + 0.056 * j
        parts.append(f'text("{line}", x: 0.06, y: {y:.3f}, size: 0.044, font: "mono", align: "left") * (x < 0.06 + clamp(n_{name} - {off}, 0, {len(line)}) * adv)')
        off += len(line) + 1
    body = parts[0] if len(parts) == 1 else "merge(" + ", ".join(parts) + ")"
    out.append(f"  let c_{name} = {body} * win({start}, {end})")
    return out


import re

TOKEN = re.compile(r'(//[^\n]*)|("(?:[^"\\]|\\.)*")|(~~?)|(\b\d+(?:\.\d+)?\b|\b0x[0-9a-f]+\b)|([A-Za-z_][A-Za-z0-9_]*)(\s*\()?|([\s\S])', re.I)


def mx(items):
    """max of many fields, nested two by two."""
    items = list(items)
    return items[0] if len(items) == 1 else f"max({items[0]}, {mx(items[1:])})"


def classes(text, shell=False):
    """The site's highlighter (site/site.js): each character's class."""
    out = []
    if shell:
        for ch in text:
            out.append("GREY" if ch == "$" else "INK")
        return out
    for m in TOKEN.finditer(text):
        if m.group(1):
            out += ["GREY"] * len(m.group(1))
        elif m.group(2):
            out += ["STR"] * len(m.group(2))
        elif m.group(3):
            out += ["RED"] * len(m.group(3))
        elif m.group(4):
            out += ["NUM"] * len(m.group(4))
        elif m.group(5):
            word = m.group(5)
            cls = "KEY" if word in KEYWORDS else ("FN" if m.group(6) else "INK")
            out += [cls] * len(word)
            if m.group(6):
                out += ["INK"] * len(m.group(6))
        else:
            out += ["INK"] * len(m.group(7))
    return out


def code_block(name, lines, at, y0=0.14, step=CODE_STEP, size=CODE_SIZE, dy=CODE_DY, x0=0.07):
    """Lines of code on the site's panel, typed one per eighth note, highlighted as on
    alea.bngz.io: keywords and numbers white, functions and strings grey, `~` red.
    Defines {name}_code, a field in palette levels."""
    out, fields = [], []
    fit = int((0.96 - x0) / (0.5172 * size)) + 2   # the glyphs are a little narrower than the advance
    for k, line in enumerate(lines):
        t0 = at + k * step
        txt = line["text"] if isinstance(line, dict) else line
        if len(txt) > fit:
            print(f"gen.py: line {k} of {name} is {len(txt)} characters, the panel fits {fit} at size {size}: {txt}", file=sys.stderr)
        cls = classes(txt, shell=isinstance(line, dict) and line.get("shell", False))
        y = y0 + k * dy
        for c in sorted(set(cls)):
            masked = "".join(ch if cc == c else " " for ch, cc in zip(txt, cls)).rstrip()
            if not masked:
                continue   # a blank line: nothing to type
            esc = masked.replace("{", "{{").replace('"', "\u201d")   # `{` doubled, quotes curly: both live inside an alea string
            out.append(f'  let {name}_k{k}_{c} = text("{esc}", x: {x0}, y: {y:.3f}, size: {size}, font: "mono", align: "left") * {c} * (T >= {t0:.2f})')
            fields.append(f"{name}_k{k}_{c}")
    top = y0 - 0.055
    bottom = y0 + (len(lines) - 1) * dy + 0.055
    cy = (top + bottom) / 2
    h = bottom - top
    out.append(f"  let {name}_panel = max(rect(0.5, {cy:.3f}, 0.92, {h:.3f}, 0, 0.001) * PANEL, rect(0.042, {cy:.3f}, 0.004, {h:.3f}, 0, 0.001) * RULE) * (T >= {at:.2f})")
    out.append(f"  let {name}_code = {mx([f'{name}_panel'] + fields)}")
    return out


def main():
    ep = json.load(open(sys.argv[1]))
    secs = ep["sections"]
    total = ep["total"]
    starts = [s["start"] for s in secs] + [total]
    L = []
    L.append(f"// alea, one feature at a time · {ep['number']} — {ep['title']}")
    for line in ep.get("comment", []):
        L.append(f"// {line}")
    L.append(f"// Generated by series/gen.py from series/{ep['file']}; the template is the")
    L.append("// generator, the episode is the JSON.")
    L.append("")
    for u in ep.get("uses", []):
        L.append(f'use "{u}"')
    if ep.get("uses"):
        L.append("")
    L.append(f"work Episode{ep['number']} {{")
    L.append(f"  let total = {total}")
    L.append("  let T = t * total")
    L.append(f'  let ENGINE = "{ENGINE}"')
    L.append(f'  let VOICE = "{VOICE}"')
    L.append("  let win(a, b) = (T >= a) * (T < b)")
    L.append("  let beat = floor(T * 2)")
    for k, v in LEVELS.items():
        L.append(f"  let {k} = {v} / 9")
    L.append("  let px = 1 / 1080")
    L.append("  let blink = fract(T * 2) < 0.5")
    L.append(f"  let adv = {ADV}")
    for line in ep.get("draws", []):
        L.append(f"  {line}")
    L.append("")
    L.append("  // ── the voice ──")
    says = []
    for i, s in enumerate(secs):
        if "say" in s:
            name = f"l{i}"
            says.append(name)
            L.append(f'  let {name} = say("{s["say"]}", at: {s["at"]:.2f}, engine: ENGINE, voice: VOICE, speed: 1.0, gain: 2.5)')
    L.append("")
    L.append("  // ── the labels, top left, in red; the captions, typed as they are said ──")
    labels = []
    for i, s in enumerate(secs):
        if s.get("label"):
            L.append(f'  let h_{i} = text("{s["label"]}", x: 0.06, y: -0.19, size: 0.032, font: "mono", align: "left") * win({s["start"]}, {starts[i + 1]})')
            labels.append(f"h_{i}")
    L.append(f"  let header = merge({', '.join(labels)})")
    caps = []
    for i, s in enumerate(secs):
        if "say" in s:
            L.extend(caption(f"l{i}", f"l{i}", s["say"], f"l{i}.start", starts[i + 1]))
            caps.append(f"c_l{i}")
    L.append(f"  let captions = merge({', '.join(caps)})")
    L.append("")
    L.append("  // ── the opening: the mark of alea.bngz.io, the handle, a cursor ──")
    L += [
        "  let wx0 = 0.29",
        "  let wx1 = 0.71",
        "  let ww = 0.034",
        "  let wu = (x - 0.5) / (wx1 - wx0) * tau",
        "  let wcurve = 0.5 + 0.075 * sin(wu)",
        "  let wslope = 0.075 * cos(wu) * tau / (wx1 - wx0)",
        "  let wdist = abs(y - wcurve) / sqrt(1 + wslope * wslope)",
        "  let wfoot = x + (y - wcurve) * wslope / (1 + wslope * wslope)",
        "  let drawnto = wx0 + (wx1 - wx0) * clamp((T - 0.3) / 1.0, 0, 1)",
        "  let wband = smoothstep(ww + 0.002, ww - 0.002, wdist) * step(wx0, wfoot) * step(wfoot, min(wx1, drawnto))",
        "  let wcaps = merge(circle(wx0, 0.5, 2 * ww, softness: 0.002) * (T >= 0.3), circle(wx1, 0.5, 2 * ww, softness: 0.002) * (T >= 1.3))",
        "  let wave = merge(wband, wcaps)",
        '  let handle = text("written.in.alea", x: 0.5, y: 0.74, size: 0.036, font: "mono") * (T >= 1.6)',
        "  let ocur = (abs(x - 0.5) < 0.012) * (abs(y - 0.9) < 0.028) * blink * (T >= 1.6)",
        "  let openf = max(wave * INK, max(handle * GREY, ocur * RED))",
        "",
        "  // ── the frames of the results ──",
        "  let frame(cx, cy, w, h) = cut(rect(cx, cy, w + 0.006, h + 0.006, 0, 0.001), rect(cx, cy, w, h, 0, 0.001))",
    ]
    L.append("")
    stages = []
    for i, s in enumerate(secs):
        L.append(f"  // ── {s['name']} ──")
        if "code" in s:
            L.extend(code_block(f"k{i}", s["code"]["lines"], s["code"]["at"], **s["code"].get("layout", {})))
        for line in s.get("stage", []):
            L.append(f"  {line}")
        if s.get("field"):
            stages.append(f"{s['field']} * win({s['start']}, {starts[i + 1]})")
        L.append("")
    L.append("  // ── the page ──")
    L.append(f"  let drawn = merge({', '.join(stages)})")
    L.append("  let over = merge(header * RED, captions * INK)")
    L.append(f"  let base = paint(max(drawn, over), palette: {PAL}, range: [0, 1])")
    look = "base"
    for o in ep.get("overlays", []):
        look = f"overlay({look}, {o['paint']}, mask: {o['mask']})"
    L.append(f"  let look = film({look}, amount: 0.03)")
    L.append("")
    L.append("  // ── the cuts ──")
    L.append("  let reel = timeline([")
    trans = ep.get("transitions", {})
    for i, s in enumerate(secs):
        a, b = s["start"], starts[i + 1]
        tr = trans.get(str(i), "jump()") if i < len(secs) - 1 else f"fade({ep.get('fadeout', 0.8)}, through: 0)"
        # a section may be cut inside: "subcuts": [[time, transition], …], the demonstration's own cuts
        pieces = []
        p = a
        for (c, ctr) in s.get("subcuts", []):
            pieces.append((p, c, ctr))
            p = c
        pieces.append((p, b, tr))
        for (p0, p1, ptr) in pieces:
            L.append(f"      scene(look, {p1 - p0:.2f}, from: {p0} / total, to: {p1} / total), {ptr},")
    L.append("  ], loop: 1)")
    L.append("")
    score(L, ep, starts)
    L.append(f"  render image(reel, width: 1080, height: 1920, at: {ep.get('cover', 0.5)})")
    L.append("  render video(reel, fps: 30, width: 1080, height: 1920, duration: total, sound: music, room: 0.2, echo: 0.15)")
    L.append("}")
    print("\n".join(L))


def score(L, ep, starts):
    total = ep["total"]
    sc = ep["score"]
    cuts = sc["cuts"]
    booms = sc["booms"]
    hits = sc["hits"]
    beat_from, beat_to = sc["beat"]
    half_from = sc.get("half", [beat_from, beat_from])
    s16 = int(total * 8)
    s64 = s16 * 4
    L += [
        "  // ── the score: a pad on a worn tape, FM dust, a beat redrawn every two bars from `beat` on,",
        "  //    a stutter into every cut, a boom on the sections, a glass hit on every run ──",
        f"  let s16 = {s16}",
        f"  let s64 = {s64}",
        "  let semi(s) = 1 - (s + 0.5) / 36",
        "  let h(a, b) = fract(sin(a * 12.9898 + b * 78.233 + 0.5) * 43758.5453)",
        "  let st(j) = j % 16",
        "  let br(j) = floor(j / 16)",
        "  let kon(j) = max(st(j) == 0, (h(st(j), floor(j / 32)) > 0.8) * (st(j) % 2 == 0))",
        "  let son(j) = max(st(j) == 12, max((st(j) == 4) * (h(br(j), 1) > 0.4), h(st(j), br(j) + 7) > 0.9))",
        "  let hon(j) = h(st(j), br(j) + 3) > 0.42",
        "  let bon(j) = max(st(j) == 0, (h(st(j), floor(j / 64) + 11) > 0.74) * (1 - kon(j) * (st(j) != 0)))",
        "  let pad = synth {",
        "    let wob = 1 + 0.005 * sine(0.21) + 0.0018 * sine(5.3)",
        "    let f = note.freq * wob",
        "    let osc = saw(f) * 0.5 + saw(f * 1.007) * 0.5 + triangle(f * 2.003) * 0.3 + sine(f * 0.5) * 0.4",
        "    chorus(lowpass(drive(osc, amount: 0.3), cutoff: 750 + 450 * sine(0.13), resonance: 0.25)",
        "           * adsr(attack: 0.5, decay: 0.8, sustain: 0.7, release: 1.8) * 0.22, depth: 0.8, rate: 0.27)",
        "  }",
        "  let kick = synth { drive(sine(note.freq * (1 + 4 * perc(attack: 0.0005, release: 0.03))) * perc(attack: 0.001, release: 0.2), amount: 0.4)",
        "                      + highpass(noise(), cutoff: 3000) * perc(release: 0.003) * 0.4 }",
        "  let snare = synth { drive(bandpass(noise(), cutoff: 2600, resonance: 0.4) * perc(release: 0.08)",
        "                       + sine(190 * (1 + 0.6 * perc(release: 0.01))) * perc(release: 0.05) * 0.7, amount: 0.6) }",
        "  let tick = synth { sine(note.freq * (1 + 6 * sine(note.freq * 2.76))) * perc(attack: 0.0003, release: 0.016) }",
        "  let hat = synth { (highpass(noise(), cutoff: 9000, resonance: 0.3) + sine(note.freq * (1 + 3 * sine(note.freq * 1.47))) * 0.25) * perc(release: 0.022) }",
        "  let sub = synth { sine(note.freq * (1 + 0.7 * sine(note.freq * 2.01) * perc(release: 0.1))) * adsr(attack: 0.004, decay: 0.18, sustain: 0.5, release: 0.1) }",
        "  let bell = synth { sine(note.freq * (1 + 1.4 * sine(note.freq * 3.53) * perc(release: 0.5))) * perc(attack: 0.002, release: 1.3) * 0.5",
        "                      + sine(note.freq * 2.001) * perc(release: 0.25) * 0.15 }",
        "  let glitch = synth { bandpass(square(note.freq * (1 + 5 * sine(note.freq * 0.51))), cutoff: 2200, resonance: 0.6) * perc(attack: 0.0005, release: 0.028) }",
        "  let boom = synth { drive(sine(note.freq * (1 + 2.5 * perc(release: 0.06))) * perc(attack: 0.001, release: 1.1), amount: 0.45)",
        "                      + bandpass(noise(), cutoff: 500, resonance: 0.3) * perc(release: 0.35) * 0.4 }",
        "  let glass = synth { sine(note.freq * (1 + 0.8 * sine(note.freq * 4.2) * perc(release: 0.06))) * perc(attack: 0.001, release: 0.3) }",
        "",
        f"  let chords = {sc['chords']}",
        f"  let roots = {sc['roots']}",
        "  let secs(j) = j / 8",
        "  let inside(j, a, b) = (secs(j) >= a) * (secs(j) < b)",
        f"  let half(j) = inside(j, {half_from[0]}, {half_from[1]})",
        f"  let full(j) = inside(j, {beat_from}, {beat_to})",
        "  let chord(j) = floor(secs(j) / 4) % 4",
        "",
    ]
    npads = int(total / 2) + 1
    L += [
        f"  pattern pads = repeat {npads * 4} event(t: floor(i / 4) * 2 / total, x: 0.2 + 0.2 * (i % 4),",
        "      y: semi(chords[floor(i / 8) % 4][i % 4]), size: 0.15, intensity: (0.35 + 0.1 * (i % 4 == 0)) * (1 - 0.35 * (floor(i / 4) % 2)))",
        "  pattern dust = repeat s16 * 2 event(t: i / (s16 * 2), x: h(i, 41), y: semi(floor(h(i, 43) * 30)), size: 0.002,",
        f"      intensity: (h(i, 42) > 0.72) * (0.25 + 0.5 * h(i, 44)) * (inside(i / 2, 0, {starts[1]}) * 0.9 + inside(i / 2, {starts[1]}, {starts[-2]}) * 0.35))",
        "  pattern kicks = repeat s16 event(t: i / s16, x: 0.5, y: semi(0), size: 0.01,",
        "      intensity: full(i) * kon(i) * (0.75 + 0.25 * (st(i) == 0)) + half(i) * ((st(i) == 0) + (st(i) == 10) * (h(br(i), 5) > 0.4)) * 0.8)",
        "  pattern snares = repeat s16 event(t: i / s16, x: 0.55, y: semi(0), size: 0.01,",
        "      intensity: full(i) * son(i) * (0.4 + 0.5 * (st(i) % 4 == 0)) + half(i) * (st(i) == 12) * 0.7)",
        "  pattern hats = repeat s16 event(t: i / s16, x: 0.25 + 0.5 * h(i, 9), y: semi(floor(h(i, 8) * 20)), size: 0.002,",
        f"      intensity: inside(i, {half_from[0]}, {beat_to}) * hon(i) * (0.2 + 0.4 * h(i, 10)))",
        "  pattern rolls = repeat s64 event(t: i / s64, x: 0.3 + 0.4 * h(floor(i / 4), 21), y: semi(10 + (i % 4) * 4), size: 0.002,",
        "      intensity: full(floor(i / 4)) * (h(st(floor(i / 4)), br(floor(i / 4)) + 5) > 0.86) * (0.6 - (i % 4) * 0.12))",
        "  pattern basses = repeat s16 event(t: i / s16, x: 0.5, y: semi(roots[chord(i)] + 12 * (h(i, 31) > 0.7)), size: 0.004,",
        "      intensity: full(i) * bon(i) * 0.85)",
        f"  pattern bells = repeat {int(total / 0.4) + 1} event(t: i * 0.4 / total, x: 0.2 + 0.6 * h(i, 51),",
        "      y: semi([12, 14, 15, 17, 19, 21, 22, 24, 26][floor(h(i, 52) * 9)]), size: 0.05,",
        f"      intensity: (h(i, 53) > 0.55) * (inside(i * 3.2, {sc['bells'][0]}, {sc['bells'][1]}) * 0.5 + inside(i * 3.2, {starts[-2]}, {total - 1}) * 0.8))",
        f"  let cuts = {cuts}",
        f"  pattern stutters = repeat {len(cuts) * 8} event(t: (cuts[floor(i / 8)] - 0.25 + (i % 8) * 0.03125) / total, x: 0.5 + 0.3 * sin(i * 2.1),",
        "      y: semi(6 + (i % 8) * 3), size: 0.002, intensity: 0.25 + (i % 8) * 0.07)",
        f"  pattern glitches = repeat {len(cuts) * 3} event(t: (cuts[floor(i / 3)] + (i % 3) * 0.047) / total, x: 0.5 + 0.35 * sin(i * 1.3),",
        "      y: semi(18 - (i % 3) * 6), size: 0.002, intensity: 0.8 - (i % 3) * 0.2)",
        f"  pattern booms = repeat {len(booms)} event(t: {booms}[i] / total, x: 0.5, y: semi(0), size: 0.05, intensity: 0.9)",
        f"  let hittimes = {hits}",
        f"  pattern flips = repeat {len(hits)} event(t: hittimes[i] / total, x: 0.2 + 0.6 * h(i, 61), y: semi([24, 27, 29, 31, 34][floor(h(i, 62) * 5)]), size: 0.005, intensity: 0.5)",
        "",
        *[f"  {line}" for line in sc.get("lets", [])],
        "",
        "  let music = [",
        "    " + ", ".join(f"l{i}" for i, s in enumerate(ep["sections"]) if "say" in s) + ",",
        *[f"    {line}," for line in sc.get("layers", [])],
        f"    notes(pads, instrument: pad, scale: \"chromatic\", key: {sc['key']}, gain: 0.4),",
        "    notes(dust, instrument: tick, scale: \"chromatic\", key: 1200, gain: 0.35),",
        "    notes(kicks, instrument: kick, scale: \"chromatic\", key: 48, gain: 1.3),",
        "    notes(snares, instrument: snare, gain: 0.7),",
        "    notes(hats, instrument: hat, scale: \"chromatic\", key: 2000, gain: 0.45),",
        "    notes(rolls, instrument: tick, scale: \"chromatic\", key: 900, gain: 0.5),",
        f"    notes(basses, instrument: sub, scale: \"chromatic\", key: {sc['key'] / 2}, gain: 0.75),",
        f"    notes(bells, instrument: bell, scale: \"chromatic\", key: {sc['key'] * 4}, gain: 0.4),",
        "    notes(stutters, instrument: glitch, scale: \"chromatic\", key: 220, gain: 0.55),",
        "    notes(glitches, instrument: glitch, scale: \"chromatic\", key: 110, gain: 0.7),",
        f"    notes(booms, instrument: boom, scale: \"chromatic\", key: {sc['key'] / 2}, gain: 1.0),",
        f"    notes(flips, instrument: glass, scale: \"chromatic\", key: {sc['key'] * 8}, gain: 0.35),",
        "  ]",
        "",
    ]


if __name__ == "__main__":
    main()
