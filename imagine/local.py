#!/usr/bin/env python3
"""alea imagine engine: an image model on your own machine, through diffusers.
    imagine("a rainy street at night, ink drawing", engine: "imagine/local.py")
Needs `pip install diffusers transformers accelerate torch`; the model is
fetched once from Hugging Face. `model:` (or $ALEA_IMAGINE_MODEL) names it,
by default stabilityai/sdxl-turbo. The seed is honoured: the same prompt and
seed draw the same picture on the same machine. $ALEA_IMAGINE_STEPS and
$ALEA_IMAGINE_GUIDANCE override the number of steps and the guidance."""
import io, os, sys

prompt = sys.stdin.read().strip()
seed = int(os.environ.get("ALEA_SEED", "0"))
width, height = int(os.environ.get("ALEA_WIDTH", "1024")), int(os.environ.get("ALEA_HEIGHT", "1024"))
model = os.environ.get("ALEA_MODEL") or os.environ.get("ALEA_IMAGINE_MODEL") or "stabilityai/sdxl-turbo"

# everything the libraries print goes to the standard error: the standard output is the picture
out, sys.stdout = sys.stdout, sys.stderr
import torch
from diffusers import AutoPipelineForText2Image

device = "cuda" if torch.cuda.is_available() else "mps" if torch.backends.mps.is_available() else "cpu"
pipe = AutoPipelineForText2Image.from_pretrained(model, torch_dtype=torch.float32 if device == "cpu" else torch.float16)
pipe = pipe.to(device)
pipe.set_progress_bar_config(disable=True)
turbo = "turbo" in model.lower()
steps = int(os.environ.get("ALEA_IMAGINE_STEPS", "4" if turbo else "30"))
guidance = float(os.environ.get("ALEA_IMAGINE_GUIDANCE", "0" if turbo else "7"))
# sizes are multiples of 8
image = pipe(prompt, width=width // 8 * 8, height=height // 8 * 8, num_inference_steps=steps, guidance_scale=guidance,
             generator=torch.Generator("cpu").manual_seed(seed)).images[0]
if image.size != (width, height):
    image = image.resize((width, height))
buf = io.BytesIO()
image.save(buf, "PNG")
out.buffer.write(buf.getvalue())
