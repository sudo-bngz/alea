#!/usr/bin/env python3
"""Make the inputs of brut.alea from a phone clip.

    python3 make-inputs.py IMG_4094.mov

Writes, next to this script:
  sheets/hi0.jpg … hi6.jpg   126 frames of the clip, 18 per sheet (6 × 3), 640 × 1138 each
  sheets/lo.jpg              the same 126 frames, 16 × 8, 256 × 455 each
  chop.wav                   the clip's sound, cut on a 128 bpm grid and rebuilt (see chop.py)

alea looks one frame up in a sheet with a single `picture`, whatever the
number of frames; a `sequence` of 126 pictures costs 126 lookups a pixel.
"""
import os, subprocess, sys, tempfile
from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
N = 126
HI, LO = (640, 1138), (256, 455)

def main(clip):
    dur = float(subprocess.check_output(["ffprobe", "-v", "error", "-show_entries", "format=duration",
                                         "-of", "csv=p=0", clip]).strip())
    out = os.path.join(HERE, "sheets"); os.makedirs(out, exist_ok=True)
    with tempfile.TemporaryDirectory() as tmp:
        subprocess.check_call(["ffmpeg", "-v", "error", "-y", "-i", clip, "-vf", f"fps={N / dur:.6f}",
                               "-frames:v", str(N), "-q:v", "2", os.path.join(tmp, "f%03d.jpg")])
        frames = sorted(os.listdir(tmp))
        while len(frames) < N: frames.append(frames[-1])
        ims = [Image.open(os.path.join(tmp, f)).convert("RGB") for f in frames[:N]]
        for s in range(7):
            sheet = Image.new("RGB", (HI[0] * 6, HI[1] * 3))
            for j in range(18):
                sheet.paste(ims[s * 18 + j].resize(HI, Image.LANCZOS), ((j % 6) * HI[0], (j // 6) * HI[1]))
            sheet.save(os.path.join(out, f"hi{s}.jpg"), quality=90)
        lo = Image.new("RGB", (LO[0] * 16, LO[1] * 8))
        for k, im in enumerate(ims):
            lo.paste(im.resize(LO, Image.LANCZOS), ((k % 16) * LO[0], (k // 16) * LO[1]))
        lo.save(os.path.join(out, "lo.jpg"), quality=90)
    wav = os.path.join(HERE, "clip.wav")
    subprocess.check_call(["ffmpeg", "-v", "error", "-y", "-i", clip, "-vn", "-ac", "1", "-ar", "44100", wav])
    subprocess.check_call([sys.executable, os.path.join(HERE, "chop.py"), wav, os.path.join(HERE, "chop.wav")])
    os.remove(wav)

if __name__ == "__main__":
    main(sys.argv[1])
