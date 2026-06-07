# Security Policy

System Trace is a local-first desktop application. It does not have accounts,
servers, or telemetry, so its security surface is the desktop app itself and the
local SQLite database it writes on the user's machine.

## Supported versions

Security fixes are applied to the latest released version on the `main` branch.
Older versions are not patched separately.

| Version  | Supported |
| -------- | --------- |
| latest   | yes       |
| older    | no        |

## Reporting a vulnerability

Please do **not** open a public GitHub issue for security problems.

Email the maintainer privately at **sanand03072005@gmail.com** with:

- A clear description of the issue and the impact you believe it has.
- Steps to reproduce, or a minimal proof of concept.
- The operating system, System Trace version, and any relevant configuration.
- Optionally, a suggested fix.

You can expect an acknowledgement within a few days. Once the issue is
understood, a fix will be prepared on a private branch, released, and credited
to the reporter in the changelog (unless you ask to remain anonymous).

## Scope

In scope:

- The desktop application code in this repository (Rust core, React UI, IPC).
- The per-OS watchers in `app/src-tauri/src/platform/`.
- The local SQLite schema, migrations, and data-retention behavior.
- The website blocker's edits to the system `hosts` file.
- The website source code in the [System-Trace-Website](https://github.com/anandsundaramoorthysa/System-Trace-Website)
  repository.

Out of scope:

- Vulnerabilities in third-party dependencies that already have public CVEs and
  upstream fixes - please report those upstream. We will update our dependency
  versions as part of normal maintenance.
- Issues that require an attacker who already has full local user privileges on
  the user's machine (at that point, System Trace's local database is the least
  of the user's problems).

## Our security posture

A few guarantees the project tries to keep:

- **No network calls for activity data.** The collector and UI never send the
  user's window or app activity off-device. If a feature ever needs the network
  (for example, an optional update check), it must be opt-in and clearly
  documented.
- **Hosts-file edits are bounded.** The website blocker only adds and removes
  entries between two clearly marked sentinel lines in the system `hosts` file,
  and it requires admin/root privileges to do so.
- **Local data stays local.** The SQLite database lives in the OS application
  data directory and is not synced or uploaded by the app.

Thanks for helping keep System Trace and its users safe.
