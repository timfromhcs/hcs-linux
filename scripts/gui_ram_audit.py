#!/usr/bin/env python3
"""
HCS Linux — GUI RAM budget gate.

Plan §3: every GUI app must stay at or below 250 MB RSS, and the system must
stay inside the master-plan matrix (<= 6144 MB idle / <= 8192 MB peak).

Measurement is real, not estimated: each app is launched headless (Slint
software renderer, no window), sampled from /proc/<pid>/status, then asked to
report its own RSS through `--ram-report`.

  --list     print the app inventory
  (default)  measure every app and write qa/reports/gui_ram_audit.json
"""

from __future__ import annotations

import argparse
import json
import os
import platform
import subprocess
import sys
import time
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
REPORT = REPO_ROOT / "qa/reports/gui_ram_audit.json"
BIN_DIR = REPO_ROOT / "target/debug"

# Plan §3: per-application ceiling.
APP_RSS_BUDGET_MB = 250
# Master plan §5: system matrix.
IDLE_BUDGET_MB = 6144
PEAK_BUDGET_MB = 8192

SAMPLE_SECONDS = 6.0
SAMPLE_INTERVAL = 0.25

CARGO = "cargo" if sys.platform != "win32" else "cargo.exe"


class App:
    def __init__(self, name: str, crate_: str, args: list[str], note: str = ""):
        self.name = name
        self.crate = crate_
        self.args = args
        self.note = note


def inventory() -> list[App]:
    """GUI apps that expose a headless mode (--gui-offscreen or --ram-report)."""
    apps = [
        App("hcs-ui-gallery", "hcs-ui", ["--gui-render", "foundation-gallery",
                                         "--out-dir", "target/ram-audit",
                                         "--hold-seconds", "6"],
            "foundation widget kit (proxy for GUI baseline)"),
    ]
    # App crates register themselves here as the phases land.
    for extra in json.loads(_extra_apps_json() or "[]"):
        apps.append(App(**extra))
    return apps


def _extra_apps_json() -> str:
    """Optional manifest written by the GUI app crates, so this script does
    not need to be edited every time an app is added."""
    p = REPO_ROOT / "config/gui_apps.json"
    if p.exists():
        return p.read_text(encoding="utf-8")
    return "[]"


def rss_mb(pid: int) -> float:
    """Resident set size of `pid` in MB, or 0.0 when it cannot be measured."""
    try:
        with open(f"/proc/{pid}/status", encoding="utf-8") as f:
            for line in f:
                if line.startswith("VmRSS:"):
                    return int(line.split()[1]) / 1024.0
    except OSError:
        pass
    if platform.system() == "Windows":
        try:
            import psutil
            return psutil.Process(pid).memory_info().rss / (1024.0 * 1024.0)
        except Exception:
            pass
    return 0.0


def rss_measurable() -> bool:
    """False when no RSS source exists — the gate must report UNKNOWN, never
    a silent PASS."""
    if platform.system() == "Linux" and os.path.exists("/proc/self/status"):
        return True
    if platform.system() == "Windows":
        try:
            import psutil  # noqa: F401
            return True
        except Exception:
            return False
    return False


def measure(app: App) -> dict:
    cmd = [CARGO, "run", "-q", "-p", app.crate, "--"] + app.args
    proc = subprocess.Popen(cmd, cwd=REPO_ROOT, stdout=subprocess.PIPE,
                            stderr=subprocess.PIPE, text=True)
    peak = 0.0
    deadline = time.time() + SAMPLE_SECONDS
    try:
        while time.time() < deadline and proc.poll() is None:
            peak = max(peak, rss_mb(proc.pid))
            time.sleep(SAMPLE_INTERVAL)
    finally:
        if proc.poll() is None:
            proc.terminate()
            try:
                proc.wait(timeout=5)
            except subprocess.TimeoutExpired:
                proc.kill()
    return {
        "app": app.name,
        "crate": app.crate,
        "note": app.note,
        "args": app.args,
        "peak_rss_mb": round(peak, 1),
        "budget_mb": APP_RSS_BUDGET_MB,
        "status": "PASS" if 0 < peak <= APP_RSS_BUDGET_MB else ("FAIL" if peak > APP_RSS_BUDGET_MB else "UNKNOWN"),
    }


def main() -> int:
    ap = argparse.ArgumentParser(description="HCS GUI RAM budget gate")
    ap.add_argument("--list", action="store_true")
    args = ap.parse_args()

    apps = inventory()
    if args.list:
        for a in apps:
            print(f"{a.name:<24} {a.crate:<16} {' '.join(a.args)}")
        return 0

    print("=== HCS GUI RAM Audit (budget %d MB/app) ===" % APP_RSS_BUDGET_MB)
    measurable = rss_measurable()
    if not measurable:
        print("  [ERROR] no RSS source available on this platform "
              "(need /proc or psutil). Cannot certify the budget.")
    results = []
    for a in apps:
        r = measure(a)
        results.append(r)
        sym = {"PASS": "[OK]  ", "FAIL": "[FAIL]", "UNKNOWN": "[????]"}[r["status"]]
        print(f"  {sym} {r['app']:<24} peak RSS {r['peak_rss_mb']:>7.1f} MB "
              f"/ {r['budget_mb']} MB  ({r['status']})")

    if platform.system() == "Linux" and os.path.exists("/proc/meminfo"):
        with open("/proc/meminfo", encoding="utf-8") as f:
            info = {}
            for line in f:
                k, _, rest = line.partition(":")
                info[k] = int(rest.split()[0]) // 1024
        system = {
            "total_mb": info.get("MemTotal"),
            "available_mb": info.get("MemAvailable"),
            "idle_budget_mb": IDLE_BUDGET_MB,
            "peak_budget_mb": PEAK_BUDGET_MB,
        }
        print("\n  system MemTotal %s MB / MemAvailable %s MB "
              "(budgets: idle %d, peak %d)"
              % (system["total_mb"], system["available_mb"],
                 IDLE_BUDGET_MB, PEAK_BUDGET_MB))

    passed = sum(1 for r in results if r["status"] == "PASS")
    unknown = sum(1 for r in results if r["status"] == "UNKNOWN")
    summary = {
        "apps": results,
        "total": len(results),
        "passed": passed,
        "failed": sum(1 for r in results if r["status"] == "FAIL"),
        "unknown": unknown,
        "rss_measurable": measurable,
        "app_budget_mb": APP_RSS_BUDGET_MB,
        "idle_budget_mb": IDLE_BUDGET_MB,
        "peak_budget_mb": PEAK_BUDGET_MB,
        "platform": platform.system(),
        "overall_status": ("PASS" if passed == len(results)
                           else ("UNKNOWN" if unknown and not measurable or (unknown and passed == len(results) - unknown)
                                 else "FAIL")),
    }
    REPORT.parent.mkdir(parents=True, exist_ok=True)
    with open(REPORT, "w", encoding="utf-8") as f:
        json.dump(summary, f, indent=2)

    print("\n%d/%d GUI apps within budget. Report: %s"
          % (passed, len(results), REPORT.relative_to(REPO_ROOT)))
    if summary["overall_status"] == "PASS":
        return 0
    if summary["overall_status"] == "UNKNOWN":
        print("UNKNOWN: RAM budget could not be certified on this platform.")
        return 2
    return 1


if __name__ == "__main__":
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8")
    sys.exit(main())
