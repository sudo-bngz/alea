#!/usr/bin/env python3
"""alea imagine engine: no model at all, for trying `imagine` and for tests.
    imagine("a rainy street at night", engine: "imagine/sketch.py")
It draws soft clouds of colour: the prompt chooses the colours, the seed the
shapes. Needs only Python. Copy it to write an engine of your own: the prompt
comes on the standard input, ALEA_SEED, ALEA_WIDTH, ALEA_HEIGHT and ALEA_MODEL
in the environment, and the picture (PNG, JPEG or WebP) goes to the standard
output."""
import hashlib, math, os, random, struct, sys, zlib

prompt = sys.stdin.read().strip()
seed = int(os.environ.get("ALEA_SEED", "0"))
W, H = int(os.environ.get("ALEA_WIDTH", "1024")), int(os.environ.get("ALEA_HEIGHT", "1024"))

words = random.Random(hashlib.sha256(prompt.encode()).digest())
# a dark, a strong and a light colour, so the clouds have some contrast
colours = [[words.uniform(0, 60) for _ in range(3)], [words.choice([20, 120, 235]) + words.uniform(0, 20) for _ in range(3)],
           [words.uniform(190, 255) for _ in range(3)]]
shapes = random.Random(seed)
waves = [(shapes.uniform(1, 5), shapes.uniform(1, 5), shapes.uniform(0, 6.28), shapes.uniform(0.5, 1)) for _ in range(6)]

# drawn small, then enlarged smoothly: enough for clouds, and quick
N = 96
small = []
for j in range(N + 1):
    row = []
    for i in range(N + 1):
        u, v = i / N, j / N
        a = sum(w * math.sin(6.28 * (fx * u + fy * v) + ph + 2 * math.sin(3 * v + ph)) for fx, fy, ph, w in waves[:3]) / 3
        b = sum(w * math.sin(6.28 * (fx * v - fy * u) + ph) for fx, fy, ph, w in waves[3:]) / 3
        a, b = 0.5 + 0.5 * a, 0.5 + 0.5 * b
        row.append([colours[0][c] * (1 - a) + (colours[1][c] * (1 - b) + colours[2][c] * b) * a for c in range(3)])
    small.append(row)

raw = bytearray()
for y in range(H):
    fy = y / max(H - 1, 1) * N
    j = min(int(fy), N - 1)
    ty = fy - j
    top, low = small[j], small[j + 1]
    line = [[top[i][c] * (1 - ty) + low[i][c] * ty for c in range(3)] for i in range(N + 1)]
    raw.append(0)
    for x in range(W):
        fx = x / max(W - 1, 1) * N
        i = min(int(fx), N - 1)
        tx = fx - i
        p, q = line[i], line[i + 1]
        raw += bytes(int(p[c] * (1 - tx) + q[c] * tx) for c in range(3))

def chunk(kind, data):
    return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))

sys.stdout.buffer.write(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", W, H, 8, 2, 0, 0, 0))
                        + chunk(b"IDAT", zlib.compress(bytes(raw), 6)) + chunk(b"IEND", b""))
