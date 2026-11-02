#!/bin/sh
# alea voice engine: espeak-ng, the classic formant voice of free software.
#   say("Hello.", engine: "voices/espeak.sh")
set -e
case "$ALEA_LANG" in fr) default=fr ;; *) default=en-us ;; esac
rate=$(python3 -c "import os;print(int(160*float(os.environ.get('ALEA_SPEED','1'))))")
pitch=$(python3 -c "import os;print(max(0,min(99,int(float(os.environ.get('ALEA_PITCH','110'))/2.4))))")
exec espeak-ng -v "${ALEA_VOICE:-$default}" -s "$rate" -p "$pitch" --stdout
