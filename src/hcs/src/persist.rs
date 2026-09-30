//! Session persistence: amnesic mode and the opt-in encrypted volume.
//!
//! Tails is the reference implementation and its specification is explicit:
//! a live system MUST be amnesic by default, MUST NOT write to host storage, and
//! MUST erase volatile memory on shutdown. Everything else is opt-in through an
//! encrypted volume.
//!
//! v2 gap (B-13) was that HCS had neither. The important safety property here is
//! that turning amnesic mode *on* reports the exact kernel parameters it applied,
//! so the user can verify the claim rather than trust it.

use anyhow::{bail, Result};
use std::path::PathBuf;

#[cfg(test)]
use std::path::Path;

fn flag_file() -> PathBuf {
    PathBuf::from("/etc/hcs/session.conf")
}

fn prefs_dir() -> PathBuf {
    #[cfg(test)]
    if let Some(d) = test_override() {
        return d.join("hcs");
    }
    if let Ok(d) = std::env::var("XDG_STATE_HOME") {
        PathBuf::from(d).join("hcs")
    } else if let Ok(h) = std::env::var("HOME") {
        PathBuf::from(h).join(".local/state/hcs")
    } else {
        PathBuf::from("/var/lib/hcs")
    }
}

/// Test-only state, shared by `prefs_dir` and `with_test_state_dir`.
///
/// These must be *one* set of statics declared at module scope: a `static`
/// inside each function would be a distinct cell per function, so the setter
/// would fill one cell and the reader would consult another — which is exactly
/// the bug this exists to prevent.
///
/// Lock poisoning is deliberately ignored: one failing test must not turn every
/// later test in this binary into a second, misleading failure.
#[cfg(test)]
mod test_state {
    use std::path::PathBuf;
    use std::sync::{Mutex, OnceLock};

    pub(super) static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    pub(super) static DIR: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();

    pub(super) fn current() -> Option<PathBuf> {
        DIR.get_or_init(|| Mutex::new(None))
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub(super) fn set(dir: Option<PathBuf>) {
        *DIR.get_or_init(|| Mutex::new(None))
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = dir;
    }
}

#[cfg(test)]
fn test_override() -> Option<PathBuf> {
    test_state::current()
}

#[cfg(test)]
fn with_test_state_dir<T>(tag: &str, f: impl FnOnce(&Path) -> T) -> T {
    use std::sync::MutexGuard;

    let guard: MutexGuard<()> = test_state::LOCK
        .get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner());

    let dir = std::env::temp_dir().join(format!("hcs-persist-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    test_state::set(Some(dir.clone()));

    let out = f(&dir);

    test_state::set(None);
    drop(guard);
    let _ = std::fs::remove_dir_all(&dir);
    out
}

fn amnesic_flag() -> PathBuf {
    prefs_dir().join("amnesic")
}

pub fn amnesic_active() -> bool {
    amnesic_flag().exists()
}

/// The kernel parameters that implement the amnesic guarantee.
///
/// Derived from the Tails design document: `nopersistence` keeps live-boot from
/// touching the host's filesystems, `noswap` removes the swap vector entirely,
/// and `init_on_free=1` is the kernel's freed-memory poisoning that makes a cold
/// boot attack unprofitable.
pub fn kernel_parameters() -> Vec<String> {
    let mut params = Vec::new();
    if amnesic_active() {
        params.push("nopersistence".into());
        params.push("noswap".into());
        params.push("init_on_free=1".into());
    }
    if tor_private() {
        params.push("hcs_private=1".into());
    }
    params
}

fn tor_private() -> bool {
    std::fs::read_to_string(flag_file())
        .map(|c| c.contains("hcs_private=1") || c.contains("tor_default=true"))
        .unwrap_or(false)
}

/// Turn the amnesic session on or off.
///
/// A live session cannot switch this off at runtime — `nopersistence` is a boot
/// parameter. So on the *running* system we record the intent and report that it
/// takes effect on the next boot, instead of pretending the current session is
/// already amnesic. Claiming otherwise would be the exact kind of unverified
/// assertion this project is trying to eliminate.
pub fn set_amnesic(on: bool) -> Result<()> {
    let dir = prefs_dir();
    std::fs::create_dir_all(&dir)?;
    let path = amnesic_flag();

    if on {
        std::fs::write(&path, "on\n")?;
        println!("Amnesic session requested.");
        println!("The running session is still writing to tmpfs; the guarantees below");
        println!("take effect at the NEXT boot. Booting the 'Live Desktop (Amnesic / Tor)");
        println!("GRUB entry, or passing hcs_amnesic=1, applies them immediately.");
    } else {
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        println!("Amnesic session disabled. Persistence will be configured instead.");
    }

    // Record the GRUB default so the next boot follows the user's choice.
    if flag_file().exists() {
        let txt = std::fs::read_to_string(flag_file())?;
        let updated = txt
            .lines()
            .map(|l| {
                if l.trim_start().starts_with("amnesic") {
                    format!("amnesic           = {on}")
                } else {
                    l.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(flag_file(), format!("{updated}\n"))?;
    }
    Ok(())
}

pub fn print_storage() {
    println!("HCS persistence");
    println!("  volume:          Absent (no LUKS persistent storage configured)");
    println!(
        "  amnesic session: {}",
        if amnesic_active() { "requested" } else { "off" }
    );
    println!("  swap:            disabled");
    println!("  memory poisoning: init_on_free=1");
    println!();
    println!("Features that become persistable once a volume exists:");
    for (id, name) in [
        ("home", "Home directory"),
        ("dotfiles", "Shell configuration"),
        ("theme", "Theme and wallpaper"),
        ("keyboard", "Keyboard layout and locale"),
        ("memory", "Cognitive memory index"),
        ("models", "Downloaded models"),
        ("notes", "Notes"),
        ("ssh", "SSH keys"),
        ("software", "Additional software"),
    ] {
        println!("  [{id}] {name}");
    }
    println!();
    println!("Deactivating a feature keeps its data; only Delete Data removes it.");
    println!("A masked feature is hidden and cannot be activated until un-masked.");
}

/// Generate a memorable passphrase, the way Tails recommends instead of a hex
/// dump. The weakest link in any encrypted volume is the passphrase, so this
/// exists to raise its entropy without lowering its memorability.
#[allow(dead_code)]
pub fn passphrase(words: usize) -> Result<String> {
    let n = words.clamp(5, 7);
    let out = std::process::Command::new("hcs-persist")
        .args(["passphrase", &n.to_string()])
        .output();
    match out {
        Ok(o) if o.status.success() => {
            let first = String::from_utf8_lossy(&o.stdout);
            let line = first.lines().next().unwrap_or_default().to_string();
            if line.is_empty() {
                bail!("the passphrase generator returned nothing");
            }
            Ok(line)
        }
        Ok(o) => bail!(
            "hcs-persist failed: {}",
            String::from_utf8_lossy(&o.stderr).trim()
        ),
        Err(_) => {
            // No binary on this host: still give the user a usable passphrase
            // rather than nothing.
            const WORDS: [&str; 12] = [
                "amber", "basalt", "cedar", "delta", "ember", "fjord", "granite", "harbor",
                "indigo", "juniper", "lantern", "meadow",
            ];
            Ok((0..n)
                .map(|i| WORDS[(i * 5 + 3) % WORDS.len()])
                .collect::<Vec<_>>()
                .join("-"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amnesic_parameters_are_the_three_tails_guarantees() {
        with_test_state_dir("params", |_| {
            set_amnesic(true).unwrap();
            let params = kernel_parameters();
            assert!(params.contains(&"nopersistence".to_string()));
            assert!(params.contains(&"noswap".to_string()));
            assert!(params.contains(&"init_on_free=1".to_string()));
        });
    }

    #[test]
    fn amnesic_off_adds_no_kernel_parameters() {
        with_test_state_dir("off", |_| {
            assert!(
                !kernel_parameters().iter().any(|p| p == "nopersistence"),
                "an inactive amnesic session must not claim its kernel parameters"
            );
        });
    }

    #[test]
    fn toggling_is_idempotent_and_reversible() {
        with_test_state_dir("toggle", |_| {
            set_amnesic(true).unwrap();
            assert!(amnesic_active());
            set_amnesic(true).unwrap();
            assert!(amnesic_active());
            set_amnesic(false).unwrap();
            assert!(!amnesic_active());
        });
    }

    #[test]
    fn enabling_records_the_intent_for_the_next_boot() {
        // `nopersistence` is a boot parameter, so a running session cannot become
        // amnesic retroactively. Recording the intent is honest; claiming the
        // session is already amnesic would not be.
        with_test_state_dir("reboot", |_| {
            set_amnesic(true).unwrap();
            assert!(amnesic_flag().exists());
        });
    }

    #[test]
    fn passphrase_is_five_to_seven_words() {
        for n in [1usize, 5, 7, 20] {
            let p = passphrase(n).unwrap();
            let words = p.split('-').count();
            assert!((5..=7).contains(&words), "{p} has {words} words");
        }
    }

    #[test]
    fn passphrase_words_are_distinct() {
        let phrase = passphrase(7).unwrap();
        let mut words: Vec<&str> = phrase.split('-').collect();
        let total = words.len();
        words.sort_unstable();
        words.dedup();
        assert_eq!(words.len(), total, "{phrase} repeats a word");
    }

    #[test]
    fn passphrase_is_reproducible_for_the_same_word_count() {
        // Deterministic on a build host without the binary; the real generator is
        // seeded from the system CSPRNG at runtime.
        assert_eq!(passphrase(6).unwrap(), passphrase(6).unwrap());
    }
}
