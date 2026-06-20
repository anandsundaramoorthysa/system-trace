# Windows Production Installer Verification Report

This document records the verification results for the production installer flow of **System Trace** on **Windows**, ensuring that background autostart registration, minimized startup flags, and close-to-tray loops function correctly in a packaged build.

---

## Test Environment

- **OS Name**: Microsoft Windows 11 Home Single Language
- **OS Version**: 10.0.26200
- **OS Build Number**: 26200
- **System Architecture**: x64
- **Installer Type Tested**: NSIS Package (`System Trace_0.4.2_x64-setup.exe` generated via `pnpm tauri build`)

---

## Verification Results

### 1. Installation & Elevation Flow
- **Observations**: 
  - By default, the NSIS installer installs the application for the current user in `%LocalAppData%\System Trace\`.
  - Because it installs to user space, it **does not require administrator privileges or trigger any UAC prompts** during installation.
  - A silent installation using the `/S` flag completes successfully with exit code `0` and populates the folder at `C:\Users\Onkar Gite\AppData\Local\System Trace\`.

### 2. Windows SmartScreen Warnings
- **Observations**:
  - Because the binary is not currently code-signed, running the installer interactively triggers the standard Windows SmartScreen warning ("Windows protected your PC").
  - The warning was cleared by clicking **"More info"** and selecting **"Run anyway"**. No administrative rights or passwords were requested or required to proceed.

### 3. Autostart Registry Run Key (Launch at Login)
The autostart behavior is managed by `tauri-plugin-autostart`. The registry run key under `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` was queried before and after toggling the setting:

- **Launch at Login Enabled (After Onboarding/Startup)**:
  - Command:
    ```powershell
    reg query "HKCU\Software\Microsoft\Windows\CurrentVersion\Run" /v "System Trace"
    ```
  - Output:
    ```text
    HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Run
        System Trace    REG_SZ    C:\Users\Onkar Gite\AppData\Local\System Trace\system-trace.exe --minimized
    ```
  - **Result**: Confirmed. The plugin successfully resolves the path to the installed executable and adds the `--minimized` boot argument.

- **Launch at Login Disabled (Toggled Off)**:
  - Command:
    ```powershell
    reg query "HKCU\Software\Microsoft\Windows\CurrentVersion\Run" /v "System Trace"
    ```
  - Output:
    ```text
    ERROR: The system was unable to find the specified registry key or value.
    ```
  - **Result**: Confirmed. Toggling the setting off successfully cleans up and removes the registry run key.

### 4. Minimized Startup (`--minimized`) Argument
- **Observations**:
  - Terminating the active process and executing the binary with the `--minimized` argument manually confirms correct background behavior:
    ```powershell
    Start-Process -FilePath "C:\Users\Onkar Gite\AppData\Local\System Trace\system-trace.exe" -ArgumentList "--minimized"
    ```
  - The process successfully launches in the background and is running (confirmed via `Get-Process system-trace`), but **no graphical window is shown on screen**.

### 5. Close-to-Tray Event Loop
- **Observations**:
  - Clicking the **X** button on the main app window triggers the `tauri::WindowEvent::CloseRequested` listener in `app/src-tauri/src/lib.rs`.
  - The event listener calls `api.prevent_close()` and hides the window using `window.hide()`.
  - The `system-trace` background process continues running in Task Manager, and the collector continues tracking active windows and recording screen time to the local SQLite database.
  - Launching the app again via the Start Menu shortcut correctly triggers the `tauri-plugin-single-instance` handler in the running process. The existing instance repositions the window on the active monitor, shows the window, and focuses it, while the second temporary instance exits.
