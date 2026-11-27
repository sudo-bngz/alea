# Pictures from outside

`imagine(prompt)` asks a program for a picture, described in words, and
gives it to the work as an image:

```alea
let street = imagine("a rainy street at night, ink drawing", engine: "imagine/local.py")
```

An engine is any program that reads the prompt on its standard input and
writes a PNG, a JPEG or a WebP on its standard output. It gets `ALEA_SEED`
(a whole number, the edition's own unless `seed:` is given), `ALEA_WIDTH`,
`ALEA_HEIGHT` and `ALEA_MODEL` (the `model:` given, or empty) in its
environment, to use or ignore. The path is resolved next to the work, then on
the PATH; without `engine:`, `$ALEA_IMAGINE` names it.

What an engine draws is kept in `imagined/`, next to the work, and it is
asked once: afterwards the kept file is the picture, on any machine, with or
without the engine, and in the browser. Keep the folder with the work. Delete
a file to imagine it again.

| Engine | Needs | |
|---|---|---|
| `sketch.py` | Python | no model: soft clouds of colour, the prompt choosing the colours and the seed the shapes. For trying `imagine`, and as a model of an engine |
| `local.py` | `pip install diffusers transformers accelerate torch`; the model is fetched once from Hugging Face | an image model on your own machine. `model:` names it (default `stabilityai/sdxl-turbo`); the seed is honoured |

Copy either to write your own, for the model or the service you use.
