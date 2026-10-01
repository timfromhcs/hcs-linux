# Tor Anonymity and the Kill Switch

The kill switch is the most important privacy feature in HCS Linux, and it is
**fail-closed**: when armed, traffic that is not transparently proxied is
dropped, not merely flagged.

## What it does

```bash
hcs security tor enable     # arm
hcs security tor disable    # disarm
hcs security tor toggle     # from the keyboard: HCS+Alt+T
```

When armed, the nftables ruleset enforces:

* Tor transparent proxy on ports 9040 (SOCKS) and 9053 (DNS)
* `debian-tor` bypass allowed so the daemon itself can reach the network
* loopback permitted
* IPv6 dropped, because Tor is not running there
* everything else dropped

The Control Center and the taskbar pill both read their state from
`TorTransparentProxy` itself, so the display cannot disagree with the firewall.

## Zero DNS leaks

The specific failure a Tor setup usually has is DNS: an application resolves a
hostname over a channel that bypasses the proxy, and the resolver's query reveals
who is asking for what.

The transparent proxy therefore intercepts DNS itself rather than trusting
applications to use SOCKS. A leak test asserts exactly this and it is part of the
security gate:

```bash
hcs security status
#   Tor Transparent Proxy: Active
#   Rule count: 6
#   DNS leak test: PASS
```

## What the kill switch does not do

* It does not hide traffic analysis. A global observer still sees that a
  connection happened.
* It does not protect against a compromised kernel or firmware.
* It does not protect a machine already compromised before Tor was armed.

`docs/THREAT_MODEL.md` lists what is defended and what is explicitly out of
scope. Overclaiming here is the failure mode the project is trying to remove.

## Amnesic mode

A separate, stronger guarantee: an amnesic live session writes nothing to local
storage at all.

```bash
hcs privacy amnesic on
```

| Guarantee | Mechanism |
|---|---|
| No writes to host disks | `nopersistence` |
| No swap vector | `noswap` |
| No cold-boot recovery | `init_on_free=1` |
| No volatile leftovers | memory erasure on shutdown and on USB removal |

These are boot parameters. The CLI records the intent and tells you it applies at
the next boot, rather than claiming the running session is already amnesic.

## Persistent storage

Persistence is opt-in and encrypted. The volume is LUKS on the boot medium, and
you may start **without** unlocking it — that is allowed, not an error.

```bash
hcs privacy storage
```

Each feature is `active`, `enabled` or `masked`. Deactivating a feature keeps its
data; only an explicit delete removes it.

## Recall

HCS Recall keeps a local, encrypted timeline of what you have seen, in the spirit
of Windows Recall but with three differences:

* **It is local.** Nothing is uploaded. Ever.
* **Sensitive information is filtered before storage.** Passwords, API keys,
  bearer tokens and card numbers are matched and discarded rather than indexed.
* **It does not exist in an amnesic session.** A feature that silently records
  nothing is indistinguishable from one that records nothing useful, so it
  refuses to start.

```bash
hcs settings show | grep recall
hcs recall "that recipe from last week"
```

## Verifying your own posture

```bash
hcs security status
python scripts/run_security_audit.py
```

Both are gates. If either fails, the release does not ship.
