#!/usr/bin/env python3
"""alea voice engine: Kokoro (82M, open weights), run locally with sherpa-onnx.

    pip install sherpa-onnx numpy
    say("Hello.", engine: "voices/kokoro.py", voice: "af_heart")

The model (about 350 MB, English, French, Chinese, Japanese, and more) is
fetched once from the sherpa-onnx releases on GitHub into the cache
directory ($KOKORO_DIR to put it elsewhere). The text comes on stdin; the
WAV goes to stdout. Voices: af_heart (the default), af_bella, af_sky,
am_adam, am_michael, bf_emma, bm_george, ff_siwis (French)… ALEA_LANG picks
the language; words the model's lexicon does not know (names) are
pronounced with espeak-ng if it is installed, after kokoro-words.txt next to
this script ("word phones", in the model's own tokens).
"""
import os, re, subprocess, sys, tarfile, urllib.request, wave

MODEL = "kokoro-multi-lang-v1_0"
URL = f"https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/{MODEL}.tar.bz2"
SPEAKERS = ("af_alloy af_aoede af_bella af_heart af_jessica af_kore af_nicole af_nova af_river af_sarah af_sky "
            "am_adam am_echo am_eric am_fenrir am_liam am_michael am_onyx am_puck am_santa bf_alice bf_emma bf_isabella "
            "bf_lily bm_daniel bm_fable bm_george bm_lewis ef_dora em_alex ff_siwis hf_alpha hf_beta hm_omega hm_psi "
            "if_sara im_nicola jf_alpha jf_gongitsune jf_nezumi jf_tebukuro jm_kumo pf_dora pm_alex pm_santa zf_xiaobei "
            "zf_xiaoni zf_xiaoxiao zf_xiaoyi zm_yunjian zm_yunxi zm_yunxia zm_yunyang em_santa").split()
# espeak-ng's IPA to the model's tokens (misaki's)
IPA = {"eɪ": "A", "aɪ": "I", "oʊ": "O", "aʊ": "W", "ɔɪ": "Y", "tʃ": "ʧ", "dʒ": "ʤ", "ɚ": "əɹ", "ɜː": "ɜɹ", "ɐ": "ə",
       "ɑː": "ɑ", "iː": "i", "uː": "u", "ɔː": "ɔ", "r": "ɹ", "ː": "", "‍": "", "ˑ": "", "ʔ": ""}


def model_dir():
    d = os.environ.get("KOKORO_DIR")
    if d and os.path.exists(os.path.join(d, "model.onnx")):
        return d
    cache = os.environ.get("XDG_CACHE_HOME") or os.path.join(os.path.expanduser("~"), ".cache")
    d = d or os.path.join(cache, "alea", MODEL)
    if not os.path.exists(os.path.join(d, "model.onnx")):
        print(f"kokoro: fetching the model into {d} (about 350 MB, once)", file=sys.stderr)
        os.makedirs(os.path.dirname(d), exist_ok=True)
        tmp = d + ".tar.bz2"
        urllib.request.urlretrieve(URL, tmp)
        with tarfile.open(tmp) as t:
            t.extractall(os.path.dirname(d))
        os.remove(tmp)
    return d


def extra_lexicon(text, d, lang):
    """Words the lexicon does not have, pronounced by espeak-ng: a lexicon file of their own."""
    if lang != "en":
        return None
    words = sorted({w.lower() for w in re.findall(r"[A-Za-z][A-Za-z']*", text)})
    known = set()
    with open(os.path.join(d, "lexicon-us-en.txt"), encoding="utf-8") as f:
        for line in f:
            known.add(line.split(" ", 1)[0])
    # words of your own first: kokoro-words.txt next to this script, "word phones"
    own = {}
    here = os.path.join(os.path.dirname(os.path.abspath(__file__)), "kokoro-words.txt")
    if os.path.exists(here):
        with open(here, encoding="utf-8") as f:
            for line in f:
                if line.strip() and not line.startswith("#"):
                    w, p = line.strip().split(" ", 1)
                    own[w.lower()] = p
    lines = [w + " " + own[w] for w in words if w in own]
    missing = [w for w in words if w not in known and w not in own]
    if missing:
        try:
            ipa = subprocess.run(["espeak-ng", "-q", "--ipa=3", "-v", "en-us", " ".join(missing)], capture_output=True, text=True, check=True).stdout.split()
        except (OSError, subprocess.CalledProcessError):
            ipa = []
            print("kokoro: words not in the lexicon are skipped (install espeak-ng to pronounce them): " + ", ".join(missing), file=sys.stderr)
        for w, p in zip(missing, ipa):
            for k, v in IPA.items():
                p = p.replace(k, v)
            lines.append(w + " " + " ".join(p))
    if not lines:
        return None
    path = os.path.join(d, "lexicon-extra.txt")
    with open(path, "w", encoding="utf-8") as f:
        f.write("\n".join(lines) + "\n")
    return path


def main():
    import numpy as np
    import sherpa_onnx
    text = sys.stdin.read().strip()
    lang = os.environ.get("ALEA_LANG", "en")
    voice = os.environ.get("ALEA_VOICE") or ("ff_siwis" if lang == "fr" else "af_heart")
    if voice not in SPEAKERS:
        sys.exit(f"kokoro: no voice `{voice}`; the voices are " + ", ".join(SPEAKERS))
    speed = float(os.environ.get("ALEA_SPEED", "1"))
    d = model_dir()
    lex = [os.path.join(d, "lexicon-us-en.txt"), os.path.join(d, "lexicon-zh.txt")]
    extra = extra_lexicon(text, d, lang)
    if extra:
        lex.insert(0, extra)
    cfg = sherpa_onnx.OfflineTtsConfig(
        model=sherpa_onnx.OfflineTtsModelConfig(
            kokoro=sherpa_onnx.OfflineTtsKokoroModelConfig(
                model=os.path.join(d, "model.onnx"), voices=os.path.join(d, "voices.bin"), tokens=os.path.join(d, "tokens.txt"),
                data_dir=os.path.join(d, "espeak-ng-data"), dict_dir=os.path.join(d, "dict"), lexicon=",".join(lex)),
            num_threads=os.cpu_count() or 2),
        max_num_sentences=1)
    tts = sherpa_onnx.OfflineTts(cfg)
    audio = tts.generate(text, sid=SPEAKERS.index(voice), speed=speed)
    out = wave.open(sys.stdout.buffer, "wb")
    out.setnchannels(1)
    out.setsampwidth(2)
    out.setframerate(audio.sample_rate)
    out.writeframes((np.clip(np.array(audio.samples), -1, 1) * 32767).astype(np.int16).tobytes())
    out.close()


if __name__ == "__main__":
    main()
