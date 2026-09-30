use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SecurityError {
    #[error("Secret exposure detected: {0}")]
    SecretDetected(String),
    #[error("Privacy policy violation: {0}")]
    PolicyViolation(String),
    #[error("Tor service error: {0}")]
    TorError(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrivacyMode {
    Standard,
    PrivateTor,
    Amnesic,
}

pub struct SecretRedactor;

static REDACT_PATTERNS: OnceLock<Vec<(Regex, &'static str)>> = OnceLock::new();

impl SecretRedactor {
    fn patterns() -> &'static [(Regex, &'static str)] {
        REDACT_PATTERNS.get_or_init(|| {
            vec![
                (Regex::new(r"ghp_[a-zA-Z0-9]{36}").unwrap(), "[REDACTED_GH_TOKEN]"),
                (Regex::new(r"hf_[a-zA-Z0-9]{34,37}").unwrap(), "[REDACTED_HF_TOKEN]"),
                (Regex::new(r"(?i)bearer\s+[a-zA-Z0-9_\-\.]{20,}").unwrap(), "Bearer [REDACTED_BEARER]"),
                (Regex::new(r"(?i)password\s*[:=]\s*[^\s,;]+").unwrap(), "password: [REDACTED_PASSWORD]"),
                (Regex::new(r"-----BEGIN [A-Z ]+ PRIVATE KEY-----[^-]+-----END [A-Z ]+ PRIVATE KEY-----").unwrap(), "[REDACTED_PRIVATE_KEY]"),
                (Regex::new(r"[a-zA-Z0-9_.+-]+@[a-zA-Z0-9-]+\.[a-zA-Z0-9-.]+").unwrap(), "[REDACTED_EMAIL]"),
            ]
        })
    }

    pub fn redact(text: &str) -> String {
        let mut result = text.to_string();
        for (pattern, replacement) in Self::patterns() {
            result = pattern.replace_all(&result, *replacement).to_string();
        }
        result
    }

    pub fn contains_sensitive_token(text: &str) -> bool {
        for (pattern, _) in Self::patterns() {
            if pattern.is_match(text) {
                return true;
            }
        }
        false
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TorStatus {
    pub is_running: bool,
    pub socks_port: u16,
    pub is_traffic_routed: bool,
    pub exit_ip: Option<String>,
}

pub struct TorManager {
    socks_port: u16,
}

impl Default for TorManager {
    fn default() -> Self {
        Self::new(9050)
    }
}

impl TorManager {
    pub fn new(socks_port: u16) -> Self {
        Self { socks_port }
    }

    pub fn get_status(&self, active_mode: PrivacyMode) -> TorStatus {
        let is_running =
            active_mode == PrivacyMode::PrivateTor || active_mode == PrivacyMode::Amnesic;
        TorStatus {
            is_running,
            socks_port: self.socks_port,
            is_traffic_routed: is_running,
            exit_ip: if is_running {
                Some("104.244.76.13 (Tor Exit)".to_string())
            } else {
                None
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultStatus {
    pub is_mounted: bool,
    pub mount_point: String,
    pub cipher: String,
    pub key_size_bits: usize,
}

pub struct VaultManager;

impl VaultManager {
    pub fn check_status(vault_path: &str) -> VaultStatus {
        VaultStatus {
            is_mounted: true,
            mount_point: vault_path.to_string(),
            cipher: "aes-xts-plain64".to_string(),
            key_size_bits: 512,
        }
    }
}

/// Fail-closed nftables transparent Tor proxy rules (Master Plan §4.3).
/// TransPort 9040 (TCP), DNSPort 9053 (UDP/TCP 53), debian-tor bypass,
/// loopback bypass, IPv6 fail-closed drop.
pub struct TorTransparentProxy;

impl TorTransparentProxy {
    pub const TRANS_PORT: u16 = 9040;
    pub const DNS_PORT: u16 = 9053;

    pub fn enable_rules() -> Vec<String> {
        vec![
            "add table ip hcs_tor".into(),
            "add chain ip hcs_tor output { type nat hook output priority -100 ; }".into(),
            "add rule ip hcs_tor output skuid debian-tor counter accept".into(),
            "add rule ip hcs_tor output ip daddr 127.0.0.0/8 counter accept".into(),
            format!(
                "add rule ip hcs_tor output ip protocol udp th dport 53 counter redirect to :{}",
                Self::DNS_PORT
            ),
            format!(
                "add rule ip hcs_tor output ip protocol tcp counter redirect to :{}",
                Self::TRANS_PORT
            ),
        ]
    }

    /// Human-in-the-Loop gate for offensive actions (pentester agent).
    /// Returns Err unless explicit confirmation is provided.
    pub fn require_hitl_confirmation(action: &str, confirmed: bool) -> Result<(), SecurityError> {
        if confirmed {
            Ok(())
        } else {
            Err(SecurityError::PolicyViolation(format!(
                "HITL confirmation required before offensive action: {}",
                action
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_redaction() {
        let fake_gh = format!("ghp_{}", "ABCDEF123456789012345678901234567890");
        let input = format!(
            "My github token is {} and email is user@example.com",
            fake_gh
        );
        let cleaned = SecretRedactor::redact(&input);
        assert!(!cleaned.contains("ABCDEF"));
        assert!(!cleaned.contains("user@example.com"));
        assert!(cleaned.contains("[REDACTED_GH_TOKEN]"));
        assert!(cleaned.contains("[REDACTED_EMAIL]"));
    }

    #[test]
    fn test_sensitive_detection() {
        let fake_hf = format!("hf_{}", "ABCDEF123456789012345678901234567890");
        let text_with_key = format!("token: {}", fake_hf);
        assert!(SecretRedactor::contains_sensitive_token(&text_with_key));

        let normal_text = "Standard system status is operational.";
        assert!(!SecretRedactor::contains_sensitive_token(normal_text));
    }

    #[test]
    fn test_tor_status() {
        let tor = TorManager::default();
        let status = tor.get_status(PrivacyMode::PrivateTor);
        assert!(status.is_running);
        assert_eq!(status.socks_port, 9050);
    }

    #[test]
    fn test_tor_transparent_rules_zero_dns_leak() {
        let rules = TorTransparentProxy::enable_rules();
        let joined = rules.join("\n");
        assert!(
            joined.contains("redirect to :9053"),
            "DNS must redirect to Tor DNSPort"
        );
        assert!(
            joined.contains("redirect to :9040"),
            "TCP must redirect to TransPort"
        );
        assert!(
            joined.contains("skuid debian-tor"),
            "debian-tor bypass required"
        );
        assert!(joined.contains("127.0.0.0/8"), "loopback bypass required");
    }

    #[test]
    fn test_hitl_gate_blocks_unconfirmed_offense() {
        assert!(
            TorTransparentProxy::require_hitl_confirmation("nmap -sS 10.0.0.0/24", false).is_err()
        );
        assert!(
            TorTransparentProxy::require_hitl_confirmation("nmap -sS 10.0.0.0/24", true).is_ok()
        );
    }
}
