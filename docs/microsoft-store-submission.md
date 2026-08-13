# Microsoft Store submission checklist (MSIX / free path)

How to publish **System Trace** on the Microsoft Store via **Path A: MSIX**. The
Store is a free, optional Windows channel. With MSIX, **Microsoft signs the
package for you after certification — no code-signing certificate to buy.**
Direct download from the website stays the primary Windows channel.

> **Why MSIX and not the .msi?** The Store's EXE/MSI channel requires you to
> Authenticode-sign the installer yourself with a paid certificate (Microsoft
> does not re-sign EXE/MSI). MSIX is the free route: Microsoft signs it. Our
> package is a **full-trust** desktop app (`runFullTrust`), so app/window
> tracking works normally — it is **not** the restricted UWP sandbox.

---

## 1. Build + package the MSIX

The **Store edition** is built with the `msstore` Cargo feature, which compiles
out system-wide website blocking (it edits the hosts file and needs admin
rights — disallowed by Store policy, and impossible in a packaged app anyway).
Everything else is unchanged.

```bash
# 1) Build the Store-edition executable
cd app
pnpm tauri build --features msstore --bundles msi     # the exe is what we need;
                                                      # --bundles none also works

# 2) Package it as MSIX with your Partner Center identity values (see step 2b)
pwsh app/src-tauri/msix/pack.ps1 `
  -IdentityName        "<Package/Identity/Name from Partner Center>" `
  -Publisher           "<Package/Identity/Publisher from Partner Center>" `
  -PublisherDisplayName "Anand Sundaramoorthy SA" `
  -Version             "0.5.0.0"
```

Output: `app/src-tauri/target/msix/SystemTrace_0.5.0.0.msix` ← **upload this**
(unsigned; the Store signs it). The packaging scaffolding lives in
`app/src-tauri/msix/` (`AppxManifest.xml` + `pack.ps1`); the tile/logo assets
are the ones Tauri already generates under `app/src-tauri/icons/`.

> For a **local install test** only, add `-SelfSign` (needs an elevated session
> to trust the throwaway cert). Never submit a self-signed package.

---

## 2. Partner Center account + app identity (your action)

### 2a. Create the account — DONE ✅
- **Individual** account (registration fee waived, ₹0).
- Publisher display name: **Anand Sundaramoorthy SA**.

### 2b. Reserve the name and copy the identity values ← **DO THIS NEXT**
1. Dashboard → **Apps and games → + New product → App** → reserve **`System Trace`**
   (if taken, e.g. `System Trace - Screen Time`).
2. Open the product → **Product management → Product identity**. Copy these
   **three values** and send them to your build (they must match the MSIX
   manifest exactly, or certification rejects the package):

   | Partner Center field | Goes into |
   |---|---|
   | **Package/Identity/Name** (e.g. `1234Publisher.SystemTrace`) | `-IdentityName` |
   | **Package/Identity/Publisher** (e.g. `CN=XXXXXXXX-XXXX-…`) | `-Publisher` |
   | **Package/Properties/PublisherDisplayName** | `-PublisherDisplayName` |

Once you paste those into `pack.ps1`, the produced `.msix` is submission-ready.

---

## 3. App identity (matches `tauri.conf.json`)

| Field | Value |
|---|---|
| Product name | System Trace |
| Publisher display name | Anand Sundaramoorthy SA |
| Version | 0.5.0 (MSIX uses 0.5.0.0; keep the 4th part 0 — Store reserves it) |
| Category | Utilities & tools |
| Short description | Privacy-first screen-time tracker. |
| Long description | System Trace is a privacy-first, local-first, cross-platform screen-time tracker. All activity data stays on your computer. |

---

## 4. Store listing assets

| Asset | Status / source |
|---|---|
| **Privacy policy URL** (required) | https://system-trace.pages.dev/privacy/ |
| Support / website URL | https://system-trace.pages.dev/ |
| Screenshots (min 1, 1366×768+) | Reuse the 8 in the website repo `public/screenshots/` (dashboard, app-limits/focus, break-reminders, weekly-report — dark + light). |
| Store logo / tiles | Already in the MSIX (`StoreLogo`, `Square44/71/150`). |
| Age rating (IARC questionnaire) | Answer honestly → expected **everyone / PEGI 3**. No ads, no user-to-user comms, no data collection. |
| Markets | All markets (English listing). |
| Pricing | Free. |

**Listing angle:** emphasize *local-first, no account, no telemetry, encrypted
at rest*. Do **not** advertise website blocking — it's disabled in the Store
edition (shown greyed-out with a note pointing to the full version).

---

## 5. Certification

For MSIX, **Microsoft runs certification** (automated + policy) after you submit;
they sign the package on success. You can optionally pre-check the package
locally with the Windows App Certification Kit (needs an **elevated** session):

```powershell
& "C:\Program Files (x86)\Windows Kits\10\App Certification Kit\appcert.exe" reset
& "C:\Program Files (x86)\Windows Kits\10\App Certification Kit\appcert.exe" test `
  -apptype appx `
  -packagefullpath "app\src-tauri\target\msix\SystemTrace_0.5.0.0.msix" `
  -reportoutputpath "wack-report.xml"
```

---

## 6. Submission steps (Partner Center)

1. **Apps and games → System Trace → Start your submission.**
2. **Packages** → upload `SystemTrace_0.5.0.0.msix`. (Partner Center validates
   the identity matches the reserved app.)
3. **Store listing** → description, screenshots, category, search terms.
4. **Privacy policy URL** → the link above (required — submission blocks without it).
5. **Age ratings** → complete the IARC questionnaire.
6. **Properties / declarations** → note it reads the foreground app/window title
   for time tracking, stores data locally, and does **not** transmit data.
7. **Submit** → certification runs (hours to a few days). On success it goes live.

---

## 7. After it's live

- You get an `apps.microsoft.com` / `ms-windows-store://` link — add it to the
  website Download page as a secondary Windows option.
- Updates: bump the version, rebuild `--features msstore`, re-run `pack.ps1`
  (bump `-Version`, keep the same identity), upload the new MSIX.

---

## Known limitations / follow-ups (Store edition)

- **Autostart:** the default `tauri-plugin-autostart` (registry/startup-folder)
  is virtualized under MSIX and won't actually start on login. The manifest
  declares a `windows.startupTask` extension (disabled by default); wiring the
  in-app "start on login" toggle to the Windows `StartupTask` API is a follow-up.
  Not blocking for a first submission.
- **Website blocking:** intentionally disabled in this edition (see above).

## Known review risks (be ready to respond)

- **System-monitoring behavior.** The app reads the foreground app/window title
  for time tracking — legitimate for a screen-time tracker; explain in the
  submission notes if asked. Window-title capture is **optional and off by
  default**, which helps.
- **If the Store rejects it anyway**, direct download from the website (the NSIS
  setup `.exe`) remains fully functional and is the recommended primary Windows
  channel. Store approval is a bonus, not a dependency.
