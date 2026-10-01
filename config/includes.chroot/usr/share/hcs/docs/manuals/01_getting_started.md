# Getting Started with HCS Linux

HCS Linux is an AI-native, privacy-oriented, CPU-first desktop. Everything works
offline; the network is optional, and Tor is one keystroke away.

## The HCS key

The HCS key sits where the Windows key sits on a Windows keyboard, and it does
what the Windows key does — but it is called **HCS**. If you are coming from
Windows your muscle memory already works. If you are coming from Linux, `Super`
and `HCS` are the same key; every shortcut in this manual and in the on-screen
cheatsheet (`HCS+/`) uses the name HCS.

The HCS key is a modifier, so **no keyboard layout can move it**. Switching
between QWERTZ and QWERTY never breaks your shortcuts.

## First boot

1. Write the ISO to a USB stick and boot from it.
2. The Plymouth splash shows real progress, then the Neural Glass desktop starts.
   The session is live-session: your changes are gone at shutdown unless you
   install or enable persistence.
3. `hcs-welcome` appears with five short steps. Everything is skippable.
4. The default keyboard layout is QWERTZ (German). Press `HCS+Space` to cycle
   QWERTZ / EN-US / FR / ES / IT / GB.

## Desktop tour

| Element | What it does |
|---|---|
| Bottom taskbar | Window list with active underline, plus a tray showing model, RAM and the Tor pill |
| Start monogram (left) | Opens the Start Menu with an omnibar at the top |
| Taskbar search pill | The same omnibar without leaving the desktop |
| Cheatsheet HUD (`HCS+/` or `F1`) | Every keyboard shortcut, grouped |
| Tor pill | Green when the kill switch is armed, grey when not. Click to toggle. |

## The omnibar: find things **and** do things

One field, `HCS+Space`, handles both. It searches apps, files, cognitive memory,
windows, settings and *actions*, ranked together.

```bash
hcs search "ram budget"      # find a setting
hcs search "tor"             # find the toggle, then run it
hcs-actions quickkey to      # resolve a quick key
hcs-actions run privacy.tor.toggle --yes
```

Short queries resolve as **quick keys**: type `to` for the Tor toggle, `ch` for
chat, `ss` for a screenshot. Two keystrokes instead of menu hunting.

## The five most useful shortcuts

| Keys | Action |
|---|---|
| `HCS+Space` | Omnibar — find and act |
| `HCS+Return` | AI Chat |
| `HCS+I` | Image Studio (offline CPU image generation) |
| `HCS+D` | Documentation (native viewer, no browser) |
| `HCS+Alt+T` | Tor kill switch |

Press `HCS+/` for the complete list, or run `hcs settings keyboard` to see the
available layouts.

## Installing to disk

Choose **Install HCS Linux (Calamares)** from the boot menu. You can select
optional full-disk LUKS2 encryption (AES-XTS-512), and an AI profile:

| Profile | For | RAM |
|---|---|---|
| `LOWRAM-4GB` | Small laptops | 4 GB |
| `EDGE-8GB` | Typical desktop or laptop | 8 GB |
| `WORKSTATION-16GB` | Large models and image work | 16 GB |

Before each update the system takes a snapshot, and the previous state stays
reachable from the boot menu. A bad update is recoverable, not final.

## Where things live

| Path | What |
|---|---|
| `/usr/share/hcs/shell/` | The Neural Glass shell (niri + Quickshell) |
| `/usr/share/hcs/theme/colors.toml` | The one file that defines every theme |
| `/usr/share/hcs/docs/manuals/` | These six manuals, available offline |
| `/var/lib/hcs/models/` | Models, including anything baked into the image |
| `/var/lib/hcs/memory/` | The cognitive memory index |
