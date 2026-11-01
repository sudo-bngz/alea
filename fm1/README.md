# alea on the M-VAVE FM-1

Patches written in alea, played by the FM-1: a `synth { … }` compiled to a
small bytecode, and the ALEA engine of a fork of the [Felucca](https://github.com/hugelton/Felucca)
firmware ([sudo-bngz/Felucca](https://github.com/sudo-bngz/Felucca)) that plays it, one voice
per note, in fixed point.

```
work Bass {
  output synth {
    let env = adsr(attack: 0.004, decay: 0.2, sustain: 0.5, release: 0.15)
    drive(lowpass(saw() + pulse(note.freq, width: ~between(0.2, 0.4)) * 0.5 + square(note.freq * 0.5) * 0.6,
                  cutoff: 200 + 2500 * perc(release: 0.25) * note.velocity, resonance: ~between(0.3, 0.6)) * env,
          amount: 0.25)
  }
}
```

```sh
alea bass.alea --seed 7 -n 8 --fm1 bass.syx    # eight editions of the bass: eight patches, one bank
```

Every `~` is drawn when the patch is compiled, so each edition is a patch of its
own; a bank holds sixteen (3584 bytes at most).

## On the device

1. Install the fork's firmware on the FM-1 (its `./build.sh` makes `build/felucca.fwsc`; Felucca's web installer
   installs it, and its **Return to official V15** takes the FM-1 back).
2. Open [alea.bngz.io/fm1.html](https://alea.bngz.io/fm1.html) in Chrome or Edge (or `fm1/send.html` from here),
   connect, choose the `.syx`, send. Or `python3 fm1/send.py bass.syx` (`pip install mido python-rtmidi`).
3. On the FM-1, pick the ALEA engine (PRESETS, the ENG knob: after the other engines) and choose the patch
   with EDIT 1 > PATCH. Without a bank sent, ALEA plays its four factory patches (BASS, PAD, BELL, STAB:
   `examples/fm1/`).

Felucca's own envelope is not applied (the patch has its envelopes); its LFO → AMP,
the sends (DIST, CHORUS, DELAY, REVERB), the arpeggiator and the sequencer are. The
pitch is Felucca's: glide, bend and unison detune reach the patch.

What fits: a voice keeps at most 128 words of state (a filter takes 9 with its op,
an envelope 4); a part has room for 2860 samples of delay and chorus lines, and the
first ALEA part that needs more borrows 8192 more. A chorus takes 1105 samples a
voice: on one part all eight voices keep theirs, on a second part two do and the
others play it dry. A long `delay` won't fit: use Felucca's DELAY send for echoes.
`alea --fm1` warns about both.

## What is here

| File | |
|---|---|
| `alea_vm.h` | the interpreter (MIT): loads a patch, plays a voice; no float; `ALEA_FS` is the host's rate. The fork has a copy in `firmware/src/alea/` |
| `alea_vm_tables.h` | its tables, made by `gen_tables.py` for 44100 Hz (`python3 fm1/gen_tables.py 48000 > …` for another rate) |
| `FORMAT.md` | the bytecode, the bank and the messages that store it |
| `send.html`, `send.py` | send a bank to the FM-1 (Web MIDI; Python with mido), each message after the device's answer |
| `test/vm_main.c` | plays a patch on a computer: `cargo test` builds it and checks the interpreter against alea's own voice, sample by sample for a sine, by loudness along the note for a bass, a bell with echoes and a chorus, and a hat |

The compiler is `src/fm1.rs` (`alea --fm1`). The engine, its bank in flash and the
editor commands are in the fork (`firmware/src/eng_alea.c`, `alea_bank.c`,
`editor_alea.c`), with their tests (`tests/alea_test.c`): the bank written over
simulated flash with power cuts, and the engine's voice checked sample for sample
against `alea_vm.h`.
