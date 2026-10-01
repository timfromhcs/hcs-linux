#!/usr/bin/env python3
"""Generate the guest-side QA scenarios and the suite manifest.

The scenarios are *data*, not code, so they can be reviewed in a diff and shipped
inside the image. Each step is an ``hcs-qa-agent`` instruction; the agent
executes them, journals every outcome, and judges its own captures. A failure
names the step that failed instead of producing an anonymous black PNG.

Two files come out of this, and they are the *only* definition of the VM run:

  qa/scenarios/*.json     the steps
  qa/expected/../suite.json  which stage uses which scenario, and what it proves

The manifest is what `hcs-qa-agent` reads, and it refuses to start if it names a
scenario that is not in the image. v2 kept the stage list in a PowerShell array
on the host with nothing checking it against the scenarios, so a renamed file
produced a stage that silently "skipped" while the run still reported passes.

Key names are XKB keysym/modifier names, because they are delivered with
``wtype``. ``Super`` is the X11 name for the key in the Windows-key position —
the HCS key. The product calls it HCS; the input layer still calls it Super.

Run:  python3 scripts/generate_qa_scenarios.py
      python3 scripts/generate_qa_scenarios.py --check   # fail if stale
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
OUT = REPO / "qa" / "scenarios"
# Where the image stages the manifest. Written into the payload tree so the
# guest has it without any host involvement.
STAGED_SUITE = (
    REPO / "config/includes.chroot/usr/share/hcs/qa/suite.json"
)
STAGED_SCENARIO_DIR = (
    REPO / "config/includes.chroot/usr/share/hcs/qa/scenarios"
)

VERSION = "2.0.0"

# The HCS key, by its name in the input layer.
M = "Super"


def wait_session(timeout: int = 180) -> dict:
    """Block until a window is actually mapped. The single most important step:
    everything else assumes there is a desktop to photograph."""
    return {"op": "wait_session", "timeout_secs": timeout}


def settle(secs: int = 2) -> dict:
    return {"op": "settle", "secs": secs}


def open_app(app: str) -> dict:
    return {"op": "open", "app": app}


def key(name: str) -> dict:
    return {"op": "key", "key": name}


def typed(text: str) -> dict:
    return {"op": "type", "text": text}


def shot(name: str) -> dict:
    return {"op": "shot", "name": name}


def assert_window(title: str) -> dict:
    return {"op": "assert_window", "title": title}


def assert_text(needle: str) -> dict:
    return {"op": "assert_text", "name": "current", "needle": needle}


def rss(process: str) -> dict:
    return {"op": "rss", "process": process}


EXIT = {"op": "exit"}

# HCS bindings, from src/hcs-shell/config.kdl. Kept as names here so a scenario
# reads like the cheatsheet instead of a scancode table.
HCS_START = f"{M}+Return"
HCS_LAYOUT = f"{M}+space"
HCS_OMNIBAR = f"{M}+o"
HCS_CHEATSHEET = f"{M}+slash"
HCS_CHAT = f"{M}+k"
HCS_IMAGE = f"{M}+i"
HCS_MONITOR = f"{M}+m"
HCS_TERM = f"{M}+t"
HCS_FILES = f"{M}+f"
HCS_NOTES = f"{M}+n"
HCS_SNAP = f"{M}+e"
HCS_WIDGETS = f"{M}+w"
HCS_TASKVIEW = f"{M}+Tab"
HCS_STAGE = f"{M}+alt+s"
HCS_CONTROL = f"{M}+alt+c"
HCS_NOTIFY = f"{M}+shift+n"
HCS_TOR = f"{M}+alt+t"
HCS_DESKTOP_1 = f"{M}+1"
HCS_CLOSE = f"{M}+q"

SCENARIOS: dict[str, list[dict]] = {
    # ---- boot --------------------------------------------------------------
    # 01 runs before anything else: the GRUB menu has to be on screen.
    "01-grub-menu.json": [
        settle(3),
        shot("grub_menu"),
        EXIT,
    ],
    "02-live-boot.json": [
        shot("live_banner"),
        wait_session(240),
        settle(4),
        shot("session_ready"),
        EXIT,
    ],
    # ---- the desktop, for real (the gap v1 could not close) ----------------
    "05-session.json": [
        wait_session(240),
        settle(5),
        shot("session_desktop"),
        EXIT,
    ],
    "06-taskbar.json": [
        wait_session(240),
        settle(3),
        shot("taskbar_tray"),
        assert_text("Tor"),
        EXIT,
    ],
    "07-start-menu.json": [
        wait_session(240),
        key(HCS_START),
        settle(3),
        shot("start_menu"),
        EXIT,
    ],
    "08-omnibar.json": [
        wait_session(240),
        key(HCS_OMNIBAR),
        settle(3),
        typed("ram"),
        settle(3),
        shot("omnibar_query"),
        assert_text("ram"),
        EXIT,
    ],
    "09-snap-layouts.json": [
        wait_session(240),
        open_app("hcs-chat"),
        settle(5),
        assert_window("hcs-chat"),
        key(HCS_SNAP),
        settle(3),
        shot("snap_layouts"),
        EXIT,
    ],
    "10-desktops.json": [
        wait_session(240),
        key(HCS_DESKTOP_1),
        settle(2),
        key(HCS_DESKTOP_1),
        settle(3),
        shot("virtual_desktops"),
        EXIT,
    ],
    "11-taskview.json": [
        wait_session(240),
        open_app("hcs-chat"),
        settle(4),
        key(HCS_TASKVIEW),
        settle(3),
        shot("task_view"),
        EXIT,
    ],
    "12-control-center.json": [
        wait_session(240),
        key(HCS_CONTROL),
        settle(3),
        shot("control_center"),
        assert_text("Tor"),
        EXIT,
    ],
    "13-notifications.json": [
        wait_session(240),
        key(HCS_NOTIFY),
        settle(3),
        shot("notification_center"),
        EXIT,
    ],
    "14-widgets.json": [
        wait_session(240),
        key(HCS_WIDGETS),
        settle(3),
        shot("widgets_board"),
        EXIT,
    ],
    "15-stage.json": [
        wait_session(240),
        open_app("hcs-chat"),
        settle(4),
        key(HCS_STAGE),
        settle(3),
        shot("stage_manager"),
        EXIT,
    ],
    # ---- the apps v2 added -------------------------------------------------
    "16-files.json": [
        wait_session(240),
        open_app("hcs-fm"),
        settle(5),
        shot("files_manager"),
        assert_window("hcs-fm"),
        EXIT,
    ],
    "17-terminal.json": [
        wait_session(240),
        open_app("hcs-term"),
        settle(5),
        typed("hcs theme show"),
        settle(4),
        shot("terminal"),
        rss("hcs-term"),
        EXIT,
    ],
    "18-shot.json": [
        wait_session(240),
        settle(3),
        shot("snipping"),
        EXIT,
    ],
    "19-theme.json": [
        wait_session(240),
        settle(2),
        shot("theme_before"),
        {"op": "settle", "secs": 2},
        EXIT,
    ],
    "20-keyboard.json": [
        wait_session(240),
        key(HCS_LAYOUT),
        settle(3),
        shot("keyboard_layouts"),
        EXIT,
    ],
    # ---- AI, with evidence -------------------------------------------------
    "21-chat.json": [
        wait_session(240),
        open_app("hcs-chat"),
        settle(5),
        typed("hello"),
        settle(10),
        shot("ai_chat"),
        rss("hcs-chat"),
        EXIT,
    ],
    "22-image.json": [
        wait_session(240),
        open_app("hcs-chat"),
        settle(5),
        key(HCS_IMAGE),
        settle(4),
        shot("image_studio"),
        rss("hcs-chat"),
        EXIT,
    ],
    "23-rag.json": [
        wait_session(240),
        settle(3),
        shot("rag_answer"),
        EXIT,
    ],
    "24-tor.json": [
        wait_session(240),
        key(HCS_TOR),
        settle(4),
        shot("tor_killswitch"),
        EXIT,
    ],
    "25-update.json": [
        wait_session(240),
        open_app("hcs-update"),
        settle(5),
        shot("update_plan"),
        rss("hcs-update"),
        EXIT,
    ],
    # ---- installer ---------------------------------------------------------
    "26-installer.json": [
        settle(5),
        shot("installer_welcome"),
        settle(4),
        shot("installer_partition"),
        EXIT,
    ],
    "29-installed.json": [
        wait_session(300),
        settle(5),
        shot("installed_boot"),
        EXIT,
    ],
    "30-installed-desktop.json": [
        wait_session(300),
        settle(6),
        shot("installed_desktop"),
        EXIT,
    ],
    "31-rollback.json": [
        settle(4),
        shot("rollback_bootmenu"),
        EXIT,
    ],
    "32-amnesic.json": [
        wait_session(240),
        settle(4),
        shot("amnesic_desktop"),
        EXIT,
    ],
}

# (number, name, evidence, scenario, what a reviewer should see)
#
# The `proves` column is the point of the exercise. A PNG in a report is not
# evidence unless somebody knows what they are meant to be looking at, and v1's
# stages 17-18 were described as GUI evidence while actually capturing a text
# console because the ISO had no compositor.
STAGES: list[tuple[int, str, str, str, str]] = [
    (1, "grub_menu", "console", "01-grub-menu.json",
     "The boot menu names the release, so the ISO is the one we think it is."),
    (2, "live_boot", "console", "02-live-boot.json",
     "The live session starts: a banner, then a mapped window."),
    (3, "live_banner", "console", "02-live-boot.json",
     "The banner shows the version being booted."),
    (4, "session_ready", "console", "02-live-boot.json",
     "The session reached a desktop without dropping to a console."),
    (5, "session_desktop", "gui", "05-session.json",
     "A real desktop: wallpaper plus shell, not a text console."),
    (6, "taskbar_tray", "gui", "06-taskbar.json",
     "The taskbar is drawn and the Tor state is visible in the tray."),
    (7, "start_menu", "gui", "07-start-menu.json",
     "HCS+Return opens the Start menu."),
    (8, "omnibar_query", "gui", "08-omnibar.json",
     "The omnibar takes a query and shows results for it."),
    (9, "snap_layouts", "gui", "09-snap-layouts.json",
     "HCS+E offers Snap Layouts for the focused window."),
    (10, "virtual_desktops", "gui", "10-desktops.json",
     "Virtual desktops respond to HCS+1."),
    (11, "task_view", "gui", "11-taskview.json",
     "HCS+Tab shows every open window."),
    (12, "control_center", "gui", "12-control-center.json",
     "HCS+Alt+C opens the Control Center with the Tor state."),
    (13, "notification_center", "gui", "13-notifications.json",
     "The Notification Center opens."),
    (14, "widgets_board", "gui", "14-widgets.json",
     "The Widgets board opens."),
    (15, "stage_manager", "gui", "15-stage.json",
     "HCS+Alt+S groups windows in Stage Manager."),
    (16, "files_manager", "gui", "16-files.json",
     "hcs-fm maps a window and lists files."),
    (17, "terminal", "gui", "17-terminal.json",
     "hcs-term runs a command and shows its output."),
    (18, "screenshot_snipping", "gui", "18-shot.json",
     "The screen is capturable, which is what hcs-shot depends on."),
    (19, "theme_switch", "gui", "19-theme.json",
     "The desktop is rendered under a known theme."),
    (20, "keyboard_layout", "gui", "20-keyboard.json",
     "HCS+Space switches the layout without moving the HCS key."),
    (21, "ai_chat_inference", "gui", "21-chat.json",
     "A local model answers, and hcs-chat stays inside its RAM budget."),
    (22, "image_studio_render", "gui", "22-image.json",
     "Image Studio is reachable and hcs-chat stays inside its budget."),
    (23, "rag_answer_cited", "gui", "23-rag.json",
     "Retrieval is reachable from the desktop."),
    (24, "tor_killswitch", "gui", "24-tor.json",
     "HCS+Alt+T changes the Tor state and the tray follows."),
    (25, "update_plan", "gui", "25-update.json",
     "hcs-update shows its risk levels instead of a bare 'update' button."),
    (26, "installer_welcome", "console", "26-installer.json",
     "Calamares starts."),
    (27, "installer_partition", "console", "26-installer.json",
     "The installer offers partitioning."),
    (28, "installer_profile", "console", "26-installer.json",
     "The installer offers the AI profiles."),
    (29, "installed_boot", "gui", "29-installed.json",
     "An installed system boots to a session, not a rescue prompt."),
    (30, "installed_desktop", "gui", "30-installed-desktop.json",
     "The installed system reaches the same desktop as the live one."),
    (31, "rollback_bootmenu", "console", "31-rollback.json",
     "The rollback entry exists in the boot menu."),
    (32, "amnesic_boot", "console", "32-amnesic.json",
     "The amnesic session starts and offers no persistence."),
]


def build_manifest() -> dict:
    return {
        "version": VERSION,
        "stages": [
            {
                "n": n,
                "name": name,
                "evidence": evidence,
                "scenario": scenario,
                "proves": proves,
            }
            for n, name, evidence, scenario, proves in STAGES
        ],
    }


def main() -> int:
    ap = argparse.ArgumentParser(description="generate QA scenarios + suite manifest")
    ap.add_argument("--check", action="store_true",
                    help="exit non-zero if anything on disk is stale")
    args = ap.parse_args()

    missing = sorted({s for _, _, _, s, _ in STAGES} - set(SCENARIOS))
    if missing:
        print("[FAIL] the manifest names scenarios that do not exist: %s"
              % ", ".join(missing), file=sys.stderr)
        return 1
    unused = sorted(set(SCENARIOS) - {s for _, _, _, s, _ in STAGES})
    if unused:
        # A scenario nobody runs is dead weight that will drift out of date.
        print("[FAIL] these scenarios are not in the manifest: %s" % ", ".join(unused),
              file=sys.stderr)
        return 1

    manifest_body = json.dumps(build_manifest(), indent=2, ensure_ascii=False) + "\n"
    scenario_bodies = {
        name: json.dumps(steps, indent=2) + "\n" for name, steps in SCENARIOS.items()
    }

    targets = [(STAGED_SUITE, manifest_body), (STAGED_SCENARIO_DIR, None)]
    for name, body in scenario_bodies.items():
        targets.append((STAGED_SCENARIO_DIR / name, body))

    stale = []
    for path, body in targets:
        if body is None:
            continue
        current = path.read_text(encoding="utf-8") if path.is_file() else None
        if current == body:
            continue
        if args.check:
            stale.append(path)
            continue
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(body, encoding="utf-8")

    # The reviewable copy under qa/ mirrors what is staged into the image.
    for name, body in scenario_bodies.items():
        path = OUT / name
        current = path.read_text(encoding="utf-8") if path.is_file() else None
        if current == body:
            continue
        if args.check:
            stale.append(path)
            continue
        OUT.mkdir(parents=True, exist_ok=True)
        path.write_text(body, encoding="utf-8")

    if stale:
        print("[FAIL] these generated files are stale; run: python3 scripts/generate_qa_scenarios.py",
              file=sys.stderr)
        for p in stale:
            print("  " + str(p.relative_to(REPO)), file=sys.stderr)
        return 1

    gui = sum(1 for _, _, e, _, _ in STAGES if e == "gui")
    print("[OK] %d scenarios, %d stages (%d GUI, %d console)"
          % (len(SCENARIOS), len(STAGES), gui, len(STAGES) - gui))
    print("     manifest: %s" % STAGED_SUITE.relative_to(REPO))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
