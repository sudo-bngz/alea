# alea on the M-VAVE FM-1

Patches written in alea, played by the FM-1: a `synth { … }` compiled to a
small bytecode, and an engine for the [Felucca](https://github.com/hugelton/Felucca)
firmware that plays it, one voice per note, in fixed point.

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
own; a bank holds sixteen. Send the `.syx` to the FM-1 (any SysEx sender over
USB-MIDI), pick the ALEA engine, and EDIT 1 > PATCH chooses one. Felucca's
own envelope is not applied (the patch has its envelopes); its LFO → AMP, the
sends (DIST, CHORUS, DELAY, REVERB) and the sequencer are.

## What is here

| File | |
|---|---|
| `alea_vm.h` | the interpreter (MIT): loads a patch, plays a voice; no float, no division per sample; `ALEA_FS` is the host's rate |
| `alea_vm_tables.h` | its tables, made by `gen_tables.py` for 44100 Hz (`python3 fm1/gen_tables.py 48000 > …` for another rate) |
| `eng_alea.c` | the Felucca engine (GPL-3.0): the bank in flash, the voices, their pool, EDIT 1 > PATCH |
| `FORMAT.md` | the bytecode, the bank and the SysEx |
| `test/vm_main.c` | plays a patch on a computer: `cargo test` builds it and checks the interpreter against alea's own voice, sample by sample for a sine, by loudness along the note for a bass, a bell with echoes and a hat |
| `test/host_main.c` | a mock of Felucca's host: `cargo test` builds `eng_alea.c` against `felucca_engine.h` (a copy of Felucca-Engines' header, MIT) and plays a bank |

## Into the firmware

In a fork of Felucca:

1. Copy `alea_vm.h`, `alea_vm_tables.h` and `eng_alea.c` into `firmware/src/`.
2. `engines.c`: include `eng_alea.c` with the other engines and add `&ENG_ALEA` to `ENGINES[]` (and 1 to `NENGINES`).
3. The bank in flash: a sector like the FM6 voice bank's (`storage.c`, `fm6_vbank.c`): `alea_bank_bytes()` and `alea_bank_size()` return it, `alea_bank_boot()` runs at boot and after a write.
4. The SysEx: in the MIDI input, a message starting `F0 7D 41 4C 45 41 01` is a bank: `alea_sysex_unpack` the body into a buffer, write it to the sector, `alea_bank_boot()`.
5. `ALEA_POOL_WORDS` (12 K int16 a voice by default, 192 KB for eight voices in `.pool`): lower it if the pool is short; a patch's delay longer than it plays dry, and the compiler warns.
6. Build (`./build.sh`), test in the browser build first (the engine has no float, so it compiles for WebAssembly as it is), then flash with the installer. Have the recovery dongle at hand.

The compiler side lives in alea (`src/fm1.rs`, `alea --fm1`); the engine side belongs in the fork.
