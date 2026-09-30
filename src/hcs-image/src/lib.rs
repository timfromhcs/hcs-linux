//! HCS Linux — Offline CPU Image Generation (`hcs-image`) v1.0.0 Stable
//! Spec: docs/V1_STABLE_RELEASE_MASTER_PLAN.md §4.2
//! Engine: stable-diffusion.cpp (AVX2/AVX-512) via /usr/lib/hcs/sd-cpp backend
//! Model: SD 1.5 LCM / SD-Turbo GGUF q4_0 (~1.6GB disk, 1.8-2.2GB peak RSS, pure CPU)

pub mod engine;
pub mod memory_guard;
pub mod postprocess;

pub use engine::{EngineConfig, ImageMode, InferenceRequest};
pub use memory_guard::{MemoryGuard, MemoryGuardError};
pub use postprocess::tag_png_metadata;
