#!/usr/bin/env python3
"""Assembles the site (alea.bngz.io) in _site/: the pages of site/, the browser
engine and the work drawn live on the home page, the exhibition's editions
(resized for the web), two Reels, the fonts, the playground and the handbook.

    web/build.sh && python3 web/handbook.py   # the engine, playground, handbook
    python3 site/build.py                     # → _site/
    python3 -m http.server -d _site           # to look at it

Needs Pillow."""
import pathlib, re, shutil

from PIL import Image

root = pathlib.Path(__file__).resolve().parent.parent
site, out = root / "site", root / "_site"
web, dist, expo = root / "web", root / "web" / "dist", root / "exhibition"
posts = expo / "instagram"

# The works of the gallery, in order: (file stem in instagram/, title, source, what it is)
WORKS = [
    ("isohypses", "Isohypses", "exhibition/isohypses.alea",
     "Maps of places that don't exist: contour lines over an imaginary relief, layer tints, a hatched sea, towns, and the cartouche of a survey sheet."),
    ("nocturne", "Nocturne", "exhibition/nocturne.alea",
     "The moon over a sea at night, its path broken into glints by waves drawn in perspective."),
    ("rosace", "Rosace", "exhibition/rosace.alea",
     "A rose window: panes from a noise field, lead drawn as contour lines, tracery folded into the window's symmetry."),
    ("correspondances", "Correspondances", "exhibition/correspondances.alea",
     "The plan of a metro for a Paris that was never built: the real city's grammar, a new network every edition."),
    ("culture", "Culture", "exhibition/culture.alea",
     "A petri dish: colonies seeded at random grow by reaction–diffusion, two kinds of life side by side."),
]
REELS = [
    ("8-correspondances-reel", "Correspondances", "One day of service on the network, with its sound."),
    ("10-culture-reel", "Culture", "Seventy-two hours of incubation in twenty seconds."),
]


def editions(stem):
    """The posted editions of a work: [(file, seed)], in carousel order."""
    found = []
    for p in sorted(posts.glob(f"*-{stem}-*-0x*.jpg"), key=lambda p: int(p.name.split("-")[2])):
        found.append((p, re.search(r"(0x[0-9a-f]+)\.jpg$", p.name).group(1)))
    return found


def resized(src, dest, width, quality=82):
    im = Image.open(src).convert("RGB")
    if im.width > width:
        im = im.resize((width, round(im.height * width / im.width)), Image.LANCZOS)
    dest.parent.mkdir(parents=True, exist_ok=True)
    im.save(dest, "WEBP", quality=quality, method=6)
    return im.size


def main():
    if out.exists():
        shutil.rmtree(out)
    out.mkdir()
    for f in ["index.html", "site.css", "site.js", "CNAME", "favicon.svg"]:
        shutil.copy(site / f, out / f)

    # the engine, and the work drawn live on the home page
    shutil.copy(web / "engine.js", out / "engine.js")
    shutil.copy(dist / "alea.wasm", out / "alea.wasm")
    (out / "works").mkdir()
    shutil.copy(expo / "isohypses.alea", out / "works" / "isohypses.alea")

    # fonts: alea's own (Lora for text, DejaVu Sans Mono for code)
    (out / "fonts").mkdir()
    shutil.copy(root / "fonts" / "serif.ttf", out / "fonts" / "lora.ttf")
    shutil.copy(root / "fonts" / "mono.ttf", out / "fonts" / "mono.ttf")

    # the gallery: every posted edition, at two sizes
    gallery = []
    for stem, title, source, about in WORKS:
        eds = []
        for path, seed in editions(stem):
            name = f"{stem}-{seed}"
            w, h = resized(path, out / "img" / f"{name}-s.webp", 360)
            resized(path, out / "img" / f"{name}.webp", 1080, 86)
            eds.append((name, seed, w, h))
        gallery.append((stem, title, source, about, eds))
    # the still shown before the live edition is ready
    first = editions("isohypses")[0][0]
    resized(first, out / "img" / "hero.webp", 1080, 86)
    shutil.copy(posts / "6-alea-card.jpg", out / "og.jpg")

    # two Reels, with their covers
    for stem, _, _ in REELS:
        shutil.copy(posts / f"{stem}.mp4", out / "img" / f"{stem}.mp4")
        resized(posts / f"{stem}-cover.jpg", out / "img" / f"{stem}.webp", 720)

    # the gallery and the Reels go into the page
    html = (out / "index.html").read_text()
    html = html.replace("<!--GALLERY-->", gallery_html(gallery))
    html = html.replace("<!--REELS-->", reels_html())
    (out / "index.html").write_text(html)

    # the playground and the handbook, if built
    for page in ["playground.html", "handbook.html"]:
        if (dist / page).exists():
            shutil.copy(dist / page, out / page)
        else:
            print(f"note: web/dist/{page} is missing (web/build.sh, web/handbook.py)")
    size = sum(p.stat().st_size for p in out.rglob("*") if p.is_file())
    print(f"_site/: {sum(1 for p in out.rglob('*') if p.is_file())} files, {size / 1e6:.1f} MB")


def gallery_html(gallery):
    parts = []
    repo = "https://github.com/sudo-bngz/alea/blob/main/"
    for stem, title, source, about, eds in gallery:
        figs = "\n".join(
            f'''        <li><a href="img/{name}.webp" class="ed"><img src="img/{name}-s.webp" width="{w}" height="{h}" loading="lazy" alt="{title}, edition {seed}"><span class="seed">{seed}</span></a></li>'''
            for name, seed, w, h in eds)
        parts.append(f'''    <article class="work" id="{stem}">
      <header class="work-head">
        <h3>{title}</h3>
        <p>{about}</p>
        <p class="work-links"><a href="{repo}{source}">Read the code</a> <code>alea {source.split('/')[-1]} --seed {eds[0][1]}</code></p>
      </header>
      <ol class="editions">
{figs}
      </ol>
    </article>''')
    return "\n".join(parts)


def reels_html():
    return "\n".join(
        f'''      <figure class="reel">
        <video src="img/{stem}.mp4" poster="img/{stem}.webp" controls preload="none" playsinline width="360" height="640"></video>
        <figcaption><strong>{title}</strong> {caption}</figcaption>
      </figure>''' for stem, title, caption in REELS)


if __name__ == "__main__":
    main()
