# The FM-1 patch format

`alea work.alea --fm1 bank.syx` compiles the work's output synth into a
*patch*, and a *bank* of patches into a SysEx file for the FM-1's ALEA engine.
All numbers are little-endian; Q16 means a signed 32-bit number with sixteen
fractional bits (1.0 = 65536): the same unit for a sample, an envelope and a
frequency in Hz.

## A patch (`ALP1`)

| Bytes | |
|---|---|
| `ALP1` | magic |
| u16 | sample rate the patch was compiled for (44100) |
| u16 | ops |
| u16 | states |
| u32 | int16 samples of delay and chorus lines one voice needs (the states' lengths added up) |
| … | the ops, in order; op *i* writes slot *i* and reads slots before it; the last op is the output |
| … | the states: for each, u8 kind (0 phase, 1 envelope, 2 filter, 3 delay line, 4 chorus) and u32 length (samples, for lines; a chorus's is 1105, its 25 ms of lateness and one sample more) |

| Op | Code | Operands |
|---|---|---|
| CONST | 0 | i32 value (Q16) |
| NOTE | 1 | u8: 0 freq (Hz), 1 velocity (0–1), 2 gate, 3 time (seconds), 4 level |
| OSC | 2 | u8 wave (0 sine, 1 saw, 2 square, 3 triangle, 4 pulse), u16 freq slot, u16 width slot, u16 state |
| NOISE | 3 | |
| ADSR | 4 | i32 attack, decay, sustain, release (Q16), u16 state |
| PERC | 5 | i32 attack, release |
| FILTER | 6 | u8 pass (0 low, 1 high, 2 band), u16 input, u16 cutoff (Hz), u16 resonance (0–1), u16 state |
| DELAY | 7 | u16 input, u32 length (samples), i32 feedback, i32 mix, u16 state |
| DRIVE | 8 | u16 input, u16 amount |
| CHORUS | 9 | u16 input, i32 depth, i32 rate (Hz), i32 mix, u16 state |
| ADD SUB MUL DIV | 10–13 | u16 a, u16 b |
| NEG | 14 | u16 a |

The interpreter (`alea_vm.h`) plays exactly what alea's own voice plays (`src/synth.rs`):
polyBLEP saw, square and pulse, a TPT state-variable filter with its
coefficients refreshed every 32 samples, envelopes on the note's clock, a
15 Hz DC blocker, the output clamped to ±8. A patch may have 64 ops and 32
states; on the FM-1 a voice keeps at most 128 words of state (one per op, and
1 per oscillator or chorus, 3 per envelope, 8 per filter), which `alea --fm1`
checks. Delay and chorus lines are Q13 (±4) in a pool the host gives each voice;
a line the pool can't hold plays dry. On the FM-1 a part has 2860 samples of
lines for its voices, and the first part that needs more borrows 8192 more:
`alea --fm1` says how many voices keep their lines.

## A bank (`ALB1`)

| Bytes | |
|---|---|
| `ALB1` | magic |
| u8 | patches (1–16) |
| for each: 16 bytes name (ASCII, zero-padded), u32 size, the patch | |

A bank is at most 3584 bytes: what the FM-1 keeps in its flash sector.

## The SysEx (`.syx`)

The requests of Felucca's editor protocol that store a bank (`F0 7D 46 4C cmd … F7`;
the protocol is the fork's `web/EDITOR_PROTOCOL.md`, "ALEA bank"), one after the other:

| cmd | Args | Reply |
|---|---|---|
| 96 BEGIN | size (3 × 7 bits, low first); 0 erases the stored bank | rc |
| 97 WRITE | offset (3 × 7 bits), then up to 256 of the bank's bytes packed 7 to 8: each group of up to seven bytes after a byte of their high bits (bit *k* for byte *k*) | offset, rc |
| 98 END | CRC-32 (zlib's) of the bank (5 × 7 bits) | rc, patches |
| 99 LIST | — | stored (1 sent over USB, 0 the factory bank), patches, their names (0-terminated) |

rc 0 is ok; 1 arguments, 2 the CRC, 3 the sequencer is running (or, for END, not a
bank the engine plays), 4 flash, 5 no transfer. Each request waits for the reply
to the one before (the device drops a message that comes while it is still busy
with the last), so a generic SysEx sender needs a pause between messages (about
1 s after BEGIN and END, which erase and write flash). `fm1/send.html` (in the
browser, with Web MIDI; on the site as alea.bngz.io/fm1.html) and `fm1/send.py`
do it properly.
