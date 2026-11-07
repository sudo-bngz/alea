# alea, one sound at a time

The sound series: ten episodes of 2 to 3 minutes. Each one builds a single
track from silence, one layer per beat, then lets it play on its own for the
last 40 seconds while the picture listens to it.

Each episode has two passes, so the picture follows the real track exactly:

```sh
python3 gen.py s01-beat.json                      # writes the three programs below
alea s01-beat-track.alea --seed 0xa1ea01          # the music and the voice → a WAV
cp <the WAV> s01-beat.wav                          # (not in the repository)
alea s01-beat.alea --seed 0xa1ea01                # the Reel: plays s01-beat.wav, listens to it
alea s01-beat-program.alea                        # the program on screen, whole, on its own
```

- `gen.py` is the template. The look, the opening, the code panel and the
  captions come from `../series/gen.py`. It adds the step grid (a row per
  layer, lit on the meter's clock), the drop (the whole beat on the panel,
  with meters reading the track's `.bass`, `.mid` and `.treble`), and the
  track's machinery: each layer comes in on its bar, and every event plays
  at half intensity while a line is spoken.
- `NN-name.json` is an episode: tempo, bars, key, the layers (synth, hits,
  bar of entry, gain, swing), the beats (label, spoken line, code) and the
  drop.
- `NN-name-program.alea` is the program the episode teaches, whole and
  runnable. One difference from the track: `steps` gives each hit its moment
  as its `x`, so on its own the program's drums sweep from left to right
  over the work; the track keeps them in the middle.

Run from this directory (paths resolve from where alea runs). Master the
Reel to about −14 LUFS when encoding (`loudnorm=I=-14:TP=-1`).

| # | Episode | Feature | Length |
|---|---|---|---|
| S01 | The beat | `meter`, `steps("X...x...")`, a synth per drum, chance on the hats, `swing` | 2:00 |
| S02 | Euclid | `euclid(hits, steps, rotate)`, five layers on one wheel, rotation as a second voice | 2:06 |
| S03 | A synth | `synth { … }` built unit by unit: `sine`, `saw`, `adsr`, `lowpass` opened by the envelope, `drive`, `chorus` | 2:32 |
