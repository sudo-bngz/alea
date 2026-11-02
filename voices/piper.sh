#!/bin/sh
# alea voice engine: piper, a small local neural voice (pip install piper-tts).
#   say("Hello.", engine: "voices/piper.sh", voice: "en_US-lessac-medium")
# The voice is a piper model name (downloaded by piper on first use into
# $PIPER_DIR, default ~/.cache/alea/piper) or the path of a .onnx file.
set -e
dir="${PIPER_DIR:-$HOME/.cache/alea/piper}"
mkdir -p "$dir"
case "$ALEA_LANG" in fr) default=fr_FR-siwis-medium ;; *) default=en_US-lessac-medium ;; esac
voice="${ALEA_VOICE:-$default}"
case "$voice" in *.onnx) model="$voice" ;; *) model="$dir/$voice.onnx"; [ -f "$model" ] || python3 -m piper.download_voices --data-dir "$dir" "$voice" >&2 ;; esac
rate=$(python3 -c "import os;print(1/float(os.environ.get('ALEA_SPEED','1')))")
exec piper --model "$model" --length-scale "$rate" --output-raw 2>/dev/null | ffmpeg -v error -f s16le -ar 22050 -ac 1 -i - -f wav -
