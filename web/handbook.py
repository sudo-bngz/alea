#!/usr/bin/env python3
"""Builds the alea Handbook: docs/guide.md and docs/reference.md as one web
page, with a side navigation, highlighted alea code, and each guide chapter
illustrated by the output of its tour file (examples/tour/, edition 0x2a):
its pictures, its videos as looping animated WebP, and its sounds behind a
play button. A `> Figure: name.alea` line in the guide shows the videos of
docs/figures/name.alea the same way.

    python3 web/handbook.py            # renders the tour with target/release/alea
    ALEA_RENDERS=dir python3 web/handbook.py   # reuses renders already in dir/<name>/

Writes web/dist/handbook.html (a full page) and web/dist/handbook.body.html
(the same content without the document skeleton, for hosts that add their own).
Needs the `markdown` and `Pillow` Python packages, and ffmpeg."""
import base64, io, os, pathlib, re, subprocess, tempfile

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

def data(path, mime):
    return f"data:{mime};base64," + base64.b64encode(path.read_bytes()).decode()

def ffmpeg(*args):
    subprocess.run(["ffmpeg", "-v", "error", "-y", *map(str, args)], check=True, stdin=subprocess.DEVNULL)

def still(p, high=150):
    """A picture as a JPEG thumbnail, with its width and height when shown `high` pixels high."""
    im = Image.open(p).convert("RGB")
    im.thumbnail((high * 4, high * 2))
    buf = io.BytesIO()
    im.save(buf, "JPEG", quality=82)
    return "data:image/jpeg;base64," + base64.b64encode(buf.getvalue()).decode(), round(high * im.width / im.height), high

def loop(mp4, width=220, high=150):
    """A video as a silent looping WebP, made from alea's own render (cached next to it).
    A busy one is made again smaller and slower, to keep the page light."""
    out = mp4.with_suffix(".hb.webp")
    if not out.exists():
        for fps, w, q in [(10, width, 32), (8, 180, 26), (6, 160, 20)]:
            ffmpeg("-i", mp4, "-t", 12, "-vf", f"fps={fps},scale={w}:-2:flags=lanczos", "-an",
                   "-c:v", "libwebp_anim", "-lossless", 0, "-q:v", q, "-compression_level", 6, "-loop", 0, out)
            if out.stat().st_size < 240_000:
                break
    w, h = Image.open(out).size
    return data(out, "image/webp"), round(high * w / h), high

def clip(src, video):
    """A sound (or a video with its sound) small enough to live in the page: at most 15 seconds, fading out."""
    out = src.with_suffix(".hb.mp4" if video else ".hb.m4a")
    if not out.exists():
        dur = float(subprocess.run(["ffprobe", "-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0", str(src)],
                                   capture_output=True, text=True).stdout or 15)
        cut = min(dur, 15)
        fade = f"afade=t=out:st={max(cut - 1, 0):.2f}:d=1"
        if video:
            ffmpeg("-i", src, "-t", cut, "-vf", "scale=300:-2", "-c:v", "libx264", "-crf", 34, "-preset", "slow",
                   "-pix_fmt", "yuv420p", "-movflags", "+faststart", "-af", fade, "-c:a", "aac", "-b:a", "64k", out)
        else:
            ffmpeg("-i", src, "-t", cut, "-af", fade, "-c:a", "aac", "-b:a", "64k", "-movflags", "+faststart", out)
    return data(out, "video/mp4" if video else "audio/mp4")

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
    mp4s = [p for p in mp4s if ".hb." not in p.name]
    poster, w, h = still(pngs[0], high=260)
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
    pngs = [p for p in ordered(folder, "*.png") if ".hb." not in p.name]
    mp4s = [p for p in ordered(folder, "*.mp4") if ".hb." not in p.name]
    figs = [img(*(loop(p, 360, 300) if big else loop(p)), alt="") for p in mp4s]
    # a video's own still is usually the work's first picture: show the others
    figs += [img(*still(p)) for p in (pngs[1:] if figs else pngs)][: max(limit - len(figs), 0)]
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
    return strip(stem, folder, caption, limit=12, big=len(list(folder.glob("*.mp4"))) == 1)

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
