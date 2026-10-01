#!/usr/bin/env python3
"""Grade a QA evidence bundle produced by the in-guest agent.

The VirtualBox driver collects an evidence directory from the guest and then
decides whether the release gate passed. That decision is the part of the gate
most worth testing, and it is the part the v2 driver got wrong: it counted a
stage as PASS if a file happened to exist, and it never noticed that "skipped"
was being reported as a pass.

So the decision lives here, in a script that runs without VirtualBox, and
`tests/unit/test_qa_grading.py` exercises it against synthetic bundles.

What a green run requires, all of it:

  * every stage passed — a failed stage fails the run;
  * nothing was skipped — "we could not test this" is not "this works";
  * the run was not aborted;
  * the host's independent re-check of every frame agrees with the guest.

Usage:
  python3 scripts/grade_qa_evidence.py <evidence-dir> [--report out.json]

Exit code 0 = green. Non-zero = not green, with the reasons listed.
"""

from __future__ import annotations

import argparse
import importlib.util
import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent


def _load(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec and spec.loader, f"cannot load {path}"
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


_grader = None


def grader():
    global _grader
    if _grader is None:
        _grader = _load("grade_capture", REPO / "scripts/grade_capture.py")
    return _grader


def recheck_frames(shots_dir: Path) -> list[dict]:
    """Re-judge every frame on the host, sharing nothing with the guest's code.

    A gate that both produces and grades its own evidence is one bug away from
    grading itself green, which is precisely how the entropy-only version passed
    three real rendering bugs.
    """
    if not shots_dir.is_dir():
        return []
    out = []
    for png in sorted(shots_dir.rglob("*.png")):
        try:
            ok, detail = grader().grade(png)
        except Exception as exc:  # noqa: BLE001 - a broken frame is a failure
            ok, detail = False, f"could not be analysed ({exc})"
        out.append({"file": str(png.relative_to(shots_dir)), "ok": ok, "detail": detail})
    return out


def grade(evidence: Path) -> dict:
    """Decide whether an evidence bundle is a pass. Pure, so it can be tested."""
    result_file = evidence / "result.json"
    if not result_file.is_file():
        return {
            "green": False,
            "reasons": [
                f"no result.json in {evidence}. The guest writes DONE only after "
                f"result.json, so either the agent crashed or the disk did not mount."
            ],
            "total": 0, "passed": 0, "failed": 0, "skipped": 0,
            "host_recheck": [],
        }

    data = json.loads(result_file.read_text(encoding="utf-8"))
    reasons: list[str] = []

    failed = int(data.get("failed", 0))
    skipped = int(data.get("skipped", 0))
    aborted = data.get("aborted")

    if aborted:
        reasons.append(f"the run was aborted: {aborted}")
    if failed:
        names = [
            f"[{r['stage']:02d}] {r['name']}: {r.get('detail', 'no detail')}"
            for r in data.get("results", [])
            if r.get("status") == "fail"
        ]
        reasons.append(f"{failed} stage(s) failed:\n      " + "\n      ".join(names))
    if skipped:
        names = [
            f"[{r['stage']:02d}] {r['name']}"
            for r in data.get("results", [])
            if r.get("status") == "skip"
        ]
        reasons.append(
            f"{skipped} stage(s) were skipped. A skipped stage is not a pass: "
            f"the gate did not test it.\n      " + "\n      ".join(names)
        )

    recheck = recheck_frames(evidence / "shots")
    bad_frames = [r for r in recheck if not r["ok"]]
    if bad_frames:
        reasons.append(
            f"{len(bad_frames)} frame(s) failed the host's independent check, "
            f"which disagrees with the guest's own verdict:\n      "
            + "\n      ".join(f"{r['file']}: {r['detail']}" for r in bad_frames)
        )

    # Consistency: the counters and the per-stage rows have to agree, or the
    # report is describing a run that did not happen.
    rows = data.get("results", [])
    by_status: dict[str, int] = {}
    for r in rows:
        key = r.get("status", "?")
        by_status[key] = by_status.get(key, 0) + 1
    if by_status.get("pass", 0) != data.get("passed"):
        reasons.append(
            f"the result file is internally inconsistent: it claims "
            f"{data.get('passed')} passes but {by_status.get('pass', 0)} stage rows "
            f"say pass"
        )
    # The v2 failure shape exactly: 32 stages declared, a handful of rows, and a
    # pass count that nobody reconciled with either number.
    if data.get("total") != len(rows):
        reasons.append(
            f"the result file declares {data.get('total')} stages but contains "
            f"{len(rows)} stage rows. Stages that were never run cannot be counted "
            f"as passing."
        )

    return {
        "green": not reasons,
        "reasons": reasons,
        "version": data.get("version"),
        "profile": data.get("profile"),
        "total": data.get("total", len(rows)),
        "passed": data.get("passed", 0),
        "failed": failed,
        "skipped": skipped,
        "gui_stages": data.get("gui_stages", 0),
        "console_stages": data.get("console_stages", 0),
        "host_recheck": recheck,
        "results": rows,
    }


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description="grade a QA evidence bundle")
    ap.add_argument("evidence", help="the evidence directory collected from the guest")
    ap.add_argument("--report", help="write the verdict as JSON here")
    args = ap.parse_args(argv[1:])

    evidence = Path(args.evidence)
    verdict = grade(evidence)

    if args.report:
        Path(args.report).parent.mkdir(parents=True, exist_ok=True)
        Path(args.report).write_text(
            json.dumps(verdict, indent=2) + "\n", encoding="utf-8")

    frames = len(verdict["host_recheck"])
    if verdict["green"]:
        print(f"[OK] {verdict['passed']}/{verdict['total']} stages PASS "
              f"({verdict['gui_stages']} GUI, {verdict['console_stages']} console); "
              f"{frames} frame(s) re-checked on the host and agreeing")
        return 0

    print("[FAIL] the QA run is not green:")
    for r in verdict["reasons"]:
        print(f"  - {r}")
    return 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
