# HCS Linux — Threat Model

> **Status:** required by Gate 2. A privacy claim without a threat model is
> marketing, so this file is a release gate: CI fails if it is missing or does
> not cite its reference systems.
>
> **Reference systems:** [Tails](https://tails.net/) for amnesic operation and
> persistent storage, [Linux Mint](https://www.linuxmint.com/) for update risk
> levels, [Omarchy](https://omarchy.org/) for snapshot-based rollback.

## 1. What this system protects, and from whom

HCS Linux is an AI-native, privacy-oriented, CPU-first desktop. It runs a
cognitive model locally, stores a memory index locally, and can optionally route
all traffic through Tor. Those three facts create the threats below.

| # | Adversary | Capability | What we defend |
|---|---|---|---|
| T1 | Network observer | Sees traffic metadata and, without TLS, content | Everything goes through Tor by choice; the kill switch makes non-Tor egress impossible rather than discouraged |
| T2 | Malicious exit relay | Sees and alters unencrypted traffic | Nothing sensitive is sent unencrypted; the kill switch drops non-Tor traffic outright |
| T3 | Post-mortem examiner with the device | Reads disks, RAM, cold-boot | Amnesic mode writes nothing to local storage and poisons freed memory; the persistent volume is LUKS-encrypted |
| T4 | Casual observer at the keyboard | Physical access to a running session | Session lock, no guest account, privileged actions gated behind explicit confirmation |
| T5 | Supply-chain attacker | Modifies a dependency or a model | Every source is hash-pinned; models are hash-verified before `hcs-modeld` loads them; only redistributable-licence models ship in the image |
| T6 | Co-resident process | Reads memory of other users' processes | Non-root execution for agents; the sandbox gate in `hcs-agents` denies capabilities explicitly rather than by default |
| T7 | User error | Deletes data, breaks the system | Snapshots before updates, rollback from the boot menu, recoverable Trash in Files |

## 2. What we explicitly do **not** protect against

Stated plainly, because overclaiming is the failure mode this document exists to
prevent.

* **A compromised kernel or firmware.** Secure boot is not available on every
  machine we target.
* **A hostile user with root.** If someone can read `/etc/shadow` they do not
  need our cryptography.
* **A malicious USB stick that was plugged in before the boot.** Booting from a
  medium you did not verify is a decision, not a bug.
* **Traffic analysis by a global observer.** Tor hides content and origin, not
  the fact that a connection happened.
* **Forensic recovery from flash storage.** See §5.

## 3. Amnesic mode

Adopted from Tails, whose specification is explicit that a live system must be
amnesic by default.

| Guarantee | Mechanism | Where it lives |
|---|---|---|
| No writes to host storage | `nopersistence` on the kernel command line | `hcs-persist`, GRUB entries |
| No swap vector | `noswap`, and the host's swap volume is never used | kernel parameters |
| No cold-boot recovery | `init_on_free=1` (freed-memory poisoning) | kernel parameters |
| No volatile leftovers | memory erasure on shutdown **and** on physical removal of the boot medium | `hcs-persist` |

**Honest limitation.** These are boot-time parameters. Turning amnesic mode on in
a running session records the intent and applies it at the *next* boot; the CLI
says so rather than claiming the current session is already amnesic.

## 4. Persistent storage

Tails' model: persistence is opt-in, per-feature, and encrypted.

* The volume is LUKS, occupying the free space of the boot medium.
* The user unlocks it at boot or starts **without** unlocking. Starting without
  is allowed and is not an error.
* Each feature is `active`, `enabled` or `masked`. A masked feature is hidden
  from the interface and cannot be activated until un-masked.
* Deactivating a feature keeps its data. Only an explicit delete removes it.
* The weakest link is the passphrase, so the vault generates five-to-seven
  random words rather than a hex dump.

## 5. Secure deletion: what we claim and what we do not

**We do not offer secure file deletion.** Tails removed its shredding tools in
6.0 because overwriting is not reliable on SSDs and flash: wear levelling means
the old block may never be written again, and the controller's cache is not under
our control.

What we offer instead, in order of preference:

1. **Do not save the file.** Amnesic mode makes this automatic.
2. **Encrypt the volume.** Data recovery then yields only ciphertext.
3. **Overwrite the whole device** with zeroes.
4. **Destroy the device physically.** For media that will never be reused, this
   is the only guarantee.

Anyone who tells you a `shred` invocation makes an SSD safe is mistaken, and we
would rather say so than ship the illusion.

## 6. Tor kill switch

Fail-closed. When engaged, the nftables rules drop traffic that has not been
transparently proxied, so a misconfigured application is isolated rather than
leaking. The Control Center shows the state and the rule count comes from the
same `TorTransparentProxy` the CLI uses, so the display cannot disagree with the
firewall.

DNS leaks are the specific failure this defends against: the transparent proxy
must intercept DNS itself, or an application can resolve a hostname over a
channel that bypasses Tor.

## 7. Models and supply chain

* Every source dependency is hash-pinned; `scripts/verify_sources.py` fails the
  build otherwise.
* Models are verified by SHA-256 before `hcs-modeld` will load one.
* **Only models whose licence permits redistribution of the weights are baked
  into the ISO.** The rule lives in `scripts/stage_starter_models.py` and is
  evaluated against `config/models/registry.yaml`, so there is one
  implementation rather than a list that can drift.
* Everything else downloads on demand and is hash-verified.

## 8. Privileged actions and agents

* Any action that changes system state is marked `Mutating`; anything touching
  the network, Tor rules, the vault or package management is `Privileged`.
* A privileged action requires explicit human confirmation. The omnibar, the
  CLI and an agent all go through the same gate, so there is no path that skips
  it.
* Agents run non-root with a granted capability list, checked per invocation
  rather than assumed.

## 9. Recall

Windows Recall is copied in three ways and refused in one.

Copied: the timeline, semantic search, and a **sensitive-information filter** —
anything matching credential or payment-card patterns is excluded *before* a
snapshot is written.

Refused: Recall uploads snapshots to a vendor. HCS Recall keeps its index on the
machine, encrypted at rest, requires an explicit opt-in, and **does not exist at
all in an amnesic session**. "Silently records nothing" would be
indistinguishable from "records nothing useful", so it refuses instead.

## 10. Residual risks we accept

* A user who disables the kill switch has no Tor protection. We make that state
  visible at all times rather than hiding it.
* A user who installs a package from outside the pinned sources has accepted the
  risk. The Software Manager shows the source.
* The image itself can be modified by anyone with write access to the USB stick.
  Verification instructions are printed at first boot.
