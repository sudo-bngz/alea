#!/bin/sh
# S08 · Listening: the music is not the program's, it is a track by JL
# (Momo's Basement). So the episode's sound is made in three steps:
#   1. the music: the first 108.5 s of the track, fading out over the end card
#   2. the voice: s08-listening-track.alea, which says the lines and nothing else
#   3. the mix: the music ducked under the voice (a sidechain), then both
# The Reel opens the mix for its sound and the music alone to listen to.
#
# usage: ./s08-mix.sh path/to/track.wav [alea binary]
# The WAVs stay out of the repository (exhibition/sound/*.wav).
set -eu
src=$1
alea=${2:-../../target/release/alea}
total=108.522   # 52 bars at 115 BPM
fade=100.174    # bar 48: the end card

ffmpeg -v error -y -i "$src" -t $total -af "afade=t=out:st=$fade:d=$(echo "$total - $fade" | bc)" \
  -ar 44100 -ac 2 s08-listening-music.wav

tmp=$(mktemp -d)
"$alea" s08-listening-track.alea --seed 0xa1ea01 -o "$tmp" >/dev/null
voice=$(ls "$tmp"/*.wav)

ffmpeg -v error -y -i s08-listening-music.wav -i "$voice" -filter_complex \
  "[1:a]aresample=44100,apad,asplit=2[sc][v];\
   [0:a][sc]sidechaincompress=threshold=0.015:ratio=8:attack=20:release=450:makeup=1[m];\
   [v]volume=1.0[vv];\
   [m][vv]amix=inputs=2:normalize=0:duration=first,alimiter=limit=0.89" \
  -ar 44100 s08-listening.wav
rm -rf "$tmp"
echo "s08-listening-music.wav and s08-listening.wav written"
