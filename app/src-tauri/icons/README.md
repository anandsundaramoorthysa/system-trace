# App icons

These files are generated, not authored by hand. Run the Tauri icon generator
from the master logo SVG at app-setup time (DECISIONS.md #18):

    pnpm tauri icon ../../assets/logo/system-trace-dark.svg

That command writes every platform size into this folder:

- `32x32.png`, `128x128.png`, `128x128@2x.png` (Linux / general)
- `icon.icns` (macOS)
- `icon.ico` (Windows)
- `icon.png` (tray; referenced by `tauri.conf.json` -> `app.trayIcon.iconPath`)

Do not commit hand-edited icons here; regenerate from the source SVG so the brand
stays consistent. The source logos live in `assets/logo/` (BRAND.md).
