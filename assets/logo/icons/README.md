# App Icons

The platform icon set (Windows `.ico`, macOS `.icns`, and the various PNG sizes)
is generated from the master logo, not hand-made. We do this at app-setup time so
there is a single source of truth.

## Source of truth

`../system-trace-dark.svg` (the "Pulse Scope" logo). The dark tile reads well as
an app icon on all platforms.

## How to generate (once the Tauri app exists)

Tauri ships an icon generator that produces every required size and format:

```
pnpm tauri icon ../assets/logo/system-trace-dark.svg
```

This creates the full set under the Tauri app's `src-tauri/icons/` directory
(`32x32.png`, `128x128.png`, `icon.ico`, `icon.icns`, and the Store/PNG sizes).

## Favicons (for the website repo)

Export `system-trace-dark.svg` (or light, per background) to `favicon.ico` and
PNG sizes (16, 32, 180, 512) when the website is set up. Any SVG-to-PNG tool or an
online favicon generator works; the SVG is the source.

## Note

No raster files are committed yet because no renderer is configured in this
environment and the app is not scaffolded. Generate them during app setup.
