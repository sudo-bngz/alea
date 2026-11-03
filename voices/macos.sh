#!/bin/sh
# alea voice engine: the voices of macOS (`say`).
#   say("Hello.", engine: "voices/macos.sh", voice: "Samantha")
# `say -v ?` lists the voices; the better ones are downloaded in System
# Settings > Accessibility > Spoken Content. ALEA_SPEED scales the rate.
set -e
text=$(cat)
case "$ALEA_LANG" in fr) default=Thomas ;; *) default=Samantha ;; esac
rate=$(python3 -c "import os;print(int(175*float(os.environ.get('ALEA_SPEED','1'))))")
tmp=$(mktemp -t alea-voice).aiff
say -v "${ALEA_VOICE:-$default}" -r "$rate" -o "$tmp" "$text"
afconvert -f WAVE -d LEI16 "$tmp" -o - 2>/dev/null || ffmpeg -v error -i "$tmp" -f wav -
rm -f "$tmp"
