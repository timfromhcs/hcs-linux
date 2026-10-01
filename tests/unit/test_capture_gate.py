"""The host-side capture grader and the in-guest one must agree.

Two graders exist on purpose: the guest judges a frame where it can explain the
failure, and the host re-judges it after the frame has crossed the disk, sharing
nothing but the thresholds. The value of a second opinion evaporates if the two
drift apart, so the thresholds are pinned here from both sides.

The synthetic frames are the real point. The first version of this gate scored on
Shannon entropy alone and passed three genuine rendering bugs, because a solid
fill and a window can both reach ~1.6 entropy. These tests encode the pictures
that fooled it.
"""

from __future__ import annotations

import importlib.util
import math
import re
import unittest
from collections import Counter
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent.parent


def _load(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec and spec.loader, f"cannot load {path}"
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


grader = _load("grade_capture", REPO / "scripts/grade_capture.py")
RUST_CAPTURE = REPO / "src/hcs-qa-agent/src/capture.rs"


def solid(w: int, h: int, colour=(16, 20, 28)):
    return [colour] * (w * h)


def gradient(w: int, h: int):
    """A smooth gradient: many colours, high entropy, no interface anywhere."""
    out = []
    for y in range(h):
        for x in range(w):
            g = (x * 200) // w + (y * 50) // h
            out.append((g // 2, g // 2, g))
    return out


def window(w: int, h: int):
    """Something that looks like the real renders: chrome, surfaces, antialiased
    glyph rows. Flat three-colour mocks would test a picture no renderer makes."""
    out = []
    for y in range(h):
        for x in range(w):
            body = 14 + ((x // 7 + y // 5) % 6)
            if y < h // 8:
                out.append((34 + (y % 5), 42 + (y % 7), 54 + (y % 9)))
            elif y > h - h // 6 and 24 < x < w - 24:
                out.append((body + 10, body + 13, body + 18))
            elif y % 29 < 9 and x % 7 < 4 and 40 < x < w - 60:
                ramp = 90 if x % 7 == 3 else 0
                out.append((225 - ramp, 235 - ramp, 245 - ramp))
            else:
                out.append((body, body + 4, body + 12))
    return out


class TestThresholdsMatchTheAgent(unittest.TestCase):
    """The two graders are independent code, not independent numbers."""

    def setUp(self):
        self.rust = RUST_CAPTURE.read_text(encoding="utf-8")

    def _rust_const(self, name: str) -> int:
        # Struct fields are `name: 4096,`; EDGE_STEP is `name: u8 = 24`.
        m = re.search(rf"\b{name}\s*:\s*(?:u8\s*=\s*)?(\d+)", self.rust)
        assert m, f"{name} not found in capture.rs"
        return int(m.group(1))

    def _rust_float(self, name: str) -> float:
        m = re.search(rf"\b{name}\s*:\s*([0-9.]+)\s*,", self.rust)
        assert m, f"{name} not found in capture.rs"
        return float(m.group(1))

    def test_numeric_thresholds_are_identical(self):
        self.assertEqual(grader.MIN_BYTES, self._rust_const("min_bytes"))
        self.assertEqual(grader.MIN_WIDTH, self._rust_const("min_width"))
        self.assertEqual(grader.MIN_HEIGHT, self._rust_const("min_height"))
        self.assertEqual(grader.MIN_UNIQUE_COLORS, self._rust_const("min_unique_colors"))
        self.assertAlmostEqual(grader.MIN_ENTROPY, self._rust_float("min_entropy"))
        self.assertAlmostEqual(grader.MIN_TEXT_RATIO, self._rust_float("min_text_ratio"))
        self.assertAlmostEqual(grader.MIN_EDGE_DENSITY, self._rust_float("min_edge_density"))
        self.assertEqual(grader.EDGE_STEP, self._rust_const("EDGE_STEP"))

    def test_the_entropy_threshold_is_above_the_floor(self):
        """A threshold at or below log2(3) can never fail, because every image
        scores at least that. v1 used 0.5, so its entropy gate was decoration."""
        floor = math.log2(3)
        self.assertGreater(
            grader.MIN_ENTROPY, floor,
            f"min_entropy {grader.MIN_ENTROPY} is at or below the {floor:.3f} floor, "
            f"so the check can never fail",
        )


class TestMeasures(unittest.TestCase):
    def test_entropy_of_a_solid_fill_is_the_three_channel_floor(self):
        """Not zero. Three separate channel histograms each hold one bucket, so
        the score is log2(3). This is the number that fooled v1's gate."""
        self.assertAlmostEqual(grader.entropy(solid(50, 50)), math.log2(3), places=6)

    def test_entropy_of_a_gradient_is_high(self):
        # The trap: a gradient is not a low-information image.
        self.assertGreater(grader.entropy(gradient(60, 60)), 7.0)

    def test_text_ratio_of_a_gradient_is_high(self):
        # ...and it also defeats the "not the modal colour" check, which is why
        # that check alone is not a text gate.
        self.assertGreater(grader.text_ratio(gradient(60, 60)), 0.9)

    def test_edge_density_separates_a_window_from_a_gradient(self):
        w, h = 120, 90
        win = grader.edge_density(window(w, h), w, h)
        grad = grader.edge_density(gradient(w, h), w, h)
        self.assertGreater(win, grad * 20.0,
                           f"window {win} vs gradient {grad}")

    def test_edge_density_of_a_solid_fill_is_zero(self):
        self.assertEqual(grader.edge_density(solid(40, 40), 40, 40), 0.0)


class TestAgainstRealFrames(unittest.TestCase):
    """The gate is calibrated against the committed renders, not against mocks.

    If a future UI redesign produces legitimately sparser frames, this test is
    what should notice — and force a decision about the threshold, rather than
    letting the gate quietly reject good work.
    """

    refs = sorted((REPO / "qa/expected/gui").rglob("*.png"))

    def setUp(self):
        if not self.refs:
            self.skipTest("no reference frames committed")

    def test_every_committed_render_passes(self):
        from PIL import Image

        failures = []
        for p in self.refs:
            with Image.open(p) as im:
                rgb = im.convert("RGB")
                w, h = rgb.size
                ed = grader.edge_density(grader._pixels(rgb), w, h)
            if ed < grader.MIN_EDGE_DENSITY:
                failures.append((p.name, round(ed, 5)))
        self.assertEqual(
            failures, [],
            "committed renders below the edge threshold — the UI got sparser, "
            "so the threshold needs a deliberate decision: " + str(failures),
        )

    def test_real_renders_clear_the_threshold_by_a_wide_margin(self):
        from PIL import Image

        worst = 1.0
        for p in self.refs:
            with Image.open(p) as im:
                rgb = im.convert("RGB")
                w, h = rgb.size
                worst = min(worst, grader.edge_density(grader._pixels(rgb), w, h))
        self.assertGreater(
            worst, grader.MIN_EDGE_DENSITY * 3,
            f"sparsest real render scores {worst:.5f}; the threshold has no margin",
        )


class TestThePicturesThatFooledTheOldGate(unittest.TestCase):
    """Each of these scored PASS under the entropy-only gate."""

    def _write(self, pixels, w, h, tmp, name):
        from PIL import Image

        p = Path(tmp) / name
        Image.frombytes("RGB", (w, h), bytes(
            c for px in pixels for c in px)).save(p)
        # Pad the file so the size gate is not what rejects it: the point of
        # these cases is that the *content* is what must fail.
        with open(p, "ab") as f:
            f.write(b"\0" * grader.MIN_BYTES)
        return p

    def test_a_solid_fill_is_rejected(self):
        import tempfile

        with tempfile.TemporaryDirectory() as tmp:
            p = self._write(solid(1280, 800), 1280, 800, tmp, "solid.png")
            ok, detail = grader.grade(p)
            self.assertFalse(ok, f"a solid fill passed: {detail}")

    def test_a_gradient_splash_is_rejected(self):
        import tempfile

        with tempfile.TemporaryDirectory() as tmp:
            p = self._write(gradient(1280, 800), 1280, 800, tmp, "splash.png")
            ok, detail = grader.grade(p)
            self.assertFalse(ok, f"a gradient splash passed: {detail}")
            self.assertIn("nothing was drawn", detail)

    def test_a_real_window_is_accepted(self):
        import tempfile

        with tempfile.TemporaryDirectory() as tmp:
            p = self._write(window(1280, 800), 1280, 800, tmp, "window.png")
            ok, detail = grader.grade(p)
            self.assertTrue(ok, f"a rendered window was rejected: {detail}")

    def test_a_tiny_frame_is_rejected(self):
        import tempfile

        with tempfile.TemporaryDirectory() as tmp:
            p = self._write(window(320, 240), 320, 240, tmp, "small.png")
            ok, detail = grader.grade(p)
            self.assertFalse(ok)
            self.assertIn("smaller than", detail)


if __name__ == "__main__":
    unittest.main()
