# alea, one feature at a time

The series after *alea speaks*: an episode a week, 45–60 s, one feature,
one program that fits on one screen by the end. The documentation, told one
page at a time, in the theme of alea.bngz.io.

- `gen.py` is the template: everything the episodes share. The opening (the
  site's wave, the handle, a cursor), the red label top left, the captions
  typed letter by letter as the voice says them, the code typed a line to an
  eighth note with its `~` in red, the cuts, the end card, the voice (Kore
  through `../../voices/kokoro-misaki.py`, speed 1.0; paths resolve from where alea runs, so run from this directory) and the score (a pad on
  tape, dust, a beat, a stutter into every cut, a boom on the sections, a
  glass hit on every `hit`).
- `NN-name.json` is an episode: its sections (start, label, spoken line and
  when, the code that types, the stage: alea `let` lines ending in the field
  drawn in that window), `overlays` (paintings composed over the page through
  a mask, for results in their own colours), the transitions, and the score's
  moments (`lets` and `layers` add an episode's own instruments).
- `NN-name.alea` is generated: `python3 gen.py 01-tilde.json > 01-tilde.alea`.

```sh
alea 01-tilde.alea --check
alea 01-tilde.alea --seed 0xa1ea01        # the cover and the 1080×1920 Reel
```

For a preview, lower the render in a copy (`width: 270, height: 480`,
`fps: 10`): half a minute instead of an hour.

| # | Episode | Feature | Length |
|---|---|---|---|
| 01 | The tilde | `~`: the same program, a different picture every run; the seed | 55 s |
| 02 | Fields | a picture is a function of x and y: `paint(x)`, `paint(y)`, a sine, x and y mixed, a palette | 58 s |
