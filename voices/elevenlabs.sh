#!/bin/sh
# alea voice engine: ElevenLabs (needs $ELEVENLABS_API_KEY, curl and ffmpeg).
#   say("Hello.", engine: "voices/elevenlabs.sh", voice: "JBFqnCBsd6RMkjVDRZzb")
# The voice is a voice id from your ElevenLabs library; the model is
# $ELEVENLABS_MODEL (default eleven_multilingual_v2).
set -e
[ -n "$ELEVENLABS_API_KEY" ] || { echo "elevenlabs: set ELEVENLABS_API_KEY" >&2; exit 1; }
text=$(cat)
voice="${ALEA_VOICE:-JBFqnCBsd6RMkjVDRZzb}"
model="${ELEVENLABS_MODEL:-eleven_multilingual_v2}"
body=$(python3 -c 'import json,sys; print(json.dumps({"text":sys.argv[1],"model_id":sys.argv[2]}))' "$text" "$model")
curl -sS --fail "https://api.elevenlabs.io/v1/text-to-speech/$voice?output_format=mp3_44100_128" -H "xi-api-key: $ELEVENLABS_API_KEY" -H "Content-Type: application/json" -d "$body" | ffmpeg -v error -i - -f wav -
