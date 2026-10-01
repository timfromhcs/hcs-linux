#!/usr/bin/env python3
"""Grade one captured frame, independently of the guest's own verdict.

The in-guest agent already judges its captures, because it is the only thing
that can explain *why* a frame failed. This is the second opinion: the frames are
re-checked here, after they have crossed the disk, by code that shares nothing
with the agent except the thresholds.

That independence is the point. The first version of this gate lived entirely
inside the capture script, scored frames on Shannon entropy alone, and passed
three real rendering bugs. A gate that both produces and grades its own evidence
is one bug away from grading itself green.

Usage:
  python3 scripts/grade_capture.py shot.png [shot2.png ...]

Exit code 0 = every frame passed. Non-zero = at least one did not, and each
failure is printed with the numbers behind it.
"""

from __future__ import annotations

import math
import sys
from collections import Counter
from pathlib import Path

# The thresholds live in src/hcs-qa-agent/src/capture.rs. They are duplicated
# here on purpose — a second opinion that imported the first opinion's numbers
# would not be a second opinion — and the unit test
# `test_thresholds_match_the_agent` fails if the two drift apart.
MIN_BYTES = 4096
MIN_WIDTH = 640
MIN_HEIGHT = 400
MIN_UNIQUE_COLORS = 24
# Entropy is computed over three separate channel histograms, so every image
# scores at least log2(3) = 1.585 — a solid fill measures exactly that. v1's
# threshold of 0.5 was therefore dead code, which is exactly how a solid fill
# passed it. Real renders measure 2.21 to 4.02.
MIN_ENTROPY = 1.8
MIN_TEXT_RATIO = 0.003
MIN_EDGE_DENSITY = 0.010
EDGE_STEP = 24


def entropy(pixels: list[tuple[int, int, int]]) -> float:
    hist: Counter = Counter()
    for p in pixels:
        hist[p[0]] += 1
        hist[p[1]] += 1
        hist[p[2]] += 1
    total = sum(hist.values())
    if not total:
        return 0.0
    return -sum((c / total) * math.log2(c / total) for c in hist.values() if c)


def text_ratio(pixels: list[tuple[int, int, int]]) -> float:
    """Share of pixels that are not the modal colour. Catches a solid fill."""
    if not pixels:
        return 0.0
    modal = Counter(pixels).most_common(1)[0][1]
    return 1.0 - (modal / len(pixels))


def edge_density(pixels: list[tuple[int, int, int]], w: int, h: int) -> float:
    """Share of pixels next to a much brighter or darker pixel.

    The gate that actually proves an interface was rendered. Text, borders and
    icons are edges; a gradient wallpaper or a boot splash is not, no matter how
    many colours or how much entropy it carries.
    """
    if w < 2 or h < 2:
        return 0.0
    edges = 0
    total = 0
    for y in range(h):
        row = y * w
        nrow = row + w
        for x in range(w):
            total += 1
            p = pixels[row + x]
            hit = False
            if x + 1 < w:
                q = pixels[row + x + 1]
                if (abs(p[0] - q[0]) >= EDGE_STEP
                        or abs(p[1] - q[1]) >= EDGE_STEP
                        or abs(p[2] - q[2]) >= EDGE_STEP):
                    hit = True
            if not hit and y + 1 < h:
                q = pixels[nrow + x]
                if (abs(p[0] - q[0]) >= EDGE_STEP
                        or abs(p[1] - q[1]) >= EDGE_STEP
                        or abs(p[2] - q[2]) >= EDGE_STEP):
                    hit = True
            if hit:
                edges += 1
    return edges / total if total else 0.0


def _pixels(im) -> list[tuple[int, int, int]]:
    """Decode to a flat RGB list, coping with the Pillow 13 rename.

    `Image.getdata` is deprecated in favour of `get_flattened_data`, and on a
    host that treats the deprecation as an error this would otherwise throw —
    turning a perfectly good capture into "could not be analysed".
    """
    fn = getattr(im, "get_flattened_data", None) or im.getdata
    return list(fn())


def grade(path: Path) -> tuple[bool, str]:
    from PIL import Image

    size = path.stat().st_size
    if size < MIN_BYTES:
        return False, f"only {size} bytes on disk (minimum {MIN_BYTES})"
    with Image.open(path) as im:
        rgb = im.convert("RGB")
        w, h = rgb.size
        pixels = _pixels(rgb)

    if w < MIN_WIDTH or h < MIN_HEIGHT:
        return False, f"{w}x{h} is smaller than the {MIN_WIDTH}x{MIN_HEIGHT} minimum"

    colors = len(set(pixels))
    if colors < MIN_UNIQUE_COLORS:
        return False, f"only {colors} unique colours (minimum {MIN_UNIQUE_COLORS}) — a solid fill"

    e = entropy(pixels)
    if e < MIN_ENTROPY:
        return False, f"entropy {e:.3f} below {MIN_ENTROPY}"

    tr = text_ratio(pixels)
    if tr < MIN_TEXT_RATIO:
        return False, f"text ratio {tr:.5f} below {MIN_TEXT_RATIO} — the frame is one flat fill"

    ed = edge_density(pixels, w, h)
    if ed < MIN_EDGE_DENSITY:
        return False, (f"edge density {ed:.5f} below {MIN_EDGE_DENSITY} — nothing was "
                       f"drawn; a gradient or splash has colours but no interface")

    return True, (f"{w}x{h} entropy {e:.3f} colours {colors} "
                  f"text {tr:.4f} edges {ed:.4f}")


def main(argv: list[str]) -> int:
    if len(argv) < 2:
        print(__doc__)
        return 2
    bad = 0
    for raw in argv[1:]:
        p = Path(raw)
        if not p.is_file():
            print(f"[FAIL] {p.name}: no such file")
            bad += 1
            continue
        try:
            ok, detail = grade(p)
        except Exception as exc:  # noqa: BLE001 - a broken frame is a failure, not a crash
            print(f"[FAIL] {p.name}: could not be analysed ({exc})")
            bad += 1
            continue
        print(f"  [{'OK  ' if ok else 'FAIL'}] {p.name:<48} {detail}")
        if not ok:
            bad += 1
    if bad:
        print(f"{bad} of {len(argv) - 1} frame(s) failed the host-side check")
        return 1
    print(f"{len(argv) - 1} frame(s) passed the host-side check")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
