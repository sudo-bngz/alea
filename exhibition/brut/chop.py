#!/usr/bin/env python3
"""The clip's sound, through a modular patch: chop.py clip.wav chop.wav

Nothing of the clip is played as it was. Its sound is cut into
sixteenth-note slices (the loudest ones more often), and a sequencer fires
them on a 128 bpm grid, each through its own modules:

  slicer → pitch (½×, 1×, 2×, backwards) → gate envelope → stutter (1/64
  retriggers) → state-variable lowpass with a slow LFO → bitcrusher on some
  bars → dotted-eighth delay

and a granular drone (50 ms grains of the clip, an octave down, hundreds
a second) under the breaks. Deterministic: the same clip gives the same
chop. 60 s, 44.1 kHz, mono, peaks at −6 dBFS, 32 bars at 128 bpm.
"""
import sys, wave
import numpy as np

SR = 44100
BPM = 128
BARS = 32
STEP = 60 / BPM / 4                      # a sixteenth, in seconds
SN = int(round(STEP * SR))               # … in samples
TOTAL = BARS * 16 * SN
rng = np.random.default_rng(0xA1EA01)


def read(path):
    with wave.open(path) as w:
        n, sw, ch = w.getnframes(), w.getsampwidth(), w.getnchannels()
        raw = np.frombuffer(w.readframes(n), dtype={1: np.int8, 2: np.int16, 4: np.int32}[sw]).astype(np.float64)
        raw /= float(2 ** (8 * sw - 1))
        if ch > 1:
            raw = raw.reshape(-1, ch).mean(axis=1)
        return raw


def write(path, x):
    x = np.clip(x, -1, 1)
    with wave.open(path, "wb") as w:
        w.setnchannels(1); w.setsampwidth(2); w.setframerate(SR)
        w.writeframes((x * 32767).astype(np.int16).tobytes())


def env(n, attack=0.002, release=0.12):
    a, r = int(attack * SR), int(release * SR)
    e = np.ones(n)
    e[:a] = np.linspace(0, 1, a)
    if r < n:
        e[n - r:] *= np.linspace(1, 0, r)
    else:
        e *= np.linspace(1, 0, n)
    return e


def slicer(src):
    """All the sixteenth slices of the clip, with their loudness."""
    n = len(src) // SN
    slices = [src[k * SN:(k + 1) * SN] for k in range(n)]
    loud = np.array([np.sqrt(np.mean(s * s)) for s in slices])
    return slices, loud / loud.max()


def pitch(s, rate):
    """Resample a slice: rate 2 is an octave up, −1 backwards."""
    if rate < 0:
        s = s[::-1]; rate = -rate
    idx = np.arange(0, len(s), rate)
    idx = idx[idx < len(s) - 1]
    i0 = idx.astype(int)
    return s[i0] * (1 - (idx - i0)) + s[i0 + 1] * (idx - i0)


def lowpass(x, cutoff):
    """A one-pole lowpass whose cutoff (Hz, an array) moves."""
    y = np.zeros_like(x)
    a = np.exp(-2 * np.pi * cutoff / SR)
    acc = 0.0
    for i in range(len(x)):
        acc = a[i] * acc + (1 - a[i]) * x[i]
        y[i] = acc
    return y


def crush(x, bits=6, hold=4):
    q = 2 ** (bits - 1)
    held = x[::hold].repeat(hold)[:len(x)]
    return np.round(held * q) / q


def delay(x, time=STEP * 3, feedback=0.45, mix=0.3):
    d = int(time * SR)
    y = x.copy()
    for i in range(d, len(x)):
        y[i] += feedback * y[i - d]
    return x * (1 - mix) + y * mix


def sequencer(slices, loud):
    """Which slice fires on each sixteenth, at which rate, how loud."""
    out = np.zeros(TOTAL)
    heavy = np.argsort(loud)[-max(4, len(slices) // 3):]      # the loud third of the clip
    for bar in range(BARS):
        section = bar // 4
        # a gate pattern for these 4 bars: dense in the drops, sparse in the intro and break
        if section == 0:            density = 0.25
        elif section in (4,):       density = 0.15              # the break
        elif section == 7:          density = 0.6
        else:                       density = 0.45
        gates = rng.random(16) < density
        gates[0] = section not in (4,)                          # the downbeat, except in the break
        for step in range(16):
            if not gates[step]:
                continue
            k = int(rng.choice(heavy)) if rng.random() < 0.7 else int(rng.integers(len(slices)))
            rate = rng.choice([0.5, 1, 1, 1, 2, -1], p=[0.15, 0.35, 0.2, 0.1, 0.1, 0.1])
            s = pitch(slices[k], rate)
            vel = 0.6 + 0.4 * rng.random()
            pos = (bar * 16 + step) * SN
            if rng.random() < 0.18 and step % 4 == 2:            # a stutter: 1/64 retriggers
                for r in range(4):
                    piece = s[:SN // 4] * env(SN // 4, 0.001, 0.02)
                    out[pos + r * (SN // 4):pos + r * (SN // 4) + len(piece)] += piece * vel * (1 - 0.15 * r)
            else:
                n = min(len(s), SN * (2 if rng.random() < 0.2 else 1))
                piece = s[:n] * env(n, 0.002, 0.06 if rate >= 1 else 0.15)
                end = min(pos + n, TOTAL)
                out[pos:end] += piece[:end - pos] * vel
    return out


def drone(src):
    """A granular drone from the clip, an octave down: under the breaks."""
    out = np.zeros(TOTAL)
    g = int(0.05 * SR)
    win = np.hanning(g * 2)
    for pos in range(0, TOTAL, g // 3):
        bar = pos / (16 * SN)
        gain = 0.0
        if 14 <= bar < 20:  gain = min(1, (bar - 14) / 2) * min(1, (20 - bar) / 1)
        if 28 <= bar < 31.5: gain = min(1, (bar - 28) / 2)
        if gain <= 0:
            continue
        start = int(rng.integers(0, len(src) - g * 2 - 1))
        grain = pitch(src[start:start + g * 2], 0.5)[:g * 2] * win
        end = min(pos + len(grain), TOTAL)
        out[pos:end] += grain[:end - pos] * gain * 0.5
    return out


def main(src_path, out_path):
    src = read(src_path)
    src = src / (np.abs(src).max() + 1e-9)
    slices, loud = slicer(src)
    seq = sequencer(slices, loud)

    # the filter: a slow LFO over each 8 bars, closed in the intro, open in the drops
    tt = np.arange(TOTAL) / SR
    bar = tt / (16 * STEP)
    lfo = 0.5 - 0.5 * np.cos(2 * np.pi * bar / 8)
    base = np.where(bar < 4, 400, np.where(bar < 16, 1200, np.where(bar < 20, 500, 2500)))
    seq = lowpass(seq, base + 5000 * lfo * (bar >= 4))

    # the bitcrusher, on bars 12–15 and 24–27
    crushed = crush(seq)
    mask = ((bar >= 12) & (bar < 16)) | ((bar >= 24) & (bar < 28))
    seq = np.where(mask, crushed, seq)

    mix = delay(seq) + drone(src)
    mix[-int(0.3 * SR):] *= np.linspace(1, 0, int(0.3 * SR))
    mix *= 0.5 / (np.abs(mix).max() + 1e-9)              # −6 dBFS
    write(out_path, mix)


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
