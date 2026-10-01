# The HCS Brain

The Brain is a local model runtime, a memory index and a set of agents. Nothing
in this chapter requires a network connection.

## The model catalog

| id | Model | Size | RSS | Tier |
|---|---|---|---|---|
| `hcs-controller` | Qwen3-0.6B Q4_K_M | 0.6 B | ~550 MB | resident |
| `hcs-assistant` | Qwen3-1.7B Q4_K_M | 1.7 B | ~1450 MB | on demand |
| `hcs-coder` | Qwen2.5-Coder-1.5B Q4_K_M | 1.5 B | ~1350 MB | on demand |
| `hcs-reasoner` | Qwen3-4B Q4_K_M | 4 B | ~2900 MB | on demand |
| `hcs-embedding` | Qwen3-Embedding-0.6B Q4_K_M | 0.6 B | ~500 MB | indexer |

```bash
hcs model list
hcs model status
hcs model load hcs-assistant
```

## The Single-Heavy-Model rule

Only **one** heavy model may be resident at a time. Loading a second one while a
heavy model is loaded is refused rather than silently swapping, because a swap
under RAM pressure is how a machine starts to thrash.

The GUI reflects this: if the 4 B reasoner is resident, Image Studio shows a
"unload first" dialog instead of failing at render time.

## The RAM budget

| State | Budget |
|---|---|
| System idle | 6144 MB |
| System peak | 8192 MB |
| Each GUI app | 250 MB |

The budget is measured, not estimated. `hcs-monitor` shows the same numbers the
gate asserts, so a violation is visible before it becomes a problem:

```bash
hcs-monitor --json          # the machine-readable source of truth
python scripts/gui_ram_audit.py   # the same check CI runs
```

## Memory

The memory engine is SQLite FTS5 with five classes: working, episodic, semantic,
skill and ledger. Retrieval is hybrid — lexical plus a ranking pass — so it works
with no model resident.

```bash
hcs memory search "tor bridge"
hcs memory stats
```

Memory is what makes the omnabar answer questions about *your* system rather
than only about its files.

## Asking questions

```bash
hcs ask "what is the RAM budget"
hcs ask --selection          # explain whatever is on screen
```

Retrieval is offline and every answer cites the manual section it came from:

```bash
hcs rag query "how does the kill switch work"
```

An out-of-scope question is **refused**, not answered hopefully. A guess about a
security feature is worse than no answer, so the system says it does not know.

## Agents

Agents are scoped by capability, checked per invocation rather than assumed:

| Role | Can |
|---|---|
| `planner` | read files, reason |
| `researcher` | read files, search |
| `coder` | read, write, run tests |
| `verifier` | read, run tests |
| `pentester` | read, run — **and only with human confirmation** |

```bash
hcs agent run "audit the privacy defaults" --role verifier
hcs agent ledger --limit 5
```

Every run is recorded in the task ledger with its outcome and resource usage.

## Control surface

Everything an agent can do, you can do from the CLI, and everything answers
`--json`:

```bash
hcs actions list --json
hcs actions search "snapshot"
hcs settings show
hcs theme list
hcs window mode toggle
hcs desktop new
```

This is deliberate. A desktop whose behaviour can only be reached by clicking is a
desktop an agent cannot help with.
