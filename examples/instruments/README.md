# Instruments

alea's built-in instruments, written in alea as synths you can open, read,
copy and change. Each file is a work whose `output` is a synth; it takes a
few parameters, and plays a short demo when run on its own
(`alea instruments/bell.alea`).

```alea
use "instruments/marimba.alea"
use "instruments/strings.alea"

work Duo {
  pattern melody = repeat 16 event(t: i / n, y: ~between(0.2, 0.8), size: 0.04)
  render sound(notes(melody, instrument: Marimba(ring: 1.5)),
               drone(clouds(), voices: 3, instrument: Strings(vibrato: 0.006)))
}
```

| File | Plays | Parameters |
|---|---|---|
| `marimba.alea` | notes | `ring` (1), `mallet` (0.3) |
| `bell.alea` | notes | `ring` (1) |
| `kalimba.alea` | notes | `ring` (1) |
| `glass.alea` | notes | `ring` (1) |
| `wood.alea` | notes | `ring` (1) |
| `pluck.alea` | notes | `ring` (1), `bite` (0.4) |
| `drum.alea` | notes, two octaves down | `ring` (1), `punch` (1.5) |
| `tick.alea` | notes (hats) | `tone` (500 Hz), `length` (0.04 s) |
| `organ.alea` | drones | `brightness` (1) |
| `strings.alea` | drones | `vibrato` (0.004), `spread` (0.004) |
| `reed.alea` | drones | |
| `choir.alea` | drones | `vibrato` (0.003) |
| `wind.alea` | drones | `narrow` (0.95) |

They are built the way the built-in instruments are (the same partials,
the same decays), so they sound close to them, not identical: the built-in
ones are hand-written in Rust (`src/sound.rs`), and `"pluck"` uses a delay
line, which a synth can't tune to each note. Synths are explained in the
guide's chapter 27 and in the reference (*Synths*).
