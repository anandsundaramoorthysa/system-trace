# Walkthrough: Database Encryption Key Persistence & Fallback Verification

This walkthrough documents the successful verification of the database encryption key persistence and fallback logic (`v0.4.2`) on **Windows** (natively) and **Linux** (headless WSL Ubuntu).

---

## 1. Verified Scenarios & Test Results

### 🧪 Scenario 1: Windows Native Verification (Credential Manager)
*   **Method**: Run `pnpm tauri dev` natively on Windows.
*   **Observations**:
    1.  The database key was successfully generated on first launch.
    2.  Checking the **Windows Credential Manager** (Generic Credentials) confirmed that an entry named **`com.systemtrace.app`** with username **`db-encryption-key`** was successfully created and populated.
    3.  Created tracked data, closed the application cleanly via the system tray, and relaunched it.
    4.  The application read the key from the Credential Manager and successfully decrypted the database `system-trace.enc` without creating any `.corrupt` recovery files.

### 🧪 Scenario 2: Windows Fallback Verification (Simulated Keyring Failure)
*   **Method**: Edited `crypto.rs` to force `keyring_get()` to return an error and `keyring_set()` to return `false`.
*   **Observations (Data Exists)**:
    1.  With `system-trace.enc` existing on disk, but the keyring failing, the application refused to boot and panicked with:
        `Failed to setup app: error encountered during setup hook: secure key store is unavailable (Simulated Keyring Failure); not creating a new key because encrypted data already exists.`
    2.  This successfully validates that System Trace protects user data from being overwritten with a fresh key if the keyring experiences transient failure.
*   **Observations (Fresh Install / No Data)**:
    1.  Renamed `system-trace.enc` to `system-trace.enc.bak`.
    2.  Relaunched the application. It booted successfully and fell back to creating a local `db.key` file in `%APPDATA%/com.systemtrace.app/`.
    3.  A new encrypted snapshot database was generated using this local key file.

### 🧪 Scenario 3: Linux Headless Fallback Verification (WSL Ubuntu)
*   **Method**: Installed Rust, cargo, and GTK build/runtime dependencies inside WSL Ubuntu. Compiled and ran the system-trace backend core.
*   **Observations**:
    1.  All **25 unit tests** passed successfully under Linux.
    2.  Ran `cargo run --bin system-trace -- --minimized`. Because WSL runs without a Secret Service keyring daemon, the keyring failed as expected.
    3.  The application fell back to writing a local key file `db.key` inside `~/.local/share/com.systemtrace.app/` and launched successfully.
    4.  Verified the file details of the fallback key:
        ```bash
        $ ls -la ~/.local/share/com.systemtrace.app/
        -rw------- 1 onkar_gite onkar_gite    32 Jun 16 06:13 db.key
        -rw-r--r-- 1 onkar_gite onkar_gite 73768 Jun 16 06:16 system-trace.enc
        ```
        The fallback key file permission is strictly restricted to **`0o600` (`-rw-------`)**, ensuring it can only be read/written by the current user.

---

## 2. macOS Verification Checklist (For Reviewers)

Since this verification was performed on Windows and WSL, a reviewer or teammate with macOS hardware should perform the following quick test:

1.  Clone the repository and run `pnpm tauri dev`.
2.  Open **Keychain Access** app and check under "Local Items" or "login" to confirm that an entry for `com.systemtrace.app` / `db-encryption-key` is written.
3.  Add some test data and restart/reboot the machine.
4.  Relaunch the app and verify that data is loaded successfully without error popups.
