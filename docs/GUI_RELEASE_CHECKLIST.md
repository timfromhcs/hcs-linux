# HCS Linux — GUI Release Checklist (Gate 7, manual)

Plan: `docs/GUI_BUILD_PLAN.md` §4 step 8. Tick every box before publishing a
release that contains GUI changes. The automated gates (unit, headless render,
visual regression, RAM audit, Wayland) run in CI; this is the human pass.

## 1. Look & Feel (one screen per view)
- [ ] Taskbar: bottom-anchored, Start monogram opens the Start Menu, active app
      highlighted with a cyan underline, tray shows model + RAM + Tor pill.
- [ ] Start Menu: omnibar filters apps/files/memory, pinned tiles launch, power
      actions present.
- [ ] `Super+/` cheatsheet overlay lists every binding from `cheatsheet.json`.

## 2. Themes
- [ ] Obsidian (default), Titanium and Stealth all apply **live** and look right.
- [ ] `hcs-settings` persists the choice to `$XDG_CONFIG_HOME/hcs/theme.json`.
- [ ] A GUI app started afterwards comes up in the persisted theme.
- [ ] Wallpaper selection is reflected on the next desktop start.

## 3. Apps (functional, on real hardware)
- [ ] `hcs-chat --gui`: send a message, get an answer, composer clears, busy
      indicator toggles.
- [ ] Image Studio: change steps/CFG/resolution, Generate produces a plan entry,
      Cancel releases the buffer, Single-Heavy-Model hint is shown.
- [ ] `hcs-monitor --gui`: numbers match `hcs-monitor --json` exactly
      (total RSS, headroom, status).
- [ ] `hcs-control --gui`: Tor pill toggles DIRECT/ISOLATED, the nftables rule
      count matches `hcs-tor-switch status`, AI profile selection sticks.
- [ ] `hcs-search --gui`: type to filter, Enter opens, Esc clears.
- [ ] `hcs-diagnose --gui`: run diagnostics, apply fixes, log updates.
- [ ] `hcs-docs`: every one of the six manuals opens; "Explain with Brain" works
      with the network **disconnected**.
- [ ] `hcs-settings`: shortcuts table matches the shell; About Slint shows the
      Royalty-free License 2.0 disclosure.

## 4. Budgets
- [ ] `python scripts/gui_ram_audit.py` green (<= 250 MB per app).
- [ ] System idle stays <= 6144 MB, peak <= 8192 MB with a GUI app open.

## 5. Licensing
- [ ] `THIRD-PARTY-NOTICES.txt` lists Slint under the Royalty-free License 2.0.
- [ ] The Slint disclosure is visible in the GUI (Settings → About Slint).

## 6. Evidence
- [ ] `qa/expected/gui/<platform>/` is up to date for all four themes
      (`python scripts/verify_gui.py --update --theme <t>` per theme, images
      reviewed). References are per-platform, because the software renderer
      rasterises text through the platform font stack.
- [ ] VirtualBox E2E stages 17-20 captured and entropy-checked.
