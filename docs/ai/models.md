# HCS Linux Model Architecture & Profiles

HCS Linux features a role-based, on-demand AI model architecture.

## Model Catalog

| Role | Candidate Model | Quantization | Expected RAM | Tier | License |
|------|-----------------|--------------|--------------|------|---------|
| **Controller** | `Qwen3-0.6B` | Q4_K_M | ~650 MB | Resident | Apache-2.0 |
| **Assistant** | `Qwen3-1.7B` | Q4_K_M | ~1450 MB | Interactive | Apache-2.0 |
| **Coder** | `Qwen2.5-Coder-1.5B` | Q4_K_M | ~1350 MB | On-Demand | Apache-2.0 |
| **Reasoner** | `Qwen3-4B-abliterated` | Q4_K_M | ~2850 MB | On-Demand | Apache-2.0 |
| **Embedding** | `Qwen3-Embedding-0.6B` | Q8_0 | ~680 MB | On-Demand | Apache-2.0 |
| **Reranker** | `Qwen3-Reranker-0.6B` | Q8_0 | ~680 MB | On-Demand | Apache-2.0 |

## Single Heavy Model Resident Rule

To guarantee that HCS Linux strictly abides by the `<= 8 GB RAM` normal/heavy peak budget:
- Only **one** generation model $\ge 1.0B$ is loaded in memory at any given time on the Edge profile.
- When the user or an agent switches from Chat (Assistant 1.7B) to Coding (Coder 1.5B) or Deep Reasoning (Reasoner 4B), `hcs-modeld` gracefully unloads the prior model before loading the requested model.
