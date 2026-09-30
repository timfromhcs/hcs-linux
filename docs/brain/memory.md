# Cognitive Memory Architecture

HCS Memory is an SQLite-backed multi-class cognitive store powering the local Brain.

## Memory Classes

- **Working:** Transient scratchpad for ongoing agent tasks.
- **Session:** Per-conversation context that expires on session close.
- **Episodic:** Append-only narrative of completed system events.
- **Semantic:** General world facts, operating system documentation, and software knowledge.
- **Preference:** User configuration habits and formatting preferences.
- **Skill:** Verified executable workflows and procedures.
- **Outcome:** Verified deterministic task results.
- **Failure:** Explicit negative examples stored for avoidance and DPO pairs.
- **Project:** Workspace-scoped architectural facts.
- **Training:** Validated positive training candidates.

## Hybrid Retrieval

Retrieval combines:
$$\text{Score} = w_{\text{lex}} \cdot S_{\text{lex}} + w_{\text{sem}} \cdot S_{\text{sem}} + w_{\text{rec}} \cdot S_{\text{rec}} + w_{\text{conf}} \cdot S_{\text{conf}}$$
Where $S_{\text{lex}}$ is SQLite FTS5 BM25 matching, and $S_{\text{rec}}$ decays over time to prioritize fresh context.
