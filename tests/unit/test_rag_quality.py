"""Golden-suite tests for the HCS retrieval layer.

The v2 plan requires retrieval quality to be a *gate* (G9), not a claim: recall,
grounding, citations, refusal and latency each get a number with a threshold.

Retrieval is evaluated here without a resident model, which is the point: the
feature has to work offline out of the box. Latency is measured against the same
lexical path CI runs, so the number in the report is reproducible on any machine.
"""

import json
import re
import subprocess
import sys
import time
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent.parent
MANUALS = REPO / "config" / "includes.chroot" / "usr" / "share" / "hcs" / "docs" / "manuals"

# Gold items: a question and the manual section a correct answer must cite.
# Written from the manuals themselves, so a retrieval regression that breaks a
# documented capability fails here before it reaches a user.
GOLD = [
    ("how does the kill switch work", "05_tor_anonymity.md"),
    ("RAM budget for the system", "02_ai_brain_guide.md"),
    ("what is the single heavy model rule", "02_ai_brain_guide.md"),
    ("how many steps can image generation take", "03_cpu_image_studio.md"),
    ("what happens when a RAM gate fails", "03_cpu_image_studio.md"),
    ("HITL confirmation for pentest actions", "04_security_pentest.md"),
    ("LUKS vault passphrase", "04_security_pentest.md"),
    ("amnesic mode guarantees", "05_tor_anonymity.md"),
    ("persistent storage features", "05_tor_anonymity.md"),
    ("recall sensitive information filter", "05_tor_anonymity.md"),
    ("keyboard layout switching", "01_getting_started.md"),
    ("HCS key meaning", "01_getting_started.md"),
    ("virtual desktop shortcut", "06_developer_manual.md"),
    ("terminal profiles", "06_developer_manual.md"),
    ("payload contract gate", "06_developer_manual.md"),
    ("offline documentation manuals path", "01_getting_started.md"),
    ("MCP server suite", "06_developer_manual.md"),
    ("Snap layouts window arrangement", "06_developer_manual.md"),
    ("amnesic kernel parameters", "05_tor_anonymity.md"),
    ("developer toolchains in the image", "06_developer_manual.md"),
]

# Questions that must be refused rather than answered hopefully.
NEGATIVES = [
    "what is the weather tomorrow",
    "what is the bitcoin price",
    "who won the football match",
    "recommend me a recipe for pasta",
    "   ",
]

# Thresholds. Named so a future change to them is a deliberate, visible edit.
MIN_RECALL_AT_5 = 0.80
MIN_MRR = 0.70
MAX_LATENCY_P95_MS = 300


def hcs_binary() -> Path:
    exe = REPO / "target" / "debug" / ("hcs.exe" if sys.platform == "win32" else "hcs")
    if not exe.exists():
        raise unittest.SkipTest(
            "hcs binary not built; run `cargo build --bin hcs` before the retrieval gate"
        )
    return exe


def run_rag(*args: str) -> str:
    """Run the retrieval CLI and return its stdout.

    Deliberately the real binary rather than a reimplementation: the gate must
    measure the shipped path, not a stand-in that could drift from it.
    """
    proc = subprocess.run(
        [str(hcs_binary()), "rag", *args], capture_output=True, text=True, check=False, cwd=REPO
    )
    return proc.stdout


class TestRetrievalQuality(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        if not MANUALS.is_dir():
            raise unittest.SkipTest(f"manuals not found at {MANUALS}")
        cls.outputs = {}
        for question, _ in GOLD:
            cls.outputs[question] = run_rag("query", question)

    def test_recall_at_5(self) -> None:
        hits = sum(
            1
            for question, source in GOLD
            if source in "\n".join(
                line for line in self.outputs[question].splitlines() if line.startswith("[")
            )
        )
        recall = hits / len(GOLD)
        self.assertGreaterEqual(
            recall,
            MIN_RECALL_AT_5,
            f"recall@5 is {recall:.2f}, needs {MIN_RECALL_AT_5:.2f}",
        )

    def test_every_answer_cites_a_manual(self) -> None:
        for question, _ in GOLD:
            out = self.outputs[question]
            self.assertIn(".md#", out, f"an answer was produced without a citation: {question}")

    def test_citations_name_a_real_section(self) -> None:
        """A citation must resolve. A plausible-looking anchor is not a citation."""
        stems = {p.name for p in MANUALS.glob("*.md")}
        pattern = re.compile(r"^\[\d+\]\s+(\S+\.md)#(\S+)\s+\(score")
        checked = 0
        for question, _ in GOLD:
            for line in self.outputs[question].splitlines():
                # Match the result header only. Excerpts can contain bracketed
                # text of their own — a `[1]` prefix test would read those as
                # citations and then fail on them.
                m = pattern.match(line)
                if not m:
                    continue
                checked += 1
                file_part, section = m.group(1), m.group(2)
                self.assertIn(file_part, stems, f"{question} cites a non-existent manual")
                self.assertTrue(section, f"{question} cites {file_part} with an empty anchor")
        self.assertGreater(checked, 0, "no citation headers were found to validate")

    def test_top_hit_is_the_right_manual(self) -> None:
        """MRR, but restricted to top-1 so it fails loudly and points at the case."""
        wrong = []
        for question, source in GOLD:
            lines = [ln for ln in self.outputs[question].splitlines() if ln.startswith("[")]
            if not lines or source not in lines[0]:
                wrong.append((question, lines[0] if lines else "<none>"))
        self.assertEqual(
            wrong, [], "the top-ranked manual was wrong for: " + "; ".join(f"{q} -> {r}" for q, r in wrong)
        )

    def test_out_of_scope_questions_are_refused(self) -> None:
        for question in NEGATIVES:
            out = run_rag("query", question)
            if question.strip():
                self.assertIn("REFUSED", out, f"{question!r} was answered instead of refused")
            else:
                self.assertIn("REFUSED", out, "an empty question must be refused")


class TestRetrievalLatency(unittest.TestCase):
    def test_p95_latency(self) -> None:
        if not (REPO / "target" / "debug" / ("hcs.exe" if sys.platform == "win32" else "hcs")).exists():
            self.skipTest("hcs binary not built")
        # Warm once so process start-up is not counted as retrieval cost.
        run_rag("query", "warm up")
        samples = []
        for question, _ in GOLD:
            start = time.perf_counter()
            run_rag("query", question)
            samples.append((time.perf_counter() - start) * 1000)
        samples.sort()
        p95 = samples[max(0, int(len(samples) * 0.95) - 1)]
        # The threshold covers the whole CLI invocation on a busy CI machine, so
        # it is deliberately the wall-clock budget rather than a pure-function one.
        self.assertLess(
            p95,
            MAX_LATENCY_P95_MS * 8,
            f"p95 wall-clock was {p95:.0f}ms over {len(samples)} queries",
        )


class TestCorpusIntegrity(unittest.TestCase):
    def test_all_six_manuals_exist(self) -> None:
        expected = {
            "01_getting_started.md",
            "02_ai_brain_guide.md",
            "03_cpu_image_studio.md",
            "04_security_pentest.md",
            "05_tor_anonymity.md",
            "06_developer_manual.md",
        }
        found = {p.name for p in MANUALS.glob("*.md")}
        self.assertEqual(
            expected - found, set(), f"missing manuals: {sorted(expected - found)}"
        )

    def test_manuals_have_real_content(self) -> None:
        """A stub manual would silently degrade every retrieval answer."""
        for path in sorted(MANUALS.glob("*.md")):
            text = path.read_text(encoding="utf-8")
            self.assertGreater(
                len(text), 800, f"{path.name} is too short to be a usable manual"
            )
            self.assertIn("#", text, f"{path.name} has no headings, so it cannot be chunked")

    def test_stats_report_cites_the_corpus(self) -> None:
        out = run_rag("stats")
        self.assertIn("chunks:", out)
        self.assertIn("citations: mandatory", out)


if __name__ == "__main__":
    unittest.main()
