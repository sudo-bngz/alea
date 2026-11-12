# The presentation of alea

Videos that show the language, each made by an alea program: the code on
screen is the code that runs, the voice says what it does, and the picture
is what it draws.

| | Folder | What it is |
|---|---|---|
| **alea speaks** | `intro.alea` (+ `intro/`) | The introduction, about a minute and a quarter: what alea is and what it does, one feature to a cut on the beat, with a voice and its words typed as it speaks. |
| **alea, one feature at a time** | `series/` | Twelve short episodes on the language, one idea each: the tilde, fields, shapes, transforms, time, filters, pictures, patterns, growing, text, sculpture, components. See `series/README.md`. |
| **alea, one sound at a time** | `sound/` | The sound series: a track built up one layer at a time, then the picture listening to it: the beat, Euclid, a synth, notes, harmony, drones, voice, listening, editions. See `sound/README.md`. |

The voice is spoken through `voices/kokoro-misaki.py` (`pip install
kokoro-onnx misaki num2words phonemizer`, then `python3 -m spacy download
en_core_web_sm`). Each episode renders with one command, from its folder:

```sh
alea intro.alea --seed 0xa1ea01
cd series && alea 01-tilde.alea --seed 0xa1ea01
cd sound && alea s01-beat.alea --seed 0xa1ea01   # after its track: alea s01-beat-track.alea
```

The renders (videos, sounds, pictures) stay out of the repository.
