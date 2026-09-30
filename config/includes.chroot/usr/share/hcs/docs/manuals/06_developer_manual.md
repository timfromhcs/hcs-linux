# Developer Manual

The developer workflow is keyboard-first, and every tool in it is reachable from
the CLI as well as the editor — so an agent can use it too.

## Toolchains in the image

| Tool | Notes |
|---|---|
| Rust (rustc, cargo, clippy, rustfmt) | Matches the workspace toolchain |
| Node.js, npm | For the shell's QML tooling |
| Python 3.12 | With PyYAML, Pillow, psutil |
| Neovim + LSP | Preconfigured |
| VSCodium | Available as an alternative editor |
| GDB, LLDB | Wrapped by `hcs dev debug` |
| tmux, git, tmux | |

## The hcs CLI

```bash
hcs dev init rust      # scaffold with CI templates and pre-commit hooks
hcs dev test           # unit, integration and lint checks in a sandbox
hcs dev debug hcsd     # wrapped debugger with terminal visualisation
```

## Profiles

`hcs-term` ships profiles rather than a bare shell, because a developer rarely
wants just "a shell":

```bash
hcs-term --profiles                 # list
hcs-term --profile rust             # open a Rust shell
hcs-term --profile workstation      # the WORKSTATION-16GB environment
hcs-term --profiles ai              # filter (Alt+, in the window)
```

Press `Alt+,` inside the terminal window to search profiles. The list includes
the AI profiles, so switching a shell to a different RAM budget is one action.

## Window management for development

```bash
hcs window snap --side left      # editor left, terminal right
hcs window snap --layout thirds  # three columns
hcs window taskview               # all windows and virtual desktops
hcs desktop new                   # a fresh virtual desktop per project
hcs window mode toggle            # floating (Windows-style) or tiling
hcs window stage toggle           # one app in front, the rest in a strip
```

Two modes, because niri is a tiling compositor and Windows users expect floating
windows with Snap Layouts. Both are available and the choice is yours per
session.

## Files

`HCS+E` opens the file manager, which is also a terminal hand-off:

* Search with visible filter pills, and narrow by date with `YYYY-MM-DD`
* Tabs, so several locations stay open
* `Open With` is an explicit choice, not a silent default association
* `HCS+.` opens the current folder in the terminal

```bash
hcs-fm --json                     # machine-readable listing
```

## Notes

`HCS+Shift+N` opens Markdown notes with offline search and session restore, so a
crash does not lose a draft.

## MCP

The native MCP server suite exposes the system's capabilities to external tools:

```bash
cat /etc/hcs/mcp_servers.json
```

`hcs-mcp-hub` speaks the Model Context Protocol (2024-11-05) and exposes tools
such as `fs_read_file`. Nothing leaves the machine; the point is interoperability,
not connectivity.

## Releases and gates

```bash
make gate-static      # fmt, clippy --all-targets, tests
make gate-supply      # pinned sources + starter-model staging report
make gui              # render, regression, RAM
make a11y             # focus order, WCAG contrast, motion
make payload          # the ISO payload contract
make gate-all         # everything, in order
```

The payload gate is the one that matters most for a release. v1 shipped an ISO
whose launchers, icons, wallpapers and manuals were silently missing, because the
copy step swallowed errors. `scripts/verify_payload.sh` now fails the build
instead, and CI checks the payload tree on every push.

## Building the ISO

```bash
make iso                       # target/rootfs → squashfs → hybrid ISO
scripts/verify_payload.sh target/rootfs 2.0.0
```

If a required file is missing, the build stops. That is the intended behaviour.
