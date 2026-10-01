#!/usr/bin/env python3
"""
HCS Linux — generate cheatsheet.json from the niri binding config.

`src/hcs-shell/config.kdl` is the single source of truth for what a key does.
The docs portal serves `cheatsheet.json`, and when the two disagree the user
reads a shortcut that does nothing — which is exactly the v1 class of bug where
the cheatsheet advertised bindings the compositor never had.

So the JSON is *generated*, and `scripts/verify_bindings.py --check-docs`
fails if the committed file is stale. Editing means editing config.kdl.

Usage:
  python scripts/generate_cheatsheet.py            # write the JSON
  python scripts/generate_cheatsheet.py --check    # fail if out of date
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
CONFIG = REPO_ROOT / "src/hcs-shell/config.kdl"
OUT = REPO_ROOT / "config/includes.chroot/usr/share/hcs/docs/cheatsheet.json"

BIND_RE = re.compile(r"^\s{4}([A-Za-z0-9+]+)\s+\{(.*)$")
SPAWN_RE = re.compile(r'spawn\s+"([^"]+)"((?:\s+"[^"]*")*)')

# Which section a binding belongs in, and what to call it. Keyed on the spawn
# target, because the combo alone does not say what it does.
LABELS: dict[tuple[str, ...], tuple[str, str]] = {
    ("hcs", "settings", "next-keyboard"): ("system", "Cycle Keyboard Layout"),
    ("hcs", "settings", "reduce-motion", "toggle"): ("accessibility", "Reduce Motion"),
    ("hcs", "settings", "scale", "up"): ("accessibility", "Larger Text"),
    ("hcs", "settings", "scale", "down"): ("accessibility", "Smaller Text"),
    ("hcs", "theme", "set", "high_contrast"): ("accessibility", "High Contrast Theme"),
    ("hcs", "security", "tor", "toggle"): ("privacy", "Tor Kill Switch"),
    ("hcs", "security", "vault"): ("privacy", "Vault (LUKS)"),
    ("hcs", "privacy", "amnesic", "toggle"): ("privacy", "Amnesic Session"),
    ("hcs", "recall"): ("privacy", "Recall Timeline (local, encrypted)"),
    ("hcs", "window", "mode", "toggle"): ("system", "Floating / Tiling Mode"),
    ("hcs", "ask", "--selection"): ("ai", "Ask about Selection"),
    ("hcs", "rag", "query"): ("ai", "Search the Manuals"),
    ("hcs-search",): ("system", "Omnibar (find and act)"),
    ("hcs-chat",): ("apps", "HCS Chat"),
    ("hcs-chat", "--gui"): ("apps", "HCS Chat"),
    ("hcs-chat", "--gui", "--tab", "image"): ("apps", "Image Studio"),
    ("hcs-monitor",): ("apps", "System Monitor"),
    ("hcs-monitor", "--gui"): ("apps", "System Monitor"),
    ("hcs-term",): ("apps", "Terminal"),
    ("hcs-fm",): ("apps", "Files"),
    ("hcs-notes",): ("apps", "Notes"),
    ("hcs-settings",): ("apps", "Settings"),
    ("hcs-update", "--gui"): ("apps", "Software Updates"),
    ("hcs-rag-ingest", "--gui"): ("apps", "Knowledge Ingest (RAG)"),
    ("hcs-shot", "--region"): ("apps", "Screenshot (region)"),
    ("hcs-shot", "--screen"): ("apps", "Screenshot (screen)"),
    ("quickshell", "start_menu.qml"): ("system", "Start Menu"),
    ("quickshell", "control_drawer.qml"): ("system", "HCS Menu (install / setup / update)"),
    ("quickshell", "control_center.qml"): ("system", "Control Center"),
    ("quickshell", "notification_center.qml"): ("system", "Notification Center"),
    ("quickshell", "widgets.qml"): ("system", "Widgets Board"),
    ("quickshell", "snap_flyout.qml"): ("system", "Snap Layouts"),
    ("quickshell", "taskview.qml"): ("system", "Task View"),
    ("quickshell", "stage.qml"): ("system", "Stage Manager"),
    ("quickshell", "cheatsheet.qml"): ("system", "All Keyboard Shortcuts (this HUD)"),
    ("quickshell", "clipboard.qml"): ("privacy", "Redacted Clipboard"),
    ("close-window",): ("system", "Close Window"),
    ("focus-column-left",): ("system", "Focus Window Left"),
    ("focus-column-right",): ("system", "Focus Window Right"),
    ("move-column-left",): ("system", "Move Window Left"),
    ("move-column-right",): ("system", "Move Window Right"),
    ("focus-workspace",): ("system", "Jump to Virtual Desktop"),
}

# The `Mod` prefix is niri's word for the HCS key. Users never see it.
SECTION_ORDER = ["system", "apps", "privacy", "ai", "accessibility"]


def describe(combo: str, body: str) -> tuple[str, str] | None:
    """(group, human label) for one binding, or None if it is unlabelled."""
    niri = re.match(r"^([a-z-]+)\b", body)
    if niri and niri.group(1) != "spawn":
        key = (niri.group(1),)
        if key in LABELS:
            return LABELS[key]

    m = SPAWN_RE.search(body)
    if not m:
        return None
    argv = re.findall(r'"([^"]*)"', m.group(2))
    if m.group(1) == "quickshell" and argv:
        qml = next((a for a in argv if a.endswith(".qml")), None)
        if qml is None:
            return None
        return LABELS.get(("quickshell", Path(qml).name))
    return LABELS.get(tuple([m.group(1)] + argv))


def collect() -> list[dict]:
    text = CONFIG.read_text(encoding="utf-8")
    block = re.search(r"^\s*binds\s*\{(.*?)^\s*\}", text, re.S | re.M)
    if not block:
        raise SystemExit("no binds { … } block in %s" % CONFIG)

    out: list[dict] = []
    for line in block.group(1).splitlines():
        b = BIND_RE.match(line)
        if not b:
            continue
        combo = b.group(1).replace("Mod", "HCS")
        d = describe(b.group(1), b.group(2).strip())
        if d is None:
            # An unlabelled binding is a documentation gap, not a silent drop:
            # it still appears, with the raw combo as its name.
            out.append({"action": combo, "keys": combo, "group": "system",
                        "_unlabelled": True})
            continue
        group, label = d
        out.append({"action": label, "keys": combo, "group": group})
    return out


def build() -> dict:
    entries = collect()
    groups: dict[str, list[dict]] = {}
    for e in entries:
        groups.setdefault(e["group"], []).append(
            {"action": e["action"], "keys": e["keys"], "group": e["group"]})
    doc = {
        "_meta": {
            "version": "2.0.0",
            "generated_from": "src/hcs-shell/config.kdl",
            "generator": "scripts/generate_cheatsheet.py",
            "note": ("The Windows-key position is the HCS key. It is a modifier, "
                     "so no keyboard layout can move it. Layout changes never "
                     "remap these bindings."),
            "layouts": ["de (QWERTZ, default)", "us (QWERTY)", "fr (AZERTY)",
                        "es", "it", "gb"],
            "layout_switch": "HCS+Space",
        }
    }
    for g in SECTION_ORDER:
        if g in groups:
            doc[g] = groups[g]
    for g, items in groups.items():
        doc.setdefault(g, items)
    return doc


def main() -> int:
    ap = argparse.ArgumentParser(description="generate cheatsheet.json from config.kdl")
    ap.add_argument("--check", action="store_true",
                    help="exit non-zero if the committed JSON is stale")
    args = ap.parse_args()

    doc = build()
    body = json.dumps(doc, indent=2, ensure_ascii=False) + "\n"

    if args.check:
        if not OUT.is_file():
            print("[FAIL] %s does not exist" % OUT.relative_to(REPO_ROOT))
            return 1
        if OUT.read_text(encoding="utf-8") != body:
            print("[FAIL] %s is stale — run: python scripts/generate_cheatsheet.py"
                  % OUT.relative_to(REPO_ROOT))
            return 1
        total = sum(len(v) for k, v in doc.items() if not k.startswith("_"))
        print("[OK] cheatsheet.json matches config.kdl (%d bindings)" % total)
        return 0

    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(body, encoding="utf-8")
    total = sum(len(v) for k, v in doc.items() if not k.startswith("_"))
    unlabelled = [e for e in collect() if e.get("_unlabelled")]
    print("[OK] wrote %s (%d bindings)" % (OUT.relative_to(REPO_ROOT), total))
    for e in unlabelled:
        print("[WARN] no label for %s — it will appear under its raw combo" % e["combo"]
              if "combo" in e else "[WARN] unlabelled binding: %s" % e["keys"])
    return 0


if __name__ == "__main__":
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8")
    sys.exit(main())
