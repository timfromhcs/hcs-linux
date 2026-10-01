#!/usr/bin/env python3
"""
HCS Linux — GUI verification gate.

Two modes, both usable on machines without GPU/display/Wayland (CI, VirtualBox
host, headless build agents). The Slint software renderer produces the pixels,
this script audits them.

  --render-only : render every view, then assert the output is not blank
                  (size + Shannon entropy), same method as verify_visual_qa.py
  --regress     : render every view and diff against qa/expected/gui/<platform>/
                  (mean absolute pixel difference, per-pixel tolerance)
  --update      : render and overwrite the reference images
  --list        : print the view inventory

Plan: docs/GUI_BUILD_PLAN.md §4 steps 2-3.
"""

from __future__ import annotations

import argparse
import json
import math
import shutil
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
GUI_DIR = REPO_ROOT / "qa/gui"
EXPECTED_ROOT = REPO_ROOT / "qa/expected/gui"
REPORT = REPO_ROOT / "qa/reports/gui_regression.json"


def platform_key() -> str:
    """Reference images are per-platform, and that is not negotiable.

    The Slint software renderer rasterises text through the platform's font
    stack: DirectWrite on Windows, FreeType/fontconfig on Linux. The same Slint
    document therefore produces slightly different glyph antialiasing and
    hinting on each platform, which is a ~2-6% pixel difference on text-heavy
    views. A single shared reference set can only ever be exact on the platform
    that generated it; on the other it fails while the UI is in fact correct.

    So references live under qa/expected/gui/<platform>/ and the gate compares
    a render against the references captured on that same platform. The
    non-blank render audit still runs everywhere, because "did anything draw at
    all" is platform independent.
    """
    if sys.platform == "win32":
        return "windows"
    if sys.platform.startswith("linux"):
        return "linux"
    if sys.platform == "darwin":
        return "macos"
    return "other"


def expected_dir() -> Path:
    return EXPECTED_ROOT / platform_key()

# Same thresholds as scripts/verify_visual_qa.py so GUI and VM gates agree.
MIN_SIZE_BYTES = 1500
MIN_ENTROPY = 0.5
MIN_WIDTH = 320
MIN_HEIGHT = 240
# A flat fill still reaches ~1.6 entropy after PNG quantisation, so entropy
# alone cannot detect a blank frame. A real UI always uses many colours.
MIN_UNIQUE_COLORS = 8
# Visual regression tolerance.
MAX_MEAN_DIFF = 2.0
PIXEL_TOLERANCE = 12

CARGO = "cargo" if sys.platform != "win32" else "cargo.exe"


def sh(cmd: list[str]) -> subprocess.CompletedProcess:
    return subprocess.run(cmd, cwd=REPO_ROOT, capture_output=True, text=True)


def render(theme: str, extra: list[str] | None = None) -> bool:
    cmd = [CARGO, "run", "-q", "-p", "hcs-gui-shots", "--bin", "hcs-gui-shots",
           "--", "--out-dir", str(GUI_DIR), "--theme", theme]
    if extra:
        cmd += extra
    p = sh(cmd)
    if p.returncode != 0:
        print(p.stdout)
        print(p.stderr, file=sys.stderr)
        return False
    print(p.stdout.strip())
    return True


def entropy(img) -> float:
    hist = img.histogram()
    total = sum(hist)
    if total == 0:
        return 0.0
    e = 0.0
    for count in hist:
        if count:
            p = count / total
            e -= p * math.log2(p)
    return e


def audit_render(theme: str) -> list[dict]:
    from PIL import Image

    results = []
    for png in sorted(GUI_DIR.glob(f"*-{theme}.png")):
        size = png.stat().st_size
        entry = {"file": png.name, "size_bytes": size}
        if size < MIN_SIZE_BYTES:
            entry.update(status="FAIL", error="file too small (blank/truncated render)")
            results.append(entry)
            print(f"  [FAIL] {png.name:<44} size {size} B")
            continue
        with Image.open(png) as im:
            rgb = im.convert("RGB")
            w, h = rgb.size
            e = entropy(rgb)
            colors = rgb.getcolors(maxcolors=4096)
            n_colors = len(colors) if colors is not None else 4096
        ok = (w >= MIN_WIDTH and h >= MIN_HEIGHT
              and e >= MIN_ENTROPY and n_colors >= MIN_UNIQUE_COLORS)
        entry.update(width=w, height=h, entropy=round(e, 3), unique_colors=n_colors,
                     status="PASS" if ok else "FAIL")
        if not ok:
            entry["error"] = (f"blank frame (entropy {e:.3f}, {n_colors} unique "
                              f"colours) or too small")
        results.append(entry)
        sym = "[OK]  " if ok else "[FAIL]"
        print(f"  {sym} {png.name:<44} {w}x{h} entropy {e:>5.3f} colors {n_colors:>5}")
    return results


def diff_against_reference(theme: str) -> list[dict]:
    from PIL import Image, ImageChops

    results = []
    refs = expected_dir()
    for png in sorted(GUI_DIR.glob(f"*-{theme}.png")):
        ref = refs / png.name
        entry = {"file": png.name, "reference": str(ref.relative_to(REPO_ROOT))}
        if not ref.exists():
            entry.update(status="FAIL", error="no reference image (run with --update)")
            results.append(entry)
            print(f"  [FAIL] {png.name:<44} missing reference")
            continue
        with Image.open(png) as a, Image.open(ref) as b:
            ia, ib = a.convert("RGB"), b.convert("RGB")
        if ia.size != ib.size:
            entry.update(status="FAIL",
                         error=f"size changed {ia.size} -> {ib.size}")
            results.append(entry)
            print(f"  [FAIL] {png.name:<44} size {ia.size} vs {ib.size}")
            continue
        # Count pixels differing beyond tolerance, then mean abs difference.
        diff = ImageChops.difference(ia, ib)
        bands = diff.split()
        big = 0
        total_delta = 0
        px = ia.size[0] * ia.size[1]
        pix = list(diff.getdata())
        for p in pix:
            d = max(p)
            total_delta += d
            if d > PIXEL_TOLERANCE:
                big += 1
        mean_diff = total_delta / (px * 3)
        changed_pct = 100.0 * big / px
        ok = mean_diff <= MAX_MEAN_DIFF and changed_pct <= 2.0
        entry.update(mean_diff=round(mean_diff, 4),
                     changed_pct=round(changed_pct, 4),
                     status="PASS" if ok else "FAIL")
        if not ok:
            entry["error"] = (f"visual drift: mean {mean_diff:.3f} "
                              f"> {MAX_MEAN_DIFF} or {changed_pct:.2f}% pixels changed")
        results.append(entry)
        sym = "[OK]  " if ok else "[FAIL]"
        print(f"  {sym} {png.name:<44} mean {mean_diff:>6.3f} changed {changed_pct:>6.2f}%")
    return results


def main() -> int:
    ap = argparse.ArgumentParser(description="HCS GUI render / regression gate")
    ap.add_argument("--render-only", action="store_true",
                    help="render and assert non-blank output only")
    ap.add_argument("--regress", action="store_true",
                    help="render and diff against qa/expected/gui/<platform>")
    ap.add_argument("--update", action="store_true",
                    help="render and overwrite reference images")
    ap.add_argument("--theme", default="obsidian",
                    # high_contrast is an accessibility preset, not a brand theme,
                    # but it is gated exactly like the other three: a preset nobody
                    # renders is a preset nobody has looked at.
                    choices=["obsidian", "titanium", "stealth", "high_contrast"])
    ap.add_argument("--list", action="store_true", help="print view inventory")
    ap.add_argument("--no-render", action="store_true",
                    help="audit/diff the PNGs already in qa/gui (no cargo run)")
    args = ap.parse_args()

    if args.list:
        p = sh([CARGO, "run", "-q", "-p", "hcs-gui-shots", "--bin", "hcs-gui-shots", "--", "--list"])
        print(p.stdout.strip() or p.stderr.strip())
        return p.returncode

    if args.update:
        print("=== HCS GUI reference update ===")
        if not render(args.theme, ["--update-references"]):
            return 1
        return 0

    if args.regress:
        refs = expected_dir()
        print("=== HCS GUI visual regression (theme=%s, references=%s) ==="
              % (args.theme, refs.relative_to(REPO_ROOT)))
        if not args.no_render and not render(args.theme):
            return 1
        if not refs.is_dir():
            print("[ERROR] no reference set for platform '%s'.\n"
                  "        Generate it on this platform with:\n"
                  "          python scripts/verify_gui.py --update --theme %s"
                  % (platform_key(), args.theme), file=sys.stderr)
            return 1
        results = diff_against_reference(args.theme)
    else:
        print("=== HCS GUI headless render audit (theme=%s) ===" % args.theme)
        if not args.no_render and not render(args.theme):
            return 1
        results = audit_render(args.theme)

    if not results:
        print("[ERROR] no GUI renders found in %s" % GUI_DIR, file=sys.stderr)
        return 1

    passed = sum(1 for r in results if r["status"] == "PASS")
    summary = {
        "mode": "regress" if args.regress else "render-only",
        "theme": args.theme,
        "platform": platform_key(),
        "total": len(results),
        "passed": passed,
        "failed": len(results) - passed,
        "overall_status": "PASS" if passed == len(results) else "FAIL",
        "thresholds": {
            "min_entropy": MIN_ENTROPY,
            "min_size_bytes": MIN_SIZE_BYTES,
            "max_mean_diff": MAX_MEAN_DIFF,
            "pixel_tolerance": PIXEL_TOLERANCE,
        },
        "results": results,
    }
    REPORT.parent.mkdir(parents=True, exist_ok=True)
    with open(REPORT, "w", encoding="utf-8") as f:
        json.dump(summary, f, indent=2)

    print("\n%d/%d GUI checks passed. Report: %s"
          % (passed, len(results), REPORT.relative_to(REPO_ROOT)))
    return 0 if passed == len(results) else 1


if __name__ == "__main__":
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8")
    sys.exit(main())
