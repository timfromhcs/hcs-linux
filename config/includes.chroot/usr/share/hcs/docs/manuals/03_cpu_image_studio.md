# CPU Image Studio

Image generation runs entirely on the CPU, offline, with no GPU requirement.
The model is Stable Diffusion 1.5 in the LCM configuration, quantised to Q4.

## Why LCM

Latent Consistency Model changes the shape of diffusion sampling: instead of
dozens of steps it converges in a handful. That is what makes CPU generation
practical at all — a step count in the dozens would take hours here.

**The consequence is a hard limit: 1 to 8 steps.** The sliders clamp to that
range rather than accepting a number the model cannot honour.

## Generating from the CLI

```bash
hcs image "a glass monolith in a neural forest" --steps 6 -o render.png
```

| Flag | Meaning |
|---|---|
| `--steps` | 1–8. Six is a good default. |
| `-o, --output` | Output PNG path |
| `--input` | Image-to-image source |

## Generating from the GUI

`HCS+I` opens Image Studio next to chat.

| Control | Range | Note |
|---|---|---|
| Prompt | free text | The subject of the image |
| Negative prompt | free text | What to avoid |
| Steps | 1–8 | Clamped, not merely warned |
| CFG | 1–8 | Guidance scale |
| Resolution | 256–768 | 512 is the trained size; larger is extrapolation |
| Seed | integer | Same seed plus same prompt is reproducible |
| Generate / Cancel | — | Cancel releases the buffer immediately |

The window shows the RAM gate and, if a heavy model is already resident, a
"unload first" dialog rather than a failure at render time.

## The RAM gate

Rendering is refused when available memory is insufficient, and it is refused
*before* loading weights rather than after. Transient peak is about 2200 MB.

```
[hcs-image] mode=Txt2Img steps=6 -> render.png
[hcs-image] RAM gate PASS (free 2400MB)
```

This is the behaviour you want from a system with a hard RAM budget: a clear
"not now" instead of an out-of-memory kill that takes the desktop with it.

## Buffer lifecycle

The render buffer is released as soon as the image is written, and the weights
are deallocated if the model was loaded on demand. Steady-state memory returns to
the idle baseline; the peak is transient.

## Gallery

Finished renders appear in the Studio gallery with their metadata: prompt, steps,
CFG, seed, resolution and elapsed time. The seed is the important part — it is
what lets you come back to an image and change one thing.

## Practical notes

* 512×512 at six steps is the configuration the model was trained for. Larger
  sizes produce soft, duplicated subjects.
* Low CFG (1–2) gives loose interpretation; high CFG (7–8) follows the prompt
  more literally and can saturate.
* On a `LOWRAM-4GB` profile, close the chat window first. The gate will tell you
  if you have not.
