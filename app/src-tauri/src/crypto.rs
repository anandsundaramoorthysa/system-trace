//! Data-at-rest encryption for the local database.
//!
//! The live database is held **in memory**; only encrypted snapshots are ever
//! written to disk (periodically and on exit). So no plaintext database file
//! exists at rest - the on-disk file is XChaCha20-Poly1305 ciphertext. The key
//! is a random 32 bytes kept in the OS credential store (Windows Credential
//! Manager / macOS Keychain / Linux Secret Service) via `keyring`, with a
//! restricted key-file fallback when no keyring is available (e.g. a headless
//! box) so the app still works.
//!
//! Pure-Rust crypto (no OpenSSL / C build tools), so it builds on every target
//! with just `cargo`.

use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use rand::RngCore;
use std::path::Path;

const KEYRING_SERVICE: &str = "com.systemtrace.app";
const KEYRING_USER: &str = "db-encryption-key";
const NONCE_LEN: usize = 24;

/// Load the database key.
///
/// `data_exists` is whether an encrypted snapshot (or key file) already exists.
/// This matters for safety: if the keyring **errors** (locked / not ready /
/// transient) while encrypted data exists, we must NOT mint a fresh key - that
/// would overwrite the only key and make the data permanently undecryptable. In
/// that case we return `Err` so the caller can fail safely and recover on a
/// later launch. A new key is only created when there is genuinely no key and
/// no existing data (a fresh install).
pub fn load_or_create_key(fallback_path: &Path, data_exists: bool) -> Result<[u8; 32], String> {
    match keyring_get() {
        // The key is in the OS store - the normal path.
        Ok(Some(k)) => Ok(k),

        // The store works but holds no entry. A durable key-file copy is always
        // written alongside the keyring (see `create_and_store`), so try it
        // first - that is the normal recovery when the OS credential store
        // loses the entry (login-password change, keychain reset, profile
        // rebuild). If it too is missing AND encrypted data already exists, the
        // key is genuinely gone: minting a new one would make that data
        // permanently undecryptable, so we FAIL CLOSED instead of silently
        // wiping months of history. A new key is only created for a genuinely
        // fresh install (no data, no key anywhere).
        Ok(None) => {
            if let Some(k) = read_key_file(fallback_path) {
                // Re-seed the keyring so it holds the key again next launch.
                let _ = keyring_set(&k);
                return Ok(k);
            }
            if data_exists {
                return Err(
                    "the database encryption key is missing from the OS secure store \
                     and no key backup was found, but your encrypted data still exists. \
                     To avoid destroying it, System Trace will not create a new key. If \
                     your keychain/credential store was reset, restore it (or restore a \
                     System Trace backup) and launch again."
                        .to_string(),
                );
            }
            Ok(create_and_store(fallback_path))
        }

        // The store itself errored (locked / temporarily unavailable). Do NOT
        // mint a new key when encrypted data exists - that could discard
        // recoverable data once the store comes back. Try the file fallback;
        // otherwise fail the launch so a retry recovers.
        Err(e) => {
            if let Some(k) = read_key_file(fallback_path) {
                return Ok(k);
            }
            if data_exists {
                return Err(format!(
                    "secure key store is unavailable ({e}); not creating a new key \
                     because encrypted data already exists. Please try launching again."
                ));
            }
            Ok(create_and_store(fallback_path))
        }
    }
}

fn read_key_file(path: &Path) -> Option<[u8; 32]> {
    let bytes = std::fs::read(path).ok()?;
    // Current format: OS-protected blob (DPAPI on Windows; raw on Unix).
    if let Some(raw) = unprotect(&bytes) {
        if raw.len() == 32 {
            let mut k = [0u8; 32];
            k.copy_from_slice(&raw);
            return Some(k);
        }
    }
    // Legacy format: a raw 32-byte key written by older builds (the file
    // fallback used to be plaintext). Accept it so existing installs still
    // open; it is rewritten in the protected format on the next create/store.
    if bytes.len() == 32 {
        let mut k = [0u8; 32];
        k.copy_from_slice(&bytes);
        return Some(k);
    }
    None
}

fn create_and_store(fallback_path: &Path) -> [u8; 32] {
    let mut key = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut key);
    // Store in the OS keyring (primary) AND always write a durable, OS-protected
    // key-file copy. Keeping a second copy is deliberate: the OS credential
    // store losing its entry over time is the leading cause of "my data
    // vanished after months", and a keyring-only design has no recovery. The
    // file is not a plaintext downgrade - it is DPAPI-protected on Windows and
    // mode-0600 on Unix (see `write_key_file`).
    let _ = keyring_set(&key);
    let _ = write_key_file(fallback_path, &key);
    key
}

/// `Ok(Some(key))` = found, `Ok(None)` = no entry yet, `Err` = keyring error
/// (locked / unavailable / corrupt entry) - which the caller must NOT treat as
/// "no key".
fn keyring_get() -> Result<Option<[u8; 32]>, String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER).map_err(|e| e.to_string())?;
    match entry.get_password() {
        Ok(hex) => match from_hex(&hex) {
            Some(bytes) if bytes.len() == 32 => {
                let mut k = [0u8; 32];
                k.copy_from_slice(&bytes);
                Ok(Some(k))
            }
            _ => Err("stored key is malformed".into()),
        },
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

fn keyring_set(key: &[u8; 32]) -> bool {
    match keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER) {
        Ok(entry) => entry.set_password(&to_hex(key)).is_ok(),
        Err(_) => false,
    }
}

fn write_key_file(path: &Path, key: &[u8; 32]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    // Protect the key at rest: DPAPI (bound to the current Windows user) so a
    // copy of the file is useless on another machine/account; raw bytes on Unix
    // where the 0600 permission below is the protection.
    let blob =
        protect(key).ok_or_else(|| std::io::Error::other("could not protect key material"))?;
    std::fs::write(path, &blob)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

/// OS-level protection for the durable key file. On Windows this is DPAPI (the
/// blob is bound to the current user account and machine); elsewhere the key is
/// stored as-is and protected by the 0600 permission + user-owned data dir.
#[cfg(target_os = "windows")]
fn protect(data: &[u8]) -> Option<Vec<u8>> {
    dpapi::crypt_protect(data)
}
#[cfg(target_os = "windows")]
fn unprotect(data: &[u8]) -> Option<Vec<u8>> {
    dpapi::crypt_unprotect(data)
}
#[cfg(not(target_os = "windows"))]
fn protect(data: &[u8]) -> Option<Vec<u8>> {
    Some(data.to_vec())
}
#[cfg(not(target_os = "windows"))]
fn unprotect(data: &[u8]) -> Option<Vec<u8>> {
    Some(data.to_vec())
}

/// DPAPI (`CryptProtectData`/`CryptUnprotectData`) wrappers. Encrypts key
/// material to the current user so the on-disk key file is unusable if copied
/// off the machine or read by another account.
#[cfg(target_os = "windows")]
mod dpapi {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{LocalFree, HLOCAL};
    use windows::Win32::Security::Cryptography::{
        CryptProtectData, CryptUnprotectData, CRYPT_INTEGER_BLOB,
    };

    fn in_blob(data: &[u8]) -> CRYPT_INTEGER_BLOB {
        CRYPT_INTEGER_BLOB {
            cbData: data.len() as u32,
            pbData: data.as_ptr() as *mut u8,
        }
    }

    // Copy the LocalAlloc'd output into an owned Vec, then free the OS buffer.
    fn take(out: CRYPT_INTEGER_BLOB) -> Vec<u8> {
        let v = unsafe { std::slice::from_raw_parts(out.pbData, out.cbData as usize) }.to_vec();
        unsafe {
            let _ = LocalFree(HLOCAL(out.pbData as *mut _));
        }
        v
    }

    pub fn crypt_protect(data: &[u8]) -> Option<Vec<u8>> {
        let input = in_blob(data);
        let mut output = CRYPT_INTEGER_BLOB::default();
        let ok =
            unsafe { CryptProtectData(&input, PCWSTR::null(), None, None, None, 0, &mut output) };
        ok.ok()?;
        Some(take(output))
    }

    pub fn crypt_unprotect(data: &[u8]) -> Option<Vec<u8>> {
        let input = in_blob(data);
        let mut output = CRYPT_INTEGER_BLOB::default();
        let ok = unsafe { CryptUnprotectData(&input, None, None, None, None, 0, &mut output) };
        ok.ok()?;
        Some(take(output))
    }
}

/// Encrypt a plaintext blob; output is `nonce || ciphertext`.
pub fn encrypt(key: &[u8; 32], plaintext: &[u8]) -> Result<Vec<u8>, String> {
    let cipher = XChaCha20Poly1305::new(Key::from_slice(key));
    let mut nonce_bytes = [0u8; NONCE_LEN];
    rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
    let ct = cipher
        .encrypt(XNonce::from_slice(&nonce_bytes), plaintext)
        .map_err(|_| "encryption failed".to_string())?;
    let mut out = Vec::with_capacity(NONCE_LEN + ct.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ct);
    Ok(out)
}

/// Decrypt a `nonce || ciphertext` blob produced by [`encrypt`].
pub fn decrypt(key: &[u8; 32], data: &[u8]) -> Result<Vec<u8>, String> {
    if data.len() < NONCE_LEN {
        return Err("encrypted database is too short / corrupt".into());
    }
    let (nonce_bytes, ct) = data.split_at(NONCE_LEN);
    let cipher = XChaCha20Poly1305::new(Key::from_slice(key));
    cipher
        .decrypt(XNonce::from_slice(nonce_bytes), ct)
        .map_err(|_| "could not decrypt database (wrong key or corrupt file)".to_string())
}

fn to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

fn from_hex(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_encrypts_and_decrypts() {
        let key = [7u8; 32];
        let msg = b"SQLite format 3\0some database bytes";
        let ct = encrypt(&key, msg).unwrap();
        assert_ne!(&ct[24..], &msg[..]); // actually encrypted
        let pt = decrypt(&key, &ct).unwrap();
        assert_eq!(pt, msg);
    }

    #[test]
    fn wrong_key_fails() {
        let ct = encrypt(&[1u8; 32], b"secret").unwrap();
        assert!(decrypt(&[2u8; 32], &ct).is_err());
    }

    #[test]
    fn hex_round_trips() {
        let b = [0u8, 15, 16, 255, 42];
        assert_eq!(from_hex(&to_hex(&b)).unwrap(), b);
    }
}
