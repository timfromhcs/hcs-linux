# Continual Improvement & Adapter Training

HCS Linux features an anti-self-training learning loop designed to prevent model drift and hallucination spirals.

## Anti-Self-Training Rules

1. **Objective Verification Required:** A model output may never enter `data/training-candidates/` unless it is backed by deterministic evidence (passing test suite, matching hash, verified file change).
2. **Immutable Holdout Sets:** Training strictly excludes all golden evaluation tasks stored in `data/golden/` and `data/eval/`.
3. **Champion vs. Challenger Lifecycle:** Newly trained LoRA adapters are evaluated against the existing champion model across regression suites. Adapters are promoted only if they improve performance without causing critical task regressions.
4. **User-Controlled Modes:**
   - `Off`: Learning disabled.
   - `Curate`: Collects verified candidates without fine-tuning.
   - `Auto-Adapt`: Trains compact LoRA adapters on schedule or when threshold reached.
