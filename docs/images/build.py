#!/usr/bin/env python3
"""Makes the README's pictures, from the repository's own works:

    cargo build --release && python3 docs/images/build.py

Needs Pillow and ffmpeg. Writes, next to this file:
  banner.jpg               editions from the exhibition, side by side
  rosace-editions.jpg      examples/rosace.alea, four editions with their seeds
  isohypses-editions.jpg   the six posted editions of Isohypses, with their seeds
  breathing-0x3.webp, breathing-0x7.webp   examples/breathing.alea, two looping editions
  culture.webp             the Culture Reel: colonies growing in a petri dish
"""
import glob, pathlib, subprocess, tempfile

from PIL import Image, ImageDraw, ImageFont

here = pathlib.Path(__file__).resolve().parent
root = here.parent.parent
alea = root / "target" / "release" / "alea"
posts = root / "exhibition" / "instagram"
mono = ImageFont.truetype(str(root / "fonts" / "mono.ttf"), 22)
INK, PAPER, MUTED = (24, 27, 48), (237, 239, 243), (86, 91, 122)


def render(work, seed, out):
    subprocess.run([str(alea), str(root / work), "--seed", seed, "-o", str(out)], check=True,
                   stdout=subprocess.DEVNULL, stdin=subprocess.DEVNULL)


def labelled_row(images, labels, tile_w, gap=14, pad=18, label_h=40):
    """Tiles side by side on paper, each with its seed underneath."""
    tiles = [im.convert("RGB").resize((tile_w, round(im.height * tile_w / im.width)), Image.LANCZOS) for im in images]
    h = max(t.height for t in tiles)
    out = Image.new("RGB", (pad * 2 + len(tiles) * tile_w + (len(tiles) - 1) * gap, pad + h + label_h), PAPER)
    d = ImageDraw.Draw(out)
    for i, (t, label) in enumerate(zip(tiles, labels)):
        x = pad + i * (tile_w + gap)
        out.paste(t, (x, pad))
        d.text((x, pad + h + 8), label, fill=MUTED, font=mono)
    return out


def anim(frames_glob, dest, ms):
    frames = [Image.open(p).convert("RGB") for p in sorted(glob.glob(frames_glob))]
    frames[0].save(dest, save_all=True, append_images=frames[1:], duration=ms, loop=0, quality=70, method=6)


def main():
    tmp = pathlib.Path(tempfile.mkdtemp())

    # the banner: one edition of each work of the exhibition
    picks = ["1-isohypses-1-0xa1ea03", "3-rosace-2-0xa1ea05", "2-nocturne-3-0xa1ea09",
             "7-correspondances-2-0xa1ea01", "9-culture-3-0xc06", "1-isohypses-3-0xa1ea01",
             "3-rosace-6-0xa1ea0f"]
    tiles = [Image.open(posts / f"{p}.jpg").convert("RGB").resize((240, 300), Image.LANCZOS) for p in picks]
    gap = 8
    banner = Image.new("RGB", (len(tiles) * 240 + (len(tiles) - 1) * gap, 300), INK)
    for i, t in enumerate(tiles):
        banner.paste(t, (i * (240 + gap), 0))
    banner.save(here / "banner.jpg", quality=86, optimize=True, progressive=True)

    # Rosace: the same 13 lines, four editions
    seeds = ["0x6", "0x5", "0xb", "0x2"]
    for s in seeds:
        render("examples/rosace.alea", s, tmp / "rosace")
    ims = [Image.open(next((tmp / "rosace").glob(f"rosace-{int(s, 16):016x}.png"))) for s in seeds]
    labelled_row(ims, [f"--seed {s}" for s in seeds], 300).save(here / "rosace-editions.jpg", quality=86, optimize=True)

    # Isohypses: the six posted editions
    posted = sorted(posts.glob("1-isohypses-*-0x*.jpg"), key=lambda p: int(p.name.split("-")[2]))
    labelled_row([Image.open(p) for p in posted], [p.stem.split("-")[-1] for p in posted], 220).save(
        here / "isohypses-editions.jpg", quality=84, optimize=True)

    # Breathing: two editions of a looping video
    for s in ["0x3", "0x7"]:
        render("examples/breathing.alea", s, tmp / "breathing")
        mp4 = next((tmp / "breathing").glob(f"breathing-{int(s, 16):016x}.mp4"))
        frames = tmp / f"frames-{s}"
        frames.mkdir()
        subprocess.run(["ffmpeg", "-v", "error", "-i", str(mp4), "-vf", "fps=15,scale=300:-1:flags=lanczos",
                        str(frames / "%03d.png")], check=True)
        anim(str(frames / "*.png"), here / f"breathing-{s}.webp", 67)

    # Culture: the Reel, cropped to the dish
    frames = tmp / "culture"
    frames.mkdir()
    subprocess.run(["ffmpeg", "-v", "error", "-i", str(posts / "10-culture-reel.mp4"), "-vf",
                    "fps=10,crop=1080:1080:0:420,scale=300:300:flags=lanczos", str(frames / "%03d.png")], check=True)
    anim(str(frames / "*.png"), here / "culture.webp", 100)

    for f in sorted(here.glob("*.*")):
        if f.suffix != ".py":
            print(f"{f.name:28} {f.stat().st_size / 1e3:7.0f} kB")


if __name__ == "__main__":
    main()
