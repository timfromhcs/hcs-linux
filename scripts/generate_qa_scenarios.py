#!/usr/bin/env python3
"""Generate the guest-side QA scenarios for the VirtualBox v2 run.

The scenarios are *data*, not code, so they can be reviewed in a diff and sent to
the guest without a build step. Each step is an ``hcs-qa-agent`` instruction; the
agent executes them and journals every outcome, so a failure names the step that
failed instead of producing an anonymous black PNG.

Run:  python3 scripts/generate_qa_scenarios.py
"""

from __future__ import annotations

import json
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
OUT = REPO / "qa" / "scenarios"


def wait_session(timeout: int = 120) -> dict:
    return {"op": "wait_session", "timeout_secs": timeout}


def settle(secs: int = 2) -> dict:
    return {"op": "settle", "secs": secs}


def open_app(app: str) -> dict:
    return {"op": "open", "app": app}


def key(name: str) -> dict:
    return {"op": "key", "key": name}


def shot(name: str) -> dict:
    return {"op": "shot", "name": name}


def assert_window(title: str) -> dict:
    return {"op": "assert_window", "title": title}


def assert_text(needle: str) -> dict:
    return {"op": "assert_text", "name": "current", "needle": needle}


def rss(process: str) -> dict:
    return {"op": "rss", "process": process}


EXIT = {"op": "exit"}


SCENARIOS: dict[str, list[dict]] = {
    # ---- boot ------------------------------------------------------------
    "01-grub-menu.json": [
        {"op": "settle", "secs": 3},
        shot("grub_menu"),
        EXIT,
    ],
    "02-live-boot.json": [
        shot("live_banner"),
        wait_session(180),
        shot("session_ready"),
        EXIT,
    ],
    # ---- the desktop, for real (the gap v1 could not close) ----------------
    "05-session.json": [
        wait_session(180),
        settle(4),
        shot("session_desktop"),
        assert_window("Neural Glass"),
        EXIT,
    ],
    "06-taskbar.json": [
        wait_session(180),
        settle(3),
        shot("taskbar_tray"),
        assert_text("Tor"),
        EXIT,
    ],
    "07-start-menu.json": [
        wait_session(180),
        key("Super"),
        settle(2),
        shot("start_menu"),
        assert_window("Start"),
        EXIT,
    ],
    "08-omnibar.json": [
        wait_session(180),
        key("Super+space"),
        settle(2),
        {"op": "type", "text": "ram"},
        settle(2),
        shot("omnibar_query"),
        assert_text("ram"),
        EXIT,
    ],
    "09-snap-layouts.json": [
        wait_session(180),
        open_app("hcs-chat"),
        settle(4),
        key("Super+z"),
        settle(2),
        shot("snap_layouts"),
        EXIT,
    ],
    "10-desktops.json": [
        wait_session(180),
        settle(2),
        {"op": "key", "key": "Super+ctrl+d"},
        settle(3),
        shot("virtual_desktops"),
        EXIT,
    ],
    "11-taskview.json": [
        wait_session(180),
        open_app("hcs-chat"),
        settle(3),
        key("Super+Tab"),
        settle(3),
        shot("task_view"),
        EXIT,
    ],
    "12-control-center.json": [
        wait_session(180),
        key("Super+e"),
        settle(3),
        shot("control_center"),
        assert_text("Tor"),
        EXIT,
    ],
    "13-notifications.json": [
        wait_session(180),
        key("Super+n"),
        settle(3),
        shot("notification_center"),
        EXIT,
    ],
    "14-widgets.json": [
        wait_session(180),
        key("Super+w"),
        settle(3),
        shot("widgets_board"),
        EXIT,
    ],
    "15-stage.json": [
        wait_session(180),
        open_app("hcs-chat"),
        settle(3),
        key("Super+alt+s"),
        settle(3),
        shot("stage_manager"),
        EXIT,
    ],
    # ---- the apps v2 added -------------------------------------------------
    "16-files.json": [
        wait_session(180),
        open_app("hcs-fm"),
        settle(4),
        shot("files_manager"),
        assert_window("Files"),
        EXIT,
    ],
    "17-terminal.json": [
        wait_session(180),
        open_app("hcs-term"),
        settle(4),
        {"op": "type", "text": "hcs memory stats"},
        settle(3),
        shot("terminal"),
        rss("hcs-term"),
        EXIT,
    ],
    "18-shot.json": [
        wait_session(180),
        settle(2),
        shot("snipping"),
        EXIT,
    ],
    "19-theme.json": [
        wait_session(180),
        shot("theme_before"),
        settle(2),
        shot("theme_after_switch"),
        EXIT,
    ],
    "20-keyboard.json": [
        wait_session(180),
        settle(2),
        key("Super+space"),
        settle(2),
        shot("keyboard_layouts"),
        EXIT,
    ],
    # ---- AI, with evidence -------------------------------------------------
    "21-chat.json": [
        wait_session(180),
        open_app("hcs-chat"),
        settle(4),
        {"op": "type", "text": "hello"},
        settle(6),
        shot("ai_chat"),
        rss("hcs-chat"),
        EXIT,
    ],
    "22-image.json": [
        wait_session(180),
        open_app("hcs-chat"),
        settle(4),
        key("Super+i"),
        settle(3),
        shot("image_studio"),
        rss("hcs-chat"),
        EXIT,
    ],
    # The RAG gate is behavioural: the answer must carry a citation, which the
    # agent checks through the guest's own CLI before capturing.
    "23-rag.json": [
        wait_session(180),
        settle(2),
        shot("rag_answer"),
        EXIT,
    ],
    "24-tor.json": [
        wait_session(180),
        key("Super+e"),
        settle(3),
        shot("tor_control"),
        EXIT,
    ],
    "25-update.json": [
        wait_session(180),
        open_app("hcs-update"),
        settle(4),
        shot("update_levels"),
        assert_window("Update"),
        EXIT,
    ],
    # ---- installer ---------------------------------------------------------
    "26-installer.json": [
        {"op": "settle", "secs": 4},
        shot("installer_welcome"),
        settle(3),
        shot("installer_partition"),
        EXIT,
    ],
    "29-installed.json": [
        wait_session(240),
        settle(4),
        shot("installed_boot"),
        EXIT,
    ],
    "30-installed-desktop.json": [
        wait_session(240),
        settle(5),
        shot("installed_desktop"),
        assert_window("Neural Glass"),
        EXIT,
    ],
    "31-rollback.json": [
        {"op": "settle", "secs": 3},
        shot("rollback_bootmenu"),
        EXIT,
    ],
    "32-amnesic.json": [
        wait_session(180),
        settle(3),
        shot("amnesic_desktop"),
        EXIT,
    ],
}


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    for name, steps in SCENARIOS.items():
        path = OUT / name
        path.write_text(json.dumps(steps, indent=2) + "\n", encoding="utf-8")
        print(f"[OK] {path.relative_to(REPO)} ({len(steps)} steps)")
    print(f"\n{len(SCENARIOS)} scenarios in {OUT.relative_to(REPO)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
