use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum AgentError {
    #[error("Permission denied: role '{role}' is not granted capability '{capability}'")]
    PermissionDenied { role: String, capability: String },
    #[error("Privileged action requires explicit interactive confirmation: {0}")]
    ConfirmationRequired(String),
    #[error("Root execution is forbidden for autonomous agent tasks")]
    RootExecutionForbidden,
    #[error("Task execution failed with exit code: {0}")]
    ExecutionFailed(i32),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentRole {
    Planner,
    Researcher,
    Coder,
    Debugger,
    Browser,
    Filesystem,
    System,
    SecurityLab,
    Privacy,
    Ui,
    VisualQa,
    Qa,
    LicenseAuditor,
    ReleaseManager,
    MemoryCurator,
    TrainingCurator,
    Judge,
    Critic,
    Verifier,
}

impl AgentRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Planner => "planner",
            Self::Researcher => "researcher",
            Self::Coder => "coder",
            Self::Debugger => "debugger",
            Self::Browser => "browser",
            Self::Filesystem => "filesystem",
            Self::System => "system",
            Self::SecurityLab => "security-lab",
            Self::Privacy => "privacy",
            Self::Ui => "ui",
            Self::VisualQa => "visual-qa",
            Self::Qa => "qa",
            Self::LicenseAuditor => "license-auditor",
            Self::ReleaseManager => "release-manager",
            Self::MemoryCurator => "memory-curator",
            Self::TrainingCurator => "training-curator",
            Self::Judge => "judge",
            Self::Critic => "critic",
            Self::Verifier => "verifier",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    FilesystemRead,
    FilesystemWrite,
    FilesystemDelete,
    AppOpen,
    WindowControl,
    BrowserOpen,
    BrowserRead,
    NetworkFetch,
    GitRead,
    GitWrite,
    ProcessRead,
    ProcessSpawn,
    SystemSettings,
    PackageInstall,
    RootExecute,
    PartitionModify,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PermissionClass {
    Read,
    Write,
    Reversible,
    Privileged,
    Irreversible,
    Network,
}

pub struct AgentPermissions;

impl AgentPermissions {
    pub fn get_allowed_capabilities(role: AgentRole) -> HashSet<Capability> {
        let mut caps = HashSet::new();
        // Common safe read capabilities
        caps.insert(Capability::FilesystemRead);
        caps.insert(Capability::ProcessRead);
        caps.insert(Capability::GitRead);

        match role {
            AgentRole::Researcher => {
                caps.insert(Capability::BrowserOpen);
                caps.insert(Capability::BrowserRead);
                caps.insert(Capability::NetworkFetch);
            }
            AgentRole::Coder | AgentRole::Debugger => {
                caps.insert(Capability::FilesystemWrite);
                caps.insert(Capability::GitWrite);
                caps.insert(Capability::ProcessSpawn);
            }
            AgentRole::Filesystem => {
                caps.insert(Capability::FilesystemWrite);
                caps.insert(Capability::FilesystemDelete);
            }
            AgentRole::System => {
                caps.insert(Capability::ProcessSpawn);
                caps.insert(Capability::SystemSettings);
            }
            AgentRole::ReleaseManager => {
                caps.insert(Capability::FilesystemWrite);
                caps.insert(Capability::GitWrite);
                caps.insert(Capability::ProcessSpawn);
            }
            AgentRole::Qa | AgentRole::VisualQa | AgentRole::Verifier => {
                caps.insert(Capability::ProcessSpawn);
                caps.insert(Capability::WindowControl);
            }
            AgentRole::SecurityLab => {
                caps.insert(Capability::ProcessSpawn);
                caps.insert(Capability::NetworkFetch);
            }
            AgentRole::Planner
            | AgentRole::Judge
            | AgentRole::Critic
            | AgentRole::MemoryCurator
            | AgentRole::TrainingCurator
            | AgentRole::LicenseAuditor
            | AgentRole::Privacy
            | AgentRole::Browser
            | AgentRole::Ui => {}
        }
        caps
    }

    pub fn check_permission(
        role: AgentRole,
        capability: Capability,
        is_user_confirmed: bool,
    ) -> Result<(), AgentError> {
        if capability == Capability::RootExecute {
            return Err(AgentError::RootExecutionForbidden);
        }

        let allowed = Self::get_allowed_capabilities(role);
        if !allowed.contains(&capability) {
            return Err(AgentError::PermissionDenied {
                role: role.as_str().to_string(),
                capability: format!("{:?}", capability),
            });
        }

        // Privileged checks
        if (capability == Capability::FilesystemDelete || capability == Capability::PackageInstall)
            && !is_user_confirmed
        {
            return Err(AgentError::ConfirmationRequired(format!(
                "Action {:?} requires explicit user approval",
                capability
            )));
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationConfig {
    pub temperature: f64,
    pub top_p: f64,
    pub seed: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionOutcome {
    pub exit_code: i32,
    pub tests_passed: usize,
    pub tests_failed: usize,
    pub modified_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualEvidence {
    pub screenshot_path: Option<String>,
    pub regression_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub peak_rss_mb: usize,
    pub duration_ms: u64,
}

/// Experience / Event Ledger record adhering to Section 20 of GEMINI.md
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskLedgerRecord {
    pub task_id: String,
    pub timestamp: DateTime<Utc>,
    pub user_intent: String,
    pub context_hash: String,
    pub model: String,
    pub model_revision: String,
    pub generation_config: GenerationConfig,
    pub tool_calls: Vec<String>,
    pub output: String,
    pub execution: ExecutionOutcome,
    pub visual: VisualEvidence,
    pub resource: ResourceUsage,
    pub user_feedback: Option<String>,
}

impl TaskLedgerRecord {
    pub fn new(
        user_intent: &str,
        model: &str,
        model_revision: &str,
        output: &str,
        execution: ExecutionOutcome,
        resource: ResourceUsage,
    ) -> Self {
        let task_id = Uuid::new_v4().to_string();
        let timestamp = Utc::now();
        let mut hasher = Sha256::new();
        hasher.update(user_intent.as_bytes());
        hasher.update(output.as_bytes());
        let context_hash = hex::encode(hasher.finalize());

        Self {
            task_id,
            timestamp,
            user_intent: user_intent.to_string(),
            context_hash,
            model: model.to_string(),
            model_revision: model_revision.to_string(),
            generation_config: GenerationConfig {
                temperature: 0.0,
                top_p: 1.0,
                seed: 42,
            },
            tool_calls: Vec::new(),
            output: output.to_string(),
            execution,
            visual: VisualEvidence {
                screenshot_path: None,
                regression_score: 0.0,
            },
            resource,
            user_feedback: None,
        }
    }

    pub fn to_json(&self) -> Result<String, AgentError> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_least_privilege_enforcement() {
        // Researcher should have network fetch but cannot delete files
        assert!(AgentPermissions::check_permission(
            AgentRole::Researcher,
            Capability::NetworkFetch,
            false
        )
        .is_ok());
        assert!(AgentPermissions::check_permission(
            AgentRole::Researcher,
            Capability::FilesystemDelete,
            false
        )
        .is_err());

        // Coder has process spawn and fs write
        assert!(AgentPermissions::check_permission(
            AgentRole::Coder,
            Capability::FilesystemWrite,
            false
        )
        .is_ok());
        assert!(AgentPermissions::check_permission(
            AgentRole::Coder,
            Capability::ProcessSpawn,
            false
        )
        .is_ok());

        // Root execute is forbidden unconditionally
        assert!(AgentPermissions::check_permission(
            AgentRole::System,
            Capability::RootExecute,
            true
        )
        .is_err());
    }

    #[test]
    fn test_task_ledger_creation() {
        let record = TaskLedgerRecord::new(
            "Build HCS Linux ISO",
            "qwen3-1.7b",
            "rev-1",
            "ISO built and verified",
            ExecutionOutcome {
                exit_code: 0,
                tests_passed: 15,
                tests_failed: 0,
                modified_files: vec!["dist/HCS-Linux-0.1.0-alpha.1-amd64.iso".to_string()],
            },
            ResourceUsage {
                peak_rss_mb: 4120,
                duration_ms: 12500,
            },
        );

        assert_eq!(record.execution.exit_code, 0);
        assert_eq!(record.execution.tests_passed, 15);
        assert!(!record.context_hash.is_empty());
    }
}
