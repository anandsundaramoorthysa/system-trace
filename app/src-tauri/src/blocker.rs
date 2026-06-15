//! System-wide website blocking via the hosts file (Phase 4, gated).
//!
//! Writes `127.0.0.1` entries for blocked domains BETWEEN marker lines, so we only
//! ever touch our own managed block and never disturb the user's existing hosts
//! file. Editing the hosts file needs elevated/administrator permissions; without
//! them the write fails and we return an error the UI surfaces. Off by default;
//! invoked only by the explicit `apply_website_block` / `clear_website_block`
//! commands. Runtime behavior must be verified in an elevated session.

use std::fs;
use std::path::PathBuf;

const BEGIN: &str = "# >>> System Trace block (managed) >>>";
const END: &str = "# <<< System Trace block (managed) <<<";

fn hosts_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        let root = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into());
        PathBuf::from(root).join("System32\\drivers\\etc\\hosts")
    }
    #[cfg(not(target_os = "windows"))]
    {
        PathBuf::from("/etc/hosts")
    }
}

/// Return the hosts content with any existing managed block removed.
///
/// Defensive against a block that was left unterminated (a `BEGIN` with no
/// `END`, e.g. if a previous write was interrupted): inside a managed block we
/// only drop lines that are actually ours (our `127.0.0.1` entries or blanks).
/// The moment we encounter a line that is *not* ours, we stop skipping and keep
/// it - so an unterminated block can never silently delete the user's own hosts
/// entries that follow it.
fn strip_managed(content: &str) -> String {
    let mut out = String::new();
    let mut skipping = false;
    for line in content.lines() {
        let t = line.trim();
        if t == BEGIN {
            skipping = true;
            continue;
        }
        if t == END {
            skipping = false;
            continue;
        }
        if skipping {
            let is_ours = t.is_empty() || t.starts_with("127.0.0.1 ");
            if is_ours {
                continue;
            }
            // Not one of our lines: the block was never terminated. Stop
            // skipping and preserve this (user) line.
            skipping = false;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// Write the managed block for the given domains (idempotent). Returns the count
/// of domains written.
pub fn apply(domains: &[String]) -> Result<usize, String> {
    let path = hosts_path();
    let current = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut next = strip_managed(&current);
    if !next.ends_with('\n') {
        next.push('\n');
    }
    let mut count = 0;
    if !domains.is_empty() {
        next.push_str(BEGIN);
        next.push('\n');
        for d in domains {
            let d = d.trim();
            if d.is_empty() {
                continue;
            }
            next.push_str(&format!("127.0.0.1 {d}\n"));
            next.push_str(&format!("127.0.0.1 www.{d}\n"));
            count += 1;
        }
        next.push_str(END);
        next.push('\n');
    }
    write_hosts(next)?;
    Ok(count)
}

/// Remove the managed block entirely.
pub fn clear() -> Result<(), String> {
    let path = hosts_path();
    let current = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let next = strip_managed(&current);
    write_hosts(next)?;
    Ok(())
}

fn write_hosts(content: String) -> Result<(), String> {
    let path = hosts_path();

    #[cfg(target_os = "linux")]
    {
        use rand::RngCore;
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        use std::process::Command;

        // Stage the new hosts content to a private, unpredictably-named temp
        // file (mode 0600, created with O_EXCL) so a local user cannot
        // pre-create, read, or swap it in before the privileged copy runs.
        // Prefer the user-private XDG_RUNTIME_DIR over the world-writable /tmp.
        let dir = std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(std::env::temp_dir);
        let mut rand_bytes = [0u8; 16];
        rand::rngs::OsRng.fill_bytes(&mut rand_bytes);
        let temp_path = dir.join(format!(
            "system-trace-hosts-{:032x}",
            u128::from_le_bytes(rand_bytes)
        ));

        {
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&temp_path)
                .map_err(|e| format!("failed to create temporary file: {e}"))?;
            file.write_all(content.as_bytes())
                .map_err(|e| format!("failed to write temporary file: {e}"))?;
        }

        let status = Command::new("pkexec")
            .args(["cp", temp_path.to_str().unwrap(), path.to_str().unwrap()])
            .status();

        let _ = fs::remove_file(&temp_path);

        match status {
            Ok(s) if s.success() => Ok(()),
            Ok(s) => {
                let code = s.code().unwrap_or(-1);
                if code == 126 {
                    Err("Elevation prompt was cancelled".to_string())
                } else if code == 127 {
                    Err("pkexec not found. Please install polkit or apply changes manually with sudo."
                        .to_string())
                } else {
                    Err(format!("pkexec failed with exit code {code}"))
                }
            }
            Err(e) => Err(format!("failed to launch pkexec: {e}")),
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        fs::write(&path, content)
            .map_err(|e| format!("could not write hosts file (run as administrator): {e}"))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_removes_only_the_managed_block() {
        let c = format!("127.0.0.1 localhost\n{BEGIN}\n127.0.0.1 x.com\n{END}\n10.0.0.1 keep\n");
        let s = strip_managed(&c);
        assert!(s.contains("127.0.0.1 localhost"));
        assert!(s.contains("10.0.0.1 keep"));
        assert!(!s.contains("x.com"));
        assert!(!s.contains("System Trace block"));
    }

    #[test]
    fn strip_keeps_user_lines_after_an_unterminated_block() {
        // A previous run wrote BEGIN + an entry but crashed before END. The
        // user's real entries follow. They must survive the next strip.
        let c = format!(
            "127.0.0.1 localhost\n{BEGIN}\n127.0.0.1 x.com\n10.0.0.1 keep-me\n203.0.113.5 also-keep\n"
        );
        let s = strip_managed(&c);
        assert!(s.contains("127.0.0.1 localhost"));
        assert!(s.contains("10.0.0.1 keep-me"));
        assert!(s.contains("203.0.113.5 also-keep"));
        assert!(!s.contains("x.com"));
        assert!(!s.contains("System Trace block"));
    }
}
