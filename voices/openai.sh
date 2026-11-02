#!/bin/sh
# alea voice engine: OpenAI's text to speech (needs $OPENAI_API_KEY, curl and ffmpeg).
#   say("Hello.", engine: "voices/openai.sh", voice: "nova")
# Voices: alloy, ash, coral, echo, fable, nova, onyx, sage, shimmer. The model
# is $OPENAI_TTS_MODEL (default gpt-4o-mini-tts); $OPENAI_TTS_INSTRUCTIONS can
# say how to read ("calm, a little amused").
set -e
[ -n "$OPENAI_API_KEY" ] || { echo "openai: set OPENAI_API_KEY" >&2; exit 1; }
text=$(cat)
voice="${ALEA_VOICE:-nova}"
model="${OPENAI_TTS_MODEL:-gpt-4o-mini-tts}"
body=$(python3 -c 'import json,os,sys; d={"model":sys.argv[1],"input":sys.argv[2],"voice":sys.argv[3],"response_format":"wav","speed":float(os.environ.get("ALEA_SPEED","1"))}
i=os.environ.get("OPENAI_TTS_INSTRUCTIONS")
if i: d["instructions"]=i
print(json.dumps(d))' "$model" "$text" "$voice")
curl -sS --fail https://api.openai.com/v1/audio/speech -H "Authorization: Bearer $OPENAI_API_KEY" -H "Content-Type: application/json" -d "$body" | ffmpeg -v error -i - -f wav -
