# HCS Linux — Autonomous Build & Continual-Brain Engineering Contract

> **Mission:** Start from an empty Windows folder and autonomously build HCS Linux into a real, installable, bootable, reproducible, local-first AI operating system. The agent must research current upstream sources, fetch them headlessly, build real software, run real tests, render a real ISO, boot/test it in a VM, perform visual QA, stress the code and runtime, audit licenses, generate SBOM/provenance, and only publish a release when every required gate has objective evidence.
>
> **Important:** Never claim “perfect”, “bug-free”, “secure”, “anonymous”, “Tails-equivalent”, or “fully verified” unless the exact claim is demonstrated by recorded tests. A successful build is not proof of correctness. If a required external or hardware test is unavailable, mark the gate `BLOCKED`, document exactly why, and do not manufacture a pass.

---

## 1. Product Definition

HCS Linux is an **AI-native, privacy-oriented, CPU-first Linux distribution**. It should feel like one coherent computer rather than a Linux desktop with a chatbot glued onto it.

Core user experience:

```text
                 HCS LINUX
                     │
          ┌──────────┴──────────┐
          │                     │
      HCS SHELL             HCS BRAIN
          │                     │
   modern Wayland          local AI + memory
   glass / smooth          agents + skills
   dock / launcher         tools + planner
   workspaces              verifier + learner
          │                     │
          └──────────┬──────────┘
                     │
                Linux + Apps
```

Primary interaction modes:

- click
- type
- search
- chat
- voice (optional)
- visual interaction

Developer and security capabilities exist underneath the polished consumer interface.

Developer Mode and Security Lab must be discoverable but must not make the normal desktop look like a terminal-first hacker distribution.

---

# 2. Non-Negotiable Runtime Budget

HCS Linux has a hard engineering target:

```text
IDLE BASELINE TARGET: <= 6 GB RAM
NORMAL/HEAVY PEAK TARGET: <= 8 GB RAM
```

These are **targets that must be measured**, not assumed.

The system must use:

- one generation model at a time for heavy inference
- on-demand model loading
- explicit model unloading
- small context windows by default
- on-demand reranking and embedding
- Prime Agent only when agentic work is active
- no unnecessary resident Python runtimes
- no Electron in the core shell
- minimal indexing services
- zram where appropriate
- bounded services through systemd resource controls

A model being “only 4B parameters” does not prove that the complete process will fit the budget. RAM must include weights, KV cache, runtime overhead, tokenizer, buffers and simultaneous processes.

For every model profile record measured:

```text
weights RAM
KV RAM
runtime RAM
peak RSS
startup RSS
steady-state RSS
load time
unload time
tokens/sec where meaningful
```

---

# 3. Base OS Strategy

Use Debian Stable as the first production base.

As of 2026-09-30, Debian 13.7 (codename `trixie`) is the current Debian 13 point release. Debian states that 13.7 was released on 2026-09-12 and the Debian 13 lifecycle extends through 2030-06-30 under the published support schedule.

Primary build stack:

```text
Debian 13 (trixie)
live-build
systemd
Wayland
niri
Quickshell
Calamares
```

Do NOT mix Kali repositories into Debian.

Do NOT randomly mix Debian, Ubuntu PPAs, AUR packages and third-party repositories.

Prefer:

```text
Debian package
OR
pinned upstream source
OR
HCS-owned .deb package
```

Every external source must be pinned and recorded.

---

# 4. Windows Build Host

The development machine is Windows.

Use:

```text
Windows 11
WSL2
Debian
Git
GitHub CLI
VirtualBox
VS Code or another editor
```

The Windows host is the orchestration/build host.

The Linux build environment lives in the WSL2 ext4 filesystem for performance.

Recommended directories:

```text
Windows:
C:\HCSLinux

WSL:
~/hcs-linux
```

Build source and large intermediate trees should stay inside `~/hcs-linux` rather than `/mnt/c/HCSLinux`.

Copy release artifacts to Windows only after the build has finished.

---

# 5. Empty-Directory Bootstrap

The agent starts in an empty directory.

First action:

```bash
git init -b main
git config core.autocrlf false
```

Create the canonical structure:

```text
hcs-linux/
├── .github/
│   ├── workflows/
│   └── ISSUE_TEMPLATE/
├── auto/
├── config/
│   ├── package-lists/
│   ├── includes.chroot/
│   ├── includes.binary/
│   ├── hooks/live/
│   ├── bootloaders/
│   ├── archives/
│   └── installer/
├── src/
│   ├── hcsd/
│   ├── hcs-shell/
│   ├── hcs-chat/
│   ├── hcs-search/
│   ├── hcs-memory/
│   ├── hcs-modeld/
│   ├── hcs-agents/
│   ├── hcs-security/
│   ├── hcs-settings/
│   └── hcs-installer/
├── config/models/
│   ├── registry.yaml
│   ├── profiles.yaml
│   └── policies.yaml
├── vendor/
│   ├── sources/
│   ├── models/
│   ├── datasets/
│   ├── patches/
│   └── locks/
├── scripts/
├── tests/
│   ├── unit/
│   ├── integration/
│   ├── boot/
│   ├── privacy/
│   ├── ai/
│   ├── agent/
│   ├── stress/
│   └── install/
├── qa/
│   ├── expected/
│   ├── latest/
│   ├── screenshots/
│   ├── logs/
│   └── reports/
├── assets/
│   ├── logo/
│   ├── icons/
│   ├── wallpapers/
│   ├── boot/
│   └── installer/
├── docs/
│   ├── getting-started/
│   ├── ai/
│   ├── brain/
│   ├── privacy/
│   ├── security/
│   ├── developer/
│   ├── models/
│   ├── build/
│   └── troubleshooting/
├── licenses/
│   ├── third-party/
│   ├── models/
│   └── reports/
├── data/
│   ├── golden/
│   ├── eval/
│   ├── experience/
│   ├── preference/
│   └── training-candidates/
├── LICENSE
├── NOTICE
├── TRADEMARKS.md
├── SECURITY.md
├── CONTRIBUTING.md
├── CHANGELOG.md
├── README.md
├── GEMINI.md
├── Makefile
└── .gitignore
```

---

# 6. Research-First Rule

Before implementing a moving part, research the current upstream state.

Primary sources only where possible:

1. official upstream project repository
2. official project documentation
3. Debian package metadata
4. official Hugging Face model card/repository
5. official SPDX information
6. GitHub documentation

For each dependency write a source record containing:

```yaml
name:
source:
upstream_url:
revision:
version:
license:
license_source:
files:
build_method:
patches:
sha256:
notes:
```

Never silently use `main`, `master`, `latest`, an untagged tarball or an unpinned Docker image in a release build.

Development may use fresh sources, but stable releases must pin exact revisions.

---

# 7. Fetch/Build Separation

There are two phases.

## FETCH

Internet is allowed.

Download:

- source repositories
- source archives
- Debian package metadata
- model files
- tokenizers
- model cards
- license files
- patches
- datasets
- required documentation

Generate lockfiles.

## BUILD

The build must consume prepared inputs only.

Do not unexpectedly access the network from inside the core ISO build just because a package or script happened to need something.

For deeper reproducibility, introduce a Debian package cache or pinned snapshot in a later milestone.

---

# 8. Headless Downloads

Use CLI tools and avoid GUI download workflows.

For Hugging Face:

```bash
python3 -m venv .venv
source .venv/bin/activate
python -m pip install --upgrade pip huggingface_hub
```

Then use the `hf` CLI.

Example:

```bash
hf download ggml-org/Qwen3-1.7B-GGUF Qwen3-1.7B-Q4_K_M.gguf \
  --local-dir vendor/models/qwen3-1.7b
```

For every model:

```bash
sha256sum vendor/models/.../*.gguf
```

Record the result in `config/models/registry.yaml` and `vendor/locks/models.lock.yaml`.

Never put model weights into normal Git history.

---

# 9. Model Architecture

HCS Brain is a multi-role system, not a single-model chatbot.

```text
                       HCS BRAIN
                           │
             ┌─────────────┼─────────────┐
             │             │             │
          ROUTER        MEMORY        POLICY
             │             │             │
      ┌──────┼──────┐      │       capabilities
      │      │      │      │
   CHAT    CODE   REASON  RAG
      │      │      │      │
      └──────┼──────┴──────┘
             │
        PRIME AGENT
             │
      subagents / RLM
             │
      tools / execution
             │
          verifier
             │
          outcome
             │
      judge / reward
             │
      lesson candidate
             │
      memory / training
```

---

# 10. Core Model Catalog

The following catalog is the default **candidate pool**. Do not assume every candidate belongs in the ISO. The release profile selects a subset after measuring it.

## 10.1 HCS Controller

Preferred base:

```text
Qwen/Qwen3-0.6B
```

License reported by its current Hugging Face repository: Apache-2.0.

A current abliterated candidate exists:

```text
mlabonne/Qwen3-0.6B-abliterated
```

Its current Hugging Face repository reports Apache-2.0.

GGUF candidate:

```text
mlabonne/Qwen3-0.6B-abliterated-GGUF
```

Use cases:

```text
intent classification
cheap routing
simple tool choice
small command extraction
short answers
state summarization
```

Recommended runtime role:

```text
resident or frequently available
```

---

## 10.2 HCS Assistant

Preferred base:

```text
Qwen/Qwen3-1.7B
```

Current Hugging Face repository reports Apache-2.0.

A current abliterated candidate exists:

```text
mlabonne/Qwen3-1.7B-abliterated
```

and:

```text
huihui-ai/Qwen3-1.7B-abliterated
```

Both are current community abliterated candidates; treat community checkpoints as experimental and benchmark them against the base model.

Use this role for:

```text
normal HCS chat
planning
small tool calls
memory interaction
document summarization
workspace assistance
```

Only one assistant-generation model should be loaded at a time.

---

## 10.3 HCS Reasoner

Preferred base:

```text
Qwen/Qwen3-4B
```

Current Hugging Face repository reports Apache-2.0.

Current abliterated candidates include:

```text
mlabonne/Qwen3-4B-abliterated
mradermacher/Qwen3-4B-abliterated-GGUF
prithivMLmods/Qwen3-4B-abliterated-f32-GGUFs
```

The `mradermacher` GGUF repository reports Apache-2.0. The `prithivMLmods` repository currently publishes multiple GGUF quants including a Q4_K_M around 2.5 GB.

Use for:

```text
hard reasoning
complex planning
multi-step diagnosis
large refactors
agent arbitration
```

This model is **on-demand only** in Edge mode.

---

## 10.4 HCS Coder — Edge

Base:

```text
Qwen/Qwen2.5-Coder-0.5B-Instruct
```

Current official repository reports Apache-2.0.

Abliterated candidate:

```text
huihui-ai/Qwen2.5-Coder-0.5B-Instruct-abliterated
```

Current repository reports Apache-2.0.

GGUF candidates:

```text
bartowski/Qwen2.5-Coder-0.5B-Instruct-abliterated-GGUF
mradermacher/Qwen2.5-Coder-0.5B-Instruct-abliterated-GGUF
```

Use for:

```text
small edits
shell snippets
configuration
simple bug fixes
code completion
```

---

## 10.5 HCS Coder — Standard

Base:

```text
Qwen/Qwen2.5-Coder-1.5B-Instruct
```

Current official repository reports Apache-2.0.

Abliterated candidate:

```text
huihui-ai/Qwen2.5-Coder-1.5B-Instruct-abliterated
```

Its current Hugging Face repository reports Apache-2.0.

This is the primary low-RAM coding agent candidate.

---

## 10.6 HCS Coder — Experimental

Abliterated candidate:

```text
huihui-ai/Qwen2.5-Coder-3B-Instruct-abliterated
```

Current Hugging Face metadata reports `qwen-research`, not Apache-2.0.

Therefore:

```text
NOT default ISO
OPTIONAL / REVIEW REQUIRED
```

License handling must use the actual repository license file, not an inferred license from the base model.

---

## 10.7 SmolLM3

Base:

```text
HuggingFaceTB/SmolLM3-3B
```

An abliterated candidate exists:

```text
richardyoung/SmolLM3-3B-abliterated-obliteratus
```

Current repository reports Apache-2.0.

Use as an alternate reasoning/assistant candidate, never automatically replace the production model without benchmark evidence.

---

## 10.8 Phi-4-mini

Base:

```text
microsoft/Phi-4-mini-instruct
```

Abliterated candidates include:

```text
huihui-ai/Phi-4-mini-instruct-abliterated
lunahr/Phi-4-mini-instruct-abliterated
tensorblock/Phi-4-mini-instruct-abliterated-GGUF
```

Current community repositories report MIT for the abliterated checkpoints. The GGUF repository provides Q4_K_M around 2.49 GB.

Treat `trust_remote_code`-dependent repositories as higher supply-chain risk and prefer a verified GGUF conversion when compatible.

---

## 10.9 Llama 3.2

Base lightweight models:

```text
meta-llama/Llama-3.2-1B-Instruct
meta-llama/Llama-3.2-3B-Instruct
```

Meta documents the 1B/3B lightweight models and the quantized versions. The Llama 3.2 license is a custom Llama 3.2 Community License, not Apache-2.0.

A current abliterated candidate exists for 3B:

```text
huihui-ai/Llama-3.2-3B-Instruct-abliterated
```

Use only if the exact Meta licensing conditions are acceptable for the intended HCS distribution.

Do not copy Meta weights into the public repository merely because the code can fetch them.

---

## 10.10 Gemma 3

Abliterated candidates include:

```text
lunahr/gemma-3-1b-it-abliterated
mlabonne/gemma-3-4b-it-abliterated-GGUF
mradermacher/gemma-3-4b-it-abliterated-GGUF
```

These repositories report the Gemma license. This is a Google-specific license/terms regime and is **not** Apache-2.0.

Gemma 3 is attractive as a multimodal candidate, but it should not enter the default HCS core without an explicit license review and memory benchmark.

---

## 10.11 Qwen3 Larger Abliterated Family

Current `mlabonne` repositories exist for:

```text
Qwen3-0.6B-abliterated
Qwen3-1.7B-abliterated
Qwen3-4B-abliterated
Qwen3-8B-abliterated
Qwen3-14B-abliterated
Qwen3-30B-A3B-abliterated
```

The current repositories report Apache-2.0.

However:

```text
8B  = optional high-RAM pack
14B = high-RAM pack
30B-A3B = never default Edge runtime
```

The `30B-A3B` community model card currently describes the checkpoint as work-in-progress; do not treat it as a production Edge model.

---

# 11. Do Not Abliterate the Evaluator by Default

The HCS learning loop needs a stable evaluator.

Abliterated generation models are useful as candidate generators, but evaluator drift is dangerous.

Default evaluator stack:

```text
Qwen3-Reranker-0.6B
+
Qwen3-0.6B judge/analyzer
+
objective execution evidence
+
optional reward model
```

Do not let the same model generate and uncritically grade its own output.

At minimum, the evaluation path should have a different prompt, different model role, or preferably a different model checkpoint.

---

# 12. Retrieval Rankers

Primary:

```text
Qwen/Qwen3-Reranker-0.6B
```

Current repository reports Apache-2.0.

Use for:

```text
memory ranking
knowledge retrieval
candidate lesson ranking
training example ranking
search reranking
```

Alternative:

```text
BAAI/bge-reranker-v2-m3
```

Current repository reports Apache-2.0.

Jina:

```text
jinaai/jina-reranker-v2-base-multilingual
```

Current repository reports CC-BY-NC-4.0, so do not use it in a general commercial HCS core without a separate license decision.

---

# 13. Reward / Preference Models

A compact reward candidate exists:

```text
puwaer/Safe-Reward-Qwen3-0.6B
```

A separate current candidate exists:

```text
puwaer/Unsafe-Reward-Qwen3-0.6B
```

There are also community Qwen2.5 reward models around 0.5B/1.5B. Example:

```text
ajeet9843/qwen2.5-0.5b-hh-rlhf-rm
```

The current community repository reports MIT and documents a Bradley–Terry preference reward setup.

Another research candidate:

```text
internlm/internlm2-1_8b-reward
```

Its current repository reports a nonstandard/other license and is therefore not a default HCS dependency.

Rule:

```text
reward model != absolute truth
```

Reward signals must be combined with deterministic evidence and holdout tests.

---

# 14. Embeddings

Primary:

```text
Qwen/Qwen3-Embedding-0.6B
```

Current repository reports Apache-2.0.

GGUF variant:

```text
Qwen/Qwen3-Embedding-0.6B-GGUF
```

Current repository reports Apache-2.0 and is usable through llama.cpp-compatible tooling.

The embedding model is loaded on demand and unloaded when indexing/retrieval completes if necessary for the RAM target.

---

# 15. Vision

Optional low-RAM model:

```text
HuggingFaceTB/SmolVLM-500M-Instruct
```

Current repository reports Apache-2.0.

Optional OCR/vision model:

```text
microsoft/Florence-2-base
```

Current repository reports MIT.

Vision models should be optional packs in the first release if they materially threaten the RAM budget.

A vision pack can provide:

```text
screenshot understanding
OCR assistance
document image parsing
visual QA explanation
image-to-text
```

---

# 16. Speech

Optional:

```text
openai/whisper-small
```

Current repository reports Apache-2.0.

Use it only on demand.

Never keep a speech model resident unless voice input is actively enabled.

---

# 17. Recommended HCS Edge Model Set

Default Edge target:

```text
HCS Controller:
Qwen3-0.6B (base or measured abliterated candidate)

HCS Assistant:
Qwen3-1.7B (base or measured abliterated candidate)

HCS Coder:
Qwen2.5-Coder-1.5B abliterated candidate

HCS Reasoner:
Qwen3-4B Q4_K_M, on demand

Memory:
Qwen3-Embedding-0.6B, on demand

Memory Ranker:
Qwen3-Reranker-0.6B, on demand

Judge/Analyzer:
Qwen3-0.6B role-tuned/evaluated

Optional Reward:
compact 0.5B/0.6B candidate after benchmark/license review

Vision:
SmolVLM-500M optional

Speech:
Whisper Small optional
```

The exact production checkpoint is chosen by measured quality-per-RAM-per-latency, not by model hype or download counts.

---

# 18. Model Routing

The router should choose the cheapest model that can plausibly solve the task.

Example:

```text
simple UI command
    -> controller

normal chat
    -> assistant

code edit
    -> coder

retrieval
    -> embedding + reranker

complex planning
    -> reasoner

long-running software task
    -> Prime Agent + reasoner/coder

image/screenshot
    -> vision pack

voice
    -> Whisper
```

Model switching policy:

```text
load model
run task
record resource use
release model
```

Never load every model simultaneously merely because they are installed.

---

# 19. HCS Brain State Model

The brain has multiple memory classes.

```text
working
session
episodic
semantic
preference
skill
outcome
failure
project
training
```

Each record should carry:

```text
id
created_at
updated_at
source
content
confidence
verification_state
last_verified
project
privacy_class
hash
```

No raw model self-report should be considered ground truth without evidence.

---

# 20. Experience/Event Ledger

Every task can generate an append-only event record.

Example schema:

```json
{
  "task_id": "uuid",
  "timestamp": "ISO-8601",
  "user_intent": "...",
  "context_hash": "...",
  "model": "...",
  "model_revision": "...",
  "generation_config": {
    "temperature": 0.0,
    "top_p": 1.0,
    "seed": 42
  },
  "tool_calls": [],
  "output": "...",
  "execution": {
    "exit_code": 0,
    "tests_passed": 12,
    "tests_failed": 0
  },
  "visual": {
    "regression": 0.001
  },
  "resource": {
    "peak_rss_mb": 0
  },
  "user_feedback": null
}
```

Do not store secrets, tokens or credentials in experience logs.

Provide configurable redaction for personal/private content.

---

# 21. Deterministic Outcome Capture

For an OS action, objective evidence is higher-value than a language model saying “success”.

Example:

```text
User:
"Open Firefox and navigate to example.com"

Evidence:
process launched
window exists
window title observed
URL observed
exit state
```

For coding:

```text
build exit code
unit tests
integration tests
lint
static analysis
changed files
runtime test
```

For UI:

```text
screenshot
pixel/structural comparison
window presence
expected widgets
```

For ISO:

```text
checksum
ISO structure
boot
login
services
AI inference
installer
shutdown/reboot
```

---

# 22. Self-Scoring Pipeline

The HCS Brain should not score itself with one unstructured prompt.

Use a layered evaluator:

```text
                 OUTPUT
                    │
          ┌─────────┴─────────┐
          │                   │
      DETERMINISTIC        SEMANTIC
        EVIDENCE            REVIEW
          │                   │
          │             ┌─────┴─────┐
          │             │           │
       tests          judge       ranker
       commands       model       model
       screenshots       │           │
       state             └─────┬─────┘
                                │
                             ANALYZER
                                │
                           final record
```

---

# 23. Score Dimensions

Use a structured rubric.

```text
correctness
instruction_following
relevance
completeness
clarity
tool_use_quality
verification_quality
efficiency
robustness
regression_risk
user_satisfaction
```

Score on a fixed numeric scale, for example 0–100, but preserve the individual dimensions.

Never reduce all evaluation to one opaque scalar in the stored data.

Example:

```json
{
  "correctness": 94,
  "instruction_following": 98,
  "relevance": 91,
  "completeness": 89,
  "tool_use_quality": 96,
  "verification_quality": 100,
  "efficiency": 82,
  "regression_risk": 3,
  "overall": 93
}
```

---

# 24. Judge Model

The judge receives:

```text
user task
relevant context
candidate response
objective evidence
errors
final state
```

The judge MUST return machine-valid structured JSON.

It must explicitly distinguish:

```text
observed
inferred
unknown
```

The judge must never award a high score for a task that lacks evidence merely because the language sounds convincing.

---

# 25. Ranker Model

The reranker is not the final judge.

Use Qwen3-Reranker-0.6B for:

```text
retrieve related memories
rank candidate lessons
rank candidate training examples
rank similar successful tasks
rank similar failures
```

This keeps retrieval and quality evaluation conceptually separate.

---

# 26. Analyzer Model

The analyzer reads:

```text
conversation
planning trace
tool trace
output
objective result
judge scores
ranker results
user feedback
```

It should generate a structured lesson:

```json
{
  "lesson_type": "workflow|failure|preference|fact|skill",
  "summary": "...",
  "trigger": "...",
  "correct_behavior": "...",
  "bad_behavior": "...",
  "evidence": ["task_id"],
  "confidence": 0.0,
  "reusable": true,
  "requires_human_confirmation": false
}
```

The analyzer does not directly modify production prompts or executable skills.

It writes a candidate.

A separate promotion step evaluates it.

---

# 27. The Continual Improvement Loop

This is the central HCS “learning brain”.

```text
TASK
 ↓
EXECUTE
 ↓
OBSERVE
 ↓
VERIFY
 ↓
SCORE
 ↓
COMPARE
 ↓
ANALYZE
 ↓
LESSON CANDIDATE
 ↓
RANK
 ↓
FILTER
 ↓
PROMOTE TO MEMORY / SKILL
 ↓
OPTIONAL TRAINING CANDIDATE
 ↓
TRAIN ADAPTER
 ↓
EVALUATE AGAINST HOLDOUT
 ↓
CHAMPION vs CHALLENGER
 ↓
PROMOTE ONLY IF BETTER
```

---

# 28. Critical Anti-Self-Training Rule

Never do:

```text
model output
 -> training dataset
 -> train model
 -> use new output
 -> train again
 -> infinite self-reinforcing drift
```

Instead use:

```text
experience
 + objective evidence
 + user feedback
 + independent judge
 + ranking
 + fixed validation set
 + data provenance
```

The model must not be allowed to manufacture its own truth merely by repeating it.

---

# 29. Positive Example Filter

A training candidate may enter `data/training-candidates/` only if:

```text
task completed
AND
objective evidence exists
AND
no critical tool error
AND
no unresolved contradiction
AND
judge passes minimum correctness threshold
AND
ranker confidence is sufficient
AND
example is not near-duplicate of existing training data
```

For subjective tasks, add explicit user feedback or independent review.

For coding tasks, require executable tests whenever possible.

---

# 30. Negative Example Filter

Keep failures too.

Store them separately:

```text
data/experience/failures/
```

Each failure should have:

```text
what happened
why it happened
corrected version
reproduction steps
whether the failure was environmental
whether the model or tool caused it
```

Negative examples are useful for:

```text
DPO pairs
critic training
failure classification
routing improvements
```

---

# 31. Corrections After the Fact

The system should support a correction lifecycle.

```text
original output
 ↓
error detected
 ↓
corrected output
 ↓
objective verification
 ↓
pair created
```

For example:

```text
chosen = corrected verified answer
rejected = original incorrect answer
```

This creates a high-value preference pair.

Never train on an unverified “better-looking” rewrite.

---

# 32. Human Feedback

Support:

```text
👍 accepted
👎 incorrect
⭐ excellent
✏ correction
```

A correction should be stored as a separate event referencing the original task.

User feedback must never silently mutate historical records.

---

# 33. Daily HCS Brain Report

Create a systemd user timer.

Suggested behavior:

```text
once per day
```

It summarizes:

```text
successful tasks
failures
new lessons
repeated failures
memory contradictions
new training candidates
model performance
RAM regressions
recommended updates
```

Example notification:

```text
HCS Brain — Daily Review

23 tasks completed
19 verified successful
3 corrected
1 unresolved

5 new lessons
8 training candidates
0 regressions

HCS recommends reviewing 2 recurring failures.
```

The notification itself must remain lightweight and local.

No cloud telemetry is required.

---

# 34. Training Schedule

Do not train every day by default.

Preferred cadence:

```text
daily:
curate + evaluate + report

weekly or threshold-triggered:
adapter training

release cycle:
full benchmark + champion promotion
```

Training can also trigger when:

```text
>= N new high-quality examples
OR
repeated failure pattern reaches threshold
OR
verified correction class reaches threshold
```

---

# 35. Training Strategy

Start with **LoRA/PEFT adapters**, not full-weight retraining.

Use Hugging Face PEFT and TRL where appropriate.

PEFT is specifically designed to fine-tune a small number of additional parameters instead of all model parameters.

TRL currently provides SFT, DPO, KTO, reward modeling and multiple other post-training methods.

Use:

```text
SFT
```

for verified demonstrations.

Use:

```text
DPO / preference method
```

when trustworthy chosen/rejected pairs exist.

Use reward-model training only when the data and label quality justify it.

Use knowledge distillation where a stronger local model creates high-quality, independently filtered targets.

---

# 36. Training Dataset Schema

Example JSONL:

```json
{"task_id":"...","prompt":"...","chosen":"...","rejected":"...","evidence":{"tests":true,"exit_code":0},"scores":{"correctness":97,"verification":100},"source":"verified_task","data_revision":"..."}
```

Every record needs a provenance hash.

Use a dataset manifest:

```yaml
revision:
created_at:
source_tasks:
count:
filters:
min_score:
holdout_exclusion:
sha256:
```

---

# 37. Never Train on the Holdout

Maintain:

```text
data/golden/
data/eval/
data/training-candidates/
```

The golden/eval set must be immutable for a release line.

The training pipeline must refuse to run if a training record shares the same task ID or content hash with a protected holdout record.

---

# 38. Champion / Challenger Model Lifecycle

Never replace the current model automatically.

Use:

```text
champion
challenger
```

Pipeline:

```text
train challenger
 ↓
run fixed eval
 ↓
run regression suite
 ↓
run tool-use suite
 ↓
run memory suite
 ↓
run RAM test
 ↓
compare against champion
 ↓
promotion decision
```

Promotion requires explicit deterministic gates.

If a challenger improves one metric while regressing an essential metric, reject it.

---

# 39. Example Model Promotion Gates

The exact thresholds should be configured by task class rather than invented at runtime.

Suggested hard gates:

```text
critical task regression = 0

fatal tool-use regression = 0

crash/OOM = 0

schema-invalid judge output = 0 in required test set

minimum correctness = configured per suite

minimum verification compliance = configured per suite
```

Use score deltas, not just one overall score.

---

# 40. Prime Agent Integration

Prime Agent is the long-running agentic layer.

Current upstream architecture includes:

```text
RLM
persistent Python environment
subagents
skills
continual harness
memory
scheduled/heartbeat behavior
```

Integrate it below HCS Brain rather than exposing its TUI as the primary user interface.

HCS should present:

```text
HCS is working…

Plan
✓ inspect
✓ retrieve memory
● execute
○ verify
```

while the backend may use Prime Agent/RLM.

Prime Agent must remain a pinned upstream dependency and be benchmarked against the HCS tool interface.

---

# 41. HCS Agent Roles

Create explicit subagent specifications.

Recommended roles:

```text
planner
researcher
coder
debugger
browser
filesystem
system
security-lab
privacy
UI
visual-QA
QA
license-auditor
release-manager
memory-curator
training-curator
judge
critic
verifier
```

Do not let every subagent have every tool.

Use least privilege.

---

# 42. Agent Permission Model

Capabilities:

```text
filesystem.read
filesystem.write
filesystem.delete
app.open
window.control
browser.open
browser.read
network.fetch
git.read
git.write
process.read
process.spawn
system.settings
package.install
root.execute
partition.modify
```

Permission classes:

```text
READ
WRITE
REVERSIBLE
PRIVILEGED
IRREVERSIBLE
NETWORK
```

Defaults:

```text
read = allowed
reversible = allowed where scoped
write = workspace-scoped
privileged = confirmation
irreversible = explicit confirmation + warning
root = never implicit
```

---

# 43. Sandboxing

Use Linux mechanisms such as:

```text
systemd user services
bubblewrap
AppArmor
namespaces
seccomp where appropriate
polkit
```

Optional developer/security isolation:

```text
Podman
Firecracker
```

Do not install or run all isolation infrastructure permanently when it threatens the RAM budget.

---

# 44. Security Lab

HCS may include a dedicated authorized security research environment with tooling classes such as:

```text
network analysis
web security testing
wireless analysis
forensics
reverse engineering
CTF/lab tools
OSINT utilities
```

The security agent must assume authorized use only and should warn when a target scope has not been established.

Do not make the Security Lab part of the default always-running resource profile.

---

# 45. Privacy Stack

HCS should provide:

```text
Tor service
Tor Browser
nftables
DNS controls
LUKS2
encrypted HCS Vault
AppArmor
sandboxing
Live/Amnesic mode
optional encrypted persistence
```

Do not claim application-wide anonymity merely because Tor is installed.

Document exactly which traffic is covered by each mode.

---

# 46. Live USB

Use Debian Live tooling.

Expected design:

```text
HCS ISO
 ↓
USB
 ↓
UEFI/BIOS
 ↓
SquashFS root
 ↓
RAM overlay
 ↓
optional persistence
```

Provide:

```text
HCS Live
HCS Private Live
HCS Install
HCS Recovery
```

Persistent storage must be explicitly configured and preferably encrypted.

---

# 47. Installable System

Use a branded Calamares setup.

Installer options should include:

```text
Language
Keyboard
Disk
Encryption
User
Privacy profile
AI profile
Developer profile
Security profile
```

The installer must produce an actually bootable installed system, not merely an ISO that launches a live desktop.

---

# 48. Desktop Stack

Recommended:

```text
Wayland
niri
Quickshell
QtQuick
```

niri handles compositor/window behavior.

Quickshell handles HCS Shell.

HCS owns the UI identity.

Do not copy another distro's logo, branding, proprietary artwork or UI assets.

---

# 49. HCS Programs

Build first-party programs:

```text
HCS Chat
HCS Search
HCS Files
HCS Memory
HCS Agents
HCS Models
HCS Security
HCS Settings
HCS Control
HCS Vault
HCS Help
HCS Terminal
HCS Update
```

Every app should share one design system.

---

# 50. Design System

Create:

```text
assets/design/hcs-design-tokens.json
```

Tokens include:

```text
color
surface
blur
radius
spacing
typography
shadow
opacity
animation
iconography
```

Target aesthetic:

```text
modern
smooth
minimal
glass-like
soft shadows
large radii
subtle glow
high-quality typography
```

The UI must remain usable without blur or animation on lower-power hardware.

---

# 51. HCS Control Center

Expose:

```text
AI
Network
Tor
Firewall
Privacy
Vault
Agents
Memory
System
Storage
Models
```

Never hide critical privacy state behind ambiguous icons only.

---

# 52. Model Center

For each installed or available model show:

```text
name
role
parameters
quantization
RAM measurement
license
source
revision
SHA256
context
capabilities
```

The UI must indicate when a checkpoint is:

```text
stable
experimental
community
license-review
high-memory
```

---

# 53. HCS Search

Search order:

```text
exact filename / metadata
FTS5
filesystem metadata
embedding retrieval
reranker
AI interpretation
```

Do not invoke a language model for simple exact matches.

---

# 54. HCS Memory Retrieval

Use:

```text
SQLite
FTS5
Qwen3 Embedding 0.6B
Qwen3 Reranker 0.6B
```

Memory retrieval should be hybrid:

```text
lexical score
+
semantic score
+
recency
+
verification confidence
+
project relevance
```

Store the component scores for debugging.

---

# 55. Memory Consolidation

Daily:

```text
find duplicates
find contradictions
merge compatible facts
flag conflicts
expire stale low-confidence records
rank useful skills
```

Never silently delete high-value historical evidence.

Prefer:

```text
superseded_by
```

relationships over destructive updates.

---

# 56. Daily Review Agent

The daily review agent should inspect:

```text
tasks
failures
memory
lessons
training candidates
model performance
resource regressions
system events
```

Then produce:

```text
Daily Brain Report
```

and optionally:

```text
recommended model update
recommended skill update
recommended user review
```

Recommendations do not automatically change the production system.

---

# 57. Update Discovery

HCS may check:

```text
Debian security updates
HCS updates
model registry updates
security advisories
source revisions
```

But release candidates must be pinned.

When an upstream update is found:

```text
fetch
verify
build
run regression
compare
```

Never auto-promote a new upstream model because its repository has a newer timestamp.

---

# 58. Headless Model Discovery

The agent should search Hugging Face programmatically for role candidates.

For each candidate collect:

```text
repo id
base model
parameter count
license
downloads
update timestamp
quantizations
GGUF availability
chat template
context
```

Do not equate downloads with quality.

Search for abliterated and uncensored candidates, but benchmark them as separate checkpoints and verify their licenses independently.

Abliteration can change model behavior beyond simply removing refusals. A 2026 paper reports measurable shifts in decision disposition and confidence-like language after refusal-direction removal. Therefore abliterated variants must be treated as distinct models, not assumed equivalent to the base model.

---

# 59. Model Evaluation Matrix

Every candidate must be evaluated for:

```text
instruction following
reasoning
coding
tool use
structured JSON
German
English
multilingual behavior
hallucination tendency
verbosity control
latency
RAM
context stability
```

Use a fixed reproducible suite.

Generation configuration must be pinned.

For deterministic tests:

```text
temperature=0
fixed seed where supported
fixed context
fixed model revision
fixed prompt
fixed tokenizer
```

Determinism is not guaranteed merely by setting temperature to zero; record actual runtime and sampling settings.

---

# 60. Agent Evaluation

Test real tool use.

Examples:

```text
open application
find file
create file
modify file
run test
read log
summarize result
remember lesson
retrieve lesson
```

The agent must execute real operations in a controlled VM/workspace.

No mocked tool results.

---

# 61. Coding Evaluation

Each coding agent must pass real repositories/tasks.

Pipeline:

```text
inspect
plan
edit
format
lint
build
test
review
re-run
```

The agent must not mark the task successful because the generated code “looks correct”.

---

# 62. Visual QA

Real ISO in a real VM.

Capture:

```text
boot
login
desktop
launcher
chat
memory
files
settings
control center
model center
security
private mode
installer
shutdown
```

Use screenshot regression.

Store:

```text
qa/expected
qa/latest
qa/reports
```

Visual QA must include:

```text
1080p
1440p
4K
100%
125%
150%
1 display
2 displays where available
```

Do not fake screenshots.

---

# 63. Visual Regression Rules

Compare:

```text
layout
missing widgets
text overlap
clipping
unexpected scrollbars
bad scaling
broken transparency
animation artifacts where capturable
incorrect icons
incorrect branding
```

Pixel-difference alone is insufficient.

Use both structural assertions and visual comparison.

---

# 64. VM Matrix

At minimum:

```text
4 GB RAM / 2 vCPU
8 GB RAM / 4 vCPU
```

Where possible also test:

```text
UEFI
legacy/CSM where relevant
single disk
multiple disk
fresh install
upgrade install
```

VirtualBox is the Windows-host default VM.

QEMU may be used in CI where available.

---

# 65. RAM Test Procedure

Measure:

```text
fresh boot idle
launcher
browser
files
HCS Chat
1.7B inference
agent task
4B inference
return to idle
```

Record maximum resident set size.

Example tools:

```bash
free -h
ps -eo pid,rss,cmd --sort=-rss
/usr/bin/time -v ...
```

The build is not release-ready if the RAM target is silently exceeded.

---

# 66. Stress Testing

Run repeated cycles:

```text
start/stop hcsd x 100
load/unload model x 50
open/close chat x 100
memory write/read x 1000
agent tool call x 100
browser open/close
workspace switching
sleep/wake where supported
reboot loop
```

Look for:

```text
memory leaks
file descriptor leaks
zombie processes
service crashes
corrupt SQLite state
stale locks
model unload failures
UI freezes
```

---

# 67. Fuzz / Property Testing

Where practical add:

```text
JSON parser fuzzing
IPC message fuzzing
model-router input fuzzing
memory schema fuzzing
permission-policy fuzzing
installer config validation fuzzing
```

Security-critical parsers need malformed-input tests.

---

# 68. External Testing

“External test” means an independent test environment or evaluator, not the same process claiming it passed.

Possible layers:

```text
GitHub Actions
independent VM job
separate runner
separate verifier process
second model evaluator
```

Never claim an external test passed when only local self-tests were run.

---

# 69. Security Tests

Verify:

```text
agent privilege boundaries
sandbox escapes attempted in a lab
filesystem scope
network policy
Tor state
DNS behavior
secret redaction
credential storage
model file verification
package provenance
installer behavior
```

The agent should create adversarial tests for its own permission boundaries.

---

# 70. License Gate

The release must have a generated software and model inventory.

Use:

```text
Debian copyright metadata
SPDX identifiers
ScanCode
ORT where useful
```

Record:

```text
package/model
version/revision
license
source
copyright
modifications
redistribution notes
```

Rules:

```text
unknown license = BLOCK
unreviewed custom license = BLOCK
non-commercial dependency in default core = BLOCK unless intended and documented
missing notice = BLOCK
```

---

# 71. Third-Party Notices

Generate:

```text
THIRD-PARTY-NOTICES.txt
```

and store machine-readable inventory too.

Never claim all components are Apache-2.0 merely because HCS-owned code is Apache-2.0.

Each upstream license remains authoritative.

---

# 72. HCS License Policy

Recommended default for HCS-owned code:

```text
Apache-2.0
```

But this must not override third-party licensing.

Maintain:

```text
LICENSE
NOTICE
TRADEMARKS.md
licenses/
```

---

# 73. Source Locking

Create:

```text
vendor/locks/sources.lock.yaml
vendor/locks/models.lock.yaml
vendor/locks/packages.lock.yaml
```

No release build should have:

```text
latest
main
master
unversioned URL
```

unless that value is explicitly represented as a development-only input and the release gate blocks it.

---

# 74. Reproducible Source Fetch

Preferred:

```bash
gh repo clone owner/repository path
cd path
git fetch --tags
git checkout <exact commit>
```

For archives:

```text
URL
SHA256
```

For models:

```text
Hugging Face repo
revision
filename
SHA256
```

---

# 75. Native Build Components

The initial native source set is:

```text
llama.cpp
niri
Quickshell
Prime Agent
HCS source
```

Build each from a pinned revision.

Do not use development-only test binaries as runtime dependencies.

---

# 76. llama.cpp

Use CPU-first build.

The upstream build should be pinned.

Typical CPU build shape:

```bash
cmake -S . -B build \
  -DCMAKE_BUILD_TYPE=Release \
  -DGGML_NATIVE=OFF

cmake --build build --config Release \
  --target llama-cli llama-server \
  -j"$(nproc)"
```

Use a second optional native-performance build for local hardware benchmarks if required.

Release packages should favor portability unless a hardware-specific image is intentionally produced.

---

# 77. Qwen3 Generation Configuration

Qwen3 supports thinking and non-thinking behavior.

HCS should expose this through routing rather than forcing all tasks into expensive reasoning.

Example:

```text
normal chat -> non-thinking / short reasoning
complex task -> thinking/reasoning
```

Generation parameters must be represented in task records so experiments are reproducible.

---

# 78. HCS Brain API

Use a local Unix socket as the default internal transport.

Example:

```text
/run/user/<uid>/hcsd.sock
```

Optional localhost HTTP API:

```text
/v1/chat/completions
/v1/models
/v1/memory
/v1/tasks
/v1/agents
/v1/system
/v1/events
```

No network listener is required for the core desktop unless explicitly configured.

---

# 79. HCS Model Daemon

`hcs-modeld` should:

```text
load
unload
serve
inspect
benchmark
verify hash
track RSS
switch model
```

It must expose model state:

```text
loaded
loading
unloading
ready
error
```

Never report “ready” before an actual inference smoke test has succeeded.

---

# 80. HCS Judge Protocol

Every judge request should return something like:

```json
{
  "schema_version": 1,
  "task_id": "...",
  "decision": "pass|fail|review",
  "scores": {},
  "evidence": [],
  "uncertainties": [],
  "regressions": [],
  "lesson_candidate": null
}
```

The parser must reject malformed JSON.

Use schema validation.

---

# 81. Deterministic Judge Mode

For reproducible evaluation:

```text
temperature = 0
fixed model revision
fixed prompt revision
fixed schema
fixed input
```

If the backend has nondeterministic kernels or sampling behavior, record that limitation explicitly.

---

# 82. Learning Safety Against Reward Hacking

Reward functions are gameable.

Never let the model optimize directly against a reward that it can manipulate by:

```text
verbosity
fake citations
fake test logs
self-reported success
repeating evaluator keywords
```

Use objective checks wherever possible.

Examples:

```text
code -> test result
UI -> screenshot assertions
system task -> actual state
file task -> filesystem hash
network task -> observable connection state
```

---

# 83. Training Data Quality Gates

Candidate data must pass:

```text
schema valid
provenance complete
objective result complete
no secret leakage
no duplicate
no unresolved contradiction
judge above threshold
ranker confidence above threshold
holdout exclusion
```

For high-value examples require repeated success or explicit user confirmation.

---

# 84. Training Modes

Support three modes.

## OFF

No training. Brain still learns memory/skills.

## CURATE

Collect and evaluate training candidates, but do not train.

## AUTO-ADAPT

Train LoRA candidates when thresholds are reached, but never promote automatically without passing the full champion/challenger suite.

Default install mode should be `CURATE`.

---

# 85. User-Controlled Training

HCS UI should expose:

```text
Training

○ Off
● Curate only
○ Auto-adapt
```

Show:

```text
examples
estimated training time
adapter size
last run
current champion
candidate quality
```

---

# 86. Personal Data Protection

Training data may contain:

```text
names
paths
emails
private documents
credentials
browser data
project secrets
```

Implement redaction and privacy classes.

Never train on secrets.

Never upload user data to a cloud service as part of local training.

---

# 87. Adapter Storage

Store outside Git:

```text
~/.local/share/hcs/adapters/
```

or encrypted Vault storage.

Maintain:

```text
adapter id
base model revision
training dataset revision
method
hyperparameters
metrics
parent adapter
creation timestamp
hash
```

---

# 88. Adapter Rollback

Every promoted adapter must be reversible.

Maintain:

```text
base
adapter-001
adapter-002
adapter-003
```

Never destroy the prior champion when a new adapter is promoted.

---

# 89. Skill Learning

A repeated successful workflow can become a skill.

Promotion criteria:

```text
observed multiple times
objective success
clear trigger
bounded scope
safe permissions
no contradictory examples
```

A skill should be executable, reviewable and versioned.

Prime Agent skills must not be modified solely from one failed or ambiguous task.

---

# 90. Brain Self-Reflection

The brain should periodically ask:

```text
What tasks failed repeatedly?
Which tools cause the failures?
Which memories conflict?
Which model performs best by task class?
Which prompts are causing regressions?
Which skills are stale?
```

The answer should produce evidence-linked improvement proposals.

---

# 91. Model Routing Telemetry

Local-only metrics:

```text
task class
selected model
latency
RSS
success rate
correction rate
user acceptance
verification rate
```

Use aggregate statistics for routing improvements.

Do not silently export telemetry.

---

# 92. Dynamic Router Learning

The router can learn:

```text
task -> model success rate
```

but must keep a minimum exploration rate only in experimental mode.

Stable mode uses a pinned routing policy.

Never let one weird task permanently reroute the system.

---

# 93. Browser / External Research Layer

HCS may research current information when the user asks.

For autonomous development:

```text
search
open official page
extract facts
record source
record date
```

Research results used to make source changes should be archived in a build report.

---

# 94. Documentation as Product

Everything important must have an offline local help page.

Minimum docs:

```text
What is HCS?
Install
Live USB
Private Mode
Tor
Vault
AI models
Memory
Agents
Prime Agent
Training
Developer Mode
Security Lab
Troubleshooting
```

The HCS Help application must work without an Internet connection.

---

# 95. First-Run Experience

First boot should explain:

```text
Welcome to HCS.

Your local Brain is ready.

Choose:

Personal
Private
Developer
Security
```

Then detect hardware and choose the appropriate model profile.

---

# 96. Hardware Detection

Detect:

```text
CPU model
architecture
RAM
GPU
AVX/AVX2/AVX512 where available
storage
screens
```

Generate:

```text
hcs-hardware-profile.json
```

Use it to choose model profile.

---

# 97. Edge Profiles

At minimum:

```text
EDGE-8GB
STANDARD-16GB
LARGE-32GB
GPU-ACCELERATED
```

The Edge profile must not accidentally install large model packs into the runtime.

---

# 98. Build Profiles

ISO profiles:

```text
hcs-core
hcs-private
hcs-developer
hcs-security
hcs-full
```

Prefer one base ISO with optional downloadable packs later if size becomes excessive.

---

# 99. Core ISO vs Model Packs

Core ISO may contain:

```text
controller
assistant
small embedding/ranker assets where licensing permits
```

Optional downloads:

```text
reasoner
coder
vision
speech
security pack
large-model pack
```

The core ISO should stay practical to download and maintain.

---

# 100. Package Lists

Keep package lists modular:

```text
hcs-core.list.chroot
hcs-private.list.chroot
hcs-developer.list.chroot
hcs-security.list.chroot
hcs-media.list.chroot
```

Do not create one giant list if it can be separated cleanly.

---

# 101. live-build

Use Debian live-build.

The current Debian Trixie package exists as:

```text
live-build
```

Use `auto/config` and `auto/build`.

The ISO should be generated as a bootable hybrid image suitable for USB.

---

# 102. Example auto/config

```bash
#!/bin/sh
set -eu

lb config \
  --distribution trixie \
  --architectures amd64 \
  --binary-images iso-hybrid \
  --archive-areas "main contrib non-free-firmware"
```

Add further options only after testing them.

---

# 103. Build Hooks

Use hooks for:

```text
first-boot setup
cache preparation
HCS configuration
branding
service enablement
cleanup
```

Hooks must be idempotent.

Running a hook twice must not corrupt the build.

---

# 104. Native Packaging

Prefer creating HCS `.deb` packages for:

```text
hcsd
hcs-modeld
hcs-chat
hcs-search
hcs-memory
hcs-agents
hcs-control
hcs-shell
```

This provides:

```text
versioning
dependencies
upgrade
rollback
license metadata
```

---

# 105. Versioning

Use SemVer:

```text
0.1.0-alpha.1
0.1.0-beta.1
0.1.0-rc.1
0.1.0
1.0.0
```

No release should be called stable while mandatory QA gates are missing.

---

# 106. Git Branch Strategy

```text
main
 dev
 feature/*
 fix/*
 security/*
 performance/*
 release/*
 hotfix/*
```

Flow:

```text
feature/* -> PR -> dev
fix/*     -> PR -> dev
security/*-> PR -> dev
performance/* -> PR -> dev
release/* -> main
hotfix/*  -> main + back-merge to dev
```

No direct pushes to `main` during normal development.

---

# 107. Branch Protection

Protect `main` with:

```text
pull request required
CI required
license check required
ISO build required
security check required
no force push
no deletion
```

---

# 108. Commit Format

Use Conventional Commit-like prefixes:

```text
feat:
fix:
security:
perf:
build:
test:
docs:
refactor:
chore:
release:
```

Examples:

```text
feat: add hcs memory panel
fix: unload reasoner after task completion
security: restrict agent filesystem scope
perf: reduce shell idle RSS
build: pin niri source revision
test: add ISO boot smoke test
release: 0.1.0
```

---

# 109. GitHub Release Artifacts

Each release should contain:

```text
HCS-Linux-<version>-amd64.iso
SHA256SUMS
MODEL-MANIFEST.json
THIRD-PARTY-NOTICES.txt
SBOM.spdx.json
RELEASE-NOTES.md
BUILD-MANIFEST.json
```

Do not put GGUF model weights into normal Git history.

---

# 110. Release Provenance

Where supported, publish artifact provenance/attestations.

The build manifest should include:

```text
Git commit
branch
build timestamp
builder
Debian release
source locks
model locks
package inventory
checksums
```

---

# 111. GitHub Actions

CI should have layers.

PR fast path:

```text
lint
unit
schema tests
license metadata check
source lock validation
native compile
small model smoke
```

Integration path:

```text
ISO build
static QA
VM boot
AI smoke
```

Nightly/full path:

```text
full ISO
VM matrix
visual QA
RAM QA
stress tests
privacy tests
security tests
license scan
SBOM
```

---

# 112. CI Reality Rule

GitHub-hosted CI may not have VirtualBox or nested virtualization configured exactly like the Windows development environment.

If a test needs unavailable hardware or virtualization, create:

```text
local-only gate
OR
self-hosted runner gate
```

Do not pretend the test ran externally when it did not.

---

# 113. README Structure

README should be product-oriented:

```text
# HCS Linux

AI-Native • Local-First • Private • CPU-First

What is HCS?
Screenshots
Features
AI Brain
Memory
Agents
Privacy
Live USB
Developer Mode
Security Lab
Hardware
Download
Install
Model Packs
Build
Architecture
Security
Licensing
Roadmap
Contributing
```

Link into detailed docs.

---

# 114. README Honesty

Never write:

```text
perfect
bug-free
anonymous
secure by default
```

without defining the exact tested claim.

Prefer:

```text
Designed for
Measured on
Tested under
Verified by
Known limitations
```

---

# 115. GEMINI Agent Loop

For every engineering task:

```text
READ
 ↓
RESEARCH
 ↓
PLAN
 ↓
FETCH
 ↓
IMPLEMENT
 ↓
BUILD
 ↓
TEST
 ↓
VISUAL QA
 ↓
STRESS
 ↓
LICENSE QA
 ↓
DOCUMENT
 ↓
COMMIT
```

If a step fails:

```text
capture evidence
 ↓
classify root cause
 ↓
fix
 ↓
rebuild
 ↓
rerun failed test
 ↓
rerun dependent tests
```

Do not continue as if failure did not happen.

---

# 116. Deterministic Development

For each fix maintain:

```text
failing test before
patch
passing test after
regression suite
```

Whenever practical, convert a discovered bug into a regression test before or together with the fix.

---

# 117. No Mock Policy

Forbidden in release paths:

```text
mock model responses
fake agent tools
fake shell outputs
fake screenshots
fake network responses
fake installer success
hard-coded PASS strings
```

Mocks may exist only in isolated unit tests when clearly named and never used as proof of integration correctness.

---

# 118. No Hallucinated Evidence

The agent must never invent:

```text
benchmark scores
RAM measurements
test results
license conclusions
hashes
commit IDs
versions
```

All such values must come from tool output or explicit source records.

---

# 119. External Verification

Whenever practical, test an artifact with an independent verifier.

Examples:

```text
independent shell script
separate process
second VM boot
second evaluator model
separate CI job
```

The final release report must distinguish:

```text
self-check
independent check
external check
manual review
```

---

# 120. ISO Verification

At minimum:

```bash
xorriso -indev <iso> -toc
sha256sum <iso>
```

Verify expected boot content and file manifests.

Boot it.

Install it.

Boot the installed system.

Run AI.

Run the Brain.

Run a real agent task.

Shut down and reboot.

---

# 121. VM Automation

Use `VBoxManage` from Windows to:

```text
create VM
attach ISO
start VM
capture screenshot
power off
restore snapshot
```

Keep a disposable QA VM.

Snapshots must not be treated as evidence unless the final state is recorded.

---

# 122. Visual Test Script

Create:

```text
scripts/qa-visual.sh
```

and:

```text
scripts/qa-virtualbox.ps1
```

The scripts should produce deterministic artifact names:

```text
boot.png
desktop.png
launcher.png
chat.png
memory.png
settings.png
security.png
installer.png
```

---

# 123. Stress Report

Create:

```text
qa/reports/stress.json
```

with:

```json
{
  "cycles": 100,
  "crashes": 0,
  "oom": 0,
  "fd_leaks": 0,
  "sqlite_errors": 0,
  "model_unload_failures": 0
}
```

These values must be generated from the actual test run.

---

# 124. Security Release Gate

Before release verify:

```text
agent permission tests
sandbox tests
secret scanning
package provenance
model hash verification
installer privilege behavior
network policy
Tor configuration
LUKS setup path
```

No “security passed” checkbox without evidence.

---

# 125. Privacy Release Gate

Test:

```text
normal mode
private mode
live mode
persistence
reboot
shutdown
```

Inspect:

```text
network sockets
DNS state
browser state
shell history
AI logs
temporary files
swap
persistent memory
```

Document limitations.

---

# 126. AI Privacy

Default behavior:

```text
local inference
no cloud telemetry
local memory
local logs
encrypted vault when enabled
```

If remote models/providers are optionally supported, they must be visibly marked as remote.

Never silently forward user prompts to external services.

---

# 127. Brain Data Retention

Provide controls:

```text
Forget this conversation
Delete memory
Delete project memory
Clear session
Export brain
Import brain
Disable learning
```

Deleting data should be explicit and auditable in the local UI.

---

# 128. Self-Healing vs Unsafe Self-Modification

HCS may automatically:

```text
restart crashed user service
retry transient model load
rebuild corrupted index
repair known schema migration
```

HCS may not automatically:

```text
disable security
replace kernel
rewrite firewall to bypass a failure
modify bootloader blindly
grant itself root
```

---

# 129. Daily Brain Message

The user should receive at most one normal daily summary by default.

The message should be useful, not noisy.

Include:

```text
tasks completed
failures
new lessons
training status
model changes
security advisories relevant to HCS
recommended attention items
```

---

# 130. Long-Term Learning Strategy

HCS learning should progress through levels:

```text
Level 1: memory
Level 2: skills
Level 3: routing statistics
Level 4: verified preference pairs
Level 5: LoRA adapter
Level 6: continual adapter evaluation
Level 7: multi-model teacher/student distillation
```

Do not jump directly to self-training at Level 1.

---

# 131. Teacher / Student Distillation

A stronger optional model may act as a teacher.

Flow:

```text
hard task
 ↓
teacher generates solution
 ↓
objective verifier
 ↓
judge
 ↓
student target
 ↓
SFT / distillation
 ↓
evaluation
```

Teacher outputs still require verification.

A larger teacher is not automatically correct.

---

# 132. Training Cost Awareness

Training is not part of the low-RAM runtime budget.

The runtime target is:

```text
<= 6 GB baseline
<= 8 GB peak
```

Training jobs may require more resources and can run:

```text
separately
on demand
nightly
on another machine
```

The training infrastructure must never consume the normal desktop AI budget during ordinary use.

---

# 133. Optional CPU Training

Small adapters can technically be trained on CPU, but performance will be much slower than GPU training.

The agent must benchmark actual wall-clock time before promising CPU-only training is practical.

Prefer:

```text
small model
small dataset
LoRA
short context
few epochs
```

No full-weight training in the Edge runtime.

---

# 134. Dataset Curation Loop

Nightly:

```text
collect experience
 ↓
dedupe
 ↓
redact
 ↓
verify
 ↓
rank
 ↓
score
 ↓
select
 ↓
write dataset manifest
```

Weekly:

```text
train candidate
 ↓
evaluate
 ↓
compare
 ↓
promote or reject
```

---

# 135. Deterministic Dataset Hashing

Every dataset snapshot needs:

```text
manifest hash
content hashes
schema version
source task IDs
creation time
filter version
```

A training result without dataset provenance is invalid.

---

# 136. Model Self-Analysis

HCS may periodically analyze its own outputs, but the analysis itself must be treated as data, not truth.

Use:

```text
output
objective result
judge
ranker
analyzer
```

The analyzer identifies:

```text
failure class
success pattern
missing context
tool problem
routing problem
memory problem
prompt problem
model problem
```

---

# 137. Root Cause Taxonomy

Every failure should map to one or more:

```text
MODEL
PROMPT
MEMORY
ROUTER
TOOL
ENVIRONMENT
PERMISSION
DEPENDENCY
UI
NETWORK
DATA
UNKNOWN
```

`UNKNOWN` is allowed.

Do not force an explanation where evidence is insufficient.

---

# 138. HCS Brain Confidence

Confidence must be separate from correctness.

Store:

```text
model_confidence
judge_confidence
evidence_strength
retrieval_confidence
final_confidence
```

The system should learn when it is uncertain rather than merely becoming more verbose.

---

# 139. Regression Prevention

Before every model/skill/router update rerun:

```text
basic tasks
memory tasks
agent tasks
coding tasks
privacy tasks
UI tasks
```

A change that fixes one known issue but causes a core regression must not be promoted.

---

# 140. Golden Tasks

Create `data/golden/` with a fixed suite covering:

```text
chat
German
English
search
memory
coding
file operations
UI operation
agent planning
error recovery
security permissions
privacy state
```

Do not let the training loop modify the golden set automatically.

---

# 141. Benchmark Registry

`config/models/evals.yaml` should describe:

```yaml
suite:
  name:
  revision:
  tasks:
  thresholds:
  hardware_profile:
  expected_outputs:
```

Every benchmark run should reference an immutable suite revision.

---

# 142. Build Reproducibility

A release build should be reconstructable from:

```text
Git commit
source locks
package locks
model locks
build environment
configuration
patches
assets
```

The build manifest must list all of them.

---

# 143. Clean Build Requirement

Before release:

```bash
make clean
make fetch
make native
make iso
make verify
```

The clean build should work from no pre-existing build output.

---

# 144. Dirty Build Detection

The build script should detect unexpected artifacts in source directories.

If a build accidentally consumes:

```text
previous binary
previous ISO
untracked model
local-only patch
```

fail the build.

---

# 145. Checksums

Use SHA256 for:

```text
ISO
model files
source archives
release assets
training datasets
adapters
```

Do not fabricate checksums.

---

# 146. Model Manifest

Example:

```yaml
models:
  - id: hcs-controller
    repo: Qwen/Qwen3-0.6B
    revision: <PIN>
    license: Apache-2.0
    role: controller

  - id: hcs-assistant
    repo: Qwen/Qwen3-1.7B
    revision: <PIN>
    license: Apache-2.0
    role: assistant

  - id: hcs-reasoner
    repo: mradermacher/Qwen3-4B-abliterated-GGUF
    revision: <PIN>
    license: Apache-2.0
    role: reasoner
```

Replace `<PIN>` only with real revision values obtained during fetch.

---

# 147. No Placeholder in Release

Strings like:

```text
<PIN>
<TODO>
<INSERT HASH>
example.com
fake checksum
```

must be absent from release metadata unless they are literal documentation placeholders inside docs.

The release manifest must contain real values.

---

# 148. Boot Branding

The ISO should boot into HCS branding:

```text
HCS
HCS Linux
HCS logo
```

Branding must use HCS-owned assets.

---

# 149. Installer Branding

Calamares should use:

```text
HCS logo
HCS colors
a concise welcome screen
privacy profile
AI profile
```

The installer should feel like an HCS application.

---

# 150. HCS Logo

Create a vector logo system:

```text
hcs.svg
hcs-mark.svg
hcs-mono.svg
hcs-white.svg
hcs-boot.svg
```

Ensure:

```text
16px
24px
32px
64px
128px
512px
```

are visually correct.

Do not use a raster-only logo for core UI.

---

# 151. Accessibility

Support:

```text
keyboard navigation
high contrast
reduced motion
screen reader labels
scaling
focus indication
```

Reduced-motion mode must be available.

---

# 152. Failure UX

When HCS cannot complete a task:

show:

```text
what happened
what was verified
what was not verified
possible next step
```

Do not show:

```text
Success!
```

when only part of the task succeeded.

---

# 153. Agent Progress UI

Progress should reflect actual task state:

```text
Queued
Planning
Reading
Executing
Verifying
Needs confirmation
Blocked
Completed
Failed
```

Never display an artificial percentage unless it is backed by a real workflow stage.

---

# 154. Chat History

Every conversation stores:

```text
conversation id
model
model revision
generation settings
memory references
tool events
verification events
```

The raw transcript and derived memory must be separate entities.

---

# 155. Memory Promotion

A chat message should not automatically become a permanent memory.

Promotion levels:

```text
raw
candidate
verified
promoted
superseded
```

This prevents memory pollution.

---

# 156. Contradiction Detector

When two memories conflict:

```text
A says X
B says not-X
```

do not silently choose.

Create:

```text
conflict event
```

and ask for user confirmation when the contradiction materially affects future behavior.

---

# 157. Skill Registry

Each skill contains:

```text
id
version
trigger
purpose
permissions
inputs
outputs
tests
provenance
last_verified
```

A skill without tests should remain experimental.

---

# 158. Prime Agent Skill Integration

Prime Agent skills can be imported into HCS only after:

```text
source review
permission review
dependency review
execution sandbox review
real test
```

Do not let a skill automatically inherit unrestricted host permissions.

---

# 159. Update / Rollback

HCS updater should support:

```text
check
stage
verify
install
reboot if required
rollback
```

A failed update must preserve a known-good boot path.

---

# 160. Recovery Environment

Provide a lightweight recovery mode with:

```text
filesystem check
boot repair
rollback
logs
network diagnostics
model repair
HCS Vault recovery
```

Do not load the full AI stack in recovery mode unless necessary.

---

# 161. Logging

Logs should be structured where possible.

Use:

```text
journald
JSON task records
machine-readable QA reports
```

Keep user-visible logs concise.

---

# 162. Privacy of Logs

Logs must not contain:

```text
passwords
access tokens
private keys
session cookies
raw secrets
```

Redact command arguments where necessary.

---

# 163. Supply Chain Security

For source dependencies:

```text
source pin
license
hash
build recipe
```

For model dependencies:

```text
repo
revision
file
license
hash
conversion source
```

For CI actions:

```text
pin versions carefully
review permissions
minimal GITHUB_TOKEN permissions
```

---

# 164. GitHub Token Permissions

Use least privilege.

Default CI should prefer:

```yaml
permissions:
  contents: read
```

Release workflows may grant narrowly scoped write permissions where required.

---

# 165. Release Workflow

The authoritative release order is:

```text
feature branches
 ↓
PR
 ↓
CI
 ↓
dev
 ↓
release branch
 ↓
freeze
 ↓
full build
 ↓
full QA
 ↓
main
 ↓
tag
 ↓
GitHub Release
 ↓
checksums/SBOM/attestation
```

---

# 166. Release Freeze

At freeze:

```text
model revisions frozen
source revisions frozen
package state frozen
assets frozen
UI baseline frozen
training adapter frozen
```

Only release-blocking fixes may enter.

---

# 167. Release Candidate

Candidate naming:

```text
v0.1.0-rc.1
v0.1.0-rc.2
```

RC must run the complete release suite.

If RC fails:

```text
fix
rebuild
new RC
```

Do not silently mutate an RC artifact after publication.

---

# 168. Stable Release

Stable only after all required gates are green.

Release report must include:

```text
build commit
ISO hash
model manifest hash
RAM peak
boot results
install result
visual result
AI result
agent result
license result
security result
```

---

# 169. What “Done” Means

A feature is DONE only when it is:

```text
implemented
built
executed
tested
verified
documented
```

An ISO is DONE only when it:

```text
builds
boots
starts desktop
starts Brain
runs real local inference
runs real agent task
passes memory tests
passes install test
passes visual QA
passes license audit
passes security gate
```

---

# 170. Final Autonomous End-to-End Loop

When the user says:

```text
READ GEMINI.md AND BUILD HCS LINUX
```

the agent should autonomously do:

```text
1. inspect empty folder
2. bootstrap Git
3. create project skeleton
4. research current Debian/live-build/niri/Quickshell/Prime Agent state
5. research current candidate models and licenses
6. create source/model/package locks
7. fetch sources headlessly
8. fetch required model files headlessly
9. verify checksums
10. build native components
11. build minimum Debian Live ISO
12. boot first smoke ISO
13. implement HCS Shell
14. implement HCS Brain
15. implement model daemon
16. integrate real local inference
17. implement Memory
18. implement Reranker retrieval
19. integrate Prime Agent
20. implement subagent permission system
21. implement daily brain report
22. implement learning/event ledger
23. implement judge + analyzer
24. implement training-candidate pipeline
25. implement optional LoRA trainer
26. implement Developer Mode
27. implement Security Lab
28. implement Privacy Mode
29. implement Tor integration
30. implement Vault/LUKS flow
31. implement Calamares integration
32. implement HCS branding
33. implement HCS docs/help
34. run code QA
35. run unit/integration tests
36. run real agent tests
37. run RAM tests
38. build clean ISO
39. boot ISO in VirtualBox
40. run visual QA
41. run installer QA
42. boot installed system
43. run post-install AI tests
44. run privacy/security tests
45. run stress tests
46. run license scan
47. generate SBOM
48. generate third-party notices
49. generate source/model manifests
50. generate release report
51. push feature/development commits
52. create release branch
53. run release candidate suite
54. merge main only if gates pass
55. tag stable release
56. publish ISO + checksums + SBOM + notices
57. publish provenance/attestation where available
```

---

# 171. Autonomous Loop Termination Rule

The agent must continue until one of these is true:

```text
SUCCESS:
all required gates pass

BLOCKED:
required verification is impossible in the available environment

HUMAN_DECISION_REQUIRED:
license choice, destructive storage operation, unresolved security risk, or other decision requiring explicit approval
```

Never terminate with:

```text
probably works
mostly done
should be fine
looks good
```

---

# 172. Error Recovery Loop

When a build/test fails:

```text
FAIL
 ↓
collect logs
 ↓
collect exact command
 ↓
collect environment
 ↓
reproduce
 ↓
search upstream issue/docs
 ↓
form root-cause hypothesis
 ↓
implement smallest fix
 ↓
run targeted test
 ↓
run regression suite
 ↓
continue
```

After three failed hypotheses, widen the research rather than blindly repeating the same patch.

---

# 173. External Research During Debugging

When needed, search:

```text
official upstream issue
current release notes
current docs
Debian bug tracker
GitHub issue/PR
Hugging Face model card
```

Record useful external evidence in:

```text
qa/reports/debug/<issue-id>.md
```

---

# 174. Visual QA During Development

Do not wait until the end.

After every major UI milestone:

```text
build
boot
screenshot
compare
fix
```

Major milestones:

```text
shell
launcher
chat
memory
settings
control
installer
private mode
```

---

# 175. Code Quality

Preferred HCS core language:

```text
Rust
```

Use Python for:

```text
training
research scripts
model conversion
data pipelines
evaluation
```

Use QML/QtQuick for the shell where appropriate.

Avoid creating a permanent high-overhead runtime merely because it is convenient during prototyping.

---

# 176. Testing Languages / Tools

Use the actual ecosystem tools:

```text
Rust tests
cargo clippy
cargo fmt --check
Python pytest
ruff/mypy where appropriate
shellcheck
JSON schema validators
SPDX/license scanners
ScanCode
ORT where useful
```

Never rely on formatting checks as correctness tests.

---

# 177. Build Reproducibility Test

At least periodically:

```text
clean tree
fresh WSL build environment
fetch using locks
build
compare release manifest
```

If two builds differ, determine why before calling the pipeline reproducible.

---

# 178. Binary Difference Handling

Some timestamps/metadata may naturally differ.

Use normalized build inputs where possible.

For important binaries verify:

```text
source revision
build flags
linked dependencies
embedded assets
```

Do not fake bit-for-bit reproducibility claims.

---

# 179. Documentation Build

Before release:

```text
docs links valid
documentation files present
screenshots current
commands tested
versions current
license docs current
```

If documentation command is not tested, label it as unverified.

---

# 180. Final Release Directory

Create:

```text
dist/
├── HCS-Linux-vX.Y.Z-amd64.iso
├── SHA256SUMS
├── MODEL-MANIFEST.json
├── BUILD-MANIFEST.json
├── SBOM.spdx.json
├── THIRD-PARTY-NOTICES.txt
├── LICENSES.tar.zst
├── RELEASE-NOTES.md
└── QA-REPORT.md
```

---

# 181. Daily / Weekly / Release Automation

Daily:

```text
experience curation
memory consolidation
brain report
security advisory check
```

Weekly:

```text
training candidate build
model evaluation
router analysis
skill review
```

Release:

```text
full build
full QA
full license audit
full visual QA
full VM install
release artifacts
```

---

# 182. Never Train Automatically From Raw Chat

Raw chat is not a training set.

Required transformations:

```text
raw chat
 ↓
redact
 ↓
segment
 ↓
verify
 ↓
score
 ↓
correct
 ↓
dedupe
 ↓
rank
 ↓
freeze dataset revision
```

Only then may the record become a training candidate.

---

# 183. Best Data Comes From Tasks With Ground Truth

Highest-value sources:

```text
code + tests
system action + observable state
UI action + screenshot
file operation + hash
structured answer + validator
```

Lower-value sources:

```text
subjective conversation
free-form brainstorming
unverified opinions
```

Weight the data accordingly.

---

# 184. Brain Evolution Without Weight Corruption

The safest progression is:

```text
memory -> skill -> router -> preference dataset -> adapter
```

The underlying base model remains immutable.

This provides rollback and prevents uncontrolled self-modification.

---

# 185. Model Selection Policy

Do not choose a model because it is called:

```text
uncensored
abliterated
heretic
ultimate
reasoning
```

Choose it because measured evidence shows it fits the target role.

Abliteration is a behavioral modification; it must be benchmarked independently.

---

# 186. Model Candidate Promotion

Candidate -> accepted when:

```text
license clear
source pinned
hash recorded
runtime works
prompt format correct
tool calls work
quality suite passes
RAM suite passes
no critical regressions
```

---

# 187. Default Model Policy Example

Use placeholders only during development.

Production values must be resolved automatically during `make lock-models` or equivalent.

Example conceptual output:

```yaml
controller: <verified checkpoint>
assistant: <verified checkpoint>
coder: <verified checkpoint>
reasoner: <verified checkpoint>
reranker: <verified checkpoint>
embedding: <verified checkpoint>
judge: <verified checkpoint>
```

---

# 188. Release Metrics Dashboard

HCS Control should eventually show:

```text
Brain quality
Task success
Correction rate
Average latency
RAM peak
Agent success
Memory hit rate
Model usage
Training delta
```

Never show fake “AI intelligence” points.

---

# 189. HCS Quality Loop

The whole operating system follows:

```text
OBSERVE
→ THINK
→ ACT
→ VERIFY
→ REMEMBER
→ IMPROVE
→ VERIFY AGAIN
```

This is the defining HCS Brain loop.

---

# 190. Production Philosophy

HCS Linux is not complete because the desktop looks beautiful.

It is not complete because an AI chat window works.

It is not complete because an ISO builds.

It is complete only when the complete chain works:

```text
HARDWARE
→ BOOT
→ DESKTOP
→ BRAIN
→ MODEL
→ MEMORY
→ AGENT
→ TOOLS
→ VERIFICATION
→ LEARNING
→ PRIVACY
→ INSTALLATION
→ UPDATE
→ ROLLBACK
→ RELEASE
```

---

# 191. Source References Used for This Architecture

Current references that should be rechecked during implementation:

- Debian 13.7 stable release: https://www.debian.org/News/2026/20260912
- Debian Trixie release/support information: https://www.debian.org/releases/stable/index.en.html
- Debian live-build package: https://packages.debian.org/trixie/live-build
- Debian Calamares package: https://packages.debian.org/trixie/utils/calamares
- Prime Agent: https://github.com/PrimeIntellect-ai/prime-agent
- llama.cpp: https://github.com/ggml-org/llama.cpp
- niri: https://github.com/niri-wm/niri
- Quickshell: https://github.com/quickshell-mirror/quickshell
- Qwen3-0.6B: https://huggingface.co/Qwen/Qwen3-0.6B
- Qwen3-1.7B: https://huggingface.co/Qwen/Qwen3-1.7B
- Qwen3-4B: https://huggingface.co/Qwen/Qwen3-4B
- Qwen3-0.6B abliterated: https://huggingface.co/mlabonne/Qwen3-0.6B-abliterated
- Qwen3-0.6B abliterated GGUF: https://huggingface.co/mlabonne/Qwen3-0.6B-abliterated-GGUF
- Qwen3-1.7B abliterated: https://huggingface.co/mlabonne/Qwen3-1.7B-abliterated
- Qwen3-4B abliterated: https://huggingface.co/mlabonne/Qwen3-4B-abliterated
- Qwen3-4B abliterated GGUF: https://huggingface.co/mradermacher/Qwen3-4B-abliterated-GGUF
- Qwen3-8B abliterated: https://huggingface.co/mlabonne/Qwen3-8B-abliterated
- Qwen3-14B abliterated: https://huggingface.co/mlabonne/Qwen3-14B-abliterated
- Qwen3-30B-A3B abliterated: https://huggingface.co/mlabonne/Qwen3-30B-A3B-abliterated
- Qwen2.5-Coder-0.5B: https://huggingface.co/Qwen/Qwen2.5-Coder-0.5B-Instruct
- Qwen2.5-Coder-0.5B abliterated: https://huggingface.co/huihui-ai/Qwen2.5-Coder-0.5B-Instruct-abliterated
- Qwen2.5-Coder-1.5B: https://huggingface.co/Qwen/Qwen2.5-Coder-1.5B-Instruct
- Qwen2.5-Coder-1.5B abliterated: https://huggingface.co/huihui-ai/Qwen2.5-Coder-1.5B-Instruct-abliterated
- Qwen2.5-Coder-3B abliterated: https://huggingface.co/huihui-ai/Qwen2.5-Coder-3B-Instruct-abliterated
- SmolLM3 abliterated: https://huggingface.co/richardyoung/SmolLM3-3B-abliterated-obliteratus
- Phi-4-mini abliterated: https://huggingface.co/huihui-ai/Phi-4-mini-instruct-abliterated
- Llama 3.2 official model card: https://github.com/meta-llama/llama-models/blob/main/models/llama3_2/MODEL_CARD.md
- Llama 3.2 abliterated: https://huggingface.co/huihui-ai/Llama-3.2-3B-Instruct-abliterated
- Gemma 3 abliterated GGUF: https://huggingface.co/mlabonne/gemma-3-4b-it-abliterated-GGUF
- Qwen3 Reranker 0.6B: https://huggingface.co/Qwen/Qwen3-Reranker-0.6B
- Qwen3 Embedding 0.6B: https://huggingface.co/Qwen/Qwen3-Embedding-0.6B
- Qwen3 Embedding GGUF: https://huggingface.co/Qwen/Qwen3-Embedding-0.6B-GGUF
- BGE reranker: https://huggingface.co/BAAI/bge-reranker-v2-m3
- Jina reranker: https://huggingface.co/jinaai/jina-reranker-v2-base-multilingual
- compact Qwen reward model: https://huggingface.co/puwaer/Safe-Reward-Qwen3-0.6B
- compact preference RM: https://huggingface.co/ajeet9843/qwen2.5-0.5b-hh-rlhf-rm
- SmolVLM 500M: https://huggingface.co/HuggingFaceTB/SmolVLM-500M-Instruct
- Florence-2: https://huggingface.co/microsoft/Florence-2-base
- Whisper Small: https://huggingface.co/openai/whisper-small
- PEFT: https://huggingface.co/docs/peft
- TRL: https://huggingface.co/docs/trl
- TRL RewardTrainer: https://github.com/huggingface/trl/blob/main/docs/source/reward_trainer.md
- TRL SFTTrainer: https://github.com/huggingface/trl/blob/main/docs/source/sft_trainer.md
- GitHub releases: https://cli.github.com/manual/gh_release_create
- GitHub artifact attestations: https://docs.github.com/en/actions/concepts/security/artifact-attestations
- GitHub branch protection: https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches
- SPDX: https://spdx.org/licenses/
- ScanCode: https://scancode-toolkit.readthedocs.io/
- OSS Review Toolkit: https://oss-review-toolkit.org/

All versions, licenses, model cards and availability must be rechecked before a release because external projects change.

---

# 192. Final Agent Instruction

You are the implementation agent for HCS Linux.

Do not merely describe what should be built.

Build it.

Do not stop after writing source files.

Compile it.

Do not stop after compilation.

Run it.

Do not stop after running it.

Verify it.

Do not stop after verification.

Run the release pipeline.

Do not state that the ISO is perfect or bug-free unless the exact release criteria have objective evidence.

Your final response must contain:

```text
STATUS

COMMIT

ISO PATH

ISO SHA256

MODEL MANIFEST

RAM RESULTS

BOOT RESULTS

INSTALL RESULTS

AI RESULTS

AGENT RESULTS

VISUAL QA RESULTS

STRESS RESULTS

PRIVACY RESULTS

SECURITY RESULTS

LICENSE RESULTS

SBOM PATH

KNOWN LIMITATIONS

UNVERIFIED GATES
```

If any mandatory gate is not verified, the final status must be `BLOCKED` rather than `PASS`.

**Build HCS Linux as a real product, with evidence at every layer.**
