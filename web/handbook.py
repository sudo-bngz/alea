#!/usr/bin/env python3
"""Builds the alea Handbook: docs/guide.md and docs/reference.md as one web
page, with a side navigation, highlighted alea code, and each guide chapter
illustrated by the output of its tour file (examples/tour/, edition 0x2a):
its pictures, its videos as looping animated WebP, and its sounds behind a
play button. A `> Figure: name.alea` line in the guide shows the videos of
docs/figures/name.alea the same way.

    python3 web/handbook.py            # renders the tour with target/release/alea
    ALEA_RENDERS=dir python3 web/handbook.py   # reuses renders already in dir/<name>/

Writes web/dist/handbook.html (a full page), web/dist/handbook.body.html, and
the pictures, animations and sounds they show in web/dist/handbook-media/
(the same content without the document skeleton, for hosts that add their own).
Needs the `markdown` and `Pillow` Python packages, and ffmpeg."""
import os, pathlib, re, shutil, subprocess, tempfile

import markdown
from markdown.extensions.toc import slugify as md_slugify
from PIL import Image

root = pathlib.Path(__file__).resolve().parent.parent
web, docs, tour = root / "web", root / "docs", root / "examples" / "tour"

# ── tour renders ──
figures_dir = docs / "figures"

def render_all(src_dir, out):
    for f in sorted(src_dir.glob("*.alea")):
        if not (out / f.stem).exists():
            subprocess.run([str(root / "target/release/alea"), str(f), "--seed", "0x2a", "-o", str(out / f.stem)],
                           stdin=subprocess.DEVNULL, check=True, stdout=subprocess.DEVNULL)

renders = pathlib.Path(os.environ.get("ALEA_RENDERS") or tempfile.mkdtemp(prefix="alea-tour-"))
render_all(tour, renders)
render_all(figures_dir, renders / "_figures")

def ordered(folder, pattern):
    return sorted(folder.glob(pattern), key=lambda p: (len(p.name), p.name))

# The media sit beside the page, in web/dist/handbook-media/, and load as the
# reader scrolls to them: at full quality they are too heavy to inline.
MEDIA = web / "dist" / "handbook-media"
shutil.rmtree(MEDIA, ignore_errors=True)
MEDIA.mkdir(parents=True)

def ffmpeg(*args):
    subprocess.run(["ffmpeg", "-v", "error", "-y", *map(str, args)], check=True, stdin=subprocess.DEVNULL)

def shipped(made, name):
    """Copy a made file into the media folder; its URL from the page."""
    shutil.copy(made, MEDIA / name)
    return f"handbook-media/{name}"

def label(p):
    """A stable name for one render: tour or figure stem, then its index among the work's renders."""
    return f"{p.parent.name}{p.stem.split('-000000000000002a')[-1] or ''}"

def still(p, high=240):
    """A picture, sharp on a high-density screen; its URL, and its width and height shown `high` pixels high."""
    out = p.with_suffix(f".hb{high}.jpg")
    if not out.exists():
        im = Image.open(p).convert("RGB")
        im.thumbnail((high * 8, high * 2), Image.LANCZOS)
        im.save(out, "JPEG", quality=90)
    w, h = Image.open(out).size
    return shipped(out, f"{label(p)}.jpg"), round(high * w / h), high

def loop(mp4, high=240):
    """A video as a silent looping animated WebP, from alea's own render (cached next to it):
    twice the size it is shown, 20 frames a second. A very busy one is made a little
    lighter, never blurry."""
    out = mp4.with_suffix(f".hb{high}.webp")
    if not out.exists():
        for fps, q in [(20, 70), (20, 55), (15, 50)]:
            ffmpeg("-i", mp4, "-t", 10, "-vf", f"fps={fps},scale=-2:{high * 2}:flags=lanczos", "-an",
                   "-c:v", "libwebp_anim", "-lossless", 0, "-q:v", q, "-compression_level", 6, "-loop", 0, out)
            if out.stat().st_size < 2_500_000:
                break
    w, h = Image.open(out).size
    return shipped(out, f"{label(mp4)}.webp"), round(high * w / h), high

def clip(src, video):
    """A sound, or a video with its sound: at most 15 seconds, fading out."""
    out = src.with_suffix(".hb.mp4" if video else ".hb.m4a")
    if not out.exists():
        dur = float(subprocess.run(["ffprobe", "-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0", str(src)],
                                   capture_output=True, text=True).stdout or 15)
        cut = min(dur, 15)
        fade = f"afade=t=out:st={max(cut - 1, 0):.2f}:d=1"
        if video:
            ffmpeg("-i", src, "-t", cut, "-vf", "scale=-2:720:flags=lanczos", "-c:v", "libx264", "-crf", 22, "-preset", "slow",
                   "-pix_fmt", "yuv420p", "-movflags", "+faststart", "-af", fade, "-c:a", "aac", "-b:a", "160k", out)
        else:
            ffmpeg("-i", src, "-t", cut, "-af", fade, "-c:a", "aac", "-b:a", "160k", "-movflags", "+faststart", out)
    return shipped(out, f"{label(src)}{out.suffix}")

# ── markdown ──
def convert(path, prefix):
    md = markdown.Markdown(
        extensions=["tables", "fenced_code", "toc"],
        extension_configs={"toc": {"slugify": lambda v, s: prefix + md_slugify(v, s), "toc_depth": "2"}},
    )
    html = md.convert(path.read_text())
    return html, md.toc_tokens

guide_html, guide_toc = convert(docs / "guide.md", "guide-")
ref_html, ref_toc = convert(docs / "reference.md", "ref-")

def relink(html, own):
    html = re.sub(r'href="reference\.md#([^"]+)"', r'href="#ref-\1"', html)
    html = html.replace('href="reference.md"', 'href="#reference"').replace('href="guide.md"', 'href="#guide"')
    if own == "guide-":
        html = re.sub(r'href="#(?!ref-|guide|reference)([^"]+)"', r'href="#guide-\1"', html)
    if own == "ref-":
        html = re.sub(r'href="#(?!ref-|guide|reference)([^"]+)"', r'href="#ref-\1"', html)
    return html

guide_html = relink(guide_html, "guide-")
# A chapter's own number goes into its tag: <h2>3. Chance</h2> → <h2><span class="num">03</span>Chance</h2>
guide_html = re.sub(r'(<h2 id="[^"]+">)(\d+)\. ', lambda m: f'{m.group(1)}<span class="num">{int(m.group(2)):02d}</span>', guide_html)
ref_html = relink(ref_html, "ref-")

# Illustrate each guide chapter with its tour file's renders: pictures as they
# are, videos as looping animations, and in the chapters about sound, the
# sound itself, to play over its picture (or its video).
SOUND = {"13-sound", "16-lighthouse", "24-listening", "25-midi", "26-harmony", "27-synths", "28-files"}

def img(src, w, h, alt=""):
    return f'<img src="{src}" alt="{alt}" loading="lazy" width="{w}" height="{h}">'

def has_sound(mp4):
    probe = ["ffprobe", "-v", "error", "-select_streams", "a", "-show_entries", "stream=index", "-of", "csv=p=0", str(mp4)]
    return bool(subprocess.run(probe, capture_output=True, text=True).stdout.strip())

def player(stem, folder):
    pngs, mp4s, wavs = ordered(folder, "*.png"), ordered(folder, "*.mp4"), ordered(folder, "*.wav")
    mp4s = [p for p in mp4s if ".hb" not in p.name]
    poster, w, h = still(pngs[0], high=360)
    if mp4s and has_sound(mp4s[0]):
        media = f'<video src="{clip(mp4s[0], True)}" poster="{poster}" preload="none" playsinline width="{w}" height="{h}"></video>'
    elif wavs:
        media = img(poster, w, h) + f'<audio src="{clip(wavs[0], False)}" preload="none"></audio>'
    else:
        return None
    return (f'<figure class="strip"><div class="player" style="--w:{w}px">{media}'
            f'<button type="button" class="play" aria-label="Play {stem}.alea">'
            f'<svg viewBox="0 0 16 16" aria-hidden="true"><path class="i-play" d="M4 2.5v11l9.5-5.5z"/>'
            f'<path class="i-pause" d="M3.5 2.5h3v11h-3zM9.5 2.5h3v11h-3z"/></svg></button>'
            f'<span class="bar"><i></i></span></div>'
            f'<figcaption>{stem}.alea, edition 0x2a · press play to listen</figcaption></figure>')

def strip(stem, folder, caption, limit=4, big=False):
    high = 360 if big else 240
    pngs = [p for p in ordered(folder, "*.png") if ".hb" not in p.name]
    mp4s = [p for p in ordered(folder, "*.mp4") if ".hb" not in p.name]
    figs = [img(*loop(p, high)) for p in mp4s]
    # a video's own still is usually the work's first picture: show the others
    figs += [img(*still(p, high)) for p in (pngs[1:] if figs else pngs)[: max(limit - len(figs), 0)]]
    if not figs:
        return ""
    return f'<figure class="strip">{"".join(figs)}<figcaption>{caption}</figcaption></figure>'

def illustrate(m):
    stem = m.group(1).split(" ")[0].removesuffix(".alea")
    folder = renders / stem
    if not folder.exists():
        return m.group(0)
    shown = (player(stem, folder) if stem in SOUND else None) or strip(stem, folder, f"{stem}.alea, edition 0x2a")
    return m.group(0) + shown

def figure(m):
    stem, about = m.group(1).removesuffix(".alea"), m.group(2).strip(" ,")
    caption = f"{about} · docs/figures/{stem}.alea" if about else f"docs/figures/{stem}.alea"
    folder = renders / "_figures" / stem
    shown = [p for p in folder.glob("*") if p.suffix in (".png", ".mp4") and ".hb" not in p.name]
    return strip(stem, folder, caption, limit=12, big=len(shown) == 1)

guide_html = re.sub(r"<blockquote>\s*<p>Tour: <code>([^<]+)</code>.*?</blockquote>", illustrate, guide_html, flags=re.S)
guide_html = re.sub(r"<blockquote>\s*<p>Figure: <code>([^<]+)</code>(.*?)</p>\s*</blockquote>", figure, guide_html, flags=re.S)

def nav(tokens, label, anchor):
    items = "".join(f'<li><a href="#{t["id"]}">{t["name"]}</a></li>' for top in tokens for t in ([top] if top["level"] == 2 else top["children"]))
    return f'<p class="nav-head"><a href="#{anchor}">{label}</a></p><ul>{items}</ul>'

page = (web / "handbook.html").read_text()
page = page.replace("<!--NAV-->", nav(guide_toc, "Guide", "guide") + nav(ref_toc, "Reference", "reference"))
page = page.replace("<!--GUIDE-->", guide_html).replace("<!--REFERENCE-->", ref_html)
(web / "dist").mkdir(exist_ok=True)
(web / "dist" / "handbook.body.html").write_text(page)
skeleton = ('<!doctype html><html lang="en"><head><meta charset="utf-8">'
            '<meta name="viewport" content="width=device-width,initial-scale=1,viewport-fit=cover">'
            '<style>[hidden]{display:none!important}body{margin:0}</style></head><body>')
(web / "dist" / "handbook.html").write_text(skeleton + page + "</body></html>")
print(f"web/dist/handbook.html  {len(page) / 1e6:.2f} MB")
