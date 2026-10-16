#!/usr/bin/env python3
"""Builds the alea Handbook: docs/guide.md and docs/reference.md as one web
page, with a side navigation, highlighted alea code, and each guide chapter
illustrated by the output of its tour file (examples/tour/, edition 0x2a).

    python3 web/handbook.py            # renders the tour with target/release/alea
    ALEA_RENDERS=dir python3 web/handbook.py   # reuses renders already in dir/<name>/

Writes web/dist/handbook.html (a full page) and web/dist/handbook.body.html
(the same content without the document skeleton, for hosts that add their own).
Needs the `markdown` and `Pillow` Python packages."""
import base64, io, os, pathlib, re, subprocess, tempfile

import markdown
from markdown.extensions.toc import slugify as md_slugify
from PIL import Image

root = pathlib.Path(__file__).resolve().parent.parent
web, docs, tour = root / "web", root / "docs", root / "examples" / "tour"

# ── tour renders ──
renders = os.environ.get("ALEA_RENDERS")
if not renders:
    renders = tempfile.mkdtemp(prefix="alea-tour-")
    for f in sorted(tour.glob("*.alea")):
        subprocess.run([str(root / "target/release/alea"), str(f), "--seed", "0x2a", "-o", f"{renders}/{f.stem}"],
                       check=True, stdout=subprocess.DEVNULL)
renders = pathlib.Path(renders)

def thumbs(stem, limit=4):
    files = sorted((renders / stem).glob("*.png"), key=lambda p: (len(p.name), p.name))
    out = []
    for p in files[:limit]:
        im = Image.open(p).convert("RGB")
        im.thumbnail((300, 300))
        buf = io.BytesIO()
        im.save(buf, "JPEG", quality=82)
        out.append("data:image/jpeg;base64," + base64.b64encode(buf.getvalue()).decode())
    return out

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
    if own == "ref-":
        html = re.sub(r'href="#(?!ref-|guide|reference)([^"]+)"', r'href="#ref-\1"', html)
    return html

guide_html = relink(guide_html, "guide-")
ref_html = relink(ref_html, "ref-")

# Illustrate each guide chapter with its tour file's renders.
def illustrate(m):
    stem = m.group(1).split(" ")[0].removesuffix(".alea")
    imgs = thumbs(stem)
    if not imgs:
        return m.group(0)
    figs = "".join(f'<img src="{s}" alt="" loading="lazy" width="150" height="150">' for s in imgs)
    return (m.group(0) + f'<figure class="strip">{figs}<figcaption>{stem}.alea, edition 0x2a</figcaption></figure>')

guide_html = re.sub(r"<blockquote>\s*<p>Tour: <code>([^<]+)</code>.*?</blockquote>", illustrate, guide_html, flags=re.S)

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
