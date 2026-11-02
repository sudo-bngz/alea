# Voices from outside

`say(text)` speaks with alea's own voice, a formant synthesizer written in
Rust: no recording, no model, drawable by chance, and it sings. For a voice
that sounds like a person, `engine:` hands the text to a program:

```alea
let v = say("Hello. I am alea.", engine: "voices/kokoro.py", voice: "af_heart")
```

An engine is any program that reads the text on its standard input and
writes a WAV on its standard output. It gets `ALEA_LANG` (`en`, `fr`),
`ALEA_VOICE` (the `voice:` given), `ALEA_SPEED`, `ALEA_PITCH` (Hz) and
`ALEA_SIZE` in its environment, to use or ignore. The path is resolved next
to the work, then on the PATH. What an engine says is kept in
`~/.cache/alea/voices/` (or `$ALEA_VOICES`), so a text is said once and the
same edition renders the same; delete the directory to say it all again.
`v.words`, `v.syllables` and the rest are estimated from alea's own timing
of the text, scaled onto the engine's sound. `melody:` (singing) is alea's
own voice only; in the browser, so is everything.

| Engine | Needs | Voices |
|---|---|---|
| `kokoro.py` | `pip install sherpa-onnx numpy`; the model (350 MB) is fetched once from GitHub; `espeak-ng` for names it doesn't know | `af_heart`, `af_bella`, `af_sky`, `am_adam`, `am_michael`, `bf_emma`, `bm_george`, `ff_siwis` (French)… 54 in all, English, French, Chinese, Japanese, Italian, Spanish, Portuguese, Hindi |
| `piper.sh` | `pip install piper-tts`, `ffmpeg` | a piper voice name (`en_US-lessac-medium`, `fr_FR-siwis-medium`…), fetched on first use |
| `openai.sh` | `OPENAI_API_KEY`, `curl`, `ffmpeg` | `nova`, `alloy`, `onyx`, `fable`…; `OPENAI_TTS_INSTRUCTIONS` says how to read |
| `elevenlabs.sh` | `ELEVENLABS_API_KEY`, `curl`, `ffmpeg` | a voice id from your library |
| `macos.sh` | macOS | `say -v ?` |
| `espeak.sh` | `espeak-ng` | its voices (`en-us`, `fr`, `en-gb-x-rp`…) |

Copy any of them to write your own.
