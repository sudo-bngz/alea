#!/usr/bin/env python3
"""Assembles the site (alea.bngz.io) in _site/: the pages of site/, the browser
engine and the work drawn live on the home page (examples/isohypses.alea, with
a still shown until it is drawn), the fonts, the playground and the handbook.

    web/build.sh && python3 web/handbook.py   # the engine, playground, handbook
    cargo build --release                     # alea, to render the network example
    python3 site/build.py                     # → _site/
    python3 -m http.server -d _site           # to look at it

Needs Pillow, and alea built (target/release/alea)."""
import pathlib, shutil, subprocess, tempfile

from PIL import Image

root = pathlib.Path(__file__).resolve().parent.parent
site, out = root / "site", root / "_site"
web, dist = root / "web", root / "web" / "dist"
# the work on the home page, and the still shown until the browser has drawn it
WORK = root / "examples" / "isohypses.alea"
STILL = root / "exhibition" / "intro" / "stills" / "isohypses-0xa1ea03.jpg"
# the example beside "Chance is part of the language": one edition per structure `choose` can pick
NETWORK, NETWORK_SEEDS = root / "examples" / "network.alea", ["0x1", "0x2", "0x3"]

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
    shutil.copy(root / "fm1" / "send.html", out / "fm1.html")   # a bank of patches to the M-VAVE FM-1, over Web MIDI

    # the engine, and the work drawn live on the home page
    shutil.copy(web / "engine.js", out / "engine.js")
    shutil.copy(dist / "alea.wasm", out / "alea.wasm")
    (out / "works").mkdir()
    shutil.copy(WORK, out / "works" / "isohypses.alea")

    # fonts: alea's own (Lora for text, DejaVu Sans Mono for code)
    (out / "fonts").mkdir()
    shutil.copy(root / "fonts" / "serif.ttf", out / "fonts" / "lora.ttf")
    shutil.copy(root / "fonts" / "mono.ttf", out / "fonts" / "mono.ttf")

    # the still shown before the live edition is ready, and the card for links
    resized(STILL, out / "img" / "hero.webp", 1080, 86)
    shutil.copy(site / "og.jpg", out / "og.jpg")

    # the network example's editions, rendered by alea (cargo build --release first)
    alea = root / "target" / "release" / "alea"
    with tempfile.TemporaryDirectory() as tmp:
        for seed in NETWORK_SEEDS:
            subprocess.run([str(alea), str(NETWORK), "--seed", seed, "-o", f"{tmp}/{seed}"], check=True,
                           stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL)
            resized(next(pathlib.Path(tmp, seed).glob("*.png")), out / "img" / f"network-{seed}.webp", 480)

    # the playground and the handbook, if built
    for page in ["playground.html", "handbook.html"]:
        if (dist / page).exists():
            shutil.copy(dist / page, out / page)
        else:
            print(f"note: web/dist/{page} is missing (web/build.sh, web/handbook.py)")
    # the handbook's pictures, animations and sounds, beside it
    if (dist / "handbook-media").exists():
        shutil.copytree(dist / "handbook-media", out / "handbook-media", dirs_exist_ok=True)
    size = sum(p.stat().st_size for p in out.rglob("*") if p.is_file())
    print(f"_site/: {sum(1 for p in out.rglob('*') if p.is_file())} files, {size / 1e6:.1f} MB")


if __name__ == "__main__":
    main()
