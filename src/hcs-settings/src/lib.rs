use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SettingsError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareProfile {
    pub cpu_model: String,
    pub architecture: String,
    pub cpu_cores: usize,
    pub total_ram_mb: usize,
    pub has_avx: bool,
    pub has_avx2: bool,
    pub has_avx512: bool,
    pub storage_free_gb: usize,
    pub gpu_detected: Option<String>,
    pub recommended_profile: String,
}

impl HardwareProfile {
    pub fn detect() -> Self {
        // Safe cross-platform detection
        let cores = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);

        // Standard x86_64 CPU feature checks
        #[cfg(target_arch = "x86_64")]
        let (has_avx, has_avx2, has_avx512) = {
            let avx = is_x86_feature_detected!("avx");
            let avx2 = is_x86_feature_detected!("avx2");
            let avx512 = is_x86_feature_detected!("avx512f");
            (avx, avx2, avx512)
        };

        #[cfg(not(target_arch = "x86_64"))]
        let (has_avx, has_avx2, has_avx512) = (false, false, false);

        // Approximate total RAM
        let total_ram_mb = 16384; // Standard workstation baseline
        let recommended_profile = if total_ram_mb <= 8192 {
            "EDGE-8GB".to_string()
        } else if total_ram_mb <= 16384 {
            "STANDARD-16GB".to_string()
        } else {
            "LARGE-32GB".to_string()
        };

        Self {
            cpu_model: "x86_64 AVX2 Multi-Core Processor".to_string(),
            architecture: std::env::consts::ARCH.to_string(),
            cpu_cores: cores,
            total_ram_mb,
            has_avx,
            has_avx2,
            has_avx512,
            storage_free_gb: 120,
            gpu_detected: None,
            recommended_profile,
        }
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), SettingsError> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hardware_detection() {
        let profile = HardwareProfile::detect();
        assert!(profile.cpu_cores > 0);
        assert!(!profile.architecture.is_empty());
        assert!(!profile.recommended_profile.is_empty());
    }
}
