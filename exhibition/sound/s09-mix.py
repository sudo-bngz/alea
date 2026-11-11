#!/usr/bin/env python3
"""S09 · Editions: the episode's sound is three renders of one program.

  1. the editions: s09-editions-program.alea with --seed 1, 2, 3, each
     brought to the same loudness (-16 LUFS)
  2. the voice: s09-editions-track.alea, which says the lines and nothing else
  3. the music: the editions in turn, as `editions.plays` in s09-editions.json
     lists them ([from bar, to bar, edition, from second in it]), with short
     fades, ducked under the voice, then both mixed into s09-editions.wav

usage: ./s09-mix.py [alea binary]      (run in exhibition/sound)
The WAVs stay out of the repository (exhibition/sound/*.wav)."""
import json
import pathlib
import subprocess
import sys
import tempfile

alea = sys.argv[1] if len(sys.argv) > 1 else "../../target/release/alea"
ep = json.load(open("s09-editions.json"))
bar = 240 / ep["tempo"]
total = ep["bars"] * bar
ed = ep["editions"]


def run(*cmd):
    subprocess.run(cmd, check=True)


with tempfile.TemporaryDirectory() as tmp:
    tmp = pathlib.Path(tmp)
    eds = []
    for k, seed in enumerate(ed["seeds"]):
        out = tmp / f"r{k}"
        out.mkdir()
        run(alea, "s09-editions-program.alea", "--seed", str(seed), "-o", str(out))
        wav = next(out.glob("*.wav"))
        level = f"s09-edition-{seed}.wav"
        run("ffmpeg", "-v", "error", "-y", "-i", str(wav), "-af", "loudnorm=I=-16:TP=-2:LRA=11", "-ar", "44100", level)
        eds.append(level)
    vo = tmp / "voice"
    vo.mkdir()
    run(alea, "s09-editions-track.alea", "--seed", "0xa1ea01", "-o", str(vo))
    voice = next(vo.glob("*.wav"))

    # the music: each play cut from its edition, faded in and out, put in place
    inputs, parts = [], []
    for j, (a, b, k, off) in enumerate(ed["plays"]):
        start, length = a * bar, (b - a) * bar
        last = j == len(ed["plays"]) - 1
        fade_out = f"afade=t=out:st={length - (4 if last else 0.06):.3f}:d={4 if last else 0.06}"
        inputs += ["-i", eds[k]]
        parts.append(f"[{j}:a]atrim=start={off}:duration={length:.3f},asetpts=PTS-STARTPTS,"
                     f"afade=t=in:d=0.06,{fade_out},adelay={int(start * 1000)}:all=1[p{j}]")
    n = len(ed["plays"])
    graph = ";".join(parts) + ";" + "".join(f"[p{j}]" for j in range(n)) + \
        f"amix=inputs={n}:normalize=0:duration=longest,apad,atrim=duration={total:.3f}[mus]"
    music = tmp / "music.wav"
    run("ffmpeg", "-v", "error", "-y", *inputs, "-filter_complex", graph, "-map", "[mus]", "-ar", "44100", str(music))

    # the voice over it, the music ducked under the voice
    run("ffmpeg", "-v", "error", "-y", "-i", str(music), "-i", str(voice), "-filter_complex",
        "[1:a]aresample=44100,apad,asplit=2[sc][v];"
        "[0:a][sc]sidechaincompress=threshold=0.015:ratio=8:attack=20:release=450[m];"
        "[m][v]amix=inputs=2:normalize=0:duration=first,alimiter=limit=0.89",
        "-ar", "44100", "s09-editions.wav")
print("s09-editions.wav written, with s09-edition-1/2/3.wav")
