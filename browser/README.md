# Zenko Browser

A fast everyday browser in Zenko's night-shrine style that talks to Zenko.

- **Engine:** WebView2 (the Edge/Chromium engine already on Windows 10/11), so sites, sign-ins, video and PDFs work like in Edge.
- **Shell:** Rust (`tao` + `wry`), a few MB, starts instantly. The frame (tabs, toolbar, command bar) is a web page drawn by
  Chromium's GPU compositor, so animations run at your screen's full refresh rate.
- **Built by GitHub Actions** (`.github/workflows/browser.yml`): nothing to install on your PC.

## Stage 1 (this version)
Tabs on top or on the side (drag to reorder, middle-click to close), session restore, Ctrl+K command bar (tabs, sites, commands,
inline maths), Claude side panel (resizable, pop-out window), shrine new-tab page, one heavily customisable theme (8 Zenko palettes or
your own accent, fonts, tab shapes, roundness, spacing, size, stars, glow, motion), remappable shortcuts, saved passwords and autofill,
tab sleep, mute per tab, video full screen, DevTools, print/PDF, auto-update from GitHub Releases, and the Zenko link
(window title + `%APPDATA%\FoxBuddy\browser_now.json`, so Zenko logs study time per subject).

## Releases
Publish a **pre-release** with a tag like `browser-v0.1.0` (must match `version` in `Cargo.toml`). The workflow attaches
`ZenkoBrowser.exe` + `.sha256`. Pre-release keeps it out of Zenko's own "latest release" updater.

## Data
`%LOCALAPPDATA%\ZenkoBrowser` (session, sign-ins, saved passwords).
