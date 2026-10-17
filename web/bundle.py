#!/usr/bin/env python3
"""Bundles the playground into one self-contained page: the WebAssembly engine,
the examples and their pictures are all embedded, so it works offline and from
a double-click. Writes web/dist/playground.html (a full page) and
web/dist/playground.body.html (the same content without the document skeleton,
for hosts that add their own)."""
import base64, json, pathlib

root = pathlib.Path(__file__).resolve().parent.parent
web, ex = root / "web", root / "examples"
GALLERY = [
    ("rosace", "Rosace — a rose window"), ("moon", "Moon — a crescent"),
    ("bauhaus", "Bauhaus — bold shapes"), ("poster", "Poster — words and a photo"),
    ("breathing", "Breathing — a looping video"), ("phases", "Phases — works made of works"),
    ("crossing", "Crossing — mixing two works"), ("tide", "Tide — image, sound, sculpture"),
    ("synths", "Synths — instruments written in alea"),
    ("constellation", "Constellation — a galaxy (slower)"), ("hello", "Hello"),
    ("sky", "NightSky — a component"), ("frame", "Frame — needs another work"),
]
examples = []
# The tour first: one concept per file, in order.
for f in sorted((ex / "tour").glob("*.alea")):
    num, _, rest = f.stem.partition("-")
    label = f"{int(num)} · " + rest.replace("-", " ").capitalize()
    examples.append({"name": f.stem, "label": label, "group": "Tour", "dir": "tour/", "source": f.read_text()})
for f in sorted((ex / "tour" / "lib").glob("*.alea")):
    examples.append({"name": f.stem, "label": f"{f.stem.capitalize()} (used by 15)", "group": "Tour",
                     "dir": "tour/lib/", "source": f.read_text()})
listed = set()
for name, label in GALLERY:
    examples.append({"name": name, "label": label, "group": "Gallery", "dir": "", "source": (ex / f"{name}.alea").read_text()})
    listed.add(name)
for f in sorted(ex.glob("*.alea")):
    if f.stem not in listed:
        examples.append({"name": f.stem, "label": f.stem, "group": "Gallery", "dir": "", "source": f.read_text()})
# alea's instruments, written as synths: each opens, plays, and can be `use`d
for f in sorted((ex / "instruments").glob("*.alea")):
    examples.append({"name": f.stem, "label": f.stem.capitalize(), "group": "Instruments", "dir": "instruments/",
                     "source": f.read_text()})
assets = [{"path": f"assets/{p.name}", "data": base64.b64encode(p.read_bytes()).decode()}
          for p in sorted((ex / "assets").glob("*")) if p.suffix.lower() in (".png", ".jpg", ".jpeg", ".webp", ".wav")]

page = (web / "playground.html").read_text()
page = page.replace("{{WASM}}", base64.b64encode((web / "dist" / "alea.wasm").read_bytes()).decode())
page = page.replace("/*ENGINE*/", (web / "engine.js").read_text())
page = page.replace("/*EXAMPLES*/", json.dumps(examples))
page = page.replace("/*ASSETS*/", json.dumps(assets))
(web / "dist" / "playground.body.html").write_text(page)
skeleton = ('<!doctype html><html lang="en"><head><meta charset="utf-8">'
            '<meta name="viewport" content="width=device-width,initial-scale=1,viewport-fit=cover">'
            '<style>[hidden]{display:none!important}body{margin:0}</style></head><body>')
(web / "dist" / "playground.html").write_text(skeleton + page + "</body></html>")
print(f"web/dist/playground.html  {len(page) / 1e6:.1f} MB")
