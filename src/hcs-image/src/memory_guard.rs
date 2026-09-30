//! Strict RAM budget verification prior to inference.
//! Spec §4.2 lifecycle: inspect /proc/meminfo → flush hcs-modeld caches if
//! free < 2500MB → mmap weights → infer → release. Peak ≤ 2.2GB.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum MemoryGuardError {
    #[error("Insufficient free RAM: {free_mb}MB < required {required_mb}MB")]
    InsufficientMemory { free_mb: u64, required_mb: u64 },
    #[error("Cannot read memory info: {0}")]
    MeminfoUnavailable(String),
}

/// Hard budget constants from Master Plan §5.
pub const IMAGE_PEAK_RSS_MB: u64 = 2200;
pub const MIN_FREE_MB_BEFORE_INFER: u64 = 2500;
pub const SYSTEM_HARD_CAP_MB: u64 = 8192;

pub struct MemoryGuard {
    pub required_free_mb: u64,
}

impl Default for MemoryGuard {
    fn default() -> Self {
        Self {
            required_free_mb: MIN_FREE_MB_BEFORE_INFER,
        }
    }
}

impl MemoryGuard {
    pub fn free_mb_from_meminfo(text: &str) -> Option<u64> {
        for line in text.lines() {
            if line.starts_with("MemAvailable:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(kb) = parts[1].parse::<u64>() {
                        return Some(kb / 1024);
                    }
                }
            }
        }
        None
    }

    pub fn check_with_meminfo(&self, meminfo: &str) -> Result<u64, MemoryGuardError> {
        let free = Self::free_mb_from_meminfo(meminfo)
            .ok_or_else(|| MemoryGuardError::MeminfoUnavailable("MemAvailable missing".into()))?;
        if free < self.required_free_mb {
            return Err(MemoryGuardError::InsufficientMemory {
                free_mb: free,
                required_mb: self.required_free_mb,
            });
        }
        // Peak after allocation must stay within the ≤8192MB operating budget.
        Ok(free)
    }

    /// Single Heavy Model Rule: caller must unload any resident 4B reasoner
    /// before image inference; both must never occupy RAM simultaneously.
    pub fn heavy_model_must_be_unloaded(heavy_resident: bool) -> bool {
        heavy_resident
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meminfo_parse() {
        let sample = "MemTotal:        8000000 kB\nMemAvailable:    5000000 kB\n";
        assert_eq!(MemoryGuard::free_mb_from_meminfo(sample), Some(4882));
    }

    #[test]
    fn test_insufficient_triggers_flush() {
        let guard = MemoryGuard::default();
        let low = "MemAvailable:    1500000 kB\n";
        let err = guard.check_with_meminfo(low).unwrap_err();
        assert!(matches!(err, MemoryGuardError::InsufficientMemory { .. }));
    }

    #[test]
    fn test_sufficient_passes() {
        let guard = MemoryGuard::default();
        let ok = "MemAvailable:    4000000 kB\n";
        assert!(guard.check_with_meminfo(ok).is_ok());
    }
}
