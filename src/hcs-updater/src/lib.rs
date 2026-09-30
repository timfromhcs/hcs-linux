use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum UpdateError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Git sync failed: {0}")]
    GitSyncFailed(String),
    #[error("Verification failed: {0}")]
    VerificationFailed(String),
    #[error("Rollback failed: {0}")]
    RollbackFailed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateConfig {
    pub repo_url: String,
    pub branch: String,
    pub install_bin_dir: PathBuf,
    pub backup_dir: PathBuf,
    pub auto_reload_units: bool,
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            repo_url: "https://github.com/timfromhcs/hcs-linux.git".to_string(),
            branch: "main".to_string(),
            install_bin_dir: PathBuf::from("/usr/bin"),
            backup_dir: PathBuf::from("/var/backups/hcs"),
            auto_reload_units: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateStatus {
    pub current_commit: String,
    pub remote_commit: String,
    pub update_available: bool,
    pub last_checked: String,
    pub status: String,
}

pub struct UpdateManager {
    config: UpdateConfig,
}

impl UpdateManager {
    pub fn new(config: UpdateConfig) -> Self {
        Self { config }
    }

    pub fn check_status(&self, current_rev: &str, remote_rev: &str) -> UpdateStatus {
        let is_available = !remote_rev.is_empty() && current_rev != remote_rev;
        UpdateStatus {
            current_commit: current_rev.to_string(),
            remote_commit: remote_rev.to_string(),
            update_available: is_available,
            last_checked: Utc::now().to_rfc3339(),
            status: if is_available {
                "Update Available".to_string()
            } else {
                "Up to Date".to_string()
            },
        }
    }

    pub fn backup_binary(&self, bin_name: &str) -> Result<PathBuf, UpdateError> {
        let src_path = self.config.install_bin_dir.join(bin_name);
        fs::create_dir_all(&self.config.backup_dir)?;
        let backup_path = self.config.backup_dir.join(format!("{}.bak", bin_name));

        if src_path.exists() {
            fs::copy(&src_path, &backup_path)?;
        }
        Ok(backup_path)
    }

    pub fn atomic_replace_binary(
        &self,
        bin_name: &str,
        new_binary_bytes: &[u8],
    ) -> Result<(), UpdateError> {
        fs::create_dir_all(&self.config.install_bin_dir)?;
        let target_path = self.config.install_bin_dir.join(bin_name);
        let temp_path = self.config.install_bin_dir.join(format!(
            "{}.tmp.{}",
            bin_name,
            Utc::now().timestamp_millis()
        ));

        fs::write(&temp_path, new_binary_bytes)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&temp_path)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&temp_path, perms)?;
        }

        fs::rename(&temp_path, &target_path)?;
        Ok(())
    }

    pub fn rollback_binary(&self, bin_name: &str) -> Result<bool, UpdateError> {
        let backup_path = self.config.backup_dir.join(format!("{}.bak", bin_name));
        let target_path = self.config.install_bin_dir.join(bin_name);

        if backup_path.exists() {
            fs::copy(&backup_path, &target_path)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn run_health_check(&self, test_binary_path: &Path) -> bool {
        if !test_binary_path.exists() {
            return false;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_update_status_detection() {
        let mgr = UpdateManager::new(UpdateConfig::default());
        let status = mgr.check_status("rev-1234", "rev-5678");
        assert!(status.update_available);
        assert_eq!(status.status, "Update Available");

        let status_synced = mgr.check_status("rev-1234", "rev-1234");
        assert!(!status_synced.update_available);
        assert_eq!(status_synced.status, "Up to Date");
    }

    #[test]
    fn test_atomic_binary_replacement_and_rollback() {
        let temp = tempdir().unwrap();
        let bin_dir = temp.path().join("bin");
        let backup_dir = temp.path().join("backups");

        let config = UpdateConfig {
            repo_url: "https://example.com/repo.git".to_string(),
            branch: "main".to_string(),
            install_bin_dir: bin_dir.clone(),
            backup_dir: backup_dir.clone(),
            auto_reload_units: false,
        };
        let mgr = UpdateManager::new(config);

        // 1. Initial write
        mgr.atomic_replace_binary("test-daemon", b"VERSION_1")
            .unwrap();
        let target = bin_dir.join("test-daemon");
        assert_eq!(fs::read(&target).unwrap(), b"VERSION_1");

        // 2. Backup
        mgr.backup_binary("test-daemon").unwrap();
        assert!(backup_dir.join("test-daemon.bak").exists());

        // 3. Update to Version 2
        mgr.atomic_replace_binary("test-daemon", b"VERSION_2")
            .unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"VERSION_2");

        // 4. Rollback
        let rolled = mgr.rollback_binary("test-daemon").unwrap();
        assert!(rolled);
        assert_eq!(fs::read(&target).unwrap(), b"VERSION_1");
    }
}
