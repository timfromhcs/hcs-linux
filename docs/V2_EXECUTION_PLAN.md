# HCS Linux 2.0 — Execution Plan

From "it boots to a stall" to "it is verifiably a desktop, and `main` says so".

This plan is ordered by *what can be falsified fastest*, not by what is most
visually satisfying. Everything in Phase 1 is verifiable on this host within
minutes; everything in Phase 4 needs a physical machine and is marked as such
because it cannot be faked.

---

## Phase 0 — One boot answers the open question

The boot stops between `local-fs.target` and `multi-user.target`. Nothing has
observed that window from inside the guest. Everything else is downstream of it.

**Action:** rebuild with the diagnostics unit now ordered at `basic.target`, boot
once, read the evidence VHD.

**Answers:** what `local-fs.target` is waiting on. `systemctl list-jobs`, the
failed-units list and `dmesg` are all already in the dump.

**Exit criterion:** a named unit or mount that is blocking. If the dump is still
empty, the instrument itself is broken and that is the finding.

---

## Phase 1 — Local verification (no VM, minutes each)

Run these before touching the VM again, because they are free and they catch
whole classes of bug that a boot log cannot.

| Check | Command | Catches |
|---|---|---|
| Rust workspace | `cargo test --workspace` | logic regressions |
| Clippy | `cargo clippy --workspace -- -D warnings` | latent defects |
| Formatting | `cargo fmt --all --check` | churn |
| Python | `pytest tests/unit` | grader and gate logic |
| Shell syntax | `bash -n` on every `scripts/*.sh` | the class of bug in `build_base.sh` |
| Shellcheck | `shellcheck -S warning scripts/*.sh` | quoting, unset vars |
| Payload gate | `scripts/verify_payload.sh` against a staged tree | contract violations |
| Bindings | `scripts/verify_bindings.py` | duplicate or fake bindings |
| Package availability | `verify_package_availability.py --base debian:sid` | drift; still targets trixie |

**A regression test is owed for each bug found**, because every bug in the last
cycle was a *structural* fault — an unreached hook, a dangling link, a wrong
namespace — and none of them were covered.

---

## Phase 2 — VM visual, app and workflow testing

Once Phase 0 names the blocker and Phase 1 is green.

**2a. Visual**
- A screenshot in which the Neural Glass shell is drawn, graded by
  `grade_capture.py` running on the host.
- The capture gate must reject a console: that is its regression test and it
  already exists.

**2b. Apps** — each must open as a *window*, not merely exist as a binary:
`hcs-settings`, `hcs-term`, `hcs-docs`, `hcs-chat`, `hcs-fm`, `hcs-notes`,
`hcs-shot`, `hcs-actions`, `hcs-monitor`, `hcs-control`, `hcs-search`,
`hcs-diagnose`.

**2c. Workflows** — the QA suite split into four boot cohorts, because GRUB, the
installer, an installed system and amnesic mode are separate boots by definition
and cannot be stages of one desktop session:

| Cohort | Cmdline | Proves |
|---|---|---|
| `live` | `hcs_session=graphical hcs.qa=1` | session + all GUI stages |
| `installer` | `hcs_install=1 hcs.qa=1` | Calamares GUI end to end |
| `installed` | normal boot from disk | the installed system draws |
| `amnesic` | `hcs_amnesic=1 hcs.qa=1` | nothing persists |

Each cohort gets its own GRUB entry, its own evidence disk and its own verdict.
Aggregate GREEN only if all four are.

---

## Phase 3 — Fix what Phase 2 finds

Not a fixed list, because the list is not known until the desktop appears. Rules
that apply to every fix in this phase:

- Every bug gets a **gate**, not just a patch. Eight bugs in this cycle were
  invisible because nothing asserted their absence.
- Every assertion is checked **in the namespace it will be evaluated in**. The
  dangling-symlink check and its immediate correction are the same mistake twice.
- A fix that produces no observable change in a build log is not verified. It is
  hypothesized.

---

## Phase 4 — Missing things, honestly scoped

| Item | Scope | Notes |
|---|---|---|
| Real starter model | **in** | `placeholder.gguf` is not a model; needs a licensed, digest-pinned GGUF and proven inference |
| `hcs-agent-bridge` | **decide** | in the master plan, never built — build it or delete it from the plan |
| `hcs-update`, `hcs-persist`, `hcs-recall` | **in** | currently QML over mock data |
| `hcs-shot`, `hcs-term`, `hcs-notes` | **in** | same |
| sid → snapshot pin | **in** | sid is rolling; today's build is not tomorrow's |
| `sources.lock.yaml` | **in** | placeholder entries must be the real pins |
| SBOM + third-party notices | **in** | regenerate from the image, not by hand |
| Bare-metal GPU acceptance | **physical only** | `vulkaninfo` naming a hardware ICD, `vkcube`, hardware renderer, Wi-Fi associate |

**Phase 4 cannot be honestly completed for the hardware row without the
machine.** It will be recorded as `UNTESTED`, not as passing.

---

## Phase 5 — README

The README is rewritten last, from the gates, not from intent. Rules:

- A capability appears only if a gate verifies it. This is the standard the last
  three releases failed and the reason the current README had to be taken apart.
- `non-free-firmware` named as a redistribution exception, not buried.
- No "universal GPU driver" claim. The honest formulation:

  > Runs on any x86-64 machine. Hardware acceleration on Intel and AMD via Mesa.
  > NVIDIA needs a separately installed driver. No supported GPU still produces a
  > correct desktop via software rendering.

- Known limitations state: no working model, VirtualBox unsupported with the
  measured reason, nothing run on physical hardware.

---

## Phase 6 — Landing on `main`

**`main` currently means "v1.1.0 is the stable release".** Pushing a development
state there does not merge it into the product — it silently redefines what the
branch claims. Two coherent options:

**A. Merge `dev` to `main`, keep v1.1.0 as stable.**
`main` gains v2 work; the README must state plainly that v2.0.0 is *not*
released. Acceptable, and honest if the README is explicit.

**B. Hold `main`.** Land on `dev`, keep PR #5 open until the Phase 2 gate is
GREEN. `main` keeps meaning "the last verified release".

**Recommendation: B, until Phase 2 is green.** The reason is specific rather than
procedural: a `main` that contains a `dev`-grade desktop invites exactly the
mistake this project has made three times — treating merged as finished.

Whichever is chosen, the branch rule is: **a green gate on `dev` is what opens the
PR; merging does not mean released.**

---

## Definition of done

1. `local-fs.target` blocker named and fixed.
2. Phase 1 green, plus a regression test for every bug in this cycle.
3. A screenshot exists in which the shell is drawn, graded GREEN by the host
   checker.
4. All four boot cohorts return their own verdict.
5. A real licensed model produces real output.
6. Every gate green on a tag, with no `continue-on-error` in the release path.
7. README states only what the gates verify.
8. `main` reflects a deliberate decision, not an accidental merge.