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
fn strip_managed(content: &str) -> String {
    let mut out = String::new();
    let mut skipping = false;
    for line in content.lines() {
        match line.trim() {
            BEGIN => skipping = true,
            END => skipping = false,
            _ if !skipping => {
                out.push_str(line);
                out.push('\n');
            }
            _ => {}
        }
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
    fs::write(&path, next)
        .map_err(|e| format!("could not write hosts file (run as administrator): {e}"))?;
    Ok(count)
}

/// Remove the managed block entirely.
pub fn clear() -> Result<(), String> {
    let path = hosts_path();
    let current = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let next = strip_managed(&current);
    fs::write(&path, next)
        .map_err(|e| format!("could not write hosts file (run as administrator): {e}"))?;
    Ok(())
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
}
