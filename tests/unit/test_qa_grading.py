"""The QA gate's verdict, tested without VirtualBox.

The v2 driver decided pass/fail inline in PowerShell, which is why it could
report a pass for a run where every stage had been skipped: the count of PASS
rows was never compared against the number of stages. This module exercises the
replacement decision logic against synthetic evidence bundles, so the rules are
pinned without needing a VM or an ISO.
"""

from __future__ import annotations

import importlib.util
import json
import tempfile
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent.parent


def _load(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec and spec.loader, f"cannot load {path}"
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


grading = _load("grade_qa_evidence", REPO / "scripts/grade_qa_evidence.py")
grader = _load("grade_capture", REPO / "scripts/grade_capture.py")


def stage_row(n, name, status, detail="", evidence="gui"):
    return {"stage": n, "name": name, "evidence": evidence,
            "status": status, "detail": detail, "duration_ms": 10}


def bundle(tmp: Path, rows, *, passed=None, failed=None, skipped=None,
           aborted=None, frames=(), total=None) -> Path:
    """Write an evidence directory shaped like the guest's output."""
    evidence = tmp / "evidence"
    (evidence / "shots").mkdir(parents=True, exist_ok=True)
    counts = {
        "pass": sum(1 for r in rows if r["status"] == "pass"),
        "fail": sum(1 for r in rows if r["status"] == "fail"),
        "skip": sum(1 for r in rows if r["status"] == "skip"),
    }
    data = {
        "version": "2.0.0",
        "profile": "edge",
        "total": total if total is not None else len(rows),
        "passed": passed if passed is not None else counts["pass"],
        "failed": failed if failed is not None else counts["fail"],
        "skipped": skipped if skipped is not None else counts["skip"],
        "gui_stages": sum(1 for r in rows if r["evidence"] == "gui"),
        "console_stages": sum(1 for r in rows if r["evidence"] == "console"),
        "aborted": aborted,
        "results": rows,
        "journal": [],
    }
    (evidence / "result.json").write_text(json.dumps(data, indent=2), encoding="utf-8")
    (evidence / "DONE").write_text("done\n", encoding="utf-8")
    for name, pixels, w, h in frames:
        from PIL import Image
        d = evidence / "shots" / name
        d.parent.mkdir(parents=True, exist_ok=True)
        Image.frombytes("RGB", (w, h), bytes(c for px in pixels for c in px)).save(d)
        # A real capture is a few hundred kilobytes. Synthetic frames compress
        # far too well, so without this the byte-size gate — not the content —
        # would be what every content test actually exercises.
        with open(d, "ab") as f:
            f.write(b"\0" * 8192)
    return evidence


def good_frame(w=400, h=300):
    out = []
    for y in range(h):
        for x in range(w):
            body = 14 + ((x // 7 + y // 5) % 6)
            if y < h // 8:
                out.append((34 + (y % 5), 42 + (y % 7), 54 + (y % 9)))
            elif y % 29 < 9 and x % 7 < 4 and 40 < x < w - 60:
                ramp = 90 if x % 7 == 3 else 0
                out.append((225 - ramp, 235 - ramp, 245 - ramp))
            else:
                out.append((body, body + 4, body + 12))
    return out


def solid_frame(w=400, h=300):
    return [(16, 20, 28)] * (w * h)


class TestGreenRuns(unittest.TestCase):
    def test_every_stage_passing_is_green(self):
        with tempfile.TemporaryDirectory() as t:
            rows = [stage_row(i, f"s{i}", "pass") for i in range(1, 4)]
            v = grading.grade(bundle(Path(t), rows))
            self.assertTrue(v["green"], v["reasons"])
            self.assertEqual(v["passed"], 3)

    def test_a_green_run_with_frames_is_green(self):
        with tempfile.TemporaryDirectory() as t:
            rows = [stage_row(1, "desktop", "pass")]
            # Full frame size, so the byte-size gate is not what is under test.
            frames = [("01_desktop/01.png", good_frame(1280, 800), 1280, 800)]
            v = grading.grade(bundle(Path(t), rows, frames=frames))
            self.assertTrue(v["green"], v["reasons"])
            self.assertEqual(len(v["host_recheck"]), 1)
            self.assertTrue(v["host_recheck"][0]["ok"])


class TestSkippedIsNotGreen(unittest.TestCase):
    """The exact v2 bug: stages that could not run were counted as passes."""

    def test_a_skipped_stage_fails_the_gate(self):
        with tempfile.TemporaryDirectory() as t:
            rows = [stage_row(1, "a", "pass"), stage_row(2, "b", "skip", "scenario missing")]
            v = grading.grade(bundle(Path(t), rows))
            self.assertFalse(v["green"])
            self.assertTrue(any("skipped" in r for r in v["reasons"]), v["reasons"])

    def test_a_fully_skipped_run_is_not_a_pass(self):
        with tempfile.TemporaryDirectory() as t:
            rows = [stage_row(i, f"s{i}", "skip", "no scenario") for i in range(1, 4)]
            v = grading.grade(bundle(Path(t), rows))
            self.assertFalse(v["green"])
            self.assertEqual(v["passed"], 0)

    def test_the_skip_reason_names_the_stage(self):
        with tempfile.TemporaryDirectory() as t:
            rows = [stage_row(7, "start_menu", "skip", "scenario missing")]
            v = grading.grade(bundle(Path(t), rows))
            self.assertTrue(any("start_menu" in r for r in v["reasons"]), v["reasons"])


class TestFailedStages(unittest.TestCase):
    def test_one_failed_stage_fails_the_gate(self):
        with tempfile.TemporaryDirectory() as t:
            rows = [stage_row(1, "a", "pass"),
                    stage_row(2, "b", "fail", "no window matching 'Files'")]
            v = grading.grade(bundle(Path(t), rows))
            self.assertFalse(v["green"])
            self.assertTrue(any("no window matching" in r for r in v["reasons"]),
                            v["reasons"])

    def test_an_abort_fails_the_gate_even_with_no_failures(self):
        with tempfile.TemporaryDirectory() as t:
            rows = [stage_row(1, "a", "pass")]
            v = grading.grade(bundle(Path(t), rows, aborted="the session never came up"))
            self.assertFalse(v["green"])
            self.assertTrue(any("aborted" in r for r in v["reasons"]), v["reasons"])


class TestInconsistentResults(unittest.TestCase):
    """A report that describes a run which did not happen is not a pass."""

    def test_a_counter_that_disagrees_with_the_rows_fails(self):
        with tempfile.TemporaryDirectory() as t:
            rows = [stage_row(1, "a", "pass"), stage_row(2, "b", "pass")]
            v = grading.grade(bundle(Path(t), rows, passed=32))
            self.assertFalse(v["green"])
            self.assertTrue(any("inconsistent" in r for r in v["reasons"]), v["reasons"])

    def test_more_passes_than_stages_is_caught(self):
        # The v2 shape: 32 stages declared, 0 actually run, reported as passing.
        with tempfile.TemporaryDirectory() as t:
            rows = [stage_row(i, f"s{i}", "pass") for i in range(1, 3)]
            v = grading.grade(bundle(Path(t), rows, total=32))
            self.assertFalse(v["green"])


class TestMissingEvidence(unittest.TestCase):
    def test_no_result_file_is_not_a_pass(self):
        with tempfile.TemporaryDirectory() as t:
            empty = Path(t) / "evidence"
            empty.mkdir()
            v = grading.grade(empty)
            self.assertFalse(v["green"])
            self.assertTrue(any("result.json" in r for r in v["reasons"]), v["reasons"])


class TestHostRecheckOverridesTheGuest(unittest.TestCase):
    """The guest said it passed. The host looked again and disagrees."""

    def test_a_solid_frame_fails_even_though_the_guest_passed_the_stage(self):
        with tempfile.TemporaryDirectory() as t:
            rows = [stage_row(1, "desktop", "pass")]
            # Padded so the size gate cannot be what rejects it: the content is
            # the thing under test.
            frames = [("01_desktop/01.png", solid_frame(1280, 800), 1280, 800)]
            v = grading.grade(bundle(Path(t), rows, frames=frames))
            self.assertFalse(v["green"], "the host's independent check was ignored")
            self.assertTrue(any("independent check" in r for r in v["reasons"]),
                            v["reasons"])

    def test_a_gradient_frame_fails_the_host_check(self):
        w, h = 400, 300
        grad = []
        for y in range(h):
            for x in range(w):
                g = (x * 200) // w + (y * 50) // h
                grad.append((g // 2, g // 2, g))
        with tempfile.TemporaryDirectory() as t:
            rows = [stage_row(1, "splash", "pass")]
            frames = [("01_splash/01.png", grad, w, h)]
            v = grading.grade(bundle(Path(t), rows, frames=frames))
            self.assertFalse(v["green"])

    def test_no_shots_directory_is_not_a_failure_by_itself(self):
        # Console stages legitimately produce no frame. Absence of frames must
        # not be mistaken for a broken run.
        with tempfile.TemporaryDirectory() as t:
            rows = [stage_row(1, "grub", "pass", evidence="console")]
            v = grading.grade(bundle(Path(t), rows))
            self.assertTrue(v["green"], v["reasons"])


if __name__ == "__main__":
    unittest.main()
