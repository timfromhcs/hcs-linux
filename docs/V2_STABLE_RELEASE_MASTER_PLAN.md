# HCS Linux v2.0.0 Stable — Master Release Plan

> **Status:** verbindlich. Ein Weg, keine Alternativen.
> **Grundlage:** Ist-Audit des Repos (30.09.2026, `dev` @ `383283f` + 60 uncommittete
> 1.1.0-Dateien) + Feature-Audit von macOS 27/Tahoe, Windows 11, Ubuntu/GNOME 49,
> Linux Mint 22.3 (Cinnamon 6.6), Tails, Omarchy 4, Android 16, iOS 26.
> **Sprache:** Deutsch, technische Begriffe Englisch. Repo-Arbeitssprache bleibt Englisch.
> **Verhältnis zu `docs/GUI_BUILD_PLAN.md`:** dieses Dokument ersetzt es für v2. Der GUI-Plan
> gilt als erfüllt und wird nicht weitergeführt. `docs/V1_STABLE_QA_STATUS.md` bleibt die
> Wahrheitsquelle für v1.

---

## 0. Kurzfassung

v1 hat eine **gute Backend-Basis und eine dünne, unbewiesene Oberfläche**. Das ISO bootet
nicht in eine grafische Sitzung, der ISO-Payload verliert Dateien stillschweigend, und zwei
Gates sind rot, während die Release-Notes das Gegenteil behaupten. v2 behebt zuerst die
Lücke zwischen Behauptung und Wirklichkeit und holt danach die Bedienqualität der
Referenzsysteme.

**Nicht-Ziel von v2:** neue Kernmodelle, Cloud-Sync, Multi-User-Produktivsystem.
**Ziel von v2:** ein Desktop, eine Live-Sitzung, eine Bedienung, die sich bei jedem
Referenzsystem etwas geliehen hat, ein*e* QA-Schleife, die beweist statt behauptet.

---

## 1. IST-Audit (verifiziert am 30.09.2026, nicht behauptet)

### 1.1 Was existiert und trägt

| Bereich | Befund |
|---|---|
| Crates | 20 Member, `cargo test --workspace` grün (alle Binaries grün) |
| GUI-Fundament | `hcs-ui` Slint-Kit, 3 Themes, 9 Views × 3 Themes = 27 Referenzrenders, `verify_gui.py` 9/9 PASS pro Theme |
| GUI-Apps | `hcs-chat --gui` (Chat + Image Studio), `hcs-monitor`, `hcs-control`, `hcs-search`, `hcs-diagnose`, `hcs-docs`, `hcs-settings` — alle mit Referenzbild |
| Shell | `src/hcs-shell/*.qml` (taskbar, start_menu, cheatsheet, clipboard, control_drawer, notifications, shell) + `config.kdl` für niri |
| KI | `hcsd`, `hcs-modeld` (On-Demand-GGUF, Single-Heavy-Regel), `hcs-memory` (SQLite FTS5, 5 Klassen, hybrid_search), `hcs-agents` (Capability-Matrix), `hcs-mcp` (MCP-Hub), `hcs-image` (SD 1.5 LCM) |
| Security | `hcs-security` (nftables Tor-Transparent-Proxy, LUKS-Vault), `hcs-tor-switch`, HITL-Gate |
| Packaging | Hybrid-ISO BIOS+UEFI, ISO9660 verifiziert, SBOM/Manifeste/Notices generiert |
| QA | 18 VirtualBox-Stufen, Entropie-Audit, Stress 100 Zyklen, Security-Audit, RAM-Audit |

### 1.2 Was fehlt oder falsch ist (Blockerliste)

**B-01 · Gate 1 rot — `cargo fmt --check`**
`src/hcs-ui/tests/ui_tests.rs:115` verletzt `cargo fmt`. CI-Job `fast-path` würde rot.

**B-02 · Gate 1 rot — `cargo clippy --workspace -- -D warnings`**
`src/hcs-ui/src/bin/hcs-docs.rs:15` und `src/hcs-ui/src/bin/hcs-settings.rs:15`:
`redundant closure` (`unwrap_or_else(|| hcs_ui::load_theme())`). Release-Notes 1.1.0
behaupten „clippy 0 warnings" — das ist falsch.

**B-03 · Gate 4 Payload-Verlust — native `hcs-docs` GUI kommt nicht ins ISO**
`scripts/build_iso.sh:54` kopiert die Rust-Binaries, `:141` überschreibt
`/usr/bin/hcs-docs` danach mit dem 298-Byte-Bash-Skript aus
`config/includes.chroot/usr/share/hcs/scripts/hcs-docs`. Ergebnis: `target/rootfs/usr/bin/hcs-docs`
ist 298 Bytes, nicht 22 MB. Die native Markdown-Ansicht existiert nur auf dem Host.

**B-04 · Gate 4 Payload-Verlust — Start-Menü-Pins, Icons, Wallpapers, Manuals fehlen**
`mkdir` in `build_iso.sh:31-36` erzeugt nur `branding/` und `shell/`. Die `cp`-Zeilen
`:133` (Icons) und `:134` (Wallpapers) schlagen fehl und werden von `|| true` geschluckt.
`.desktop`-Dateien (`config/includes.chroot/usr/share/applications/`, 7 Stück) werden
**nie** kopiert — es gibt keine Zeile dafür. Verifiziert: `target/rootfs/usr/share`
enthält nur `hcs/` mit `branding/` und `shell/`. Kein `applications/`, kein `docs/`,
keine Icons, keine Wallpapers.

**B-05 · Keine grafische Live-Sitzung**
`config/package-lists/hcs-core.list.chroot` enthält `libwayland-client0`,
`wayland-protocols` — aber **kein `niri`, kein `quickshell`, kein `seatd`-Setup, kein
Session-Manager, kein Plymouth-Start**. `build_iso.sh:76-127` schreibt ein
`/sbin/init`, das ein Banner druckt und `sleep 3600` schleift. Konsequenz, im QA-Status
korrekt vermerkt: VirtualBox-Stufen 17–18 sind **keine** GUI-Pixel, nur
"Konsole bootet, Toolchain ist gestaged". Der dokumentierte Gap schließt sich erst mit B-06.

**B-06 · Gate 6 nie ausgeführt für den GUI-Code**
`origin/dev` steht auf `a792257`; die beiden GUI-Commits (`9ab7a59`, `383283f`) sind
lokal und nie auf GitHub. Der grüne CI-Lauf `36754352667` gehört zum Plan-Commit, nicht
zum GUI-Code. Es existiert **kein** CI-Nachweis für `hcs-ui`.

**B-07 · Kein Dateimanager**
Zwischen Omarchy-Nutzer-Wunsch ("TUI-Version von Total Commander") und Windows/Mint
ist das die auffälligste fehlende Programmklassse. `hcs-fm` existiert nicht.

**B-08 · Kein Terminal-Emulator als GUI-Programm**
`cheatsheet.json` verspricht `Super+T` → Terminal. Es gibt kein Binary, das ein
Terminal-Fenster öffnet.

**B-09 · Keine Fensterverwaltung**
Kein Snap Layout, keine Snap Groups, keine virtuellen Desktops, kein Stage-Manager-
Äquivalent, kein Fenster-Switcher mit Live-Preview, kein Alt-Tab. `hcs-shell` malt
eine Taskbar, aber nichts darüber.

**B-10 · Suche ohne Aktionen**
`hcs-search` rankt Apps/Files/Memory. Es fehlen die macOS-Spotlight-Killer: Aktionen
ausführen, Quick Keys, Vorschau-Pane, Suchhistorie (`↑`), Fenster/Tabs als Treffer,
`?`-Bereich für Hilfe.

**B-11 · Kein Screenshot-/Snipping-Werkzeug**
Kein `hcs-shot`. Kein Region-/Fenster-/Delay-Capture, kein OCR-Textextrakt, kein
Color Picker, kein Screenrecord. Für QA ist das zugleich ein Werkzeug und ein Produkt.

**B-12 · Kein Update-Manager mit Risikostufen + Rollback**
`hcs-updater` ist ein GitHub-Poller. Es fehlen: Level-System (Mint), Reboot-Pflicht-
Hinweis, Pre-Update-Snapshot, Rollback aus dem Bootmenü (Omarchy-Btrfs-Muster).

**B-13 · Keine Amnesie / keine opt-in Persistenz**
Tails' stärkstes Argument fehlt vollständig: `nopersistence` auf der Kernel-Cmdline,
`init_on_free=1`, `swapon` no-op, LUKS-Persistent-Storage mit Feature-Opt-in,
`active/enabled/masked`-Zustände, Unlock beim Boot.

**B-14 · Theme-Atomizität fehlt**
`hcs-ui` persistiert `theme.json` pro App. Omarchy löst das über **eine** `colors.toml`,
die per Template in alle Apps, Terminal, Editor, Wallpaper, Tastatur-RGB geschrieben
wird. Bei uns: Themewechsel ist nicht atomar, nicht systemweit, kein Live-Preview.

**B-15 · Kein RAG-Backend im Produkt + keine RAG-Tests**
`hcs-rag-ingest` ist eine Bibliothek ohne UI, ohne GUI, ohne Evaluation. `tests/unit`
enthält nur `test_golden_suite.py` und `test_models_lock.py` — **kein einziger Test
prüft Retrieval-Qualität, Grounding, Zitat-Leistung oder Refusal.**

**B-16 · Kein Agent im Desktop**
Omarchy behandelt Agents als First-Class: Crash-Benachrichtigung → Agent analysiert
Dump → Bug-Report. Bei uns existiert `hcs-agents` als Backend ohne jede Desktop-
Integration. Kein `hcs-actions`-Layer: kein App registriert Aktionen.

**B-17 · Keine Barrierefreiheit**
`docs/GUI_BUILD_PLAN.md` §4 Schritt 7 (Fokus-Walk-Test pro View, WCAG-Kontrast-Assert)
ist **nicht implementiert**. Kein Screenreader, kein `Reduce Motion`, keine
Tastatur-only-Navigation, kein High-Contrast-Modus.

**B-18 · `docs/GUI_RELEASE_CHECKLIST.md` ist 0/20 abgehakt**
Gate 7 verlangt die manuelle Checkliste. Sie existiert, ist leer.

**B-19 · `auto/config` trägt Alt-Version**
`--iso-application "HCS Linux 0.1.0-alpha.1"`, `--iso-volume "HCS_LINUX_010"`.
Metadaten im ISO widersprechen 2.0.0.

**B-20 · README-Behauptungen überholen**
„VirtualBox 16/16" (jetzt 18), Clippy-Aussage falsch, keine Erwähnung des bekannten
GUI-Gaps im prominenten Abschnitt.

---

## 2. Feature-Audit der Referenzsysteme

Jedes System wird nur auf das reduziert, was **übertragbar** ist auf: Debian 13 Trixie
+ Slint + niri/Quickshell + Rust, **CPU-only, kein GPU-Zwang, ≤6 GB idle**.

### 2.1 macOS 27 "Golden Gate" / Tahoe

**Übernehmen**

| Feature | Warum es zählt | HCS-Ziel |
|---|---|---|
| **Spotlight = ein omnibar für Finden *und* Tun** | Zwei Modi (Apps/Finden vs. Actions), 300+ Aktionen ohne App-Wechsel | W3 `hcs-actions` |
| **Quick Keys** (`sm` → senden, `ar` → Reminder) | Zwei Tasten statt Menü-Hunting | W3 |
| **Alles in einer Result-Liste, danach ranken** | Apps, Files, Events, Messages, Fenster, Tabs in *einer* Liste | W3 |
| **History mit `↑`** | Wiederholen ohne Tippen | W3 |
| **Browse-Views** (Cmd+1 Apps, Cmd+3 Actions) | Scannen statt Suchen | W3 |
| **Vorschau im Suchergebnis** | Vertrauen ohne Öffnen | W3 |
| **Stage Manager** (eine App vorn, Rest gestapelt links) | Anti-Desktop-Chaos, reversibel, gruppierbar | W4 |
| **Shortcuts-Automation-Trigger**: Zeit, Ort, App-Nutzung, Datei gespeichert, Display verbunden, **Screenshot gemacht**, Benachrichtigung empfangen | Ereignis-getriebene Automatisierung ohne Skript | W5 |
| **„Use Model" in Automation** | LLM als Schritt einer Automation, nicht als Chat-Fenster | W7 |
| **Shortcuts aus Natursprache** | „Wenn ich Feierabend habe, ETA + Nachricht" | W7 |
| **Control Center als Galerie** (Module rein/raus, third-party) | Personalisierbare Schnellzugriffe | W4 |
| **Finder-Notions**: Liste mit kollabierbaren Ordnern, „Open With", Default-App pro Typ, eigene Ordner-Icons+Farben, Tags | Orientierung in großen Datenmengen | W2 `hcs-fm` |
| **Focus Modes + Benachrichtigungs-Screening** | Deep Work, Fremdnummer-Screen | W4 |
| **Handoff / Universal Clipboard / Continuity Camera** | Geräte-Grenze auflösen | v2.1 (siehe §10) |
| **Writing Tools / Genmoji / Live Translation / Visual Intelligence** | Systemweite KI-Aktionen auf dem, was auf dem Schirm ist | W7 |

**Nicht übernehmen:** Apple-Silizium-Pflicht, iCloud-Zwang, Store-Ökosystem.

**Pflichtlektion:** macOS 27 hob Liquid Glass **viermal** nach (Control Center war
unlesbar → Buttons dunkler + opaker; Menüleisten wieder undurchsichtiger). **Regel für
uns:** Transparenz nie über Textkontrast. Siehe §4.4.

### 2.2 Windows 11

**Übernehmen**

| Feature | Warum es zählt | HCS-Ziel |
|---|---|---|
| **Snap Layouts** (Hover über Maximieren / `Win+Z` → Layout-Raster) | Der beste Fenster-Organizer aller Systeme; Snap merkt sich das Layout | **W2 Priorität 1** |
| **Snap Groups** (ganze Fenstergruppe per Hover auf Taskbar-Icon zurück) | „Wo war ich nochmal" lösen | W2 |
| **Snap Assist** (nach Snap: Thumbnails der übrigen Fenster) | Zweites Fenster ohne Alt-Tab | W2 |
| **Virtual Desktops pro Projekt**, aus der Taskbar switchbar | Arbeitskontext statt Fensterchaos | W2 |
| **Task View** mit Overlay-Screenshots | Räumliche Übersicht | W2 |
| **Alt+Tab mit Live-Preview** | switching ohne Blindheit | W2 |
| **Click to Do** (`Win`+Klick auf Text/Bild → Aktionen) | Aktionen ohne Kontextwechsel | W7 |
| **Copilot Vision** (Bildschirm teilen, Bildschirm-für-Bildschirm-Anleitung) | „Zeig mir wie" | W7 |
| **Press to Talk / Voice Access** (Diktat, App starten, navigieren) | Hände frei | W7 |
| **Recall** (`Win+J`): Timeline + **semantische** Suche + Rückkehr in den Kontext — verschlüsselt, Hello-geschützt, **Sensitive-Info-Filter** | Zeitsuche über das, was man *gesehen* hat. Wichtig: HCS bekommt das **ohne Cloud und mit Amnesie-Ausnahme** (B-13) | W6 |
| **Snipping Tool**: *Perfect Screenshot* (auto einrasten), **Text Extractor** (OCR), **Color Picker**, Screenrecord mit Mikrofon | Drei echte Alltagsgewinnste in einem Werkzeug | **W2 `hcs-shot`** |
| **Focus Sessions / DND im Quick-Settings-Panel** | Deep Work | W4 |
| **Widgets-Board + Lock-Screen-Widgets** (mit Auto-Vorschlag) | Glance-Info ohne Fenster öffnen | W4 |
| **Live-Unlock-Telemetrie**: hell/dunkel automatisch, Akku-Ladegrenzen sichtbar, Warnung wenn Update Neustart braucht | Kleine Ehrlichkeits-Details, die Vertrauen schaffen | W4/W5 |
| **File-Explorer-Tabs** | mehrere Orte gleichzeitig | W2 |
| **Notepad mit Markdown** (nativ) | Notizen im Repo-Format | W2 `hcs-notes` |
| **Phone Companion im Start-Menü** | Android/iOS-Gerät als Ziel in der Suche | v2.1 |

**Nicht übernehmen:** Recall-Cloud-Telemetrie (wir machen sie lokal + amnesic),
Werbe-Ökosystem, Copilot-Zwang.

### 2.3 Ubuntu / GNOME 49

**Übernehmen**

| Feature | Warum es zählt | HCS-Ziel |
|---|---|---|
| **Such-Popover mit Filter-"Pills" + Datums-Kalender** (GNOME 49 Nautilus) | Beste Filter-UX aller Systeme; Pillen zeigen aktive Filter | W3 |
| **Overview behält Fensterpositionen über Sessions** | Setup-Friction null | W2 |
| **Do Not Disturb wandert ins Quick-Settings-Panel** | Status an einem Ort | W4 |
| **Sperrbildschirm: Mediensteuerung + Neustart/Herunterfahren ohne Login** | Win11-artig, aber GNOME-typisch ruhig | W4 |
| **Accessibility-Menü auf dem Greeter** (Screenreader per Toggle) | Zugänglichkeit darf versteckt sein, aber existieren | W7 |
| **Per-Monitor-Helligkeit in Quick Settings** | Multi-Monitor ehrlich | W4 |
| **Terminal mit `Alt+,` Container-/Profil-Suche** (`hcs dev`-Profile!) | Profile auffindbar | W2 `hcs-term` |
| **`Ctrl+.` = aktueller Ordner im Terminal** | Brücke Datei→Terminal | W2 |
| **Wayland-only** (GNOME 49 fährt X11 ab) | X11 raus, eine Display-Server-Wahrheit | W1 |
| **Exakte Bruchskalierung** (Fractional-Scaling exakt statt gerundet) | Schärferer Text | W1 |
| **Remote-Desktop mit Multi-Touch + virtuellen Monitoren** | HCS als ferngesteuerter Desktop | v2.1 |

**Nicht übernehmen:** Snap-Store-Zwang, GNOME-Apps-Overhead für HCS-eigene Apps.

### 2.4 Linux Mint 22.3 (Cinnamon 6.6)

**Übernehmen**

| Feature | Warum es zählt | HCS-Ziel |
|---|---|---|
| **Windows-vertrautes Layout** (Panel unten, Menü links, Tray rechts) | Wir haben das schon — deshalb behalten | unverändert |
| **Update-Manager-Level 1…5** (sicher → experimentell), automatisch/manual wählbar | Das ehrlichste Update-Modell aller Distros | **W5 `hcs-update`** |
| **Reboot-Pflicht-Warnung** im Tray nach Updates | Verhindert der Klassiker „Kernel gewechselt, altes System" | W5 |
| **Timeshift: automatisches Pre-Update-Snapshot + Rollback aus dem GRUB-Menü** | Ein kaputtes Update ist nie eine Katastrophe | **W5** |
| **APT + Flatpak in einer Oberfläche**, ohne Format-Denkfrage | Ein Installationsweg | W5 |
| **Welcome-Screen / First-Run-Assistent** (Codecs, Firewall, Treiber, Snapshot einrichten) | 5 Minuten, die alles erleichtern | W1 |
| **Spices-Ökosystem**: Applets/Desklets/Extensions/Themes über GUI-Manager | Anpassbarkeit ohne Config-Dateien | v2.1 (HCS-Plugins) |
| **Kein Gast-Login per Default** | Sicherheit als Default | W1 |
| **Night-Light-Applet, always-on** | Nachtmodus | W4 |
| **App-Menu: Sidebar (Avatar, Orte, Favoriten) konfigurierbar bis minimal** | Dichte ist Geschmack → Option statt Dogma | W1 |

**Nicht übernehmen:** X11-Applets, Cinnamon-Shell.

### 2.5 Tails

**Übernehmen** — das ist das stärkste Sicherheitsargument, das wir haben

| Feature | Umsetzung für HCS |
|---|---|
| **Amnesie als Default** | Kernel-Cmdline `nopersistence`, kein `swapon` (Binary → no-op), `init_on_free=1` |
| **RAM beim Shutdown überschreiben**, auch bei **physischem USB-Entfernen** (`udev-watchdog` → Speichernullung, umgeht alle Shutdown-Skripte) | Selbes Muster in unserem `init` |
| **Persistent Storage** = LUKS auf dem Bootmedium, **optional**, Feature-Liste pro User wählbar, Zustände `active` / `enabled` / `masked`, Unlock im Welcome-Screen, Hooks bei Aktivierung | `hcs-persist` + Installer-Partition |
| **Bind-Mount/Symlink statt Kopie** pro Feature | Wie Tails (inkl. `nosymfollow`-Schutz gegen Symlink-Angriffe — bei uns nötig, sobald es einen unprivilegierten User gibt) |
| **Kill-Switch: Apps werden blockiert, wenn sie ohne Tor verbinden wollen** | Haben wir als `TorTransparentProxy` (nftables) — **muss im GUI sichtbar und pro App erzwingbar sein** (W4 `hcs-control`) |
| **Sichere Defaults, keine Konfiguration, die Nutzer unterscheidbar macht** | Keine Standard-Benutzer-ID, keine Timezone-Leaks, kein Plaintext-Log |
| **Schwaches Glied ist die Passphrase** → 5–7 Zufallswörter empfehlen | Vault-UI: Wortliste-Generator statt Hexdump |
| **Keine „sichere Löschung" versprechen** (auf SSD/Flash unmöglich) | Dokumentation ehrlich: Verschlüsselung + überschreiben + physisch zerstören |
| **Threat-Model dokumentiert** | `docs/THREAT_MODEL.md` Teil von Gate 2 |

**Nicht übernehmen:** Amnesie als erzwungener Default für das Installierte System —
dort ist Persistent Storage Default, Amnesie die Live-Modus-Option.

### 2.6 Omarchy 4 "Quattro"

**Übernehmen** — das wichtigste Lehrmodell für v2

| Feature | Warum es zählt | HCS-Ziel |
|---|---|---|
| **Eine einzige CLI, die alles dispatcht** (`omarchy` → 300+ Skripte) | **Agent-lesbare Kontrollfläche.** Ein KI-Agent lernt das System in einem Dokument | **W6 `hcs`-CLI erweitern: jederzeit `hcs <cmd> --json`** |
| **Theme = eine `colors.toml`, Template-Expansion, atomarer Swap, Restart der betroffenen Komponenten, Hooks `theme-set.d/`** | Ein Theme-Wechsel restylt *alles* konsistent — das ist die Kern-Idee | **W4 (B-14)** |
| **Btrfs-Snapshots bei jedem Update, Rückkehr aus dem Bootmenü** | Update-Sicherheit | W5 |
| **LUKS Full-Disk per Default, Dual-Boot in freie Partition (eigene EFI)** | Laptop-tauglich ohne Windows zu zerstören | W5 |
| **Agenten als First-Class**: Crash-Notification anklicken → Agent untersucht Dump | Autonomie im Desktop | **W6 `hcs-agent-bridge`** |
| **Super+Space Launcher, Super+Alt+Space Menü, alles per Taste** | Tastatur zuerst — ohne Maus benutzbar | **W4** |
| **Web-App-Installer** (URL → App mit Icon + Shortcut) | Software ohne Distro-Paket | v2.1 |
| **Skills**: Agent lernt eigene Plugins/Themes zu bauen | Erweiterbarkeit | v2.1 |
| **One-Command-Setup / Bootstrap-Wizard in 5 Fragen** | Erstnutzer-Erfolg | W1 |
| **Honest first-run**: „installiere deinen Agenten" | Der Nutzer entscheidet | W1 |

**Nicht übernehmen:** Arch statt Debian, Hyprland statt niri, Secure-Boot-Kompromiss
(bei HCS behalten wir Signed Boot, wenn machbar).

### 2.7 Android 16

**Übernehmen**

| Feature | Warum es zählt | HCS-Ziel |
|---|---|---|
| **Live Updates**: fortschrittsorientierte, dauerhafte Benachrichtigungen + Statusleiste-Chips + AOD | Der Nutzer muss die App **nicht** öffnen, um den Status zu sehen | **W4 Live-Activity-Panel** (für Image-Render, Tor-Verbindung, Update-Fortschritt) |
| **Auto-Gruppierung** ähnlicher Benachrichtigungen | Anti-Spam, bleibt aber vollständig | W4 |
| **Predictive Back**: Ziel-Vorschau **vor** dem Abschluss der Geste | Grundsatz: destruktive/umleitende Aktionen zeigen vorher ihr Ziel | W4 |
| **Desktop-Windowing**: mehrere frei verschiebbare/resizable App-Fenster + Taskbar-Overflow + eigene Tastenkürzel auf großen Screens | Wir bauen ohnehin Fensterverwaltung — dann richtig | W2 |
| **Haptic Slider** (takte Detents bei Lautstärke/Helligkeit) | Präzision ohne Blick | v2.1 (nur mit Trackpad-Haptik) |
| **Dynamic Color / Material You**: UI-Farben aus dem Wallpaper ableiten | Themes, die zum Bild passen | W4 |
| **Edge-to-edge erzwungen** | Keine Layout-Sprünge zwischen Fenstergrößen | W2 |
| **Text-Scaling konsistent über Formfaktoren** | Skalierung darf nicht brechen | W7 |
| **Adaptive Refresh** | nur mit Hardware-Support | v2.1 |

### 2.8 iOS 26

**Übernehmen**

| Feature | Warum es zählt | HCS-Ziel |
|---|---|---|
| **Dynamic Tab Bars**: schrumpfen beim Scrollen, wachsen bei Bedarf | maximaler Inhaltsraum ohne Kontrollverlust | W1 (Slint-Shell) |
| **Focus / Notification-Screening** (Unbekannte-Nummern, "Halten") | Ablenkungs-Kontrolle als OS-Funktion | W4 |
| **Visual Intelligence + Highlight-to-Search**: Finger über Text/Bild → Suche | Suche auf dem, was man sieht | **W7** (`hcs-shot --extract` + `hcs-search`) |
| **Files: "Open With", Default-App pro Typ, eigene Ordner-Icons/Farben** | siehe macOS | W2 |
| **Preview (PDF + Scan + Export)** | Dokumente ohne Office | W2 |
| **Widget-Stacks, die sich anpassen** | Glance | W4 |
| **Adaptive Lock-Screen-Uhr** | Kleiner Luxus, große Wirkung | W1 |
| **Apple Games als Hub** (nicht nur Store) | Sammlung im Hub | v2.1 |

**Pflichtlektion:** Liquid Glass ist *ein* Materialsystem über alle Geräte hinweg. HCS
Neural Glass ist bereits eines (Slint-Kit) — v2 nutzt es konsequent: **jedes** neue
Programm nutzt nur Kit-Widgets, keine eigene Optik.

---

## 3. Zielbild v2.0.0

> **HCS Linux v2 ist ein Windows-11-typisches Bedienmodell (Taskbar unten, Start-Menü,
> Snap Layouts, Win-Tasten-Navigation, Quick Settings, Live-Updates), mit der Präzision von
> macOS Spotlight (ein Omnibar für Finden **und** Tun), der Update-Sicherheit von Mint +
> Omarchy (Level, Snapshots, Rollback), der Amnesie-Option von Tails, der
> Agenten-Integration von Omarchy, dem Tastatur-Workflow von Omarchy/GNOME, und einer
> QA-Schleife, die Screenshots beweist statt behauptet.**

### Was v2 dem Nutzer verspricht

1. **Es startet.** Vom USB-Stick in eine grafische Sitzung, in <60 s, mit Login-Autologin
   für den Live-Modus.
2. **Alles ist per Taste erreichbar.** Jedes Programm, jede Aktion, jeder Modus.
3. **Ein Suchfeld für Finden und Tun.** Apps, Dateien, Memory, Fenster, Einstellungen,
   Aktionen — eine Liste, gerankt, mit Vorschau und Quick Keys.
4. **Fenster ordnen sich selbst.** Snap Layouts, Snap Groups, virtuelle Desktops,
   Stage-Mode, Task View.
5. **Nichts geht verloren.** Update vor Update snapshottet, Rollback aus dem GRUB-Menü.
6. **Privatsphäre ist ein Schalter, kein Versprechen.** Tor-Killswitch sichtbar,
   Amnesie-Modus optional, Recall nur lokal und verschlüsselt.
7. **Die KI ist eingebaut, nicht verklebt.** Chat, RAG, Agent-Aktionen, Live-Updates,
   Bildstudio — alles in denselben Workflows wie die Systemaktionen.
8. **Jede Behauptung hat einen Screenshot.**

---

## 4. Architekturentscheidungen für v2

### 4.1 Crates (neu / erweitert)

```
src/hcs-ui/            erweitert: Kit + Snapping-freie Basis-Widgets, A11y-Props
src/hcs-actions/       NEU   Aktions-Registry (jede App registriert Aktionen + Quick Keys)
src/hcs-fm/            NEU   Dateimanager (Nemo-Klasse)
src/hcs-term/          NEU   Terminal-Emulator (vte-Parser, Grid-Render in Slint)
src/hcs-shot/          NEU   Screenshot/Region/OCR/Color Picker/Screenrecord
src/hcs-notes/         NEU   Markdown-Notizen (Notepad-Klasse)
src/hcs-update/        NEU   Level-Update-Manager + Snapshot + Rollback
src/hcs-persist/       NEU   Amnesie-Modus + opt-in LUKS-Persistent-Storage
src/hcs-recall/        NEU   lokale, verschlüsselte Screen-Zeitleiste
src/hcs-agent-bridge/  NEU   Agent ↔ Desktop (Crash→Agent, Aktionen, Fortschritts-UI)
src/hcs-a11y/          NEU   Screenreader-Bridge, Fokus-Walk, Kontrast-Gate
src/hcs-qa-agent/      NEU   deterministischer Gast-QA-Treiber (siehe §7.3)
src/hcs-shell/         erweitert: Control Center, Notification Center, Widgets, Stage
src/hcsd/              erweitert: Fenster-/Sitzungs-IPC an niri
```

### 4.2 Fensterverwaltung: wie ehrlich sind wir?

niri ist ein scrollender Tiling-Compositor. Win11-Snap-Layouts sind *freies* Fenstermanagement.
Beides gleichzeitig ist eine Entscheidung, keine Kleinigkeit.

**Entscheidung (verbindlich): Dual-Modus.**
- **Free-Floating-Modus** (Default, wie Win11): `niri` läuft mit `layout { … }`-Overrides
  ohne Tiling-Spalten; Fenster frei platzierbar. Snap wird über niri-IPC
  (`move-window-left/right`, `toggle-maximize`, `set-window-width/height`,
  `move-window-to-floating`, `toggle-window-fullscreen`) gesteuert.
- **Tiling-Modus** (Super+Alt+T, wie Omarchy): scrollende Spalten, Tastatur-gesteuert.
- Der Modus ist pro Sitzung umschaltbar, der Wechsel wird in `theme.json` gespeichert.

Das ist ehrlich, mit den vorhandenen Mitteln umsetzbar und liefert beide Nutzungsarten.

### 4.3 Theme-Atomizität (B-14)

```
/etc/hcs/theme/colors.toml          # EIN Quelle der Wahrheit
hcs theme set obsidian|titanium|stealth|custom
  1. colors.toml validieren
  2. Templates in /etc/hcs/theme/templates/ expandieren  ({{ accent }}, {{ accent_rgb }} …)
  3. atomar nach /var/lib/hcs/theme/active/ swappen (rename)
  4. betroffene Prozesse neu laden: hcs-shell, laufende hcs-*-GUIs, Terminal, Wallpaper
  5. Hooks: /etc/hcs/theme/hooks.d/*.sh
Live-Preview: `hcs theme preview <name>` rendert in den Ziel-Apps ohne Persist
```
Jede App liest nur `/var/lib/hcs/theme/active/palette.json` (Cache), nie selbst Dateien.
Damit ist Win11-Snap, GNOME-Material-Y und Omarchy-`colors.toml` in **einem** Mechanismus
vereinheitlicht.

### 4.4 Neural Glass: Lesbarkeit ist nicht verhandelbar

Aus der Liquid-Glass-Korrekturfolge (4 Revisionen, weil Control Center unlesbar war)
ergeben sich **verbindliche Regeln**, die `hcs-a11y::contrast_gate()` prüft:

1. Kein Text über einem Hintergrund mit Kontrast < **4.5:1** (WCAG AA).
2. Glas-Flächen mit Text bekommen Mindest-Opazität; der Theme-Parameter
   `glass_min_opacity` ist pro Theme definiert und nie 0.
3. `Reduce Motion` schaltet alle Animationen ab (iOS liefert das als OS-Feature, nicht als
   App-Sonderwunsch).
4. `High Contrast`-Theme existiert neben den drei Brand-Themes.
5. Fenster-Eckenradius == Control-Button-Radius (Apple-Fix, kostet nichts).

### 4.5 KI-Integration statt KI-Fremdkörper

- **Aktionen statt Chat:** `hcs-actions` registriert `hcs.image.generate`,
  `hcs.rag.query`, `hcs.tor.toggle`, `hcs.update.apply` … Spotlight/Omnibar ruft sie auf.
  Der Agent (lokal, `hcs-coder`/`hcs-assistant`) **planiert und bestätigt** über HITL,
  genau wie bei Pentester-Aktionen (bestehendes Gate).
- **RAG ist ein Dienst:** `hcs-rag-ingest` bekommt Index-Status, GUI
  (`hcs-rag-ingest --gui`) und ein Evaluation-Skript (§7.5).
- **„Use Model" in Automationen:** v2.1, aber die Schnittstelle wird jetzt definiert
  (`hcs automation add --trigger <event> --action <action>`).

---

## 5. Wellenplan (jede Welle endet grün in allen Gates)

Jede Welle hat ein **DoD**, das wörtlich das Gate-Skript ist. Kein Merge ohne grünes Gate.
Wellen sind aufeinander aufbaubar; jede endet mit lauffähigem ISO.

### W0 · Wahrheit wiederherstellen (Halber Tag)

| # | Aufgabe | Datei |
|---|---|---|
| 0.1 | `cargo fmt` | `src/hcs-ui/tests/ui_tests.rs` |
| 0.2 | Clippy: `unwrap_or_else(hcs_ui::load_theme)` | `src/hcs-ui/src/bin/{hcs-docs,hcs-settings}.rs:15` |
| 0.3 | `hcs-docs`-Kollision lösen: Bash-Skript nach `hcs-docs-html` umbenennen, GUI behält `hcs-docs`; `.desktop` zeigt auf GUI, neues `hcs-docs-html.desktop` für den Browser-Portal | `scripts/build_iso.sh`, `config/includes.chroot/usr/share/{applications,hcs/scripts}` |
| 0.4 | `mkdir` vervollständigen: `applications`, `docs`, `docs/manuals`, `icons`, `wallpapers` | `scripts/build_iso.sh:31` |
| 0.5 | **Payload-Vertrag**: `scripts/verify_payload.sh` prüft jede Pflichtdatei im SquashFS; `build_iso.sh` **bricht ab** statt `|| true` | neu |
| 0.6 | `auto/config` auf 2.0.0 (`--iso-application`, `--iso-volume`) | `auto/config` |
| 0.7 | README/QA-Status auf wahre Zahlen korrigieren, GUI-Gap ins prominenteste Abschnitt | `README.md`, `docs/V1_STABLE_QA_STATUS.md` |
| 0.8 | `.gitignore`-Check: `dist/*.iso` bleibt draußen, `dist/*.iso.sha256` **muss** getrackt werden | `.gitignore` |
| 0.9 | Push + CI grün auf dem GUI-Code (schließt B-06) | `.github/workflows/ci.yml` |

**DoD:** `make gate-all` grün, `verify_payload.sh` grün, CI grün, `git status` sauber bis auf
die bewusst uncommitted Belege.

### W1 · Die Live-Sitzung (2–3 Tage) — schließt B-05

| # | Aufgabe |
|---|---|
| 1.1 | `hcs-core.list.chroot`: `niri`, `quickshell`, `seatd`, `pipewire`, `wireplumber`, `xdg-desktop-portal{,-wl,-gtk}`, `plymouth`, `systemd`-user-Session, `fonts-dejavu-core`, `fonts-noto-cjk`, `accountsservice`/`greetd` |
| 1.2 | Wayland-only: kein X-Server im Image; `XWayland` bleibt **abschaltbar** (niri-Flag) |
| 1.3 | Plymouth-Theme inkl. Fortschritt (das deterministische Boot-Bild, das VBox zuverlässig sieht) |
| 1.4 | Session-Autostart: `hcs-guest-session` startet `niri` + `quickshell -p /usr/share/hcs/shell` + `hcsd` + `hcs-modeld`; Rechte-Verzeichnis + Log-Rotation |
| 1.5 | `hcs-installer`/`init` wählen Live-Modus (autologin `hcs`) vs. Installer-Modus |
| 1.6 | Erste Anmeldung ohne Display vorhanden: `--tty1`-Fallback bleibt, damit VBox-Stufen 01–16 nicht brechen |
| 1.7 | **First-Run-Assistent** (Mint-Welcome-Muster): Codecs, Firewall, Theme, Wallpaper, AI-Profil, Snapshot, Agent-Setup — 5 Schritte, überspringbar |
| 1.8 | Exakte Bruchskalierung + Skalierungs-Presets (1.0 / 1.25 / 1.5 / 2.0) |
| 1.9 | Kein Gast-Login (Mint-Regel), `hcs`-User mit sudo, LUKS beim Install |

**DoD:** ISO bootet in einer grafischen Sitzung. Neuer Beweis: VBox-Stufe
`20_live_desktop.png` zeigt den echten Desktop; Entropie-Gate ≥1.5 **und**
Unique-Color-Gate (siehe §7.2).

### W2 · Fenster, Dateien, Werkzeuge (4–5 Tage) — schließt B-07..B-09, B-11

| # | Aufgabe | Crate |
|---|---|---|
| 2.1 | Snap Layouts (Flyout bei Hover über Maximieren, `Win+Z`), Snap Assist, Snap Groups | `hcs-shell`, `hcsd` |
| 2.2 | Virtual Desktops (1–10, `Win+Ctrl+←/→`, `Win+Ctrl+D`, Schalter in der Taskbar) | `hcs-shell` |
| 2.3 | Tiling-Modus (`Super+Alt+T`) + Free-Floating-Modus (§4.2) | niri-Konfig |
| 2.4 | Alt+Tab / Super+Tab mit **Live-Preview** (Screenshot-Slice des Fensters) | `hcs-shell`, `hcs-shot` |
| 2.5 | Task View mit Overlay-Thumbnails | `hcs-shell` |
| 2.6 | Stage-Mode (eine App vorn, Rest gestapelt links, gruppierbar) | `hcs-shell` |
| 2.7 | `hcs-fm`: Datei-/Ordnerbaum, Suche mit **Filter-Pills + Datums-Kalender** (GNOME-Muster), Tabs, „Open With", Default-App pro Typ, eigene Ordner-Icons/Farben, Tags, Trash mit Papierkorb-Taste, `Ctrl+.` → Terminal, Preview-Pane für Text/PNG/MD | `hcs-fm` |
| 2.8 | `hcs-term`: `vte`-Parser, Grid-Render in Slint, Tabs/Splits, `Alt+,` Profilsuche (`hcs dev`-Profile), Copy-on-Select, Link-Unterstreichung | `hcs-term` |
| 2.9 | `hcs-shot`: `--region`, `--window`, `--screen`, `--delay`, `--extract` (OCR), `--color`, `--record`; Wayland-Portal + X-Fallback; globales Hotkey `Print` | `hcs-shot` |
| 2.10 | `hcs-notes`: Markdown nativ, Auto-Save, Session-Restore, `cheatsheet.json`-Import | `hcs-notes` |
| 2.11 | `hcs-docs` bleibt der native Manuals-Viewer; HTML-Portal als `hcs-docs-html` | `hcs-ui` |

**DoD:** §7 Gates 1–8 grün **inklusive** neuer View-Referenzen
(`fm`, `term`, `shot`, `notes`, `snap-layouts`, `stage`, `taskview`, `virtual-desktops`)
für alle 3 Themes.

### W3 · Omnibar: Finden und Tun (3 Tage) — schließt B-10

| # | Aufgabe |
|---|---|
| 3.1 | `hcs-actions`: Registry. Jede App registriert `id`, Titel, Kategorie, Keyboard-Shortcut, Vorschau-Renderer, `execute()`. |
| 3.2 | Eine Result-Liste: **Apps, Dateien, Memory (FTS5), Fenster, Tabs, Einstellungen, Aktionen, Web (nur wenn erlaubt)** — alles together, gerankt (FTS5 + Häufigkeit + Letzte-Nutzung) |
| 3.3 | Quick Keys: `sm` → senden, `ar` → erinnern, `t` → Terminal, `?` → Hilfe-Palette. Konfigurierbar. |
| 3.4 | `↑` = Suchhistorie (persistiert, in Amnesie-Modus nicht) |
| 3.5 | Browse-Views wie macOS: `Ctrl+1` Apps, `Ctrl+2` Dateien, `Ctrl+3` Aktionen, `Ctrl+4` Memory |
| 3.6 | Vorschau-Pane (Text, Bild, Manuals-Abschnitt, App-Aktion) ohne Öffnen |
| 3.7 | Aktionen mit HITL-Bestätigung, wenn sie privileged sind (Konsistenz zum Pentester-Gate) |
| 3.8 | Tastatur-only: vollständig bedienbar, Fokus sichtbar, Test als Konstanten-Test |

**DoD:** `hcs-search` Unit-Tests: Ranking, Quick-Key-Auflösung, History, Aktion-Dispatch,
HITL-Blockade. GUI-Referenzen pro Theme.

### W4 · Persönliches System (3 Tage) — schließt B-14, B-17 (teilweise)

| # | Aufgabe |
|---|---|
| 4.1 | Theme-Atomizität nach §4.3, inkl. Live-Preview |
| 4.2 | **Control Center** (macOS-Galerie): Module rein/raus (Netz, Tor, Audio, Helligkeit, AI-Profil, RAM-Budget, Bluetooth, Nachtmodus, Kamera, Screenshot) — persistiert |
| 4.3 | **Notification Center**: Auto-Gruppierung, Live-Activity-Chips (Android-Muster), DND/Focus-Sessions (Win11 + GNOME), Lock-Screen-Mediensteuerung |
| 4.4 | **Widget-Board** + Lock-Screen-Widgets mit Auto-Vorschlag (Windows) |
| 4.5 | **Dynamic Color**: Akzent aus Wallpaper ableiten (Material-You-Muster), pro Theme übersteuerbar |
| 4.6 | Per-Monitor-Helligkeit, Night-Light-always-on |
| 4.7 | **Predictive Close**: Hover über Schließen zeigt Ziel-Vorschau (Desktop / vorheriges Fenster / Warnung bei ungespeichert) |
| 4.8 | Killswitch sichtbar in Control Center + Taskbar-Pill, Regel-Count aus `TorTransparentProxy` |
| 4.9 | Keyboard-first-Komplettierung: `Super+Space` Launcher, `Super+Alt+Space` System-Menü (Omarchy-Muster) — **jede** Shell-Funktion per Taste |

**DoD:** Theme-Wechsel in ≤1 s systemweit sichtbar (Screenshot-Vorher/Nachher im Report),
`glass_min_opacity`-Gate grün, alle Shell-Aktionen per Taste erreichbar (Test: Cheatsheet
ist vollständig, jedes Element hat einen Shortcut).

### W5 · Update, Rollback, Persistenz (2–3 Tage) — schließt B-12, B-13

| # | Aufgabe |
|---|---|
| 5.1 | `hcs-update` mit **Level 1–5**, automatisch/manual je Level (Mint), Reboot-Pflicht-Warnung |
| 5.2 | Pre-Update-Snapshot (Btrfs `btrfs subvolume snapshot` + GRUB-Eintrag), Rollback aus dem **Bootmenü** (Omarchy-Muster) |
| 5.3 | APT + Flatpak in einer Oberfläche, ohne Format-Denkfrage |
| 5.4 | **Amnesie-Modus** (Tails): `nopersistence`, `swapon`→no-op, `init_on_free=1`, RAM-Nullung bei Shutdown **und** bei USB-Entfernung (`udev-watchdog`), kein Host-Dateisystem |
| 5.5 | **Opt-in Persistent Storage**: LUKS-Partition auf dem Zielmedium, Feature-Liste, `active/enabled/masked`, Unlock im First-Run, `nosymfollow`-Härtung |
| 5.6 | LUKS-Wortlisten-Generator (5–7 Wörter) statt Hexdump |
| 5.7 | `docs/THREAT_MODEL.md` (Tails-Muster): Was schützen wir, gegen wen, was wir **nicht** können |
| 5.8 | Dual-Boot-Install in freie Partition mit eigener EFI (Omarchy-Muster) |

**DoD:** Snapshot→Update→Rollback in VBox durchgespielt, Screenshot-Beleg;
Amnesie-Test: nach Shutdown keine Spuren (Kriterium in `qa/reports/amnesia.json`).

### W6 · Agent, RAG, Recall (3–4 Tage) — schließt B-15, B-16

| # | Aufgabe |
|---|---|
| 6.1 | `hcs`-CLI als Agenten-Kontrollfläche: **jeder** Unterbefehl mit `--json`, stabile Exit-Codes, `hcs help --json` (Omarchy-Muster) |
| 6.2 | `hcs-agent-bridge`: Crash-Benachrichtigung anklicken → Agent analysiert Dump → Report (Omarchy-Muster). HITL bei jeder Mutation. |
| 6.3 | Fortschritts-UI für Agentenläufe (GEMINI §153) im Notification Center als Live-Activity |
| 6.4 | `hcs-rag-ingest --gui`: Dokumente indexieren, Chunk-Übersicht, Index-Status, Rebuild |
| 6.5 | `hcs-recall`: lokale verschlüsselte Zeitleiste (SQLite + `age`-verschlüsselt), Snapshots nur bei Opt-in, **Sensitive-Info-Filter** (Passwort-/Kreditkarten-Muster ausgeschlossen), Suche semantisch über `hcs-memory`, `Win+J` |
| 6.6 | RAG in die Omnibar: „was steht in den Manuals über X" → Antwort mit Quellenbeleg |
| 6.7 | **Evaluation** (siehe §7.5) als Release-Gate |

**DoD:** `qa/reports/rag_eval.json` und `qa/reports/agent_eval.json` grün;
Recall-Test: Suche findet einen Session-Inhalt von vor 30 Minuten, ohne Cloud.

### W7 · Barrierefreiheit & KI-Qualität (2 Tage) — schließt B-17

| # | Aufgabe |
|---|---|
| 7.1 | `hcs-a11y`: Screenreader-Bridge (Slint `AccessibleItem`), Fokus-Reihenfolge pro View, Skip-Links |
| 7.2 | Fokus-Walk-Test als **Konstanten-Test** (Plan §4 Schritt 7, bisher nicht implementiert) |
| 7.3 | WCAG-Kontrast-Assert für alle Theme-Farben-Paare |
| 7.4 | `Reduce Motion`, `High Contrast`, Textskalierung 100–200 % ohne Layoutbruch |
| 7.5 | Textskalierung konsistent (Android-Befund: konsistentes Scaling über Formfaktoren) |
| 7.6 | CLI barrierefrei: `hcs --help` ist vollständig, `cheatsheet.json` vollständig, jede Fehlermeldung erklärt den nächsten Schritt (GEMINI §152) |
| 7.7 | Visual Intelligence / Highlight-to-Search: Auswahl im Fenster → `hcs-shot --extract` → Omnibar |

**DoD:** §7 Gates 9–10 grün.

### W8 · Release-Härtung (1–2 Tage)

| # | Aufgabe |
|---|---|
| 8.1 | VirtualBox 24+1 Stufen, alle mit echten GUI-Pixeln |
| 8.2 | `docs/GUI_RELEASE_CHECKLIST.md` vollständig abgearbeitet und abgehakt |
| 8.3 | `SHA256SUMS` + `SHA256SUMS.gpg` (nur mit Maintainer-Key; **nicht fälschen**) |
| 8.4 | SBOM, THIRD-PARTY-NOTICES (inkl. Slint-Lizenz), BUILD/MODEL-Manifeste |
| 8.5 | README ehrlich: was ist GUI-verifiziert, was hostseitig, was ist manuell |
| 8.6 | PR `dev`→`main`, GitHub Release mit allen Assets, CI grün |

---

## 6. RAM-Budget v2 (verbindlich, `hcs-monitor --json` ist die Quelle)

| Zustand | Budget | Messmethode |
|---|---|---|
| Shell (Quickshell, 3 Themes) | ≤ 220 MB idle | `/proc/<pid>/smaps_rollup` |
| Jede GUI-App einzeln | ≤ 250 MB RSS | `gui_ram_audit.py`, je Binary |
| `hcs-fm` mit 10k Dateien im Ordner | ≤ 250 MB | eigener Testfall |
| `hcs-term` mit 200 k Scrollback | ≤ 220 MB | eigener Testfall |
| `hcs-recall` nach 10k Snapshots | ≤ 300 MB | eigener Testfall |
| RAG-Index (500 Manuals, 2 MB Chunk) | ≤ 700 MB | eigener Testfall |
| System gesamt idle (Desktop + Shell) | ≤ 6144 MB | Summen-Matrix |
| System gesamt peak (Chat + Monitor + RAG + FM) | ≤ 8192 MB | Summen-Matrix |
| Image-Generierung (SD 1.5 LCM Q4, 512²) | ≤ 2200 MB transient | bestehendes Gate |

Verletzung ⇒ kein Merge (Plan §3 bleibt bindend).

---

## 7. Test- und Prüfsystem (der Kern von v2)

### 7.1 Gate-Übersicht

| Gate | Inhalt | Skript | Sperrt |
|---|---|---|---|
| **G1 Static** | `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` | `scripts/gate_static.sh` | kein Build |
| **G2 Security & License** | Secret-Scan, Least-Privilege, Tor-Policy, Root-Verbot, `THREAT_MODEL.md` gepflegt, Lizenz-Disclosure vorhanden | `run_security_audit.py` | kein Release |
| **G3 Supply Chain** | `verify_sources.py` (100 % gepinnt), SBOM, Notices | `verify_sources.py` | kein Release |
| **G4 Payload** | ISO bauen, **Payload-Vertrag** (jede Pflichtdatei im SquashFS), ISO9660, Größen-Budget | `build_iso.sh` + `verify_payload.sh` + `verify_iso.py` | kein Release |
| **G5 Host-GUI** | Headless-Render 9+ Views × 3 Themes, Visual-Regression, Blank-Frame-Detektor | `verify_gui.py` | kein Merge |
| **G6 GUI-RAM** | §6-Matrix | `gui_ram_audit.py` | kein Merge |
| **G7 Wayland** | Weston headless, echtes Fenster, 60 s Laufzeit, Screenshot-Datei vorhanden | CI-Job `gui-wayland` | kein Release |
| **G8 VM-E2E** | VirtualBox 25 Stufen, echte GUI-Pixel | `qa_virtualbox.ps1`, `qa_virtualbox_install.ps1` | kein Release |
| **G9 RAG** | Retrieval-Qualität, Grounding, Zitate, Refusal | `rag_eval.py` (neu) | kein Release |
| **G10 A11y** | Fokus-Walk, WCAG-Kontrast, Reduce-Motion, Textskalierung | `a11y_gate.py` (neu) | kein Release |
| **G11 Stress** | 1000 Zyklen, 0 Crash/OOM/FD-Leak/SQLite-Fehler | `run_stress_test.py` | kein Release |
| **G12 CI** | alles oben in GitHub Actions grün | `ci.yml` | kein Release |
| **G13 Manuell** | `GUI_RELEASE_CHECKLIST.md` 20/20, Release-Freeze | manuell | kein Release |

**Neu gegenüber v1:** G4-Payload-Vertrag, G9, G10, G13. G4 verhindert B-03/B-04
strukturell, nicht durch Sichtprüfung.

### 7.2 Der Blank-Frame-Detektor (gelernt aus v1)

Die v1-Regression: Bild-Entropie **bewertete** drei echte Bugs als PASS, weil eine
Vollton-Fläche 1.6 Entropie erreicht. Deshalb gilt ab sofort für jedes gerenderte Bild:

| Prüfung | Schwelle | Fängt |
|---|---|---|
| Entropie | ≥ 0.5 | komplett schwarzes Bild |
| **Unique-Color-Count** | ≥ 8 | Vollton-Fläche, unsichtbare Widgets |
| Größe | ≥ 1500 B | leerer Buffer |
| **Text-Pixel-Anteil** | ≥ 0.3 % | gerenderte Grafik ohne eine einzige Textzeile |
| Mean-Diff gegen Referenz | ≤ 2.0 | Layout-Drift |

Der **Text-Pixel-Anteil** ist neu und fängt den Fall „Fenster rendert, aber die
Textschicht fehlt" (genau der Slint-Globals-Bug aus P0).

### 7.3 Deterministischer Gast-QA-Treiber (`hcs-qa-agent`)

Das Kernproblem von v1: Screenshots aus der VM waren **zeit- und zufallsabhängig**
(schwarze Framebuffer, "40 s Verzögerung" als Fix). v2 macht die VM zum deterministischen
Ziel:

1. `hcs-qa-agent` liegt **im Image** (`/usr/bin/hcs-qa-agent`), gesetzt vom Live-Build.
2. Startet automatisch bei `hcs.qa=1` an der Kernel-Cmdline (nur in Test-ISOs).
3. Läuft als `systemd`-User-Service im Hintergrund, hört auf einem Unix-Socket.
4. Der Host (`qa_virtualbox*.ps1`) sendet Szenarien:
   `wait_session · shot <name> · key <scancode-seq> · type <text> · open <app> ·
   assert_window <title> · assert_pixel <name,region> · rss · exit`
5. Der Agent **wartet** auf definierte Signale (XDG-Dienst bereit, Fenster-gekartet,
   Animation beendet) statt auf `sleep`.
6. Jeder Schritt schreibt ein Journal (`/var/log/hcs/qa/journal.jsonl`) → im Gast-Log
   sichtbar, im ISO-Log extrahierbar.
7. Host prüft: Journal vollständig, kein Schritt `timeout`, Screenshots mit Entropie-
   und Unique-Color-Gate, optional OCR-Text-Assert (`assert_text "HCS Linux"`).

Ergebnis: **kein** `sleep`-Wettkampf mehr, Screenshots sind reproduzierbar, und ein
Fehlschlag sagt *wo* er passiert ist.

### 7.4 VirtualBox-Matrix (Gate 8)

| # | Stufe | Neu? |
|---|---|---|
| 01–08 | GRUB → Calamares → Installation | unverändert |
| 09–16 | Installierter VDI-Boot, Desktop, Start, Cheatsheet, Image Studio, Tor-Pill, Security Lab, Docs | unverändert |
| 17 | Plymouth-Bootprogress | neu |
| 18 | Live-Desktop in der Sitzung | neu (war Konsole) |
| 19 | **Taskbar + Tray** (Modell, RAM, Tor-Pill) | neu |
| 20 | **Snap-Layout-Flyout** offen | neu |
| 21 | **Omnibar** mit Query + Treffer + Aktion | neu |
| 22 | **Virtual Desktops** (2. Desktop aktiv) | neu |
| 23 | **Control Center** offen | neu |
| 24 | **Notification Center** mit Live-Activity | neu |
| 25 | **Dateimanager** mit Tabs + Suche | neu |
| 26 | **Terminal** + `htop` | neu |
| 27 | **Screenshot-Snipping** mit Region + OCR | neu |
| 28 | **Theme-Wechsel** live (Vorher/Nachher) | neu |
| 29 | **AI-Chat** mit echter Inferenz | neu |
| 30 | **Image-Studio** echter Render | unverändert (Inhalt), echte GUI |
| 31 | **RAG-Frage** mit Quellenbeleg sichtbar | neu |
| 32 | **Rollback-Bootmenü** sichtbar | neu |

VM-Matrix: `EDGE-8GB` (8 GB RAM, 4 CPU) und `LOWRAM-4GB` (4 GB RAM, 2 CPU).
Selbstheilung: max 3 Retry-Loops pro Stufe mit klassifizierten Heuristiken
(schwarzes Bild → Retry mit längerem Settle; falsches Fenster → falscher Compositor;
Crash → Log-Extraktion + Stopp mit Diagnose-Bundle).

### 7.5 RAG- und KI-Evaluation (Gate 9)

`tests/rag/` + `scripts/rag_eval.py`, Ergebnis `qa/reports/rag_eval.json`:

| Metrik | Schwelle | Testset |
|---|---|---|
| Recall@5 über `hcs-memory` | ≥ 0.80 | 100 Gold-Items |
| MRR | ≥ 0.70 | dasselbe |
| Grounded-Answer-Rate (Antwort stützt sich auf Quellen) | 100 % | 60 Fragen |
| Quellenbeleg vorhanden | 100 % | 60 Fragen |
| Refusal bei Unbekanntem | 100 % | 25 Negativfragen |
| Zitier-Präzision (Abschnitt existiert wirklich) | 100 % | 60 Antworten |
| Latenz p95 (Retrieval, ohne Inferenz) | ≤ 300 ms | 200 Queries |
| Chunking stabil bei 6 Manuals + 40 Nutzerdokumente | keine Trunkation | 46 Dateien |

`scripts/agent_eval.py` → `qa/reports/agent_eval.json`:

| Metrik | Schwelle |
|---|---|
| Aktion-Dispatch korrekt (Absicht → Aktion) | ≥ 0.90 über 120 Szenarien |
| HITL-Blockade bei privileged Aktionen ohne Bestätigung | 100 % |
| Single-Heavy-Model-Regel eingehalten | 100 % |
| RAM-Gate verhindert Überlast | 100 % |
| Agent bricht sauber ab bei Fehler (kein Hänger) | 100 % in 50 Fehlerszenarien |

### 7.6 Der autonome Build-Loop (GEMINI §170–§172, verschärft)

```
REPEAT
  1 fetch      → verify_sources.py, fetch_models.py --verify-only
  2 build      → cargo build --release --workspace, build_iso.sh (bricht bei Payload-Lücke ab)
  3 static     → G1
  4 host-gui   → G5, G6 (alle Themes)
  5 vm         → G8 (Matrix, Selbstheilung max 3)
  6 rag-ai     → G9
  7 a11y       → G10
  8 stress     → G11
  9 classify   → Jeder Fehlschlag wird einem Fehler-Taxonomie-Eintrag zugeordnet
                 (GEMINI §137). Bekannt → Fix anwenden, Retry. Unbekannt → STOPP.
 10 report     → qa/reports/*.json, Diff gegen Vorgänger-Lauf
UNTIL alle Gates grün ODER Retry-Budget erschoepft ODER unbekannter Fehler
```
**Terminierungsregel (bindend):** nach **3** Retry-Loops oder **einem** unbekannten
Fehler stoppt der Loop und schreibt `qa/reports/BLOCKED.md` mit vollständigem Bundle.
Kein Selbst-Fix-Versuch an Gate-Skripten, kein Threshold-Absenken, kein
Referenzbild-Update ohne Review.

---

## 8. Reihenfolge, Abhängigkeiten, Aufwand

```
W0 Wahrheit (0.5 d)
 └─> W1 Live-Sitzung (2-3 d)          ← ohne das ist G8 unmöglich
      └─> W2 Fenster/Fm/Term/Shot (4-5 d)
           ├─> W3 Omnibar (3 d)
           └─> W4 Persönliches System (3 d)
                ├─> W5 Update/Persistenz (2-3 d)
                ├─> W6 Agent/RAG/Recall (3-4 d)
                └─> W7 A11y (2 d)
                     └─> W8 Release-Härtung (1-2 d)
```
**Summe:** ~21–26 Tage Arbeit inkl. QA-Läufe (VirtualBox-Läufe dominieren).

**Kritischer Pfad:** W0 → W1 → W2 → W8. Alles andere ist parallelisierbar.

**Risiken**

| Risiko | Wirkung | Gegenmaßnahme |
|---|---|---|
| niri-Free-Floating widerspricht Tiling-Voreinstellung | Snap wirkt unzuverlässig | Modusumschalter, `hcsd`-IPC-Test mit Dummy-Fenstern **vor** UI-Arbeit |
| `hcs-qa-agent` braucht systemd im Image, das Image ist busybox-basiert | G8 bleibt unzuverlässig | Dual-Mode: `hcs-qa-agent` läuft auch unter dem bestehenden busybox-`init` (Polling statt Socket) |
| 3D-Glas-Rendering auf llvmpipe | Performance unter 6 GB | Software-Renderer erzwingen, RAM-Gate beobachten, Themes mit reduzierter Blur-Stufe |
| Slint-Lizenz bei neuen Crates | Build bricht | `vendor/locks/sources.lock.yaml` + Lizenz-Begründung bei **jedem** neuen Slint-Modul |
| RAG-Qualität ohne echte Modelle im CI | Gate 9 nicht reproduzierbar | Gate 9 nutzt deterministische Mock-Embeddings für Retrieval + optional echte Inferenz nur im VM-Lauf |
| Umfang | 2.0 wird zu 2.5 | Jede Welle ist einzeln release-fähig; bei Zeitdruck wird W6/W7 in ein 2.1.0 verschoben, **W0–W5 nicht** |

---

## 9. Was v2 **nicht** ist

- Kein Snap/Flatpak-Zwang, kein Fremd-Store.
- Keine Cloud, kein Telemetrie, kein Recall-Server.
- Keine X11-Session.
- Kein GPU-Zwang (Software-Renderer ist Pflichtpfad).
- Keine erfundenen Belege: Wenn etwas nicht geprüft wurde, steht das in
  `docs/V1_STABLE_QA_STATUS.md` bzw. `qa/reports/BLOCKED.md` — mit Datum.
- Kein Umbenennen von v1-Features, nur Ergänzen.

---

## 10. Nach v2.0.0 (Roadmap, nicht Teil dieses Plans)

| Version | Inhalt | Herkunft |
|---|---|---|
| 2.1 | Continuity: Handoff zwischen Geräten, Universal Clipboard, Remote-Desktop mit Multi-Touch, Web-App-Installer, Spices-Plugin-Ökosystem, „Use Model" in Automationen, Haptic Slider | macOS, GNOME, Omarchy, Android |
| 2.2 | AI-First-Runtime: Teach/Student-Distillation, on-device fine-tuning (GEMINI §133), Vision-Pipeline | GEMINI |
| 2.3 | Mobile-Formfaktor: HCS-Client mit Live-Updates, Widgets, Pairing | Android/iOS |

---

## 11. Vollständige Dateiliste (was v2 anfasst / neu erstellt)

**Neu — Crates**
`src/hcs-actions/`, `src/hcs-fm/`, `src/hcs-term/`, `src/hcs-shot/`, `src/hcs-notes/`,
`src/hcs-update/`, `src/hcs-persist/`, `src/hcs-recall/`, `src/hcs-agent-bridge/`,
`src/hcs-a11y/`, `src/hcs-qa-agent/`

**Neu — Scripts**
`scripts/gate_static.sh`, `scripts/verify_payload.sh`, `scripts/rag_eval.py`,
`scripts/agent_eval.py`, `scripts/a11y_gate.py`, `scripts/qa_virtualbox_scenarios.ps1`

**Neu — Tests**
`tests/rag/{gold_items.json,questions.json,negatives.json}`,
`tests/agents/scenarios.json`, `tests/a11y/focus_order.rs` (Konstanten-Test)

**Neu — Docs**
`docs/THREAT_MODEL.md`, `docs/V2_RELEASE_CHECKLIST.md`, `docs/V2_QA_STATUS.md`

**Neu — Konfig/Payload**
`/etc/hcs/theme/{colors.toml,templates/}`, `config/includes.chroot/usr/share/applications/hcs-*.desktop`
(+ `hcs-fm`, `hcs-term`, `hcs-shot`, `hcs-notes`, `hcs-docs-html`),
`config/package-lists/hcs-core.list.chroot` (niri/quickshell/portal),
`config/hooks/live/02-hcs-session.hook.chroot`

**Geändert**
`scripts/build_iso.sh` (Payload-Vertrag, harte Fehler), `auto/config`,
`.github/workflows/ci.yml` (+ Jobs `gui-views`, `rag`, `a11y`, `payload`),
`src/hcs-shell/*` (+ `control_center.qml`, `notification_center.qml`, `widgets.qml`,
`snap_flyout.qml`, `taskview.qml`, `stage.qml`), `src/hcs-ui/ui/hcs_ui.slint`,
`src/hcsd` (Fenster-/Sitzungs-IPC), `Makefile` (`gate-all`, `qa-vm`, `qa-rag`),
`README.md`, `CHANGELOG.md`, `Cargo.toml` (Version 2.0.0)

---

## 12. Definition of Done für v2.0.0 Stable

1. `make gate-all` grün: G1–G13, ohne Threshold-Absenkung, ohne `|| true`.
2. `verify_payload.sh` bestätigt jede Pflichtdatei im ausgelieferten ISO.
3. VirtualBox: **alle 25+ Stufen** mit echten GUI-Pixeln, Entropie **und**
   Unique-Color **und** Text-Pixel-Anteil bestanden, Journal lückenlos.
4. `docs/V2_RELEASE_CHECKLIST.md` 20/20 abgezeichnet.
5. `qa/reports/rag_eval.json` + `agent_eval.json` in den Gates grün.
6. RAM-Matrix §6 eingehalten, Matrix-Summe ≤ 6144 idle / ≤ 8192 peak.
7. ISO `HCS-Linux-2.0.0-amd64.iso` mit SHA-256, SBOM, Notices, Manifesten,
   Release-Seite mit allen Assets.
8. README beschreibt exakt den geprüften Stand — inklusive der Punkte, die
   **nicht** geprüft wurden.
9. `SHA256SUMS.gpg` vorhanden **oder** die Abwesenheit ist im Release explizit
   begründet. Nicht fälschen.
10. `docs/GUI_BUILD_PLAN.md` als erfüllt markiert, `docs/V2_STABLE_RELEASE_MASTER_PLAN.md`
    als erfüllt markiert.

---

## 13. Quellen des Feature-Audits

- Apple — macOS Tahoe / macOS 27 Golden Gate Feature-Overviews (`apple.com/os/pdf/All_New_Features_*`,
  `support.apple.com/en-us/127257`), Stage Manager (`support.apple.com/en-ca/guide/mac-help/mchl534ba392`)
- Cult of Mac — Continuity-Feature-Übersicht (Handoff, Universal Control/Clipboard, iPhone Mirroring)
- WIRED / MacRumors — Liquid-Glass-Revisionsfolge (Lesbarkeits-Korrekturen)
- Microsoft — Windows 11 Workflows (`microsoft.com/en-us/windows/learning-center/windows-11-transforms-workflows`),
  Snap/Snap Groups/Snap Assist (`support.microsoft.com/en-us/windows/experience/snap-your-windows`),
  Recall (`support.microsoft.com/en-us/windows/ai/ai-features/retrace-your-steps-with-recall`),
  Windows Experience Blog 2025 (Click to Do, Snipping Tool, Settings-Agent, Start-Redesign)
- GNOME 49 Release Notes (`release.gnome.org/49`), OMG!Ubuntu / It's FOSS / Phoronix
  (Wayland-only, Quick-Settings-Reorganisation, Nautilus-Such-Popover, Lock-Screen-Mediensteuerung,
  Accessibility-Menü, per-Monitor-Helligkeit, exakte Bruchskalierung)
- Linux Mint 22 / 22.2 / 22.3 What's New (`linuxmint.com/rel_*_whatsnew.php`),
  FOSS Linux (Update-Level, Timeshift, APT+Flatpak, Spices)
- Tails — `tails.net/about/`, `/doc/persistent_storage/`, `/contribute/design/`
  (Amnesie, Persistent Storage, `init_on_free`, `udev-watchdog`, `nosymfollow`, Threat Model)
- Omarchy — `omarchy.org`, The Omarchy Manual (`learn.omacom.io`), DHH-Blog (`world.hey.com/dhh`),
  Omarchy-4.0-Architektur (`colors.toml`, `omarchy`-CLI, Hooks, Quickshell-Shell, Btrfs/Limine/LUKS)
- Android 16 — `developer.android.com/about/versions/16/{features,summary}`,
  `android.com/articles/android-16-features`, Android Police / Android Authority / Android Central
  (Live Updates, Auto-Gruppierung, Predictive Back, Desktop-Windowing, Taskbar-Overflow,
  Material You, Edge-to-edge, Text-Scaling, Advanced Protection)
- iOS 26 — `apple.com/os/pdf/All_New_Features_iOS_26_Sept_2025.pdf`,
  `support.apple.com/guide/iphone/whats-new-in-ios-26-iphfed2c4091/ios`,
  MacRumors Liquid-Guide, Wirecutter (Dynamic Tab Bars, Focus, Live Activities, Visual Intelligence)

---

## 14. Wartungsmodus für dieses Dokument

Dieses Dokument ist die **einzige** Quelle für v2-Reihenfolge. Abweichungen brauchen eine
begründete Änderung hier, nicht in Code-Diskussionen. Jede Welle aktualisiert
`docs/V2_QA_STATUS.md` mit Datum, Gate-Ergebnis und Screenshot-Belegen.
Wenn ein Gate grün ist, aber kein Beleg existiert, gilt es als **rot**.
