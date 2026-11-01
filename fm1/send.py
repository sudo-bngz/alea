#!/usr/bin/env python3
"""Send a bank of alea patches to an M-VAVE FM-1 running Felucca with the ALEA engine, over USB-MIDI: each message
after the device's answer to the one before (Felucca's editor protocol, fm1/FORMAT.md). The same as fm1/send.html.

    pip install mido python-rtmidi
    alea bass.alea -n 8 --fm1 bass.syx
    python3 fm1/send.py bass.syx            # or a .alb; --port NAME if the FM-1 isn't found by its name
    python3 fm1/send.py --list              # what the FM-1 holds
    python3 fm1/send.py --factory           # back to its factory bank
"""
import argparse
import sys
import time

HDR, BEGIN, WRITE, END, LIST, CHUNK = (0x7D, 0x46, 0x4C), 96, 97, 98, 99, 256
RC = {1: "refused (size, order or arguments)", 2: "the CRC does not match", 3: "busy: stop the sequencer and try again",
      4: "the flash could not be written", 5: "no transfer open"}


def v7(v, n):
    return [(v >> (7 * k)) & 0x7F for k in range(n)]


def pack7(b):
    out = []
    for i in range(0, len(b), 7):
        g = b[i:i + 7]
        out.append(sum(((c >> 7) & 1) << k for k, c in enumerate(g)))
        out.extend(c & 0x7F for c in g)
    return out


def crc32(b):
    import zlib
    return zlib.crc32(bytes(b)) & 0xFFFFFFFF


def frames_of(bank):
    m = [[BEGIN, *v7(len(bank), 3)]]
    for off in range(0, len(bank), CHUNK):
        m.append([WRITE, *v7(off, 3), *pack7(bank[off:off + CHUNK])])
    m.append([END, *v7(crc32(bank), 5)])
    return m


def frames_in(syx):
    """a .syx from alea: its messages, without F0 / F7"""
    out, i = [], 0
    while (i := syx.find(0xF0, i)) >= 0:
        j = syx.index(0xF7, i)
        body = list(syx[i + 1:j])
        if tuple(body[:3]) != HDR:
            raise SystemExit("not a bank for the FM-1 (no ALEA messages)")
        out.append(body[3:])
        i = j + 1
    return out


def main():
    import mido
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("bank", nargs="?", help="a .syx or .alb written by alea --fm1")
    ap.add_argument("--port", help="the MIDI port (default: the one named like the FM-1)")
    ap.add_argument("--list", action="store_true", help="show the bank the FM-1 holds")
    ap.add_argument("--factory", action="store_true", help="erase the sent bank: the factory one plays again")
    a = ap.parse_args()
    outs, ins = mido.get_output_names(), mido.get_input_names()
    like = lambda ns: next((n for n in ns if (a.port or "").lower() in n.lower()) if a.port else
                           (n for n in ns if any(k in n.lower() for k in ("fm-1", "fm1", "felucca", "m-vave"))), None)
    o, i = like(outs), like(ins)
    if not o or not i:
        raise SystemExit("no FM-1 among the MIDI ports: " + ", ".join(outs) + " (--port NAME)")
    with mido.open_output(o) as out, mido.open_input(i) as inp:
        def ask(body, wait):
            for _ in inp.iter_pending():
                pass
            out.send(mido.Message("sysex", data=[*HDR, *body]))
            t = time.time() + wait
            while time.time() < t:
                for msg in inp.iter_pending():
                    if msg.type == "sysex" and tuple(msg.data[:3]) == HDR and msg.data[3] == body[0]:
                        return list(msg.data[4:])
                time.sleep(0.002)
            raise SystemExit("no answer from the FM-1 (is the ALEA build of Felucca on it?)")

        def show():
            r = ask([LIST], 2)
            names = bytes(r[2:]).split(b"\0")[:r[1]]
            print(("its bank, sent over USB" if r[0] else "the factory bank") + f": {r[1]} patches")
            for k, n in enumerate(names):
                print(f"  {k:2}  {n.decode()}")

        if a.factory:
            fr = [[BEGIN, 0, 0, 0]]
        elif a.bank:
            raw = open(a.bank, "rb").read()
            fr = frames_of(raw) if a.bank.lower().endswith(".alb") else frames_in(raw)
        else:
            show()
            return
        for k, f in enumerate(fr):
            r = ask(f, 3 if f[0] == WRITE else 6)
            rc = r[3] if f[0] == WRITE else r[0]
            if rc:
                raise SystemExit(f"{('BEGIN', 'WRITE', 'END')[f[0] - BEGIN]}: {RC.get(rc, f'rc {rc}')}")
            print(f"\r  {k + 1}/{len(fr)}", end="", file=sys.stderr)
        print(file=sys.stderr)
        show()


if __name__ == "__main__":
    main()
