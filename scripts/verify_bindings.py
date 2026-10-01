#!/usr/bin/env python3
"""
HCS Linux — shell binding gate.

Reads the niri config (the source of truth: src/hcs-shell/config.kdl) and
checks three things that are cheap to check here and expensive to discover on
a live desktop:

  1. No duplicate key combination. niri refuses to start with a config that
     binds the same combo twice, and the v2 config really did: Mod+E was both
     "open the file manager" and "open the control center", and Mod+Alt+T was
     both "floating/tiling" and "Tor kill switch". Nothing tests this by
     default, so the session simply never came up.

  2. Every `spawn "hcs" <subcommand…>` target actually exists in the CLI. A
     binding that calls a subcommand nobody wrote is a dead key that looks
     correct in the file.

  3. Every QML surface a binding opens exists on disk.

Usage:
  python scripts/verify_bindings.py [--config PATH] [--quiet]

Exit code 0 = every binding resolves. Non-zero = at least one does not.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
DEFAULT_CONFIG = REPO_ROOT / "src/hcs-shell/config.kdl"
SHELL_DIR = REPO_ROOT / "config/includes.chroot/usr/share/hcs/shell"
REPORT = REPO_ROOT / "qa/reports/bindings.json"

# `Mod` is niri's name for the HCS key (Super in niri's own vocabulary). The
# product name never leaks into the compositor config.
BIND_RE = re.compile(r"^\s{4}([A-Za-z0-9+]+)\s+\{(.*)$")
SPAWN_RE = re.compile(r'spawn\s+"([^"]+)"((?:\s+"[^"]*")*)')
QML_RE = re.compile(r'([A-Za-z0-9_]+\.qml)')

# Actions niri executes itself. Anything else in a binding body has to be a
# spawn, so a typo cannot hide behind an unrecognised keyword.
NIRI_ACTIONS = {
    "close-window", "focus-column-left", "focus-column-right",
    "move-column-left", "move-column-right", "focus-workspace",
    "toggle-overview", "screenshot-screen", "spawn",
}


def parse_bindings(config: Path) -> list[dict]:
    text = config.read_text(encoding="utf-8")
    m = re.search(r"^\s*binds\s*\{(.*?)^\s*\}", text, re.S | re.M)
    if not m:
        raise SystemExit("no binds { … } block found in %s" % config)
    out: list[dict] = []
    for line in m.group(1).splitlines():
        b = BIND_RE.match(line)
        if not b:
            continue
        out.append({"combo": b.group(1), "body": b.group(2).strip()})
    return out


def cli_subcommands() -> set[str]:
    """Top-level `hcs` subcommands, read from the real binary.

    Asking the binary beats pattern-matching clap's derive attributes: if the
    CLI is not built, this gate says so instead of silently passing.
    """
    cargo = "cargo.exe" if sys.platform == "win32" else "cargo"
    p = subprocess.run(
        [cargo, "run", "-q", "--bin", "hcs", "--", "--help"],
        cwd=REPO_ROOT, capture_output=True, text=True,
    )
    if p.returncode != 0:
        return set()
    names: set[str] = set()
    for line in p.stdout.splitlines():
        s = line.strip()
        if not s or s.endswith("Commands:") or s.endswith("Options:"):
            continue
        tok = s.split()[0]
        if re.fullmatch(r"[a-z][a-z0-9-]*", tok):
            names.add(tok)
    return names


def display_path(p: Path) -> str:
    """Repo-relative when possible, absolute otherwise (a --config outside the
    repo must not crash the gate)."""
    try:
        return str(p.relative_to(REPO_ROOT))
    except ValueError:
        return str(p)


def main() -> int:
    ap = argparse.ArgumentParser(description="HCS shell binding gate")
    ap.add_argument("--config", default=str(DEFAULT_CONFIG))
    ap.add_argument("--quiet", action="store_true")
    args = ap.parse_args()

    config = Path(args.config)
    bindings = parse_bindings(config)
    subs = cli_subcommands()
    if not subs:
        print("[ERROR] could not read `hcs --help`; build the CLI first",
              file=sys.stderr)
        return 1

    problems: list[str] = []
    seen: dict[str, int] = {}
    resolved: list[dict] = []

    # The docs portal serves a generated cheatsheet; if it is stale, the user
    # reads shortcuts that do not exist.
    doc = subprocess.run(
        [sys.executable, str(REPO_ROOT / "scripts/generate_cheatsheet.py"), "--check"],
        cwd=REPO_ROOT, capture_output=True, text=True,
    )
    if doc.returncode != 0:
        problems.append(
            "cheatsheet.json is out of date: "
            + (doc.stdout.strip().splitlines() or [""])[-1])

    for b in bindings:
        combo, body = b["combo"], b["body"]
        seen[combo] = seen.get(combo, 0) + 1

        spawns = SPAWN_RE.findall(body)
        if not spawns and not any(a in body for a in NIRI_ACTIONS):
            problems.append(f"{combo}: binding body resolves to nothing: {body!r}")

        for binary, raw_args in spawns:
            argv = re.findall(r'"([^"]*)"', raw_args)
            if binary == "hcs":
                if not argv:
                    problems.append(f"{combo}: bare `hcs` with no subcommand")
                    continue
                if argv[0] not in subs:
                    problems.append(
                        f"{combo}: `hcs {argv[0]}` is not a subcommand of the CLI")
            elif binary == "quickshell":
                for qml in QML_RE.findall(" ".join(argv)):
                    if not (SHELL_DIR / qml).is_file():
                        problems.append(
                            f"{combo}: shell surface {qml} is not staged")
            elif not binary.startswith("/") and not binary.startswith("hcs-"):
                problems.append(f"{combo}: unknown binary {binary!r}")
            resolved.append({"combo": combo, "target": f"{binary} {' '.join(argv)}".strip()})

    for combo, n in seen.items():
        if n > 1:
            problems.append(
                f"{combo}: bound {n} times — niri refuses to load a config "
                f"with a duplicate binding, so the whole session dies")

    if not args.quiet:
        print("=== HCS shell binding gate ===")
        print(f"  config   : {display_path(config)}")
        print(f"  bindings : {len(bindings)}")
        print(f"  surfaces : {len(list(SHELL_DIR.glob('*.qml')))} staged")
        for r in resolved:
            print(f"  [OK]   {r['combo']:<18} {r['target']}")
        for p in problems:
            print(f"  [FAIL] {p}")
        if not problems:
            print("  every binding resolves to a real command, and the "
                  "cheatsheet matches")

    REPORT.parent.mkdir(parents=True, exist_ok=True)
    with open(REPORT, "w", encoding="utf-8") as f:
        json.dump({
            "config": display_path(config),
            "bindings": len(bindings),
            "problems": problems,
            "overall_status": "PASS" if not problems else "FAIL",
        }, f, indent=2)

    print("\n%d/%d bindings resolved. Report: %s"
          % (len(bindings) - len({p.split(':')[0] for p in problems}),
             len(bindings), display_path(REPORT)))
    return 0 if not problems else 1


if __name__ == "__main__":
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8")
    sys.exit(main())
