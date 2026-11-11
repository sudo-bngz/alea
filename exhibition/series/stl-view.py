#!/usr/bin/env python3
"""Render an STL that alea wrote as a lit solid on black, for the picture
series (episode 11, Sculpture): what the file holds, seen as an object.

usage: ./stl-view.py in.stl out.png [elevation azimuth]"""
import struct
import sys

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt  # noqa: E402
import numpy as np  # noqa: E402
from mpl_toolkits.mplot3d.art3d import Poly3DCollection  # noqa: E402


def read_stl(path):
    data = open(path, "rb").read()
    n = struct.unpack_from("<I", data, 80)[0]
    rec = np.frombuffer(data, dtype=np.dtype([("n", "<f4", 3), ("v", "<f4", (3, 3)), ("a", "<u2")]), count=n, offset=84)
    return rec["v"].astype(float)


src, out = sys.argv[1], sys.argv[2]
elev = float(sys.argv[3]) if len(sys.argv) > 3 else 32
azim = float(sys.argv[4]) if len(sys.argv) > 4 else -58
tri = read_stl(src)

# shade each face by its normal: a key light, a soft fill, a red rim from behind
nrm = np.cross(tri[:, 1] - tri[:, 0], tri[:, 2] - tri[:, 0])
nrm /= np.linalg.norm(nrm, axis=1, keepdims=True) + 1e-12
# light every face from its outer side (the file's winding is not trusted here)
cen = tri.mean(axis=1) - tri.reshape(-1, 3).mean(0)
up = np.where(np.abs(nrm[:, 2]) > 0.05, np.sign(nrm[:, 2]), np.sign(np.sum(nrm[:, :2] * cen[:, :2], axis=1)))
nrm *= up[:, None]
key = np.array([0.35, -0.75, 0.6]); key /= np.linalg.norm(key)
rim = np.array([-0.7, 0.6, 0.3]); rim /= np.linalg.norm(rim)
lam = np.clip(nrm @ key, 0, 1)
r = np.clip(nrm @ rim, 0, 1) ** 6
g = 0.08 + 0.8 * lam
col = np.stack([np.clip(g + 0.55 * r, 0, 1), g * (1 - 0.3 * r), g * (1 - 0.4 * r), np.ones_like(g)], axis=1)

fig = plt.figure(figsize=(6, 6), dpi=360, facecolor="black")
ax = fig.add_axes([0, 0, 1, 1], projection="3d", facecolor="black")
ax.add_collection3d(Poly3DCollection(tri, facecolors=col, edgecolors=col, linewidths=0.25, antialiased=False))
lo, hi = tri.reshape(-1, 3).min(0), tri.reshape(-1, 3).max(0)
mid, span = (lo + hi) / 2, (hi - lo).max() / 2
ax.set_xlim(mid[0] - span, mid[0] + span)
ax.set_ylim(mid[1] - span, mid[1] + span)
ax.set_zlim(mid[2] - span, mid[2] + span)
ax.set_box_aspect((1, 1, 1))
ax.view_init(elev=elev, azim=azim)
ax.set_proj_type("persp", focal_length=0.6)
ax.set_axis_off()
ax.dist = 7.2
fig.savefig(out, facecolor="black")
# drawn at twice the size, brought down: no seams between the faces
from PIL import Image  # noqa: E402
im = Image.open(out).convert("RGB")
# keep the object: crop to what is not black, square, with a little air
box = Image.eval(im.convert("L"), lambda v: 255 if v > 12 else 0).getbbox()
cx, cy = (box[0] + box[2]) / 2, (box[1] + box[3]) / 2
half = max(box[2] - box[0], box[3] - box[1]) * 0.54
im = im.crop((int(cx - half), int(cy - half), int(cx + half), int(cy + half)))
im.resize((1080, 1080), Image.LANCZOS).save(out)
print(out)
