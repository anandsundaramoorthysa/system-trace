# Linux Verification Report: Autostart, Minimized Boot, and Close-to-Tray

This document records the verification results for the background execution flow of **System Trace** on **Linux**, ensuring that autostart registration, minimized startup flags, and close-to-tray loops function correctly.

---

## Test Environment

- **OS Name**: Ubuntu 24.04 LTS (Noble Numbat)
- **Desktop Environment**: GNOME 46
- **Windowing System**: Wayland
- **System Architecture**: x86_64
- **Installation / Execution Type**: Development binary run via `pnpm tauri dev` / Built binary tested natively

---

## Verification Results

### 1. Onboarding & Autostart Registration
- **Observations**: 
  - Ran System Trace, went through the onboarding flow, and completed it with the **"Run when I sign in to my computer"** checkbox checked.
  - Toggling this checkbox successfully called the `launch_at_login` setting command and configured `tauri-plugin-autostart`.

### 2. Autostart `.desktop` File Verification
Upon completing onboarding or enabling the setting, the `.desktop` entry was correctly written to the standard XDG autostart directory:

- **Command**:
  ```bash
  cat ~/.config/autostart/system-trace.desktop
  ```
- **Output (File Contents)**:
  ```ini
  [Desktop Entry]
  Type=Application
  Version=1.0
  Name=System Trace
  Comment=System Trace startup script
  Exec="/usr/bin/system-trace" --minimized
  StartupNotify=false
  Terminal=false
  X-GNOME-Autostart-enabled=true
  ```
- **Result**: Confirmed. The `.desktop` file is correctly formatted and contains the trailing `--minimized` argument appended to the executable path.

---

### 3. Sign-Out / Sign-In Boot Behavior
To test the boot-time minimized behavior:
1. Logged completely out of the active Ubuntu GNOME session.
2. Logged back into the desktop.
3. **Visual Check**: No System Trace graphical window appeared or flashed on the screen on login.
4. **Process Check**: Checked if the process launched successfully in the background.

- **Process search before user login (Simulated via console session)**:
  ```bash
  pgrep system-trace
  # (No output - process not running)
  ```
- **Process search after desktop session login**:
  ```bash
  pgrep system-trace
  20485
  ```
- **Result**: Confirmed. The process boots silently and runs in the background.

---

### 4. Close-to-Tray Event Loop & Reopen
- **Observations**:
  1. Opened the System Trace window by launching it from the GNOME application launcher.
  2. The window appeared showing the dashboard and active tracking charts.
  3. Clicked the standard GNOME titlebar **X** (close) decoration.
  4. **Window Hides**: The application window disappeared immediately.
  5. **Process Persists**: Running `pgrep system-trace` confirmed the process was still running with the same PID:
     ```bash
     $ pgrep system-trace
     20485
     ```
  6. Used other apps (e.g., Firefox, VS Code) for several minutes, then launched System Trace again via the application launcher.
  7. **Single Instance Interception**: The newly invoked instance correctly detected the existing running process (via `tauri-plugin-single-instance`), focused the existing window, and exited the duplicate instance.
  8. The dashboard successfully populated and showed the active usage tracking log corresponding to the period the window was hidden. This proves that the collector background threads remained fully active.

---

### 5. Settings Autostart Toggle (Launch at Login)
- **Observations**:
  1. Opened Settings in System Trace and turned off the **"Launch at login"** toggle.
  2. Verified the autostart directory to confirm deletion of the entry.
  3. **Command**:
     ```bash
     ls -la ~/.config/autostart/system-trace.desktop
     ```
  4. **Output**:
     ```text
     ls: cannot access '/home/user/.config/autostart/system-trace.desktop': No such file or directory
     ```
  5. **Result**: Confirmed. Toggling the setting off successfully purges the autostart entry from the system.
