# HCS Linux — GUI Build Plan (verbindlich, genau ein Weg)

> Status: beschlossen. Kein Toolkit-Vergleich, keine Alternativen — so wird gebaut und getestet.
> Sprache dieses Dokuments: Deutsch (technische Begriffe Englisch). Repo-Arbeitssprache bleibt Englisch.

## 1. Die eine Entscheidung

**Toolkit: Slint.** Begründung in einem Satz: Rust-nativ, ~5–15 MB Footprint, Wayland mit
Software-Renderer (läuft ohne GPU — Pflicht für Headless-CI und VirtualBox), deklarative
`.slint`-Dateien, statisch linkbar wie alle bisherigen Binaries, Repo-Präzedenz (im
Evolution-Plan für `hcs-monitor` bereits vorgesehen).

**Architektur in einem Satz:** CLI bleibt unverändert (Scripting/Automatisierung), jede App
bekommt zusätzlich ein `--gui`-Flag; GUI ist eine dünne Schicht über den existierenden Libs.

```
hcs-chat --gui ─┐
hcs-monitor --gui├─→ src/hcs-ui (Theme + Widgets, einmalig) + je App: ui/*.slint + src/gui.rs
hcs-control --gui┘   Bindings rufen existierende Libs auf (hcs-memory, hcs-modeld,
                     hcs-security, hcs-image-Engine, hcsd-IPC). Keine Logik-Duplikate.
```

**Lizenz-Prüfung (Pflicht vor P0-Merge):** Slint steht unter GPLv3 *oder* kommerzieller
Lizenz. Vor dem ersten Slint-Commit wird die gewählte Lizenz-Option in
`dist/THIRD-PARTY-NOTICES.txt` + `vendor/locks/sources.lock.yaml` dokumentiert.
Falls GPLv3 mit Apache-2.0-Ziel kollidiert, wird P0 gestoppt und der Maintainer entscheidet —
das ist der einzige Verzweigungspunkt in diesem Plan.

## 2. Baureihenfolge (genau diese)

### P0 — Fundament (ein PR, danach ist jede App Routine)
1. Deps pinnen: `slint` (+ später `pulldown-cmark` erst in P3) in
   `vendor/locks/sources.lock.yaml` eintragen; `cargo verify`-Lauf via `scripts/verify_sources.py`
   muss grün bleiben.
2. Neue Workspace-Crate `src/hcs-ui` (`Cargo.toml` in `Cargo.toml`-Members aufnehmen):
   - `ui/theme.slint` — Farben `#0d1117`/`#38bdf8`, Typo, Abstände (einmalig, alle Apps erben).
   - `ui/widgets.slint` — `GlassCard`, `PillButton`, `Toggle`, `SliderRow`, `ModelBadge`
     (Modell + RAM), `RamBar` (Balken gegen 6144/8192), `TorShield` (grün/grau),
     `HITLDialog` (Bestätigungs-Dialog für Pentester-Aktionen), `LogView`.
   - `src/lib.rs` — Konstruktoren + Theme-Umschaltung
     (Obsidian / Titanium / Stealth, steuert QML- und Slint-Seite über eine JSON-Datei).
3. Test-Skripte (siehe §4) + CI-Job `gui` in `.github/workflows/ci.yml` anlegen.
4. Referenz-Beispiel: ein Widget-Screenshot unter `qa/expected/gui/`.
5. `.desktop`-Datei-Konvention festlegen (siehe P1, Schritt 4).

### P1 — v1.1.0: `hcs-chat --gui` + Image Studio (ein PR)
1. `src/hcs-chat/ui/chat.slint` (Verlauf, Eingabe, Senden) + `src/hcs-chat/ui/image.slint`
   als Tab daneben (Prompt, Negativ-Prompt, Steps-/CFG-Slider, Seed, Auflösung,
   img2img-Dateiwahl, Fortschrittsbalken, Galerie-Grid, Metadaten-Panel).
2. `src/hcs-chat/src/gui.rs` — startet Slint-Fenster, ruft `HcsBrain` (existierend) für Chat
   und `hcs_image`-Lib (existierend) für Render auf; RAM-Gate-Fehlermeldung als Dialog;
   bei residentem 4B-Reasoner: Hinweis-Dialog „erst entladen (Single-Heavy-Model-Regel)".
3. `--gui`-Flag in `src/hcs-chat/src/main.rs` (`--prompt`-One-Shot bleibt für Scripting).
4. `config/includes.chroot/usr/share/applications/hcs-chat.desktop` (Start-Menü-Pin wird echt).
5. DoD §5 erfüllen, dann Merge.

### P2 — v1.2.0: `hcs-monitor`, `hcs-control`, `hcs-search` (ein PR pro App)
- Monitor: `RamBar`-Widgets gegen Budget, Modell-Residenz, Daemon-Liste; Datenquelle ist der
  existierende `--json`-Output (kein neues Sammeln).
- Control: `TorShield`-Toggle (ruft dieselbe Logik wie `hcs security tor` + zeigt
  `TorTransparentProxy::enable_rules`-Status), AI-Profil-Wahl EDGE/LOWRAM/WORKSTATION,
  Hardware-Info aus `hcs-settings`.
- Search: Spotlight-Omnibar, Ergebnisliste (Apps/Dateien/Memory), keyboard-first (Enter =
  öffnen, Esc = schließen); ruft `hcs-search`-Lib auf.

### P3 — v1.3.0: `hcs-diagnose`, Docs-Viewer, `hcs-settings` (ein PR pro App)
- Diagnose: Triage-Karten, Log-Tail (`LogView`), „Fix anwenden" ruft `--apply`-Logik auf.
- Docs: **nativer** Markdown-Viewer mit gepinntem `pulldown-cmark` (kein Webview, kein
  Browser-Embed!), rendert `/usr/share/hcs/docs/manuals/*.md` + Button „Explain with Brain".
- Settings: Theme-/Wallpaper-Wahl (die 3 PNGs aus `assets/wallpapers`), Keybindings-Tabelle
  aus `cheatsheet.json`, Privacy-Defaults.
- Calamares-Installer und Shell-Chrome (Quickshell-QML) werden **nicht** angefasst.

## 3. RAM-Budget (verbindliche Obergrenzen)

Shell (Quickshell) ≤180 MB idle (unverändert) · jede GUI-App ≤250 MB RSS ·
System gesamt ≤6144 MB idle / ≤8192 MB peak (unverändert). Verletzt eine App ihre Grenze,
wird der PR nicht gemergt. Gemessen wird per `/proc`-RSS, nicht per Schätzung.

## 4. Testweg (genau dieser, in dieser Reihenfolge pro App)

1. **Unit** — Slint-Test-API (Properties setzen, Callbacks aufrufen, kein Display nötig):
   `cargo test -p hcs-ui -p hcs-chat` (je App entsprechend). Assert-Beispiele: Senden-Callback
   hängt Nachricht an, Steps-Slider clamp 1–8, HITL-Dialog blockt ohne Bestätigung.
2. **Headless-Render** — Slint Software-Renderer rendert jede View in einen Pixel-Buffer:
   `python scripts/verify_gui.py --render-only` (neu in P0). Assert: kein Blank-Frame,
   Entropie ≥0.5, Mindestgröße — dieselbe Methode wie `scripts/verify_visual_qa.py`.
3. **Visual Regression** — Render mit Referenz vergleichen:
   `python scripts/verify_gui.py --regress` gegen `qa/expected/gui/<app>-<view>-<theme>.png`
   (SSIM-/Pixel-Schwelle, JSON-Report im bekannten Audit-Format). Referenzen werden im PR
   als Screenshots zur Review angehängt und erst nach Abnahme unter `qa/expected/gui/`
   eingecheckt.
4. **RAM-Audit** — `python scripts/gui_ram_audit.py` (neu in P0): startet jede GUI-App
   headless (Offscreen-Backend), misst RSS via `/proc`, prüft ≤250 MB pro App und
   Gesamt-Matrix. Cruise: läuft in CI und vor jedem Release.
5. **Wayland-Integration (CI)** — neuer CI-Job: Headless-Weston, App mit `--gui` starten,
   Fenster-Screenshot, Assert Fenster vorhanden + kein Crash bei 60 s Laufzeit; zusätzlich
   `hcsd`-Socket einmal gemockt, einmal echt.
6. **VirtualBox E2E (Release-Gate)** — Gate-5-Skript bekommt Stufen 17–20:
   Chat-Fenster offen, Image-Studio-Render, Tor-Pill aktiv, Docs-Viewer; Bedienung per
   `VBoxManage keyboardputscancode` (Enter-Pulse-Muster existiert), Assert Entropie >1.5
   pro Bild wie bisher.
7. **A11y/Tastatur (CI)** — Fokus-Walk-Test pro View (Tab-Reihenfolge als Konstanten-Test),
   Kontrast-Assert der Theme-Farben (WCAG-Richtwerte als Test-Konstanten).
8. **Manuell (Release)** — einseitige Checkliste `docs/GUI_RELEASE_CHECKLIST.md` (neu in P1):
   Look & Feel, Theme-Wechsel live, echte Inferenz im Chat, echter Render im Studio,
   Tor-Toggle mitircuit-Check. Abhaken pro Release, Teil von Gate 7.

**Definition of Done pro App (alle vier Pflicht, dann Merge):**
`cargo test` grün · `verify_gui.py --render-only` grün · `--regress` grün (Referenz abgenommen) ·
`gui_ram_audit.py` grün. Fehlt eines, kein Merge.

## 5. CI-Änderung (P0, einmalig)

`.github/workflows/ci.yml` bekommt Job `gui` (nach `fast-path`):
`cargo test -p hcs-ui …` → `verify_gui.py --render-only` → `--regress` →
`gui_ram_audit.py`. Plus Job `gui-wayland` (Weston-Container). Alles andere bleibt.

## 6. Dateiliste (was neu entsteht, vollständig)

- `src/hcs-ui/{Cargo.toml,src/lib.rs,ui/theme.slint,ui/widgets.slint}`
- je App: `src/hcs-<name>/{src/gui.rs,ui/*.slint}`, `--gui`-Flag in `main.rs`
- `config/includes.chroot/usr/share/applications/hcs-*.desktop`
- `scripts/verify_gui.py`, `scripts/gui_ram_audit.py`, `docs/GUI_RELEASE_CHECKLIST.md`
- `qa/expected/gui/*.png` (Referenzen), `.github/workflows/ci.yml` (2 Jobs)
- `vendor/locks/sources.lock.yaml` (Slint-Pin), `dist/THIRD-PARTY-NOTICES.txt` (Lizenz)
- P3 zusätzlich: `pulldown-cmark`-Pin (sonst keine neuen Deps — alles andere existiert)

## 7. Startbedingung

P0 startet auf Maintainer-Go. Erster Commit: Lizenz-Doku + Slint-Pin + `hcs-ui`-Gerüst.
Schätzung erst nach P0 (dann ist die Routinearbeit pro App vermessen).
