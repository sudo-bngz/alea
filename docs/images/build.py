#!/usr/bin/env python3
"""Makes the README's pictures, from the repository's own works:

    cargo build --release && python3 docs/images/build.py

Needs Pillow and ffmpeg. Writes, next to this file:
  banner.jpg               one edition of each of eight examples, side by side
  rosace-editions.jpg      examples/rosace.alea, four editions with their seeds
  isohypses-editions.jpg   examples/isohypses.alea, six editions with their seeds
  breathing-0x3.webp, breathing-0x7.webp   examples/breathing.alea, two looping editions
"""
import glob, io, pathlib, re, subprocess, tempfile

from PIL import Image, ImageDraw, ImageFont

here = pathlib.Path(__file__).resolve().parent
root = here.parent.parent
alea = root / "target" / "release" / "alea"
mono = ImageFont.truetype(str(root / "fonts" / "mono.ttf"), 22)
INK, PAPER, MUTED = (24, 27, 48), (237, 239, 243), (86, 91, 122)


def render(work, seed, out):
    subprocess.run([str(alea), str(root / work), "--seed", seed, "-o", str(out)], check=True,
                   stdout=subprocess.DEVNULL, stdin=subprocess.DEVNULL)


def still(work, seed):
    """The work's image only: a copy of it without its other renders (a video,
    a sculpture, a sound), which would take minutes and aren't shown here."""
    src = (root / work).read_text()
    out, i = [], 0
    for m in re.finditer(r"\brender\s+(\w+)\s*\(", src):
        if m.group(1) == "image" or m.start() < i:
            continue
        depth, j = 0, m.end() - 1
        while True:
            depth += {"(": 1, ")": -1}.get(src[j], 0)
            j += 1
            if depth == 0:
                break
        out.append(src[i:m.start()])
        i = j
    out.append(src[i:])
    copy = root / work.replace(".alea", ".still.alea")
    copy.write_text("".join(out))
    try:
        png = subprocess.run([str(alea), str(copy), "--seed", seed, "-o", "-"], check=True,
                             capture_output=True, stdin=subprocess.DEVNULL).stdout
    finally:
        copy.unlink()
    return Image.open(io.BytesIO(png))


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

    # the banner: one edition of each of eight examples, cropped to the same frame
    picks = [("isohypses", "0xa1ea03"), ("rosace", "0x5"), ("moon", "0x3"), ("eclipse", "0x2"),
             ("constellation", "0x4"), ("bauhaus", "0x6"), ("tide", "0x1"), ("phosphor", "0x7")]
    tiles = []
    for work, seed in picks:
        im = still(f"examples/{work}.alea", seed).convert("RGB")
        w, h = im.size
        side = min(w, h * 240 // 300)
        im = im.crop(((w - side) // 2, (h - side * 300 // 240) // 2, (w + side) // 2, (h + side * 300 // 240) // 2))
        tiles.append(im.resize((240, 300), Image.LANCZOS))
    gap = 8
    banner = Image.new("RGB", (len(tiles) * 240 + (len(tiles) - 1) * gap, 300), INK)
    for i, t in enumerate(tiles):
        banner.paste(t, (i * (240 + gap), 0))
    banner.save(here / "banner.jpg", quality=86, optimize=True, progressive=True)

    # Rosace: the same 13 lines, four editions
    seeds = ["0x6", "0x5", "0xb", "0x2"]
    ims = [still("examples/rosace.alea", s) for s in seeds]
    labelled_row(ims, [f"--seed {s}" for s in seeds], 300).save(here / "rosace-editions.jpg", quality=86, optimize=True)

    # Isohypses: six editions
    seeds = ["0xa1ea03", "0xa1ea0a", "0xa1ea01", "0xa1ea06", "0xa1ea0d", "0xa1ea07"]
    ims = [still("examples/isohypses.alea", s) for s in seeds]
    labelled_row(ims, seeds, 220).save(here / "isohypses-editions.jpg", quality=84, optimize=True)

    # Breathing: two editions of a looping video
    for s in ["0x3", "0x7"]:
        render("examples/breathing.alea", s, tmp / "breathing")
        mp4 = next((tmp / "breathing").glob(f"breathing-{int(s, 16):016x}.mp4"))
        frames = tmp / f"frames-{s}"
        frames.mkdir()
        subprocess.run(["ffmpeg", "-v", "error", "-i", str(mp4), "-vf", "fps=15,scale=300:-1:flags=lanczos",
                        str(frames / "%03d.png")], check=True)
        anim(str(frames / "*.png"), here / f"breathing-{s}.webp", 67)

    for f in sorted(here.glob("*.*")):
        if f.suffix != ".py":
            print(f"{f.name:28} {f.stat().st_size / 1e3:7.0f} kB")


if __name__ == "__main__":
    main()
