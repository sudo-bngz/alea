#!/usr/bin/env python3
"""alea voice engine: Kokoro (82M, open weights) with misaki, its own G2P.

    pip install kokoro-onnx misaki num2words phonemizer
    python3 -m spacy download en_core_web_sm
    say("Hello.", engine: "voices/kokoro-misaki.py", voice: "af_kore")

The same voices as kokoro.py, but the text is turned into phonemes by misaki
(the G2P Kokoro was trained with: stress, phrase and context aware) instead
of a word-by-word lexicon, and a line is spoken in one breath: the prosody is
noticeably more natural. English and French (French through espeak-ng, as
misaki does). The model (about 330 MB) is fetched once from the kokoro-onnx
releases on GitHub into the cache directory ($KOKORO_ONNX_DIR to put it
elsewhere). The text comes on stdin; the WAV goes to stdout. Words the G2P
does not know (names) are pronounced with espeak-ng, after kokoro-words.txt
next to this script ("word phones", in the model's own tokens).
"""
import os, re, sys, urllib.request, wave

MODEL = "kokoro-onnx-v1_0"
URL = "https://github.com/thewh1teagle/kokoro-onnx/releases/download/model-files-v1.0/"
FILES = ("kokoro-v1.0.onnx", "voices-v1.0.bin")


def model_dir():
    d = os.environ.get("KOKORO_ONNX_DIR")
    if d and all(os.path.exists(os.path.join(d, f)) for f in FILES):
        return d
    cache = os.environ.get("XDG_CACHE_HOME") or os.path.join(os.path.expanduser("~"), ".cache")
    d = d or os.path.join(cache, "alea", MODEL)
    os.makedirs(d, exist_ok=True)
    for f in FILES:
        p = os.path.join(d, f)
        if not os.path.exists(p):
            print(f"kokoro-misaki: fetching {f} into {d} (once)", file=sys.stderr)
            urllib.request.urlretrieve(URL + f, p + ".part")
            os.rename(p + ".part", p)
    return d


def own_words():
    """kokoro-words.txt next to this script: word, a space, the phones (spaced or not)."""
    words = {}
    here = os.path.join(os.path.dirname(os.path.abspath(__file__)), "kokoro-words.txt")
    if os.path.exists(here):
        with open(here, encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if not line or line.startswith("#"):
                    continue
                w, _, phones = line.partition(" ")
                words[w.lower()] = phones.replace(" ", "")
    return words


def phonemize(text, lang, british):
    from misaki import en, espeak
    if lang == "fr":
        g = espeak.EspeakG2P(language="fr-fr")
        return g(text)[0]
    g = en.G2P(trf=False, british=british, fallback=espeak.EspeakFallback(british=british))
    own = own_words()
    if own:
        # words of your own go into the gold lexicon, so misaki uses them in context
        for w, phones in own.items():
            g.lexicon.golds[w] = phones
    return g(text)[0]


def main():
    import numpy as np
    from kokoro_onnx import Kokoro
    text = sys.stdin.read().strip()
    lang = os.environ.get("ALEA_LANG", "en")
    voice = os.environ.get("ALEA_VOICE") or ("ff_siwis" if lang == "fr" else "af_heart")
    speed = float(os.environ.get("ALEA_SPEED", "1"))
    british = voice.startswith("b")
    phonemes = phonemize(text, lang, british)
    d = model_dir()
    tts = Kokoro(os.path.join(d, FILES[0]), os.path.join(d, FILES[1]))
    if voice not in tts.get_voices():
        sys.exit(f"kokoro-misaki: no voice `{voice}`; the voices are " + ", ".join(tts.get_voices()))
    samples, rate = tts.create(phonemes, voice=voice, speed=speed, is_phonemes=True, sentence_pause=0.35, clause_pause=0.12)
    out = wave.open(sys.stdout.buffer, "wb")
    out.setnchannels(1)
    out.setsampwidth(2)
    out.setframerate(rate)
    out.writeframes((np.clip(np.asarray(samples), -1, 1) * 32767).astype(np.int16).tobytes())
    out.close()


if __name__ == "__main__":
    main()
